mod commands;
mod config;
mod models;

use tauri::Manager;

use commands::games::{
    add_game, create_desktop_shortcut, launch_game, list_games, remove_game, take_pending_launch,
    update_game, PendingLaunch,
};
use commands::prefixes::{add_prefix, delete_prefix, list_prefixes};
use commands::runner_downloads::{download_runner, list_runner_releases, list_runner_sources};
use commands::runners::list_runners;
use config::load_config;

/// Looks for `--launch <game-id>` among the process args, as invoked by a
/// desktop shortcut created via `create_desktop_shortcut`.
fn find_launch_arg() -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.windows(2)
        .find(|pair| pair[0] == "--launch")
        .map(|pair| pair[1].clone())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let config = load_config(app.handle())?;
            app.manage(std::sync::Mutex::new(config));
            app.manage(PendingLaunch(std::sync::Mutex::new(find_launch_arg())));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_runners,
            list_games,
            add_game,
            update_game,
            remove_game,
            launch_game,
            take_pending_launch,
            create_desktop_shortcut,
            add_prefix,
            delete_prefix,
            list_prefixes,
            list_runner_sources,
            list_runner_releases,
            download_runner
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
