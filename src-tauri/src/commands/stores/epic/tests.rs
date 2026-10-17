use super::*;

const LIST: &str = include_str!("fixtures/list.json");
const LIST_INSTALLED: &str = include_str!("fixtures/list_installed.json");
const LAUNCH: &str = include_str!("fixtures/launch.json");
const INFO: &str = include_str!("fixtures/info.json");
const INSTALL_LOG: &str = include_str!("fixtures/install.log");
const INSTALL_NO_SPACE: &str = include_str!("fixtures/install_no_space.log");
const NOT_LOGGED_IN: &str = include_str!("fixtures/list_not_logged_in.stderr");

#[test]
fn reads_the_account_from_status() {
    assert_eq!(
        account_name(include_str!("fixtures/status_logged_in.json")).unwrap(),
        Some("PrefixrTester".to_string())
    );
    // The real output of a Legendary nobody has logged into.
    assert_eq!(
        account_name(include_str!("fixtures/status_logged_out.json")).unwrap(),
        None
    );
    assert!(account_name("not json").is_err());
}

#[test]
fn takes_the_code_from_whatever_was_pasted() {
    let code = "0123456789abcdef0123456789abcdef";
    assert_eq!(auth_code(code).as_deref(), Some(code));
    assert_eq!(auth_code(&format!("  \"{code}\"\n")).as_deref(), Some(code));
    let page = format!(
        r#"{{"warning":"Do not share this code with any 3rd party service.","redirectUrl":"https://localhost/launcher/authorized?code={code}","authorizationCode":"{code}","exchangeCode":null,"sid":null}}"#
    );
    assert_eq!(auth_code(&page).as_deref(), Some(code));
}

