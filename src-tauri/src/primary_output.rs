//! Starting a game on the monitor the user is working on. Wine puts a
//! fullscreen game on the X11 primary output (its first display adapter),
//! also under Xwayland, whatever monitor Prefixr's window is on. So for a
//! launch, the output Prefixr's window is on (or, if that can't be told, the
//! one KWin calls active) becomes the X11 primary until the game exits.
//! Only X11 clients see that primary: a Wayland desktop's own panels stay
//! where they are. Best-effort throughout: without an X server, libXrandr
//! or a monitor to go by, nothing changes.

use std::ffi::CStr;
use std::ptr;

use tauri::{AppHandle, Manager};
use x11_dl::xlib::{Display, Window, Xlib};
use x11_dl::xrandr::{RROutput, RR_Connected, Xrandr};

/// The primary output a launch switched away from, switched back when
/// dropped — unless something else changed the primary in the meantime.
pub struct PrimaryOutputSwitch {
    previous: RROutput,
    target: RROutput,
    /// The output's name, e.g. `DP-2`, for the launch log.
    pub name: String,
}

impl Drop for PrimaryOutputSwitch {
    fn drop(&mut self) {
        let _ = with_display(|x| {
            if x.primary() == self.target {
                x.set_primary(self.previous);
            }
        });
    }
}

/// Makes the monitor the user is on the X11 primary. `None` if it already
/// is, or if there's nothing to go by.
pub fn switch_to_active_output(app: &AppHandle) -> Option<PrimaryOutputSwitch> {
    let window_monitor = window_monitor_name(app);
    let kwin_output = kwin_active_output();
    with_display(|x| {
        let outputs = x.connected_outputs();
        // GDK names a monitor by its RandR output on X11, but by its model
        // on Wayland, which then matches no output: KWin's name is the
        // connector there, as Xwayland's output names are.
        let (target, name) = [window_monitor, kwin_output]
            .into_iter()
            .flatten()
            .find_map(|wanted| outputs.iter().find(|(_, name)| *name == wanted))?
            .clone();
        let previous = x.primary();
        if previous == target {
            return None;
        }
        x.set_primary(target);
        Some(PrimaryOutputSwitch {
            previous,
            target,
            name,
        })
    })
    .flatten()
}

/// The monitor Prefixr's window is on, while it's shown.
fn window_monitor_name(app: &AppHandle) -> Option<String> {
    let window = app.get_webview_window("main")?;
    if !window.is_visible().unwrap_or(false) || window.is_minimized().unwrap_or(false) {
        return None;
    }
    window.current_monitor().ok()??.name().cloned()
}

/// The output KWin considers active: the one with the focused window (by
/// default) or the mouse pointer. `None` outside KDE Plasma.
fn kwin_active_output() -> Option<String> {
    let connection = zbus::blocking::Connection::session().ok()?;
    let reply = connection
        .call_method(
            Some("org.kde.KWin"),
            "/KWin",
            Some("org.kde.KWin"),
            "activeOutputName",
            &(),
        )
        .ok()?;
    let name: String = reply.body().deserialize().ok()?;
    Some(name).filter(|name| !name.is_empty())
}

struct X11 {
    xlib: Xlib,
    xrandr: Xrandr,
    display: *mut Display,
}

impl X11 {
    fn root(&self) -> Window {
        unsafe { (self.xlib.XDefaultRootWindow)(self.display) }
    }

    fn primary(&self) -> RROutput {
        unsafe { (self.xrandr.XRRGetOutputPrimary)(self.display, self.root()) }
    }

    fn set_primary(&self, output: RROutput) {
        unsafe {
            (self.xrandr.XRRSetOutputPrimary)(self.display, self.root(), output);
            (self.xlib.XSync)(self.display, 0);
        }
    }

    /// The outputs with a monitor on them, by name.
    fn connected_outputs(&self) -> Vec<(RROutput, String)> {
        let mut outputs = Vec::new();
        unsafe {
            let resources = (self.xrandr.XRRGetScreenResourcesCurrent)(self.display, self.root());
            if resources.is_null() {
                return outputs;
            }
            let count = usize::try_from((*resources).noutput).unwrap_or(0);
            for i in 0..count {
                let output = *(*resources).outputs.add(i);
                let info = (self.xrandr.XRRGetOutputInfo)(self.display, resources, output);
                if info.is_null() {
                    continue;
                }
                if (*info).connection == RR_Connected as u16 && (*info).crtc != 0 {
                    let name = CStr::from_ptr((*info).name).to_string_lossy().into_owned();
                    outputs.push((output, name));
                }
                (self.xrandr.XRRFreeOutputInfo)(info);
            }
            (self.xrandr.XRRFreeScreenResources)(resources);
        }
        outputs
    }
}

/// Runs `f` on a fresh connection to `$DISPLAY`. `None` without an X server
/// or the X libraries.
fn with_display<T>(f: impl FnOnce(&X11) -> T) -> Option<T> {
    let xlib = Xlib::open().ok()?;
    let xrandr = Xrandr::open().ok()?;
    let display = unsafe { (xlib.XOpenDisplay)(ptr::null()) };
    if display.is_null() {
        return None;
    }
    let x = X11 {
        xlib,
        xrandr,
        display,
    };
    let result = f(&x);
    unsafe { (x.xlib.XCloseDisplay)(x.display) };
    Some(result)
}
