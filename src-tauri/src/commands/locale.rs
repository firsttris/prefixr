use tauri::{AppHandle, State};

use crate::locale::{Locale, LocaleState};
use crate::tray::rebuild_tray_menu;

/// Tells the backend which language the frontend is showing, so the tray
/// menu and the few native dialogs Rust renders itself (see `locale.rs`)
/// match it. Called once on startup and again on every switch — see the
/// `$effect` in `+layout.svelte`.
#[tauri::command]
pub fn set_ui_locale(app: AppHandle, state: State<LocaleState>, locale: String) {
    state.set(Locale::from_code(&locale));
    rebuild_tray_menu(&app);
}
