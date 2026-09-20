use std::fs;

use futures_util::StreamExt;
use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tokio::io::AsyncWriteExt;

use crate::config::ConfigState;

const PROTON_GE_RELEASES_URL: &str =
    "https://api.github.com/repos/GloriousEggroll/proton-ge-custom/releases?per_page=20";

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    name: Option<String>,
    published_at: String,
    assets: Vec<GitHubAsset>,
}

/// A downloadable Proton-GE build, as surfaced to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct ProtonGeRelease {
    pub tag: String,
    pub name: String,
    pub published_at: String,
    pub download_url: String,
    pub size: u64,
}

#[derive(Clone, Serialize)]
struct RunnerDownloadProgressPayload<'a> {
    tag: &'a str,
    downloaded: u64,
    total: Option<u64>,
}

#[derive(Clone, Serialize)]
struct RunnerDownloadErrorPayload<'a> {
    tag: &'a str,
    message: String,
}

#[derive(Clone, Serialize)]
struct RunnerDownloadDonePayload<'a> {
    tag: &'a str,
}

/// Lists recent Proton-GE releases from GitHub, one entry per release that
/// ships a `.tar.gz` asset (skips checksum-only or source-only releases).
#[tauri::command]
pub async fn list_proton_ge_releases() -> Result<Vec<ProtonGeRelease>, String> {
    let response = reqwest::Client::new()
        .get(PROTON_GE_RELEASES_URL)
        .header(USER_AGENT, "prefixr")
        .send()
        .await
        .map_err(|e| format!("Could not reach GitHub: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("GitHub API returned status {}", response.status()));
    }

    let releases: Vec<GitHubRelease> = response
        .json()
        .await
        .map_err(|e| format!("Could not parse GitHub response: {e}"))?;

    let result = releases
        .into_iter()
        .filter_map(|release| {
            // Recent releases ship both aarch64 and x86_64 tarballs; older ones
            // ship a single unsuffixed one (implicitly x86_64). Exclude aarch64
            // explicitly so we never grab the wrong architecture on a normal PC.
            let asset = release
                .assets
                .into_iter()
                .find(|asset| asset.name.ends_with(".tar.gz") && !asset.name.contains("aarch64"))?;
            Some(ProtonGeRelease {
                tag: release.tag_name.clone(),
                name: release.name.unwrap_or(release.tag_name),
                published_at: release.published_at,
                download_url: asset.browser_download_url,
                size: asset.size,
            })
        })
        .collect();

    Ok(result)
}

/// Downloads a Proton-GE release into the runners directory and extracts it.
/// Progress (bytes downloaded so far, and the total if known) is streamed via
/// `runner-download-progress` events; the final outcome is reported both as
/// the command's `Result` and as `runner-download-done` / `runner-download-error`.
#[tauri::command]
pub async fn download_runner(
    app: AppHandle,
    state: State<'_, ConfigState>,
    tag: String,
    download_url: String,
) -> Result<(), String> {
    let runners_dir = {
        let config = state
            .lock()
            .map_err(|_| "Configuration is locked".to_string())?;
        config.runners_dir.clone()
    };

    fs::create_dir_all(&runners_dir)
        .map_err(|e| format!("Could not create runners directory: {e}"))?;

    let target_dir = runners_dir.join(&tag);
    if target_dir.exists() {
        return Err(format!("Runner '{tag}' already exists"));
    }

    let response = reqwest::Client::new()
        .get(&download_url)
        .header(USER_AGENT, "prefixr")
        .send()
        .await
        .map_err(|e| format!("Could not start download: {e}"))?;

    if !response.status().is_success() {
        let message = format!("Download failed with status {}", response.status());
        let _ = app.emit(
            "runner-download-error",
            RunnerDownloadErrorPayload {
                tag: &tag,
                message: message.clone(),
            },
        );
        return Err(message);
    }

    let total = response.content_length();
    let archive_path = std::env::temp_dir().join(format!("prefixr-{tag}.tar.gz"));

    let download_result: Result<(), String> = async {
        let mut file = tokio::fs::File::create(&archive_path)
            .await
            .map_err(|e| format!("Could not create temp file: {e}"))?;

        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Download interrupted: {e}"))?;
            downloaded += chunk.len() as u64;
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Could not write temp file: {e}"))?;
            let _ = app.emit(
                "runner-download-progress",
                RunnerDownloadProgressPayload {
                    tag: &tag,
                    downloaded,
                    total,
                },
            );
        }
        file.flush()
            .await
            .map_err(|e| format!("Could not flush temp file: {e}"))
    }
    .await;

    if let Err(message) = download_result {
        let _ = tokio::fs::remove_file(&archive_path).await;
        let _ = app.emit(
            "runner-download-error",
            RunnerDownloadErrorPayload {
                tag: &tag,
                message: message.clone(),
            },
        );
        return Err(message);
    }

    let extract_dir = runners_dir.clone();
    let archive_path_for_extraction = archive_path.clone();
    let extraction = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let tar_gz = fs::File::open(&archive_path_for_extraction)
            .map_err(|e| format!("Could not open downloaded archive: {e}"))?;
        let decompressed = flate2::read::GzDecoder::new(tar_gz);
        let mut archive = tar::Archive::new(decompressed);
        archive
            .unpack(&extract_dir)
            .map_err(|e| format!("Could not extract archive: {e}"))
    })
    .await
    .map_err(|e| format!("Extraction task panicked: {e}"))?;

    let _ = tokio::fs::remove_file(&archive_path).await;

    if let Err(message) = extraction {
        let _ = app.emit(
            "runner-download-error",
            RunnerDownloadErrorPayload {
                tag: &tag,
                message: message.clone(),
            },
        );
        return Err(message);
    }

    let _ = app.emit(
        "runner-download-done",
        RunnerDownloadDonePayload { tag: &tag },
    );
    Ok(())
}
