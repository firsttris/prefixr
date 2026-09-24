use crate::error::AppError;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager, State};

use crate::commands::github::read_token;
use crate::config::ConfigState;

/// umu-launcher (https://github.com/Open-Wine-Components/umu-launcher) is
/// what every Proton runner is launched through — see `launch_game`. It runs
/// the runner's real `proton` script inside the Steam Linux Runtime
/// container, exactly the way Steam itself does, and applies
/// umu-protonfixes. That's what makes `PROTON_*` env vars, fsync, DXVK-NVAPI
/// etc. actually work, none of which do when a Proton build's wine binary is
/// run directly. Lutris, Heroic and Faugus all launch Proton this way.
///
/// We always use our own managed copy (the release's "zipapp": a single
/// ~400 KB Python file) rather than a system-wide `umu-run`: a distro
/// package can lag behind what a current Proton build needs (GE-Proton 11
/// requires the steamrt4 runtime, which older umu versions don't know
/// about), and a copy under our own data dir is equally reachable from
/// inside a distrobox dev container and from the host (see
/// `runner_command`).
const UMU_REPO: &str = "Open-Wine-Components/umu-launcher";

fn umu_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve data directory: {e}"))?
        .join("umu-launcher"))
}

/// The zipapp tarball unpacks to `umu/umu-run`.
fn umu_run_path(dir: &Path) -> PathBuf {
    dir.join("umu").join("umu-run")
}

fn version_file(dir: &Path) -> PathBuf {
    dir.join("version")
}

fn release_zipapp_asset(assets: &[GitHubAsset]) -> Option<&GitHubAsset> {
    assets.iter().find(|asset| asset.name.ends_with("-zipapp.tar"))
}

fn asset_sha256_hex(asset: &GitHubAsset) -> Option<&str> {
    asset.digest.as_deref()?.strip_prefix("sha256:")
}

#[derive(Debug, Clone, Serialize)]
pub struct UmuStatus {
    pub installed: bool,
    /// The GitHub release tag the installed copy came from.
    pub version: Option<String>,
}

fn read_status(dir: &Path) -> UmuStatus {
    let installed = umu_run_path(dir).is_file();
    let version = installed
        .then(|| fs::read_to_string(version_file(dir)).ok())
        .flatten()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());
    UmuStatus { installed, version }
}

fn required_runtime_name(manifest: &str) -> Option<&'static str> {
    let app_id = manifest.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("\"require_tool_appid\"")?;
        Some(rest.trim().trim_matches('"').to_string())
    });
    match app_id.as_deref() {
        Some("4183110") => Some("steamrt4"),
        Some("1628350") => Some("steamrt3"),
        Some("1391110") => Some("steamrt2"),
        _ => None,
    }
}

fn runtime_dir_complete(dir: &Path) -> bool {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .any(|entry| entry.file_name().to_string_lossy().contains("_platform_"))
        })
        .unwrap_or(false)
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    /// `sha256:<hex>`, computed by GitHub itself on upload. umu doesn't
    /// publish a separate checksum file for its zipapp, so this is what the
    /// download gets verified against.
    digest: Option<String>,
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

fn with_auth(builder: reqwest::RequestBuilder, token: Option<&str>) -> reqwest::RequestBuilder {
    match token {
        Some(token) => builder.bearer_auth(token),
        None => builder,
    }
}

