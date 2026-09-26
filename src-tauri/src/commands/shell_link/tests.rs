use super::{parse_link_info, read_ascii_cstr, read_utf16_cstr, resolve_windows_path};
use std::fs;
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use uuid::Uuid;

fn temp_path(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("prefixr-{name}-{}", Uuid::new_v4()));
    fs::create_dir_all(&path).unwrap();
    path
}

fn build_ascii_link_info(base: &str, suffix: &str) -> Vec<u8> {
    let header_size = 0x1Cusize;
    let base_offset = header_size;
    let suffix_offset = base_offset + base.len() + 1;
    let size = suffix_offset + suffix.len() + 1;
    let mut info = vec![0u8; size];
    info[0..4].copy_from_slice(&(size as u32).to_le_bytes());
    info[4..8].copy_from_slice(&(header_size as u32).to_le_bytes());
    info[8..12].copy_from_slice(&1u32.to_le_bytes());
    info[16..20].copy_from_slice(&(base_offset as u32).to_le_bytes());
    info[24..28].copy_from_slice(&(suffix_offset as u32).to_le_bytes());
    info[base_offset..base_offset + base.len()].copy_from_slice(base.as_bytes());
    info[suffix_offset..suffix_offset + suffix.len()].copy_from_slice(suffix.as_bytes());
    info
}

fn build_unicode_link_info(base: &str, suffix: &str) -> Vec<u8> {
    let header_size = 0x24usize;
    let base_units: Vec<u16> = base.encode_utf16().collect();
    let suffix_units: Vec<u16> = suffix.encode_utf16().collect();
    let base_offset = header_size;
    let suffix_offset = base_offset + (base_units.len() + 1) * 2;
    let size = suffix_offset + (suffix_units.len() + 1) * 2;
    let mut info = vec![0u8; size];
    info[0..4].copy_from_slice(&(size as u32).to_le_bytes());
    info[4..8].copy_from_slice(&(header_size as u32).to_le_bytes());
    info[8..12].copy_from_slice(&1u32.to_le_bytes());
    info[16..20].copy_from_slice(&1u32.to_le_bytes());
    info[24..28].copy_from_slice(&1u32.to_le_bytes());
    info[28..32].copy_from_slice(&(base_offset as u32).to_le_bytes());
    info[32..36].copy_from_slice(&(suffix_offset as u32).to_le_bytes());

    for (index, unit) in base_units.into_iter().enumerate() {
        let start = base_offset + index * 2;
        info[start..start + 2].copy_from_slice(&unit.to_le_bytes());
    }
    for (index, unit) in suffix_units.into_iter().enumerate() {
        let start = suffix_offset + index * 2;
        info[start..start + 2].copy_from_slice(&unit.to_le_bytes());
    }
    info
}

#[test]
fn resolves_windows_paths_through_dosdevices_symlinks() {
    let prefix = temp_path("shell-link-prefix");
    let drive_target = prefix.join("mapped-c");
    fs::create_dir_all(drive_target.join("Games/Foo")).unwrap();
    fs::create_dir_all(prefix.join("dosdevices")).unwrap();
    symlink(&drive_target, prefix.join("dosdevices/c:")).unwrap();

    let resolved = resolve_windows_path(&prefix, r"C:\Games/Foo\game.exe").unwrap();
    assert_eq!(resolved, drive_target.join("Games/Foo/game.exe"));

    let _ = fs::remove_dir_all(prefix);
}

#[test]
fn parses_ascii_link_info_targets() {
    let info = build_ascii_link_info(r"C:\Games\Foo", r"\game.exe");

    assert_eq!(
        parse_link_info(&info, 0),
        Some(r"C:\Games\Foo\game.exe".to_string())
    );
}

#[test]
fn parses_unicode_link_info_targets() {
    let info = build_unicode_link_info(r"C:\Spiele\Äon", r"\start.exe");

    assert_eq!(
        parse_link_info(&info, 0),
        Some(r"C:\Spiele\Äon\start.exe".to_string())
    );
}

#[test]
fn c_string_helpers_stop_at_the_first_null() {
    assert_eq!(read_ascii_cstr(b"foo\0bar", 0), Some("foo".to_string()));

    let utf16 = [b'f', 0, b'o', 0, b'o', 0, 0, 0, b'b', 0, b'a', 0, b'r', 0];
    assert_eq!(read_utf16_cstr(&utf16, 0), Some("foo".to_string()));
}
