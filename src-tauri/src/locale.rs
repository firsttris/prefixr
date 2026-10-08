use std::sync::{Mutex, OnceLock};

use crate::lock::LockExt;

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
        *self.0.locked()
    }

    pub fn set(&self, locale: Locale) {
        *self.0.locked() = locale;
    }
}

/// The UI's own texts (`messages/*.json`, the same files the frontend compiles with Paraglide),
/// built into the binary so the tray and the native dialogs need no copy of their own.
const MESSAGES_DE: &str = include_str!("../../messages/de.json");
const MESSAGES_EN: &str = include_str!("../../messages/en.json");

type Messages = serde_json::Map<String, serde_json::Value>;

fn messages(locale: Locale) -> &'static Messages {
    static DE: OnceLock<Messages> = OnceLock::new();
    static EN: OnceLock<Messages> = OnceLock::new();
    let (cell, source) = match locale {
        Locale::De => (&DE, MESSAGES_DE),
        Locale::En => (&EN, MESSAGES_EN),
    };
    cell.get_or_init(|| serde_json::from_str(source).expect("messages/*.json is valid JSON"))
}

/// A text by key with its `{placeholders}` filled in, e.g.
/// `text(locale, "native_tray_killGame", &[("name", "Doom")])`. Only plain messages (no plurals
/// or variants); an unknown key comes back as the key itself.
pub fn text(locale: Locale, key: &str, params: &[(&str, &str)]) -> String {
    let Some(template) = messages(locale).get(key).and_then(|v| v.as_str()) else {
        return key.to_string();
    };
    let mut out = template.to_string();
    for (name, value) in params {
        out = out.replace(&format!("{{{name}}}"), value);
    }
    out.replace("\\{", "{").replace("\\}", "}")
}

#[cfg(test)]
mod tests;
