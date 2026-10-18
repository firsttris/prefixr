use super::*;
use clap::CommandFactory;

fn game(id: &str, name: &str) -> Game {
    serde_json::from_value(serde_json::json!({
        "id": id,
        "name": name,
        "exe_path": "/g/game.exe",
        "prefix_path": "/p",
        "runner_id": "r",
    }))
    .unwrap()
}

fn library() -> Vec<Game> {
    vec![
        game("6f2c1a3e-0000-4000-8000-000000000001", "DOOM Eternal"),
        game("6f2c1a3e-0000-4000-8000-000000000002", "DOOM The Dark Ages"),
        game("a1b2c3d4-0000-4000-8000-000000000003", "Quake"),
    ]
}

fn args(line: &str) -> Vec<String> {
    line.split_whitespace().map(str::to_string).collect()
}

fn input() -> GameInput {
    game_input(&game("6f2c1a3e-0000-4000-8000-000000000001", "x"))
}

#[test]
fn the_command_line_is_well_formed() {
    Cli::command().debug_assert();
}

#[test]
fn only_subcommands_and_help_make_it_a_command_line() {
    assert!(is_cli(&args("prefixr game list")));
    assert!(is_cli(&args("prefixr --help")));
    assert!(is_cli(&args("prefixr --version")));
    assert!(!is_cli(&args("prefixr")));
    assert!(!is_cli(&args("prefixr --launch 6f2c1a3e")));
    assert!(!is_cli(&args("prefixr --run 6f2c1a3e")));
    assert!(!is_cli(&args("prefixr --install /x/setup.exe")));
}

#[test]
fn finds_a_game_by_id_start_or_name() {
    let games = library();
    let find = |query| find_game(&games, query).map(|g| g.name.clone());

    assert_eq!(
        find("6f2c1a3e-0000-4000-8000-000000000002").unwrap(),
        "DOOM The Dark Ages"
    );
    assert_eq!(find("a1b2").unwrap(), "Quake");
    assert_eq!(find("quake").unwrap(), "Quake");
    assert_eq!(find("doom eternal").unwrap(), "DOOM Eternal");
    assert_eq!(find("dark").unwrap(), "DOOM The Dark Ages");
}

#[test]
fn an_ambiguous_or_unknown_game_is_an_error() {
    let games = library();
    let ambiguous = find_game(&games, "doom").unwrap_err();
    assert!(ambiguous.contains("DOOM Eternal (6f2c1a3e)"), "{ambiguous}");
    assert!(ambiguous.contains("DOOM The Dark Ages"), "{ambiguous}");
    // Shared by two ids.
    assert!(find_game(&games, "6f2c").is_err());
    assert!(find_game(&games, "Half-Life").is_err());
}

#[test]
fn names_a_game_after_its_exe_like_the_window() {
    let name = |exe: &str| name_from_exe(Path::new(exe));
    assert_eq!(name("/g/DOOMTheDarkAges.exe"), "DOOM The Dark Ages");
    assert_eq!(name("/g/BaldursGate3.EXE"), "Baldurs Gate3");
    assert_eq!(name("/g/Quake_x64_steam.exe"), "Quake x64 steam");
    assert_eq!(name("/g/f4se_loader.exe"), "f4se loader");
    assert_eq!(name("/g/Sam4"), "Sam4");
}

#[test]
fn parses_env_and_proton_switches() {
    assert_eq!(
        parse_env("DXVK_FRAME_RATE=58").unwrap(),
        ("DXVK_FRAME_RATE".to_string(), "58".to_string())
    );
    assert_eq!(
        parse_env("WINEDLLOVERRIDES=d3d9=n,b").unwrap().1,
        "d3d9=n,b"
    );
    assert_eq!(parse_env("EMPTY=").unwrap().1, "");
    assert!(parse_env("NOVALUE").is_err());
    assert!(parse_env("=1").is_err());

    assert_eq!(
        parse_proton("proton_use_wined3d=On").unwrap(),
        ("PROTON_USE_WINED3D".to_string(), Toggle::On)
    );
    assert!(parse_proton("PROTON_X=maybe").is_err());
}

