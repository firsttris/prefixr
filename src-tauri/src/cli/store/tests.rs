use super::*;
use std::path::PathBuf;

fn runner(name: &str, kind: RunnerKind) -> Runner {
    Runner {
        id: name.to_string(),
        name: name.to_string(),
        path: PathBuf::from(format!("/runners/{name}")),
        kind,
    }
}

#[test]
fn runner_names_compare_their_numbers_as_numbers() {
    assert_eq!(
        natural_cmp("GE-Proton10-4", "GE-Proton9-27"),
        Ordering::Greater
    );
    assert_eq!(
        natural_cmp("GE-Proton10-30", "GE-Proton10-4"),
        Ordering::Greater
    );
    assert_eq!(natural_cmp("wine-9.0", "wine-9.0"), Ordering::Equal);
    assert_eq!(natural_cmp("proton-08", "Proton-8"), Ordering::Equal);
    assert_eq!(natural_cmp("GE-Proton10", "GE-Proton10-1"), Ordering::Less);
}

#[test]
fn store_games_get_the_newest_proton() {
    let runners = [
        runner("GE-Proton9-27", RunnerKind::Proton),
        runner("wine-11.0", RunnerKind::Wine),
        runner("GE-Proton10-4", RunnerKind::Proton),
    ];
    assert_eq!(default_runner(&runners).unwrap().id, "GE-Proton10-4");
    let only_wine = [
        runner("wine-10.0", RunnerKind::Wine),
        runner("wine-11.0", RunnerKind::Wine),
    ];
    assert_eq!(default_runner(&only_wine).unwrap().id, "wine-11.0");
    assert!(default_runner(&[]).is_none());
}

fn entry(key: &str, name: &str, source: &str, game_id: Option<&str>) -> LibraryEntry {
    LibraryEntry {
        key: key.to_string(),
        name: name.to_string(),
        source: source.to_string(),
        installed: game_id.is_some(),
        game_id: game_id.map(str::to_string),
        categories: Vec::new(),
        favorite: false,
        hidden: false,
        playtime_secs: 0,
        last_played: None,
        compatibility: None,
    }
}

#[test]
fn only_store_games_are_found_for_installing() {
    let entries = [
        entry(
            "3f1c2a52-0000-4000-8000-000000000000",
            "Doom",
            "local",
            Some("3f1c2a52-0000-4000-8000-000000000000"),
        ),
        entry("gog:1", "Doom II", "gog", None),
    ];
    // "doom" would match both; only the store's one counts here.
    assert_eq!(find_store_game(&entries, "doom").unwrap().key, "gog:1");
    assert!(find_store_game(&entries, "3f1c2a52").is_err());
    assert_eq!(split_key("gog:1"), Some((StoreKind::Gog, "1".to_string())));
    assert_eq!(split_key("nope"), None);
}

#[test]
fn no_store_command_holds_the_config_lock_from_the_start() {
    use clap::Parser;
    let parse = |line: &str| match super::super::Cli::try_parse_from(line.split(' '))
        .unwrap()
        .command
    {
        super::super::Command::Store(command) => command,
        other => panic!("not a store command: {other:?}"),
    };
    assert!(!changes_config(&parse("prefixr store uninstall doom")));
    assert!(!changes_config(&parse(
        "prefixr store install doom --runner GE-Proton10-4"
    )));
    assert!(!changes_config(&parse("prefixr store connect epic abc")));
    assert!(!changes_config(&parse("prefixr store sync gog")));
    assert!(!changes_config(&parse("prefixr store list")));
}

#[test]
fn progress_shows_every_percent_in_a_terminal_and_every_ten_otherwise() {
    assert_eq!(
        progress_line("Doom", None, 0.0, false).as_deref(),
        Some("Doom: 0 %\n")
    );
    assert_eq!(progress_line("Doom", Some(0), 9.9, false), None);
    assert_eq!(
        progress_line("Doom", Some(0), 10.2, false).as_deref(),
        Some("Doom: 10 %\n")
    );
    assert_eq!(progress_line("Doom", Some(42), 42.7, true), None);
    assert_eq!(
        progress_line("Doom", Some(42), 43.0, true).as_deref(),
        Some("\rDoom: 43 %   ")
    );
    assert_eq!(
        progress_line("Doom", None, 0.0, true).as_deref(),
        Some("\rDoom: 0 %   ")
    );
}
