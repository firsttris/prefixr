use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::commands::github::read_token;
use crate::commands::runner_downloads::{
    extraction_dir, move_extracted_dir, replace_dir, with_optional_auth,
};
use crate::config::ConfigState;
use crate::error::AppError;

/// A plain Wine build (unlike Proton) has no DXVK/VKD3D of its own — see
/// `sync_directx_overrides_from_cache` in `games.rs`. This downloads and
/// caches the latest release of each from the same upstream projects
/// PortProton itself packages (`doitsujin/dxvk`, `HansKristian-Work/vkd3d-proton`),
/// shared by every Wine-kind runner since the DXVK/VKD3D build needed
/// doesn't depend on which Wine build is running it. Fetched on first use,
/// and only replaced by a newer release when the user asks for it (see
/// `update_directx_layers`).

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

/// One of the cached layers.
struct Layer {
    /// Its directory in the cache.
    dir: &'static str,
    label: &'static str,
    repo: &'static str,
    /// Picks the release asset with the Windows DLLs.
    matches_asset: fn(&str) -> bool,
}

const LAYERS: &[Layer] = &[
    Layer {
        dir: "dxvk",
        label: "DXVK",
        repo: "doitsujin/dxvk",
        matches_asset: |name| {
            name.starts_with("dxvk-") && name.ends_with(".tar.gz") && !name.contains("native")
        },
    },
    Layer {
        dir: "vkd3d-proton",
        label: "VKD3D-Proton",
        repo: "HansKristian-Work/vkd3d-proton",
        matches_asset: |name| name.ends_with(".tar.zst"),
    },
];

/// Held while filling the cache, so two games launched at once don't both
/// download and extract the same thing into the same place.
static CACHE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// GETs `url` and fails on an error status, so a 404 or rate-limit page is
/// never taken for the file itself — everything here is cached for good once
/// it's on disk. A GitHub token, where given, counts the request against the
/// user's own rate limit instead of the low anonymous one.
async fn get(url: &str, token: Option<&str>) -> Result<reqwest::Response, String> {
    with_optional_auth(crate::http::client().get(url), token)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|e| format!("Could not download {url}: {e}"))
}

fn cache_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve data directory: {e}"))?
        .join("directx-layers"))
}

/// The release tag a cached layer came from, kept next to its DLLs.
fn version_file(layer_dir: &Path) -> PathBuf {
    layer_dir.join("version")
}

fn read_version(layer_dir: &Path) -> Option<String> {
    fs::read_to_string(version_file(layer_dir))
        .ok()
        .map(|version| version.trim().to_string())
        .filter(|version| !version.is_empty())
}

async fn latest_release(layer: &Layer, token: Option<&str>) -> Result<GitHubRelease, String> {
    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        layer.repo
    );
    get(&url, token)
        .await?
        .json()
        .await
        .map_err(|e| format!("Could not parse GitHub response: {e}"))
}

fn extract_archive(name: &str, bytes: &[u8], dest_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dest_dir)
        .map_err(|e| format!("Could not create {}: {e}", dest_dir.display()))?;
    if name.ends_with(".tar.gz") {
        let decompressed = flate2::read::GzDecoder::new(bytes);
        tar::Archive::new(decompressed)
            .unpack(dest_dir)
            .map_err(|e| format!("Could not extract {name}: {e}"))
    } else if name.ends_with(".tar.zst") {
        let decompressed = zstd::stream::read::Decoder::new(bytes)
            .map_err(|e| format!("Could not decompress {name}: {e}"))?;
        tar::Archive::new(decompressed)
            .unpack(dest_dir)
            .map_err(|e| format!("Could not extract {name}: {e}"))
    } else {
        Err(format!("Unsupported archive format: {name}"))
    }
}

/// Downloads `release` of `layer` and puts it in place of the cached copy,
/// if any (see `replace_dir`), with its tag as the version.
async fn install_layer(
    cache: &Path,
    layer: &Layer,
    release: GitHubRelease,
    token: Option<&str>,
) -> Result<(), String> {
    let asset = release
        .assets
        .into_iter()
        .find(|a| (layer.matches_asset)(&a.name))
        .ok_or_else(|| format!("No matching release asset found in {}", layer.repo))?;
    let bytes = get(&asset.browser_download_url, token)
        .await?
        .bytes()
        .await
        .map_err(|e| format!("Could not download {}: {e}", asset.name))?;

    // Decompressing tens of MB and deleting the old copy is blocking work,
    // kept off the async runtime that a game launch waits on.
    let cache = cache.to_path_buf();
    let label = layer.label;
    let target = cache.join(layer.dir);
    let tag = release.tag_name;
    tauri::async_runtime::spawn_blocking(move || {
        let extract_dir = extraction_dir(&cache);
        if let Err(e) = extract_archive(&asset.name, &bytes, &extract_dir) {
            let _ = fs::remove_dir_all(&extract_dir);
            return Err(e);
        }
        let staging = extraction_dir(&cache);
        move_extracted_dir(&extract_dir, &staging)?;
        let installed = fs::write(version_file(&staging), &tag)
            .map_err(|e| format!("Could not write {label} version file: {e}"))
            .and_then(|()| replace_dir(&staging, &target));
        if installed.is_err() {
            let _ = fs::remove_dir_all(&staging);
        }
        installed
    })
    .await
    .map_err(|e| format!("Extraction task failed: {e}"))?
}

