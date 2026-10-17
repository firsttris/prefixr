//! Amazon Games (Prime Gaming), through Nile (see `helpers::NILE`).
//!
//! Nile is the Amazon Games client the Heroic Games Launcher uses. It keeps
//! its login, library and installed games as JSON files in its config folder
//! (`NILE_CONFIG_PATH`), which Prefixr reads directly where Nile has no JSON
//! output for them. A login takes two steps: Nile makes a login request
//! (`auth --login --non-interactive`), the user signs in on its address, and
//! the address Amazon sends them to afterwards carries the code Nile
//! registers the device with (`register`).
//!
//! A game starts from its `fuel.json` (Amazon's launch description), with
//! Nile's `launch --json` adding the Amazon Games SDK's environment, which
//! games that check their entitlement read.
//!
//! As in `epic`, everything that reads Nile's output or files is a plain
//! function, tested against fixtures (`amazon/fixtures/`).

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{StoreInstallInfo, StoreLaunch, StoreLibraryEntry};
use crate::host_env::host_command;
use crate::models::StorePrerequisites;

/// Runs Nile with Prefixr's own config folder, apart from a Nile or Heroic
/// the user may have installed.
pub struct Nile {
    pub binary: PathBuf,
    pub config_dir: PathBuf,
}

impl Nile {
    /// Where Nile keeps its files (`NILE_CONFIG_PATH` + `nile`).
    pub fn files_dir(&self) -> PathBuf {
        files_dir(&self.config_dir)
    }

    /// A command for Nile with `args`; stdin closed, as for Legendary.
    pub fn command<I, S>(&self, args: I) -> tokio::process::Command
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        let mut command = host_command(&self.binary);
        command
            .env("NILE_CONFIG_PATH", &self.config_dir)
            .args(args.into_iter().map(Into::into))
            .stdin(Stdio::null());
        command
    }

    /// Runs Nile and returns its stdout and stderr, or the error it logged.
    pub async fn output<I, S>(&self, args: I) -> Result<(String, String), String>
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        let output = self
            .command(args)
            .output()
            .await
            .map_err(|e| format!("Could not run Nile: {e}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        if !output.status.success() {
            return Err(error_message(&stderr)
                .unwrap_or_else(|| format!("Nile exited with {}", output.status)));
        }
        Ok((stdout, stderr))
    }
}

/// Nile's files inside the folder given as `NILE_CONFIG_PATH`.
pub fn files_dir(config_dir: &Path) -> PathBuf {
    config_dir.join("nile")
}

/// The last error Nile logged (`ERROR [AUTH_MANAGER]:\t Failed to …`), or
/// when it crashed, the exception's message.
pub fn error_message(stderr: &str) -> Option<String> {
    let logged = stderr.lines().rev().find_map(|line| {
        let line = line.trim_start();
        ["ERROR [", "CRITICAL ["]
            .iter()
            .find(|level| line.starts_with(*level))
            .and_then(|_| line.split_once("]:"))
            .map(|(_, text)| text.trim())
            .filter(|text| !text.is_empty())
    });
    let crashed = || {
        stderr.lines().rev().find_map(|line| {
            let (kind, text) = line.trim().split_once(": ")?;
            let is_exception = !kind.contains(' ')
                && !kind.starts_with('[')
                && (kind.ends_with("Error") || kind.ends_with("Exception"));
            is_exception
                .then(|| text.trim())
                .filter(|text| !text.is_empty())
        })
    };
    logged.or_else(crashed).map(str::to_string)
}

/// The login Nile prepared (`auth --login --non-interactive`): the address
/// to sign in on, and what `register` needs again afterwards.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoginRequest {
    pub client_id: String,
    pub code_verifier: String,
    pub serial: String,
    pub url: String,
}

pub fn parse_login_request(json: &str) -> Result<LoginRequest, String> {
    let request: LoginRequest = serde_json::from_str(json.trim())
        .map_err(|e| format!("Could not read Nile's login request: {e}"))?;
    let amazon = reqwest::Url::parse(&request.url).is_ok_and(|url| {
        url.scheme() == "https"
            && url
                .host_str()
                .is_some_and(|host| host == "amazon.com" || host.ends_with(".amazon.com"))
    });
    if !amazon {
        return Err(format!("Unexpected login address: {}", request.url));
    }
    Ok(request)
}

