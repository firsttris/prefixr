use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Serialize;

/// An `.exe` shortcut target found among the `.lnk` files an installer
/// created inside a prefix's Desktop/Start Menu, used to guess which exe is
/// the actual game once its installer has finished (see `run_installer` in
/// `commands::games`).
#[derive(Debug, Clone, Serialize)]
pub struct DetectedShortcut {
    /// The shortcut's own file name, without the `.lnk` extension — e.g.
    /// "Baldur's Gate 3".
    pub name: String,
    pub exe_path: String,
}

/// Shortcut filename fragments (checked lowercased) that mark a `.lnk` as
/// *not* the game itself — uninstallers, redistributable installers,
/// readmes and other noise most installers drop alongside the real
/// shortcut.
const NOISE_PATTERNS: &[&str] = &[
    "uninstall",
    "unins0",
    "readme",
    "changelog",
    "license",
    "eula",
    "register",
    "activation",
    "support",
    "website",
    "help",
    "redist",
    "vcredist",
    "directx",
    "dxsetup",
    "dotnet",
];

/// Directories, relative to a prefix's `drive_c`, that installers commonly
/// drop shortcuts into — both wine's classic (XP-style) layout and the
/// Vista+ layout some runners configure instead, for both the (steered, see
/// `steer_profile_to_steamuser`) `steamuser` profile and `Public`.
const SHORTCUT_DIRS: &[&str] = &[
    "users/steamuser/Desktop",
    "users/steamuser/Start Menu/Programs",
    "users/steamuser/AppData/Roaming/Microsoft/Windows/Start Menu/Programs",
    "users/Public/Desktop",
    "users/Public/Start Menu/Programs",
    "ProgramData/Microsoft/Windows/Start Menu/Programs",
];

/// Scans `prefix_path`'s known shortcut locations for `.lnk` files modified
/// at or after `since`, resolves each one's target, and returns those
/// pointing at an `.exe` that isn't obvious installer noise (see
/// `NOISE_PATTERNS`). Installers overwhelmingly create exactly one such
/// shortcut for their main executable but not for redistributables or
/// uninstallers, which is the same signal tools like PortProton effectively
/// rely on instead of a generic "which exe is the game" heuristic.
pub fn find_recently_created_shortcuts(
    prefix_path: &Path,
    since: SystemTime,
) -> Vec<DetectedShortcut> {
    let drive_c = prefix_path.join("drive_c");
    let mut lnk_files = Vec::new();
    for dir in SHORTCUT_DIRS {
        collect_lnk_files(&drive_c.join(dir), 4, &mut lnk_files);
    }

    let mut seen_targets = HashSet::new();
    let mut results = Vec::new();
    for lnk_path in lnk_files {
        let Ok(metadata) = fs::metadata(&lnk_path) else {
            continue;
        };
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        if modified < since {
            continue;
        }

        let Some(stem) = lnk_path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let lower = stem.to_lowercase();
        if NOISE_PATTERNS.iter().any(|pattern| lower.contains(pattern)) {
            continue;
        }

        let Some(target) = read_shell_link_target(&lnk_path) else {
            continue;
        };
        if !target.to_lowercase().ends_with(".exe") {
            continue;
        }
        let Some(resolved) = resolve_windows_path(prefix_path, &target) else {
            continue;
        };
        if !resolved.is_file() {
            continue;
        }
        let resolved_str = resolved.display().to_string();
        if !seen_targets.insert(resolved_str.clone()) {
            continue;
        }

        results.push(DetectedShortcut {
            name: stem.to_string(),
            exe_path: resolved_str,
        });
    }

    results
}

fn collect_lnk_files(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    if depth == 0 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            collect_lnk_files(&path, depth - 1, out);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("lnk"))
        {
            out.push(path);
        }
    }
}

/// Resolves a shortcut's Windows-style target path (e.g.
/// `C:\Games\Foo\foo.exe`) to a real filesystem path inside `prefix_path`,
/// via that drive letter's `dosdevices` symlink — rather than assuming `C:`
/// always maps straight to `drive_c`, since a prefix can remap drives.
fn resolve_windows_path(prefix_path: &Path, windows_path: &str) -> Option<PathBuf> {
    let mut chars = windows_path.chars();
    let drive = chars.next()?.to_ascii_lowercase();
    if chars.next() != Some(':') {
        return None;
    }
    // What's left after `X:` — via the iterator, since a drive "letter"
    // from a malformed shortcut needn't be one byte long.
    let rest = chars.as_str().trim_start_matches(['\\', '/']);

    let drive_link = prefix_path.join("dosdevices").join(format!("{drive}:"));
    let mut resolved = fs::canonicalize(&drive_link).ok()?;
    for part in rest.split(['\\', '/']) {
        if !part.is_empty() {
            resolved.push(part);
        }
    }
    Some(resolved)
}