#[test]
fn settings_change_only_what_is_given() {
    let mut input = input();
    input.env_vars.insert("KEEP".into(), "1".into());
    input.env_vars.insert("DROP".into(), "1".into());
    input.umu_id = Some("umu-1".into());
    input.umu_store = Some("gog".into());
    input
        .overrides
        .proton
        .insert("PROTON_ENABLE_HDR".into(), true);
    input.overrides.performance.gamemode_enabled = Some(true);

    apply_settings(
        GameSettings {
            env: vec![("DXVK_FRAME_RATE".into(), "58".into())],
            unset_env: vec!["DROP".into()],
            proton: vec![
                ("PROTON_USE_WINED3D".into(), Toggle::On),
                ("PROTON_ENABLE_HDR".into(), Toggle::Default),
            ],
            mangohud: Some(Toggle::Off),
            gamemode: Some(Toggle::Default),
            ..Default::default()
        },
        &mut input,
    );

    assert_eq!(input.env_vars.get("KEEP").map(String::as_str), Some("1"));
    assert_eq!(
        input.env_vars.get("DXVK_FRAME_RATE").map(String::as_str),
        Some("58")
    );
    assert!(!input.env_vars.contains_key("DROP"));
    assert_eq!(input.umu_id.as_deref(), Some("umu-1"));
    assert_eq!(input.umu_store.as_deref(), Some("gog"));
    assert_eq!(
        input.overrides.proton.get("PROTON_USE_WINED3D"),
        Some(&true)
    );
    assert!(!input.overrides.proton.contains_key("PROTON_ENABLE_HDR"));
    assert_eq!(input.overrides.overlay.enabled, Some(false));
    assert_eq!(input.overrides.performance.gamemode_enabled, None);
}

#[test]
fn sets_and_clears_notes() {
    let mut input = input();
    apply_settings(
        GameSettings {
            notes: Some("  Login: RELOADED / 1\n".into()),
            ..Default::default()
        },
        &mut input,
    );
    assert_eq!(input.notes, "Login: RELOADED / 1");
    apply_settings(GameSettings::default(), &mut input);
    assert_eq!(input.notes, "Login: RELOADED / 1", "kept when not given");
    apply_settings(
        GameSettings {
            notes: Some(String::new()),
            ..Default::default()
        },
        &mut input,
    );
    assert_eq!(input.notes, "");
}

#[test]
fn a_new_umu_id_drops_the_old_store() {
    let mut input = input();
    input.umu_id = Some("umu-1".into());
    input.umu_store = Some("gog".into());
    apply_settings(
        GameSettings {
            umu_id: Some("1091500".into()),
            ..Default::default()
        },
        &mut input,
    );
    assert_eq!(input.umu_id.as_deref(), Some("1091500"));
    assert_eq!(input.umu_store, None);
}

#[test]
fn parses_a_game_add() {
    let cli = Cli::try_parse_from([
        "prefixr",
        "game",
        "add",
        "--exe",
        "/g/Fallout4.exe",
        "--prefix",
        "/p",
        "--runner",
        "GE-Proton10-30",
        "--env",
        "DXVK_FRAME_RATE=58",
        "--args",
        "-dx11 -skipintro",
        "--umu-id",
        "377160",
        "--json",
    ])
    .unwrap();
    assert!(cli.json);
    let Command::Game(GameCommand::Add(add)) = cli.command else {
        panic!("not a game add");
    };
    assert_eq!(add.runner, "GE-Proton10-30");
    assert_eq!(add.settings.args.as_deref(), Some("-dx11 -skipintro"));
    assert_eq!(add.settings.umu_id.as_deref(), Some("377160"));
}

#[test]
fn passes_a_program_its_own_arguments() {
    let cli = Cli::try_parse_from(args(
        "prefixr prefix run /p --runner R regedit /S /tmp/fix.reg",
    ))
    .unwrap();
    let Command::Prefix(PrefixCommand::Run {
        program,
        args,
        runner,
        ..
    }) = cli.command
    else {
        panic!("not a prefix run");
    };
    assert_eq!(program, "regedit");
    assert_eq!(args, ["/S", "/tmp/fix.reg"]);
    assert_eq!(runner.runner.as_deref(), Some("R"));
}

#[test]
fn takes_a_list_of_artwork_kinds() {
    let cli = Cli::try_parse_from(args("prefixr game artwork quake --kinds cover,icon")).unwrap();
    let Command::Game(GameCommand::Artwork(artwork)) = cli.command else {
        panic!("not artwork");
    };
    assert_eq!(artwork.kinds, [ImageKind::Cover, ImageKind::Icon]);
}

