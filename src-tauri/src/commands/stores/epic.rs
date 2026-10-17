//! The Epic Games Store, through Legendary (see `helpers::LEGENDARY`).
//!
//! Everything here either builds a Legendary command line or reads what
//! Legendary prints, so it's tested against Legendary's real output
//! (`epic/fixtures/`) without an Epic account. Legendary keeps its own state
//! (login, library metadata, installed games) in a config folder of
//! Prefixr's (`LEGENDARY_CONFIG_PATH`), apart from a Legendary or Heroic the
//! user may have installed themselves.

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::Deserialize;
use serde_json::Value;

use super::{StoreInstallInfo, StoreLaunch, StoreLibraryEntry};
use crate::host_env::host_command;
use crate::models::StorePrerequisites;

/// Where Legendary's own login page sends the user: Epic's login, which
/// ends on a JSON page holding the one-time `authorizationCode` to pass to
/// `legendary auth --code`.
pub const LOGIN_URL: &str = "https://legendary.gl/epiclogin";

/// What `legendary status` reports as the account when nobody is logged in.
const NOT_LOGGED_IN: &str = "<not logged in>";

/// Runs Legendary with Prefixr's own config folder.
pub struct Legendary {
    pub binary: PathBuf,
    pub config_dir: PathBuf,
}

impl Legendary {
    /// A command for Legendary with `args`. Its stdin is closed, so a
    /// prompt Legendary didn't expect to show fails instead of waiting
    /// forever for an answer nobody can type.
    pub fn command<I, S>(&self, args: I) -> tokio::process::Command
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        let mut command = host_command(&self.binary);
        command
            .env("LEGENDARY_CONFIG_PATH", &self.config_dir)
            .args(args.into_iter().map(Into::into))
            .stdin(Stdio::null());
        command
    }

    /// Runs Legendary and returns what it printed to stdout — its JSON, for
    /// the `--json` commands — or the error it logged.
    pub async fn output<I, S>(&self, args: I) -> Result<String, String>
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        let output = self
            .command(args)
            .output()
            .await
            .map_err(|e| format!("Could not run Legendary: {e}"))?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() {
            return Err(error_message(&stderr)
                .unwrap_or_else(|| format!("Legendary exited with {}", output.status)));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

/// The last error Legendary logged (`[cli] ERROR: Login failed, …`), its
/// logger name and level taken off — or, when it crashed instead, the
/// exception's message (`ValueError: No saved credentials`).
pub fn error_message(stderr: &str) -> Option<String> {
    let logged = stderr.lines().rev().find_map(|line| {
        ["ERROR:", "CRITICAL:", "FATAL:"]
            .iter()
            .find_map(|level| line.split_once(level).map(|(_, text)| text.trim()))
            .filter(|text| !text.is_empty())
    });
    let crashed = || {
        let last = stderr.lines().rev().find(|line| !line.trim().is_empty())?;
        let (kind, text) = last.split_once(": ")?;
        let is_exception =
            !kind.contains(' ') && (kind.ends_with("Error") || kind.ends_with("Exception"));
        is_exception
            .then(|| text.trim())
            .filter(|text| !text.is_empty())
    };
    logged.or_else(crashed).map(str::to_string)
}

/// Whether Legendary failed for want of a login: never logged in, logged
/// out, or the saved login expired.
pub fn needs_login(message: &str) -> bool {
    ["No saved credentials", "Login failed", "not logged in"]
        .iter()
        .any(|text| message.contains(text))
}

/// The authorization code out of what the user pasted: the code itself, the
/// code in quotes, or the whole JSON page Epic shows after the login.
pub fn auth_code(input: &str) -> Option<String> {
    let input = input.trim();
    let code = if input.starts_with('{') {
        let page: Value = serde_json::from_str(input).ok()?;
        page.get("authorizationCode")?.as_str()?.to_string()
    } else {
        input.trim_matches('"').trim().to_string()
    };
    // Epic's codes are 32 hex digits; anything with spaces or quotes in it
    // is a paste of something else.
    let plausible = !code.is_empty() && code.chars().all(|c| c.is_ascii_alphanumeric());
    plausible.then_some(code)
}

/// The logged-in account's display name from `legendary status --json`.
pub fn account_name(status_json: &str) -> Result<Option<String>, String> {
    #[derive(Deserialize)]
    struct Status {
        account: String,
    }
    let status: Status = serde_json::from_str(status_json)
        .map_err(|e| format!("Could not read Legendary's status: {e}"))?;
    Ok((status.account != NOT_LOGGED_IN && !status.account.is_empty()).then_some(status.account))
}

/// The arguments for `legendary list --json` over Windows games.
pub fn library_args() -> Vec<&'static str> {
    vec!["list", "--json", "--platform", "Windows"]
}