/// Ensures DXVK and VKD3D-Proton are downloaded into the shared cache,
/// fetching each the first time it's needed (a no-op once both are already
/// there). Returns the cache directory; `<dir>/dxvk/{x32,x64}` and
/// `<dir>/vkd3d-proton/{x86,x64}` hold the actual DLLs.
pub async fn ensure_directx_layer_cache(
    app: &AppHandle,
    token: Option<&str>,
) -> Result<PathBuf, String> {
    let _lock = CACHE_LOCK.lock().await;
    let cache = cache_dir(app)?;
    for layer in LAYERS {
        if !cache.join(layer.dir).is_dir() {
            let release = latest_release(layer, token).await?;
            install_layer(&cache, layer, release, token).await?;
        }
    }
    Ok(cache)
}

/// A cached layer, as shown in the runner settings.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct DirectXLayerStatus {
    pub label: &'static str,
    pub installed: bool,
    /// The release tag it came from; `None` for a copy cached before the
    /// tag was recorded.
    pub version: Option<String>,
}

fn layer_statuses(cache: &Path) -> Vec<DirectXLayerStatus> {
    LAYERS
        .iter()
        .map(|layer| {
            let dir = cache.join(layer.dir);
            DirectXLayerStatus {
                label: layer.label,
                installed: dir.is_dir(),
                version: read_version(&dir),
            }
        })
        .collect()
}

#[tauri::command(async)]
pub fn get_directx_layers_status(app: AppHandle) -> Result<Vec<DirectXLayerStatus>, AppError> {
    Ok(layer_statuses(&cache_dir(&app)?))
}

/// Replaces each cached layer whose release isn't the latest one anymore.
/// Prefixes keep working throughout: their DLLs are links into the cache,
/// whose paths stay the same. A layer not cached yet stays that way, as
/// it's fetched on the first Wine launch anyway.
#[tauri::command]
pub async fn update_directx_layers(
    app: AppHandle,
    state: State<'_, ConfigState>,
) -> Result<Vec<DirectXLayerStatus>, AppError> {
    let token = read_token(&state);
    let _lock = CACHE_LOCK.lock().await;
    let cache = cache_dir(&app)?;
    for layer in LAYERS {
        let dir = cache.join(layer.dir);
        if !dir.is_dir() {
            continue;
        }
        let release = latest_release(layer, token.as_deref()).await?;
        if read_version(&dir).as_deref() != Some(release.tag_name.as_str()) {
            install_layer(&cache, layer, release, token.as_deref()).await?;
        }
    }
    Ok(layer_statuses(&cache))
}

/// Every `href="..."` attribute value in an HTML page, in document order —
/// good enough to read a plain directory listing (like dl.winehq.org's)
/// without pulling in a full HTML parser.
fn hrefs(html: &str) -> Vec<&str> {
    html.split("href=\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .collect()
}

const MONO_BASE_URL: &str = "https://dl.winehq.org/wine/wine-mono/";

/// Where a runner keeps `appwiz.cpl`, the module that installs wine-mono and
/// so knows which version this Wine build expects: a plain build's layout,
/// the `lib64` one some builds use, and Proton's under `files/`.
fn appwiz_candidates(runner_path: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for base in ["", "files/"] {
        for lib in ["lib/wine", "lib64/wine"] {
            for arch in ["x86_64-windows", "i386-windows"] {
                candidates.push(runner_path.join(format!("{base}{lib}/{arch}/appwiz.cpl")));
            }
        }
    }
    candidates
}

/// The wine-mono version a runner's Wine expects. Its `appwiz.cpl` holds
/// the installer's file name (`wine-mono-11.3.0-x86.msi`) as a UTF-16
/// string; `None` if no runner module has one.
fn required_mono_version(runner_path: &Path) -> Option<String> {
    appwiz_candidates(runner_path)
        .into_iter()
        .find_map(|path| mono_version_in(&fs::read(path).ok()?))
}

/// The version out of the first `wine-mono-<X.Y.Z>-x86.msi` in a module's
/// UTF-16 strings.
fn mono_version_in(module: &[u8]) -> Option<String> {
    let utf16 =
        |text: &str| -> Vec<u8> { text.encode_utf16().flat_map(u16::to_le_bytes).collect() };
    let needle = utf16("wine-mono-");
    let suffix = utf16("-x86.msi");
    let mut offset = 0;
    while let Some(found) = module[offset..]
        .windows(needle.len())
        .position(|window| window == needle.as_slice())
    {
        let start = offset + found + needle.len();
        offset = start;
        let version: String = module[start..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&pair| u16::from_le_bytes(pair))
            .map_while(|unit| {
                char::from_u32(unit.into()).filter(|c| c.is_ascii_digit() || *c == '.')
            })
            .collect();
        let end = start + version.len() * 2;
        if version.contains('.') && module[end..].starts_with(&suffix) {
            return Some(version);
        }
    }
    None
}

/// Any wine-mono installer already in the cache.
fn cached_mono_msi(cache: &Path) -> Option<PathBuf> {
    fs::read_dir(cache)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            name.starts_with("wine-mono-") && name.ends_with(".msi")
        })
}

