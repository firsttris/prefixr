//! In a Flatpak, MangoHud, Gamescope and vkBasalt aren't part of Prefixr:
//! they come as Flathub extensions the user installs
//! (`org.freedesktop.Platform.VulkanLayer.<Name>`), and without one the
//! setting silently does nothing. The settings show the install command for
//! a missing one, as Bottles and Heroic do. Natively nothing is reported.

use std::path::Path;

use serde::Serialize;

use crate::host_env::is_flatpak;

/// Where Flatpak mounts the Vulkan layer extensions, one folder each.
const EXTENSIONS_DIR: &str = "/usr/lib/extensions/vulkan";

/// The branch the extensions need: the freedesktop base of the runtime
/// (GNOME 51), the same as the manifests' `version: "26.08"`.
const EXTENSION_BRANCH: &str = "26.08";

/// The tools, by the name the frontend uses and their extension's name.
const TOOLS: &[(&str, &str)] = &[
    ("gamescope", "gamescope"),
    ("mangohud", "MangoHud"),
    ("vkbasalt", "vkBasalt"),
];

#[derive(Debug, PartialEq, Serialize)]
#[cfg_attr(test, derive(ts_rs::TS), ts(export))]
pub struct MissingExtension {
    /// `gamescope`, `mangohud` or `vkbasalt`.
    pub tool: String,
    /// The command that installs its extension.
    pub command: String,
}

/// The tools whose extension isn't installed, in a Flatpak; empty natively.
#[tauri::command]
pub fn missing_flatpak_extensions() -> Vec<MissingExtension> {
    if !is_flatpak() {
        return Vec::new();
    }
    missing_in(Path::new(EXTENSIONS_DIR))
}

fn missing_in(extensions_dir: &Path) -> Vec<MissingExtension> {
    TOOLS
        .iter()
        .filter(|(_, name)| !extensions_dir.join(name).is_dir())
        .map(|(tool, name)| MissingExtension {
            tool: tool.to_string(),
            command: format!(
                "flatpak install flathub org.freedesktop.Platform.VulkanLayer.{name}//{EXTENSION_BRANCH}"
            ),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::TestDir;

    #[test]
    fn lists_the_extensions_that_arent_mounted() {
        let dir = TestDir::new("vulkan-extensions");
        std::fs::create_dir_all(dir.join("MangoHud")).unwrap();
        let missing = missing_in(&dir);
        assert_eq!(
            missing,
            vec![
                MissingExtension {
                    tool: "gamescope".into(),
                    command: "flatpak install flathub org.freedesktop.Platform.VulkanLayer.gamescope//26.08".into(),
                },
                MissingExtension {
                    tool: "vkbasalt".into(),
                    command: "flatpak install flathub org.freedesktop.Platform.VulkanLayer.vkBasalt//26.08".into(),
                },
            ]
        );
    }

    /// The branch has to follow the runtime: the manifest's 32-bit
    /// extension names the same one.
    #[test]
    fn branch_matches_the_manifest() {
        let manifest = include_str!("../../../flatpak/spike/io.github.firsttris.Prefixr.yml");
        assert!(manifest.contains(&format!("version: \"{EXTENSION_BRANCH}\"")));
    }
}
