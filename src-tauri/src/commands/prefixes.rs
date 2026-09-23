use std::fs;
use std::path::PathBuf;

use tauri::{AppHandle, State};

use crate::commands::games::{LaunchingGames, RunningGames};
use crate::config::{save_config, ConfigState};
use crate::models::PrefixInfo;

/// Registers a prefix path with the app — either a brand-new, empty folder
/// (created here, initialized lazily via `wineboot` on the first game launch
/// that uses it) or an already-existing Wine prefix from another tool. No
/// runner is involved here: which runner initializes/launches a prefix is
/// decided per-game, via `Game::runner_id`.
#[tauri::command]
pub fn add_prefix(app: AppHandle, state: State<ConfigState>, path: String) -> Result<(), String> {
    let prefix_path = PathBuf::from(&path);

    let mut config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;

    if config.prefixes.iter().any(|p| p.path == prefix_path) {
        return Err(format!("Prefix at {path} already exists"));
    }

    if prefix_path.exists() {
        if !prefix_path.is_dir() {
            return Err(format!("{path} is not a directory"));
        }
        let has_contents = fs::read_dir(&prefix_path)
            .map_err(|e| format!("Could not read {path}: {e}"))?
            .next()
            .is_some();
        // A Wine prefix has `drive_c` directly inside it; a Proton "compat
        // data" folder instead has it under `pfx/` (Proton's own layout).
        let looks_like_prefix =
            prefix_path.join("drive_c").is_dir() || prefix_path.join("pfx").is_dir();
        if has_contents && !looks_like_prefix {
            return Err(format!(
                "{path} already contains files but doesn't look like a Wine or Proton prefix"
            ));
        }
    } else {
        fs::create_dir_all(&prefix_path)
            .map_err(|e| format!("Could not create prefix directory: {e}"))?;
    }

    config.prefixes.push(PrefixInfo { path: prefix_path });
    save_config(&app, &config)?;
    Ok(())
}

/// Removes a prefix from the app, and with `delete_files` also its folder.
/// Refused while a game in it is running or being launched: deleting the
/// folder would pull it out from under the game.
#[tauri::command]
pub fn delete_prefix(
    app: AppHandle,
    state: State<ConfigState>,
    running: State<RunningGames>,
    launching: State<LaunchingGames>,
    path: String,
    delete_files: bool,
) -> Result<(), String> {
    let prefix_path = PathBuf::from(&path);
    let mut config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;

    if !config.prefixes.iter().any(|p| p.path == prefix_path) {
        return Err(format!("No prefix known at {path}"));
    }

    let active_game = {
        let running = running
            .0
            .lock()
            .map_err(|_| "Running games list is locked".to_string())?;
        config
            .games
            .iter()
            .filter(|g| g.prefix_path == prefix_path)
            .find(|g| running.contains_key(&g.id) || launching.contains(g.id))
            .map(|g| g.name.clone())
    };
    if let Some(name) = active_game {
        return Err(format!(
            "„{name}“ läuft noch in diesem Prefix. Beende das Spiel zuerst."
        ));
    }

    if delete_files && prefix_path.exists() {
        fs::remove_dir_all(&prefix_path)
            .map_err(|e| format!("Could not delete prefix directory: {e}"))?;
    }

    config.prefixes.retain(|p| p.path != prefix_path);
    save_config(&app, &config)?;
    Ok(())
}

#[tauri::command]
pub fn list_prefixes(state: State<ConfigState>) -> Result<Vec<PrefixInfo>, String> {
    let config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    Ok(config.prefixes.clone())
}
