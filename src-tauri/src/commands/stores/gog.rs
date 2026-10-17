//! GOG, through gogdl (see `helpers::GOGDL`) and GOG's own web APIs.
//!
//! gogdl is Heroic's GOG downloader. It keeps the login (`auth`), sizes up
//! and downloads games (`info`, `download`) and their shared redistributables
//! (`redist`), but has no game list and no uninstall: the list comes from
//! the same GOG Galaxy APIs Heroic uses, with the login's access token, and
//! uninstalling deletes the game's folder. A GOG game needs no tokens to
//! start, so its launch comes straight from the `goggame-<id>.info` file the
//! download writes into its folder.
//!
//! As in `epic`, everything that reads the tool's or the APIs' output is a
//! plain function, tested against fixtures (`gog/fixtures/`).

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::Deserialize;
use serde_json::Value;

use super::{StoreInstallInfo, StoreLaunch, StoreLibraryEntry};
use crate::host_env::host_command;
use crate::models::{split_launch_args, StorePrerequisites};

/// GOG Galaxy's own client id, which gogdl logs in with too (see its
/// `auth.py`): its login ends on `embed.gog.com/on_login_success?code=…`.
pub const CLIENT_ID: &str = "46899977096215655";

/// GOG's login page for that client.
pub const LOGIN_URL: &str = "https://auth.gog.com/auth?client_id=46899977096215655&redirect_uri=https%3A%2F%2Fembed.gog.com%2Fon_login_success%3Forigin%3Dclient&response_type=code&layout=galaxy";

/// The language games are installed in. GOG's own default; most games ship
/// it, and a game's other languages can still be picked in-game for most.
pub const LANGUAGE: &str = "en-US";

/// Runs gogdl with Prefixr's own login file and manifests folder, apart
/// from a Heroic the user may have installed.
pub struct Gogdl {
    pub binary: PathBuf,
    pub config_dir: PathBuf,
}

impl Gogdl {
    pub fn auth_file(&self) -> PathBuf {
        self.config_dir.join("auth.json")
    }

    /// A command for gogdl with `args`; stdin closed, as for Legendary.
    pub fn command<I, S>(&self, args: I) -> tokio::process::Command
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        let mut command = host_command(&self.binary);
        command
            .env("GOGDL_CONFIG_PATH", &self.config_dir)
            .arg("--auth-config-path")
            .arg(self.auth_file())
            .args(args.into_iter().map(Into::into))
            .stdin(Stdio::null());
        command
    }

    /// Runs gogdl and returns its stdout, or the error it logged.
    pub async fn output<I, S>(&self, args: I) -> Result<String, String>
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        let output = self
            .command(args)
            .output()
            .await
            .map_err(|e| format!("Could not run gogdl: {e}"))?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() {
            return Err(super::epic::error_message(&stderr)
                .unwrap_or_else(|| format!("gogdl exited with {}", output.status)));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

/// Where gogdl keeps the manifests of installed games (it needs them for
/// updates and repairs) when `GOGDL_CONFIG_PATH` is `config_dir`.
pub fn manifests_dir(config_dir: &Path) -> PathBuf {
    config_dir.join("heroic_gogdl").join("manifests")
}

/// The authorization code out of what the user pasted: the address the
/// login ended on (`https://embed.gog.com/on_login_success?origin=client&code=…`)
/// or the code itself.
pub fn auth_code(input: &str) -> Option<String> {
    let input = input.trim().trim_matches('"').trim();
    let code = if input.contains("code=") {
        let query = input.split_once('?').map_or(input, |(_, query)| query);
        query
            .split('&')
            .find_map(|pair| pair.strip_prefix("code="))?
            .split('#')
            .next()?
            .to_string()
    } else {
        input.to_string()
    };
    // GOG's codes are URL-safe base64; anything else is a paste of
    // something else.
    let plausible = code.len() >= 8
        && code
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    plausible.then_some(code)
}

/// The login gogdl holds, from `gogdl auth` (which refreshes it first when
/// it expired). `null`, or `{"error": true}` for a failed `auth --code`,
/// is no login.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Credentials {
    pub access_token: String,
    pub user_id: String,
}

