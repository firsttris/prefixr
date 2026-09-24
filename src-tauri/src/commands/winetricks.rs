use crate::error::AppError;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, SystemTime};

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::commands::games::{prepare_prefix, steer_profile_to_steamuser};
use crate::commands::github::read_token;
use crate::commands::logs::{new_log_file, prefix_log_dir};
use crate::commands::runners::{
    find_runner, prefix_command, runner_command, wine_binary, wineserver_binary,
};
use crate::config::ConfigState;
use crate::models::{Runner, RunnerKind};

fn winetricks_script_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve data directory: {e}"))?
        .join("winetricks")
        .join("winetricks"))
}

/// How long a downloaded winetricks is used before it's fetched again.
/// Winetricks downloads Microsoft's installers from fixed URLs and checks
/// them against fixed checksums, so an old copy starts failing as soon as
/// Microsoft moves or replaces a file — upstream follows within days.
const SCRIPT_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// Held around a download, which writes to one fixed `.part` file: listing
/// verbs and installing them can both ask for the script at once.
static SCRIPT_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Returns winetricks itself (see https://github.com/Winetricks/winetricks),
/// a single POSIX shell script, downloading it the first time it's needed
/// and again once it's older than `SCRIPT_MAX_AGE`. We shell out to the real
/// thing rather than reimplementing individual verbs ourselves: installing
/// e.g. a VC++ redistributable properly means actually running Microsoft's
/// real installer under Wine (silent flags, cab extraction, registry bits)
/// — exactly what winetricks already does reliably for hundreds of
/// packages, so reimplementing even a handful of verbs natively would just
/// be re-deriving winetricks worse.
async fn ensure_winetricks_script(app: &AppHandle) -> Result<PathBuf, String> {
    let path = winetricks_script_path(app)?;
    let _lock = SCRIPT_LOCK.lock().await;
    if !path.is_file() {
        download_script(&path).await?;
        return Ok(path);
    }
    let age = fs::metadata(&path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok());
    if age.is_some_and(|age| age >= SCRIPT_MAX_AGE) && download_script(&path).await.is_err() {
        // Offline, say: the old copy still works for most verbs. Marked as
        // fresh so the next try is in `SCRIPT_MAX_AGE` rather than on every
        // single use, each waiting for the request to fail.
        let _ = fs::File::options()
            .write(true)
            .open(&path)
            .and_then(|file| file.set_modified(SystemTime::now()));
    }
    Ok(path)
}

/// Downloads the current winetricks from its `master` branch to `path`.
async fn download_script(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    }

    // An error status fails here rather than getting saved as the script.
    let bytes = crate::http::client()
        .get("https://raw.githubusercontent.com/Winetricks/winetricks/master/src/winetricks")
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| format!("Could not download winetricks: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("Could not download winetricks: {e}"))?;

    // Made executable under another name first and renamed into place, so
    // `path` only ever exists as the complete script.
    let partial = path.with_extension("part");
    fs::write(&partial, &bytes)
        .map_err(|e| format!("Could not save {}: {e}", partial.display()))?;
    let mut perms = fs::metadata(&partial)
        .map_err(|e| format!("Could not read {} metadata: {e}", partial.display()))?
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&partial, perms)
        .map_err(|e| format!("Could not make {} executable: {e}", partial.display()))?;
    fs::rename(&partial, path).map_err(|e| format!("Could not save {}: {e}", path.display()))
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
pub struct WinetricksVerbMeta {
    pub id: String,
    pub category: String,
    pub title: String,
}

/// Parses `w_metadata <verb> <category> \` (plus the `title="..."` line right
/// after it) out of the winetricks script's own plain text — every verb
/// declares itself this way immediately before its `load_<verb>()` function,
/// so a straight text scan finds all of them without ever running the
/// script. Restricted to `dlls`/`fonts`: winetricks also has `apps`/`games`/
/// `benchmarks`/`settings` verbs, none of which are "a dependency a game is
/// missing" in the sense this dialog is for.
fn parse_verb_catalogue(script: &str) -> Vec<WinetricksVerbMeta> {
    let lines: Vec<&str> = script.lines().collect();
    let mut verbs = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let Some(rest) = line.strip_prefix("w_metadata ") else {
            continue;
        };
        let rest = rest.trim_end_matches('\\').trim();
        let mut parts = rest.splitn(2, char::is_whitespace);
        let Some(id) = parts.next() else { continue };
        let Some(category) = parts.next().map(str::trim) else {
            continue;
        };
        if category != "dlls" && category != "fonts" {
            continue;
        }
        let title = lines[i + 1..]
            .iter()
            .take(3)
            .find_map(|l| l.trim().strip_prefix("title=\"")?.split('"').next())
            .unwrap_or(id);
        verbs.push(WinetricksVerbMeta {
            id: id.to_string(),
            category: category.to_string(),
            title: title.to_string(),
        });
    }
    verbs
}

/// The full `dlls`/`fonts` catalogue (currently ~370 entries) — the frontend
/// shows a small curated subset front-and-center and this behind a "search
/// everything else" disclosure, rather than a flat 370-item list.
#[tauri::command]
pub async fn list_all_winetricks_verbs(app: AppHandle) -> Result<Vec<WinetricksVerbMeta>, AppError> {
    let script = ensure_winetricks_script(&app).await?;
    let text =
        fs::read_to_string(&script).map_err(|e| format!("Could not read winetricks script: {e}"))?;
    Ok(parse_verb_catalogue(&text))
}

