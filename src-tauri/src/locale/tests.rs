use super::*;
use std::sync::{Mutex as StdMutex, OnceLock};

fn env_lock() -> &'static StdMutex<()> {
    static LOCK: OnceLock<StdMutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| StdMutex::new(()))
}

#[test]
fn locale_from_code_prefers_german_prefixes() {
    assert_eq!(Locale::from_code("de_DE"), Locale::De);
    assert_eq!(Locale::from_code("DE-at"), Locale::De);
    assert_eq!(Locale::from_code("en_US"), Locale::En);
    assert_eq!(Locale::from_code("fr_FR"), Locale::En);
}

#[test]
fn locale_from_env_uses_first_non_empty_locale_var() {
    let _guard = env_lock().lock().unwrap();
    let old_lc_all = std::env::var_os("LC_ALL");
    let old_lc_messages = std::env::var_os("LC_MESSAGES");
    let old_lang = std::env::var_os("LANG");

    std::env::set_var("LC_ALL", "");
    std::env::set_var("LC_MESSAGES", "en_GB.UTF-8");
    std::env::set_var("LANG", "de_DE.UTF-8");
    assert_eq!(Locale::from_env(), Locale::En);

    std::env::set_var("LC_ALL", "de_DE.UTF-8");
    assert_eq!(Locale::from_env(), Locale::De);

    match old_lc_all {
        Some(value) => std::env::set_var("LC_ALL", value),
        None => std::env::remove_var("LC_ALL"),
    }
    match old_lc_messages {
        Some(value) => std::env::set_var("LC_MESSAGES", value),
        None => std::env::remove_var("LC_MESSAGES"),
    }
    match old_lang {
        Some(value) => std::env::set_var("LANG", value),
        None => std::env::remove_var("LANG"),
    }
}

#[test]
fn locale_state_round_trips_values() {
    let state = LocaleState::default();
    state.set(Locale::En);
    assert_eq!(state.get(), Locale::En);
    state.set(Locale::De);
    assert_eq!(state.get(), Locale::De);
}
