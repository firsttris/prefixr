//! The command line tools Prefixr drives a store with.
//!
//! Each store is reached through a tool from the Heroic Games Launcher
//! project (Legendary for Epic, gogdl for GOG, Nile for Amazon), which keeps up with the stores' APIs, download formats and login
//! flows. Prefixr doesn't bundle them: a tool is downloaded the first time
//! its store is connected, from a release pinned here by version and SHA-256,
//! so an update is a reviewed change to this file rather than whatever a
//! release page serves that day. Disconnecting the store deletes it again.
//!
//! Legendary's and gogdl's Linux builds are Python zipapps (a single file
//! starting with `#!/usr/bin/env python3`, its dependencies inside), just
//! like the umu-launcher Prefixr already uses, so they need nothing but a
//! system Python 3. Nile's is a self-contained PyInstaller executable,
//! which brings its own Python.

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager};

use crate::commands::games::command_on_path;
use crate::error::AppError;

/// One build of a tool, for one CPU architecture.
pub struct HelperAsset {
    /// As `std::env::consts::ARCH` names it.
    pub arch: &'static str,
    pub url: &'static str,
    pub sha256: &'static str,
}

/// A tool at a pinned version.
pub struct Helper {
    /// Its file name, `legendary`.
    pub name: &'static str,
    /// Its name as its project writes it, for the UI.
    pub display_name: &'static str,
    pub version: &'static str,
    pub assets: &'static [HelperAsset],
    /// The tool's source, linked from the docs and the about notice (the
    /// tools are GPL-3.0, like Prefixr).
    pub source_url: &'static str,
    /// Whether it runs on the system's Python (a zipapp).
    pub needs_python: bool,
}

/// Heroic's fork of Legendary, which Heroic itself ships and keeps working
/// against Epic's current APIs (the original project is no longer updated).
pub const LEGENDARY: Helper = Helper {
    name: "legendary",
    display_name: "Legendary",
    version: "0.20.43",
    assets: &[
        HelperAsset {
            arch: "x86_64",
            url: "https://github.com/Heroic-Games-Launcher/legendary/releases/download/0.20.43/legendary_linux_x86_64",
            sha256: "2b82497051afd95670994146e6038d6e1c98a1c60c21949def668b52aef7d3f7",
        },
        HelperAsset {
            arch: "aarch64",
            url: "https://github.com/Heroic-Games-Launcher/legendary/releases/download/0.20.43/legendary_linux_arm64",
            sha256: "e28d4bed5d7635e22093d62ed4686a4d66e9c1d8bc4f1b731f834cd16d43c86a",
        },
    ],
    source_url: "https://github.com/Heroic-Games-Launcher/legendary",
    needs_python: true,
};

/// Heroic's GOG downloader. Its zipapp unpacks a small native module
/// (xdelta3 patching, built for Python's stable ABI) into
/// `~/.cache/heroic_gogdl` on its first run.
pub const GOGDL: Helper = Helper {
    name: "gogdl",
    display_name: "gogdl",
    version: "1.4.0",
    assets: &[
        HelperAsset {
            arch: "x86_64",
            url: "https://github.com/Heroic-Games-Launcher/heroic-gogdl/releases/download/v1.4.0/gogdl_linux_x86_64",
            sha256: "2aa2b59c83d3bfbe7b60ead91c6297b263b5d1e6c233a04b7409bafe26738f02",
        },
        HelperAsset {
            arch: "aarch64",
            url: "https://github.com/Heroic-Games-Launcher/heroic-gogdl/releases/download/v1.4.0/gogdl_linux_arm64",
            sha256: "3df71cb07603b5d17fd914c96d17e4b381fad5a5705dc7dd3e3ff7a3856c6cc3",
        },
    ],
    source_url: "https://github.com/Heroic-Games-Launcher/heroic-gogdl",
    needs_python: true,
};

/// The Amazon Games client Heroic ships, by one of Heroic's maintainers.
/// Its builds are PyInstaller executables (about 11 MB), not zipapps.
pub const NILE: Helper = Helper {
    name: "nile",
    display_name: "Nile",
    version: "1.2.0",
    assets: &[
        HelperAsset {
            arch: "x86_64",
            url: "https://github.com/imLinguin/nile/releases/download/v1.2.0/nile_linux_x86_64",
            sha256: "d73519514df9d52c50c6feec629373cc3a0d9504e087b6c7372a8ca8a95e7bfe",
        },
        HelperAsset {
            arch: "aarch64",
            url: "https://github.com/imLinguin/nile/releases/download/v1.2.0/nile_linux_arm64",
            sha256: "197859c629c47e4e4d7ed1d2a290b7bef1e778f891ff8ca42ba65dbb020ba751",
        },
    ],
    source_url: "https://github.com/imLinguin/nile",
    needs_python: false,
};