/// Winetricks appends one line per successfully installed verb to
/// `<prefix>/winetricks.log` — its own idempotency record (it greps this same
/// file itself before reinstalling something). Reading it back lets the
/// frontend mark verbs already present in a prefix instead of leaving the
/// user to guess or just click install again. Missing file (nothing
/// installed in this prefix yet) is not an error — just an empty list.
#[tauri::command]
pub fn list_installed_winetricks_verbs(prefix_path: String) -> Result<Vec<String>, AppError> {
    match fs::read_to_string(Path::new(&prefix_path).join("winetricks.log")) {
        Ok(content) => Ok(content
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("Could not read winetricks.log: {e}").into()),
    }
}

/// Runs winetricks against a prefix with the given runner's wine, to install
/// one or more redistributable packages ("verbs", in winetricks' own
/// terminology — the frontend only offers a small curated subset of the
/// hundreds winetricks actually supports, matching `MangoHudConfig`'s and
/// `PerformanceConfig`'s "a few friendly knobs" philosophy). Keyed by
/// `prefix_path` + `runner_id` directly rather than a game id: winetricks
/// installs into `WINEPREFIX`, which is prefix-level state independent of
/// any one game — a prefix can be (and in this app's data model, is allowed
/// to be) shared by several games. The runner still has to be picked
/// explicitly, though, since a `Runner` (unlike a `Game`) isn't a WoW64/wine
/// build. `-q` puts winetricks in unattended mode — no license/confirmation
/// dialogs blocking a headless run. Returns the log file's path so the
/// frontend can offer it on failure, the same way `launch_game` does.
///
/// On a Proton runner that ships protonfixes (GE-Proton does), this goes
/// through `umu-run winetricks` instead, so the verbs get installed from
/// inside the same Steam Runtime container, by the same Proton setup, the
/// game itself later runs under — using the winetricks copy bundled in
/// protonfixes (umu requires that one; it adds `-q` itself).
#[tauri::command]
pub async fn install_winetricks_verbs(
    app: AppHandle,
    state: State<'_, ConfigState>,
    prefix_path: String,
    runner_id: String,
    verbs: Vec<String>,
) -> Result<String, AppError> {
    if verbs.is_empty() {
        return Err(AppError::NoPackagesSelected);
    }

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
    let log_out = fs::OpenOptions::new()
        .append(true)
        .open(&log_path)
        .map_err(|e| format!("Could not open log file: {e}"))?;
    let log_err = log_out
        .try_clone()
        .map_err(|e| format!("Could not open log file: {e}"))?;

    let mut command = if uses_umu_winetricks(&runner) {
        let mut command = prefix_command(&app, token.as_deref(), &runner, &prefix_path).await?;
        command.arg("winetricks").args(&verbs);
        command
    } else {
        // Readied as for a game first: winetricks would otherwise initialize
        // a fresh prefix itself, with Wine asking to download Mono and Gecko.
        prepare_prefix(&app, token.as_deref(), &runner, &prefix, &log_path, &|| {})
            .await
            .map_err(|e| AppError::WithLogDetails {
                message: e,
                log_path: log_path.display().to_string(),
            })?;
        direct_winetricks_command(&app, &runner, &prefix, &prefix_path, &verbs).await?
    };

    let status = command
        .stdout(Stdio::from(log_out))
        .stderr(Stdio::from(log_err))
        .status()
        .await
        .map_err(|e| format!("Could not run winetricks: {e}"))?;

    let log_path_string = log_path.display().to_string();
    if !status.success() {
        return Err(AppError::WinetricksFailed {
            status: status.to_string(),
            log_path: log_path_string,
        });
    }
    Ok(log_path_string)
}

/// Whether `umu-run winetricks` works for this runner: it only supports a
/// Proton build that bundles protonfixes, which is where both its winetricks
/// copy and the hook that actually runs it live.
fn uses_umu_winetricks(runner: &Runner) -> bool {
    runner.kind == RunnerKind::Proton && runner.path.join("protonfixes/winetricks").is_file()
}

/// Runs our own downloaded winetricks script directly against the runner's
/// wine binary — for a Wine runner, or a Proton build without protonfixes.
async fn direct_winetricks_command(
    app: &AppHandle,
    runner: &Runner,
    prefix: &Path,
    prefix_path: &str,
    verbs: &[String],
) -> Result<tokio::process::Command, String> {
    let wine = wine_binary(&runner.path)?;
    let wineserver = wineserver_binary(&runner.path)?;
    let wine_str = wine
        .to_str()
        .ok_or_else(|| format!("Wine path is not valid UTF-8: {}", wine.display()))?;
    let wineserver_str = wineserver.to_str().ok_or_else(|| {
        format!(
            "Wineserver path is not valid UTF-8: {}",
            wineserver.display()
        )
    })?;

    // Must run before wineboot's first initialization of this prefix — see
    // `steer_profile_to_steamuser` — which winetricks can trigger itself via
    // its own implicit `wine cmd /c echo init` if this is a fresh prefix.
    steer_profile_to_steamuser(prefix)?;

    let script = ensure_winetricks_script(app).await?;

    let mut command = runner_command(
        &script,
        [
            ("WINEPREFIX", prefix_path),
            ("WINE", wine_str),
            ("WINESERVER", wineserver_str),
        ],
    );
    command.arg("-q").args(verbs);
    Ok(command)
}
