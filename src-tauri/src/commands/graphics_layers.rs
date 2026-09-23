use std::fs;
use std::path::{Path, PathBuf};

use reqwest::header::USER_AGENT;
use serde::Deserialize;
use tauri::{AppHandle, Manager};

use crate::commands::runner_downloads::{extraction_dir, move_extracted_dir};

/// A plain Wine build (unlike Proton) has no DXVK/VKD3D of its own — see
/// `sync_directx_overrides_from_cache` in `games.rs`. This downloads and
/// caches the latest release of each from the same upstream projects
/// PortProton itself packages (`doitsujin/dxvk`, `HansKristian-Work/vkd3d-proton`),
/// shared by every Wine-kind runner since the DXVK/VKD3D build needed
/// doesn't depend on which Wine build is running it.

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct GitHubRelease {
    assets: Vec<GitHubAsset>,
}

/// Held while filling the cache, so two games launched at once don't both
/// download and extract the same thing into the same place.
static CACHE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// GETs `url` and fails on an error status, so a 404 or rate-limit page is
/// never taken for the file itself — everything here is cached for good once
/// it's on disk.
async fn get(url: &str) -> Result<reqwest::Response, String> {
    reqwest::Client::new()
        .get(url)
        .header(USER_AGENT, "prefixr")
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

async fn download_latest_asset(
    repo: &str,
    matches_asset: impl Fn(&str) -> bool,
) -> Result<(String, Vec<u8>), String> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let release: GitHubRelease = get(&url)
        .await?
        .json()
        .await
        .map_err(|e| format!("Could not parse GitHub response: {e}"))?;
    let asset = release
        .assets
        .into_iter()
        .find(|a| matches_asset(&a.name))
        .ok_or_else(|| format!("No matching release asset found in {repo}"))?;

    let bytes = get(&asset.browser_download_url)
        .await?
        .bytes()
        .await
        .map_err(|e| format!("Could not download {}: {e}", asset.name))?;

    Ok((asset.name, bytes.to_vec()))
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

async fn ensure_layer(
    app: &AppHandle,
    dir_name: &str,
    repo: &str,
    matches_asset: impl Fn(&str) -> bool,
) -> Result<PathBuf, String> {
    let target = cache_dir(app)?.join(dir_name);
    if target.is_dir() {
        return Ok(target);
    }

    let (asset_name, bytes) = download_latest_asset(repo, matches_asset).await?;

    let extract_dir = extraction_dir(&cache_dir(app)?);
    if let Err(e) = extract_archive(&asset_name, &bytes, &extract_dir) {
        let _ = fs::remove_dir_all(&extract_dir);
        return Err(e);
    }
    move_extracted_dir(&extract_dir, &target)?;

    Ok(target)
}

/// Ensures DXVK and VKD3D-Proton are downloaded into the shared cache,
/// fetching each the first time it's needed (a no-op once both are already
/// there). Returns the cache directory; `<dir>/dxvk/{x32,x64}` and
/// `<dir>/vkd3d-proton/{x86,x64}` hold the actual DLLs.
pub async fn ensure_directx_layer_cache(app: &AppHandle) -> Result<PathBuf, String> {
    let _lock = CACHE_LOCK.lock().await;
    ensure_layer(app, "dxvk", "doitsujin/dxvk", |name| {
        name.starts_with("dxvk-") && name.ends_with(".tar.gz") && !name.contains("native")
    })
    .await?;
    ensure_layer(
        app,
        "vkd3d-proton",
        "HansKristian-Work/vkd3d-proton",
        |name| name.ends_with(".tar.zst"),
    )
    .await?;
    cache_dir(app)
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

/// Downloads (caching it once fetched) the `.msi` for the latest wine-mono
/// release from WineHQ's own distribution — the same installer Wine's own
/// "couldn't find wine-mono" dialog offers to run — and returns its local
/// path. There's no GitHub-releases API here, just a plain Apache directory
/// listing per version; versions sort the same alphabetically as
/// numerically for as long as the major version stays a fixed number of
/// digits (true for the whole 10.x/11.x range so far), so the last `X.Y.Z/`
/// entry on the index page is the latest.
pub async fn ensure_wine_mono_msi(app: &AppHandle) -> Result<PathBuf, String> {
    let _lock = CACHE_LOCK.lock().await;
    let cache = cache_dir(app)?;
    if let Ok(entries) = fs::read_dir(&cache) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("wine-mono-") && name.ends_with(".msi") {
                return Ok(entry.path());
            }
        }
    }

    let index = get("https://dl.winehq.org/wine/wine-mono/")
        .await?
        .text()
        .await
        .map_err(|e| format!("Could not read wine-mono index: {e}"))?;

    let version = hrefs(&index)
        .into_iter()
        .rfind(|href| {
            href.ends_with('/')
                && href
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_digit())
                    .unwrap_or(false)
        })
        .map(|href| href.trim_end_matches('/').to_string())
        .ok_or_else(|| "Could not find a wine-mono version in the listing".to_string())?;

    let version_url = format!("https://dl.winehq.org/wine/wine-mono/{version}/");
    let version_index = get(&version_url)
        .await?
        .text()
        .await
        .map_err(|e| format!("Could not read wine-mono {version} listing: {e}"))?;

    let msi_name = hrefs(&version_index)
        .into_iter()
        .find(|href| href.ends_with(".msi") && !href.contains("arm64"))
        .map(|href| href.to_string())
        .ok_or_else(|| format!("Could not find a wine-mono installer for {version}"))?;

    let bytes = get(&format!("{version_url}{msi_name}"))
        .await?
        .bytes()
        .await
        .map_err(|e| format!("Could not download {msi_name}: {e}"))?;

    fs::create_dir_all(&cache).map_err(|e| format!("Could not create {}: {e}", cache.display()))?;
    // Written under another name and renamed, so an interrupted write never
    // leaves a truncated `.msi` for the lookup above to pick up.
    let msi_path = cache.join(&msi_name);
    let partial = cache.join(format!("{msi_name}.part"));
    fs::write(&partial, &bytes).map_err(|e| format!("Could not save {msi_name}: {e}"))?;
    fs::rename(&partial, &msi_path).map_err(|e| format!("Could not save {msi_name}: {e}"))?;
    Ok(msi_path)
}