/// The query parameter Amazon's login sends the code back in.
const CODE_PARAMETER: &str = "openid.oa2.authorization_code";

/// The authorization code out of what the user pasted: the address Amazon
/// sent them to after the login (`https://www.amazon.com/?…&openid.oa2.authorization_code=…`),
/// or the code itself.
pub fn auth_code(input: &str) -> Option<String> {
    let input = input.trim().trim_matches('"').trim();
    let code = if input.contains(CODE_PARAMETER) {
        let url = reqwest::Url::parse(input).ok()?;
        url.query_pairs()
            .find(|(key, _)| key == CODE_PARAMETER)
            .map(|(_, value)| value.into_owned())?
    } else {
        input.to_string()
    };
    let plausible = code.len() >= 8
        && code
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    plausible.then_some(code)
}

/// The arguments registering the device with the code, from the login
/// request.
pub fn register_args(request: &LoginRequest, code: &str) -> Vec<String> {
    vec![
        "register".to_string(),
        "--code".to_string(),
        code.to_string(),
        "--client-id".to_string(),
        request.client_id.clone(),
        "--code-verifier".to_string(),
        request.code_verifier.clone(),
        "--serial".to_string(),
        request.serial.clone(),
    ]
}

/// How a `register` ended.
#[derive(Debug, PartialEq)]
pub enum RegisterOutcome {
    Connected,
    /// Amazon refused the code (expired, used, or from another login).
    Refused,
    /// Amazon wasn't reached, or Nile failed otherwise; the error it logged.
    Failed(Option<String>),
}

/// The outcome of `register` from whether Nile's login files exist after it
/// (`logged_in`) and what it logged. A logged-in Nile counts as connected
/// even when the library sync after the login crashed: the next sync
/// catches up.
pub fn register_outcome(logged_in: bool, stderr: &str) -> RegisterOutcome {
    if logged_in {
        RegisterOutcome::Connected
    } else if stderr.contains("Failed to register a device") {
        RegisterOutcome::Refused
    } else {
        RegisterOutcome::Failed(error_message(stderr))
    }
}

/// Whether Nile failed for want of a login (`library sync` without one).
pub fn needs_login(message: &str) -> bool {
    message.contains("not logged in")
}

/// The logged-in account's name from Nile's `current_user.json`
/// (`{"name": "…", "user_id": "…"}`), which a login writes and a logout
/// deletes. Read directly rather than through `auth --status`, which would
/// reach Amazon on every look at the settings page.
pub fn parse_current_user(json: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct CurrentUser {
        name: Option<String>,
        user_id: Option<String>,
    }
    let user: CurrentUser = serde_json::from_str(json).ok()?;
    user.user_id.filter(|id| !id.trim().is_empty())?;
    Some(
        user.name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| "Amazon".to_string()),
    )
}

/// The login files in Nile's folder: `current_user.json` and the
/// encrypted tokens (`<md5 of the user id>.enc`). Deleting them logs out
/// without reaching Amazon; installed games and their manifests stay.
pub fn login_files(files_dir: &Path) -> Vec<PathBuf> {
    let mut files = vec![files_dir.join("current_user.json")];
    if let Ok(entries) = std::fs::read_dir(files_dir) {
        files.extend(
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.extension().is_some_and(|ext| ext == "enc")),
        );
    }
    files
}

/// The arguments syncing the library. Always in full, so games the account
/// lost leave the list too (an incremental sync only ever adds).
pub fn library_sync_args() -> [&'static str; 3] {
    ["library", "sync", "--force"]
}