pub fn parse_credentials(json: &str) -> Result<Option<Credentials>, String> {
    let value: Value = serde_json::from_str(json.trim())
        .map_err(|e| format!("Could not read gogdl's login: {e}"))?;
    if value.is_null()
        || value
            .get("error")
            .is_some_and(|error| error != &Value::Bool(false))
    {
        return Ok(None);
    }
    serde_json::from_value(value)
        .map(Some)
        .map_err(|e| format!("Could not read gogdl's login: {e}"))
}

/// Whether gogdl's login file holds a login for Galaxy's client, without
/// running gogdl (or reaching GOG): the settings page asks this on every
/// visit.
pub fn has_login(auth_json: &str) -> bool {
    serde_json::from_str::<Value>(auth_json)
        .ok()
        .and_then(|value| {
            value
                .get(CLIENT_ID)?
                .get("refresh_token")?
                .as_str()
                .map(str::to_string)
        })
        .is_some_and(|token| !token.is_empty())
}

/// The account's name, from `users.gog.com/users/<id>`.
pub fn parse_username(json: &str) -> Option<String> {
    let value: Value = serde_json::from_str(json).ok()?;
    let name = value.get("username")?.as_str()?.trim();
    (!name.is_empty()).then(|| name.to_string())
}

/// The account's GOG games on one page of
/// `galaxy-library.gog.com/users/<id>/releases` (which also lists games
/// from other stores connected to Galaxy), and the next page's token.
pub fn parse_library_page(json: &str) -> Result<(Vec<String>, Option<String>), String> {
    #[derive(Deserialize)]
    struct Item {
        platform_id: String,
        external_id: Value,
    }
    #[derive(Deserialize)]
    struct Page {
        #[serde(default)]
        items: Vec<Item>,
        #[serde(default)]
        next_page_token: Option<String>,
    }
    let page: Page =
        serde_json::from_str(json).map_err(|e| format!("Could not read GOG's game list: {e}"))?;
    let ids = page
        .items
        .into_iter()
        .filter(|item| item.platform_id == "gog")
        .filter_map(|item| match item.external_id {
            Value::String(id) => Some(id),
            Value::Number(id) => Some(id.to_string()),
            _ => None,
        })
        .collect();
    Ok((ids, page.next_page_token.filter(|token| !token.is_empty())))
}