#[test]
fn picks_the_runner_the_prefix_games_share() {
    let mut config: AppConfig = serde_json::from_value(serde_json::json!({
        "runners_dir": "/runners",
        "games": [],
    }))
    .unwrap();
    let none = || RunnerArg { runner: None };
    assert!(prefix_runner(&config, Path::new("/p"), none()).is_err());

    config.games = library();
    assert_eq!(
        prefix_runner(&config, Path::new("/p"), none()).unwrap(),
        "r"
    );
    config.games[0].runner_id = "other".into();
    assert!(prefix_runner(&config, Path::new("/p"), none()).is_err());
    assert_eq!(
        prefix_runner(
            &config,
            Path::new("/p"),
            RunnerArg {
                runner: Some("given".into())
            }
        )
        .unwrap(),
        "given"
    );
}

#[test]
fn only_commands_that_change_the_config_lock_it() {
    let changes = |line: &str| changes_config(&Cli::try_parse_from(args(line)).unwrap().command);
    assert!(changes(
        "prefixr game add --exe /g.exe --prefix /p --runner r"
    ));
    assert!(changes("prefixr game edit quake --name Q"));
    assert!(changes("prefixr game remove quake"));
    assert!(changes("prefixr game artwork quake"));
    assert!(changes("prefixr prefix add /p"));
    assert!(changes("prefixr prefix remove /p"));
    assert!(!changes("prefixr game list"));
    assert!(!changes("prefixr game launch quake"));
    assert!(!changes(
        "prefixr game install /g/setup.exe --prefix /p -- /VERYSILENT"
    ));
    assert!(!changes("prefixr prefix winetricks /p vcrun2022"));
    assert!(!changes("prefixr runner download proton-ge"));
    assert!(changes("prefixr library favorite quake"));
    assert!(changes("prefixr library hide quake --off"));
    assert!(changes("prefixr library test-status quake works --note ok"));
    assert!(!changes("prefixr library list --tested broken"));
    assert!(changes("prefixr category add Kinder"));
    assert!(changes("prefixr category rename Kinder Kids"));
    assert!(changes("prefixr category remove Kinder"));
    assert!(changes(
        "prefixr category assign Kinder quake epic:Fortnite"
    ));
    assert!(changes("prefixr category unassign Kinder quake"));
    assert!(!changes("prefixr library list --category Kinder"));
    assert!(!changes("prefixr category list"));
}

#[test]
fn library_and_category_commands_are_a_command_line() {
    assert!(is_cli(&args("prefixr library list")));
    assert!(is_cli(&args("prefixr category list")));
    assert!(
        Cli::try_parse_from(args("prefixr category assign Kinder")).is_err(),
        "needs a game"
    );
    assert!(
        Cli::try_parse_from(args("prefixr library favorite")).is_err(),
        "needs a game"
    );
    let cli =
        Cli::try_parse_from(args("prefixr library list --favorites --hidden --json")).unwrap();
    assert!(cli.json);
    assert!(matches!(
        cli.command,
        Command::Library(LibraryCommand::List {
            favorites: true,
            hidden: true,
            category: None,
            tested: None,
            compatibility: false,
        })
    ));
}

#[test]
fn lines_up_a_table() {
    let text = table(
        &["ID", "NAME", "RUNNER"],
        &[
            vec!["1".into(), "Quake".into(), "GE".into()],
            vec!["22".into(), "DOOM Eternal".into(), "GE-Proton10".into()],
        ],
    );
    assert_eq!(
        text,
        "ID  NAME          RUNNER\n\
         1   Quake         GE\n\
         22  DOOM Eternal  GE-Proton10"
    );
}

#[test]
fn finds_the_latest_log() {
    let dir = crate::test_util::TestDir::new("cli-logs");
    assert_eq!(latest_log(&dir), None);
    for name in ["1700000000.txt", "1700000000123.txt", "999.txt", "notes.md"] {
        fs::write(dir.join(name), "").unwrap();
    }
    assert_eq!(latest_log(&dir), Some(dir.join("1700000000123.txt")));
}

#[test]
fn json_before_the_subcommand_is_the_command_line_too() {
    assert!(is_cli(&args("prefixr --json game list")));
    assert!(is_cli(&args("prefixr --json --json library list")));
    assert!(is_cli(&args("prefixr game list --json")));
    // Alone, or before the window's own options, it opens the window as before.
    assert!(!is_cli(&args("prefixr --json")));
    assert!(!is_cli(&args("prefixr --json --launch 1234")));
    assert!(
        Cli::try_parse_from(args("prefixr --json game list"))
            .unwrap()
            .json
    );
}
