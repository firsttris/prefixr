use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The kind of compatibility layer a runner provides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RunnerKind {
    Proton,
    Wine,
}

/// A Proton or Wine build discovered on disk under the configured runners directory.
/// `id` is the folder name and doubles as the stable identifier used elsewhere
/// (e.g. `Game::runner_id`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Runner {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub kind: RunnerKind,
}

/// A Wine prefix known to the app, tracked in the config file. Not tied to a
/// runner — the runner used to initialize it is only needed transiently by
/// `create_prefix`; which runner launches a game living in this prefix is
/// decided per-game via `Game::runner_id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefixInfo {
    pub path: PathBuf,
}

/// A game the user has added to the library.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub id: Uuid,
    pub name: String,
    pub exe_path: PathBuf,
    pub prefix_path: PathBuf,
    pub runner_id: String,
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
}

/// Payload for `add_game`; the id is assigned by the backend.
#[derive(Debug, Clone, Deserialize)]
pub struct GameInput {
    pub name: String,
    pub exe_path: PathBuf,
    pub prefix_path: PathBuf,
    pub runner_id: String,
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
}
