use std::fs;
use std::path::{Path, PathBuf};

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
    crate::http::client()
        .get(url)
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
    let utf16 = |text: &str| -> Vec<u8> { text.encode_utf16().flat_map(u16::to_le_bytes).collect() };
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
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .map_while(|unit| char::from_u32(unit.into()).filter(|c| c.is_ascii_digit() || *c == '.'))
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
    fs::read_dir(cache).ok()?.flatten().map(|entry| entry.path()).find(|path| {
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        name.starts_with("wine-mono-") && name.ends_with(".msi")
    })
}

/// The latest wine-mono release's version and installer file name. There's
/// no GitHub-releases API here, just a plain Apache directory listing per
/// version; the index lists them in ascending version order, so the last
/// `X.Y.Z/` entry is the latest.
async fn latest_mono_release() -> Result<(String, String), String> {
    let index = get(MONO_BASE_URL)
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

    let version_index = get(&format!("{MONO_BASE_URL}{version}/"))
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

    let bytes = get(&format!("{MONO_BASE_URL}{version}/{msi_name}"))
        .await?
        .bytes()
        .await
        .map_err(|e| format!("Could not download {msi_name}: {e}"))?;

    fs::create_dir_all(&cache).map_err(|e| format!("Could not create {}: {e}", cache.display()))?;
    // Written under another name and renamed, so an interrupted write never
    // leaves a truncated `.msi` for the lookups above to pick up.
    let partial = cache.join(format!("{msi_name}.part"));
    fs::write(&partial, &bytes).map_err(|e| format!("Could not save {msi_name}: {e}"))?;
    fs::rename(&partial, &msi_path).map_err(|e| format!("Could not save {msi_name}: {e}"))?;
    Ok(msi_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16(text: &str) -> Vec<u8> {
        text.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }

    #[test]
    fn reads_the_mono_version_from_utf16_strings() {
        let mut module = b"\x00MZ wine-mono-9.9.9-x86.msi (ASCII, not a resource)".to_vec();
        module.extend(utf16("wine-mono-%s"));
        module.push(0);
        module.extend(utf16("wine-gecko-2.47.4-x86_64.msi\0wine-mono-11.3.0-x86.msi\0mono"));
        assert_eq!(mono_version_in(&module).as_deref(), Some("11.3.0"));
        assert_eq!(mono_version_in(&utf16("wine-mono-.msi")), None);
        assert_eq!(mono_version_in(b"nothing here"), None);
    }
}
