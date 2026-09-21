use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

use crate::commands::graphics_layers::{ensure_directx_layer_cache, ensure_wine_mono_msi};
use crate::commands::icons::extract_icon_data_url;
use crate::commands::mangohud::ensure_mangohud_conf;
use crate::commands::runners::{find_runner, runner_command, wine_binary, wineserver_binary};
use crate::config::{save_config, ConfigState};
use crate::models::{Game, GameInput, RunnerKind};
use crate::tray::rebuild_tray_menu;

/// A game process currently running under its runner, tracked so the tray
/// menu can list it and offer to kill it (e.g. when a Proton game hangs).
#[derive(Clone)]
pub struct RunningGame {
    pub name: String,
    /// Path to the `wineserver` binary belonging to the runner this game was
    /// launched with, and the `WINEPREFIX` it's managing that game's session
    /// under. Used to shut the session down (see `kill_running_game`) instead
    /// of tracking a PID: when a runner is invoked through
    /// `distrobox-host-exec` (see `runner_command`), the process actually
    /// runs on the host, in a different PID namespace than this app — a PID
    /// or process-group signal sent from here would only ever reach the
    /// local `distrobox-host-exec` relay, not the real process, and a
    /// SIGKILL can't be forwarded through that relay either since it kills
    /// the relay itself before it gets the chance. Running `wineserver -k`
    /// through the same `runner_command` reaches the real session either
    /// way, because wineserver finds it via the prefix's socket file rather
    /// than by PID.
    pub wineserver: PathBuf,
    pub wineprefix: String,
    /// The game's own exe path, as passed to wine. Present verbatim in the
    /// command line of both the `wine` process we spawned and the actual
    /// Windows process running under it, so it doubles as a `pkill -f`
    /// pattern that reaches both directly — see `kill_running_game`.
    pub exe_path: PathBuf,
}

#[derive(Default)]
pub struct RunningGames(pub Mutex<HashMap<Uuid, RunningGame>>);

/// Kills a running game's whole wine session, used by both the `kill_game`
/// command and the tray menu's per-game "beenden" entries. The tracked entry
/// itself is removed once `launch_game`'s `child.wait()` observes the
/// process actually exiting, not here.
///
/// Uses two mechanisms together, the same way PortProton's `kill_portwine`
/// combines a wineserver shutdown with directly hard-killing the
/// wine-preloader:
///
/// 1. `wineserver -k9` asks the whole wine session (game exe, services.exe,
///    explorer.exe, ...) to terminate via wineserver's own IPC — thorough
///    when it works, but it never reaches the `wine` process we spawned
///    itself, since wineserver has no notion of its own parent.
/// 2. `pkill -9 -f` on the game's exe path directly hard-kills anything
///    whose command line mentions it — both that process and the game's own
///    process.
pub async fn kill_running_game(running: &RunningGames, id: Uuid) -> Result<(), String> {
    let (wineserver, wineprefix, exe_path) = {
        let games = running
            .0
            .lock()
            .map_err(|_| "Running games list is locked".to_string())?;
        let game = games.get(&id).ok_or_else(|| "Game is not running".to_string())?;
        (
            game.wineserver.clone(),
            game.wineprefix.clone(),
            game.exe_path.clone(),
        )
    };

    let wineserver_result = runner_command(&wineserver, [("WINEPREFIX", wineprefix.as_str())])
        .arg("-k9")
        .status()
        .await;

    let pkill_result = runner_command(Path::new("pkill"), [])
        .args(["-9", "-f"])
        .arg(&exe_path)
        .status()
        .await;

    // Either one actually killing something is a success; only report an
    // error if both failed to even run (a real environment problem, e.g.
    // neither binary exists) rather than surfacing e.g. wineserver's "no
    // session found" exit code as a failure when pkill already got it.
    match (wineserver_result, pkill_result) {
        (Err(e), Err(_)) => Err(format!("Could not run wineserver: {e}")),
        _ => Ok(()),
    }
}

