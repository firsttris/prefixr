use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

use crate::commands::runners::{find_runner, runner_command, wine_binary};
use crate::config::{save_config, ConfigState};
use crate::models::{Game, GameInput, RunnerKind};

#[tauri::command]
pub fn list_games(state: State<ConfigState>) -> Result<Vec<Game>, String> {
    let config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    Ok(config.games.clone())
}

#[tauri::command]
pub fn add_game(
    app: AppHandle,
    state: State<ConfigState>,
    game: GameInput,
) -> Result<Game, String> {
    let new_game = Game {
        id: Uuid::new_v4(),
        name: game.name,
        exe_path: game.exe_path,
        prefix_path: game.prefix_path,
        runner_id: game.runner_id,
        env_vars: game.env_vars,
    };

    let mut config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    config.games.push(new_game.clone());
    save_config(&app, &config)?;
    Ok(new_game)
}

#[tauri::command]
pub fn update_game(
    app: AppHandle,
    state: State<ConfigState>,
    id: String,
    game: GameInput,
) -> Result<Game, String> {
    let game_id = Uuid::parse_str(&id).map_err(|e| format!("Invalid game id: {e}"))?;
    let mut config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;

    let existing = config
        .games
        .iter_mut()
        .find(|g| g.id == game_id)
        .ok_or_else(|| format!("No game with id {id}"))?;

    existing.name = game.name;
    existing.exe_path = game.exe_path;
    existing.prefix_path = game.prefix_path;
    existing.runner_id = game.runner_id;
    existing.env_vars = game.env_vars;
    let updated = existing.clone();

    save_config(&app, &config)?;
    Ok(updated)
}

#[tauri::command]
pub fn remove_game(app: AppHandle, state: State<ConfigState>, id: String) -> Result<(), String> {
    let game_id = Uuid::parse_str(&id).map_err(|e| format!("Invalid game id: {e}"))?;
    let mut config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;

    if !config.games.iter().any(|g| g.id == game_id) {
        return Err(format!("No game with id {id}"));
    }

    config.games.retain(|g| g.id != game_id);
    save_config(&app, &config)?;
    Ok(())
}

#[derive(Clone, Serialize)]
struct GameInitializingPayload<'a> {
    id: &'a str,
}

#[derive(Clone, Serialize)]
struct GameStartedPayload<'a> {
    id: &'a str,
    log_path: String,
}

#[derive(Clone, Serialize)]
struct GameExitedPayload<'a> {
    id: &'a str,
    exit_code: Option<i32>,
}

#[derive(Clone, Serialize)]
struct GameLaunchErrorPayload<'a> {
    id: &'a str,
    message: String,
    log_path: Option<String>,
}

/// A writable directory to hand Proton as `STEAM_COMPAT_CLIENT_INSTALL_PATH`.
/// Proton expects a Steam-install-shaped path for some checks even when run
/// outside of Steam; a dedicated empty directory satisfies that without
/// depending on a real Steam installation being present.
fn steam_compat_client_dir(app: &AppHandle) -> Result<String, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve data directory: {e}"))?;
    let dir = data_dir.join("steam-compat-client-install");
    fs::create_dir_all(&dir)
        .map_err(|e| format!("Could not create Steam compat client directory: {e}"))?;
    dir.to_str()
        .map(str::to_string)
        .ok_or_else(|| format!("Path is not valid UTF-8: {}", dir.display()))
}

fn log_file_path(app: &AppHandle, game_id: &str) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve data directory: {e}"))?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Could not read system time: {e}"))?
        .as_secs();
    // .txt rather than .log: most Linux desktops have no default app
    // registered for .log, so "Log anzeigen" would silently fail to open it.
    Ok(data_dir
        .join("logs")
        .join(game_id)
        .join(format!("{timestamp}.txt")))
}

