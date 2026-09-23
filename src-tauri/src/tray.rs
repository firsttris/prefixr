use std::sync::atomic::{AtomicBool, Ordering};

use futures_util::future::join_all;
use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use uuid::Uuid;

use crate::commands::games::{kill_running_game, LaunchingGames, RunningGames};

pub const TRAY_ID: &str = "main-tray";
const MAIN_WINDOW: &str = "main";
const TOGGLE_ID: &str = "toggle-window";
const QUIT_ID: &str = "quit";
const KILL_PREFIX: &str = "kill-game:";

/// Tracks whether the main window is meant to be visible, kept in sync
/// eagerly (see `set_main_window_visible`) rather than read back from
/// `window.is_visible()`: on Linux, `hide()`/`show()` only queue a request
/// that GTK's event loop applies on its next iteration, so querying it right
/// after calling either one can still report the pre-call state — which made
/// the tray menu's toggle label lag a step behind and look inverted.
pub struct WindowVisible(AtomicBool);

impl WindowVisible {
    pub fn new(visible: bool) -> Self {
        Self(AtomicBool::new(visible))
    }
}

/// Whether a tray host is there to show the tray icon. GNOME without the
/// AppIndicator extension has none: the icon then silently doesn't appear,
/// and a window closed "into the tray" could only be brought back by
/// starting Prefixr again. See `lib.rs`'s close handling.
pub struct TrayAvailable(pub bool);

/// Asks the session bus whether a StatusNotifier host (what Tauri's tray
/// icon registers with) is running. `gdbus` ships with GLib, which Prefixr
/// needs anyway; if it can't answer, a tray is assumed as before.
pub fn tray_host_available() -> bool {
    let output = std::process::Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--timeout",
            "2",
            "--dest",
            "org.freedesktop.DBus",
            "--object-path",
            "/org/freedesktop/DBus",
            "--method",
            "org.freedesktop.DBus.NameHasOwner",
            "org.kde.StatusNotifierWatcher",
        ])
        .output();
    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).contains("true"),
        _ => true,
    }
}

fn set_main_window_visible(app: &AppHandle, visible: bool) {
    app.state::<WindowVisible>().0.store(visible, Ordering::SeqCst);
}

fn is_main_window_visible(app: &AppHandle) -> bool {
    app.state::<WindowVisible>().0.load(Ordering::SeqCst)
}

/// Shows (and unminimizes) the main window and gives it focus.
///
/// Two Linux-specific quirks stack here:
///
/// - `show()`/`unminimize()` don't apply immediately — they just queue a
///   request that tao's GTK event loop processes on its next iteration —
///   while `set_focus()` bails out right away if the window isn't visible
///   *yet*. So the actual focus call always has to be deferred a beat to let
///   the show land first.
/// - On Wayland compositors with focus-stealing prevention (confirmed with
///   KWin), a window that's already mapped but just not the active one is
///   *not* allowed to raise+focus itself — the compositor only flashes its
///   taskbar entry instead. A window that's currently unmapped and gets
///   (re-)shown is treated as freshly opening, which is allowed. Hiding
///   first unconditionally, even if already visible, makes every call look
///   like that permitted case.
pub fn show_and_focus(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    let _ = window.hide();
    let _ = window.unminimize();
    let _ = window.show();
    set_main_window_visible(app, true);

    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
            let _ = window.set_focus();
        }
    });
}

/// Hides the main window, keeping `WindowVisible` in sync. Used both by the
/// tray toggle and by `lib.rs`'s close-button handler, so the tray menu's
/// label is correct no matter how the window was hidden.
pub fn hide_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        let _ = window.hide();
    }
    set_main_window_visible(app, false);
}

fn toggle_main_window(app: &AppHandle) {
    if is_main_window_visible(app) {
        hide_main_window(app);
    } else {
        show_and_focus(app);
    }
}