// --- Minimal Windows Shell Link (.lnk) parser (MS-SHLLINK) -----------------
//
// Only extracts the `LinkInfo` structure's local base path, which is what a
// shortcut to a local file (as opposed to a network share) carries — that
// covers every installer-created shortcut in practice. `LinkTargetIDList`
// (the shell-namespace form) is skipped: parsing it needs the much larger
// SHITEMID machinery for no benefit here, since installers also always
// write the simpler `LinkInfo` form alongside it for compatibility with
// tools that don't resolve shell namespace IDs.

const HAS_LINK_TARGET_ID_LIST: u32 = 1 << 0;
const HAS_LINK_INFO: u32 = 1 << 1;

/// Reads a `.lnk` file's target path, in Windows form (e.g.
/// `C:\Games\Foo\foo.exe`), or `None` if it doesn't carry a local `LinkInfo`
/// target (a network-share shortcut, or a malformed/foreign file).
fn read_shell_link_target(path: &Path) -> Option<String> {
    let data = fs::read(path).ok()?;
    if data.len() < 76 || u32::from_le_bytes(data[0..4].try_into().ok()?) != 0x0000_004C {
        return None;
    }

    let flags = u32::from_le_bytes(data[20..24].try_into().ok()?);
    let mut offset = 76usize;

    if flags & HAS_LINK_TARGET_ID_LIST != 0 {
        let id_list_size =
            u16::from_le_bytes(data.get(offset..offset + 2)?.try_into().ok()?) as usize;
        offset += 2 + id_list_size;
    }

    if flags & HAS_LINK_INFO == 0 {
        return None;
    }

    parse_link_info(&data, offset)
}

/// Parses the `LinkInfo` structure starting at `offset` in the file, and
/// returns its local target path (`LocalBasePath` + `CommonPathSuffix`, per
/// MS-SHLLINK 2.3), or `None` for a network-share target.
fn parse_link_info(data: &[u8], offset: usize) -> Option<String> {
    let raw = data.get(offset..)?;
    let link_info_size = u32::from_le_bytes(raw.get(0..4)?.try_into().ok()?) as usize;
    let info = raw.get(0..link_info_size)?;

    let header_size = u32::from_le_bytes(info.get(4..8)?.try_into().ok()?) as usize;
    let link_info_flags = u32::from_le_bytes(info.get(8..12)?.try_into().ok()?);
    if link_info_flags & 1 == 0 {
        // Network-share target (VolumeIDAndLocalBasePath not set).
        return None;
    }

    let local_base_path_offset = u32::from_le_bytes(info.get(16..20)?.try_into().ok()?) as usize;
    let common_suffix_offset = u32::from_le_bytes(info.get(24..28)?.try_into().ok()?) as usize;

    if header_size >= 0x24 {
        let base_unicode_offset = u32::from_le_bytes(info.get(28..32)?.try_into().ok()?) as usize;
        if base_unicode_offset != 0 {
            let suffix_unicode_offset =
                u32::from_le_bytes(info.get(32..36)?.try_into().ok()?) as usize;
            let base = read_utf16_cstr(info, base_unicode_offset)?;
            let suffix = if suffix_unicode_offset != 0 {
                read_utf16_cstr(info, suffix_unicode_offset).unwrap_or_default()
            } else {
                String::new()
            };
            return Some(format!("{base}{suffix}"));
        }
    }

    let base = read_ascii_cstr(info, local_base_path_offset)?;
    let suffix = read_ascii_cstr(info, common_suffix_offset).unwrap_or_default();
    Some(format!("{base}{suffix}"))
}

fn read_ascii_cstr(data: &[u8], offset: usize) -> Option<String> {
    let bytes = data.get(offset..)?;
    let end = bytes.iter().position(|&b| b == 0)?;
    Some(String::from_utf8_lossy(&bytes[..end]).into_owned())
}

fn read_utf16_cstr(data: &[u8], offset: usize) -> Option<String> {
    let bytes = data.get(offset..)?;
    let mut units = Vec::new();
    let mut i = 0;
    while i + 1 < bytes.len() {
        let unit = u16::from_le_bytes([bytes[i], bytes[i + 1]]);
        if unit == 0 {
            break;
        }
        units.push(unit);
        i += 2;
    }
    Some(String::from_utf16_lossy(&units))
}

#[cfg(test)]
mod tests;
