mod commands;
mod config;
mod models;
mod tray;

use std::path::Path;

use tauri::{Emitter, Manager, WindowEvent};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use commands::games::{
    add_game, create_desktop_shortcut, create_menu_shortcut, ensure_install_desktop_entry,
    kill_game, launch_game, list_games, remove_game, run_installer, take_pending_install,
    take_pending_launch, update_game, PendingInstall, PendingLaunch, RunningGames,
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
use commands::steamgriddb::{
    get_game_cover, get_game_icon, get_steamgriddb_config, list_steamgriddb_grids,
    list_steamgriddb_icons, remove_game_cover, remove_game_icon, save_steamgriddb_config,
    search_steamgriddb_games, set_game_cover, set_game_icon,
};
use commands::umu::{get_umu_status, install_umu};
use commands::umu_database::search_umu_ids;
use commands::wine_tools::launch_wine_tool;
use commands::winetricks::{
    install_winetricks_verbs, list_all_winetricks_verbs, list_installed_winetricks_verbs,
};
use config::load_config;
use tray::{hide_main_window, rebuild_tray_menu, setup_tray, show_and_focus, WindowVisible};

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
fn find_launch_arg() -> Option<String> {
    find_flag_arg(&std::env::args().collect::<Vec<_>>(), "--launch")
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
    tauri::Builder::default()
        // Must be the first plugin registered (see tauri-plugin-single-instance's
        // own docs) — this is what makes a second "Mit Prefixr installieren"
        // click, while Prefixr is already running, hand its `--install <path>`
        // off to this instance instead of opening a second window.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
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
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
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

            let config = load_config(app.handle())?;
            app.manage(std::sync::Mutex::new(config));
            app.manage(PendingLaunch(std::sync::Mutex::new(find_launch_arg())));
            let install_args: Vec<String> = std::env::args().collect();
            app.manage(PendingInstall(std::sync::Mutex::new(find_install_arg(
                &install_args,
            ))));
            app.manage(RunningGames::default());
            app.manage(WindowVisible::new(true));
            setup_tray(app.handle())?;
            // Best-effort: a file manager's "Öffnen mit" context menu working
            // is a nice-to-have, not something worth failing startup over.
            let _ = ensure_install_desktop_entry(app.handle());
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
            list_proton_options,
            list_games,
            add_game,
            update_game,
            remove_game,
            launch_game,
            kill_game,
            take_pending_launch,
            take_pending_install,
            run_installer,
            create_desktop_shortcut,
            create_menu_shortcut,
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
            get_github_config,
            save_github_config,
            get_umu_status,
            install_umu,
            search_umu_ids
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
