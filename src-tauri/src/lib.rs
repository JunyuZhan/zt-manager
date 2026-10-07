pub mod agent;
mod auth;
pub mod central;
mod commands;
pub mod secure;
pub mod service;

use commands::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_config_dir()?;
            std::fs::create_dir_all(&dir)?;
            let path = dir.join("settings.json");
            app.manage(AppState::load(path));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings_load,
            commands::settings_save,
            commands::auth_login,
            commands::auth_logout,
            commands::auth_status,
            commands::central_detect,
            commands::service_query,
            commands::service_control,
            commands::agent_status,
            commands::agent_networks,
            commands::agent_network_detail,
            commands::agent_join,
            commands::agent_leave,
            commands::agent_peers,
            commands::central_networks,
            commands::central_network,
            commands::central_create_network,
            commands::central_delete_network,
            commands::central_update_network,
            commands::central_members,
            commands::central_update_member,
            commands::central_member_action,
            commands::central_delete_member,
            commands::central_orgs,
            commands::controller_networks,
            commands::controller_network,
            commands::controller_create,
            commands::controller_update,
            commands::controller_members,
            commands::controller_update_member,
            commands::ping_host,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
