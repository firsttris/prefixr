use tauri::AppHandle;

use crate::error::AppError;
use crate::host_env::xdg_open;

/// The GitHub release of `version`, with what changed in it.
pub(crate) fn release_url(version: &str) -> String {
    format!("https://github.com/firsttris/prefixr/releases/tag/v{version}")
}

/// Opens the release notes of the running version in the browser.
#[tauri::command]
pub async fn open_release_notes(app: AppHandle) -> Result<(), AppError> {
    let version = app.package_info().version.to_string();
    xdg_open(release_url(&version))
        .await
        .map_err(AppError::from)
}

#[cfg(test)]
mod tests;