/// The account's games from `legendary list --json`, sorted by title. A
/// game's DLC comes nested under it there and isn't listed on its own.
pub fn parse_library(json: &str) -> Result<Vec<StoreLibraryEntry>, String> {
    #[derive(Deserialize)]
    struct ListedGame {
        app_name: String,
        app_title: String,
        #[serde(default)]
        metadata: Value,
    }
    let games: Vec<ListedGame> = serde_json::from_str(json)
        .map_err(|e| format!("Could not read Legendary's game list: {e}"))?;
    let mut entries: Vec<StoreLibraryEntry> = games
        .into_iter()
        .filter(|game| !game.app_name.is_empty())
        .map(|game| StoreLibraryEntry {
            cover_url: cover_url(&game.metadata),
            title: game.app_title.trim().to_string(),
            app_name: game.app_name,
        })
        .collect();
    entries.sort_by_key(|entry| entry.title.to_lowercase());
    Ok(entries)
}

/// The image types in a game's `keyImages`, best first: the portrait box
/// art the library's cards are shaped for, then the store's thumbnail
/// (portrait as well for most games), then the wide box art.
const COVER_TYPES: &[&str] = &[
    "DieselGameBoxTall",
    "Thumbnail",
    "DieselStoreFrontTall",
    "DieselGameBox",
];

/// The cover to show for a game, scaled down by Epic's CDN to what a card
/// needs (the originals are often several MB).
pub fn cover_url(metadata: &Value) -> Option<String> {
    let images = metadata.get("keyImages")?.as_array()?;
    let url = COVER_TYPES.iter().find_map(|wanted| {
        images
            .iter()
            .filter(|image| image.get("type").and_then(Value::as_str) == Some(*wanted))
            .filter_map(|image| image.get("url")?.as_str())
            .find(|url| is_epic_image_url(url))
    })?;
    let separator = if url.contains('?') { '&' } else { '?' };
    Some(format!("{url}{separator}h=600&w=400&resize=1"))
}

/// Whether `url` is an image on Epic's CDNs, the only place game covers
/// are downloaded from.
pub fn is_epic_image_url(url: &str) -> bool {
    reqwest::Url::parse(url).is_ok_and(|url| {
        url.scheme() == "https"
            && url.host_str().is_some_and(|host| {
                ["epicgames.com", "unrealengine.com", "epicgames.net"]
                    .iter()
                    .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
            })
    })
}

/// Download and install size from `legendary info --json`.
pub fn parse_install_info(json: &str) -> Result<StoreInstallInfo, String> {
    let info: Value = serde_json::from_str(json)
        .map_err(|e| format!("Could not read Legendary's game info: {e}"))?;
    let manifest = info.get("manifest").unwrap_or(&Value::Null);
    let size = |key: &str| manifest.get(key).and_then(Value::as_u64);
    Ok(StoreInstallInfo {
        download_size: size("download_size"),
        disk_size: size("disk_size"),
    })
}

/// The command line that installs a game into a folder of its own inside
/// `base_dir`. `-y` answers Legendary's questions the way the Epic launcher
/// would by default; `--skip-sdl` installs a game with optional parts
/// ("selective downloads") with only the parts it needs, since choosing
/// them would be another question; `--with-dlcs` installs the DLC the
/// account owns along with it.
pub fn install_args(app_name: &str, base_dir: &Path) -> Vec<OsString> {
    vec![
        "-y".into(),
        "install".into(),
        app_name.into(),
        "--base-path".into(),
        base_dir.into(),
        "--platform".into(),
        "Windows".into(),
        "--skip-sdl".into(),
        "--with-dlcs".into(),
    ]
}

