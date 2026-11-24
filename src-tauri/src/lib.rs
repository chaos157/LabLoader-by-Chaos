mod commands;
mod game;
mod melonloader;
mod mobile;
mod profiles;
mod settings;
mod state;
mod thunderstore;
mod util;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state = AppState::new(app.handle())?;
            #[cfg(target_os = "android")]
            {
                if mobile::is_storage_granted() {
                    let _ = mobile::ensure_blmp_layout();
                    let mut settings = state.snapshot();
                    if settings.game_dir.trim().is_empty() {
                        if let Some(loader) = mobile::detect_loader() {
                            let _ = mobile::ensure_loader_layout(&loader);
                            settings.game_dir = loader.to_string_lossy().to_string();
                            let _ = state.set_settings(settings);
                        }
                    }
                }
            }
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::detect_game,
            commands::fetch_packages,
            commands::fetch_package_detail,
            commands::list_profiles,
            commands::create_profile,
            commands::delete_profile,
            commands::set_active_profile,
            commands::redeploy_profile,
            commands::install_package,
            commands::install_custom_mod,
            commands::uninstall_mod,
            commands::set_mod_enabled,
            commands::list_installed,
            commands::delete_loose_mod,
            commands::set_loose_enabled,
            commands::export_profile,
            commands::import_profile,
            commands::melonloader_status,
            commands::install_melonloader,
            commands::launch_game,
            commands::open_url,
            commands::open_path,
            commands::storage_status,
            commands::request_storage_access,
            commands::get_platform
        ])
        .run(tauri::generate_context!())
        .expect("error while running BLMP");
}