#[tauri::command]
pub async fn kill_game(running: State<'_, RunningGames>, id: String) -> Result<(), String> {
    let game_id = Uuid::parse_str(&id).map_err(|e| format!("Invalid game id: {e}"))?;
    kill_running_game(&running, game_id).await
}

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

/// Direct3D/DXGI/VKD3D files a Proton runner ships pre-linked into its own
/// template prefix (`files/share/default_pfx/`) — what it copies from into a
/// fresh `pfx/` on first run. GE-Proton bakes DXVK/VKD3D directly into its
/// wine build as the builtin implementation of these modules (unlike a tool
/// that bolts external DXVK onto a plain Wine build), so no `WINEDLLOVERRIDES`
/// is needed to make wine prefer them — it uses them automatically once
/// they're the ones actually reachable at these paths.
const DIRECTX_OVERRIDE_FILES: &[&str] = &[
    "d3d8.dll",
    "d3d8thk.dll",
    "d3d9.dll",
    "d3d10core.dll",
    "d3d11.dll",
    "d3d12.dll",
    "d3d12core.dll",
    "dxgi.dll",
    "wined3d.dll",
    "libvkd3d-1.dll",
    "libvkd3d-shader-1.dll",
    "libvkd3d-utils-1.dll",
];

/// Re-links the given runner's own Direct3D/DXGI/VKD3D files into a prefix's
/// `system32`/`syswow64`. Mirrors what Proton's own wrapper script does for
/// its nested `pfx/` (see the comment on `wineprefix` in `launch_game`) and
/// what PortProton does on every launch for the same reason (its
/// `functions_helper`, `CP_DXVK_FILES`/`CP_VKD3D_FILES`): a prefix driven
/// directly by wine — because it's imported from another tool, or was last
/// used with a different runner — may have these files symlinked to a
/// completely different, binary-incompatible build, or be
/// missing them entirely, which breaks the moment this runner's wine tries
/// to load them (its own builtin version depends on companions, like
/// `wined3d.dll` on `libvkd3d-utils-1.dll`, that a foreign build won't have
/// next to it). Runners with no bundled DXVK/VKD3D (plain Wine builds, no
/// `files/share/default_pfx`) leave the prefix untouched.
fn sync_directx_overrides(runner_path: &Path, prefix_path: &Path) -> Result<(), String> {
    let template_windows = runner_path.join("files/share/default_pfx/drive_c/windows");
    if !template_windows.is_dir() {
        return Ok(());
    }

    for subdir in ["system32", "syswow64"] {
        let src_dir = template_windows.join(subdir);
        let dst_dir = prefix_path.join("drive_c/windows").join(subdir);
        if !dst_dir.is_dir() {
            continue;
        }

        for file_name in DIRECTX_OVERRIDE_FILES {
            relink_if_needed(&src_dir.join(file_name), &dst_dir.join(file_name))?;
        }
    }

    Ok(())
}

/// Symlinks `dst_path` to `src_path`'s fully-resolved target, replacing
/// whatever (if anything) is already there — unless it's already correctly
/// linked, in which case it's left untouched. A missing `src_path` is not an
/// error: it just means this particular file isn't relevant (Direct3D
/// modules a given layer doesn't provide, e.g. DXVK has no d3d12).
fn relink_if_needed(src_path: &Path, dst_path: &Path) -> Result<(), String> {
    let Ok(resolved_src) = fs::canonicalize(src_path) else {
        return Ok(());
    };
    if fs::canonicalize(dst_path).ok().as_deref() == Some(resolved_src.as_path()) {
        return Ok(());
    }
    if fs::symlink_metadata(dst_path).is_ok() {
        fs::remove_file(dst_path)
            .map_err(|e| format!("Could not remove {}: {e}", dst_path.display()))?;
    }
    std::os::unix::fs::symlink(&resolved_src, dst_path)
        .map_err(|e| format!("Could not link {}: {e}", dst_path.display()))
}

