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

#[test]
fn layer_statuses_read_the_recorded_versions() {
    let cache = std::env::temp_dir().join(format!("prefixr-test-layers-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(cache.join("dxvk")).unwrap();
    fs::write(version_file(&cache.join("dxvk")), "v2.7.1\n").unwrap();

    let statuses = layer_statuses(&cache);

    assert_eq!(statuses[0].label, "DXVK");
    assert!(statuses[0].installed);
    assert_eq!(statuses[0].version.as_deref(), Some("v2.7.1"));
    assert_eq!(statuses[1].label, "VKD3D-Proton");
    assert!(!statuses[1].installed);
    assert_eq!(statuses[1].version, None);
    fs::remove_dir_all(cache).unwrap();
}

#[test]
fn layer_assets_match_real_release_names() {
    let dxvk = LAYERS[0].matches_asset;
    assert!(dxvk("dxvk-2.7.1.tar.gz"));
    assert!(!dxvk("dxvk-native-2.7.1-steamrt-sniper.tar.gz"));
    let vkd3d = LAYERS[1].matches_asset;
    assert!(vkd3d("vkd3d-proton-2.14.1.tar.zst"));
}

#[test]
fn latest_version_dir_compares_versions_numerically() {
    let listing = ["../", "10.0.0/", "10.1.0/", "9.4.0/", "README", "?C=M;O=A"];
    assert_eq!(latest_version_dir(&listing).as_deref(), Some("10.1.0"));
    assert_eq!(latest_version_dir(&["../", "notes/"]), None);
}
