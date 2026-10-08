use super::*;

#[test]
fn locale_from_code_prefers_german_prefixes() {
    assert_eq!(Locale::from_code("de_DE"), Locale::De);
    assert_eq!(Locale::from_code("DE-at"), Locale::De);
    assert_eq!(Locale::from_code("en_US"), Locale::En);
    assert_eq!(Locale::from_code("fr_FR"), Locale::En);
}

#[test]
fn locale_from_env_uses_first_non_empty_locale_var() {
    let env = crate::env::fake(&[
        ("LC_ALL", ""),
        ("LC_MESSAGES", "en_GB.UTF-8"),
        ("LANG", "de_DE.UTF-8"),
    ]);
    assert_eq!(Locale::from_env(&env), Locale::En);

    let env = crate::env::fake(&[("LC_ALL", "de_DE.UTF-8"), ("LANG", "en_US.UTF-8")]);
    assert_eq!(Locale::from_env(&env), Locale::De);
}

#[test]
fn locale_state_round_trips_values() {
    let state = LocaleState::default();
    state.set(Locale::En);
    assert_eq!(state.get(), Locale::En);
    state.set(Locale::De);
    assert_eq!(state.get(), Locale::De);
}

fn messages_json(lang: &str) -> serde_json::Map<String, serde_json::Value> {
    let file = format!("{}/../messages/{lang}.json", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap()
}

fn placeholders(text: &str) -> Vec<String> {
    let mut names: Vec<String> = text
        .split('{')
        .skip(1)
        .filter_map(|part| part.split_once('}').map(|(name, _)| name.to_string()))
        .filter(|name| name.chars().all(|c| c.is_alphanumeric() || c == '_'))
        .collect();
    names.sort();
    names.dedup();
    names
}

#[test]
fn text_fills_placeholders_and_falls_back_to_the_key() {
    assert_eq!(
        text(Locale::De, "native_tray_killGame", &[("name", "Doom")]),
        "„Doom“ beenden (erzwingen)"
    );
    assert_eq!(text(Locale::En, "native_tray_quit", &[]), "Quit");
    assert_eq!(text(Locale::En, "no_such_key", &[]), "no_such_key");
}

/// Every key Rust renders itself has a text in both languages.
#[test]
fn every_key_rust_uses_has_a_text() {
    let mut used = Vec::new();
    let mut stack = vec![std::path::PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src"
    ))];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let src = std::fs::read_to_string(&path).unwrap();
                for part in src.split("text(").skip(1) {
                    if let Some((_, rest)) = part.split_once(", \"") {
                        let key = rest.split('"').next().unwrap();
                        if key.starts_with("native_") || key.starts_with("common_") {
                            used.push(key.to_string());
                        }
                    }
                }
            }
        }
    }
    assert!(used.len() >= 10, "found {used:?}");
    for lang in ["de", "en"] {
        let texts = messages_json(lang);
        let missing: Vec<&String> = used.iter().filter(|k| !texts.contains_key(*k)).collect();
        assert!(missing.is_empty(), "{lang}: no text for {missing:?}");
    }
}

/// Every `AppError` variant has a `backendErrors_<code>` text whose placeholders are its fields.
#[test]
fn every_app_error_has_a_text() {
    let src = include_str!("../error.rs");
    let body = src
        .split("pub enum AppError {")
        .nth(1)
        .unwrap()
        .split("\n}")
        .next()
        .unwrap();
    let code_only: String = body
        .lines()
        .map(str::trim)
        .filter(|l| !l.starts_with("//"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut variants = Vec::new();
    let (mut depth, mut current) = (0, String::new());
    for c in code_only.chars() {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            ',' if depth == 0 => {
                variants.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(c);
    }
    let mut checked = 0;
    for line in variants.iter().map(|v| v.trim()).filter(|v| !v.is_empty()) {
        let name: String = line.chars().take_while(|c| c.is_alphanumeric()).collect();
        // serde's snake_case: an underscore before every inner capital.
        let mut code = String::new();
        for (i, c) in name.chars().enumerate() {
            if c.is_uppercase() && i > 0 {
                code.push('_');
            }
            code.extend(c.to_lowercase());
        }
        let mut fields: Vec<String> = line
            .split_once('{')
            .map(|(_, f)| {
                f.trim_end_matches(['}', ',', ' '])
                    .split(',')
                    .filter_map(|f| f.split_once(':'))
                    .map(|(n, _)| n.trim().to_string())
                    .collect()
            })
            .unwrap_or_default();
        fields.sort();
        for lang in ["de", "en"] {
            let texts = messages_json(lang);
            let key = format!("backendErrors_{code}");
            let text = texts
                .get(&key)
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| panic!("{lang}: no {key}"));
            assert_eq!(placeholders(text), fields, "{lang}: placeholders of {key}");
        }
        checked += 1;
    }
    assert!(checked >= 20, "checked {checked} variants");
}

#[test]
fn app_errors_render_in_the_chosen_language() {
    use crate::error::AppError;
    let error = AppError::PrefixInUse {
        game_name: "Doom".into(),
    };
    assert_eq!(
        error.localized(Locale::De),
        "„Doom“ läuft noch in diesem Prefix. Beende das Spiel zuerst."
    );
    assert_eq!(
        error.localized(Locale::En),
        "“Doom” is still running in this prefix. Close the game first."
    );
    assert_eq!(
        AppError::from("plain".to_string()).localized(Locale::En),
        "plain"
    );
}
