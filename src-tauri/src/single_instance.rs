//! The handoff to an already running Prefixr window, over the D-Bus name
//! tauri-plugin-single-instance owns for it (see `run()` in `lib.rs`): how a
//! desktop shortcut's `--launch` reaches the window, and in a Flatpak how
//! `prefixr game stop` does.

use crate::host_env::flatpak_id;

/// The D-Bus interface tauri-plugin-single-instance serves.
const INTERFACE: &str = "org.SingleInstance.DBus";

/// The base of the plugin's D-Bus name. In a Flatpak the app ID: an app may
/// only own names below its own ID there, and Flathub never grants one
/// outside it, so the plugin's default (the Tauri `identifier`,
/// `com.tristan.prefixr`) can't be owned and a second start opened a second
/// window (seen in the Flatpak spike). Natively that default, as before.
pub fn dbus_id(identifier: &str) -> String {
    flatpak_id().unwrap_or_else(|| identifier.to_string())
}

/// The D-Bus name and object path the plugin serves for `dbus_id`, built the
/// way tauri-plugin-single-instance 2.4 does (`platform_impl/linux.rs`,
/// without its `semver` feature).
fn bus_name_and_path(dbus_id: &str) -> (String, String) {
    let name = format!("{dbus_id}.SingleInstance");
    let path = format!("/{}", name.replace('.', "/").replace('-', "_"));
    (name, path)
}

/// Hands `argv` to the running window, as a second start of the app would.
/// Whether a window took it: `false` when none runs.
pub fn forward_to_window(dbus_id: &str, argv: &[String]) -> bool {
    let (name, path) = bus_name_and_path(dbus_id);
    let Ok(connection) = zbus::blocking::Connection::session() else {
        return false;
    };
    let cwd = std::env::current_dir().unwrap_or_default();
    connection
        .call_method(
            Some(name.as_str()),
            path.as_str(),
            Some(INTERFACE),
            "ExecuteCallback",
            &(argv, cwd.to_string_lossy().as_ref()),
        )
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bus_name_and_path_match_the_plugin() {
        assert_eq!(
            bus_name_and_path("io.github.firsttris.Prefixr"),
            (
                "io.github.firsttris.Prefixr.SingleInstance".to_string(),
                "/io/github/firsttris/Prefixr/SingleInstance".to_string()
            )
        );
        assert_eq!(
            bus_name_and_path("com.tristan.prefixr").1,
            "/com/tristan/prefixr/SingleInstance"
        );
        assert_eq!(bus_name_and_path("a.b-c").1, "/a/b_c/SingleInstance");
    }
}
