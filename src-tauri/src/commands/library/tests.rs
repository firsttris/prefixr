use super::*;

fn game(store: Option<(StoreKind, &str)>) -> Game {
    let mut value = serde_json::json!({
        "id": "3f1c2a52-9a43-4f53-9a8e-1b2c3d4e5f60",
        "name": "Game",
        "exe_path": "/games/game.exe",
        "prefix_path": "/prefixes/game",
        "runner_id": "wine",
    });
    if let Some((store, app_name)) = store {
        value["store"] = serde_json::json!({ "store": store, "app_name": app_name });
    }
    serde_json::from_value(value).unwrap()
}

#[test]
fn store_games_are_keyed_by_their_store_entry() {
    assert_eq!(game(None).id.to_string(), game_key(&game(None)));
    assert_eq!(
        game_key(&game(Some((StoreKind::Gog, "1207658924")))),
        "gog:1207658924"
    );
    assert_eq!(store_key(StoreKind::Epic, "Fortnite"), "epic:Fortnite");
}

#[test]
fn favorites_and_hidden_toggle_without_duplicates() {
    let mut library = LibraryConfig::default();
    library.set_favorite("a", true);
    library.set_favorite("a", true);
    library.set_hidden("b", true);
    assert_eq!(library.favorites, ["a"]);
    library.set_favorite("a", false);
    assert!(library.favorites.is_empty());
    assert_eq!(library.hidden, ["b"]);
}

#[test]
fn categories_are_found_by_name_or_id_and_stay_unique() {
    let mut library = LibraryConfig::default();
    let kids = library.create_category("  Kinder ").unwrap();
    assert_eq!(kids.name, "Kinder");
    assert!(library.create_category("kinder").is_err());
    assert!(library.create_category("   ").is_err());
    assert_eq!(library.find_category("KINDER").unwrap().id, kids.id);
    assert_eq!(library.find_category(&kids.id).unwrap().name, "Kinder");

    library.create_category("Koop").unwrap();
    assert!(library.rename_category("Koop", "Kinder").is_err());
    library.rename_category("koop", "Koop-Spiele").unwrap();
    assert!(library.find_category("Koop-Spiele").is_ok());

    library.delete_category("Kinder").unwrap();
    assert!(library.find_category("Kinder").is_err());
    assert!(library.delete_category("Kinder").is_err());
}

#[test]
fn games_are_put_in_and_taken_out_of_categories() {
    let mut library = LibraryConfig::default();
    let kids = library.create_category("Kinder").unwrap();
    let coop = library.create_category("Koop").unwrap();
    let keys = ["a".to_string(), "b".to_string()];
    library.set_in_category("Kinder", &keys, true).unwrap();
    library.set_in_category("Kinder", &keys[..1], true).unwrap();
    assert_eq!(library.categories[0].games, keys);

    library.set_in_category(&coop.id, &keys[..1], true).unwrap();
    assert_eq!(library.categories_of("a"), ["Kinder", "Koop"]);
    assert_eq!(library.categories_of("b"), ["Kinder"]);
    assert!(library.set_in_category("nope", &keys, true).is_err());

    library.set_in_category(&kids.id, &keys, false).unwrap();
    assert!(library.categories[0].games.is_empty());
}

#[test]
fn activity_keeps_the_first_added_time_and_sums_sessions() {
    let mut library = LibraryConfig::default();
    library.record_added("a", 100);
    library.record_added("a", 200);
    library.record_session("a", 300, Duration::from_secs(60));
    library.record_session("a", 400, Duration::from_secs(90));
    let activity = &library.activity["a"];
    assert_eq!(activity.added_at, Some(100));
    assert_eq!(activity.last_played, Some(400));
    assert_eq!(activity.playtime_secs, 150);
}

#[test]
fn forgetting_a_game_drops_it_everywhere() {
    let mut library = LibraryConfig::default();
    library.create_category("Kinder").unwrap();
    library
        .set_in_category("Kinder", &["a".to_string()], true)
        .unwrap();
    library.set_favorite("a", true);
    library.set_hidden("a", true);
    library.record_added("a", 1);
    library.forget("a");
    assert_eq!(library.categories_of("a"), Vec::<String>::new());
    assert!(
        library.favorites.is_empty() && library.hidden.is_empty() && library.activity.is_empty()
    );
}

#[test]
fn configs_without_a_library_load_with_an_empty_one() {
    let config: AppConfig =
        serde_json::from_value(serde_json::json!({ "runners_dir": "/runners" })).unwrap();
    assert_eq!(config.library, LibraryConfig::default());
}

#[test]
fn removing_a_store_game_keeps_its_organization_for_a_reinstall() {
    let own = game(None);
    let from_store = game(Some((StoreKind::Epic, "Ferry")));
    let mut library = LibraryConfig::default();
    for key in [game_key(&own), game_key(&from_store)] {
        library.set_favorite(&key, true);
        library.record_added(&key, 1);
    }
    library.game_removed(&from_store);
    library.game_removed(&own);
    assert_eq!(library.favorites, ["epic:Ferry"]);
    assert!(library.activity.contains_key("epic:Ferry"));
    assert!(!library.activity.contains_key(&game_key(&own)));
}

#[test]
fn a_test_result_keeps_the_setup_it_was_found_with() {
    let mut library = LibraryConfig::default();
    let mut game = game(Some((StoreKind::Epic, "Ferry")));
    library.set_tested(
        &game,
        Some(TestStatus::Issues),
        "  Videos stay black \n",
        1_700,
    );
    let result = &library.tested["epic:Ferry"];
    assert_eq!(result.status, TestStatus::Issues);
    assert_eq!(result.note, "Videos stay black");
    assert_eq!(result.tested_at, 1_700);
    assert_eq!(result.runner_id, "wine");
    assert_eq!(result.exe_path, "/games/game.exe");
    assert!(result.is_current(&game));

    game.runner_id = "GE-Proton10-7".to_string();
    assert!(!library.tested["epic:Ferry"].is_current(&game));

    library.set_tested(&game, None, "", 1_800);
    assert!(library.tested.is_empty());
}

#[test]
fn a_test_result_goes_with_a_game_removed_for_good_only() {
    let mut library = LibraryConfig::default();
    let local = game(None);
    let store = game(Some((StoreKind::Gog, "1")));
    library.set_tested(&local, Some(TestStatus::Works), "", 1);
    library.set_tested(&store, Some(TestStatus::Broken), "", 1);
    library.game_removed(&local);
    library.game_removed(&store);
    assert_eq!(library.tested.keys().collect::<Vec<_>>(), ["gog:1"]);
}

#[test]
fn a_library_from_before_test_results_loads() {
    let library: LibraryConfig =
        serde_json::from_value(serde_json::json!({ "favorites": ["a"] })).unwrap();
    assert!(library.tested.is_empty());
    let status: TestStatus = serde_json::from_value(serde_json::json!("broken")).unwrap();
    assert_eq!(status, TestStatus::Broken);
}