/// Launches a game's exe under its configured runner and prefix. stdout/stderr
/// are redirected straight into a per-run log file. The frontend is told about
/// the outcome both via the command's own `Result` and via `game-started` /
/// `game-exited` / `game-launch-error` events, so it can show a live "running"
/// state as well as a final error with a link to the log.
#[tauri::command]
pub async fn launch_game(
    app: AppHandle,
    state: State<'_, ConfigState>,
    id: String,
) -> Result<(), String> {
    let game_id = Uuid::parse_str(&id).map_err(|e| format!("Invalid game id: {e}"))?;

    let (game, runners_dir) = {
        let config = state
            .lock()
            .map_err(|_| "Configuration is locked".to_string())?;
        let game = config
            .games
            .iter()
            .find(|g| g.id == game_id)
            .cloned()
            .ok_or_else(|| format!("No game with id {id}"))?;
        (game, config.runners_dir.clone())
    };

    let runner = find_runner(&runners_dir, &game.runner_id)?;
    let prefix_path_str = game.prefix_path.to_str().ok_or_else(|| {
        format!(
            "Prefix path is not valid UTF-8: {}",
            game.prefix_path.display()
        )
    })?;

    let log_path = log_file_path(&app, &id)?;
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Could not create log directory: {e}"))?;
    }
    fs::File::create(&log_path).map_err(|e| format!("Could not create log file: {e}"))?;
    let log_path_string = log_path.display().to_string();

    // Proton isn't just wine: its `proton` wrapper script also installs
    // DXVK/VKD3D (the Direct3D-to-Vulkan layers) into the prefix on first run
    // and manages its own prefix layout at `<prefix>/pfx`. Calling wine
    // directly for a Proton runner would skip all of that. Plain Wine
    // runners have no such wrapper, so they're launched directly and — since
    // prefixes are created without ever running wineboot (see `add_prefix`)
    // — initialized here on first use, with that game's own runner.
    let (launch_binary, launch_args, launch_env): (PathBuf, Vec<String>, Vec<(String, String)>) =
        match runner.kind {
            RunnerKind::Proton => {
                let proton_script = runner.path.join("proton");
                if !proton_script.is_file() {
                    return Err(format!(
                        "Proton script not found in runner directory {}",
                        runner.path.display()
                    ));
                }

                if !game.prefix_path.join("pfx").is_dir() {
                    let _ = app.emit("game-initializing", GameInitializingPayload { id: &id });
                }

                let mut env = vec![
                    (
                        "STEAM_COMPAT_DATA_PATH".to_string(),
                        prefix_path_str.to_string(),
                    ),
                    (
                        "STEAM_COMPAT_CLIENT_INSTALL_PATH".to_string(),
                        steam_compat_client_dir(&app)?,
                    ),
                ];
                env.extend(game.env_vars.iter().map(|(k, v)| (k.clone(), v.clone())));

                (proton_script, vec!["run".to_string()], env)
            }
            RunnerKind::Wine => {
                let wine = wine_binary(&runner.path)?;

                if !game.prefix_path.join("drive_c").is_dir() {
                    let _ = app.emit("game-initializing", GameInitializingPayload { id: &id });

                    let init_out = fs::OpenOptions::new()
                        .append(true)
                        .open(&log_path)
                        .map_err(|e| format!("Could not open log file: {e}"))?;
                    let init_err = init_out
                        .try_clone()
                        .map_err(|e| format!("Could not open log file: {e}"))?;

                    let status = runner_command(&wine, [("WINEPREFIX", prefix_path_str)])
                        .arg("wineboot")
                        .stdout(Stdio::from(init_out))
                        .stderr(Stdio::from(init_err))
                        .status()
                        .await
                        .map_err(|e| format!("Could not initialize prefix: {e}"))?;

                    if !status.success() {
                        let message = format!("Prefix initialization failed with status {status}");
                        let _ = app.emit(
                            "game-launch-error",
                            GameLaunchErrorPayload {
                                id: &id,
                                message: message.clone(),
                                log_path: Some(log_path_string.clone()),
                            },
                        );
                        return Err(message);
                    }
                }

                let mut env = vec![("WINEPREFIX".to_string(), prefix_path_str.to_string())];
                env.extend(game.env_vars.iter().map(|(k, v)| (k.clone(), v.clone())));

                (wine, vec![], env)
            }
        };

    let log_out = fs::OpenOptions::new()
        .append(true)
        .open(&log_path)
        .map_err(|e| format!("Could not open log file: {e}"))?;
    let log_err = log_out
        .try_clone()
        .map_err(|e| format!("Could not open log file: {e}"))?;

    let env_pairs: Vec<(&str, &str)> = launch_env
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    let mut child = runner_command(&launch_binary, env_pairs)
        .args(&launch_args)
        .arg(&game.exe_path)
        .stdout(Stdio::from(log_out))
        .stderr(Stdio::from(log_err))
        .spawn()
        .map_err(|e| {
            let message = format!("Could not start game: {e}");
            let _ = app.emit(
                "game-launch-error",
                GameLaunchErrorPayload {
                    id: &id,
                    message: message.clone(),
                    log_path: Some(log_path_string.clone()),
                },
            );
            message
        })?;

    let _ = app.emit(
        "game-started",
        GameStartedPayload {
            id: &id,
            log_path: log_path_string.clone(),
        },
    );

    let status = child
        .wait()
        .await
        .map_err(|e| format!("Game process failed: {e}"))?;

    let _ = app.emit(
        "game-exited",
        GameExitedPayload {
            id: &id,
            exit_code: status.code(),
        },
    );

    if !status.success() {
        let message = format!("Game exited with status {status}");
        let _ = app.emit(
            "game-launch-error",
            GameLaunchErrorPayload {
                id: &id,
                message: message.clone(),
                log_path: Some(log_path_string),
            },
        );
        return Err(message);
    }

    Ok(())
}
