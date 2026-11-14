//! Vaxelium desktop client: a Tauri window over the chat and auth services. The page is in `ui/`;
//! the networking and the account live here.

mod auth_client;
mod commands;
mod discover;
mod keychain;
mod net;
mod state;
mod trust;

use tauri::Manager;

fn main() {
    common::env::load();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            std::fs::create_dir_all(&dir)?;
            trust::init(dir.join("pins.json"));
            app.manage(state::AppState::new(app.handle().clone(), dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::prefs_load,
            commands::prefs_save,
            commands::emoji_catalog,
            commands::restore_session,
            commands::login,
            commands::logout,
            commands::profile,
            commands::update_profile,
            commands::change_email,
            commands::connect_main,
            commands::disconnect_main,
            commands::send_main,
            commands::send_dm,
            commands::dm_connect_all,
            commands::dm_disconnect,
            commands::discover,
            commands::pick_server_icon,
            commands::trust_forget,
            commands::trust_forget_auth,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Vaxelium");
}