/// More than any of the tools is (Legendary's zipapp is about 5 MB, Nile's
/// executable about 11 MB).
const MAX_HELPER_BYTES: usize = 64 * 1024 * 1024;

fn helpers_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve data directory: {e}"))?
        .join("helpers"))
}

/// Where a tool's pinned version lives: one file per version, so a newer
/// pin never runs an older download.
pub fn helper_path(dir: &Path, helper: &Helper) -> PathBuf {
    dir.join(format!("{}-{}", helper.name, helper.version))
}

/// The build of `helper` for `arch`, if there is one.
pub fn asset_for<'a>(helper: &'a Helper, arch: &str) -> Option<&'a HelperAsset> {
    helper.assets.iter().find(|asset| asset.arch == arch)
}

/// Whether `bytes` are what `asset` pins.
pub fn verify(asset: &HelperAsset, bytes: &[u8]) -> Result<(), String> {
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual.eq_ignore_ascii_case(asset.sha256) {
        Ok(())
    } else {
        Err(format!(
            "Checksum mismatch for {}: expected {}, got {actual} — the download is corrupted or incomplete",
            asset.url, asset.sha256
        ))
    }
}

/// The tool's path if its pinned version is downloaded.
pub fn installed(app: &AppHandle, helper: &Helper) -> Option<PathBuf> {
    let path = helper_path(&helpers_dir(app).ok()?, helper);
    path.is_file().then_some(path)
}

/// Every downloaded version of `helper` other than `keep`, so an update
/// leaves no older copy behind.
pub fn stale_versions(dir: &Path, helper: &Helper, keep: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let prefix = format!("{}-", helper.name);
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path != keep)
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(&prefix))
        })
        .collect()
}

/// Held while a tool downloads, so connecting a store twice at once (or a
/// launch needing the tool meanwhile) doesn't download it twice into the
/// same file.
static DOWNLOAD_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// The tool's path, downloading its pinned version first if needed.
pub async fn ensure(app: &AppHandle, helper: &Helper) -> Result<PathBuf, AppError> {
    // The zipapps run on the system's Python; without one they'd fail with
    // a shell error nobody could make sense of.
    if helper.needs_python && !command_on_path("python3") {
        return Err(AppError::PythonNotFound);
    }
    let dir = helpers_dir(app)?;
    let path = helper_path(&dir, helper);
    if path.is_file() {
        return Ok(path);
    }
    let _lock = DOWNLOAD_LOCK.lock().await;
    if path.is_file() {
        return Ok(path);
    }
    let asset = asset_for(helper, std::env::consts::ARCH).ok_or_else(|| {
        format!(
            "{} {} has no build for {}",
            helper.name,
            helper.version,
            std::env::consts::ARCH
        )
    })?;
    let bytes = download(asset.url).await?;
    verify(asset, &bytes)?;

    fs::create_dir_all(&dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    // Written next to its final name and renamed into place, so an
    // interrupted download never leaves a half file that looks installed.
    let partial = path.with_extension("download");
    let write = || -> std::io::Result<()> {
        let mut file = fs::File::create(&partial)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::set_permissions(&partial, fs::Permissions::from_mode(0o755))?;
        fs::rename(&partial, &path)
    };
    write().map_err(|e| {
        let _ = fs::remove_file(&partial);
        format!("Could not save {}: {e}", helper.name)
    })?;
    for old in stale_versions(&dir, helper, &path) {
        let _ = fs::remove_file(old);
    }
    Ok(path)
}

/// Deletes every downloaded version of the tool. Best-effort.
pub fn remove(app: &AppHandle, helper: &Helper) {
    if let Ok(dir) = helpers_dir(app) {
        for path in stale_versions(&dir, helper, Path::new("")) {
            let _ = fs::remove_file(path);
        }
    }
}

async fn download(url: &str) -> Result<Vec<u8>, String> {
    let response = crate::http::client()
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Could not download {url}: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Could not download {url} (status {})",
            response.status()
        ));
    }
    let too_large = || format!("{url} is larger than expected");
    if response
        .content_length()
        .is_some_and(|length| length > MAX_HELPER_BYTES as u64)
    {
        return Err(too_large());
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("Download of {url} interrupted: {e}"))?;
        if bytes.len() + chunk.len() > MAX_HELPER_BYTES {
            return Err(too_large());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests;
