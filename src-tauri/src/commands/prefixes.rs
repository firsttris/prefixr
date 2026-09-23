use std::fs;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, State};

use crate::commands::games::LaunchingGames;
use crate::config::{save_config, ConfigState};
use crate::models::PrefixInfo;

/// The folder to use as `WINEPREFIX` for a folder the user picked. A Proton
/// "compat data" folder (as Steam, Lutris or Heroic leave behind) keeps the
/// actual prefix in `pfx/`, next to Proton's own bookkeeping. Used as it is,
/// every check for `drive_c` would miss the existing prefix, and a Wine
/// runner would start a new, empty one next to it.
///
/// umu lays a prefix out the other way round, with `pfx` a link back to the
/// folder itself; that one is used as it is.
pub fn effective_prefix_path(path: &Path) -> PathBuf {
    let pfx = path.join("pfx");
    let links_back = fs::canonicalize(&pfx).ok() == fs::canonicalize(path).ok();
    if !path.join("drive_c").is_dir() && pfx.is_dir() && !links_back {
        pfx
    } else {
        path.to_path_buf()
    }
}

/// Registers a prefix path with the app — either a brand-new, empty folder
/// (created here, initialized lazily via `wineboot` on the first game launch
/// that uses it) or an already-existing Wine prefix from another tool. No
/// runner is involved here: which runner initializes/launches a prefix is
/// decided per-game, via `Game::runner_id`. Returns the path registered,
/// which for a Proton compat data folder is its `pfx/` (see
/// `effective_prefix_path`).
#[tauri::command]
pub fn add_prefix(
    app: AppHandle,
    state: State<ConfigState>,
    path: String,
) -> Result<PathBuf, String> {
    let prefix_path = effective_prefix_path(Path::new(&path));

    let mut config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;

    if config.prefixes.iter().any(|p| p.path == prefix_path) {
        return Err(format!("Prefix at {} already exists", prefix_path.display()));
    }

    if prefix_path.exists() {
        if !prefix_path.is_dir() {
            return Err(format!("{path} is not a directory"));
        }
        let has_contents = fs::read_dir(&prefix_path)
            .map_err(|e| format!("Could not read {path}: {e}"))?
            .next()
            .is_some();
        // A Proton "compat data" folder was already swapped for its `pfx/`.
        let looks_like_prefix = prefix_path.join("drive_c").is_dir();
        if has_contents && !looks_like_prefix {
            return Err(format!(
                "{path} already contains files but doesn't look like a Wine or Proton prefix"
            ));
        }
    } else {
        fs::create_dir_all(&prefix_path)
            .map_err(|e| format!("Could not create prefix directory: {e}"))?;
    }

    config.prefixes.push(PrefixInfo {
        path: prefix_path.clone(),
    });
    save_config(&app, &config)?;
    Ok(prefix_path)
}

/// Removes a prefix from the app, and with `delete_files` also its folder.
/// Refused while a game in it is running or being launched: deleting the
/// folder would pull it out from under the game. Async, since deleting a
/// prefix of several GB would otherwise freeze the window meanwhile.
#[tauri::command]
pub async fn delete_prefix(
    app: AppHandle,
    state: State<'_, ConfigState>,
    launching: State<'_, LaunchingGames>,
    path: String,
    delete_files: bool,
) -> Result<(), String> {
    let prefix_path = PathBuf::from(&path);

    // Every game in this prefix is held as launching until the folder is
    // gone, so none can be started while it's being deleted. A game that
    // already runs is launching until it exits, so the claim is refused.
    let mut guards = Vec::new();
    {
        let config = state
            .lock()
            .map_err(|_| "Configuration is locked".to_string())?;
        if !config.prefixes.iter().any(|p| p.path == prefix_path) {
            return Err(format!("No prefix known at {path}"));
        }
        for game in config.games.iter().filter(|g| g.prefix_path == prefix_path) {
            let Some(guard) = launching.claim(game.id) else {
                return Err(format!(
                    "„{}“ läuft noch in diesem Prefix. Beende das Spiel zuerst.",
                    game.name
                ));
            };
            guards.push(guard);
        }
    }

    if delete_files && prefix_path.exists() {
        let target = prefix_path.clone();
        tauri::async_runtime::spawn_blocking(move || fs::remove_dir_all(target))
            .await
            .map_err(|e| format!("Could not delete prefix directory: {e}"))?
            .map_err(|e| format!("Could not delete prefix directory: {e}"))?;
    }

    let mut config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    config.prefixes.retain(|p| p.path != prefix_path);
    save_config(&app, &config)
}

#[tauri::command]
pub fn list_prefixes(state: State<ConfigState>) -> Result<Vec<PrefixInfo>, String> {
    let config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    Ok(config.prefixes.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proton_compat_data_folders_use_their_pfx() {
        let dir = std::env::temp_dir().join(format!("prefixr-test-{}", uuid::Uuid::new_v4()));
        let compat = dir.join("compatdata/1091500");
        fs::create_dir_all(compat.join("pfx/drive_c")).unwrap();
        assert_eq!(effective_prefix_path(&compat), compat.join("pfx"));

        let wine = dir.join("wine-prefix");
        fs::create_dir_all(wine.join("drive_c")).unwrap();
        assert_eq!(effective_prefix_path(&wine), wine);

        // umu's layout: `pfx` links back to the prefix itself.
        let umu = dir.join("umu-prefix");
        fs::create_dir_all(&umu).unwrap();
        std::os::unix::fs::symlink(".", umu.join("pfx")).unwrap();
        assert_eq!(effective_prefix_path(&umu), umu);

        // Empty or not yet created: used as picked.
        assert_eq!(effective_prefix_path(&dir.join("new")), dir.join("new"));
        fs::remove_dir_all(&dir).unwrap();
    }
}