/// The highest version among a listing's `X.Y.Z/` directory entries,
/// compared as numbers: an index sorted by name puts `9.4.0/` after
/// `10.0.0/`.
fn latest_version_dir(hrefs: &[&str]) -> Option<String> {
    hrefs
        .iter()
        .filter_map(|href| {
            let version = href.strip_suffix('/')?;
            let parts = version
                .split('.')
                .map(|part| part.parse::<u32>().ok())
                .collect::<Option<Vec<u32>>>()?;
            Some((parts, version))
        })
        .max_by(|(a, _), (b, _)| a.cmp(b))
        .map(|(_, version)| version.to_string())
}

/// The latest wine-mono release's version and installer file name. There's
/// no GitHub-releases API here, just a plain Apache directory listing with
/// one `X.Y.Z/` directory per version.
async fn latest_mono_release() -> Result<(String, String), String> {
    let index = get(MONO_BASE_URL, None)
        .await?
        .text()
        .await
        .map_err(|e| format!("Could not read wine-mono index: {e}"))?;

    let version = latest_version_dir(&hrefs(&index))
        .ok_or_else(|| "Could not find a wine-mono version in the listing".to_string())?;

    let version_index = get(&format!("{MONO_BASE_URL}{version}/"), None)
        .await?
        .text()
        .await
        .map_err(|e| format!("Could not read wine-mono {version} listing: {e}"))?;

    let msi_name = hrefs(&version_index)
        .into_iter()
        .find(|href| href.ends_with(".msi") && !href.contains("arm64"))
        .map(|href| href.to_string())
        .ok_or_else(|| format!("Could not find a wine-mono installer for {version}"))?;
    Ok((version, msi_name))
}

/// Downloads (caching it once fetched) the wine-mono `.msi` a runner's Wine
/// expects from WineHQ's own distribution — the same installer Wine's own
/// "couldn't find wine-mono" dialog offers to run — and returns its local
/// path. Each Wine release asks for one specific wine-mono version, so the
/// version comes from the runner itself (see `required_mono_version`). Only
/// if that can't be read does this fall back to any cached installer, or
/// else the latest release.
pub async fn ensure_wine_mono_msi(app: &AppHandle, runner_path: &Path) -> Result<PathBuf, String> {
    let _lock = CACHE_LOCK.lock().await;
    let cache = cache_dir(app)?;
    let (version, msi_name) = match required_mono_version(runner_path) {
        Some(version) => {
            let msi_name = format!("wine-mono-{version}-x86.msi");
            (version, msi_name)
        }
        None => {
            if let Some(cached) = cached_mono_msi(&cache) {
                return Ok(cached);
            }
            latest_mono_release().await?
        }
    };
    let msi_path = cache.join(&msi_name);
    if msi_path.is_file() {
        return Ok(msi_path);
    }

    let bytes = get(&format!("{MONO_BASE_URL}{version}/{msi_name}"), None)
        .await?
        .bytes()
        .await
        .map_err(|e| format!("Could not download {msi_name}: {e}"))?;

    // Written under another name and renamed, so an interrupted write never
    // leaves a truncated `.msi` for the lookups above to pick up. Blocking
    // (the installer is tens of MB), so off the async runtime.
    tauri::async_runtime::spawn_blocking(move || {
        fs::create_dir_all(&cache)
            .map_err(|e| format!("Could not create {}: {e}", cache.display()))?;
        let partial = cache.join(format!("{msi_name}.part"));
        fs::write(&partial, &bytes).map_err(|e| format!("Could not save {msi_name}: {e}"))?;
        fs::rename(&partial, &msi_path).map_err(|e| format!("Could not save {msi_name}: {e}"))?;
        Ok(msi_path)
    })
    .await
    .map_err(|e| format!("Saving wine-mono failed: {e}"))?
}

#[cfg(test)]
mod tests;
