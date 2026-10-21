//! Stopping a game from another Prefixr process in a Flatpak. Every
//! `flatpak run` is its own PID namespace, so `prefixr game stop` can't see
//! the processes of a game another instance runs: the window, a CLI
//! `game launch` or a Steam shortcut's `--run`. So whichever instance runs a
//! game owns a D-Bus name for it while it runs, below the app ID (which a
//! Flatpak may own without a permission), and `game stop` asks that name.

use tauri::{AppHandle, Manager};
use uuid::Uuid;

use crate::commands::games::{kill_running_game, RunningGames};

const INTERFACE: &str = "io.github.firsttris.Prefixr.Game";
const PATH: &str = "/io/github/firsttris/Prefixr/Game";

/// The D-Bus name for a game, e.g. `io.github.firsttris.Prefixr.Game.g248930e7…`.
/// An element of a bus name can't start with a digit, hence the `g`.
fn bus_name(app_id: &str, id: Uuid) -> String {
    format!("{app_id}.Game.g{}", id.simple())
}

struct GameControl {
    app: AppHandle,
    id: Uuid,
}

#[zbus::interface(name = "io.github.firsttris.Prefixr.Game")]
impl GameControl {
    /// Ends the game the way the Stop button does; returns right away.
    fn stop(&self) {
        let (app, id) = (self.app.clone(), self.id);
        tauri::async_runtime::spawn(async move {
            let running = app.state::<RunningGames>();
            let _ = kill_running_game(&running, id).await;
        });
    }
}

/// The game's D-Bus name, owned for as long as this lives.
pub struct GameName(#[allow(dead_code)] zbus::blocking::Connection);

/// Takes the game's D-Bus name for this process. `None` if the session bus
/// isn't there or the name is taken; `game stop` then reports the game as
/// running elsewhere, as before.
pub fn serve(app: &AppHandle, app_id: &str, id: Uuid) -> Option<GameName> {
    zbus::blocking::connection::Builder::session()
        .ok()?
        .name(bus_name(app_id, id))
        .ok()?
        // Never take the name from an instance that already runs the game
        // (zbus would by default), which `game stop` then couldn't reach.
        .replace_existing_names(false)
        .allow_name_replacements(false)
        .serve_at(
            PATH,
            GameControl {
                app: app.clone(),
                id,
            },
        )
        .ok()?
        .build()
        .ok()
        .map(GameName)
}

/// Asks the Prefixr process that runs the game to stop it; whether one
/// took the request.
pub fn request_stop(app_id: &str, id: Uuid) -> bool {
    let Ok(connection) = zbus::blocking::Connection::session() else {
        return false;
    };
    connection
        .call_method(
            Some(bus_name(app_id, id).as_str()),
            PATH,
            Some(INTERFACE),
            "Stop",
            &(),
        )
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bus_name_is_a_valid_name_below_the_app_id() {
        let id = Uuid::parse_str("248930e7-2809-4558-adb4-21bc7bb85fd6").unwrap();
        let name = bus_name("io.github.firsttris.Prefixr", id);
        assert_eq!(
            name,
            "io.github.firsttris.Prefixr.Game.g248930e728094558adb421bc7bb85fd6"
        );
        assert!(zbus::names::WellKnownName::try_from(name.as_str()).is_ok());
    }
}