/// A game's library entry from GOG's game database
/// (`gamesdb.gog.com/platforms/gog/external_releases/<id>`): its title and
/// portrait cover. `None` for what the library doesn't show (DLC, hidden
/// items) and for games without a Windows version, which Prefixr can't run.
pub fn parse_gamesdb(json: &str) -> Option<StoreLibraryEntry> {
    let value: Value = serde_json::from_str(json).ok()?;
    let game = value.get("game")?;
    let kind = value.get("type").and_then(Value::as_str)?;
    if !matches!(kind, "game" | "mod") {
        return None;
    }
    if game.get("visible_in_library").and_then(Value::as_bool) == Some(false) {
        return None;
    }
    let windows = value
        .get("supported_operating_systems")
        .and_then(Value::as_array)
        .is_some_and(|systems| {
            systems
                .iter()
                .any(|os| os.get("slug").and_then(Value::as_str) == Some("windows"))
        });
    if !windows {
        return None;
    }
    let any_title = |source: &Value| {
        source
            .get("title")
            .and_then(|title| title.get("*"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .map(str::to_string)
    };
    let app_name = match value.get("external_id")? {
        Value::String(id) => id.clone(),
        Value::Number(id) => id.to_string(),
        _ => return None,
    };
    let title = any_title(&value).or_else(|| any_title(game))?;
    let cover_url = ["vertical_cover", "cover"]
        .iter()
        .find_map(|kind| image_url(game.get(kind)?))
        .filter(|url| is_gog_image_url(url));
    Some(StoreLibraryEntry {
        app_name,
        title,
        cover_url,
    })
}

/// An image address from gamesdb's `url_format`
/// (`https://images.gog.com/<hash>{formatter}.{ext}?namespace=gamesdb`):
/// the original as a JPEG.
fn image_url(image: &Value) -> Option<String> {
    let format = image.get("url_format")?.as_str()?;
    Some(format.replace("{formatter}", "").replace("{ext}", "jpg"))
}

/// Whether `url` is on GOG's image servers, the only place GOG covers are
/// downloaded from.
pub fn is_gog_image_url(url: &str) -> bool {
    reqwest::Url::parse(url).is_ok_and(|url| {
        url.scheme() == "https"
            && url.host_str().is_some_and(|host| {
                ["gog.com", "gog-statics.com"]
                    .iter()
                    .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
            })
    })
}

/// What `gogdl info` tells about an install.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct InstallPlan {
    pub sizes: StoreInstallInfo,
    /// The folder the game is installed in, inside the install folder.
    pub folder_name: String,
    /// The shared redistributables it needs (`MSVC2017`, `DXSETUP`, …).
    pub dependencies: Vec<String>,
}

/// `gogdl info <id> --platform windows --lang en-US`'s JSON: sizes per
/// language (`*` is what every language needs), the folder and the
/// dependencies.
pub fn parse_info(json: &str) -> Result<InstallPlan, String> {
    let info: Value = serde_json::from_str(json.trim())
        .map_err(|e| format!("Could not read gogdl's game info: {e}"))?;
    let sizes = info.get("size").unwrap_or(&Value::Null);
    let size = |key: &str| {
        let part = |language: &str| sizes.get(language)?.get(key)?.as_u64();
        match (part("*"), part(LANGUAGE)) {
            (None, None) => None,
            (all, language) => Some(all.unwrap_or(0) + language.unwrap_or(0)),
        }
    };
    let folder_name = info
        .get("folder_name")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let dependencies = info
        .get("dependencies")
        .and_then(Value::as_array)
        .map(|ids| {
            ids.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    Ok(InstallPlan {
        sizes: StoreInstallInfo {
            download_size: size("download_size"),
            disk_size: size("disk_size"),
        },
        folder_name,
        dependencies,
    })
}

/// The arguments sizing up an install.
pub fn info_args(app_name: &str) -> Vec<String> {
    vec![
        "info".to_string(),
        app_name.to_string(),
        "--platform".to_string(),
        "windows".to_string(),
        "--lang".to_string(),
        LANGUAGE.to_string(),
    ]
}

/// The command line that installs a game into its own folder inside
/// `base_dir` (`download` appends the game's folder name itself), with the
/// DLC the account owns.
pub fn download_args(app_name: &str, base_dir: &Path) -> Vec<OsString> {
    vec![
        "download".into(),
        app_name.into(),
        "--platform".into(),
        "windows".into(),
        "--path".into(),
        base_dir.into(),
        "--lang".into(),
        LANGUAGE.into(),
        "--with-dlcs".into(),
    ]
}

/// The command line that downloads shared redistributables into
/// `redist_dir`, together with their manifest (`.gogdl-redist-manifest`).
pub fn redist_args(ids: &[String], redist_dir: &Path) -> Vec<OsString> {
    vec![
        "redist".into(),
        "--ids".into(),
        ids.join(",").into(),
        "--path".into(),
        redist_dir.into(),
    ]
}

pub const REDIST_MANIFEST: &str = ".gogdl-redist-manifest";

/// The installers of `wanted` redistributables, from gogdl's redist
/// manifest: absolute paths inside `redist_dir`, in the order the game
/// lists them. Entries without an installer (files games only copy) and
/// ids the manifest doesn't know are left out.
pub fn redist_installers(
    manifest_json: &str,
    wanted: &[String],
    redist_dir: &Path,
) -> Result<Vec<StorePrerequisites>, String> {
    #[derive(Deserialize)]
    struct Executable {
        #[serde(default)]
        path: String,
        #[serde(default)]
        arguments: String,
    }
    #[derive(Deserialize)]
    struct Depot {
        #[serde(rename = "dependencyId")]
        dependency_id: String,
        #[serde(rename = "readableName", default)]
        readable_name: String,
        executable: Executable,
    }
    #[derive(Deserialize)]
    struct Manifest {
        #[serde(default)]
        depots: Vec<Depot>,
    }
    let manifest: Manifest = serde_json::from_str(manifest_json)
        .map_err(|e| format!("Could not read gogdl's redistributables list: {e}"))?;
    let by_id: HashMap<&str, &Depot> = manifest
        .depots
        .iter()
        .map(|depot| (depot.dependency_id.as_str(), depot))
        .collect();
    Ok(wanted
        .iter()
        .filter_map(|id| by_id.get(id.as_str()))
        .filter(|depot| !depot.executable.path.trim().is_empty())
        .map(|depot| StorePrerequisites {
            name: if depot.readable_name.is_empty() {
                depot.dependency_id.clone()
            } else {
                depot.readable_name.clone()
            },
            path: redist_dir
                .join(windows_path(&depot.executable.path))
                .to_string_lossy()
                .into_owned(),
            args: depot.executable.arguments.clone(),
            installed: false,
        })
        .collect())
}

/// A relative Windows path (`__redist\MSVC2017\vc_redist.x64.exe`) as one
/// to join onto a Linux folder.
fn windows_path(path: &str) -> PathBuf {
    path.split(['\\', '/'])
        .filter(|part| !part.is_empty())
        .collect()
}

/// The `goggame-<id>.info` file of an installed game.
pub fn info_file(install_dir: &Path, app_name: &str) -> PathBuf {
    install_dir.join(format!("goggame-{app_name}.info"))
}

/// How a GOG game starts: the primary play task of its
/// `goggame-<id>.info`, as gogdl's own `launch` picks it — its exe, its
/// arguments (one string or a list) and its working folder, all relative to
/// the install folder and with Windows separators.
pub fn parse_play_task(info_json: &str, install_dir: &Path) -> Result<StoreLaunch, String> {
    let info: Value = serde_json::from_str(info_json)
        .map_err(|e| format!("Could not read the game's goggame info: {e}"))?;
    let tasks = info
        .get("playTasks")
        .and_then(Value::as_array)
        .ok_or("The game's goggame info lists nothing to start")?;
    let is_file_task = |task: &&Value| {
        task.get("type")
            .and_then(Value::as_str)
            .unwrap_or("FileTask")
            == "FileTask"
            && task.get("path").and_then(Value::as_str).is_some()
    };
    let task = tasks
        .iter()
        .filter(is_file_task)
        .find(|task| task.get("isPrimary").and_then(Value::as_bool) == Some(true))
        .or_else(|| tasks.iter().find(is_file_task))
        .ok_or("The game's goggame info lists no program to start")?;
    let path = task.get("path").and_then(Value::as_str).unwrap_or_default();
    let game_args = match task.get("arguments") {
        Some(Value::String(args)) => {
            split_launch_args(args).map_err(|_| format!("Unreadable arguments: {args}"))?
        }
        Some(Value::Array(args)) => args
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    };
    let working_dir = task
        .get("workingDir")
        .and_then(Value::as_str)
        .filter(|dir| !dir.trim().is_empty())
        .map_or_else(
            || install_dir.to_path_buf(),
            |dir| install_dir.join(windows_path(dir)),
        );
    let exe_path = install_dir.join(windows_path(path));
    Ok(StoreLaunch {
        working_dir: if working_dir.is_dir() {
            working_dir
        } else {
            case_insensitive(&working_dir)
        },
        exe_path: if exe_path.exists() {
            exe_path
        } else {
            case_insensitive(&exe_path)
        },
        game_args,
        account_args: Vec::new(),
        env: Vec::new(),
    })
}

/// `path` with each part matched against the folder it's in regardless of
/// case, the way Windows (and gogdl) finds files: a GOG game's info names
/// `Bin\Game.exe` for a `bin/game.exe` on disk now and then. Parts that
/// don't exist at all are kept as they are.
pub fn case_insensitive(path: &Path) -> PathBuf {
    let mut resolved = PathBuf::new();
    for part in path.components() {
        let exact = resolved.join(part);
        if exact.exists() || resolved.as_os_str().is_empty() {
            resolved = exact;
            continue;
        }
        let wanted = part.as_os_str().to_string_lossy().to_lowercase();
        let found = std::fs::read_dir(&resolved).ok().and_then(|entries| {
            entries
                .flatten()
                .find(|entry| entry.file_name().to_string_lossy().to_lowercase() == wanted)
        });
        resolved = match found {
            Some(entry) => entry.path(),
            None => exact,
        };
    }
    resolved
}

#[cfg(test)]
mod tests;
