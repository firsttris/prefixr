use crate::error::AppError;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use tauri::{AppHandle, Emitter, State};
use tokio::io::AsyncWriteExt;

use crate::commands::github::read_token;
use crate::config::ConfigState;
use crate::models::RunnerKind;

/// Adds a bearer `Authorization` header when a GitHub token is configured,
/// leaving the request unauthenticated otherwise — GitHub's API accepts both,
/// just at a much lower rate limit (60 vs 5000 requests/hour) when anonymous.
fn with_optional_auth(builder: RequestBuilder, token: Option<&str>) -> RequestBuilder {
    match token {
        Some(token) => builder.bearer_auth(token),
        None => builder,
    }
}

#[derive(Debug, Clone, Copy)]
enum ChecksumAlgorithm {
    Sha256,
    Sha512,
}

/// How a source publishes checksums for its release assets, so a download
/// can be verified against them before it's ever extracted.
struct ChecksumInfo {
    algorithm: ChecksumAlgorithm,
    /// Given the matched asset's filename, returns the filename of the
    /// release asset holding its checksum (some sources publish one file per
    /// asset, others a single shared file listing every asset in the
    /// release).
    asset_name: fn(&str) -> String,
    /// Extracts this asset's expected digest from the checksum file's
    /// contents (`None` if it isn't listed there).
    extract: fn(contents: &str, asset_name: &str) -> Option<String>,
}

fn strip_tar_gz(name: &str) -> String {
    format!("{}.sha512sum", name.strip_suffix(".tar.gz").unwrap_or(name))
}

fn strip_tar_xz(name: &str) -> String {
    format!("{}.sha512sum", name.strip_suffix(".tar.xz").unwrap_or(name))
}

fn shared_sha256sums_name(_asset_name: &str) -> String {
    "sha256sums.txt".to_string()
}

/// A checksum file holding just this one asset's digest, GNU-coreutils
/// format (`<hex digest>  <filename>`) — the digest is simply the first
/// whitespace-separated token.
fn extract_single_digest(contents: &str, _asset_name: &str) -> Option<String> {
    contents.split_whitespace().next().map(str::to_string)
}

/// A shared checksum file listing every asset in the release, GNU-coreutils
/// format, one `<hex digest>  <filename>` line per asset (a leading `*`
/// before the filename marks binary mode and is stripped before matching).
fn extract_shared_digest(contents: &str, asset_name: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let file = parts.next()?;
        (file.trim_start_matches('*') == asset_name).then(|| hash.to_string())
    })
}

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
    checksum: ChecksumInfo,
}

const RUNNER_SOURCES: &[RunnerSource] = &[
    RunnerSource {
        id: "proton-ge",
        label: "Proton-GE",
        repo: "GloriousEggroll/proton-ge-custom",
        kind: RunnerKind::Proton,
        matches_asset: |name| name.ends_with(".tar.gz") && !name.contains("aarch64"),
        checksum: ChecksumInfo {
            algorithm: ChecksumAlgorithm::Sha512,
            asset_name: strip_tar_gz,
            extract: extract_single_digest,
        },
    },
    RunnerSource {
        id: "wine-kron4ek",
        label: "Wine (Kron4ek)",
        repo: "Kron4ek/Wine-Builds",
        kind: RunnerKind::Wine,
        // Kron4ek ships several variants per release (vanilla/staging,
        // amd64/x86, with/without a bundled 32-bit wow64 build, plus a
        // "-tkg" flavor with extra patches). Staging + wow64 without tkg:
        // staging for the same gaming-oriented patches Proton itself
        // carries, wow64 so a single 64-bit wine binary handles 32-bit
        // games too without a separate wine32 install.
        matches_asset: |name| name.ends_with("-staging-amd64-wow64.tar.xz"),
        checksum: ChecksumInfo {
            algorithm: ChecksumAlgorithm::Sha256,
            asset_name: shared_sha256sums_name,
            extract: extract_shared_digest,
        },
    },
    RunnerSource {
        id: "proton-cachyos",
        label: "Proton-CachyOS",
        repo: "CachyOS/proton-cachyos",
        kind: RunnerKind::Proton,
        // Also ships an arm64 build and an x86_64_v3 (newer-CPU) variant
        // alongside the plain x86_64 one; only the latter is broadly safe.
        matches_asset: |name| name.ends_with("-x86_64.tar.xz"),
        checksum: ChecksumInfo {
            algorithm: ChecksumAlgorithm::Sha512,
            asset_name: strip_tar_xz,
            extract: extract_single_digest,
        },
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
    message: AppError,
}

#[derive(Clone, Serialize)]
struct RunnerDownloadDonePayload<'a> {
    tag: &'a str,
}

