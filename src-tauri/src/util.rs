use std::path::{Path, PathBuf};

pub fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn now_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Lowercase, dash separated id usable as a folder name.
pub fn slug(input: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "profile".into()
    } else {
        out.chars().take(36).collect()
    }
}

pub fn new_profile_id(name: &str) -> String {
    format!("{}-{:x}", slug(name), now_millis())
}

/// Rejects absolute paths, drive letters and `..` segments.
pub fn clean_rel(raw: &str) -> Option<PathBuf> {
    let normalized = raw.replace('\\', "/");
    let mut out = PathBuf::new();
    for part in normalized.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." || part.contains(':') {
            return None;
        }
        out.push(part);
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

const KEEP_ROOTS: [&str; 9] = [
    "mods",
    "userdata",
    "melonloader",
    "plugins",
    "userlibs",
    "dependencies",
    "il2cppassemblies",
    "dotnet",
    "config",
];

const META_FILES: [&str; 9] = [
    "manifest.json",
    "icon.png",
    "readme.md",
    "changelog.md",
    "license",
    "license.md",
    "license.txt",
    "notice.txt",
    ".gitignore",
];

/// Where a file from a Thunderstore package belongs inside a profile.
pub fn package_target(rel: &Path) -> Option<PathBuf> {
    let text = rel.to_string_lossy().replace('\\', "/");
    let lower = text.to_lowercase();
    let base = lower.rsplit('/').next().unwrap_or("").to_string();
    let nested = lower.contains('/');

    if !nested && META_FILES.contains(&base.as_str()) {
        return None;
    }

    let first = lower.split('/').next().unwrap_or("");
    if KEEP_ROOTS.contains(&first) {
        return Some(rel.to_path_buf());
    }
    Some(PathBuf::from("Mods").join(rel))
}

/// Extracts a package zip into `dest`, placing loose files under Mods/.
/// Returns the written paths, relative to `dest`, with forward slashes.
pub fn extract_package(bytes: &[u8], dest: &Path) -> Result<Vec<String>, String> {
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("Bad zip file: {e}"))?;
    let mut written = Vec::new();

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|e| e.to_string())?;
        if entry.is_dir() {
            continue;
        }
        let Some(rel) = clean_rel(entry.name()) else {
            continue;
        };
        let Some(target) = package_target(&rel) else {
            continue;
        };
        let out_path = dest.join(&target);
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut file = std::fs::File::create(&out_path)
            .map_err(|e| format!("Could not write {}: {e}", out_path.display()))?;
        std::io::copy(&mut entry, &mut file).map_err(|e| e.to_string())?;
        written.push(target.to_string_lossy().replace('\\', "/"));
    }

    if written.is_empty() {
        return Err("The package did not contain any files.".into());
    }
    Ok(written)
}

/// Extracts every entry of a zip into `dest` verbatim (sanitized).
pub fn extract_zip_raw(bytes: &[u8], dest: &Path) -> Result<(), String> {
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("Bad zip file: {e}"))?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|e| e.to_string())?;
        if entry.is_dir() {
            continue;
        }
        let Some(rel) = clean_rel(entry.name()) else {
            continue;
        };
        let out_path = dest.join(&rel);
        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut file = std::fs::File::create(&out_path)
            .map_err(|e| format!("Could not write {}: {e}", out_path.display()))?;
        std::io::copy(&mut entry, &mut file).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Removes empty directories below `root` (best effort).
pub fn prune_empty_dirs(root: &Path) {
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                prune_empty_dirs(&path);
                let _ = std::fs::remove_dir(&path);
            }
        }
    }
}