/// The percentage from one of Legendary's progress lines
/// (`[DLManager] INFO: = Progress: 12.34% (123/1000), Running for …`), or
/// gogdl's, which are the same without the `%`
/// (`[PROGRESS] INFO: = Progress: 12.34 1234/10000, Running for …`).
pub fn parse_progress(line: &str) -> Option<f64> {
    let rest = line.split_once("= Progress:")?.1.trim_start();
    let number: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let percent: f64 = number.parse().ok()?;
    percent.is_finite().then_some(percent.clamp(0.0, 100.0))
}

/// What Prefixr keeps about an installed game, from `legendary
/// list-installed --json` (and its GOG counterpart, see `gog`).
#[derive(Debug, Clone, PartialEq)]
pub struct InstalledGame {
    pub install_path: PathBuf,
    pub exe_path: PathBuf,
    pub prerequisites: Vec<StorePrerequisites>,
}

/// The game `app_name` among `legendary list-installed --json`'s games.
pub fn parse_installed(json: &str, app_name: &str) -> Result<Option<InstalledGame>, String> {
    #[derive(Deserialize)]
    struct Prereq {
        #[serde(default)]
        name: String,
        #[serde(default)]
        path: String,
        #[serde(default)]
        args: String,
        #[serde(default)]
        installed: bool,
    }
    #[derive(Deserialize)]
    struct Installed {
        app_name: String,
        install_path: String,
        #[serde(default)]
        executable: String,
        #[serde(default)]
        prereq_info: Option<Prereq>,
    }
    let games: Vec<Installed> = serde_json::from_str(json)
        .map_err(|e| format!("Could not read Legendary's installed games: {e}"))?;
    Ok(games
        .into_iter()
        .find(|game| game.app_name == app_name)
        .map(|game| InstalledGame {
            exe_path: Path::new(&game.install_path).join(&game.executable),
            install_path: PathBuf::from(game.install_path),
            prerequisites: game
                .prereq_info
                .filter(|prereq| !prereq.path.trim().is_empty())
                .map(|prereq| StorePrerequisites {
                    name: prereq.name,
                    path: prereq.path,
                    args: prereq.args,
                    installed: prereq.installed,
                })
                .into_iter()
                .collect(),
        }))
}

/// The arguments asking Legendary how it would start a game, without
/// starting it. `offline` skips the login, for a game that can run without
/// one when Epic can't be reached.
pub fn launch_args(app_name: &str, offline: bool) -> Vec<String> {
    let mut args = vec![
        "launch".to_string(),
        app_name.to_string(),
        "--json".to_string(),
        // Prefixr doesn't update games yet; an outdated install should
        // still start rather than be refused.
        "--skip-version-check".to_string(),
    ];
    if offline {
        args.push("--offline".to_string());
    }
    args
}

/// The launch from `legendary launch --json`: the exe, the game's own
/// arguments, then the account's (`-AUTH_PASSWORD=…`, `-epicusername=…`),
/// with the user's own launch arguments meant to go between the two, as
/// Legendary itself orders them.
pub fn parse_launch(json: &str) -> Result<StoreLaunch, String> {
    #[derive(Deserialize)]
    struct Params {
        game_executable: String,
        game_directory: String,
        #[serde(default)]
        game_parameters: Vec<String>,
        #[serde(default)]
        egl_parameters: Vec<String>,
        #[serde(default)]
        working_directory: String,
        #[serde(default)]
        environment: HashMap<String, String>,
    }
    let params: Params = serde_json::from_str(json)
        .map_err(|e| format!("Could not read Legendary's launch parameters: {e}"))?;
    if params.game_executable.is_empty() {
        return Err("Legendary named no executable to start".to_string());
    }
    let directory = PathBuf::from(&params.game_directory);
    let working_dir = if params.working_directory.is_empty() {
        directory.clone()
    } else {
        PathBuf::from(&params.working_directory)
    };
    // The prefix is Prefixr's to choose; Legendary only names one when it's
    // configured to, which Prefixr's own Legendary isn't.
    let mut env: Vec<(String, String)> = params
        .environment
        .into_iter()
        .filter(|(key, _)| !matches!(key.as_str(), "WINEPREFIX" | "CX_BOTTLE"))
        .collect();
    env.sort();
    Ok(StoreLaunch {
        exe_path: directory.join(&params.game_executable),
        game_args: params.game_parameters,
        account_args: params.egl_parameters,
        working_dir,
        env,
    })
}

#[cfg(test)]
mod tests;
