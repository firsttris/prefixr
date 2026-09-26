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
mod tests;