/// Lists recent releases from the given runner source's GitHub repo, one
/// entry per release that ships an asset matching that source's filter
/// (skips checksum-only, source-only, or wrong-architecture releases).
#[tauri::command]
pub async fn list_runner_releases(
    state: State<'_, ConfigState>,
    source: String,
) -> Result<Vec<RunnerRelease>, AppError> {
    let runner_source = find_source(&source)?;
    let token = read_token(&state)?;
    let url = format!(
        "https://api.github.com/repos/{}/releases?per_page=20",
        runner_source.repo
    );

    let response = with_optional_auth(
        crate::http::client().get(&url),
        token.as_deref(),
    )
    .send()
    .await
    .map_err(|e| format!("Could not reach GitHub: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        if status.as_u16() == 403 && token.is_none() {
            return Err(AppError::GitHubRateLimited);
        }
        return Err(AppError::GitHubApiError {
            status: status.to_string(),
        });
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

/// A fresh, hidden scratch directory inside `parent` to extract an archive
/// into (see `move_extracted_dir`). Inside `parent` rather than the system
/// temp dir, so the final move is a plain rename on the same filesystem;
/// hidden, and one level above the runner itself, so `scan_runners` never
/// mistakes it for one.
pub(crate) fn extraction_dir(parent: &Path) -> PathBuf {
    parent.join(format!(".extract-{}", uuid::Uuid::new_v4()))
}

/// Moves the one top-level directory an archive was extracted to inside
/// `extract_dir` to `target`, then removes `extract_dir` either way. Sources
/// don't all name that directory after the GitHub tag (e.g. Wine-GE names
/// it after the build), while the rest of the app (the "already exists"
/// guard, `isInstalled` in the UI) keys runners by tag. Extracting into a
/// scratch directory of its own, rather than straight into the shared
/// parent, keeps two extractions running at once from mistaking each
/// other's output for their own, and a failed one from leaving a
/// half-extracted runner behind.
pub(crate) fn move_extracted_dir(extract_dir: &Path, target: &Path) -> Result<(), String> {
    let result = (|| {
        let dirs: Vec<PathBuf> = fs::read_dir(extract_dir)
            .map_err(|e| format!("Could not read {}: {e}", extract_dir.display()))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        match dirs.as_slice() {
            [only] => fs::rename(only, target)
                .map_err(|e| format!("Could not move extracted directory: {e}")),
            [] => Err("Archive did not contain a directory".to_string()),
            _ => Err("Archive contained more than one top-level directory".to_string()),
        }
    })();
    let _ = fs::remove_dir_all(extract_dir);
    result
}

/// Hashes a file on disk with the given algorithm, returning its digest as a
/// lowercase hex string. Run inside `spawn_blocking` by callers, since
/// hashing a multi-hundred-MB archive is real CPU work.
fn compute_digest(path: &Path, algorithm: ChecksumAlgorithm) -> Result<String, String> {
    let mut file =
        fs::File::open(path).map_err(|e| format!("Could not open archive for checksum: {e}"))?;
    let mut buf = [0u8; 64 * 1024];

    macro_rules! hash_with {
        ($hasher:ty) => {{
            let mut hasher = <$hasher>::new();
            loop {
                let n = file
                    .read(&mut buf)
                    .map_err(|e| format!("Could not read archive for checksum: {e}"))?;
                if n == 0 {
                    break;
                }
                hasher.update(&buf[..n]);
            }
            format!("{:x}", hasher.finalize())
        }};
    }

    Ok(match algorithm {
        ChecksumAlgorithm::Sha256 => hash_with!(Sha256),
        ChecksumAlgorithm::Sha512 => hash_with!(Sha512),
    })
}

/// Fetches the checksum file this source publishes for `asset_name` and
/// verifies the just-downloaded `archive_path` against it, before it's ever
/// extracted. A source that doesn't publish checksums (none currently, but
/// `ChecksumInfo` is per-source so this stays possible) would need an
/// `Option` here instead — as it stands, every listed source is checked.
async fn verify_checksum(
    source: &RunnerSource,
    asset_name: &str,
    download_url: &str,
    archive_path: &Path,
    token: Option<&str>,
) -> Result<(), String> {
    let checksum_asset_name = (source.checksum.asset_name)(asset_name);
    let checksum_url = download_url
        .strip_suffix(asset_name)
        .map(|prefix| format!("{prefix}{checksum_asset_name}"))
        .ok_or_else(|| "Could not derive checksum file URL".to_string())?;

    let response = with_optional_auth(
        crate::http::client().get(&checksum_url),
        token,
    )
    .send()
    .await
    .map_err(|e| format!("Could not fetch checksum file: {e}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "Could not fetch checksum file (status {})",
            response.status()
        ));
    }
    let contents = response
        .text()
        .await
        .map_err(|e| format!("Could not read checksum file: {e}"))?;

    let expected = (source.checksum.extract)(&contents, asset_name).ok_or_else(|| {
        format!("Checksum file {checksum_asset_name} didn't list an entry for {asset_name}")
    })?;

    let algorithm = source.checksum.algorithm;
    let archive_path = archive_path.to_path_buf();
    let actual = tokio::task::spawn_blocking(move || compute_digest(&archive_path, algorithm))
        .await
        .map_err(|e| format!("Checksum task panicked: {e}"))??;

    if actual.eq_ignore_ascii_case(&expected) {
        Ok(())
    } else {
        Err(format!(
            "Checksum mismatch for {asset_name}: expected {expected}, got {actual} — \
             the download may be corrupted or tampered with"
        ))
    }
}

/// Checks the `tag` and `download_url` the frontend passes back from
/// `list_runner_releases`: the tag becomes a directory name inside the
/// runners directory, and the URL gets the GitHub token, so neither is
/// taken on trust — only a plain name, and only a release asset of this
/// source's own repo.
fn check_download_request(source: &RunnerSource, tag: &str, download_url: &str) -> Result<(), String> {
    if tag.is_empty() || tag.starts_with('.') || tag.contains(['/', '\\', '\0']) {
        return Err(format!("Invalid runner tag '{tag}'"));
    }
    let expected = format!("https://github.com/{}/releases/download/", source.repo);
    let matches = download_url
        .get(..expected.len())
        .is_some_and(|start| start.eq_ignore_ascii_case(&expected));
    if !matches {
        return Err(format!("Not a {} release download: {download_url}", source.label));
    }
    Ok(())
}

/// How often `runner-download-progress` goes out at most: once per chunk
/// would be tens of thousands of events for a single runner.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

/// Downloads a runner release into the runners directory and extracts it.
/// Progress (bytes downloaded so far, and the total if known) is streamed via
/// `runner-download-progress` events; the final outcome is reported both as
/// the command's `Result` and as `runner-download-done` / `runner-download-error`.
#[tauri::command]
pub async fn download_runner(
    app: AppHandle,
    state: State<'_, ConfigState>,
    source: String,
    tag: String,
    download_url: String,
) -> Result<(), AppError> {
    let runner_source = find_source(&source)?;
    check_download_request(runner_source, &tag, &download_url)?;
    let asset_name = download_url
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| format!("Download URL has no file name: {download_url}"))?
        .to_string();

    let runners_dir = {
        let config = state
            .lock()
            .map_err(|_| "Configuration is locked".to_string())?;
        config.runners_dir.clone()
    };
    let token = read_token(&state)?;

    fs::create_dir_all(&runners_dir)
        .map_err(|e| format!("Could not create runners directory: {e}"))?;

    let target_dir = runners_dir.join(&tag);
    if target_dir.exists() {
        return Err(AppError::RunnerAlreadyExists { tag });
    }

    let is_xz = download_url.ends_with(".tar.xz");

    let response = with_optional_auth(
        crate::http::client().get(&download_url),
        token.as_deref(),
    )
    .send()
    .await
    .map_err(|e| format!("Could not start download: {e}"))?;

    if !response.status().is_success() {
        let message = format!("Download failed with status {}", response.status());
        let _ = app.emit(
            "runner-download-error",
            RunnerDownloadErrorPayload {
                tag: &tag,
                message: message.clone().into(),
            },
        );
        return Err(message.into());
    }

    let total = response.content_length();
    let archive_suffix = if is_xz { "tar.xz" } else { "tar.gz" };
    // Next to the runners rather than in /tmp, which is often a RAM-backed
    // tmpfs too small for a runner of several hundred MB; hidden and not a
    // directory, so `scan_runners` never lists it.
    let archive_path = runners_dir.join(format!(
        ".download-{}.{archive_suffix}",
        uuid::Uuid::new_v4()
    ));

    let download_result: Result<(), String> = async {
        let mut file = tokio::fs::File::create(&archive_path)
            .await
            .map_err(|e| format!("Could not create temp file: {e}"))?;

        let mut downloaded: u64 = 0;
        let mut last_progress: Option<Instant> = None;
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Download interrupted: {e}"))?;
            downloaded += chunk.len() as u64;
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Could not write temp file: {e}"))?;
            let finished = total == Some(downloaded);
            if finished || last_progress.is_none_or(|at| at.elapsed() >= PROGRESS_INTERVAL) {
                last_progress = Some(Instant::now());
                let _ = app.emit(
                    "runner-download-progress",
                    RunnerDownloadProgressPayload {
                        tag: &tag,
                        downloaded,
                        total,
                    },
                );
            }
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
                message: message.clone().into(),
            },
        );
        return Err(message.into());
    }

    if let Err(message) = verify_checksum(
        runner_source,
        &asset_name,
        &download_url,
        &archive_path,
        token.as_deref(),
    )
    .await
    {
        let _ = tokio::fs::remove_file(&archive_path).await;
        let _ = app.emit(
            "runner-download-error",
            RunnerDownloadErrorPayload {
                tag: &tag,
                message: message.clone().into(),
            },
        );
        return Err(message.into());
    }

    let extract_dir = extraction_dir(&runners_dir);
    let extract_target = extract_dir.clone();
    let archive_path_for_extraction = archive_path.clone();
    let extraction = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let archive_file = fs::File::open(&archive_path_for_extraction)
            .map_err(|e| format!("Could not open downloaded archive: {e}"))?;
        if is_xz {
            let decompressed = xz2::read::XzDecoder::new(archive_file);
            tar::Archive::new(decompressed)
                .unpack(&extract_target)
                .map_err(|e| format!("Could not extract archive: {e}"))
        } else {
            let decompressed = flate2::read::GzDecoder::new(archive_file);
            tar::Archive::new(decompressed)
                .unpack(&extract_target)
                .map_err(|e| format!("Could not extract archive: {e}"))
        }
    })
    .await
    .map_err(|e| format!("Extraction task panicked: {e}"))?;

    let _ = tokio::fs::remove_file(&archive_path).await;

    if let Err(message) = extraction {
        let _ = fs::remove_dir_all(&extract_dir);
        let _ = app.emit(
            "runner-download-error",
            RunnerDownloadErrorPayload {
                tag: &tag,
                message: message.clone().into(),
            },
        );
        return Err(message.into());
    }

    if let Err(message) = move_extracted_dir(&extract_dir, &target_dir) {
        let _ = app.emit(
            "runner-download-error",
            RunnerDownloadErrorPayload {
                tag: &tag,
                message: message.clone().into(),
            },
        );
        return Err(message.into());
    }

    let _ = app.emit(
        "runner-download-done",
        RunnerDownloadDonePayload { tag: &tag },
    );
    Ok(())
}

#[cfg(test)]
mod tests;
