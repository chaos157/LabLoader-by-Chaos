use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Folder that contains BONELAB_*_Windows64.exe
    pub game_dir: String,
    /// Folder that holds every profile folder
    pub profiles_dir: String,
    /// Theme id, see src/lib/themes.ts
    pub theme: String,
    /// Currently active profile id
    pub active_profile: String,
    /// MelonLoader version last installed by BLMP
    pub melonloader_version: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            game_dir: String::new(),
            profiles_dir: String::new(),
            theme: "midnight".into(),
            active_profile: String::new(),
            melonloader_version: String::new(),
        }
    }
}

pub fn default_profiles_dir(app: &tauri::AppHandle) -> PathBuf {
    #[cfg(target_os = "android")]
    {
        let _ = app;
        return PathBuf::from("/storage/emulated/0/BLMP/Profiles");
    }
    #[cfg(not(target_os = "android"))]
    {
        use tauri::Manager;
        app.path()
            .document_dir()
            .map(|d| d.join("BLMP").join("Profiles"))
            .unwrap_or_else(|_| PathBuf::from("BLMP").join("Profiles"))
    }
}

pub fn load(path: &Path, app: &tauri::AppHandle) -> Settings {
    let mut settings = std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<Settings>(&text).ok())
        .unwrap_or_default();
    if settings.profiles_dir.trim().is_empty() {
        settings.profiles_dir = default_profiles_dir(app).to_string_lossy().to_string();
    }
    if settings.theme.trim().is_empty() {
        settings.theme = "midnight".into();
    }
    settings
}

pub fn save(path: &Path, settings: &Settings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Could not create config folder: {e}"))?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| format!("Could not save settings: {e}"))
}
