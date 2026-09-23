mod commands;
mod config;
mod http;
mod models;
mod tray;

use std::path::Path;

use tauri::{Emitter, Manager, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use commands::games::{
    add_game, create_desktop_shortcut, create_menu_shortcut, ensure_install_desktop_entry,
    kill_game, launch_game, launch_game_headless, list_active_games, list_games, remove_game,
    run_installer, take_pending_install, take_pending_launch, update_game, LaunchingGames,
    PendingInstall, PendingLaunch, RunningGames,
};
use commands::github::{get_github_config, save_github_config};
use commands::graphics::{get_graphics_config, save_graphics_config};
use commands::mangohud::{get_mangohud_config, save_mangohud_config};
use commands::performance::{
    check_max_map_count, fix_max_map_count, get_performance_config, save_performance_config,
};
use commands::prefixes::{add_prefix, delete_prefix, list_prefixes};
use commands::proton_options::{get_proton_config, list_proton_options, save_proton_config};
use commands::runner_downloads::{download_runner, list_runner_releases, list_runner_sources};
use commands::runners::list_runners;
use commands::steam::{export_to_steam, list_steam_games, remove_from_steam};
use commands::steamgriddb::{
    get_game_cover, get_game_icon, get_steamgriddb_config, list_steamgriddb_artwork,
    list_steamgriddb_grids, list_steamgriddb_icons, remove_game_artwork, remove_game_cover,
    remove_game_icon, save_steamgriddb_config, search_steamgriddb_games, set_game_artwork,
    set_game_cover, set_game_icon,
};
use commands::umu::{get_umu_status, install_umu};
use commands::umu_database::search_umu_ids;
use commands::wine_tools::launch_wine_tool;
use commands::winetricks::{
    install_winetricks_verbs, list_all_winetricks_verbs, list_installed_winetricks_verbs,
};
use config::load_config;
use tray::{
    hide_main_window, rebuild_tray_menu, setup_tray, show_and_focus, tray_host_available,
    TrayAvailable, WindowVisible,
};

/// Checks the real (not effective) UID via `/proc/self/status`, mirroring
/// what PortProton's own launcher script checks via `id -u`. A prefix set up
/// or run as root ends up with root-owned files inside it, which the user's
/// normal desktop session then can't use or clean up without `sudo` — worth
/// catching before it happens rather than after. Batocera intentionally runs
/// its whole userspace as root, so it's exempted the same way PortProton
/// exempts it.
fn running_as_root() -> bool {
    if Path::new("/userdata/system/batocera.conf").exists() {
        return false;
    }
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
        return false;
    };
    status
        .lines()
        .find_map(|line| line.strip_prefix("Uid:"))
        .and_then(|rest| rest.split_whitespace().next())
        .is_some_and(|real_uid| real_uid == "0")
}

/// Looks for `--launch <game-id>` among the process args, as invoked by a
/// shortcut created via `create_desktop_shortcut` or `create_menu_shortcut`.
fn find_launch_arg(args: &[String]) -> Option<String> {
    find_flag_arg(args, "--launch")
}

/// Looks for `--run <game-id>` among the process args, as invoked by Steam
/// for a game exported to it (see `commands::steam`): the game then runs
/// without any window, and the app exits with it.
fn find_run_arg(args: &[String]) -> Option<String> {
    find_flag_arg(args, "--run")
}

/// Looks for `--install <exe-path>` among the process args, as invoked by the
/// "Mit Prefixr installieren" file-manager context menu entry (see
/// `ensure_install_desktop_entry`).
fn find_install_arg(args: &[String]) -> Option<String> {
    find_flag_arg(args, "--install")
}

