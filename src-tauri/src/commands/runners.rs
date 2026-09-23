use std::fs;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, State};
use tokio::process::Command;

use crate::commands::games::steer_profile_to_steamuser;
use crate::commands::umu::ensure_umu;
use crate::config::ConfigState;
use crate::models::{Runner, RunnerKind};

/// Identifies a runner build by looking for known executables inside its folder.
/// Returns `None` for folders that match neither layout, so they're skipped
/// rather than surfaced as a broken entry.
fn detect_runner_kind(runner_dir: &Path) -> Option<RunnerKind> {
    if runner_dir.join("proton").is_file() {
        return Some(RunnerKind::Proton);
    }
    if runner_dir.join("bin/wine").is_file() || runner_dir.join("files/bin/wine").is_file() {
        return Some(RunnerKind::Wine);
    }
    None
}

/// Scans `runners_dir` for valid runner folders. Shared by `list_runners` and
/// `find_runner` so both see the exact same set of recognized runners.
pub fn scan_runners(runners_dir: &Path) -> Result<Vec<Runner>, String> {
    if !runners_dir.exists() {
        return Ok(Vec::new());
    }

    let entries =
        fs::read_dir(runners_dir).map_err(|e| format!("Could not read runners directory: {e}"))?;

    let mut runners = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Could not read directory entry: {e}"))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(kind) = detect_runner_kind(&path) else {
            continue;
        };
        let id = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        runners.push(Runner {
            id: id.clone(),
            name: id,
            path,
            kind,
        });
    }

    runners.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(runners)
}

/// Looks up a single runner by id, used by commands that need to resolve a
/// `runner_id` (e.g. `launch_game`) to an actual path on disk.
pub fn find_runner(runners_dir: &Path, runner_id: &str) -> Result<Runner, String> {
    scan_runners(runners_dir)?
        .into_iter()
        .find(|r| r.id == runner_id)
        .ok_or_else(|| format!("Runner '{runner_id}' not found"))
}

/// Looks for a wine-toolchain binary (`wine`/`wine64`/`wineserver`) inside a
/// runner folder, trying both plain Wine's layout and Proton's (which bundles
/// its own copy of the same tools under `files/`).
fn find_wine_tool(runner_path: &Path, name: &str) -> Result<PathBuf, String> {
    for prefix in ["bin/", "files/bin/", ""] {
        let candidate_path = runner_path.join(format!("{prefix}{name}"));
        if candidate_path.is_file() {
            return Ok(candidate_path);
        }
    }
    Err(format!(
        "No {name} binary found in runner directory {}",
        runner_path.display()
    ))
}

/// Locates the wine binary inside a runner folder, covering both plain Wine
/// builds and Proton's bundled wine. Games and tools on a Proton runner are
/// launched through umu instead (see `prefix_command`); this is only still
/// used directly on a Proton runner for winetricks when that runner ships no
/// protonfixes of its own (see `install_winetricks_verbs`).
pub fn wine_binary(runner_path: &Path) -> Result<PathBuf, String> {
    find_wine_tool(runner_path, "wine64").or_else(|_| find_wine_tool(runner_path, "wine"))
}

/// Locates `wineserver` inside a runner folder, used to shut down a running
/// game's whole wine session (see `kill_running_game`).
pub fn wineserver_binary(runner_path: &Path) -> Result<PathBuf, String> {
    find_wine_tool(runner_path, "wineserver")
}

/// Checks whether we're actually running inside a distrobox/podman container
/// with `distrobox-host-exec` available. When developing inside such a
/// container (common on immutable hosts like Bazzite, where the container is
/// used for its dev headers but lacks a full 32-bit/GPU gaming userland),
/// this lets us run wine on the host instead, transparently.
///
/// Checking `PATH` alone isn't enough: on a host that ships distrobox
/// system-wide (Bazzite does), `distrobox-host-exec` is on `PATH` even for a
/// build running natively on that same host (e.g. the packaged AppImage) —
/// routing through it there finds no bridge to connect to and just fails.
/// `/run/.containerenv` is what podman (and so distrobox) creates inside a
/// container to mark it as one, which is what we actually care about here,
/// regardless of whether this is a debug or release build.
fn host_exec_available() -> bool {
    Path::new("/run/.containerenv").is_file()
        && std::env::var_os("PATH")
            .map(|paths| {
                std::env::split_paths(&paths).any(|dir| dir.join("distrobox-host-exec").is_file())
            })
            .unwrap_or(false)
}

