use crate::util;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const FORMAT: &str = "blmp/1";

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ModEntry {
    pub id: String,
    pub name: String,
    pub namespace: String,
    /// "thunderstore" or "custom"
    pub source: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default = "yes")]
    pub enabled: bool,
    /// File paths relative to the profile folder, e.g. "Mods/Thing.dll"
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub installed_at: i64,
}

fn yes() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    #[serde(default = "profile_format")]
    pub format: String,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub mods: Vec<ModEntry>,
}

fn profile_format() -> String {
    FORMAT.into()
}

pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && !id.contains('/')
        && !id.contains('\\')
        && !id.contains("..")
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn profile_dir(root: &Path, id: &str) -> std::path::PathBuf {
    root.join(id)
}

pub fn load(root: &Path, id: &str) -> Result<Profile, String> {
    if !valid_id(id) {
        return Err("Invalid profile id.".into());
    }
    let path = profile_dir(root, id).join("profile.json");
    let text = std::fs::read_to_string(&path)
        .map_err(|_| format!("Profile '{id}' could not be found."))?;
    let mut profile = serde_json::from_str::<Profile>(&text)
        .map_err(|e| format!("Profile '{id}' is corrupted: {e}"))?;
    if profile.id.is_empty() {
        profile.id = id.to_string();
    }
    if profile.format.is_empty() {
        profile.format = FORMAT.into();
    }
    Ok(profile)
}

pub fn save(root: &Path, profile: &Profile) -> Result<(), String> {
    if !valid_id(&profile.id) {
        return Err("Invalid profile id.".into());
    }
    let dir = profile_dir(root, &profile.id);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Could not create profile folder: {e}"))?;
    std::fs::create_dir_all(dir.join("Mods")).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(profile).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("profile.json"), text)
        .map_err(|e| format!("Could not save profile: {e}"))
}

fn collect(root: &Path) -> Vec<Profile> {
    let mut profiles = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(id) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if let Ok(profile) = load(root, id) {
                profiles.push(profile);
            }
        }
    }
    profiles.sort_by_key(|p| p.created_at);
    profiles
}

/// Removes a profile id that is not used yet.
pub fn unique_id(root: &Path, base: &str) -> String {
    let mut id = util::new_profile_id(base);
    let mut attempt = 2;
    while root.join(&id).exists() {
        id = format!("{}-{}", util::slug(base), attempt);
        attempt += 1;
    }
    id
}

pub fn unique_name(existing: &[Profile], wanted: &str) -> String {
    let names: Vec<String> = existing.iter().map(|p| p.name.to_lowercase()).collect();
    if !names.contains(&wanted.to_lowercase()) {
        return wanted.to_string();
    }
    let mut attempt = 2;
    loop {
        let candidate = format!("{wanted} ({attempt})");
        if !names.contains(&candidate.to_lowercase()) {
            return candidate;
        }
        attempt += 1;
    }
}

pub fn create(root: &Path, name: &str) -> Result<Profile, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Give your profile a name.".into());
    }
    if trimmed.chars().count() > 48 {
        return Err("Profile names are limited to 48 characters.".into());
    }
    let existing = collect(root);
    let id = unique_id(root, trimmed);
    let profile = Profile {
        format: FORMAT.into(),
        id,
        name: unique_name(&existing, trimmed),
        created_at: util::now_secs(),
        mods: Vec::new(),
    };
    std::fs::create_dir_all(profile_dir(root, &profile.id).join("Mods"))
        .map_err(|e| format!("Could not create profile folder: {e}"))?;
    save(root, &profile)?;
    Ok(profile)
}

pub fn remove(root: &Path, id: &str) -> Result<(), String> {
    if !valid_id(id) {
        return Err("Invalid profile id.".into());
    }
    let dir = profile_dir(root, id);
    if !dir.is_dir() {
        return Err("Profile not found.".into());
    }
    std::fs::remove_dir_all(&dir).map_err(|e| format!("Could not delete profile: {e}"))
}

/// Deletes files that belong to a profile's mod from the profile folder.
pub fn remove_files(profile_dir: &Path, files: &[String]) {
    for rel in files {
        if let Some(clean) = util::clean_rel(rel) {
            let _ = std::fs::remove_file(profile_dir.join(clean));
        }
    }
    util::prune_empty_dirs(&profile_dir.join("Mods"));
}