fn find_flag_arg(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == flag)
        .map(|pair| pair[1].clone())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    let run_game_id = find_run_arg(&args);

    let mut builder = tauri::Builder::default();
    // A `--run` game runs in a process of its own, next to an open Prefixr,
    // so it stays out of the single-instance handoff.
    if run_game_id.is_none() {
        // Must be the first plugin registered (see tauri-plugin-single-instance's
        // own docs) — this is what makes a second "Mit Prefixr installieren"
        // click or a desktop shortcut, while Prefixr is already running, hand
        // its `--install <path>` or `--launch <id>` off to this instance
        // instead of opening a second window.
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if let Some(game_id) = find_launch_arg(&argv) {
                let _ = app.emit("pending-launch", game_id);
                return;
            }
            let Some(exe_path) = find_install_arg(&argv) else {
                show_and_focus(app);
                rebuild_tray_menu(app);
                return;
            };
            if let Some(state) = app.try_state::<PendingInstall>() {
                if let Ok(mut pending) = state.0.lock() {
                    *pending = Some(exe_path.clone());
                }
            }
            let _ = app.emit("pending-install", exe_path);
            show_and_focus(app);
            rebuild_tray_menu(app);
        }));
    }
    builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            if running_as_root() {
                app.dialog()
                    .message(
                        "Ein als root angelegter oder gestarteter Wine-Prefix gehört danach \
                         root statt dir — deine normale Sitzung kann ihn dann oft nicht mehr \
                         verwenden oder ohne sudo löschen. Bitte Prefixr als normaler Benutzer \
                         starten.",
                    )
                    .title("Prefixr nicht als root ausführen")
                    .kind(MessageDialogKind::Error)
                    .blocking_show();
                std::process::exit(1);
            }

            // A config that can't be read is left exactly as it is: starting
            // with an empty one instead would overwrite the user's library
            // on the next save.
            let config = match load_config(app.handle()) {
                Ok(config) => config,
                Err(message) => {
                    app.dialog()
                        .message(format!(
                            "{message}\n\nPrefixr lässt die Datei unverändert. Repariere oder \
                             entferne sie und starte Prefixr dann neu."
                        ))
                        .title("Konfiguration konnte nicht gelesen werden")
                        .kind(MessageDialogKind::Error)
                        .blocking_show();
                    std::process::exit(1);
                }
            };
            app.manage(std::sync::Mutex::new(config));
            app.manage(PendingLaunch(std::sync::Mutex::new(find_launch_arg(&args))));
            app.manage(PendingInstall(std::sync::Mutex::new(find_install_arg(&args))));
            app.manage(RunningGames::default());
            app.manage(LaunchingGames::default());
            app.manage(WindowVisible::new(true));

            if let Some(game_id) = run_game_id {
                launch_game_headless(app.handle(), game_id);
                return Ok(());
            }
            // The window is created here rather than by Tauri itself (see
            // `"create": false` in tauri.conf.json), so a `--run` launch
            // never shows one.
            for window in app.config().app.windows.clone() {
                WebviewWindowBuilder::from_config(app.handle(), &window)?.build()?;
            }
            app.manage(TrayAvailable(tray_host_available()));
            setup_tray(app.handle())?;
            // Best-effort: a file manager's "Öffnen mit" context menu working
            // is a nice-to-have, not something worth failing startup over —
            // or delaying it for, since it waits on update-desktop-database.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn_blocking(move || {
                let _ = ensure_install_desktop_entry(&handle);
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window only hides it — the app keeps running in the
            // tray so a launched game (and the ability to kill it if it hangs)
            // isn't tied to the window staying open. "Beenden" in the tray
            // menu is the actual quit.
            //
            // Without a tray to come back from, closing quits as usual —
            // unless a game is launching or running, whose window is then
            // only minimized so it stays reachable to end the game.
            if window.label() != "main" {
                return;
            }
            let WindowEvent::CloseRequested { api, .. } = event else {
                return;
            };
            let app = window.app_handle();
            api.prevent_close();
            if app.state::<TrayAvailable>().0 {
                hide_main_window(app);
                rebuild_tray_menu(app);
            } else if !app.state::<LaunchingGames>().ids().is_empty() {
                let _ = window.minimize();
            } else {
                app.exit(0);
            }
        })
        .invoke_handler(tauri::generate_handler![
            list_runners,
            list_proton_options,
            list_games,
            add_game,
            update_game,
            remove_game,
            launch_game,
            kill_game,
            list_active_games,
            take_pending_launch,
            take_pending_install,
            run_installer,
            create_desktop_shortcut,
            create_menu_shortcut,
            export_to_steam,
            remove_from_steam,
            list_steam_games,
            add_prefix,
            delete_prefix,
            list_prefixes,
            list_runner_sources,
            list_runner_releases,
            download_runner,
            get_mangohud_config,
            save_mangohud_config,
            get_performance_config,
            save_performance_config,
            get_graphics_config,
            save_graphics_config,
            get_proton_config,
            save_proton_config,
            check_max_map_count,
            fix_max_map_count,
            install_winetricks_verbs,
            list_all_winetricks_verbs,
            list_installed_winetricks_verbs,
            launch_wine_tool,
            get_steamgriddb_config,
            save_steamgriddb_config,
            search_steamgriddb_games,
            list_steamgriddb_grids,
            list_steamgriddb_icons,
            set_game_cover,
            get_game_cover,
            remove_game_cover,
            set_game_icon,
            get_game_icon,
            remove_game_icon,
            list_steamgriddb_artwork,
            set_game_artwork,
            remove_game_artwork,
            get_github_config,
            save_github_config,
            get_umu_status,
            install_umu,
            search_umu_ids
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
