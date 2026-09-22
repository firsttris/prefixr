use tauri::{AppHandle, State};

use crate::config::{save_config, ConfigState};
use crate::models::GitHubConfig;

/// Reads the configured GitHub token, if any, without holding the config
/// lock across an `.await` point (a `std::sync::MutexGuard` isn't `Send`).
/// `None` means "no token configured" — callers should fall back to an
/// unauthenticated request rather than erroring, since GitHub's API accepts
/// those too (just at a much lower rate limit).
pub(crate) fn read_token(state: &State<ConfigState>) -> Result<Option<String>, String> {
    let config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    Ok(config
        .github
        .token
        .as_ref()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty()))
}

#[tauri::command]
pub fn get_github_config(state: State<ConfigState>) -> Result<GitHubConfig, String> {
    let config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    Ok(config.github.clone())
}

#[tauri::command]
pub fn save_github_config(
    app: AppHandle,
    state: State<ConfigState>,
    config: GitHubConfig,
) -> Result<GitHubConfig, String> {
    let mut app_config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    app_config.github = config;
    save_config(&app, &app_config)?;
    Ok(app_config.github.clone())
}
