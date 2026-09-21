mod commands;
mod config;
mod models;
mod tray;

use tauri::{Manager, WindowEvent};

use commands::games::{
    add_game, create_desktop_shortcut, kill_game, launch_game, list_games, remove_game,
    take_pending_launch, update_game, PendingLaunch, RunningGames,
};
use commands::mangohud::{get_mangohud_config, save_mangohud_config};
use commands::prefixes::{add_prefix, delete_prefix, list_prefixes};
use commands::runner_downloads::{download_runner, list_runner_releases, list_runner_sources};
use commands::runners::list_runners;
use config::load_config;
use tray::{hide_main_window, rebuild_tray_menu, setup_tray, WindowVisible};

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
            app.manage(RunningGames::default());
            app.manage(WindowVisible::new(true));
            setup_tray(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window only hides it — the app keeps running in the
            // tray so a launched game (and the ability to kill it if it hangs)
            // isn't tied to the window staying open. "Beenden" in the tray
            // menu is the actual quit.
            if window.label() == "main" {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    hide_main_window(window.app_handle());
                    rebuild_tray_menu(window.app_handle());
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_runners,
            list_games,
            add_game,
            update_game,
            remove_game,
            launch_game,
            kill_game,
            take_pending_launch,
            create_desktop_shortcut,
            add_prefix,
            delete_prefix,
            list_prefixes,
            list_runner_sources,
            list_runner_releases,
            download_runner,
            get_mangohud_config,
            save_mangohud_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
