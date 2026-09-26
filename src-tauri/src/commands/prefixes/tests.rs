use super::*;

#[test]
fn proton_compat_data_folders_use_their_pfx() {
    let dir = std::env::temp_dir().join(format!("prefixr-test-{}", uuid::Uuid::new_v4()));
    let compat = dir.join("compatdata/1091500");
    fs::create_dir_all(compat.join("pfx/drive_c")).unwrap();
    assert_eq!(effective_prefix_path(&compat), compat.join("pfx"));

    let wine = dir.join("wine-prefix");
    fs::create_dir_all(wine.join("drive_c")).unwrap();
    assert_eq!(effective_prefix_path(&wine), wine);

    // umu's layout: `pfx` links back to the prefix itself.
    let umu = dir.join("umu-prefix");
    fs::create_dir_all(&umu).unwrap();
    std::os::unix::fs::symlink(".", umu.join("pfx")).unwrap();
    assert_eq!(effective_prefix_path(&umu), umu);

    // Empty or not yet created: used as picked.
    assert_eq!(effective_prefix_path(&dir.join("new")), dir.join("new"));
    fs::remove_dir_all(&dir).unwrap();
}