/// Direct3D/DXGI modules DXVK provides, and D3D12 modules VKD3D-Proton
/// provides — forced to load as native (unlike a Proton runner's own
/// builtin copies, see `sync_directx_overrides`) since a plain Wine build's
/// own builtin implementations of these are the unaccelerated, OpenGL-backed
/// ones DXVK/VKD3D-Proton exist to replace.
const DXVK_MODULES: &[&str] = &["d3d8", "d3d9", "d3d10core", "d3d11", "dxgi"];
const VKD3D_MODULES: &[&str] = &["d3d12", "d3d12core"];

/// Re-links DXVK and VKD3D-Proton's DLLs (from the shared cache — see
/// `graphics_layers::ensure_directx_layer_cache`) into a Wine prefix's
/// `system32`/`syswow64`, and returns the `WINEDLLOVERRIDES` value that
/// forces wine to actually load them. Mirrors PortProton's own
/// `CP_DXVK_FILES`/`CP_VKD3D_FILES` handling in `functions_helper` — the
/// same trick `sync_directx_overrides` uses for a Proton runner, just
/// sourced from a separately downloaded DXVK/VKD3D-Proton instead of a
/// runner's bundled copy, and (unlike a Proton runner, whose builtin modules
/// already *are* DXVK/VKD3D) needing the override to actually take effect.
fn sync_directx_overrides_from_cache(cache_dir: &Path, prefix_path: &Path) -> Result<String, String> {
    let windows_dir = prefix_path.join("drive_c/windows");

    for (layer_dir, dir64, dir32, modules) in [
        ("dxvk", "x64", "x32", DXVK_MODULES),
        ("vkd3d-proton", "x64", "x86", VKD3D_MODULES),
    ] {
        for (arch_dir, subdir) in [(dir64, "system32"), (dir32, "syswow64")] {
            let src_dir = cache_dir.join(layer_dir).join(arch_dir);
            let dst_dir = windows_dir.join(subdir);
            if !dst_dir.is_dir() {
                continue;
            }
            for module in modules {
                let file_name = format!("{module}.dll");
                relink_if_needed(&src_dir.join(&file_name), &dst_dir.join(&file_name))?;
            }
        }
    }

    let modules: Vec<&str> = DXVK_MODULES
        .iter()
        .chain(VKD3D_MODULES.iter())
        .copied()
        .collect();
    Ok(format!("{}=n", modules.join(",")))
}

