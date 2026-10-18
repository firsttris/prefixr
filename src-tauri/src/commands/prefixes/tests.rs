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

#[test]
fn prefix_states_follow_the_folder() {
    let dir = TestDir::new("dir");
    let ready = dir.join("ready");
    fs::create_dir_all(ready.join("drive_c")).unwrap();
    let fresh = dir.join("fresh");
    fs::create_dir_all(&fresh).unwrap();

    assert_eq!(prefix_state(&ready), PrefixState::Ready);
    assert_eq!(prefix_state(&fresh), PrefixState::NotSetUp);
    assert_eq!(prefix_state(&dir.join("gone")), PrefixState::Missing);
}

#[test]
fn a_file_at_a_prefix_path_is_missing_not_unreadable() {
    let dir = TestDir::new("dir");
    let file = dir.join("not-a-folder");
    fs::write(&file, "x").unwrap();
    assert_eq!(prefix_state(&file), PrefixState::Missing);
    // A path through the file fails with "not a directory", which is the
    // same: there's no folder there.
    assert_eq!(prefix_state(&file.join("prefix")), PrefixState::Missing);

    // A file named drive_c doesn't make it set up.
    let odd = dir.join("odd");
    fs::create_dir_all(&odd).unwrap();
    fs::write(odd.join("drive_c"), "x").unwrap();
    assert_eq!(prefix_state(&odd), PrefixState::NotSetUp);
}

#[test]
fn states_come_back_in_order_and_a_slow_folder_times_out() {
    fn slow_for_hang(path: &Path) -> PrefixState {
        if path.ends_with("hang") {
            std::thread::sleep(Duration::from_millis(500));
        }
        PrefixState::Ready
    }
    let paths = vec![
        PathBuf::from("/a"),
        PathBuf::from("/b/hang"),
        PathBuf::from("/c"),
    ];
    let states = tauri::async_runtime::block_on(states_of(
        paths.clone(),
        Duration::from_millis(100),
        slow_for_hang,
    ));
    let got: Vec<_> = states.iter().map(|s| (s.path.clone(), s.state)).collect();
    assert_eq!(
        got,
        vec![
            (paths[0].clone(), PrefixState::Ready),
            (paths[1].clone(), PrefixState::TimedOut),
            (paths[2].clone(), PrefixState::Ready),
        ]
    );
}
