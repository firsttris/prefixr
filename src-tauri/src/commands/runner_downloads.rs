use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};
use tokio::io::AsyncWriteExt;

use crate::config::ConfigState;
use crate::models::RunnerKind;

/// A GitHub-releases-backed source of downloadable runner builds.
struct RunnerSource {
    id: &'static str,
    label: &'static str,
    repo: &'static str,
    kind: RunnerKind,
    /// Picks the one asset in a release that's the runner tarball for a
    /// normal x86_64 PC (as opposed to checksums, arm64 builds, or
    /// alternate-microarchitecture variants some sources also publish).
    matches_asset: fn(&str) -> bool,
}

const RUNNER_SOURCES: &[RunnerSource] = &[
    RunnerSource {
        id: "proton-ge",
        label: "Proton-GE",
        repo: "GloriousEggroll/proton-ge-custom",
        kind: RunnerKind::Proton,
        matches_asset: |name| name.ends_with(".tar.gz") && !name.contains("aarch64"),
    },
    RunnerSource {
        id: "wine-ge",
        label: "Wine-GE",
        repo: "GloriousEggroll/wine-ge-custom",
        kind: RunnerKind::Wine,
        matches_asset: |name| name.ends_with(".tar.xz") && !name.contains("aarch64"),
    },
    RunnerSource {
        id: "umu-proton",
        label: "UMU-Proton",
        repo: "Open-Wine-Components/umu-proton",
        kind: RunnerKind::Proton,
        matches_asset: |name| name.ends_with(".tar.gz"),
    },
    RunnerSource {
        id: "proton-cachyos",
        label: "Proton-CachyOS",
        repo: "CachyOS/proton-cachyos",
        kind: RunnerKind::Proton,
        // Also ships an arm64 build and an x86_64_v3 (newer-CPU) variant
        // alongside the plain x86_64 one; only the latter is broadly safe.
        matches_asset: |name| name.ends_with("-x86_64.tar.xz"),
    },
];

fn find_source(id: &str) -> Result<&'static RunnerSource, String> {
    RUNNER_SOURCES
        .iter()
        .find(|s| s.id == id)
        .ok_or_else(|| format!("Unknown runner source '{id}'"))
}

/// A runner source as surfaced to the frontend, to drive the source picker
/// without duplicating labels/kinds on the TypeScript side.
#[derive(Debug, Clone, Serialize)]
pub struct RunnerSourceInfo {
    pub id: &'static str,
    pub label: &'static str,
    pub kind: RunnerKind,
}

#[tauri::command]
pub fn list_runner_sources() -> Vec<RunnerSourceInfo> {
    RUNNER_SOURCES
        .iter()
        .map(|s| RunnerSourceInfo {
            id: s.id,
            label: s.label,
            kind: s.kind,
        })
        .collect()
}

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

/// A downloadable runner build, as surfaced to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct RunnerRelease {
    pub source: String,
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

