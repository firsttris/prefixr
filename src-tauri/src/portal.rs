//! XDG desktop portals, for what a Flatpak can't do by running a host tool.

use std::collections::HashMap;

use zbus::{OwnedObjectPath, Variant};

const PORTAL: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";

/// The Inhibit portal's flags: 4 suspending, 8 the session going idle (the
/// screensaver).
const INHIBIT_SUSPEND_AND_IDLE: u32 = 4 | 8;

/// Keeps the screen on and the system awake while it lives, through the
/// Inhibit portal (Faugus does the same in its Flatpak). The portal ends
/// the inhibition when the request is closed or the connection that made it
/// goes away, so the connection is kept with it.
pub struct Inhibition {
    connection: zbus::Connection,
    handle: OwnedObjectPath,
}

impl Inhibition {
    /// `None` when the portal isn't there or refuses: the game then simply
    /// runs without it, as natively without `systemd-inhibit`.
    pub fn new(reason: &str) -> Option<Self> {
        // zbus 6 has no blocking API any more; block_on drives the connection on this thread
        // and a helper thread of zbus's keeps it alive afterwards
        let connection = zbus::block_on(zbus::Connection::session()).ok()?;
        let options: HashMap<&str, Variant> = HashMap::from([("reason", Variant::from(reason))]);
        let reply = zbus::block_on(connection.call_method(
            Some(PORTAL),
            PORTAL_PATH,
            Some("org.freedesktop.portal.Inhibit"),
            "Inhibit",
            &("", INHIBIT_SUSPEND_AND_IDLE, options),
        ))
        .ok()?;
        let handle: OwnedObjectPath = reply.body().deserialize().ok()?;
        Some(Self { connection, handle })
    }
}

impl Drop for Inhibition {
    fn drop(&mut self) {
        let _ = zbus::block_on(self.connection.call_method(
            Some(PORTAL),
            self.handle.as_str(),
            Some("org.freedesktop.portal.Request"),
            "Close",
            &(),
        ));
    }
}
