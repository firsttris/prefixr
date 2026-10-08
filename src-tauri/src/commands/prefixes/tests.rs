use super::*;
use crate::test_util::TestDir;

#[test]
fn proton_compat_data_folders_use_their_pfx() {
    let dir = TestDir::new("dir");
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
}

#[test]
fn known_prefixes_are_listed_or_used_by_a_game() {
    let game: crate::models::Game = serde_json::from_value(serde_json::json!({
        "id": uuid::Uuid::new_v4(),
        "name": "Game",
        "exe_path": "/games/game.exe",
        "prefix_path": "/prefixes/removed-from-list",
        "runner_id": "wine",
    }))
    .unwrap();
    let config: AppConfig = serde_json::from_value(serde_json::json!({
        "runners_dir": "/runners",
        "prefixes": [{ "path": "/prefixes/listed" }],
        "games": [game],
    }))
    .unwrap();

    assert!(known_prefix(&config, "/prefixes/listed").is_ok());
    assert!(known_prefix(&config, "/prefixes/removed-from-list").is_ok());
    assert!(known_prefix(&config, "/home/user/.ssh").is_err());
}
