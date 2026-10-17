use super::*;
use crate::test_util::TestDir;
use std::fs;

const GAMESDB_GAME: &str = include_str!("fixtures/gamesdb_game.json");
const GOGGAME_INFO: &str = include_str!("fixtures/goggame.info");

#[test]
fn takes_the_code_from_the_address_the_login_ended_on() {
    let code = "abCD12-_xyzLongEnough";
    let address = format!("https://embed.gog.com/on_login_success?origin=client&code={code}");
    assert_eq!(auth_code(&address).as_deref(), Some(code));
    assert_eq!(auth_code(&format!("{address}#top")).as_deref(), Some(code));
    assert_eq!(
        auth_code(&format!(
            "https://embed.gog.com/on_login_success?code={code}&origin=client"
        ))
        .as_deref(),
        Some(code)
    );
    assert_eq!(auth_code(&format!("  {code}\n")).as_deref(), Some(code));
}

#[test]
fn refuses_what_isnt_a_code() {
    assert_eq!(auth_code(""), None);
    assert_eq!(auth_code("short"), None);
    assert_eq!(
        auth_code("https://embed.gog.com/on_login_success?origin=client"),
        None
    );
    assert_eq!(auth_code("https://embed.gog.com/x?code="), None);
    assert_eq!(auth_code("not a code at all"), None);
}

