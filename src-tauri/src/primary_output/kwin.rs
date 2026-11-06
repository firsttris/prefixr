//! KWin on Wayland: KWin sets the Xwayland primary output to Plasma's own
//! primary again on every RandR change (`XrandrEventFilter` in KWin's
//! `src/xwayland/xwayland.cpp`), so switching it doesn't hold there. Instead,
//! a KWin script moves the game's windows to the monitor while games run;
//! KWin then makes a fullscreen window fill that monitor. Plasma's primary
//! monitor and its panels stay as they are. Works the same with Wine's X11
//! and Wayland drivers, as long as the monitors have the same resolution
//! (Wine still thinks it's on its primary one).

use std::fs;
use std::path::Path;
use std::sync::Mutex;

/// The script's name in KWin. One at a time: a later launch replaces it.
const PLUGIN: &str = "prefixr-launch-monitor";

/// How many running games the loaded script is for.
static GAMES: Mutex<usize> = Mutex::new(0);

/// The bus connection owning `owner_name()` while this process's script is
/// loaded. Another Prefixr (a Steam shortcut's `--run` is a process of its
/// own) then leaves the script alone at its start, and doesn't load its own.
static OWNER: Mutex<Option<zbus::Connection>> = Mutex::new(None);

/// Below the app ID: a Flatpak may own those names.
fn owner_name() -> String {
    let app_id =
        crate::host_env::flatpak_id().unwrap_or_else(|| "io.github.firsttris.Prefixr".into());
    format!("{app_id}.LaunchMonitor")
}

/// Takes `owner_name()` for this process. `None` if another one holds it.
fn hold_owner_name() -> Option<()> {
    let builder = zbus::connection::Builder::session()
        .name(owner_name())
        .replace_existing_names(false)
        .allow_name_replacements(false);
    let connection = zbus::block_on(builder.build()).ok()?;
    *OWNER.lock().unwrap_or_else(|e| e.into_inner()) = Some(connection);
    Some(())
}

fn release_owner_name() {
    let owner = OWNER.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some(connection) = owner {
        let _ = zbus::block_on(connection.release_name(owner_name()));
    }
}

/// Whether a Prefixr process (this one or another) holds the script.
/// `true` if that can't be told, so nothing is unloaded on a guess.
fn owner_running() -> bool {
    zbus::block_on(async {
        let connection = zbus::Connection::session().await?;
        let dbus = zbus::fdo::DBusProxy::new(&connection).await?;
        let name = zbus::names::BusName::try_from(owner_name())?;
        dbus.name_has_owner(name).await
    })
    .unwrap_or(true)
}

/// The KWin script for moving windows to the first of `targets` (a monitor's
/// connector name, e.g. `DP-2`, or its model) that KWin knows.
pub(super) fn script(targets: &[String]) -> String {
    let targets = serde_json::to_string(targets).unwrap_or_else(|_| "[]".into());
    format!(
        r#"// Written by Prefixr while games run: moves them to the monitor Prefixr is on.
const targets = {targets};

function target() {{
    const outputs = workspace.screens;
    for (const wanted of targets) {{
        const output = outputs.find(o => o.name === wanted)
            || outputs.find(o => o.model === wanted);
        if (output) {{
            return output;
        }}
    }}
    return null;
}}

// Wine names a window's class after the exe (game.exe), Proton after the
// Steam app id (steam_app_…).
function isGame(window) {{
    const cls = String(window.resourceClass || "").toLowerCase();
    const name = String(window.resourceName || "").toLowerCase();
    return cls.endsWith(".exe") || name.endsWith(".exe") || cls.startsWith("steam_app_");
}}

function move(window) {{
    const output = target();
    if (output && !(window.output && window.output.name === output.name)) {{
        workspace.sendClientToScreen(window, output);
    }}
}}

workspace.windowAdded.connect(function (window) {{
    if (!isGame(window)) {{
        return;
    }}
    move(window);
    // A game that goes fullscreen later may first have put itself on Wine's
    // primary monitor.
    window.fullScreenChanged.connect(function () {{
        if (window.fullScreen) {{
            move(window);
        }}
    }});
}});
"#
    )
}

/// One launch's part, ended when dropped: the script is unloaded once the
/// last game it was loaded for exits.
pub(super) struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        let mut games = GAMES.lock().unwrap_or_else(|e| e.into_inner());
        *games = games.saturating_sub(1);
        if *games == 0 {
            let _ = unload();
            release_owner_name();
        }
    }
}

/// Loads the script for `targets`, written to `dir` (a path KWin can read:
/// in a Flatpak, the app's directories under `~/.var/app` are the same on the
/// host). `None` if KWin isn't there or refused it.
pub(super) fn move_game_windows(dir: &Path, targets: &[String]) -> Option<Guard> {
    let mut games = GAMES.lock().unwrap_or_else(|e| e.into_inner());
    if *games == 0 {
        hold_owner_name()?;
    }
    let loaded = load_script(dir, targets, *games > 0);
    if loaded.is_none() && *games == 0 {
        release_owner_name();
    }
    loaded?;
    *games += 1;
    Some(Guard)
}

/// Writes the script and loads it, replacing a loaded one if `replace`.
fn load_script(dir: &Path, targets: &[String], replace: bool) -> Option<()> {
    fs::create_dir_all(dir).ok()?;
    let path = dir.join(format!("{PLUGIN}.js"));
    fs::write(&path, script(targets)).ok()?;
    // A later launch's monitor wins, for the windows still to come.
    if replace {
        let _ = unload();
    }
    // On a refusal the running games keep their guards, and so their count:
    // the last of them unloads whatever is loaded by then.
    load(&path)
}

fn load(path: &Path) -> Option<()> {
    let path = path.to_str()?.to_owned();
    zbus::block_on(async {
        let connection = zbus::Connection::session().await.ok()?;
        let id: i32 = connection
            .call_method(
                Some("org.kde.KWin"),
                "/Scripting",
                Some("org.kde.kwin.Scripting"),
                "loadScript",
                &(path, PLUGIN),
            )
            .await
            .ok()?
            .body()
            .deserialize()
            .ok()?;
        if id < 0 {
            return None;
        }
        connection
            .call_method(
                Some("org.kde.KWin"),
                format!("/Scripting/Script{id}").as_str(),
                Some("org.kde.kwin.Script"),
                "run",
                &(),
            )
            .await
            .ok()?;
        Some(())
    })
}

fn unload() -> Option<()> {
    zbus::block_on(async {
        let connection = zbus::Connection::session().await.ok()?;
        connection
            .call_method(
                Some("org.kde.KWin"),
                "/Scripting",
                Some("org.kde.kwin.Scripting"),
                "unloadScript",
                &PLUGIN,
            )
            .await
            .ok()?;
        Some(())
    })
}

/// Unloads a script a crashed Prefixr left loaded, at start: one no running
/// Prefixr holds.
pub fn unload_leftover() {
    if !owner_running() {
        let _ = unload();
    }
}
