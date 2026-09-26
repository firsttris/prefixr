use super::*;

#[test]
fn toggle_labels_follow_visibility_and_locale() {
    assert_eq!(toggle_label(true, Locale::De), "Fenster verstecken");
    assert_eq!(toggle_label(false, Locale::De), "Fenster anzeigen");
    assert_eq!(toggle_label(true, Locale::En), "Hide window");
    assert_eq!(toggle_label(false, Locale::En), "Show window");
}

#[test]
fn tray_labels_are_localized() {
    assert_eq!(kill_game_label("Cyberpunk 2077", Locale::De), "„Cyberpunk 2077“ beenden (erzwingen)");
    assert_eq!(kill_game_label("Cyberpunk 2077", Locale::En), "Quit “Cyberpunk 2077” (force)");
    assert_eq!(quit_label(Locale::De), "Beenden");
    assert_eq!(quit_label(Locale::En), "Quit");
}

#[test]
fn kill_menu_ids_parse_only_valid_uuid_entries() {
    let id = Uuid::new_v4();
    assert_eq!(parse_kill_game_id(&format!("{KILL_PREFIX}{id}")), Some(id));
    assert_eq!(parse_kill_game_id("quit"), None);
    assert_eq!(parse_kill_game_id("kill-game:not-a-uuid"), None);
}

#[test]
fn quit_dialog_copy_varies_by_count_and_locale() {
    let (message_de, title_de, confirm_de, cancel_de) = quit_dialog_copy(1, Locale::De);
    assert!(message_de.contains("Ein Spiel läuft noch oder wird gerade gestartet."));
    assert_eq!(title_de, "Prefixr beenden?");
    assert_eq!(confirm_de, "Spiele und Prefixr beenden");
    assert_eq!(cancel_de, "Abbrechen");

    let (message_en, title_en, confirm_en, cancel_en) = quit_dialog_copy(3, Locale::En);
    assert!(message_en.contains("3 games are still running or starting."));
    assert_eq!(title_en, "Quit Prefixr?");
    assert_eq!(confirm_en, "Quit games and Prefixr");
    assert_eq!(cancel_en, "Cancel");
}

#[test]
fn gdbus_host_detection_parses_output_and_falls_back_open() {
    assert!(tray_host_available_from_gdbus(true, b"(true,)\n"));
    assert!(!tray_host_available_from_gdbus(true, b"(false,)\n"));
    assert!(tray_host_available_from_gdbus(false, b""));
}

#[test]
fn running_games_are_sorted_by_name() {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();

    let sorted = sorted_running_games(vec![
        (a, "Zelda".to_string()),
        (b, "Anno 1800".to_string()),
        (c, "Baldur's Gate 3".to_string()),
    ]);

    assert_eq!(
        sorted.into_iter().map(|(_, name)| name).collect::<Vec<_>>(),
        vec!["Anno 1800", "Baldur's Gate 3", "Zelda"]
    );
}
