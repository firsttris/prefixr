mod commands;
mod config;
mod models;

use tauri::Manager;

use commands::games::{add_game, launch_game, list_games, remove_game, update_game};
use commands::prefixes::{add_prefix, delete_prefix, list_prefixes};
use commands::runner_downloads::{download_runner, list_proton_ge_releases};
use commands::runners::list_runners;
use config::load_config;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let config = load_config(app.handle())?;
            app.manage(std::sync::Mutex::new(config));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_runners,
            list_games,
            add_game,
            update_game,
            remove_game,
            launch_game,
            add_prefix,
            delete_prefix,
            list_prefixes,
            list_proton_ge_releases,
            download_runner
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