async fn latest_release(token: Option<&str>) -> Result<GitHubRelease, String> {
    let url = format!("https://api.github.com/repos/{UMU_REPO}/releases/latest");
    let response = with_auth(crate::http::client().get(url), token)
        .send()
        .await
        .map_err(|e| format!("Could not reach GitHub: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("GitHub API returned status {}", response.status()));
    }
    response
        .json()
        .await
        .map_err(|e| format!("Could not parse GitHub response: {e}"))
}

/// Downloads the latest umu release's zipapp, verifies it and swaps it in
/// place of whatever copy was there before. Unpacked into a staging
/// directory first, so a failed or interrupted update never leaves a
/// half-extracted copy behind in place of a working one.
async fn install_latest(app: &AppHandle, token: Option<&str>) -> Result<UmuStatus, String> {
    let release = latest_release(token).await?;

    let asset = release_zipapp_asset(&release.assets)
        .ok_or_else(|| format!("umu {} has no zipapp release asset", release.tag_name))?;
    let expected = asset_sha256_hex(asset)
        .ok_or_else(|| format!("GitHub reported no SHA-256 digest for {}", asset.name))?
        .to_string();

    let bytes = with_auth(crate::http::client().get(&asset.browser_download_url), token)
        .send()
        .await
        .map_err(|e| format!("Could not download {}: {e}", asset.name))?
        .bytes()
        .await
        .map_err(|e| format!("Could not download {}: {e}", asset.name))?;

    let actual = format!("{:x}", Sha256::digest(&bytes));
    if !actual.eq_ignore_ascii_case(&expected) {
        return Err(format!(
            "Checksum mismatch for {}: expected {expected}, got {actual}",
            asset.name
        ));
    }

    let dir = umu_dir(app)?;
    let staging = dir.with_extension("new");
    if staging.exists() {
        fs::remove_dir_all(&staging)
            .map_err(|e| format!("Could not clean up {}: {e}", staging.display()))?;
    }
    fs::create_dir_all(&staging)
        .map_err(|e| format!("Could not create {}: {e}", staging.display()))?;
    tar::Archive::new(bytes.as_ref())
        .unpack(&staging)
        .map_err(|e| format!("Could not extract {}: {e}", asset.name))?;
    if !umu_run_path(&staging).is_file() {
        return Err(format!("{} did not contain umu/umu-run", asset.name));
    }
    fs::write(version_file(&staging), &release.tag_name)
        .map_err(|e| format!("Could not write umu version file: {e}"))?;

    if dir.exists() {
        fs::remove_dir_all(&dir)
            .map_err(|e| format!("Could not remove old {}: {e}", dir.display()))?;
    }
    fs::rename(&staging, &dir)
        .map_err(|e| format!("Could not move umu into {}: {e}", dir.display()))?;

    Ok(read_status(&dir))
}

/// Held around `install_latest`, which stages into one fixed directory: two
/// games launched at once on a fresh system would otherwise both download
/// umu and delete each other's half-extracted copy.
static INSTALL_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// Returns the path to our managed `umu-run`, downloading it the first time
/// it's needed (a no-op afterwards — updating is an explicit user action,
/// see `install_umu`).
pub async fn ensure_umu(app: &AppHandle, token: Option<&str>) -> Result<PathBuf, String> {
    let dir = umu_dir(app)?;
    let path = umu_run_path(&dir);
    if !path.is_file() {
        let _lock = INSTALL_LOCK.lock().await;
        // Another launch may have installed it while this one waited.
        if !path.is_file() {
            install_latest(app, token).await?;
        }
    }
    Ok(path)
}

/// umu's own data directory, where it keeps the Steam Runtime(s) it
/// downloads — shared with every other umu-based launcher on the system
/// (Lutris, Heroic, ...), so a runtime any of them already fetched is reused
/// as-is. Mirrors `UMU_LOCAL` in umu's `umu_consts.py`.
fn umu_local_dir() -> Option<PathBuf> {
    if let Some(folders) = std::env::var_os("UMU_FOLDERS_PATH") {
        return Some(PathBuf::from(folders).join("umu"));
    }
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))?;
    Some(data_home.join("umu"))
}

/// Whether the Steam Runtime a Proton runner requires (its
/// `toolmanifest.vdf`'s `require_tool_appid`) is already downloaded. If not,
/// umu fetches it — several hundred MB — the first time that runner is
/// used, which `launch_game` surfaces as the "initializing" state rather
/// than leaving the game looking like it silently failed to start. Unknown
/// or unparsable manifests count as present: this is only a UI hint, umu
/// itself is the authority on what gets downloaded.
pub fn runtime_present(runner_path: &Path) -> bool {
    let Ok(manifest) = fs::read_to_string(runner_path.join("toolmanifest.vdf")) else {
        return true;
    };
    let Some(runtime) = required_runtime_name(&manifest) else {
        return true;
    };
    // umu's own completeness check: an interrupted download leaves the
    // runtime's directory behind, but no `<name>_platform_<version>` inside.
    umu_local_dir().is_none_or(|dir| runtime_dir_complete(&dir.join(runtime)))
}

#[tauri::command]
pub fn get_umu_status(app: AppHandle) -> Result<UmuStatus, AppError> {
    Ok(read_status(&umu_dir(&app)?))
}

/// The tag of umu's latest release, so the settings can offer an update
/// when it's newer than the installed `UmuStatus::version`.
#[tauri::command]
pub async fn latest_umu_version(state: State<'_, ConfigState>) -> Result<String, AppError> {
    let token = read_token(&state)?;
    Ok(latest_release(token.as_deref()).await?.tag_name)
}