/// The account's games from Nile's `library.json` (written by `library
/// sync`), sorted by title. A game without a title (Amazon has those) goes
/// by its id.
pub fn parse_library(json: &str) -> Result<Vec<StoreLibraryEntry>, String> {
    let games: Vec<Value> =
        serde_json::from_str(json).map_err(|e| format!("Could not read Nile's game list: {e}"))?;
    let mut entries: Vec<StoreLibraryEntry> = games
        .iter()
        .filter_map(|game| {
            let product = game.get("product")?;
            let id = product.get("id")?.as_str()?.to_string();
            let title = product
                .get("title")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|title| !title.is_empty())
                .map_or_else(|| id.clone(), str::to_string);
            let detail = product.get("productDetail");
            // Amazon has no portrait art; Heroic covers its cards with the
            // second background, then the square icon.
            let cover_url = [
                detail
                    .and_then(|d| d.get("details"))
                    .and_then(|d| d.get("backgroundUrl2")),
                detail.and_then(|d| d.get("iconUrl")),
                detail
                    .and_then(|d| d.get("details"))
                    .and_then(|d| d.get("backgroundUrl1")),
            ]
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .find(|url| is_amazon_image_url(url))
            .map(str::to_string);
            Some(StoreLibraryEntry {
                app_name: id,
                title,
                cover_url,
            })
        })
        .collect();
    entries.sort_by_key(|entry| entry.title.to_lowercase());
    entries.dedup_by(|a, b| a.app_name == b.app_name);
    Ok(entries)
}

/// Whether `url` is on Amazon's image servers, the only place Amazon covers
/// are downloaded from.
pub fn is_amazon_image_url(url: &str) -> bool {
    reqwest::Url::parse(url).is_ok_and(|url| {
        url.scheme() == "https"
            && url.host_str().is_some_and(|host| {
                ["media-amazon.com", "ssl-images-amazon.com", "amazon.com"]
                    .iter()
                    .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
            })
    })
}

/// The download size from `install --info --json <id>`. Nile doesn't tell
/// the installed size; it's about the same, as Amazon's packages aren't
/// compressed.
pub fn parse_install_info(json: &str) -> Result<StoreInstallInfo, String> {
    let info: Value = serde_json::from_str(json.trim())
        .map_err(|e| format!("Could not read Nile's game info: {e}"))?;
    let size = info.get("download_size").and_then(Value::as_u64);
    Ok(StoreInstallInfo {
        download_size: size,
        disk_size: size,
    })
}

/// The command line installing a game into a folder of its own (named
/// after its title) inside `base_dir`.
pub fn install_args(id: &str, base_dir: &Path) -> Vec<OsString> {
    vec![
        "install".into(),
        "--base-path".into(),
        base_dir.into(),
        id.into(),
    ]
}

/// The install folder of game `id` from Nile's `installed.json`.
pub fn parse_installed(json: &str, id: &str) -> Result<Option<PathBuf>, String> {
    #[derive(Deserialize)]
    struct Installed {
        id: String,
        path: String,
    }
    let games: Vec<Installed> = serde_json::from_str(json)
        .map_err(|e| format!("Could not read Nile's installed games: {e}"))?;
    Ok(games
        .into_iter()
        .find(|game| game.id == id)
        .map(|game| PathBuf::from(game.path)))
}

/// `fuel.json` as a JSON value. Amazon writes it in JSON5 now and then
/// (comments, trailing commas), which plain JSON refuses.
pub fn parse_fuel(raw: &str) -> Result<Value, String> {
    serde_json::from_str(&relaxed_json(raw)).map_err(|e| format!("Could not read fuel.json: {e}"))
}

/// The game's exe as `fuel.json` names it (`Main.Command`, relative to the
/// install folder).
pub fn fuel_exe(fuel: &Value, install_dir: &Path) -> Option<PathBuf> {
    let command = fuel.get("Main")?.get("Command")?.as_str()?.trim();
    (!command.is_empty()).then(|| install_dir.join(windows_path(command)))
}

/// The arguments asking Nile how a game starts, without starting it.
pub fn launch_args(id: &str) -> Vec<String> {
    vec![
        "launch".to_string(),
        "--json".to_string(),
        "--no-wine".to_string(),
        id.to_string(),
    ]
}

