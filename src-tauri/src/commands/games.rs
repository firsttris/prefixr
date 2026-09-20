use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

use crate::commands::icons::extract_icon_data_url;
use crate::commands::runners::{find_runner, runner_command, wine_binary};
use crate::config::{save_config, ConfigState};
use crate::models::{Game, GameInput, RunnerKind};

/// Holds a `--launch <game-id>` argument found at startup (see `run()` in
/// `lib.rs`), so the frontend can pick it up once and start that game
/// immediately — this is what a desktop shortcut created by
/// `create_desktop_shortcut` invokes the app with.
pub struct PendingLaunch(pub Mutex<Option<String>>);

#[tauri::command]
pub fn take_pending_launch(state: State<PendingLaunch>) -> Option<String> {
    state.0.lock().ok()?.take()
}

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
    let icon = extract_icon_data_url(&game.exe_path);
    let new_game = Game {
        id: Uuid::new_v4(),
        name: game.name,
        exe_path: game.exe_path,
        prefix_path: game.prefix_path,
        runner_id: game.runner_id,
        env_vars: game.env_vars,
        icon,
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

    existing.icon = extract_icon_data_url(&game.exe_path);
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

/// The user's Desktop folder, honoring a localized `XDG_DESKTOP_DIR` (e.g.
/// "Schreibtisch" on a German system) if `~/.config/user-dirs.dirs` sets one,
/// falling back to `~/Desktop`.
fn desktop_directory() -> Result<PathBuf, String> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is not set".to_string())?;

    if let Ok(contents) = fs::read_to_string(home.join(".config/user-dirs.dirs")) {
        for line in contents.lines() {
            if let Some(value) = line.trim().strip_prefix("XDG_DESKTOP_DIR=") {
                let value = value
                    .trim_matches('"')
                    .replace("$HOME", &home.to_string_lossy());
                return Ok(PathBuf::from(value));
            }
        }
    }

    Ok(home.join("Desktop"))
}

/// Keeps a filename safe across filesystems by replacing anything but
/// alphanumerics, spaces, dashes and underscores.
fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        "game".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Decodes a game's `icon` data URI to a cached PNG file, since a `.desktop`
/// entry's `Icon=` needs a real file path, not inline image data.
fn write_shortcut_icon(app: &AppHandle, game: &Game) -> Result<Option<PathBuf>, String> {
    let Some(data_url) = &game.icon else {
        return Ok(None);
    };
    let payload = data_url
        .split_once(',')
        .map(|(_, payload)| payload)
        .ok_or_else(|| "Icon data is not a valid data URI".to_string())?;
    let bytes = STANDARD
        .decode(payload)
        .map_err(|e| format!("Could not decode icon data: {e}"))?;

    let icons_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve data directory: {e}"))?
        .join("shortcut-icons");
    fs::create_dir_all(&icons_dir)
        .map_err(|e| format!("Could not create shortcut icons directory: {e}"))?;

    let icon_path = icons_dir.join(format!("{}.png", game.id));
    fs::write(&icon_path, bytes).map_err(|e| format!("Could not write icon file: {e}"))?;
    Ok(Some(icon_path))
}

/// Resolves the path a `.desktop` shortcut should point to. When running from
/// an AppImage, `current_exe()` returns a path inside a temporary FUSE mount
/// (`/tmp/.mount_XXXXXX/...`) that's torn down when the process exits and
/// re-randomized on every launch — useless for a persistent shortcut.
/// AppImages set `APPIMAGE` to the real `.AppImage` file's path exactly for
/// cases like this, so that's preferred when present.
fn own_executable_path() -> Result<PathBuf, String> {
    if let Some(appimage_path) = std::env::var_os("APPIMAGE") {
        return Ok(PathBuf::from(appimage_path));
    }
    std::env::current_exe().map_err(|e| format!("Could not resolve own executable path: {e}"))
}

/// Creates a `.desktop` shortcut on the user's Desktop that launches this
/// game directly, by re-invoking the app's own executable with
/// `--launch <game-id>` (picked up on startup via `take_pending_launch`).
#[tauri::command]
pub fn create_desktop_shortcut(
    app: AppHandle,
    state: State<ConfigState>,
    id: String,
) -> Result<(), String> {
    let game_id = Uuid::parse_str(&id).map_err(|e| format!("Invalid game id: {e}"))?;
    let game = {
        let config = state
            .lock()
            .map_err(|_| "Configuration is locked".to_string())?;
        config
            .games
            .iter()
            .find(|g| g.id == game_id)
            .cloned()
            .ok_or_else(|| format!("No game with id {id}"))?
    };

    let desktop_dir = desktop_directory()?;
    fs::create_dir_all(&desktop_dir)
        .map_err(|e| format!("Could not access Desktop directory: {e}"))?;

    let icon_path = write_shortcut_icon(&app, &game)?;
    let exe_path = own_executable_path()?;

    let mut contents = String::new();
    contents.push_str("[Desktop Entry]\n");
    contents.push_str("Type=Application\n");
    contents.push_str(&format!("Name={}\n", game.name));
    contents.push_str(&format!(
        "Exec=\"{}\" --launch {}\n",
        exe_path.display(),
        game.id
    ));
    if let Some(icon_path) = &icon_path {
        contents.push_str(&format!("Icon={}\n", icon_path.display()));
    }
    contents.push_str("Terminal=false\n");
    contents.push_str("Categories=Game;\n");

    let shortcut_path = desktop_dir.join(format!("{}.desktop", sanitize_filename(&game.name)));
    fs::write(&shortcut_path, contents)
        .map_err(|e| format!("Could not write shortcut file: {e}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&shortcut_path)
            .map_err(|e| format!("Could not read shortcut permissions: {e}"))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&shortcut_path, perms)
            .map_err(|e| format!("Could not set shortcut permissions: {e}"))?;
    }

    Ok(())
}