#[test]
fn reads_the_login() {
    let credentials = parse_credentials(include_str!("fixtures/auth_code.json"))
        .unwrap()
        .unwrap();
    assert_eq!(credentials.access_token, "AT-0123456789");
    assert_eq!(credentials.user_id, "48551234567890123");
    // What gogdl prints without a login, and after a refused code.
    assert_eq!(parse_credentials("null\n").unwrap(), None);
    assert_eq!(parse_credentials(r#"{"error": true}"#).unwrap(), None);
    assert!(parse_credentials("garbage").is_err());
}

#[test]
fn tells_a_login_from_the_file_alone() {
    assert!(has_login(include_str!("fixtures/auth_file.json")));
    assert!(!has_login("{}"));
    assert!(!has_login(
        r#"{"46899977096215655": {"refresh_token": ""}}"#
    ));
    assert!(!has_login("not json"));
}

#[test]
fn reads_the_username() {
    assert_eq!(
        parse_username(include_str!("fixtures/user.json")).as_deref(),
        Some("PrefixrTester")
    );
    assert_eq!(parse_username(r#"{"username": " "}"#), None);
}

#[test]
fn keeps_only_gog_games_from_the_galaxy_library() {
    let (ids, next) = parse_library_page(include_str!("fixtures/library_page1.json")).unwrap();
    assert_eq!(ids, ["1207658924"]);
    assert_eq!(next.as_deref(), Some("page2token"));
    // A numeric id, and the last page.
    let (ids, next) = parse_library_page(include_str!("fixtures/library_page2.json")).unwrap();
    assert_eq!(ids, ["1453375253"]);
    assert_eq!(next, None);
    assert_eq!(
        parse_library_page(r#"{"items": []}"#).unwrap(),
        (Vec::new(), None)
    );
}

#[test]
fn reads_title_and_portrait_cover_from_gamesdb() {
    let entry = parse_gamesdb(GAMESDB_GAME).unwrap();
    assert_eq!(entry.app_name, "1207658924");
    assert_eq!(entry.title, "Jotun Valhalla Edition");
    assert_eq!(
        entry.cover_url.as_deref(),
        Some("https://images.gog.com/edd21e7a826f7774caf7138d57f75365c4c5c3f9b20f11b732cb5124597bacf1.jpg?namespace=gamesdb")
    );
}

#[test]
fn leaves_out_dlc_and_games_without_windows() {
    assert_eq!(
        parse_gamesdb(include_str!("fixtures/gamesdb_dlc.json")),
        None
    );
    assert_eq!(
        parse_gamesdb(include_str!("fixtures/gamesdb_mac_only.json")),
        None
    );
    assert_eq!(parse_gamesdb("{}"), None);
}

#[test]
fn takes_covers_only_from_gog_over_https() {
    assert!(is_gog_image_url("https://images.gog.com/a.jpg"));
    assert!(is_gog_image_url("https://images-1.gog-statics.com/a.jpg"));
    assert!(!is_gog_image_url("http://images.gog.com/a.jpg"));
    assert!(!is_gog_image_url(
        "https://images.gog.com.evil.example/a.jpg"
    ));
    let mut game: serde_json::Value = serde_json::from_str(GAMESDB_GAME).unwrap();
    game["game"]["vertical_cover"]["url_format"] = "https://evil.example/x{formatter}.{ext}".into();
    game["game"]["cover"]["url_format"] = "https://evil.example/y{formatter}.{ext}".into();
    assert_eq!(parse_gamesdb(&game.to_string()).unwrap().cover_url, None);
}

#[test]
fn sizes_add_the_language_to_what_every_language_needs() {
    let plan = parse_info(include_str!("fixtures/info.json")).unwrap();
    assert_eq!(plan.sizes.download_size, Some(1_800_000_000));
    assert_eq!(plan.sizes.disk_size, Some(2_500_000_000));
    assert_eq!(plan.folder_name, "Jotun");
    assert_eq!(plan.dependencies, ["MSVC2017", "ISI", "DXSETUP"]);
    let empty = parse_info(r#"{"size": {}}"#).unwrap();
    assert_eq!(empty.sizes, StoreInstallInfo::default());
    assert!(empty.dependencies.is_empty());
}

#[test]
fn download_installs_windows_build_with_dlc_into_the_folder() {
    let args = download_args("1207658924", Path::new("/home/user/Games/GOG"));
    let args: Vec<&str> = args.iter().map(|a| a.to_str().unwrap()).collect();
    assert_eq!(
        args,
        [
            "download",
            "1207658924",
            "--platform",
            "windows",
            "--path",
            "/home/user/Games/GOG",
            "--lang",
            "en-US",
            "--with-dlcs",
        ]
    );
    assert_eq!(info_args("1")[0], "info");
}

#[test]
fn redistributables_with_installers_in_the_games_order() {
    let wanted = ["DXSETUP", "MSVC2017", "ISI", "UNKNOWN"].map(String::from);
    let installers = redist_installers(
        include_str!("fixtures/redist_manifest.json"),
        &wanted,
        Path::new("/data/stores/gog/redist"),
    )
    .unwrap();
    let paths: Vec<&str> = installers.iter().map(|p| p.path.as_str()).collect();
    assert_eq!(
        paths,
        [
            "/data/stores/gog/redist/__redist/DirectX/DXSETUP.exe",
            "/data/stores/gog/redist/__redist/MSVC2017/VC_redist.x86.exe",
        ]
    );
    // Without a readable name, the id.
    assert_eq!(installers[0].name, "DXSETUP");
    assert_eq!(installers[1].args, "/install /quiet /norestart");
    assert!(installers.iter().all(|p| !p.installed));
    assert_eq!(
        redist_args(&wanted[..2], Path::new("/r"))
            .iter()
            .map(|a| a.to_str().unwrap())
            .collect::<Vec<_>>(),
        ["redist", "--ids", "DXSETUP,MSVC2017", "--path", "/r"]
    );
}

#[test]
fn starts_the_primary_play_task() {
    let dir = TestDir::new("gog-launch");
    fs::create_dir_all(dir.join("Bin")).unwrap();
    fs::write(dir.join("Bin/Jotun.exe"), "").unwrap();
    let launch = parse_play_task(GOGGAME_INFO, &dir).unwrap();
    assert_eq!(launch.exe_path, dir.join("Bin/Jotun.exe"));
    assert_eq!(launch.working_dir, dir.join("Bin"));
    assert_eq!(launch.game_args, ["-windowed", "-config=My Settings.ini"]);
    assert!(launch.account_args.is_empty() && launch.env.is_empty());
}

#[test]
fn finds_the_exe_whatever_its_case_on_disk() {
    let dir = TestDir::new("gog-case");
    fs::create_dir_all(dir.join("bin")).unwrap();
    fs::write(dir.join("bin/jotun.EXE"), "").unwrap();
    let launch = parse_play_task(GOGGAME_INFO, &dir).unwrap();
    assert_eq!(launch.exe_path, dir.join("bin/jotun.EXE"));
    assert_eq!(launch.working_dir, dir.join("bin"));
}

#[test]
fn falls_back_to_the_first_program_and_list_arguments() {
    let info = r#"{"playTasks": [
        {"type": "URLTask", "link": "https://example.com"},
        {"type": "FileTask", "path": "Game.exe", "arguments": ["-a", "b c"]}
    ]}"#;
    let launch = parse_play_task(info, Path::new("/g")).unwrap();
    assert_eq!(launch.exe_path, PathBuf::from("/g/Game.exe"));
    assert_eq!(launch.game_args, ["-a", "b c"]);
    assert_eq!(launch.working_dir, PathBuf::from("/g"));
    assert!(parse_play_task(r#"{"playTasks": []}"#, Path::new("/g")).is_err());
    assert!(parse_play_task("{}", Path::new("/g")).is_err());
}

#[test]
fn reads_gogdls_progress_lines() {
    let line = "[PROGRESS] INFO: = Progress: 37.50 1500/4000, Running for 00:01:00, ETA: 00:01:40";
    assert_eq!(super::super::epic::parse_progress(line), Some(37.5));
}