/// The launch from `launch --json` (the exe, its arguments and the SDK's
/// environment), with the working folder `fuel.json` may name
/// (`WorkingSubdirOverride`), which Nile's JSON leaves out.
pub fn parse_launch(json: &str, fuel: Option<&Value>) -> Result<StoreLaunch, String> {
    #[derive(Deserialize)]
    struct Command {
        instruction: String,
        #[serde(default)]
        arguments: Vec<String>,
    }
    #[derive(Deserialize)]
    struct Launch {
        command: Command,
        game_directory: String,
        #[serde(default)]
        env: std::collections::HashMap<String, String>,
    }
    let launch: Launch = serde_json::from_str(json.trim())
        .map_err(|e| format!("Could not read Nile's launch parameters: {e}"))?;
    if launch.command.instruction.trim().is_empty() {
        return Err("Nile named no executable to start".to_string());
    }
    let game_dir = PathBuf::from(&launch.game_directory);
    let working_dir = fuel
        .and_then(|fuel| fuel.get("Main")?.get("WorkingSubdirOverride")?.as_str())
        .filter(|dir| !dir.trim().is_empty())
        .map_or_else(|| game_dir.clone(), |dir| game_dir.join(windows_path(dir)));
    let mut env: Vec<(String, String)> = launch.env.into_iter().collect();
    env.sort();
    Ok(StoreLaunch {
        exe_path: PathBuf::from(&launch.command.instruction),
        game_args: launch.command.arguments,
        account_args: Vec::new(),
        working_dir,
        env,
    })
}

/// What `fuel.json` says to run after the install (`PostInstall`: Visual
/// C++ runtimes, DirectX and the like), as prerequisites relative to the
/// install folder.
pub fn post_install(fuel: &Value) -> Vec<StorePrerequisites> {
    let Some(steps) = fuel.get("PostInstall").and_then(Value::as_array) else {
        return Vec::new();
    };
    steps
        .iter()
        .filter_map(|step| {
            let command = step.get("Command")?.as_str()?.trim();
            if command.is_empty() {
                return None;
            }
            let args: Vec<&str> = step
                .get("Args")
                .and_then(Value::as_array)
                .map(|args| args.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            let path = windows_path(command);
            Some(StorePrerequisites {
                name: path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| command.to_string()),
                path: path.to_string_lossy().into_owned(),
                args: join_args(&args),
                installed: false,
            })
        })
        .collect()
}

/// Arguments as one Windows command line, the way `split_launch_args`
/// splits them again: quoted where they hold whitespace.
fn join_args(args: &[&str]) -> String {
    args.iter()
        .map(|arg| {
            if arg.is_empty() || arg.chars().any(char::is_whitespace) {
                format!("\"{arg}\"")
            } else {
                arg.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A relative Windows path as one to join onto a Linux folder.
fn windows_path(path: &str) -> PathBuf {
    path.split(['\\', '/'])
        .filter(|part| !part.is_empty() && *part != ".")
        .collect()
}

/// JSON5's leniencies taken out: `//` and `/* */` comments and trailing
/// commas, outside of strings.
pub fn relaxed_json(raw: &str) -> String {
    let raw = raw.trim_start_matches('\u{feff}');
    let chars: Vec<char> = raw.chars().collect();
    let mut out = String::with_capacity(raw.len());
    let mut i = 0;
    let mut in_string = false;
    while i < chars.len() {
        let c = chars[i];
        if in_string {
            out.push(c);
            if c == '\\' {
                if let Some(next) = chars.get(i + 1) {
                    out.push(*next);
                    i += 1;
                }
            } else if c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        match (c, chars.get(i + 1)) {
            ('"', _) => {
                in_string = true;
                out.push(c);
                i += 1;
            }
            ('/', Some('/')) => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            ('/', Some('*')) => {
                i += 2;
                while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                i += 2;
            }
            (',', _) => {
                let rest = chars[i + 1..].iter().find(|c| !c.is_whitespace());
                if !matches!(rest, Some('}') | Some(']')) {
                    out.push(c);
                }
                i += 1;
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
