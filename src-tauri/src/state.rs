use crate::settings::{self, Settings};
use crate::thunderstore::ApiPackage;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

pub struct AppState {
    pub http: reqwest::Client,
    pub settings_path: PathBuf,
    pub settings: Mutex<Settings>,
    pub packages: Mutex<Option<Vec<ApiPackage>>>,
    pub cache_path: PathBuf,
}

impl AppState {
    pub fn new(app: &tauri::AppHandle) -> Result<Self, String> {
        use tauri::Manager;
        let config_dir = app
            .path()
            .app_config_dir()
            .map_err(|e| format!("Could not resolve config folder: {e}"))?;
        std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;

        let settings_path = config_dir.join("settings.json");
        let settings = settings::load(&settings_path, app);
        if let Err(error) = std::fs::create_dir_all(&settings.profiles_dir) {
            #[cfg(not(target_os = "android"))]
            return Err(format!(
                "Could not create profiles folder {}: {error}",
                settings.profiles_dir
            ));
            #[cfg(target_os = "android")]
            eprintln!(
                "BLMP: profiles folder {} is waiting for storage access ({error})",
                settings.profiles_dir
            );
        }

        let http = reqwest::Client::builder()
            .user_agent("BLMP/0.1 (BONELAB mod manager)")
            .timeout(Duration::from_secs(120))
            .connect_timeout(Duration::from_secs(20))
            .build()
            .map_err(|e| format!("Could not create HTTP client: {e}"))?;

        Ok(Self {
            http,
            settings_path,
            settings: Mutex::new(settings),
            packages: Mutex::new(None),
            cache_path: config_dir.join("packages-cache-v2.json"),
        })
    }

    pub fn snapshot(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    pub fn set_settings(&self, settings: Settings) -> Result<(), String> {
        settings::save(&self.settings_path, &settings)?;
        *self.settings.lock().unwrap() = settings;
        Ok(())
    }

    pub fn profiles_root(&self) -> PathBuf {
        PathBuf::from(self.snapshot().profiles_dir)
    }
}
