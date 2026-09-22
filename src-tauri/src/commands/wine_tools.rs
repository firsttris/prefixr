use std::path::PathBuf;
use std::process::Stdio;

use tauri::State;

use crate::commands::games::steer_profile_to_steamuser;
use crate::commands::runners::{find_runner, runner_command, wine_binary};
use crate::config::ConfigState;

/// Wine's own built-in GUI utilities — the same set PortProton, Lutris and
/// Bottles all expose straight from their prefix view (winecfg, regedit, a
/// cmd shell, the wine-side file manager, its uninstaller and task manager).
/// Each is resolved by Wine itself as a "builtin" module purely by name, with
/// no on-disk exe required — which matters here because not every runner
/// ships a same-named binary next to `wine` (a plain Wine build does for some
/// of these, e.g. `winecfg`/`winefile`, but Proton runners generally don't).
/// So this always goes through the runner's own `wine`/`wine64` binary rather
/// than looking for a matching binary alongside it, the one path that works
/// uniformly across both runner kinds.
fn tool_arg(tool: &str) -> Result<&'static str, String> {
    match tool {
        "winecfg" => Ok("winecfg"),
        "regedit" => Ok("regedit"),
        "cmd" => Ok("cmd"),
        "winefile" => Ok("winefile"),
        "uninstaller" => Ok("uninstaller"),
        "taskmgr" => Ok("taskmgr"),
        _ => Err(format!("Unbekanntes Wine-Werkzeug: {tool}")),
    }
}

/// Launches one of Wine's built-in GUI utilities against a prefix. Detached
/// and untracked, unlike `launch_game` — these are tools the user opens and
/// closes freely on their own, not something this app manages the lifecycle
/// of.
#[tauri::command]
pub async fn launch_wine_tool(
    state: State<'_, ConfigState>,
    prefix_path: String,
    runner_id: String,
    tool: String,
) -> Result<(), String> {
    let arg = tool_arg(&tool)?;

    let runners_dir = {
        let config = state
            .lock()
            .map_err(|_| "Configuration is locked".to_string())?;
        config.runners_dir.clone()
    };

    let runner = find_runner(&runners_dir, &runner_id)?;
    let wine = wine_binary(&runner.path)?;
    let prefix = PathBuf::from(&prefix_path);

    // Must run before wineboot's first initialization of this prefix (see
    // `steer_profile_to_steamuser`), which any of these tools can trigger
    // themselves on a fresh, still-uninitialized prefix.
    steer_profile_to_steamuser(&prefix)?;

    runner_command(&wine, [("WINEPREFIX", prefix_path.as_str())])
        .arg(arg)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Konnte {tool} nicht starten: {e}"))?;

    Ok(())
}
