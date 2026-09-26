use crate::error::AppError;
use std::fs;

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::commands::runners::find_runner;
use crate::config::{save_config, ConfigState};
use crate::models::{ProtonConfig, RunnerKind};

/// One on/off option a Proton build's `proton` script reads from the
/// environment. `env` is the variable to set; `aliases` are further names the
/// script accepts for the same `config` flag (e.g. `PROTON_USE_HDR` next to
/// `PROTON_ENABLE_HDR`), listed so the UI can tell they're the same switch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProtonOption {
    pub env: String,
    pub config: String,
    pub aliases: Vec<String>,
}

/// Variables that are already covered by a dedicated setting elsewhere, so
/// offering them again as a raw switch would only create two places for the
/// same thing.
const MANAGED_ELSEWHERE: &[&str] = &[
    // Has its own setting under "Bild", see `GraphicsConfig::vkbasalt`.
    "ENABLE_VKBASALT",
];

/// Collects every `check_environment("<ENV>", "<config>")` call in a `proton`
/// script. Proton reads nearly all of its user-facing switches through that
/// one helper, which treats any non-empty value other than `"0"` as on and
/// `"0"` as an explicit off — so every hit is a plain three-state switch
/// (unset / on / off). Parsing the script rather than keeping a fixed list
/// means each runner only offers what it actually understands: names come
/// and go between Proton versions (`PROTON_ENABLE_NVAPI` is gone in
/// GE-Proton 11, for example).
pub fn parse_proton_options(script: &str) -> Vec<ProtonOption> {
    const NEEDLE: &str = "check_environment(\"";
    let mut options: Vec<ProtonOption> = Vec::new();

    let mut rest = script;
    while let Some(start) = rest.find(NEEDLE) {
        rest = &rest[start + NEEDLE.len()..];
        let Some((env, config)) = parse_call_args(rest) else {
            continue;
        };
        if MANAGED_ELSEWHERE.contains(&env) {
            continue;
        }
        match options.iter_mut().find(|o| o.config == config) {
            Some(existing) => {
                if existing.env != env && !existing.aliases.iter().any(|a| a == env) {
                    existing.aliases.push(env.to_string());
                }
            }
            None => options.push(ProtonOption {
                env: env.to_string(),
                config: config.to_string(),
                aliases: Vec::new(),
            }),
        }
    }
    options
}

/// Reads `<ENV>", "<config>")` off the start of `s`, i.e. what follows
/// `check_environment("`. Anything that doesn't look like two plain string
/// literals (a variable passed instead, say) is skipped.
fn parse_call_args(s: &str) -> Option<(&str, &str)> {
    let (env, s) = s.split_once('"')?;
    let s = s.trim_start().strip_prefix(',')?.trim_start().strip_prefix('"')?;
    let (config, s) = s.split_once('"')?;
    s.trim_start().strip_prefix(')')?;

    let is_env_name = |v: &str| {
        !v.is_empty()
            && v
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
    };
    let is_config_name = |v: &str| {
        !v.is_empty()
            && v
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    };
    (is_env_name(env) && is_config_name(config)).then_some((env, config))
}

/// Lists the switches the given runner's `proton` script understands, for
/// the "Proton" settings, both global and per game. Wine runners have none.
#[tauri::command]
pub fn list_proton_options(
    state: State<ConfigState>,
    runner_id: String,
) -> Result<Vec<ProtonOption>, AppError> {
    let runners_dir = {
        let config = state
            .lock()
            .map_err(|_| "Configuration is locked".to_string())?;
        config.runners_dir.clone()
    };
    let runner = find_runner(&runners_dir, &runner_id)?;
    if runner.kind != RunnerKind::Proton {
        return Ok(Vec::new());
    }
    let script_path = runner.path.join("proton");
    let script = fs::read_to_string(&script_path)
        .map_err(|e| format!("Could not read {}: {e}", script_path.display()))?;
    Ok(parse_proton_options(&script))
}

#[tauri::command]
pub fn get_proton_config(state: State<ConfigState>) -> Result<ProtonConfig, AppError> {
    let config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    Ok(config.proton.clone())
}

#[tauri::command]
pub fn save_proton_config(
    app: AppHandle,
    state: State<ConfigState>,
    config: ProtonConfig,
) -> Result<ProtonConfig, AppError> {
    let mut app_config = state
        .lock()
        .map_err(|_| "Configuration is locked".to_string())?;
    app_config.proton = config;
    save_config(&app, &app_config)?;
    Ok(app_config.proton.clone())
}

#[cfg(test)]
mod tests;
