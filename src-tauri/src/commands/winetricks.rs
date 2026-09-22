use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::header::USER_AGENT;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::commands::games::steer_profile_to_steamuser;
use crate::commands::runners::{find_runner, runner_command, wine_binary, wineserver_binary};
use crate::config::ConfigState;

fn winetricks_script_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve data directory: {e}"))?
        .join("winetricks")
        .join("winetricks"))
}

/// Downloads and caches winetricks itself (see
/// https://github.com/Winetricks/winetricks) — a single POSIX shell script,
/// no build step — the first time it's needed; a no-op afterwards. We shell
/// out to the real thing rather than reimplementing individual verbs
/// ourselves: installing e.g. a VC++ redistributable properly means actually
/// running Microsoft's real installer under Wine (silent flags, cab
/// extraction, registry bits) — exactly what winetricks already does
/// reliably for hundreds of packages, so reimplementing even a handful of
/// verbs natively would just be re-deriving winetricks worse.
async fn ensure_winetricks_script(app: &AppHandle) -> Result<PathBuf, String> {
    let path = winetricks_script_path(app)?;
    if path.is_file() {
        return Ok(path);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    }

    let bytes = reqwest::Client::new()
        .get("https://raw.githubusercontent.com/Winetricks/winetricks/master/src/winetricks")
        .header(USER_AGENT, "prefixr")
        .send()
        .await
        .map_err(|e| format!("Could not reach GitHub: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("Could not download winetricks: {e}"))?;

    fs::write(&path, &bytes).map_err(|e| format!("Could not save {}: {e}", path.display()))?;

    let mut perms = fs::metadata(&path)
        .map_err(|e| format!("Could not read {} metadata: {e}", path.display()))?
        .permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms)
        .map_err(|e| format!("Could not make {} executable: {e}", path.display()))?;

    Ok(path)
}

/// Logs live under `logs/prefixes/<sanitized-prefix-path>/`, mirroring
/// `games::log_file_path`'s `logs/<game_id>/` layout — deliberately keyed by
/// prefix, not by game: winetricks installs into `WINEPREFIX`, which can
/// (and often does) outlive or be shared across several games, so there's no
/// single game id that actually owns this log.
fn winetricks_log_path(app: &AppHandle, prefix_path: &Path) -> Result<PathBuf, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve data directory: {e}"))?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Could not read system time: {e}"))?
        .as_secs();
    let dir_name = prefix_path.to_string_lossy().replace(['/', '\\'], "_");
    Ok(data_dir
        .join("logs")
        .join("prefixes")
        .join(dir_name)
        .join(format!("{timestamp}.txt")))
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
pub async fn list_all_winetricks_verbs(app: AppHandle) -> Result<Vec<WinetricksVerbMeta>, String> {
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
pub fn list_installed_winetricks_verbs(prefix_path: String) -> Result<Vec<String>, String> {
    match fs::read_to_string(Path::new(&prefix_path).join("winetricks.log")) {
        Ok(content) => Ok(content
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("Could not read winetricks.log: {e}")),
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
#[tauri::command]
pub async fn install_winetricks_verbs(
    app: AppHandle,
    state: State<'_, ConfigState>,
    prefix_path: String,
    runner_id: String,
    verbs: Vec<String>,
) -> Result<String, String> {
    if verbs.is_empty() {
        return Err("Keine Pakete ausgewählt".to_string());
    }

    let runners_dir = {
        let config = state
            .lock()
            .map_err(|_| "Configuration is locked".to_string())?;
        config.runners_dir.clone()
    };

    let runner = find_runner(&runners_dir, &runner_id)?;
    let wine = wine_binary(&runner.path)?;
    let wineserver = wineserver_binary(&runner.path)?;
    let prefix = PathBuf::from(&prefix_path);
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
    steer_profile_to_steamuser(&prefix)?;

    let script = ensure_winetricks_script(&app).await?;

    let log_path = winetricks_log_path(&app, &prefix)?;
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Could not create log directory: {e}"))?;
    }
    let log_out =
        fs::File::create(&log_path).map_err(|e| format!("Could not create log file: {e}"))?;
    let log_err = log_out
        .try_clone()
        .map_err(|e| format!("Could not open log file: {e}"))?;

    let mut args = vec!["-q".to_string()];
    args.extend(verbs);

    let status = runner_command(
        &script,
        [
            ("WINEPREFIX", prefix_path.as_str()),
            ("WINE", wine_str),
            ("WINESERVER", wineserver_str),
        ],
    )
    .args(&args)
    .stdout(Stdio::from(log_out))
    .stderr(Stdio::from(log_err))
    .status()
    .await
    .map_err(|e| format!("Could not run winetricks: {e}"))?;

    let log_path_string = log_path.display().to_string();
    if !status.success() {
        return Err(format!(
            "winetricks beendete sich mit Status {status} — Details im Log: {log_path_string}"
        ));
    }
    Ok(log_path_string)
}
