use super::*;

fn entry(title: &str, umu_id: &str, acronym: Option<&str>, store: &str) -> DatabaseEntry {
    DatabaseEntry {
        title: Some(title.to_string()),
        umu_id: umu_id.to_string(),
        acronym: acronym.map(str::to_string),
        store: Some(store.to_string()),
    }
}

fn entries() -> Vec<DatabaseEntry> {
    vec![
        entry("Borderlands 3", "umu-397540", Some("bl3"), "egs"),
        entry("Borderlands 2", "umu-49520", Some("bl2"), "egs"),
        entry("Tiny Tina's Assault on Dragon Keep", "umu-1712840", None, "egs"),
        entry("Cyberpunk 2077", "umu-1091500", None, "egs"),
        entry("Cyberpunk 2077", "umu-1091500", None, "gog"),
        entry("Dark and Darker", "umu-2016590", Some("dad"), "none"),
    ]
}

fn ids(matches: &[UmuMatch]) -> Vec<(&str, Option<&str>)> {
    matches
        .iter()
        .map(|m| (m.umu_id.as_str(), m.store.as_deref()))
        .collect()
}

#[test]
fn exact_title_ranks_first() {
    let matches = search_database(&entries(), "borderlands 3");
    assert_eq!(ids(&matches)[0], ("umu-397540", Some("egs")));
}

#[test]
fn matches_ignore_punctuation_and_case() {
    let matches = search_database(&entries(), "CYBERPUNK: 2077");
    assert_eq!(
        ids(&matches),
        vec![("umu-1091500", Some("egs")), ("umu-1091500", Some("gog"))]
    );
}

#[test]
fn matches_acronyms_and_scattered_words() {
    assert_eq!(ids(&search_database(&entries(), "BL2"))[0].0, "umu-49520");
    assert_eq!(
        ids(&search_database(&entries(), "tina dragon")),
        vec![("umu-1712840", Some("egs"))]
    );
}

#[test]
fn store_none_counts_as_no_store() {
    let matches = search_database(&entries(), "dark and darker");
    assert_eq!(ids(&matches), vec![("umu-2016590", None)]);
}

#[test]
fn merge_puts_steam_first_and_drops_repeats() {
    let steam = vec![SteamApp {
        app_id: "2016590".to_string(),
        name: "Dark and Darker".to_string(),
    }];
    let mut database = search_database(&entries(), "dark and darker");
    database.extend(search_database(&entries(), "cyberpunk 2077"));
    database.extend(search_database(&entries(), "cyberpunk 2077"));
    let merged = merge(steam, database);
    assert_eq!(merged[0].source, UmuMatchSource::Steam);
    assert_eq!(
        ids(&merged),
        vec![
            ("umu-2016590", None),
            ("umu-1091500", Some("egs")),
            ("umu-1091500", Some("gog")),
        ]
    );
}
