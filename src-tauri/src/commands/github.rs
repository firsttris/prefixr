use crate::error::AppError;
use tauri::{AppHandle, State};

use crate::config::{save_config, ConfigState};
use crate::models::GitHubConfig;

fn sanitized_token(token: Option<&str>) -> Option<String> {
    token.map(str::trim)
        .filter(|token| !token.is_empty())
        .map(str::to_string)
}

/// Reads the configured GitHub token, if any, without holding the config
/// lock across an `.await` point (a `std::sync::MutexGuard` isn't `Send`).
/// `None` means "no token configured" — callers should fall back to an
/// unauthenticated request rather than erroring, since GitHub's API accepts
/// those too (just at a much lower rate limit).
pub(crate) fn read_token(state: &State<ConfigState>) -> Result<Option<String>, String> {
    let config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    Ok(sanitized_token(config.github.token.as_deref()))
}

#[tauri::command]
pub fn get_github_config(state: State<ConfigState>) -> Result<GitHubConfig, AppError> {
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
) -> Result<GitHubConfig, AppError> {
    let mut app_config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    app_config.github = config;
    save_config(&app, &app_config)?;
    Ok(app_config.github.clone())
}

#[cfg(test)]
mod tests {
    use super::sanitized_token;

    #[test]
    fn trims_configured_tokens() {
        assert_eq!(sanitized_token(Some("  secret-token  ")), Some("secret-token".to_string()));
    }

    #[test]
    fn empty_or_missing_tokens_become_none() {
        assert_eq!(sanitized_token(Some("   \t\n  ")), None);
        assert_eq!(sanitized_token(None), None);
    }
}
