use std::sync::Mutex;

/// The UI language for the handful of things Rust itself renders directly
/// (the tray menu, and a few native dialogs shown before the frontend has
/// loaded or without any window at all) — everything else is translated by
/// the frontend from the structured `AppError` a command returns (see
/// `error.rs`). Kept in sync with the frontend's own choice via
/// `set_ui_locale`, called once on startup and again on every switch (see
/// `+layout.svelte`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    De,
    En,
}

impl Locale {
    pub fn from_code(s: &str) -> Self {
        if s.to_ascii_lowercase().starts_with("de") {
            Locale::De
        } else {
            Locale::En
        }
    }

    /// Best-effort guess for the dialogs that can appear before the
    /// frontend — and its own locale detection/choice — exists at all,
    /// mirroring the frontend's own `navigator.language` fallback.
    pub fn from_env() -> Self {
        for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(val) = std::env::var(var) {
                if !val.is_empty() {
                    return Self::from_code(&val);
                }
            }
        }
        Locale::De
    }
}

pub struct LocaleState(Mutex<Locale>);

impl Default for LocaleState {
    fn default() -> Self {
        Self(Mutex::new(Locale::from_env()))
    }
}

impl LocaleState {
    pub fn get(&self) -> Locale {
        self.0.lock().map(|guard| *guard).unwrap_or(Locale::De)
    }

    pub fn set(&self, locale: Locale) {
        if let Ok(mut guard) = self.0.lock() {
            *guard = locale;
        }
    }
}

#[cfg(test)]
mod tests {
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
}
