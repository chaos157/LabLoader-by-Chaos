use crate::profiles::Profile;
use crate::util;
use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

const DEPLOY_MANIFEST: &str = ".blmp-deployed.json";
const EXE_NAMES: [&str; 4] = [
    "BONELAB_Steam_Windows64.exe",
    "BONELAB_Oculus_Windows64.exe",
    "BONELAB.exe",
    "Bonelab.exe",
];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployResult {
    pub deployed: usize,
    pub removed: usize,
}

pub fn find_exe(dir: &Path) -> Option<PathBuf> {
    for name in EXE_NAMES {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let lower = name.to_lowercase();
        if lower.starts_with("bonelab") && lower.ends_with(".exe") {
            return Some(path);
        }
    }
    None
}

pub fn resolve_dir(raw: &str) -> Result<PathBuf, String> {
    let trimmed = raw.trim().trim_matches('"');
    if trimmed.is_empty() {
        return Err("Set your BONELAB folder in Settings first.".into());
    }
    let path = PathBuf::from(trimmed);
    if path.is_file() {
        return path
            .parent()
            .map(|parent| parent.to_path_buf())
            .ok_or_else(|| "That path does not point at a folder.".to_string());
    }
    if !path.is_dir() {
        return Err(format!("Folder not found: {}", path.display()));
    }
    if find_exe(&path).is_some() {
        return Ok(path);
    }
    for candidate in ["BONELAB", "Bonelab", "BONELAB Demo"] {
        let nested = path.join(candidate);
        if nested.is_dir() && find_exe(&nested).is_some() {
            return Ok(nested);
        }
    }
    Ok(path)
}


#[cfg(windows)]
fn read_reg_string(
    root: winreg::HKEY,
    key_path: &str,
    value_name: &str,
) -> Option<String> {
    use winreg::RegKey;
    let key = RegKey::predef(root).open_subkey(key_path).ok()?;
    key.get_value::<String, _>(value_name).ok()
}

#[cfg(windows)]
fn steam_roots() -> Vec<PathBuf> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    let mut roots = Vec::new();
    for (root, key, value) in [
        (HKEY_CURRENT_USER, "Software\\Valve\\Steam", "SteamPath"),
        (
            HKEY_LOCAL_MACHINE,
            "SOFTWARE\\WOW6432Node\\Valve\\Steam",
            "InstallPath",
        ),
        (HKEY_LOCAL_MACHINE, "SOFTWARE\\Valve\\Steam", "InstallPath"),
    ] {
        if let Some(path) = read_reg_string(root, key, value) {
            roots.push(PathBuf::from(path.replace('/', "\\")));
        }
    }
    roots
}

#[cfg(not(windows))]
fn steam_roots() -> Vec<PathBuf> {
    Vec::new()
}

fn parse_library_folders(path: &Path, libraries: &mut Vec<PathBuf>) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('"') {
            continue;
        }
        let parts: Vec<&str> = trimmed.split('"').collect();
        if parts.len() < 5 {
            continue;
        }
        let key = parts[1].to_lowercase();
        if key != "path" && key.parse::<u32>().is_err() {
            continue;
        }
        let value = parts[3].replace("\\\\", "\\");
        if value.contains(':') {
            libraries.push(PathBuf::from(value));
        }
    }
}

fn steam_libraries() -> Vec<PathBuf> {
    let mut libraries = Vec::new();
    for steam in steam_roots() {
        libraries.push(steam.clone());
        parse_library_folders(&steam.join("steamapps").join("libraryfolders.vdf"), &mut libraries);
        parse_library_folders(&steam.join("config").join("libraryfolders.vdf"), &mut libraries);
    }
    let mut dedup: HashSet<String> = HashSet::new();
    libraries
        .into_iter()
        .filter(|path| dedup.insert(path.to_string_lossy().to_lowercase()))
        .collect()
}

pub fn detect_steam() -> Option<PathBuf> {
    let mut libraries = steam_libraries();
    // Also check the old default path even when the registry is unreadable.
    libraries.push(PathBuf::from("C:\\Program Files (x86)\\Steam"));
    let mut seen = HashSet::new();
    for library in libraries {
        if !seen.insert(library.to_string_lossy().to_lowercase()) {
            continue;
        }
        let common = library.join("steamapps").join("common");
        for folder in ["BONELAB", "Bonelab"] {
            let candidate = common.join(folder);
            if candidate.is_dir() && find_exe(&candidate).is_some() {
                return Some(candidate);
            }
        }
    }
    None
}

pub fn detect_oculus() -> Option<PathBuf> {
    let mut roots = Vec::new();
    for variable in ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"] {
        if let Ok(program_files) = std::env::var(variable) {
            roots.push(
                PathBuf::from(program_files)
                    .join("Oculus")
                    .join("Software")
                    .join("Software"),
            );
        }
    }
    roots.push(PathBuf::from(
        "C:\\Program Files\\Oculus\\Software\\Software",
    ));

    for root in roots {
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            if name.contains("bonelab") && find_exe(&path).is_some() {
                return Some(path);
            }
        }
    }

    let fallback = PathBuf::from(
        "C:\\Program Files\\Oculus\\Software\\Software\\stress-level-zero-inc-bonelab",
    );
    if fallback.is_dir() && find_exe(&fallback).is_some() {
        return Some(fallback);
    }
    None
}

fn read_manifest(game_dir: &Path) -> Vec<String> {
    std::fs::read_to_string(game_dir.join(DEPLOY_MANIFEST))
        .ok()
        .and_then(|text| serde_json::from_str::<Vec<String>>(&text).ok())
        .unwrap_or_default()
}