#[test]
fn refuses_what_isnt_a_code() {
    assert_eq!(auth_code(""), None);
    assert_eq!(auth_code("   "), None);
    assert_eq!(auth_code("not a code"), None);
    assert_eq!(auth_code(r#"{"authorizationCode":null}"#), None);
    assert_eq!(auth_code("{broken json"), None);
    assert_eq!(auth_code("abc; rm -rf ~"), None);
}

#[test]
fn lists_the_games_sorted_with_their_covers() {
    let games = parse_library(LIST).unwrap();
    let names: Vec<&str> = games.iter().map(|g| g.app_name.as_str()).collect();
    // Sorted by title regardless of case; the DLC isn't a game of its own.
    assert_eq!(names, ["Ashen", "Lantern", "IronVale"]);
    assert_eq!(games[0].title, "ashen road");
    assert_eq!(games[1].title, "Hollow Lantern");
}

#[test]
fn picks_the_portrait_cover_and_scales_it_down() {
    let games = parse_library(LIST).unwrap();
    let lantern = games.iter().find(|g| g.app_name == "Lantern").unwrap();
    assert_eq!(
        lantern.cover_url.as_deref(),
        Some("https://cdn1.epicgames.com/offer/hollow/EGS_HollowLantern_Tall_1200x1600-3d4e5f.jpg?h=600&w=400&resize=1")
    );
    // No tall box art: the thumbnail, keeping its own query.
    let ashen = games.iter().find(|g| g.app_name == "Ashen").unwrap();
    assert_eq!(
        ashen.cover_url.as_deref(),
        Some("https://cdn2.unrealengine.com/ashen/thumb.png?quality=high&h=600&w=400&resize=1")
    );
}

#[test]
fn takes_covers_only_from_epic_over_https() {
    let games = parse_library(LIST).unwrap();
    let iron = games.iter().find(|g| g.app_name == "IronVale").unwrap();
    // Its tall art is plain http, its thumbnail on some other host.
    assert_eq!(iron.cover_url, None);
    assert!(is_epic_image_url("https://cdn1.epicgames.com/a.jpg"));
    assert!(is_epic_image_url("https://cdn2.unrealengine.com/a.png"));
    assert!(!is_epic_image_url("http://cdn1.epicgames.com/a.jpg"));
    assert!(!is_epic_image_url(
        "https://epicgames.com.evil.example/a.jpg"
    ));
    assert!(!is_epic_image_url("https://notepicgames.com/a.jpg"));
    assert!(!is_epic_image_url("not a url"));
}

#[test]
fn an_empty_or_broken_list_is_handled() {
    assert_eq!(parse_library("[]").unwrap(), Vec::new());
    assert!(parse_library("{}").is_err());
}

#[test]
fn reads_the_installed_game_and_its_prerequisites() {
    let lantern = parse_installed(LIST_INSTALLED, "Lantern").unwrap().unwrap();
    assert_eq!(
        lantern.exe_path(),
        PathBuf::from("/home/user/Games/Epic/HollowLantern/Binaries/Win64/Lantern.exe")
    );
    assert_eq!(
        lantern.prerequisites,
        Some(StorePrerequisites {
            name: "UE4 Prerequisites".to_string(),
            path: "Engine/Extras/Redist/en-us/UE4PrereqSetup_x64.exe".to_string(),
            args: "/quiet /norestart".to_string(),
            installed: false,
        })
    );
    // An empty prerequisite entry means none.
    let ashen = parse_installed(LIST_INSTALLED, "Ashen").unwrap().unwrap();
    assert_eq!(ashen.prerequisites, None);
    assert_eq!(
        ashen.exe_path(),
        PathBuf::from("/home/user/Games/Epic/Ashen Road/AshenRoad.exe")
    );
    assert_eq!(parse_installed(LIST_INSTALLED, "Nope").unwrap(), None);
    assert_eq!(parse_installed("[]", "Lantern").unwrap(), None);
}

#[test]
fn orders_the_launch_like_legendary() {
    let launch = parse_launch(LAUNCH).unwrap();
    assert_eq!(
        launch.exe_path,
        PathBuf::from("/home/user/Games/Epic/HollowLantern/Binaries/Win64/Lantern.exe")
    );
    assert_eq!(launch.game_args, ["-SaveToUserDir"]);
    assert_eq!(launch.account_args.first().unwrap(), "-AUTH_LOGIN=unused");
    assert!(launch
        .account_args
        .contains(&"-epicapp=Lantern".to_string()));
    assert_eq!(
        launch.working_dir,
        PathBuf::from("/home/user/Games/Epic/HollowLantern/Binaries/Win64")
    );
    // The prefix stays Prefixr's.
    assert_eq!(launch.env, [("DXVK_HUD".to_string(), "0".to_string())]);
}

#[test]
fn a_launch_without_working_directory_starts_in_the_game_folder() {
    let json = r#"{"game_executable":"Game.exe","game_directory":"/g"}"#;
    let launch = parse_launch(json).unwrap();
    assert_eq!(launch.working_dir, PathBuf::from("/g"));
    assert_eq!(launch.exe_path, PathBuf::from("/g/Game.exe"));
    assert!(launch.game_args.is_empty() && launch.account_args.is_empty());
    assert!(parse_launch(r#"{"game_executable":"","game_directory":"/g"}"#).is_err());
}

#[test]
fn launch_args_skip_the_login_only_offline() {
    assert_eq!(
        launch_args("Lantern", false),
        ["launch", "Lantern", "--json", "--skip-version-check"]
    );
    assert_eq!(launch_args("Lantern", true).last().unwrap(), "--offline");
}

#[test]
fn reads_the_sizes() {
    assert_eq!(
        parse_install_info(INFO).unwrap(),
        StoreInstallInfo {
            download_size: Some(19_756_849_152),
            disk_size: Some(24_117_248_000),
        }
    );
    // An install without a manifest (offline) has no sizes, which is fine.
    assert_eq!(
        parse_install_info(r#"{"game":{},"install":{},"manifest":{}}"#).unwrap(),
        StoreInstallInfo::default()
    );
}

#[test]
fn install_answers_every_question_itself() {
    let args = install_args("Lantern", Path::new("/home/user/Games/Epic"));
    let args: Vec<&str> = args.iter().map(|a| a.to_str().unwrap()).collect();
    assert_eq!(
        args,
        [
            "-y",
            "install",
            "Lantern",
            "--base-path",
            "/home/user/Games/Epic",
            "--platform",
            "Windows",
            "--skip-sdl",
            "--with-dlcs",
        ]
    );
}

#[test]
fn follows_the_progress() {
    let progress: Vec<f64> = INSTALL_LOG.lines().filter_map(parse_progress).collect();
    assert_eq!(progress, [0.0, 45.37, 100.0]);
    assert_eq!(
        parse_progress("[DLManager] INFO:  - Downloaded: 1.00 MiB"),
        None
    );
    assert_eq!(parse_progress("= Progress: nan% (0/0)"), None);
    assert_eq!(parse_progress("= Progress: 140.00%"), Some(100.0));
}

#[test]
fn finds_the_error_legendary_logged() {
    assert_eq!(
        error_message(INSTALL_NO_SPACE).as_deref(),
        Some("Installation cannot proceed, exiting.")
    );
    assert_eq!(
        error_message("[cli] INFO: x\n[cli] ERROR: Game Foo is not currently installed!\n")
            .as_deref(),
        Some("Game Foo is not currently installed!")
    );
    // Warnings aren't errors.
    assert_eq!(error_message(INSTALL_LOG), None);
}

#[test]
fn finds_the_error_of_a_crash() {
    let message = error_message(NOT_LOGGED_IN).unwrap();
    assert_eq!(message, "No saved credentials");
    assert!(needs_login(&message));
    assert!(needs_login("Login failed, cannot continue!"));
    assert!(!needs_login("Installation cannot proceed, exiting."));
    assert_eq!(error_message("just some text: not an exception"), None);
}
