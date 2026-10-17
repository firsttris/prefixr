use super::*;
use crate::test_util::TestDir;

fn entry(app_name: &str, title: &str, cover: Option<&str>) -> StoreLibraryEntry {
    StoreLibraryEntry {
        app_name: app_name.to_string(),
        title: title.to_string(),
        cover_url: cover.map(str::to_string),
    }
}

fn game(id: &str, store: Option<(StoreKind, &str)>) -> Game {
    let mut game: Game = serde_json::from_value(serde_json::json!({
        "id": id,
        "name": "A game",
        "exe_path": "/g/game.exe",
        "prefix_path": "/p",
        "runner_id": "r",
    }))
    .unwrap();
    game.store = store.map(|(store, app_name)| StoreLink {
        store,
        app_name: app_name.to_string(),
        install_path: None,
        prerequisites: Vec::new(),
    });
    game
}

const INSTALLED_ID: &str = "6f2c1a3e-0000-4000-8000-000000000001";

#[test]
fn joins_the_library_with_the_installed_games() {
    let entries = [
        entry("Lantern", "Hollow Lantern", None),
        entry("Ashen", "Ashen Road", None),
    ];
    let games = [
        game(INSTALLED_ID, Some((StoreKind::Epic, "Lantern"))),
        // A game added by hand with the same name is no store game.
        game("6f2c1a3e-0000-4000-8000-000000000002", None),
    ];
    let listed = store_games(StoreKind::Epic, &entries, &games, None);
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].game_id.as_deref(), Some(INSTALLED_ID));
    assert_eq!(listed[0].title, "Hollow Lantern");
    assert_eq!(listed[1].game_id, None);
    assert!(listed.iter().all(|g| g.cover_path.is_none()));
}

#[test]
fn shows_a_cover_once_it_is_cached() {
    let artwork = TestDir::new("artwork");
    let entries = [
        entry(
            "Lantern",
            "Hollow Lantern",
            Some("https://cdn1.epicgames.com/a.jpg?h=600"),
        ),
        entry(
            "Ashen",
            "Ashen Road",
            Some("https://cdn1.epicgames.com/b.png"),
        ),
        entry("Iron", "Iron Vale", None),
    ];
    fs::write(artwork.join("store-epic-Lantern.jpg"), b"jpg").unwrap();
    let listed = store_games(StoreKind::Epic, &entries, &[], Some(&artwork));
    assert_eq!(
        listed[0].cover_path.as_deref(),
        Some(artwork.join("store-epic-Lantern.jpg").to_str().unwrap())
    );
    // Not downloaded (yet), or none at all.
    assert_eq!(listed[1].cover_path, None);
    assert_eq!(listed[2].cover_path, None);
}

#[test]
fn cover_files_live_in_the_artwork_folder() {
    let path = cover_cache_path(
        Path::new("/data/artwork"),
        StoreKind::Epic,
        &entry(
            "../../etc/passwd",
            "x",
            Some("https://cdn1.epicgames.com/a.webp"),
        ),
    )
    .unwrap();
    assert_eq!(
        path,
        PathBuf::from("/data/artwork/store-epic-______etc_passwd.webp")
    );
    assert_eq!(
        cover_cache_path(Path::new("/a"), StoreKind::Epic, &entry("X", "x", None)),
        None
    );
}

#[test]
fn file_names_keep_only_safe_characters() {
    assert_eq!(file_safe("Fortnite"), "Fortnite");
    assert_eq!(file_safe("a1b2-c_d"), "a1b2-c_d");
    assert_eq!(file_safe("a b/c.d"), "a_b_c_d");
}

#[test]
fn the_library_cache_survives_a_round_trip() {
    let entries = vec![entry(
        "Lantern",
        "Hollow Lantern",
        Some("https://cdn1.epicgames.com/a.jpg"),
    )];
    let raw = serde_json::to_vec(&entries).unwrap();
    let back: Vec<StoreLibraryEntry> = serde_json::from_slice(&raw).unwrap();
    assert_eq!(back, entries);
    // An entry from before covers were cached still reads.
    let old: Vec<StoreLibraryEntry> =
        serde_json::from_str(r#"[{"app_name":"A","title":"B"}]"#).unwrap();
    assert_eq!(old[0].cover_url, None);
}

#[test]
fn a_game_from_before_stores_has_none() {
    let game = game(INSTALLED_ID, None);
    assert_eq!(game.store, None);
    let linked: Game = serde_json::from_value(serde_json::json!({
        "id": INSTALLED_ID,
        "name": "A game",
        "exe_path": "/g/game.exe",
        "prefix_path": "/p",
        "runner_id": "r",
        "store": { "store": "epic", "app_name": "Lantern" },
    }))
    .unwrap();
    let link = linked.store.unwrap();
    assert_eq!(link.store, StoreKind::Epic);
    assert!(link.prerequisites.is_empty());
    assert_eq!(link.install_path, None);
}
