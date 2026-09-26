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
