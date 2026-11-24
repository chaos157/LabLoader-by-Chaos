use crate::game;
use crate::melonloader;
use crate::mobile;
use crate::profiles::{self, ModEntry, Profile};
use crate::settings::Settings;
use crate::state::AppState;
use crate::thunderstore::{self, ApiPackage, ApiVersion};
use crate::util;
use serde::Serialize;
use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::State;
use walkdir::WalkDir;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetResult {
    pub path: String,
    pub found: bool,
    pub message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MelonStatus {
    pub installed: String,
    pub latest: String,
    pub game_ready: bool,
    pub message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallOutcome {
    pub installed: usize,
    pub message: String,
    pub melon_loader: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProfileResult {
    pub profile: Profile,
    pub melon_loader: String,
    pub message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchResult {
    pub pid: u32,
    pub exe: String,
    pub vanilla: bool,
    pub message: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InstalledItem {
    #[serde(flatten)]
    pub entry: ModEntry,
    pub deployed: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LooseMod {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub disabled: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledView {
    pub mods: Vec<InstalledItem>,
    pub loose: Vec<LooseMod>,
    pub game_dir: String,
}

// ---------------------------------------------------------------- settings

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.snapshot()
}

#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: Settings) -> Result<Settings, String> {
    let mut next = settings;
    if next.profiles_dir.trim().is_empty() {
        return Err("Choose a folder for your profiles.".into());
    }
    std::fs::create_dir_all(&next.profiles_dir)
        .map_err(|e| format!("Could not use that profiles folder: {e}"))?;
    if !next.game_dir.trim().is_empty() {
        let resolved = game::resolve_dir(&next.game_dir)?;
        next.game_dir = resolved.to_string_lossy().to_string();
    }
    state.set_settings(next.clone())?;
    Ok(next)
}

#[cfg(target_os = "android")]
fn detect_mobile_game() -> PresetResult {
    if !mobile::is_storage_granted() {
        return PresetResult {
            path: String::new(),
            found: false,
            message: "Allow LabLoader all-files access before scanning /storage/emulated/0.".into(),
        };
    }
    if let Err(error) = mobile::ensure_blmp_layout() {
        return PresetResult {
            path: String::new(),
            found: false,
            message: error,
        };
    }
    let loaders = mobile::scan_loaders();
    if let Some(path) = loaders.first() {
        let _ = mobile::ensure_loader_layout(path);
        let extra = if loaders.len() > 1 {
            format!(" Found {} loader folders; using the first.", loaders.len())
        } else {
            String::new()
        };
        return PresetResult {
            path: path.to_string_lossy().to_string(),
            found: true,
            message: format!("Found {}.{extra}", path.display()),
        };
    }
    PresetResult {
        path: String::new(),
        found: false,
        message: format!(
            "No top-level MelonLoader or LemonLoader folder in {}. LabLoader skips Android/, Downloads/ and com.* folders.",
            mobile::STORAGE_ROOT
        ),
    }
}

#[cfg(not(target_os = "android"))]
fn detect_desktop_game(launcher: &str) -> PresetResult {
    let found = if launcher.eq_ignore_ascii_case("oculus") {
        game::detect_oculus()
    } else {
        game::detect_steam()
    };
    match found {
        Some(path) => PresetResult {
            path: path.to_string_lossy().to_string(),
            found: true,
            message: format!(
                "Found BONELAB in the {} library.",
                if launcher.eq_ignore_ascii_case("oculus") {
                    "Meta Horizon"
                } else {
                    "Steam"
                }
            ),
        },
        None => PresetResult {
            path: String::new(),
            found: false,
            message: if launcher.eq_ignore_ascii_case("oculus") {
                "No Meta Horizon install found. Point LabLoader at the folder with BONELAB_Oculus_Windows64.exe."
                    .into()
            } else {
                "No Steam install found. Point LabLoader at the folder with BONELAB_Steam_Windows64.exe."
                    .into()
            },
        },
    }
}

#[tauri::command]
pub fn detect_game(launcher: String) -> PresetResult {
    #[cfg(target_os = "android")]
    {
        let _ = launcher;
        detect_mobile_game()
    }
    #[cfg(not(target_os = "android"))]
    {
        detect_desktop_game(&launcher)
    }
}

#[tauri::command]
pub fn storage_status(state: State<'_, AppState>) -> mobile::StorageStatus {
    #[cfg(target_os = "android")]
    if mobile::is_storage_granted() && mobile::ensure_blmp_layout().is_ok() {
        let mut settings = state.snapshot();
        if settings.game_dir.trim().is_empty() {
            if let Some(loader) = mobile::detect_loader() {
                let _ = mobile::ensure_loader_layout(&loader);
                settings.game_dir = loader.to_string_lossy().to_string();
                let _ = state.set_settings(settings);
            }
        } else if let Ok(loader) = game::resolve_dir(&settings.game_dir) {
            let _ = mobile::ensure_loader_layout(&loader);
        }
    }
    mobile::status(&state.snapshot().game_dir)
}

#[tauri::command]
pub fn request_storage_access() -> Result<bool, String> {
    mobile::request_storage_access()?;
    Ok(mobile::is_storage_granted())
}

#[tauri::command]
pub fn get_platform() -> String {
    std::env::consts::OS.to_string()
}

// -------------------------------------------------------------- thunderstore

#[tauri::command]
pub async fn fetch_packages(
    state: State<'_, AppState>,
    force: Option<bool>,
) -> Result<Vec<thunderstore::PackageSummary>, String> {
    let packages = thunderstore::load(state.inner(), force.unwrap_or(false)).await?;
    let mut summaries: Vec<thunderstore::PackageSummary> =
        packages.iter().map(thunderstore::summarize).collect();
    summaries.sort_by(|a, b| b.downloads.cmp(&a.downloads));
    Ok(summaries)
}

#[tauri::command]
pub async fn fetch_package_detail(
    state: State<'_, AppState>,
    namespace: String,
    name: String,
) -> Result<thunderstore::PackageDetail, String> {
    thunderstore::detail(state.inner(), &namespace, &name).await
}

// ------------------------------------------------------------------ profiles

#[tauri::command]
pub fn list_profiles(state: State<'_, AppState>) -> Vec<Profile> {
    let root = state.profiles_root();
    let _ = std::fs::create_dir_all(&root);
    let mut profiles = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if let Some(id) = path.file_name().and_then(|n| n.to_str()) {
                if let Ok(profile) = profiles::load(&root, id) {
                    profiles.push(profile);
                }
            }
        }
    }
    profiles.sort_by_key(|profile| profile.created_at);
    profiles
}

#[tauri::command]
pub async fn create_profile(
    state: State<'_, AppState>,
    name: String,
) -> Result<CreateProfileResult, String> {
    let settings = state.snapshot();
    let root = PathBuf::from(&settings.profiles_dir);
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let profile = profiles::create(&root, &name)?;

    if settings.active_profile.is_empty() {
        let mut next = settings.clone();
        next.active_profile = profile.id.clone();
        let _ = state.set_settings(next);
    }

    #[allow(unused_mut)]
    let mut melon_loader = String::new();
    let mut message = "Profile created. Add mods from the Library.".to_string();
    #[cfg(target_os = "android")]
    {
        if settings.game_dir.trim().is_empty() {
            message.push_str(
                " Scan your Quest for a MelonLoader or LemonLoader folder in Settings.",
            );
        } else if let Ok(game_dir) = game::resolve_dir(&settings.game_dir) {
            let _ = mobile::ensure_loader_layout(&game_dir);
            message = format!("Profile created. Mods deploy to {}.", game_dir.display());
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        if settings.game_dir.trim().is_empty() {
            message.push_str(" Set your BONELAB folder in Settings so MelonLoader can be installed.");
        } else if let Ok(game_dir) = game::resolve_dir(&settings.game_dir) {
            match melonloader::ensure(&state.http, &game_dir, false).await {
                Ok(tag) => {
                    melon_loader = tag.clone();
                    message = format!("Profile created and MelonLoader {tag} is installed.");
                    let mut next = state.snapshot();
                    next.melonloader_version = tag;
                    let _ = state.set_settings(next);
                }
                Err(error) => {
                    message = format!("Profile created, but MelonLoader could not be installed: {error}");
                }
            }
        }
    }

    Ok(CreateProfileResult {
        profile,
        melon_loader,
        message,
    })
}

#[tauri::command]
pub fn delete_profile(state: State<'_, AppState>, profile_id: String) -> Result<(), String> {
    let settings = state.snapshot();
    let root = PathBuf::from(&settings.profiles_dir);
    if settings.active_profile == profile_id {
        if !settings.game_dir.trim().is_empty() {
            if let Ok(game_dir) = game::resolve_dir(&settings.game_dir) {
                let _ = game::clear_deployed(&game_dir);
            }
        }
        let mut next = settings.clone();
        next.active_profile = String::new();
        let _ = state.set_settings(next);
    }
    profiles::remove(&root, &profile_id)
}

#[tauri::command]
pub fn set_active_profile(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<Profile, String> {
    let settings = state.snapshot();
    let root = PathBuf::from(&settings.profiles_dir);
    let profile = profiles::load(&root, &profile_id)?;
    let mut next = settings.clone();
    next.active_profile = profile.id.clone();
    state.set_settings(next.clone())?;
    if !next.game_dir.trim().is_empty() {
        if let Ok(game_dir) = game::resolve_dir(&next.game_dir) {
            let _ = game::deploy(&profiles::profile_dir(&root, &profile.id), &game_dir, &profile);
            let _ = game::set_proxies(&game_dir, true);
        }
    }
    Ok(profile)
}

#[tauri::command]
pub fn redeploy_profile(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<game::DeployResult, String> {
    let settings = state.snapshot();
    let root = PathBuf::from(&settings.profiles_dir);
    let profile = profiles::load(&root, &profile_id)?;
    let game_dir = game::resolve_dir(&settings.game_dir)?;
    game::set_proxies(&game_dir, true)?;
    game::deploy(&profiles::profile_dir(&root, &profile_id), &game_dir, &profile)
}

// -------------------------------------------------------------------- mods

fn find_version<'a>(package: &'a ApiPackage, version: &str) -> Option<&'a ApiVersion> {
    package
        .versions
        .iter()
        .find(|candidate| candidate.version_number == version)
        .or_else(|| thunderstore::latest_version(package))
}

fn resolve_targets(
    packages: &[ApiPackage],
    namespace: &str,
    name: &str,
    version: &str,
) -> Result<(Vec<(ApiPackage, ApiVersion)>, Vec<String>), String> {
    let mut order: Vec<(ApiPackage, ApiVersion)> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut queue: Vec<(String, String, String)> = vec![(
        namespace.to_string(),
        name.to_string(),
        version.to_string(),
    )];

    while let Some((ns, package_name, wanted)) = queue.pop() {
        let key = format!("{ns}-{package_name}").to_lowercase();
        if !seen.insert(key) {
            continue;
        }
        let Some(package) = packages
            .iter()
            .find(|p| p.owner.eq_ignore_ascii_case(&ns) && p.name.eq_ignore_ascii_case(&package_name))
        else {
            warnings.push(format!("{ns}/{package_name} is not available."));
            continue;
        };
        let Some(chosen) = find_version(package, &wanted) else {
            warnings.push(format!("{ns}/{package_name} has no installable version."));
            continue;
        };
        for dependency in &chosen.dependencies {
            if let Some((dep_ns, dep_name, dep_version)) =
                thunderstore::parse_dependency(dependency)
            {
                if dep_ns.eq_ignore_ascii_case("LavaGang")
                    && dep_name.eq_ignore_ascii_case("MelonLoader")
                {
                    continue;
                }
                queue.push((dep_ns, dep_name, dep_version));
            }
        }
        order.push((package.clone(), chosen.clone()));
    }

    Ok((order, warnings))
}

#[tauri::command]
pub async fn install_package(
    state: State<'_, AppState>,
    profile_id: String,
    namespace: String,
    name: String,
    version: String,
) -> Result<InstallOutcome, String> {
    let settings = state.snapshot();
    if namespace.eq_ignore_ascii_case("LavaGang") && name.eq_ignore_ascii_case("MelonLoader") {
        #[cfg(target_os = "android")]
        {
            return Err(
                "MelonLoader is PC-only. Install LemonLoader on the Quest, patch BONELAB, then scan for its folder in Settings."
                    .into(),
            );
        }
        #[cfg(not(target_os = "android"))]
        {
            let game_dir = game::resolve_dir(&settings.game_dir)?;
            let tag = melonloader::ensure(&state.http, &game_dir, true).await?;
            return Ok(InstallOutcome {
                installed: 0,
                message: format!("MelonLoader {tag} is installed in BONELAB."),
                melon_loader: tag,
            });
        }
    }

    let root = PathBuf::from(&settings.profiles_dir);
    let mut profile = profiles::load(&root, &profile_id)?;
    let packages = thunderstore::load(state.inner(), false).await?;
    let (targets, warnings) = resolve_targets(&packages, &namespace, &name, &version)?;
    if targets.is_empty() {
        return Err("Nothing could be installed for this package.".into());
    }

    let profile_path = profiles::profile_dir(&root, &profile_id);
    let mut installed = 0usize;

    for (package, chosen) in targets {
        let bytes = thunderstore::download(&state.http, &chosen.download_url).await?;
        let previous_files = profile
            .mods
            .iter()
            .find(|entry| {
                entry.namespace.eq_ignore_ascii_case(&package.owner)
                    && entry.name.eq_ignore_ascii_case(&package.name)
            })
            .map(|entry| entry.files.clone());
        if let Some(files) = previous_files {
            profiles::remove_files(&profile_path, &files);
        }
        let files = util::extract_package(&bytes, &profile_path)?;
        profile.mods.retain(|entry| {
            !(entry.namespace.eq_ignore_ascii_case(&package.owner)
                && entry.name.eq_ignore_ascii_case(&package.name))
        });
        profile.mods.push(ModEntry {
            id: format!("{}-{}", package.owner, package.name),
            name: package.name.clone(),
            namespace: package.owner.clone(),
            source: "thunderstore".into(),
            version: chosen.version_number.clone(),
            icon: chosen.icon.clone(),
            enabled: true,
            files,
            installed_at: util::now_secs(),
        });
        installed += 1;
    }

    profiles::save(&root, &profile)?;

    let mut message = format!("Installed {installed} package(s).");
    if !warnings.is_empty() {
        message.push(' ');
        message.push_str(&warnings.join(" "));
    }

    if settings.active_profile == profile_id && !settings.game_dir.trim().is_empty() {
        if let Ok(game_dir) = game::resolve_dir(&settings.game_dir) {
            match game::deploy(&profile_path, &game_dir, &profile) {
                Ok(_) => message.push_str(" Deployed to BONELAB."),
                Err(error) => message.push_str(&format!(" Could not deploy: {error}")),
            }
            let _ = game::set_proxies(&game_dir, true);
        }
    }

    #[allow(unused_mut)]
    let mut melon_loader = String::new();
    #[cfg(not(target_os = "android"))]
    if !settings.game_dir.trim().is_empty() {
        if let Ok(game_dir) = game::resolve_dir(&settings.game_dir) {
            if let Ok(tag) = melonloader::ensure(&state.http, &game_dir, false).await {
                melon_loader = tag;
            }
        }
    }

    Ok(InstallOutcome {
        installed,
        message,
        melon_loader,
    })
}

fn safe_file_name(raw: &str) -> String {
    let base = Path::new(raw)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "mod.dll".into());
    let cleaned: String = base
        .chars()
        .map(|c| if "<>:\"/\\|?*".contains(c) { '_' } else { c })
        .collect();
    if cleaned.trim().is_empty() {
        "mod.dll".into()
    } else {
        cleaned
    }
}

#[tauri::command]
pub async fn install_custom_mod(
    state: State<'_, AppState>,
    profile_id: String,
    name: String,
    source: String,
) -> Result<InstallOutcome, String> {
    let settings = state.snapshot();
    let root = PathBuf::from(&settings.profiles_dir);
    let mut profile = profiles::load(&root, &profile_id)?;
    let profile_path = profiles::profile_dir(&root, &profile_id);
    let trimmed = source.trim().to_string();
    if trimmed.is_empty() {
        return Err("Choose a mod file or paste a download link.".into());
    }

    let remote = trimmed.starts_with("http://") || trimmed.starts_with("https://");
    let file_name = if remote {
        let without_query = trimmed.split(['?', '#']).next().unwrap_or(&trimmed);
        safe_file_name(without_query.rsplit('/').next().unwrap_or("mod.dll"))
    } else {
        safe_file_name(&trimmed)
    };
    let is_zip = file_name.to_lowercase().ends_with(".zip");
    let display_name = if name.trim().is_empty() {
        file_name
            .trim_end_matches(".dll")
            .trim_end_matches(".zip")
            .to_string()
    } else {
        name.trim().to_string()
    };

    let files = if remote {
        let bytes = thunderstore::download(&state.http, &trimmed).await?;
        if is_zip {
            util::extract_package(&bytes, &profile_path)?
        } else {
            let rel = format!("Mods/{file_name}");
            let target = profile_path.join("Mods").join(&file_name);
            std::fs::create_dir_all(target.parent().unwrap_or(&profile_path))
                .map_err(|e| e.to_string())?;
            std::fs::write(&target, bytes).map_err(|e| format!("Could not save mod: {e}"))?;
            vec![rel]
        }
    } else {
        let source_path = PathBuf::from(&trimmed);
        if !source_path.is_file() {
            return Err("That mod file does not exist.".into());
        }
        if is_zip {
            let bytes = std::fs::read(&source_path).map_err(|e| e.to_string())?;
            util::extract_package(&bytes, &profile_path)?
        } else {
            let rel = format!("Mods/{file_name}");
            let target = profile_path.join("Mods").join(&file_name);
            std::fs::create_dir_all(target.parent().unwrap_or(&profile_path))
                .map_err(|e| e.to_string())?;
            std::fs::copy(&source_path, &target)
                .map_err(|e| format!("Could not copy mod: {e}"))?;
            vec![rel]
        }
    };

    let id = format!("custom-{}", util::slug(&display_name));
    let previous_files = profile
        .mods
        .iter()
        .find(|entry| entry.id == id)
        .map(|entry| entry.files.clone());
    if let Some(files) = previous_files {
        profiles::remove_files(&profile_path, &files);
    }
    profile.mods.retain(|entry| entry.id != id);
    profile.mods.push(ModEntry {
        id,
        name: display_name.clone(),
        namespace: "Custom".into(),
        source: "custom".into(),
        version: String::new(),
        icon: String::new(),
        enabled: true,
        files,
        installed_at: util::now_secs(),
    });
    profiles::save(&root, &profile)?;

    let mut message = format!("Added {display_name}.");
    if settings.active_profile == profile_id && !settings.game_dir.trim().is_empty() {
        if let Ok(game_dir) = game::resolve_dir(&settings.game_dir) {
            match game::deploy(&profile_path, &game_dir, &profile) {
                Ok(_) => message.push_str(" Deployed to BONELAB."),
                Err(error) => message.push_str(&format!(" Could not deploy: {error}")),
            }
            let _ = game::set_proxies(&game_dir, true);
        }
    }

    Ok(InstallOutcome {
        installed: 1,
        message,
        melon_loader: String::new(),
    })
}

#[tauri::command]
pub fn uninstall_mod(
    state: State<'_, AppState>,
    profile_id: String,
    mod_id: String,
) -> Result<(), String> {
    let settings = state.snapshot();
    let root = PathBuf::from(&settings.profiles_dir);
    let mut profile = profiles::load(&root, &profile_id)?;
    let profile_path = profiles::profile_dir(&root, &profile_id);

    let removed: Vec<String> = profile
        .mods
        .iter()
        .filter(|entry| entry.id == mod_id)
        .flat_map(|entry| entry.files.clone())
        .collect();
    profile.mods.retain(|entry| entry.id != mod_id);
    profiles::remove_files(&profile_path, &removed);
    profiles::save(&root, &profile)?;

    if settings.active_profile == profile_id && !settings.game_dir.trim().is_empty() {
        if let Ok(game_dir) = game::resolve_dir(&settings.game_dir) {
            let _ = game::deploy(&profile_path, &game_dir, &profile);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn set_mod_enabled(
    state: State<'_, AppState>,
    profile_id: String,
    mod_id: String,
    enabled: bool,
) -> Result<(), String> {
    let settings = state.snapshot();
    let root = PathBuf::from(&settings.profiles_dir);
    let mut profile = profiles::load(&root, &profile_id)?;
    if let Some(entry) = profile.mods.iter_mut().find(|entry| entry.id == mod_id) {
        entry.enabled = enabled;
    }
    profiles::save(&root, &profile)?;
    if settings.active_profile == profile_id && !settings.game_dir.trim().is_empty() {
        if let Ok(game_dir) = game::resolve_dir(&settings.game_dir) {
            game::deploy(&profiles::profile_dir(&root, &profile_id), &game_dir, &profile)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn list_installed(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<InstalledView, String> {
    let settings = state.snapshot();
    let root = PathBuf::from(&settings.profiles_dir);
    let profile = profiles::load(&root, &profile_id)?;
    let game_dir = if settings.game_dir.trim().is_empty() {
        None
    } else {
        game::resolve_dir(&settings.game_dir).ok()
    };

    let tracked: HashSet<String> = profile
        .mods
        .iter()
        .flat_map(|entry| entry.files.iter().cloned())
        .collect();

    let mods = profile
        .mods
        .iter()
        .map(|entry| InstalledItem {
            deployed: game_dir
                .as_ref()
                .map(|dir| {
                    entry.enabled
                        && !entry.files.is_empty()
                        && entry.files.iter().all(|file| dir.join(file).exists())
                })
                .unwrap_or(false),
            entry: entry.clone(),
        })
        .collect();

    let mut loose = Vec::new();
    if let Some(dir) = &game_dir {
        let mods_dir = dir.join("Mods");
        for file in WalkDir::new(&mods_dir).into_iter().filter_map(Result::ok) {
            if !file.file_type().is_file() {
                continue;
            }
            let Ok(relative) = file.path().strip_prefix(dir) else {
                continue;
            };
            let relative = relative.to_string_lossy().replace('\\', "/");
            if tracked.contains(&relative) {
                continue;
            }
            let lower = relative.to_lowercase();
            if !lower.ends_with(".dll") && !lower.ends_with(".dll.disabled") {
                continue;
            }
            let disabled = lower.ends_with(".disabled");
            let file_name = file.file_name().to_string_lossy().to_string();
            let display = file_name
                .trim_end_matches(".disabled")
                .trim_end_matches(".dll")
                .to_string();
            loose.push(LooseMod {
                name: display,
                path: relative,
                size: file.metadata().map(|meta| meta.len()).unwrap_or(0),
                disabled,
            });
        }
    }
    loose.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(InstalledView {
        mods,
        loose,
        game_dir: settings.game_dir,
    })
}

fn game_mods_guard(state: &AppState, relative: &str) -> Result<PathBuf, String> {
    let settings = state.snapshot();
    let game_dir = game::resolve_dir(&settings.game_dir)?;
    let clean = util::clean_rel(relative).ok_or("Invalid mod path.")?;
    if !clean.to_string_lossy().to_lowercase().starts_with("mods") {
        return Err("Only files inside the Mods folder can be changed.".into());
    }
    Ok(game_dir.join(clean))
}

#[tauri::command]
pub fn delete_loose_mod(state: State<'_, AppState>, relative_path: String) -> Result<(), String> {
    let target = game_mods_guard(state.inner(), &relative_path)?;
    if target.is_dir() {
        std::fs::remove_dir_all(&target).map_err(|e| e.to_string())
    } else if target.exists() {
        std::fs::remove_file(&target).map_err(|e| e.to_string())
    } else {
        Err("That mod file is already gone.".into())
    }
}

#[tauri::command]
pub fn set_loose_enabled(
    state: State<'_, AppState>,
    relative_path: String,
    enabled: bool,
) -> Result<(), String> {
    let target = game_mods_guard(state.inner(), &relative_path)?;
    let text = target.to_string_lossy().to_string();
    if enabled {
        if !text.to_lowercase().ends_with(".disabled") {
            return Ok(());
        }
        let restored = PathBuf::from(text.trim_end_matches(".disabled"));
        std::fs::rename(&target, restored).map_err(|e| e.to_string())
    } else {
        if text.to_lowercase().ends_with(".disabled") || !target.exists() {
            return Ok(());
        }
        let parked = PathBuf::from(format!("{text}.disabled"));
        std::fs::rename(&target, parked).map_err(|e| e.to_string())
    }
}

// ------------------------------------------------------- profile import/export

fn zip_options() -> zip::write::SimpleFileOptions {
    zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
}

#[tauri::command]
pub fn export_profile(
    state: State<'_, AppState>,
    profile_id: String,
    destination: String,
) -> Result<String, String> {
    let root = state.profiles_root();
    let profile = profiles::load(&root, &profile_id)?;
    let profile_dir = profiles::profile_dir(&root, &profile_id);
    let mut destination = PathBuf::from(destination.trim());
    #[cfg(target_os = "android")]
    if destination.as_os_str().is_empty() {
        mobile::ensure_blmp_layout()?;
        destination = mobile::export_dir().join(format!("{}.blmp", util::slug(&profile.name)));
    }
    if destination.is_dir() {
        let file_name = format!("{}.blmp", util::slug(&profile.name));
        destination = destination.join(file_name);
    }
    if destination.extension().is_none() {
        destination.set_extension("blmp");
    }
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let file = std::fs::File::create(&destination)
        .map_err(|e| format!("Could not create the profile file: {e}"))?;
    let mut zip = zip::ZipWriter::new(file);

    let json = serde_json::to_vec_pretty(&profile).map_err(|e| e.to_string())?;
    zip.start_file("profile.json", zip_options())
        .map_err(|e| e.to_string())?;
    zip.write_all(&json).map_err(|e| e.to_string())?;

    for file in WalkDir::new(&profile_dir).into_iter().filter_map(Result::ok) {
        if !file.file_type().is_file() {
            continue;
        }
        let Ok(relative) = file.path().strip_prefix(&profile_dir) else {
            continue;
        };
        let relative = relative.to_string_lossy().replace('\\', "/");
        if relative == "profile.json" {
            continue;
        }
        zip.start_file(relative.clone(), zip_options())
            .map_err(|e| e.to_string())?;
        let bytes = std::fs::read(file.path()).map_err(|e| e.to_string())?;
        zip.write_all(&bytes).map_err(|e| e.to_string())?;
    }

    zip.finish().map_err(|e| e.to_string())?;
    Ok(destination.to_string_lossy().to_string())
}

#[tauri::command]
pub fn import_profile(state: State<'_, AppState>, source: String) -> Result<Profile, String> {
    let root = state.profiles_root();
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let source_path = if source.trim().is_empty() {
        mobile::newest_import().ok_or_else(|| {
            "No .blmp file found. Put one in /storage/emulated/0/BLMP/Imports and try again."
                .to_string()
        })?
    } else {
        PathBuf::from(source.trim())
    };
    let bytes =
        std::fs::read(&source_path).map_err(|e| format!("Could not read that file: {e}"))?;
    let existing: Vec<Profile> = {
        let mut list = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&root) {
            for entry in entries.flatten() {
                if let Some(id) = entry.file_name().to_str() {
                    if let Ok(profile) = profiles::load(&root, id) {
                        list.push(profile);
                    }
                }
            }
        }
        list
    };

    let temp_id = format!("import-{}", util::now_millis());
    let temp_dir = root.join(&temp_id);
    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    if let Err(error) = util::extract_zip_raw(&bytes, &temp_dir) {
        let _ = std::fs::remove_dir_all(&temp_dir);
        return Err(error);
    }

    let manifest_path = temp_dir.join("profile.json");
    let loaded: Profile = match std::fs::read_to_string(&manifest_path)
        .ok()
        .and_then(|text| serde_json::from_str::<Profile>(&text).ok())
    {
        Some(profile) => profile,
        None => {
            let _ = std::fs::remove_dir_all(&temp_dir);
            return Err(
                "That file is not a BLMP profile (profile.json is missing).".to_string(),
            );
        }
    };

    let base_name = if loaded.name.trim().is_empty() {
        "Imported profile".to_string()
    } else {
        loaded.name.clone()
    };
    let new_name = profiles::unique_name(&existing, &base_name);
    let new_id = profiles::unique_id(&root, &new_name);
    let target = root.join(&new_id);
    std::fs::rename(&temp_dir, &target)
        .map_err(|e| format!("Could not import the profile: {e}"))?;

    let profile = Profile {
        format: profiles::FORMAT.into(),
        id: new_id,
        name: new_name,
        created_at: util::now_secs(),
        mods: loaded.mods,
    };
    profiles::save(&root, &profile)?;
    let _ = std::fs::create_dir_all(target.join("Mods"));
    Ok(profile)
}

// -------------------------------------------------------------- melonloader

#[cfg(target_os = "android")]
#[tauri::command]
pub async fn melonloader_status(state: State<'_, AppState>) -> Result<MelonStatus, String> {
    let settings = state.snapshot();
    let loader = if settings.game_dir.trim().is_empty() {
        None
    } else {
        game::resolve_dir(&settings.game_dir).ok()
    };
    let installed = loader
        .as_ref()
        .map(|dir| {
            let name = dir
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();
            if name.contains("lemonloader") {
                "LemonLoader".to_string()
            } else {
                "MelonLoader folder".to_string()
            }
        })
        .unwrap_or_default();
    Ok(MelonStatus {
        installed: installed.clone(),
        latest: String::new(),
        game_ready: !settings.game_dir.trim().is_empty(),
        message: if installed.is_empty() {
            "Grant storage access and scan for LemonLoader in Settings.".into()
        } else {
            "LemonLoader patches BONELAB on the headset; LabLoader manages its Mods, Plugins, UserLibs and UserData folders."
                .into()
        },
    })
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn melonloader_status(state: State<'_, AppState>) -> Result<MelonStatus, String> {
    let settings = state.snapshot();
    let installed = if settings.game_dir.trim().is_empty() {
        String::new()
    } else {
        game::resolve_dir(&settings.game_dir)
            .ok()
            .and_then(|dir| melonloader::installed_version(&dir))
            .unwrap_or_default()
    };
    let (latest, message) = match melonloader::latest(&state.http).await {
        Ok((tag, _)) => (tag, String::new()),
        Err(error) => (String::new(), error),
    };
    Ok(MelonStatus {
        installed,
        latest,
        game_ready: !settings.game_dir.trim().is_empty(),
        message,
    })
}

#[cfg(target_os = "android")]
#[tauri::command]
pub async fn install_melonloader(_state: State<'_, AppState>) -> Result<MelonStatus, String> {
    Err(
        "MelonLoader is PC-only. Install LemonLoader on the Quest, patch BONELAB, then tap Scan in Settings."
            .into(),
    )
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn install_melonloader(state: State<'_, AppState>) -> Result<MelonStatus, String> {
    let settings = state.snapshot();
    let game_dir = game::resolve_dir(&settings.game_dir)?;
    let tag = melonloader::ensure(&state.http, &game_dir, true).await?;
    let mut next = state.snapshot();
    next.melonloader_version = tag.clone();
    let _ = state.set_settings(next);
    Ok(MelonStatus {
        installed: tag.clone(),
        latest: tag,
        game_ready: true,
        message: "MelonLoader is up to date.".into(),
    })
}

// ----------------------------------------------------------------- launching

#[cfg(target_os = "android")]
#[tauri::command]
pub async fn launch_game(
    state: State<'_, AppState>,
    profile_id: Option<String>,
    vanilla: bool,
) -> Result<LaunchResult, String> {
    let settings = state.snapshot();
    let game_dir = game::resolve_dir(&settings.game_dir)?;

    if vanilla {
        game::clear_deployed(&game_dir)?;
        game::park_loose_mods(&game_dir);
        mobile::launch_bonelab()?;
        return Ok(LaunchResult {
            pid: 0,
            exe: mobile::BONELAB_PACKAGE.into(),
            vanilla: true,
            message: "BONELAB started without this profile. Loose Mods DLLs were parked as .disabled."
                .into(),
        });
    }

    let id = profile_id
        .filter(|id| !id.is_empty())
        .or_else(|| {
            if settings.active_profile.is_empty() {
                None
            } else {
                Some(settings.active_profile.clone())
            }
        })
        .ok_or("Create a profile before launching with mods.")?;

    let root = PathBuf::from(&settings.profiles_dir);
    let profile = profiles::load(&root, &id)?;
    let profile_path = profiles::profile_dir(&root, &id);
    game::deploy(&profile_path, &game_dir, &profile)?;
    mobile::ensure_loader_layout(&game_dir)?;
    mobile::launch_bonelab()?;
    Ok(LaunchResult {
        pid: 0,
        exe: mobile::BONELAB_PACKAGE.into(),
        vanilla: false,
        message: format!("Launching {} · LemonLoader is loading the profile.", profile.name),
    })
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub async fn launch_game(
    state: State<'_, AppState>,
    profile_id: Option<String>,
    vanilla: bool,
) -> Result<LaunchResult, String> {
    let settings = state.snapshot();
    let game_dir = game::resolve_dir(&settings.game_dir)?;
    let exe = game::find_exe(&game_dir).ok_or_else(|| {
        format!(
            "Could not find BONELAB in {}. Choose the folder with BONELAB_Steam_Windows64.exe or BONELAB_Oculus_Windows64.exe.",
            game_dir.display()
        )
    })?;

    if vanilla {
        game::set_proxies(&game_dir, false)?;
        let pid = game::spawn_and_restore(&exe, &game_dir)?;
        return Ok(LaunchResult {
            pid,
            exe: exe.to_string_lossy().to_string(),
            vanilla: true,
            message: "BONELAB started without mods.".into(),
        });
    }

    let id = profile_id
        .filter(|id| !id.is_empty())
        .or_else(|| {
            if settings.active_profile.is_empty() {
                None
            } else {
                Some(settings.active_profile.clone())
            }
        })
        .ok_or("Create a profile before launching with mods.")?;

    let root = PathBuf::from(&settings.profiles_dir);
    let profile = profiles::load(&root, &id)?;
    let profile_path = profiles::profile_dir(&root, &id);
    game::deploy(&profile_path, &game_dir, &profile)?;
    game::set_proxies(&game_dir, true)?;

    let note = match melonloader::ensure(&state.http, &game_dir, false).await {
        Ok(tag) => format!("MelonLoader {tag} ready."),
        Err(error) => format!("Warning: MelonLoader could not be verified ({error})."),
    };
    let pid = game::spawn(&exe, &game_dir)?;
    Ok(LaunchResult {
        pid,
        exe: exe.to_string_lossy().to_string(),
        vanilla: false,
        message: format!("Launching {} · {note}", profile.name),
    })
}

// --------------------------------------------------------------- external links

#[cfg(target_os = "android")]
fn open_external(target: &str) -> Result<(), String> {
    mobile::open_url(target)
}

#[cfg(not(target_os = "android"))]
fn open_external(target: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        std::process::Command::new("rundll32.exe")
            .arg("url.dll,FileProtocolHandler")
            .arg(target)
            .spawn()
            .map_err(|e| format!("Could not open it: {e}"))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
        std::process::Command::new(opener)
            .arg(target)
            .spawn()
            .map_err(|e| format!("Could not open it: {e}"))?;
        Ok(())
    }
}

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err("Only web links can be opened.".into());
    }
    open_external(&url)
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    let target = PathBuf::from(path);
    if !target.exists() {
        return Err("That folder does not exist yet.".into());
    }
    if cfg!(target_os = "android") {
        return Err(format!(
            "Open a file manager and go to {}.",
            target.display()
        ));
    }
    open_external(&target.to_string_lossy())
}
