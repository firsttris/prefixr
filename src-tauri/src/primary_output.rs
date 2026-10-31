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
use std::sync::Mutex;

use tauri::{AppHandle, Manager};
use x11_dl::xlib::{Display, Window, Xlib};
use x11_dl::xrandr::{RROutput, RR_Connected, Xrandr};

/// One launch's part in the switch, ended when dropped. The primary goes
/// back when the last running game that switched it exits.
pub struct PrimaryOutputSwitch {
    /// The output's name, e.g. `DP-2`, for the launch log.
    pub name: String,
}

impl Drop for PrimaryOutputSwitch {
    fn drop(&mut self) {
        let mut switched = SWITCHED.lock().unwrap_or_else(|e| e.into_inner());
        let _ = with_display(|x| {
            if let Some(output) = end(&mut switched, x.primary()) {
                x.set_primary(output);
            }
        });
    }
}

/// What the running games switched: shared, since their launches can
/// overlap and each one would otherwise only know the primary it found,
/// possibly another game's.
#[derive(Debug, PartialEq)]
struct Switched {
    /// The primary before the first of them, to go back to.
    original: RROutput,
    /// The primary the latest of them set.
    set: RROutput,
    /// How many of them are still running.
    games: usize,
}

static SWITCHED: Mutex<Option<Switched>> = Mutex::new(None);

/// A launch that wants `target` as the primary, with `current` the primary
/// now. Whether it takes part (and has to `end`), and the output to make the
/// primary, if any.
fn begin(
    switched: &mut Option<Switched>,
    current: RROutput,
    target: RROutput,
) -> (bool, Option<RROutput>) {
    let change = (current != target).then_some(target);
    match switched {
        // Nothing to switch, and nothing to switch back later.
        None if change.is_none() => (false, None),
        None => {
            *switched = Some(Switched {
                original: current,
                set: target,
                games: 1,
            });
            (true, change)
        }
        Some(state) => {
            // Someone else picked a primary since: that's the one to go
            // back to.
            if current != state.set {
                state.original = current;
            }
            state.set = target;
            state.games += 1;
            (true, change)
        }
    }
}

/// A game that took part exited, with `current` the primary now. The output
/// to make the primary again, once it's the last one — unless someone else
/// picked one meanwhile.
fn end(switched: &mut Option<Switched>, current: RROutput) -> Option<RROutput> {
    let state = switched.as_mut()?;
    state.games -= 1;
    if state.games > 0 {
        return None;
    }
    let state = switched.take()?;
    (current == state.set && current != state.original).then_some(state.original)
}

/// Makes the monitor the user is on the X11 primary. `None` if it already
/// is, or if there's nothing to go by.
pub fn switch_to_active_output(app: &AppHandle) -> Option<PrimaryOutputSwitch> {
    let window_monitor = window_monitor_name(app);
    let kwin_output = kwin_active_output();
    let mut switched = SWITCHED.lock().unwrap_or_else(|e| e.into_inner());
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
        let (takes_part, change) = begin(&mut switched, x.primary(), target);
        if let Some(output) = change {
            x.set_primary(output);
        }
        takes_part.then_some(PrimaryOutputSwitch { name })
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
    let reply = zbus::block_on(async {
        let connection = zbus::Connection::session().await?;
        connection
            .call_method(
                Some("org.kde.KWin"),
                "/KWin",
                Some("org.kde.KWin"),
                "activeOutputName",
                &(),
            )
            .await
    })
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

#[cfg(test)]
mod tests;
