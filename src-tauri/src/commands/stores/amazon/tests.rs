use super::*;
use crate::commands::stores::epic::parse_progress;

const LOGIN_REQUEST: &str = include_str!("fixtures/login_request.json");
const LIBRARY: &str = include_str!("fixtures/library.json");
const INSTALLED: &str = include_str!("fixtures/installed.json");
const LAUNCH: &str = include_str!("fixtures/launch.json");
const FUEL: &str = include_str!("fixtures/fuel.json");

const EXAMPLE_QUEST: &str = "amzn1.adg.product.5a2e1f4c-0000-4b3a-9e8f-2c1d0e9f8a7b";

#[test]
fn reads_the_login_request() {
    // The real output of `nile auth --login --non-interactive`.
    let request = parse_login_request(LOGIN_REQUEST).unwrap();
    assert_eq!(request.serial, "15619C48CA3711F1AAFC02FC00000001");
    assert_eq!(
        request.code_verifier,
        "bXjTkrP7PwwGrt1p2Nvn6_5QGY9j_WPAvv1VvQjYskY"
    );
    assert!(request.url.starts_with("https://amazon.com/ap/signin?"));
}

#[test]
fn opens_no_login_address_outside_amazon() {
    let request = |url: &str| {
        format!(r#"{{"client_id":"c","code_verifier":"v","serial":"s","url":"{url}"}}"#)
    };
    assert!(parse_login_request(&request("https://www.amazon.com/ap/signin?x=1")).is_ok());
    for url in [
        "https://amazon.com.evil.example/ap/signin",
        "http://amazon.com/ap/signin",
        "file:///etc/passwd",
        "not a url",
    ] {
        assert!(parse_login_request(&request(url)).is_err(), "{url}");
    }
    assert!(parse_login_request("").is_err());
    assert!(parse_login_request("ERROR [CLI]: You are already logged in").is_err());
}

#[test]
fn takes_the_code_from_the_address_amazon_redirects_to() {
    let pasted = "https://www.amazon.com/?openid.assoc_handle=amzn_sonic_games_launcher\
        &openid.claimed_id=https%3A%2F%2Fwww.amazon.com%2Fap%2Fid%2Famzn1.account.X\
        &openid.oa2.authorization_code=ANdNAVhyhqirUelHGEHA\
        &openid.mode=id_res";
    assert_eq!(auth_code(pasted).as_deref(), Some("ANdNAVhyhqirUelHGEHA"));
    // Pasted with quotes or spaces around it, or just the code itself.
    assert_eq!(
        auth_code(&format!(" \"{pasted}\" \n")).as_deref(),
        Some("ANdNAVhyhqirUelHGEHA")
    );
    assert_eq!(
        auth_code("ANdNAVhyhqirUelHGEHA").as_deref(),
        Some("ANdNAVhyhqirUelHGEHA")
    );
    // The sign-in page itself, or something else entirely.
    assert_eq!(
        auth_code("https://www.amazon.com/ap/signin?openid.mode=checkid_setup"),
        None
    );
    assert_eq!(auth_code("short"), None);
    assert_eq!(auth_code(""), None);
    assert_eq!(
        auth_code("https://www.amazon.com/?openid.oa2.authorization_code="),
        None
    );
}

#[test]
fn registers_with_what_the_login_request_carried() {
    let request = parse_login_request(LOGIN_REQUEST).unwrap();
    let args = register_args(&request, "CODE1234");
    assert_eq!(args[..3], ["register", "--code", "CODE1234"]);
    let value = |flag: &str| {
        let at = args.iter().position(|arg| arg == flag).unwrap();
        args[at + 1].clone()
    };
    assert_eq!(value("--client-id"), request.client_id);
    assert_eq!(value("--code-verifier"), request.code_verifier);
    assert_eq!(value("--serial"), request.serial);
}

#[test]
fn tells_whether_the_device_was_registered() {
    // Nile exits with 0 whether or not Amazon took the code; only its log
    // says (and spells it "Succesfully").
    assert!(registered(include_str!("fixtures/register_ok.txt")));
    let failed = include_str!("fixtures/register_failed.txt");
    assert!(!registered(failed));
    assert_eq!(
        error_message(failed).as_deref(),
        Some("Failed to register a device")
    );
}

#[test]
fn reads_the_error_of_a_crash() {
    // The real output of Nile not reaching Amazon.
    let crash = include_str!("fixtures/register_crash.txt");
    assert!(!registered(crash));
    let message = error_message(crash).unwrap();
    assert!(
        message.starts_with("HTTPSConnectionPool(host='api.amazon.com'"),
        "{message}"
    );
}

#[test]
fn ignores_warnings_and_info() {
    assert_eq!(
        error_message("WARNING [CLI]:\t There was an error getting sdk\nINFO [CLI]:\t Found: X\n"),
        None
    );
    assert_eq!(error_message(""), None);
    assert_eq!(
        error_message("ERROR [CLI]:\t Couldn't find what you are looking for\n").as_deref(),
        Some("Couldn't find what you are looking for")
    );
}

#[test]
fn reads_the_account_from_nile_s_files() {
    // As `register` writes it.
    assert_eq!(
        parse_current_user(r#"{"name": "Tristan", "user_id": "amzn1.account.AF2X"}"#).as_deref(),
        Some("Tristan")
    );
    assert_eq!(
        parse_current_user(r#"{"name": "", "user_id": "amzn1.account.AF2X"}"#).as_deref(),
        Some("Amazon")
    );
    // No user id, no login (`is_logged_in` asks the same).
    assert_eq!(
        parse_current_user(r#"{"name": "Tristan", "user_id": ""}"#),
        None
    );
    assert_eq!(parse_current_user(r#"{"name": "Tristan"}"#), None);
    assert_eq!(parse_current_user(""), None);
}

#[test]
fn logging_out_deletes_only_the_login() {
    let dir = crate::test_util::TestDir::new("nile-login");
    let files: &Path = &dir;
    for name in [
        "current_user.json",
        "0cc175b9c0f1b6a831c399e269772661.enc",
        "installed.json",
        "library.json",
    ] {
        std::fs::write(files.join(name), "{}").unwrap();
    }
    let mut found: Vec<String> = login_files(files)
        .iter()
        .map(|path| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    found.sort();
    assert_eq!(
        found,
        ["0cc175b9c0f1b6a831c399e269772661.enc", "current_user.json"]
    );
}

#[test]
fn a_sync_without_login_asks_to_connect() {
    assert!(needs_login("User not logged in"));
    assert!(!needs_login("Failed to register a device"));
}

#[test]
fn reads_the_library() {
    let games = parse_library(LIBRARY).unwrap();
    let ids: Vec<&str> = games.iter().map(|g| g.app_name.as_str()).collect();
    // Sorted by title, case and spaces aside; the entry Amazon sent twice
    // (two entitlements of one product) listed once; the broken one left out.
    assert_eq!(
        ids,
        [
            "amzn1.adg.product.99999999-8888-4777-8666-555555555555",
            "amzn1.adg.product.11111111-2222-4333-8444-555555555555",
            EXAMPLE_QUEST,
        ]
    );
    assert_eq!(games[0].title, "alpha Racer");
    // No title: it goes by its id.
    assert_eq!(games[1].title, games[1].app_name);
    assert_eq!(games[2].title, "Example Quest");
}

#[test]
fn covers_come_from_amazons_image_servers() {
    let games = parse_library(LIBRARY).unwrap();
    let cover = |id: &str| {
        games
            .iter()
            .find(|g| g.app_name == id)
            .unwrap()
            .cover_url
            .clone()
    };
    // The second background first, as in Heroic.
    assert_eq!(
        cover(EXAMPLE_QUEST).as_deref(),
        Some("https://m.media-amazon.com/images/I/example-quest-bg2.jpg")
    );
    // A plain-http background is passed over for the icon.
    assert_eq!(
        cover("amzn1.adg.product.11111111-2222-4333-8444-555555555555").as_deref(),
        Some("https://images-na.ssl-images-amazon.com/images/I/untitled-icon.png")
    );
    // Somewhere else entirely: no cover.
    assert_eq!(
        cover("amzn1.adg.product.99999999-8888-4777-8666-555555555555"),
        None
    );
    assert!(!is_amazon_image_url(
        "https://media-amazon.com.evil.example/x.jpg"
    ));
    assert!(!is_amazon_image_url("https://notmedia-amazon.com/x.jpg"));
}

#[test]
fn an_empty_library_is_no_error() {
    assert!(parse_library("[]").unwrap().is_empty());
    assert!(parse_library("").is_err());
}

#[test]
fn reads_the_download_size() {
    let info = parse_install_info(include_str!("fixtures/install_info.json")).unwrap();
    assert_eq!(info.download_size, Some(4_194_304_000));
    assert_eq!(info.disk_size, Some(4_194_304_000));
    assert!(parse_install_info("ERROR [CLI]: Couldn't find").is_err());
}

#[test]
fn installs_into_a_folder_inside_the_base_folder() {
    let args = install_args(EXAMPLE_QUEST, Path::new("/home/user/Games/Amazon"));
    assert_eq!(
        args,
        [
            "install",
            "--base-path",
            "/home/user/Games/Amazon",
            EXAMPLE_QUEST
        ]
        .map(OsString::from)
    );
}

#[test]
fn reads_the_install_progress() {
    // Nile's progress lines read like Legendary's and gogdl's.
    let log = include_str!("fixtures/install.log");
    let last = log.lines().filter_map(parse_progress).next_back();
    assert_eq!(last, Some(42.17));
    assert_eq!(error_message(log), None);
}

#[test]
fn a_full_disk_is_an_error_even_though_nile_exits_with_0() {
    let log = include_str!("fixtures/install_no_space.log");
    assert_eq!(
        error_message(log).as_deref(),
        Some("Not enough space available")
    );
}

#[test]
fn finds_the_install_folder() {
    assert_eq!(
        parse_installed(INSTALLED, EXAMPLE_QUEST).unwrap(),
        Some(PathBuf::from("/home/user/Games/Amazon/Example Quest"))
    );
    assert_eq!(
        parse_installed(INSTALLED, "amzn1.adg.product.other").unwrap(),
        None
    );
    assert_eq!(parse_installed("[]", EXAMPLE_QUEST).unwrap(), None);
    assert!(parse_installed("{", EXAMPLE_QUEST).is_err());
}

#[test]
fn the_exe_is_the_one_fuel_json_names() {
    let fuel = parse_fuel(FUEL).unwrap();
    assert_eq!(
        fuel_exe(&fuel, Path::new("/home/user/Games/Amazon/Example Quest")),
        Some(PathBuf::from(
            "/home/user/Games/Amazon/Example Quest/Binaries/Win64/ExampleQuest.exe"
        ))
    );
    assert_eq!(
        fuel_exe(&serde_json::json!({"Main": {}}), Path::new("/g")),
        None
    );
    assert_eq!(
        fuel_exe(
            &serde_json::json!({"Main": {"Command": " "}}),
            Path::new("/g")
        ),
        None
    );
}

#[test]
fn syncs_in_full() {
    assert_eq!(library_sync_args(), ["library", "sync", "--force"]);
}

#[test]
fn reads_fuel_json_written_as_json5() {
    let fuel = parse_fuel(FUEL).unwrap();
    assert_eq!(fuel["SchemaVersion"], "2");
    assert_eq!(
        fuel["Main"]["Command"].as_str(),
        Some(r"Binaries\Win64\ExampleQuest.exe")
    );
    assert!(parse_fuel("{").is_err());
}

#[test]
fn relaxed_json_keeps_strings_as_they_are() {
    let raw =
        "\u{feff}{ \"a\": \"x // y, }\", /* c */ \"b\": [1, 2,], // end\n \"c\": \"q\\\"/*\", }";
    let value: Value = serde_json::from_str(&relaxed_json(raw)).unwrap();
    assert_eq!(value["a"], "x // y, }");
    assert_eq!(value["b"], serde_json::json!([1, 2]));
    assert_eq!(value["c"], "q\"/*");
}

#[test]
fn launches_with_the_sdk_environment_in_the_working_folder() {
    let fuel = parse_fuel(FUEL).unwrap();
    let launch = parse_launch(LAUNCH, Some(&fuel)).unwrap();
    assert_eq!(
        launch.exe_path,
        PathBuf::from("/home/user/Games/Amazon/Example Quest/Binaries/Win64/ExampleQuest.exe")
    );
    assert_eq!(launch.game_args, ["-EpicPortal", "--lang=en"]);
    assert!(launch.account_args.is_empty());
    assert_eq!(
        launch.working_dir,
        PathBuf::from("/home/user/Games/Amazon/Example Quest/Binaries/Win64")
    );
    let env = |key: &str| {
        launch
            .env
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    };
    assert_eq!(
        env("AMAZON_GAMES_FUEL_ENTITLEMENT_ID"),
        Some("7f3c2a10-8b4e-4d6f-a1c2-3e4f5a6b7c8d")
    );
    assert_eq!(
        env("AMAZON_GAMES_FUEL_PRODUCT_SKU"),
        Some("EXAMPLE_QUEST_SKU")
    );
    assert!(env("FUEL_DIR").is_some_and(|dir| dir.ends_with("Legacy")));
    assert_eq!(launch.env.len(), 5);
}

#[test]
fn without_an_override_the_game_runs_in_its_install_folder() {
    let launch = parse_launch(LAUNCH, None).unwrap();
    assert_eq!(
        launch.working_dir,
        PathBuf::from("/home/user/Games/Amazon/Example Quest")
    );
    let fuel = serde_json::json!({"Main": {"Command": "x.exe", "WorkingSubdirOverride": ""}});
    assert_eq!(
        parse_launch(LAUNCH, Some(&fuel)).unwrap().working_dir,
        PathBuf::from("/home/user/Games/Amazon/Example Quest")
    );
    assert!(parse_launch("ERROR [LAUNCHER]: Game is not installed", None).is_err());
    assert!(parse_launch(
        r#"{"command":{"instruction":" ","arguments":[]},"game_directory":"/g","env":{}}"#,
        None
    )
    .is_err());
}

#[test]
fn reads_the_post_install_steps() {
    let fuel = parse_fuel(FUEL).unwrap();
    let steps = post_install(&fuel);
    // The blank one is left out.
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].name, "vc_redist.x64.exe");
    assert_eq!(steps[0].path, "Prereqs/vc_redist.x64.exe");
    assert_eq!(steps[0].args, "/install /quiet /norestart");
    assert!(!steps[0].installed);
    assert_eq!(steps[1].path, "Prereqs/DirectX/DXSETUP.exe");
    // Split again at launch exactly as fuel.json lists them.
    assert_eq!(
        crate::models::split_launch_args(&steps[1].args).unwrap(),
        ["/silent", r"/path:C:\Program Files\Example"]
    );
    assert!(post_install(&serde_json::json!({"Main": {}})).is_empty());
}