/// Builds the command used to run a runner-provided binary (wine, or the
/// `proton` wrapper script) with the given environment variables, routed
/// through `distrobox-host-exec` when available (see `host_exec_available`)
/// so it executes against the host's libraries rather than the container's.
///
/// `distrobox-host-exec` forwards argv to the host but does not forward the
/// calling process's environment, so in that case the vars are instead passed
/// as literal `env KEY=VALUE ...` arguments ahead of the binary — that always
/// works regardless of what environment host-exec itself received.
pub fn runner_command<'a>(
    binary: &Path,
    env_vars: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Command {
    if host_exec_available() {
        let mut cmd = Command::new("distrobox-host-exec");
        cmd.arg("env");
        for (key, value) in env_vars {
            cmd.arg(format!("{key}={value}"));
        }
        cmd.arg(binary);
        cmd
    } else {
        let mut cmd = Command::new(binary);
        for (key, value) in env_vars {
            cmd.env(key, value);
        }
        cmd
    }
}

/// Builds the command that runs something inside `prefix_path` under
/// `runner` — callers append the exe (or builtin tool name, like `winecfg`)
/// and its arguments. Both binaries take it the same way (`wine <exe> ...`,
/// `umu-run <exe> ...`):
///
/// - A Proton runner goes through umu-run (see `commands::umu`), which
///   creates and sets up the prefix itself on first use.
/// - A Wine runner is driven by its own `wine` binary directly, with the
///   prefix's user profile steered to `steamuser` first (see
///   `steer_profile_to_steamuser`) in case this is what initializes it.
pub async fn prefix_command(
    app: &AppHandle,
    token: Option<&str>,
    runner: &Runner,
    prefix_path: &str,
) -> Result<Command, String> {
    match runner.kind {
        RunnerKind::Proton => {
            let umu_run = ensure_umu(app, token).await?;
            let proton_path = runner.path.to_str().ok_or_else(|| {
                format!("Runner path is not valid UTF-8: {}", runner.path.display())
            })?;
            Ok(runner_command(
                &umu_run,
                [("WINEPREFIX", prefix_path), ("PROTONPATH", proton_path)],
            ))
        }
        RunnerKind::Wine => {
            steer_profile_to_steamuser(Path::new(prefix_path))?;
            let wine = wine_binary(&runner.path)?;
            Ok(runner_command(&wine, [("WINEPREFIX", prefix_path)]))
        }
    }
}

#[tauri::command]
pub fn list_runners(state: State<ConfigState>) -> Result<Vec<Runner>, String> {
    let runners_dir = {
        let config = state
            .lock()
            .map_err(|_| "Configuration is locked".to_string())?;
        config.runners_dir.clone()
    };
    scan_runners(&runners_dir)
}

/// Deletes a runner's folder — only one no game uses, so no game is left
/// pointing at a runner that's gone. A runner that's a symlink into another
/// tool's folder (e.g. Steam's `compatibilitytools.d`) only loses the link.
/// Async, since a runner is several hundred MB of files.
#[tauri::command]
pub async fn delete_runner(state: State<'_, ConfigState>, runner_id: String) -> Result<(), String> {
    let (runner, users) = {
        let config = state
            .lock()
            .map_err(|_| "Configuration is locked".to_string())?;
        let runner = find_runner(&config.runners_dir, &runner_id)?;
        let users: Vec<String> = config
            .games
            .iter()
            .filter(|g| g.runner_id == runner_id)
            .map(|g| format!("„{}“", g.name))
            .collect();
        (runner, users)
    };
    if !users.is_empty() {
        return Err(format!(
            "{} wird noch verwendet von {}.",
            runner.name,
            users.join(", ")
        ));
    }
    tauri::async_runtime::spawn_blocking(move || fs::remove_dir_all(&runner.path))
        .await
        .map_err(|e| format!("Could not delete runner: {e}"))?
        .map_err(|e| format!("Could not delete runner: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deleting_a_linked_runner_keeps_its_target() {
        // What `delete_runner` relies on for a runner linked in from
        // elsewhere: `remove_dir_all` doesn't follow the link.
        let dir = std::env::temp_dir().join(format!("prefixr-test-{}", uuid::Uuid::new_v4()));
        let target = dir.join("steam/GE-Proton11-7");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("proton"), "").unwrap();
        let link = dir.join("runners/GE-Proton11-7");
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert_eq!(scan_runners(link.parent().unwrap()).unwrap().len(), 1);

        fs::remove_dir_all(&link).unwrap();
        assert!(!link.exists());
        assert!(target.join("proton").is_file());
        fs::remove_dir_all(&dir).unwrap();
    }
}