fn write_manifest(game_dir: &Path, files: &[String]) -> Result<(), String> {
    let text = serde_json::to_string(files).map_err(|e| e.to_string())?;
    std::fs::write(game_dir.join(DEPLOY_MANIFEST), text).map_err(|e| e.to_string())
}

pub fn clear_deployed(game_dir: &Path) -> Result<usize, String> {
    let previous = read_manifest(game_dir);
    for rel in &previous {
        if let Some(clean) = util::clean_rel(rel) {
            let _ = std::fs::remove_file(game_dir.join(clean));
        }
    }
    util::prune_empty_dirs(&game_dir.join("Mods"));
    let _ = std::fs::remove_file(game_dir.join(DEPLOY_MANIFEST));
    Ok(previous.len())
}

/// Renames DLLs that were already in the game's Mods folder (manual installs,
/// old managers, the vanilla setup) to `.disabled`, so only the active profile's
/// mods can load. Files are kept in place and can be re-enabled from Installed.
pub fn park_loose_mods(game_dir: &Path) -> usize {
    let mods_dir = game_dir.join("Mods");
    if !mods_dir.is_dir() {
        return 0;
    }
    let mut parked = 0;
    for entry in WalkDir::new(&mods_dir).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !file_name.to_lowercase().ends_with(".dll") {
            continue;
        }
        let disabled = path.with_file_name(format!("{file_name}.disabled"));
        if !disabled.exists() && std::fs::rename(path, disabled).is_ok() {
            parked += 1;
        }
    }
    parked
}

/// Mirrors the profile's files into the game folder, replacing whatever BLMP
/// deployed last time. Disabled mods are not deployed, and loose DLLs that were
/// already in the game folder are parked so profiles stay self-contained.
pub fn deploy(
    profile_dir: &Path,
    game_dir: &Path,
    profile: &Profile,
) -> Result<DeployResult, String> {
    if !game_dir.is_dir() {
        return Err(format!("Game folder not found: {}", game_dir.display()));
    }
    let previous = read_manifest(game_dir);
    for rel in &previous {
        if let Some(clean) = util::clean_rel(rel) {
            let _ = std::fs::remove_file(game_dir.join(clean));
        }
    }

    // Anything still in Mods at this point came from outside BLMP. Park it so
    // this profile does not inherit mods installed directly into the game.
    park_loose_mods(game_dir);

    let disabled: HashSet<String> = profile
        .mods
        .iter()
        .filter(|entry| !entry.enabled)
        .flat_map(|entry| entry.files.iter().cloned())
        .collect();

    let mut deployed: Vec<String> = Vec::new();
    for entry in WalkDir::new(profile_dir).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(rel) = entry.path().strip_prefix(profile_dir) else {
            continue;
        };
        let rel_text = rel.to_string_lossy().replace('\\', "/");
        if rel_text == "profile.json"
            || rel_text.starts_with('.')
            || rel_text.ends_with(".blmp")
        {
            continue;
        }
        if disabled.contains(&rel_text) {
            continue;
        }
        let Some(clean) = util::clean_rel(&rel_text) else {
            continue;
        };
        let target = game_dir.join(&clean);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::copy(entry.path(), &target)
            .map_err(|e| format!("Could not deploy {rel_text}: {e}"))?;
        deployed.push(rel_text);
    }

    write_manifest(game_dir, &deployed)?;
    util::prune_empty_dirs(&game_dir.join("Mods"));

    Ok(DeployResult {
        deployed: deployed.len(),
        removed: previous.len(),
    })
}

/// Toggles the MelonLoader proxy DLLs. With them off, BONELAB starts vanilla.
#[cfg(not(target_os = "android"))]
pub fn set_proxies(game_dir: &Path, enabled: bool) -> Result<(), String> {
    for name in ["version.dll", "winhttp.dll"] {
        let live = game_dir.join(name);
        let parked = game_dir.join(format!("{name}.blmp-off"));
        if enabled {
            if parked.exists() && !live.exists() {
                std::fs::rename(&parked, &live)
                    .map_err(|e| format!("Could not re-enable MelonLoader: {e}"))?;
            }
        } else if live.exists() {
            let _ = std::fs::remove_file(&parked);
            std::fs::rename(&live, &parked)
                .map_err(|e| format!("Could not disable MelonLoader: {e}"))?;
        }
    }
    Ok(())
}

/// Quest builds do not use proxy DLLs; LemonLoader patches the Android game.
#[cfg(target_os = "android")]
pub fn set_proxies(_game_dir: &Path, _enabled: bool) -> Result<(), String> {
    Ok(())
}

pub fn spawn(exe: &Path, game_dir: &Path) -> Result<u32, String> {
    let child = Command::new(exe)
        .current_dir(game_dir)
        .spawn()
        .map_err(|e| format!("Could not start BONELAB: {e}"))?;
    Ok(child.id())
}

/// Starts the game vanilla and re-enables MelonLoader once it exits.
pub fn spawn_and_restore(exe: &Path, game_dir: &Path) -> Result<u32, String> {
    let mut child = match Command::new(exe).current_dir(game_dir).spawn() {
        Ok(child) => child,
        Err(error) => {
            // Never leave MelonLoader parked because the game failed to start.
            let _ = set_proxies(game_dir, true);
            return Err(format!("Could not start BONELAB: {error}"));
        }
    };
    let id = child.id();
    let dir = game_dir.to_path_buf();
    std::thread::spawn(move || {
        let _ = child.wait();
        let _ = set_proxies(&dir, true);
    });
    Ok(id)
}

