use crate::error::AppError;
use std::path::PathBuf;
use std::process::Stdio;

use tauri::{AppHandle, State};

use crate::commands::games::{env_pairs, log_stdio, prepare_prefix};
use crate::commands::github::read_token;
use crate::commands::logs::{new_log_file, prefix_log_dir};
use crate::commands::runners::{find_runner, runner_command};
use crate::config::ConfigState;

const BUILTIN_WINE_TOOLS: &[&str] = &[
    "winecfg",
    "regedit",
    "cmd",
    "winefile",
    "uninstaller",
    "taskmgr",
];

/// Wine's own built-in GUI utilities — the same set PortProton, Lutris and
/// Bottles all expose straight from their prefix view (winecfg, regedit, a
/// cmd shell, the wine-side file manager, its uninstaller and task manager).
/// Each is resolved by Wine itself as a "builtin" module purely by name, with
/// no on-disk exe required — which matters here because not every runner
/// ships a same-named binary next to `wine` (a plain Wine build does for some
/// of these, e.g. `winecfg`/`winefile`, but Proton runners generally don't).
/// So this always goes through `prepare_prefix`'s binary (umu-run or the runner's own
/// `wine`/`wine64`, both of which resolve a bare name like this as a builtin)
/// rather than looking for a matching binary alongside it, the one path that
/// works uniformly across both runner kinds.
fn tool_arg(tool: &str) -> Result<&'static str, AppError> {
    BUILTIN_WINE_TOOLS
        .iter()
        .copied()
        .find(|builtin| *builtin == tool)
        .ok_or_else(|| AppError::UnknownWineTool {
            tool: tool.to_string(),
        })
}

/// Launches one of Wine's built-in GUI utilities against a prefix. Detached
/// and untracked, unlike `launch_game` — these are tools the user opens and
/// closes freely on their own, not something this app manages the lifecycle
/// of. The prefix is readied first as for a game (see `prepare_prefix`): a
/// tool can be the first thing to touch a fresh one.
#[tauri::command]
pub async fn launch_wine_tool(
    app: AppHandle,
    state: State<'_, ConfigState>,
    prefix_path: String,
    runner_id: String,
    tool: String,
) -> Result<(), AppError> {
    let arg = tool_arg(&tool)?;

    let runners_dir = {
        let config = state
            .lock()
            .map_err(|_| "Configuration is locked".to_string())?;
        config.runners_dir.clone()
    };
    let token = read_token(&state)?;

    let runner = find_runner(&runners_dir, &runner_id)?;
    let prefix = PathBuf::from(&prefix_path);
    let log_path = new_log_file(&prefix_log_dir(&app, &prefix)?)?;
    let prepared = prepare_prefix(&app, token.as_deref(), &runner, &prefix, &log_path, &|| {})
        .await
        .map_err(|e| AppError::WithLogDetails {
            message: e,
            log_path: log_path.display().to_string(),
        })?;
    let (out, err) = log_stdio(&log_path)?;
    runner_command(&prepared.binary, env_pairs(&prepared.env))
        .arg(arg)
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(err)
        .spawn()
        .map_err(|e| AppError::ToolLaunchFailed {
            tool: tool.clone(),
            error: e.to_string(),
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_arg_accepts_all_supported_builtin_tools() {
        for tool in BUILTIN_WINE_TOOLS {
            assert_eq!(tool_arg(tool).unwrap(), *tool);
        }
    }

    #[test]
    fn tool_arg_rejects_unknown_tools_with_the_original_name() {
        match tool_arg("notepad") {
            Err(AppError::UnknownWineTool { tool }) => assert_eq!(tool, "notepad"),
            other => panic!("unexpected result: {other:?}"),
        }
    }
}
