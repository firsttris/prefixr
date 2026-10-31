//! Mounting a disc image through UDisks2 over D-Bus, in a Flatpak, where
//! there's no `udisksctl`. The Flatpak may talk to UDisks2
//! (`--system-talk-name=org.freedesktop.UDisks2`, as Lutris, Heroic, Bottles
//! and Faugus have); natively Prefixr runs `udisksctl` as before
//! (`cli/install.rs`).

use std::collections::HashMap;
use std::fs::File;
use std::os::fd::AsFd;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use zbus::{Connection, Fd, OwnedObjectPath, Variant};

const UDISKS: &str = "org.freedesktop.UDisks2";
const FILESYSTEM: &str = "org.freedesktop.UDisks2.Filesystem";

/// How long to wait for UDisks to find the filesystem on a new loop device.
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);

/// A disc image on a read-only loop device, mounted. Unmounted and the loop
/// device removed again when dropped.
pub struct LoopMount {
    connection: Connection,
    block: OwnedObjectPath,
    pub mount_point: PathBuf,
}

/// Options for a call that must not ask for a password: the same as
/// `udisksctl --no-user-interaction`.
fn options(extra: &[(&'static str, bool)]) -> HashMap<&'static str, Variant<'static>> {
    let mut options = HashMap::from([("auth.no_user_interaction", Variant::from(true))]);
    options.extend(
        extra
            .iter()
            .map(|&(key, value)| (key, Variant::from(value))),
    );
    options
}

impl LoopMount {
    pub fn new(image: &Path) -> Result<Self, String> {
        // zbus 6 has no blocking API any more; each call is driven with block_on
        let connection = zbus::block_on(Connection::system())
            .map_err(|e| format!("Could not reach UDisks2: {e}"))?;
        let file =
            File::open(image).map_err(|e| format!("Could not open {}: {e}", image.display()))?;
        let reply = zbus::block_on(connection.call_method(
            Some(UDISKS),
            "/org/freedesktop/UDisks2/Manager",
            Some("org.freedesktop.UDisks2.Manager"),
            "LoopSetup",
            &(Fd::from(file.as_fd()), options(&[("read-only", true)])),
        ))
        .map_err(|e| {
            format!(
                "Could not set up a loop device for {}: {e}",
                image.display()
            )
        })?;
        let block: OwnedObjectPath = reply
            .body()
            .deserialize()
            .map_err(|e| format!("UDisks2 didn't name a loop device: {e}"))?;
        let mut mount = LoopMount {
            connection,
            block,
            mount_point: PathBuf::new(),
        };
        mount.mount_point = mount.mount()?;
        Ok(mount)
    }

    /// Mounts the loop device's filesystem, or finds where the desktop
    /// already mounted it on its own. UDisks needs a moment to probe a new
    /// loop device; until then it has no Filesystem interface.
    fn mount(&self) -> Result<PathBuf, String> {
        let deadline = Instant::now() + PROBE_TIMEOUT;
        loop {
            if let Some(mount_point) = self.mount_points().into_iter().next() {
                return Ok(mount_point);
            }
            let result = zbus::block_on(self.connection.call_method(
                Some(UDISKS),
                self.block.as_str(),
                Some(FILESYSTEM),
                "Mount",
                &(options(&[]),),
            ));
            match result {
                Ok(reply) => {
                    let mount_point: String = reply
                        .body()
                        .deserialize()
                        .map_err(|e| format!("UDisks2 didn't say where it mounted: {e}"))?;
                    return Ok(PathBuf::from(mount_point));
                }
                Err(e) if Instant::now() >= deadline => {
                    return Err(format!("Could not mount the disc image: {e}"));
                }
                Err(_) => std::thread::sleep(Duration::from_millis(250)),
            }
        }
    }

    /// The filesystem's mount points (`MountPoints`, NUL-terminated byte
    /// strings); empty while it isn't mounted or not probed yet.
    fn mount_points(&self) -> Vec<PathBuf> {
        let Ok(reply) = zbus::block_on(self.connection.call_method(
            Some(UDISKS),
            self.block.as_str(),
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &(FILESYSTEM, "MountPoints"),
        )) else {
            return Vec::new();
        };
        let body = reply.body();
        let Ok(value) = body.deserialize::<Variant>() else {
            return Vec::new();
        };
        let Ok(points) = Vec::<Vec<u8>>::try_from(value) else {
            return Vec::new();
        };
        points
            .into_iter()
            .filter_map(|bytes| mount_point(&bytes))
            .collect()
    }
}

/// A mount point from UDisks2's NUL-terminated byte string.
fn mount_point(bytes: &[u8]) -> Option<PathBuf> {
    use std::os::unix::ffi::OsStrExt;
    let bytes = bytes.strip_suffix(&[0]).unwrap_or(bytes);
    (!bytes.is_empty()).then(|| PathBuf::from(std::ffi::OsStr::from_bytes(bytes)))
}

impl Drop for LoopMount {
    fn drop(&mut self) {
        let _ = zbus::block_on(self.connection.call_method(
            Some(UDISKS),
            self.block.as_str(),
            Some(FILESYSTEM),
            "Unmount",
            &(options(&[]),),
        ));
        let _ = zbus::block_on(self.connection.call_method(
            Some(UDISKS),
            self.block.as_str(),
            Some("org.freedesktop.UDisks2.Loop"),
            "Delete",
            &(options(&[]),),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mount_points_are_nul_terminated() {
        assert_eq!(
            mount_point(b"/run/media/u/GAME\0"),
            Some(PathBuf::from("/run/media/u/GAME"))
        );
        assert_eq!(mount_point(b"/mnt/x"), Some(PathBuf::from("/mnt/x")));
        assert_eq!(mount_point(b"\0"), None);
    }
}