/// Builds the tray's menu from scratch: a show/hide toggle, one "beenden"
/// entry per currently running game (so a hung Proton process can be killed
/// without needing the main window), and quit.
fn build_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let toggle_text = if is_main_window_visible(app) {
        "Fenster verstecken"
    } else {
        "Fenster anzeigen"
    };

    let builder =
        MenuBuilder::new(app).item(&MenuItemBuilder::with_id(TOGGLE_ID, toggle_text).build(app)?);

    let running_games = app
        .state::<RunningGames>()
        .0
        .lock()
        .map(|games| {
            let mut games: Vec<_> = games.iter().map(|(id, g)| (*id, g.name.clone())).collect();
            games.sort_by(|a, b| a.1.cmp(&b.1));
            games
        })
        .unwrap_or_default();

    let builder = if running_games.is_empty() {
        builder
    } else {
        let mut builder = builder.separator();
        for (id, name) in running_games {
            let item = MenuItemBuilder::with_id(format!("{KILL_PREFIX}{id}"), format!("„{name}“ beenden (erzwingen)"))
                .build(app)?;
            builder = builder.item(&item);
        }
        builder
    };

    builder
        .separator()
        .item(&MenuItemBuilder::with_id(QUIT_ID, "Beenden").build(app)?)
        .build()
}

/// Recomputes and applies the tray menu; call this whenever a game starts or
/// stops, or the main window's visibility changes, so the tray stays in sync.
pub fn rebuild_tray_menu(app: &AppHandle) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    if let Ok(menu) = build_menu(app) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn handle_menu_event(app: &AppHandle, id: &str) {
    if id == TOGGLE_ID {
        toggle_main_window(app);
        rebuild_tray_menu(app);
        return;
    }
    if id == QUIT_ID {
        quit(app);
        return;
    }
    if let Some(game_id) = id.strip_prefix(KILL_PREFIX) {
        if let Ok(game_id) = Uuid::parse_str(game_id) {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let running = app.state::<RunningGames>();
                let _ = kill_running_game(&running, game_id).await;
            });
        }
    }
}

/// Quits Prefixr — after asking, while games are launching or running.
/// They'd keep running without it (each in a process group of its own), but
/// a restarted Prefixr wouldn't know about them anymore and so couldn't end
/// them either; confirming ends them first.
fn quit(app: &AppHandle) {
    let active = app.state::<LaunchingGames>().ids().len();
    if active == 0 {
        app.exit(0);
        return;
    }
    let text = match active {
        1 => "Ein Spiel läuft noch oder wird gerade gestartet.".to_string(),
        n => format!("{n} Spiele laufen noch oder werden gerade gestartet."),
    };
    let app = app.clone();
    app.dialog()
        .message(format!(
            "{text} Beim Beenden von Prefixr werden sie ebenfalls beendet — ungespeicherter \
             Fortschritt geht dabei verloren."
        ))
        .title("Prefixr beenden?")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Spiele und Prefixr beenden".to_string(),
            "Abbrechen".to_string(),
        ))
        .show(move |confirmed| {
            if !confirmed {
                return;
            }
            tauri::async_runtime::spawn(async move {
                let running = app.state::<RunningGames>();
                let ids: Vec<Uuid> = running
                    .0
                    .lock()
                    .map(|games| games.keys().copied().collect())
                    .unwrap_or_default();
                join_all(ids.into_iter().map(|id| kill_running_game(&running, id))).await;
                app.exit(0);
            });
        });
}

/// Creates the tray icon shown for the app's whole lifetime. Closing the main
/// window only hides it (see `lib.rs`'s `on_window_event`); this menu is what
/// lets the user get it back, or kill a hung game's Proton process, without it.
pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let icon = app.default_window_icon().cloned();
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Prefixr")
        .menu(&build_menu(app)?)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            handle_menu_event(app, event.id().as_ref());
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_main_window(tray.app_handle());
                rebuild_tray_menu(tray.app_handle());
            }
        });
    if let Some(icon) = icon {
        builder = builder.icon(icon);
    }
    builder.build(app)?;
    Ok(())
}
