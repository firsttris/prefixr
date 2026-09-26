use super::*;
use std::fs;

use crate::config::AppConfig;
use crate::models::{GameOverrides, GitHubConfig, GraphicsConfig, MangoHudConfig, PerformanceConfig, PrefixInfo, ProtonConfig};

fn temp_path(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("prefixr-{name}-{}", Uuid::new_v4()));
    fs::create_dir_all(&path).unwrap();
    path
}

fn sample_game(id: Uuid, name: &str) -> Game {
    Game {
        id,
        name: name.to_string(),
        exe_path: PathBuf::from("/games/game.exe"),
        prefix_path: PathBuf::from("/prefix"),
        runner_id: "runner".to_string(),
        env_vars: Default::default(),
        launch_args: String::new(),
        icon: None,
        steamgriddb_id: None,
        cover_grid_id: None,
        cover_url: None,
        steamgriddb_icon_grid_id: None,
        steamgriddb_icon_url: None,
        artwork: Default::default(),
        umu_id: None,
        umu_store: None,
        overrides: GameOverrides::default(),
    }
}

fn sample_config(games: Vec<Game>) -> AppConfig {
    AppConfig {
        runners_dir: PathBuf::from("/runners"),
        prefixes: Vec::<PrefixInfo>::new(),
        games,
        mangohud: MangoHudConfig::default(),
        performance: PerformanceConfig::default(),
        graphics: GraphicsConfig::default(),
        proton: ProtonConfig::default(),
        steamgriddb: SteamGridDbConfig::default(),
        github: GitHubConfig::default(),
    }
}

#[test]
fn builds_urls_with_percent_encoded_segments() {
    let url = build_url(&["search", "autocomplete", "NieR: Automata / GOTY"]).unwrap();

    assert_eq!(
        url.as_str(),
        "https://www.steamgriddb.com/api/v2/search/autocomplete/NieR:%20Automata%20%2F%20GOTY"
    );
}

#[test]
fn trims_or_rejects_api_keys() {
    assert_eq!(sanitized_api_key(Some("  secret-key  ")), Some("secret-key".to_string()));
    assert_eq!(sanitized_api_key(Some("  \t\n  ")), None);
    assert_eq!(sanitized_api_key(None), None);
}

#[test]
fn artwork_kinds_map_to_expected_endpoints() {
    assert_eq!(artwork_endpoint(ArtworkKind::Wide), ("grids", Some("920x430,460x215")));
    assert_eq!(artwork_endpoint(ArtworkKind::Hero), ("heroes", None));
    assert_eq!(artwork_endpoint(ArtworkKind::Logo), ("logos", None));
}

#[test]
fn detects_image_extensions_and_mime_types() {
    assert_eq!(image_extension("https://cdn/foo/bar.jpg?size=600"), "jpg");
    assert_eq!(image_extension("https://cdn/foo/bar.JPEG"), "jpg");
    assert_eq!(image_extension("https://cdn/foo/bar.webp"), "webp");
    assert_eq!(image_extension("https://cdn/foo/bar.unknown"), "png");
    assert_eq!(image_mime("jpg"), "image/jpeg");
    assert_eq!(image_mime("webp"), "image/webp");
    assert_eq!(image_mime("png"), "image/png");
}

#[test]
fn image_data_url_uses_matching_mime_and_base64() {
    assert_eq!(image_data_url(b"\xff\xd8", "jpg"), "data:image/jpeg;base64,/9g=");
    assert_eq!(image_data_url(b"RIFF", "webp"), "data:image/webp;base64,UklGRg==");
    assert_eq!(image_data_url(b"PNG", "png"), "data:image/png;base64,UE5H");
}

#[test]
fn builds_cache_paths_by_kind_suffix_and_extension() {
    let dir = Path::new("/tmp/artwork-cache");
    let id = Uuid::nil();

    assert_eq!(
        asset_cache_path(dir, id, "", "png"),
        PathBuf::from("/tmp/artwork-cache/00000000-0000-0000-0000-000000000000.png")
    );
    assert_eq!(
        asset_cache_path(dir, id, "_icon", "webp"),
        PathBuf::from("/tmp/artwork-cache/00000000-0000-0000-0000-000000000000_icon.webp")
    );
}

#[test]
fn removes_only_stale_files_for_the_requested_kind() {
    let dir = temp_path("steamgriddb-stale");
    let id = Uuid::new_v4();
    let cover = asset_cache_path(&dir, id, "", "png");
    let icon = asset_cache_path(&dir, id, "_icon", "png");
    let other_game = asset_cache_path(&dir, Uuid::new_v4(), "", "png");

    fs::write(&cover, b"cover").unwrap();
    fs::write(&icon, b"icon").unwrap();
    fs::write(&other_game, b"other").unwrap();

    remove_stale_asset_files(&dir, id, "").unwrap();

    assert!(!cover.exists());
    assert!(icon.exists());
    assert!(other_game.exists());

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn remove_stale_asset_files_ignores_missing_directories() {
    let dir = std::env::temp_dir().join(format!("prefixr-missing-artwork-{}", Uuid::new_v4()));

    assert!(remove_stale_asset_files(&dir, Uuid::new_v4(), "_icon").is_ok());
}

#[test]
fn sgdb_asset_conversion_preserves_fields() {
    let grid = SteamGridDbGrid::from(SgdbAsset {
        id: 42,
        url: "https://cdn.example/grid.png".to_string(),
        thumb: "https://cdn.example/thumb.png".to_string(),
        width: 600,
        height: 900,
    });

    assert_eq!(grid.id, 42);
    assert_eq!(grid.url, "https://cdn.example/grid.png");
    assert_eq!(grid.thumb, "https://cdn.example/thumb.png");
    assert_eq!(grid.width, 600);
    assert_eq!(grid.height, 900);
}

#[test]
fn find_game_returns_the_matching_entry() {
    let id = Uuid::new_v4();
    let mut config = sample_config(vec![sample_game(id, "Cyberpunk 2077")]);

    let game = find_game(&mut config, &id.to_string()).unwrap();

    assert_eq!(game.name, "Cyberpunk 2077");
    game.cover_url = Some("https://cdn.example/cover.png".to_string());
    assert_eq!(config.games[0].cover_url.as_deref(), Some("https://cdn.example/cover.png"));
}

#[test]
fn find_game_rejects_invalid_or_unknown_ids() {
    let mut config = sample_config(Vec::new());

    assert!(find_game(&mut config, "not-a-uuid")
        .unwrap_err()
        .starts_with("Invalid game id:"));

    let missing = Uuid::new_v4().to_string();
    assert_eq!(find_game(&mut config, &missing).unwrap_err(), format!("No game with id {missing}"));
}

#[test]
fn ico_icons_become_png() {
    let mut ico = Vec::new();
    image::DynamicImage::new_rgba8(32, 32)
        .write_to(&mut std::io::Cursor::new(&mut ico), image::ImageFormat::Ico)
        .unwrap();
    let png = ico_to_png(ico).unwrap();
    assert!(png.starts_with(b"\x89PNG"));
    assert_eq!(image::load_from_memory(&png).unwrap().width(), 32);

    let other = b"\x89PNG not touched".to_vec();
    assert_eq!(ico_to_png(other.clone()).unwrap(), other);
}
