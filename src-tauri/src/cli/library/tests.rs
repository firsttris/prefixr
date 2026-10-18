use super::*;

fn config() -> AppConfig {
    let mut config: AppConfig = serde_json::from_value(serde_json::json!({
        "runners_dir": "/runners",
        "games": [
            {
                "id": "3f1c2a52-9a43-4f53-9a8e-1b2c3d4e5f60",
                "name": "Hollow Signal",
                "exe_path": "/games/hollow.exe",
                "prefix_path": "/prefixes/a",
                "runner_id": "wine",
            },
            {
                "id": "9b0e8a11-0000-4000-8000-000000000001",
                "name": "Night Ferry",
                "exe_path": "/games/ferry.exe",
                "prefix_path": "/prefixes/b",
                "runner_id": "wine",
                "store": { "store": "epic", "app_name": "Ferry" },
            },
        ],
    }))
    .unwrap();
    config.library.create_category("Kinder").unwrap();
    config
        .library
        .set_in_category("Kinder", &["epic:Ferry".to_string()], true)
        .unwrap();
    config.library.set_favorite("gog:1", true);
    config
}

fn store_games() -> Vec<(String, String, String)> {
    [
        ("epic:Ferry", "Night Ferry", "epic"),
        ("gog:1", "Paper Comet", "gog"),
    ]
    .map(|(k, n, s)| (k.to_string(), n.to_string(), s.to_string()))
    .to_vec()
}

#[test]
fn the_whole_library_lists_installed_games_once_and_store_games_after() {
    let list = entries(&config(), &store_games());
    let keys: Vec<_> = list.iter().map(|e| e.key.as_str()).collect();
    assert_eq!(
        keys,
        [
            "3f1c2a52-9a43-4f53-9a8e-1b2c3d4e5f60",
            "epic:Ferry",
            "gog:1"
        ]
    );
    assert_eq!(list[0].source, "local");
    assert!(list[1].installed && !list[2].installed);
    assert_eq!(list[1].categories, ["Kinder"]);
    assert!(list[2].favorite && list[2].game_id.is_none());
}

#[test]
fn games_are_found_by_key_id_or_name() {
    let list = entries(&config(), &store_games());
    assert_eq!(find_entry(&list, "gog:1").unwrap().name, "Paper Comet");
    assert_eq!(find_entry(&list, "3f1c2a52").unwrap().name, "Hollow Signal");
    assert_eq!(find_entry(&list, "paper comet").unwrap().key, "gog:1");
    assert_eq!(find_entry(&list, "ferry").unwrap().key, "epic:Ferry");
    assert!(find_entry(&list, "e").is_err(), "several match");
    assert!(find_entry(&list, "nothing like it").is_err());
}
