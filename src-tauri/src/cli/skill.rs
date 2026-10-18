//! `prefixr skill …`: the Agent Skill that teaches an AI agent (Claude Code
//! and others that read `SKILL.md` files) how to run the library through
//! this command line. It's built into the binary, so the one installed
//! always matches the commands this Prefixr has.

use std::fs;
use std::path::{Path, PathBuf};

use super::Out;
use crate::error::AppError;

pub(crate) const SKILL: &str = include_str!("../../../skills/prefixr/SKILL.md");

#[derive(clap::Subcommand, Debug)]
pub(super) enum SkillCommand {
    /// Install the skill for Claude Code (~/.claude/skills/prefixr), or into
    /// another agent's skills folder
    Install {
        /// The folder to put SKILL.md in, instead of ~/.claude/skills/prefixr
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    /// Print the skill, to read it or hand it to an agent
    Show,
}

/// Where Claude Code looks for a user's own skills.
fn default_dir() -> Result<PathBuf, AppError> {
    std::env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .map(|home| PathBuf::from(home).join(".claude/skills/prefixr"))
        .ok_or_else(|| "HOME is not set; pass --dir".to_string().into())
}

/// Writes SKILL.md into `dir`, replacing an older one.
pub(crate) fn install(dir: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    let path = dir.join("SKILL.md");
    fs::write(&path, SKILL).map_err(|e| format!("Could not write {}: {e}", path.display()))?;
    Ok(path)
}

#[derive(serde::Serialize)]
struct Installed {
    path: String,
}

/// Runs without the app: neither command reads the config or needs the
/// desktop session, so they also work over SSH.
pub(super) fn skill_command(out: Out, command: SkillCommand) -> Result<(), AppError> {
    match command {
        SkillCommand::Install { dir } => {
            let dir = match dir {
                Some(dir) => super::absolute(&dir)?,
                None => default_dir()?,
            };
            let path = install(&dir)?;
            let installed = Installed {
                path: path.to_string_lossy().into_owned(),
            };
            out.print(&installed, || {
                format!(
                    "Installed the Prefixr skill to {}. Agents started from now on pick it up.",
                    installed.path
                )
            })
        }
        SkillCommand::Show => out.print(&SKILL, || SKILL.trim_end().to_string()),
    }
}

#[cfg(test)]
mod tests;