/// Installs wine-mono into `prefix_path` via `msiexec`, unless it's already
/// there (`drive_c/windows/mono/`, where the installer places it). Wine
/// looks for it there whenever an app tries to host a .NET assembly, and,
/// finding it missing, shows its own "Wine Mono Installation" dialog asking
/// to download and install it — which would otherwise pop up unprompted on
/// whatever needs it first. Proton bundles Mono and installs it
/// automatically via its own wrapper script; a plain Wine build (e.g.
/// Kron4ek) doesn't, so this does the same job for a Wine-kind runner.
async fn install_wine_mono(
    app: &AppHandle,
    wine: &Path,
    prefix_path: &Path,
    log_path: &Path,
) -> Result<(), String> {
    if prefix_path.join("drive_c/windows/mono").is_dir() {
        return Ok(());
    }

    let msi_path = ensure_wine_mono_msi(app).await?;
    let prefix_str = prefix_path
        .to_str()
        .ok_or_else(|| format!("Prefix path is not valid UTF-8: {}", prefix_path.display()))?;
    let msi_str = msi_path
        .to_str()
        .ok_or_else(|| format!("Installer path is not valid UTF-8: {}", msi_path.display()))?;

    let out = fs::OpenOptions::new()
        .append(true)
        .open(log_path)
        .map_err(|e| format!("Could not open log file: {e}"))?;
    let err = out
        .try_clone()
        .map_err(|e| format!("Could not open log file: {e}"))?;

    let status = runner_command(wine, [("WINEPREFIX", prefix_str)])
        .args(["msiexec", "/i", msi_str, "/qn"])
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err))
        .status()
        .await
        .map_err(|e| format!("Could not run wine-mono installer: {e}"))?;

    if !status.success() {
        return Err(format!("wine-mono installer exited with status {status}"));
    }
    Ok(())
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
    running: State<'_, RunningGames>,
    id: String,
) -> Result<(), String> {
    let game_id = Uuid::parse_str(&id).map_err(|e| format!("Invalid game id: {e}"))?;

    let (game, runners_dir, mangohud) = {
        let config = state
            .lock()
            .map_err(|_| "Configuration is locked".to_string())?;
        let game = config
            .games
            .iter()
            .find(|g| g.id == game_id)
            .cloned()
            .ok_or_else(|| format!("No game with id {id}"))?;
        (game, config.runners_dir.clone(), config.mangohud.clone())
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

    // Every prefix is driven directly by the runner's own wine binary,
    // against `<prefix>` itself — never through Proton's `proton` wrapper
    // script, which pins its prefix to `<STEAM_COMPAT_DATA_PATH>/pfx` with no
    // way to point it at `<prefix>` instead. That's deliberate, not just a
    // shortcut: it's the only way a game can be freely reassigned between a
    // Proton and a Wine runner and keep seeing the same installed files and
    // saves either way (both PortProton and Bottles manage DXVK/VKD3D this
    // same way, independently of which wine build is running them, for the
    // same reason). `kill_running_game` needs this exact value to reach the
    // right wineserver session.
    let wineprefix = prefix_path_str.to_string();
    let wineserver = wineserver_binary(&runner.path)?;
    let wine = wine_binary(&runner.path)?;

    // A prefix is created without ever running wineboot (see `add_prefix`),
    // so an empty one — no `drive_c` yet — is initialized here on first use,
    // with that game's own runner.
    let is_uninitialized_prefix = !game.prefix_path.join("drive_c").is_dir();
    if is_uninitialized_prefix {
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

    let dll_overrides = match runner.kind {
        RunnerKind::Proton => {
            sync_directx_overrides(&runner.path, &game.prefix_path)?;
            None
        }
        RunnerKind::Wine => {
            install_wine_mono(&app, &wine, &game.prefix_path, &log_path).await?;
            let cache = ensure_directx_layer_cache(&app).await?;
            Some(sync_directx_overrides_from_cache(
                &cache,
                &game.prefix_path,
            )?)
        }
    };

    let mut env = vec![("WINEPREFIX".to_string(), prefix_path_str.to_string())];
    if let Some(overrides) = dll_overrides {
        env.push(("WINEDLLOVERRIDES".to_string(), overrides));
    }
    if mangohud.enabled {
        let conf_path = ensure_mangohud_conf(&app, &mangohud)?;
        env.push(("MANGOHUD".to_string(), "1".to_string()));
        env.push((
            "MANGOHUD_CONFIGFILE".to_string(),
            conf_path.display().to_string(),
        ));
    }
    env.extend(game.env_vars.iter().map(|(k, v)| (k.clone(), v.clone())));

    let (launch_binary, launch_args, launch_env): (PathBuf, Vec<String>, Vec<(String, String)>) =
        (wine, vec![], env);

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
        // Makes this process (the `proton` script, or `wine` itself) the
        // leader of a fresh process group, so `kill_running_game` can reach
        // everything it spawns (wineserver, the game exe, ...) by signalling
        // the group instead of just this one PID.
        .process_group(0)
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

    if let Ok(mut running) = running.0.lock() {
        running.insert(
            game_id,
            RunningGame {
                name: game.name.clone(),
                wineserver: wineserver.clone(),
                wineprefix: wineprefix.clone(),
                exe_path: game.exe_path.clone(),
            },
        );
    }
    rebuild_tray_menu(&app);

    let _ = app.emit(
        "game-started",
        GameStartedPayload {
            id: &id,
            log_path: log_path_string.clone(),
        },
    );

    let wait_result = child.wait().await;

    if let Ok(mut running) = running.0.lock() {
        running.remove(&game_id);
    }
    rebuild_tray_menu(&app);

    let status = wait_result.map_err(|e| format!("Game process failed: {e}"))?;

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
