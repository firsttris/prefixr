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
        }
    }
}

/// Loads the script for `targets`, written to `dir` (a path KWin can read:
/// in a Flatpak, the app's directories under `~/.var/app` are the same on the
/// host). `None` if KWin isn't there or refused it.
pub(super) fn move_game_windows(dir: &Path, targets: &[String]) -> Option<Guard> {
    let mut games = GAMES.lock().unwrap_or_else(|e| e.into_inner());
    fs::create_dir_all(dir).ok()?;
    let path = dir.join(format!("{PLUGIN}.js"));
    fs::write(&path, script(targets)).ok()?;
    // A later launch's monitor wins, for the windows still to come.
    if *games > 0 {
        let _ = unload();
    }
    // On a refusal the running games keep their guards, and so their count:
    // the last of them unloads whatever is loaded by then.
    load(&path)?;
    *games += 1;
    Some(Guard)
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

/// Unloads a script a crashed Prefixr left loaded, at start.
pub fn unload_leftover() {
    let _ = unload();
}