/// Lists recent releases from the given runner source's GitHub repo, one
/// entry per release that ships an asset matching that source's filter
/// (skips checksum-only, source-only, or wrong-architecture releases).
#[tauri::command]
pub async fn list_runner_releases(source: String) -> Result<Vec<RunnerRelease>, String> {
    let runner_source = find_source(&source)?;
    let url = format!(
        "https://api.github.com/repos/{}/releases?per_page=20",
        runner_source.repo
    );

    let response = reqwest::Client::new()
        .get(&url)
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
            let asset = release
                .assets
                .into_iter()
                .find(|asset| (runner_source.matches_asset)(&asset.name))?;
            Some(RunnerRelease {
                source: runner_source.id.to_string(),
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

/// Names of `runners_dir`'s current top-level entries, used to spot exactly
/// which directory an extraction just created.
fn snapshot_entries(runners_dir: &Path) -> HashSet<String> {
    fs::read_dir(runners_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// Renames the directory an extraction just created to `tag`, since sources
/// don't all name their archive's top-level folder after the GitHub tag
/// (e.g. Wine-GE names it after the build, not the tag) while the rest of
/// the app (the "already exists" guard, `isInstalled` in the UI) keys
/// runners by tag.
fn normalize_extracted_dir(runners_dir: &Path, before: &HashSet<String>, tag: &str) -> Result<(), String> {
    let target_dir = runners_dir.join(tag);
    if target_dir.exists() {
        return Ok(());
    }

    let mut new_dirs: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(runners_dir)
        .map_err(|e| format!("Could not read runners directory: {e}"))?
    {
        let entry = entry.map_err(|e| format!("Could not read directory entry: {e}"))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !before.contains(&name) && entry.path().is_dir() {
            new_dirs.push(entry.path());
        }
    }

    match new_dirs.as_slice() {
        [only] => fs::rename(only, &target_dir)
            .map_err(|e| format!("Could not rename extracted runner directory: {e}")),
        [] => Err(format!("Archive for '{tag}' did not create a runner directory")),
        _ => Err(format!(
            "Archive for '{tag}' created more than one new directory; expected exactly one"
        )),
    }
}

/// Downloads a runner release into the runners directory and extracts it.
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

    let is_xz = download_url.ends_with(".tar.xz");

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
    let archive_suffix = if is_xz { "tar.xz" } else { "tar.gz" };
    let archive_path = std::env::temp_dir().join(format!("prefixr-{tag}.{archive_suffix}"));

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

    let before = snapshot_entries(&runners_dir);

    let extract_dir = runners_dir.clone();
    let archive_path_for_extraction = archive_path.clone();
    let extraction = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let archive_file = fs::File::open(&archive_path_for_extraction)
            .map_err(|e| format!("Could not open downloaded archive: {e}"))?;
        if is_xz {
            let decompressed = xz2::read::XzDecoder::new(archive_file);
            tar::Archive::new(decompressed)
                .unpack(&extract_dir)
                .map_err(|e| format!("Could not extract archive: {e}"))
        } else {
            let decompressed = flate2::read::GzDecoder::new(archive_file);
            tar::Archive::new(decompressed)
                .unpack(&extract_dir)
                .map_err(|e| format!("Could not extract archive: {e}"))
        }
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

    if let Err(message) = normalize_extracted_dir(&runners_dir, &before, &tag) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_matchers_pick_the_right_asset() {
        // Real release asset names, sampled from each source's GitHub API.
        assert!((find_source("proton-ge").unwrap().matches_asset)("GE-Proton9-20.tar.gz"));
        assert!(!(find_source("proton-ge").unwrap().matches_asset)("GE-Proton9-20.sha512sum"));

        assert!((find_source("wine-ge").unwrap().matches_asset)(
            "wine-lutris-GE-Proton8-26-x86_64.tar.xz"
        ));
        assert!(!(find_source("wine-ge").unwrap().matches_asset)(
            "wine-lutris-GE-Proton8-26-x86_64.sha512sum"
        ));

        assert!((find_source("umu-proton").unwrap().matches_asset)("UMU-Proton-10.0-4.tar.gz"));

        assert!((find_source("proton-cachyos").unwrap().matches_asset)(
            "proton-cachyos-11.0-20260703-slr-x86_64.tar.xz"
        ));
        assert!(!(find_source("proton-cachyos").unwrap().matches_asset)(
            "proton-cachyos-11.0-20260703-slr-x86_64_v3.tar.xz"
        ));
        assert!(!(find_source("proton-cachyos").unwrap().matches_asset)(
            "proton-cachyos-11.0-20260703-slr-arm64.tar.xz"
        ));
    }

    #[test]
    fn normalize_renames_mismatched_extracted_dir() {
        let dir = std::env::temp_dir().join(format!("prefixr-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let before = snapshot_entries(&dir);

        // Simulate an archive whose top-level folder name (Wine-GE style)
        // doesn't match the GitHub tag.
        fs::create_dir(dir.join("wine-lutris-GE-Proton8-26-x86_64")).unwrap();

        normalize_extracted_dir(&dir, &before, "GE-Proton8-26").unwrap();

        assert!(dir.join("GE-Proton8-26").is_dir());
        assert!(!dir.join("wine-lutris-GE-Proton8-26-x86_64").exists());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn normalize_is_noop_when_dir_already_matches_tag() {
        let dir = std::env::temp_dir().join(format!("prefixr-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let before = snapshot_entries(&dir);

        fs::create_dir(dir.join("GE-Proton9-20")).unwrap();

        normalize_extracted_dir(&dir, &before, "GE-Proton9-20").unwrap();
        assert!(dir.join("GE-Proton9-20").is_dir());

        fs::remove_dir_all(&dir).unwrap();
    }
}
