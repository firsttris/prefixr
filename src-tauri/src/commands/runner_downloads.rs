use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use reqwest::header::USER_AGENT;
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
pub async fn list_runner_releases(
    state: State<'_, ConfigState>,
    source: String,
) -> Result<Vec<RunnerRelease>, String> {
    let runner_source = find_source(&source)?;
    let token = read_token(&state)?;
    let url = format!(
        "https://api.github.com/repos/{}/releases?per_page=20",
        runner_source.repo
    );

    let response = with_optional_auth(
        reqwest::Client::new().get(&url).header(USER_AGENT, "prefixr"),
        token.as_deref(),
    )
    .send()
    .await
    .map_err(|e| format!("Could not reach GitHub: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let hint = if status.as_u16() == 403 && token.is_none() {
            " (GitHub's rate limit for unauthenticated requests is likely exhausted — \
              add a GitHub token in the settings to raise it)"
        } else {
            ""
        };
        return Err(format!("GitHub API returned status {status}{hint}"));
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
pub(crate) fn snapshot_entries(runners_dir: &Path) -> HashSet<String> {
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
pub(crate) fn normalize_extracted_dir(
    runners_dir: &Path,
    before: &HashSet<String>,
    tag: &str,
) -> Result<(), String> {
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
        reqwest::Client::new()
            .get(&checksum_url)
            .header(USER_AGENT, "prefixr"),
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
) -> Result<(), String> {
    let runner_source = find_source(&source)?;
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
        return Err(format!("Runner '{tag}' already exists"));
    }

    let is_xz = download_url.ends_with(".tar.xz");

    let response = with_optional_auth(
        reqwest::Client::new()
            .get(&download_url)
            .header(USER_AGENT, "prefixr"),
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
    fn checksum_asset_names_match_real_releases() {
        // Real asset-name pairs, sampled from each source's GitHub releases.
        assert_eq!(strip_tar_gz("GE-Proton11-7-x86_64.tar.gz"), "GE-Proton11-7-x86_64.sha512sum");
        assert_eq!(
            strip_tar_xz("proton-cachyos-11.0-20260703-slr-x86_64.tar.xz"),
            "proton-cachyos-11.0-20260703-slr-x86_64.sha512sum"
        );
        assert_eq!(shared_sha256sums_name("wine-11.18-staging-amd64-wow64.tar.xz"), "sha256sums.txt");
    }

    #[test]
    fn extract_single_digest_takes_the_first_token() {
        let contents = "7db87e9787e20c35cbdac26018431d5794626b626e4067b050684e45a88cc2ca229d7d263519eafb2e168cde5bef57611065d159d3685aaec152ccb9abe3073f  GE-Proton11-7-x86_64.tar.gz\n";
        assert_eq!(
            extract_single_digest(contents, "GE-Proton11-7-x86_64.tar.gz"),
            Some("7db87e9787e20c35cbdac26018431d5794626b626e4067b050684e45a88cc2ca229d7d263519eafb2e168cde5bef57611065d159d3685aaec152ccb9abe3073f".to_string())
        );
    }

    #[test]
    fn extract_shared_digest_finds_the_matching_line() {
        let contents = "\
5717663b0541afe99efee507e1eac88d452b8adff0d3a5065abeed527890a3e8  wine-11.18-amd64.tar.xz
f899879b8c37e0b20adca19d147cf77436f3f1a37bf16d08d27fa7137a52b9ba  wine-11.18-amd64-wow64.tar.xz
";
        assert_eq!(
            extract_shared_digest(contents, "wine-11.18-amd64-wow64.tar.xz"),
            Some("f899879b8c37e0b20adca19d147cf77436f3f1a37bf16d08d27fa7137a52b9ba".to_string())
        );
        assert_eq!(extract_shared_digest(contents, "wine-11.18-nonexistent.tar.xz"), None);
    }

    #[test]
    fn extract_shared_digest_strips_binary_mode_marker() {
        let contents = "5717663b0541afe99efee507e1eac88d452b8adff0d3a5065abeed527890a3e8 *wine-11.18-amd64.tar.xz\n";
        assert_eq!(
            extract_shared_digest(contents, "wine-11.18-amd64.tar.xz"),
            Some("5717663b0541afe99efee507e1eac88d452b8adff0d3a5065abeed527890a3e8".to_string())
        );
    }

    #[test]
    fn asset_matchers_pick_the_right_asset() {
        // Real release asset names, sampled from each source's GitHub API.
        assert!((find_source("proton-ge").unwrap().matches_asset)("GE-Proton9-20.tar.gz"));
        assert!(!(find_source("proton-ge").unwrap().matches_asset)("GE-Proton9-20.sha512sum"));

        assert!((find_source("wine-kron4ek").unwrap().matches_asset)(
            "wine-11.18-staging-amd64-wow64.tar.xz"
        ));
        assert!(!(find_source("wine-kron4ek").unwrap().matches_asset)(
            "wine-11.18-amd64-wow64.tar.xz"
        ));
        assert!(!(find_source("wine-kron4ek").unwrap().matches_asset)(
            "wine-11.18-staging-amd64.tar.xz"
        ));
        assert!(!(find_source("wine-kron4ek").unwrap().matches_asset)(
            "wine-11.18-staging-tkg-amd64-wow64.tar.xz"
        ));
        assert!(!(find_source("wine-kron4ek").unwrap().matches_asset)(
            "wine-11.18-staging-x86.tar.xz"
        ));
        assert!(!(find_source("wine-kron4ek").unwrap().matches_asset)("sha256sums.txt"));

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

        // Simulate an archive whose top-level folder name doesn't match the
        // GitHub tag (some sources name it after the build, not the tag).
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
