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

#[test]
fn compatibility_is_only_in_the_json_when_looked_up() {
    let mut list = entries(&config(), &store_games());
    assert!(serde_json::to_value(&list[0])
        .unwrap()
        .get("compatibility")
        .is_none());
    list[0].compatibility = Some(Compatibility {
        umu_id: Some("umu-1091500".to_string()),
        umu_store: None,
        protondb_tier: Some("gold".to_string()),
        steam_app_id: Some(1091500),
    });
    let json = serde_json::to_value(&list[0]).unwrap();
    assert_eq!(json["compatibility"]["protondb_tier"], "gold");
    let rows = entry_rows(&[&list[0]]);
    assert_eq!(rows[0][6], "protondb:gold, umu-fix");
}

#[test]
fn listing_filters_by_hidden_favorites_and_category() {
    let mut list = entries(&config(), &store_games());
    list[0].hidden = true;
    let keys = |list: Vec<LibraryEntry>| list.into_iter().map(|e| e.key).collect::<Vec<_>>();
    assert_eq!(
        keys(filter_entries(list.clone(), false, false, None, None)),
        ["epic:Ferry", "gog:1"]
    );
    assert_eq!(
        keys(filter_entries(list.clone(), true, false, None, None)).len(),
        3
    );
    assert_eq!(
        keys(filter_entries(list.clone(), false, true, None, None)),
        ["gog:1"]
    );
    assert_eq!(
        keys(filter_entries(list, false, false, Some("Kinder"), None)),
        ["epic:Ferry"]
    );
}

#[test]
fn test_results_are_listed_and_filtered_while_they_hold() {
    let mut config = config();
    let hollow = config.games[0].clone();
    let ferry = config.games[1].clone();
    config
        .library
        .set_tested(&hollow, Some(TestStatus::Works), "Menu reached", 5);
    config.library.set_tested(&ferry, Some(TestStatus::Broken), "", 5);
    // Since tested, Night Ferry got another runner.
    config.games[1].runner_id = "GE-Proton10-7".to_string();
    let list = entries(&config, &store_games());

    let hollow = list[0].tested.as_ref().unwrap();
    assert!(hollow.current);
    assert_eq!(hollow.result.note, "Menu reached");
    assert!(!list[1].tested.as_ref().unwrap().current);
    assert!(list[2].tested.is_none());

    let keys = |list: Vec<LibraryEntry>| list.into_iter().map(|e| e.key).collect::<Vec<_>>();
    let filter = |f| keys(filter_entries(list.clone(), false, false, None, Some(f)));
    assert_eq!(
        filter(TestedFilter::Works),
        ["3f1c2a52-9a43-4f53-9a8e-1b2c3d4e5f60"]
    );
    assert!(filter(TestedFilter::Broken).is_empty());
    assert_eq!(filter(TestedFilter::Untested), ["epic:Ferry", "gog:1"]);

    let rows = entry_rows(&list.iter().collect::<Vec<_>>());
    assert_eq!(rows[0][6], "tested:works");
    assert_eq!(rows[1][6], "tested:broken (outdated)");
}

#[test]
fn a_test_result_is_listed_in_the_json_with_whether_it_holds() {
    let mut config = config();
    let hollow = config.games[0].clone();
    config
        .library
        .set_tested(&hollow, Some(TestStatus::Issues), "No sound", 9);
    let list = entries(&config, &store_games());
    let json = serde_json::to_value(&list[0]).unwrap();
    assert_eq!(
        json["tested"],
        serde_json::json!({
            "status": "issues",
            "note": "No sound",
            "tested_at": 9,
            "runner_id": "wine",
            "exe_path": "/games/hollow.exe",
            "current": true,
        })
    );
    assert_eq!(serde_json::to_value(&list[2]).unwrap()["tested"], serde_json::Value::Null);
}
