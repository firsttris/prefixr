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

    library
        .set_game_categories("a", std::slice::from_ref(&coop.id))
        .unwrap();
    assert_eq!(library.categories_of("a"), ["Koop"]);
    assert_eq!(library.categories_of("b"), ["Kinder"]);
    assert!(library
        .set_game_categories("a", &["nope".to_string()])
        .is_err());

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