/// Installs the latest umu release, replacing the current copy if there is
/// one — also how an existing install gets updated.
#[tauri::command]
pub async fn install_umu(
    app: AppHandle,
    state: State<'_, ConfigState>,
) -> Result<UmuStatus, AppError> {
    let token = read_token(&state)?;
    let _lock = INSTALL_LOCK.lock().await;
    install_latest(&app, token.as_deref()).await.map_err(AppError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn temp_path() -> PathBuf {
        std::env::temp_dir().join(format!("prefixr-test-{}", uuid::Uuid::new_v4()))
    }

    fn zipapp_asset(name: &str, digest: Option<&str>) -> GitHubAsset {
        GitHubAsset {
            name: name.to_string(),
            browser_download_url: "https://example.invalid/umu.tar".to_string(),
            digest: digest.map(str::to_string),
        }
    }

    #[test]
    fn release_helpers_pick_zipapp_and_digest() {
        let assets = vec![
            zipapp_asset("umu-runner.tar.gz", Some("sha256:ignored")),
            zipapp_asset("umu-0.1.0-zipapp.tar", Some("sha256:abc123")),
        ];

        let asset = release_zipapp_asset(&assets).unwrap();

        assert_eq!(asset.name, "umu-0.1.0-zipapp.tar");
        assert_eq!(asset_sha256_hex(asset), Some("abc123"));
    }

    #[test]
    fn release_helpers_reject_missing_or_malformed_digest() {
        let missing = zipapp_asset("umu-0.1.0-zipapp.tar", None);
        let malformed = zipapp_asset("umu-0.1.0-zipapp.tar", Some("md5:abc123"));

        assert_eq!(asset_sha256_hex(&missing), None);
        assert_eq!(asset_sha256_hex(&malformed), None);
    }

    #[test]
    fn read_status_requires_umu_run_and_trims_versions() {
        let dir = temp_path();
        fs::create_dir_all(umu_run_path(&dir).parent().unwrap()).unwrap();
        fs::write(umu_run_path(&dir), "#!/bin/sh\n").unwrap();
        fs::write(version_file(&dir), "  v1.2.3 \n").unwrap();

        let status = read_status(&dir);

        assert!(status.installed);
        assert_eq!(status.version.as_deref(), Some("v1.2.3"));

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn read_status_ignores_empty_versions_and_missing_install() {
        let dir = temp_path();
        fs::create_dir_all(&dir).unwrap();
        fs::write(version_file(&dir), "   \n").unwrap();

        let status = read_status(&dir);

        assert!(!status.installed);
        assert_eq!(status.version, None);

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn required_runtime_name_maps_known_app_ids() {
        assert_eq!(required_runtime_name("\"require_tool_appid\" \"4183110\""), Some("steamrt4"));
        assert_eq!(required_runtime_name("\"require_tool_appid\" \"1628350\""), Some("steamrt3"));
        assert_eq!(required_runtime_name("\"require_tool_appid\" \"1391110\""), Some("steamrt2"));
        assert_eq!(required_runtime_name("\"require_tool_appid\" \"999\""), None);
        assert_eq!(required_runtime_name("\"other\" \"4183110\""), None);
    }

    #[test]
    fn runtime_dir_complete_needs_platform_marker() {
        let dir = temp_path();
        fs::create_dir_all(&dir).unwrap();
        assert!(!runtime_dir_complete(&dir));
        fs::create_dir_all(dir.join("steamrt4_payload")).unwrap();
        assert!(!runtime_dir_complete(&dir));
        fs::create_dir_all(dir.join("steamrt4_platform_4.0.20260914.260627")).unwrap();
        assert!(runtime_dir_complete(&dir));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn umu_local_dir_prefers_explicit_folders_path() {
        let _lock = ENV_LOCK.lock().unwrap();
        let base = temp_path();
        std::env::set_var("UMU_FOLDERS_PATH", &base);
        std::env::remove_var("XDG_DATA_HOME");
        std::env::remove_var("HOME");

        assert_eq!(umu_local_dir(), Some(base.join("umu")));

        std::env::remove_var("UMU_FOLDERS_PATH");
    }

    #[test]
    fn umu_local_dir_falls_back_to_xdg_data_home() {
        let _lock = ENV_LOCK.lock().unwrap();
        let base = temp_path();
        std::env::remove_var("UMU_FOLDERS_PATH");
        std::env::set_var("XDG_DATA_HOME", &base);
        std::env::remove_var("HOME");

        assert_eq!(umu_local_dir(), Some(base.join("umu")));

        std::env::remove_var("XDG_DATA_HOME");
    }

    #[test]
    fn umu_local_dir_uses_home_when_xdg_data_home_is_missing() {
        let _lock = ENV_LOCK.lock().unwrap();
        let home = temp_path();
        std::env::remove_var("UMU_FOLDERS_PATH");
        std::env::remove_var("XDG_DATA_HOME");
        std::env::set_var("HOME", &home);

        assert_eq!(umu_local_dir(), Some(home.join(".local/share/umu")));

        std::env::remove_var("HOME");
    }

    #[test]
    fn runtime_present_parses_real_manifest() {
        let _lock = ENV_LOCK.lock().unwrap();
        let dir = temp_path();
        fs::create_dir_all(&dir).unwrap();
        // Verbatim from GE-Proton11-7.
        fs::write(
            dir.join("toolmanifest.vdf"),
            "\"manifest\"\n{\n  \"version\" \"2\"\n  \"commandline\" \"/proton %verb%\"\n  \"require_tool_appid\" \"4183110\"\n  \"use_sessions\" \"1\"\n}\n",
        )
        .unwrap();

        let umu_home = dir.join("folders");
        std::env::set_var("UMU_FOLDERS_PATH", &umu_home);
        assert!(!runtime_present(&dir));
        fs::create_dir_all(umu_home.join("umu/steamrt4")).unwrap();
        assert!(!runtime_present(&dir));
        fs::create_dir_all(umu_home.join("umu/steamrt4/steamrt4_platform_4.0.20260914.260627"))
            .unwrap();
        assert!(runtime_present(&dir));
        std::env::remove_var("UMU_FOLDERS_PATH");

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn runtime_present_without_manifest_is_assumed() {
        assert!(runtime_present(Path::new("/nonexistent/runner")));
    }
}
