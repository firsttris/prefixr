use super::*;

const DP1: RROutput = 1;
const DP2: RROutput = 2;
const HDMI: RROutput = 3;

#[test]
fn one_game_switches_and_switches_back() {
    let mut switched = None;
    assert_eq!(begin(&mut switched, DP1, DP2), (true, Some(DP2)));
    assert_eq!(end(&mut switched, DP2), Some(DP1));
    assert_eq!(switched, None);
}

#[test]
fn nothing_happens_on_the_primary_already() {
    let mut switched = None;
    assert_eq!(begin(&mut switched, DP1, DP1), (false, None));
    assert_eq!(switched, None);
}

// Two games on different monitors, the first one exiting first: the
// original primary comes back once both have exited.
#[test]
fn overlapping_games_restore_the_original_primary() {
    let mut switched = None;
    assert_eq!(begin(&mut switched, DP1, DP2), (true, Some(DP2)));
    assert_eq!(begin(&mut switched, DP2, DP1), (true, Some(DP1)));
    assert_eq!(end(&mut switched, DP1), None);
    // DP1 is the original, so nothing to switch back.
    assert_eq!(end(&mut switched, DP1), None);
    assert_eq!(switched, None);

    assert_eq!(begin(&mut switched, DP1, DP2), (true, Some(DP2)));
    assert_eq!(begin(&mut switched, DP2, HDMI), (true, Some(HDMI)));
    assert_eq!(end(&mut switched, HDMI), None);
    assert_eq!(end(&mut switched, HDMI), Some(DP1));
}

// A second game on the monitor the first one already made the primary
// still counts, so the first one exiting doesn't switch back under it.
#[test]
fn a_game_on_the_switched_monitor_keeps_it() {
    let mut switched = None;
    assert_eq!(begin(&mut switched, DP1, DP2), (true, Some(DP2)));
    assert_eq!(begin(&mut switched, DP2, DP2), (true, None));
    assert_eq!(end(&mut switched, DP2), None);
    assert_eq!(end(&mut switched, DP2), Some(DP1));
}

#[test]
fn a_primary_picked_meanwhile_stays() {
    let mut switched = None;
    begin(&mut switched, DP1, DP2);
    assert_eq!(end(&mut switched, HDMI), None);

    // Picked between two launches: it's what the last exit goes back to.
    begin(&mut switched, DP1, DP2);
    assert_eq!(begin(&mut switched, HDMI, DP1), (true, Some(DP1)));
    end(&mut switched, DP1);
    assert_eq!(end(&mut switched, DP1), Some(HDMI));
}

#[test]
fn the_kwin_script_names_its_monitors_as_json() {
    let script = kwin::script(&["DP-2".into(), "Odyssey \"G7\"".into()]);
    assert!(script.contains(r#"const targets = ["DP-2","Odyssey \"G7\""];"#));
    assert!(script.contains("workspace.windowAdded.connect"));
}

/// A stand-in for KWin's scripting on a private session bus.
mod fake_kwin {
    use std::io::{BufRead, BufReader};
    use std::process::{Child, Command, Stdio};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    pub struct State {
        /// Whether loadScript refuses, as KWin does for a broken script.
        pub refuse: bool,
        /// Whether Prefixr's script is loaded.
        pub loaded: bool,
        next_id: i32,
    }

    struct Scripting(Arc<Mutex<State>>);

    #[zbus::interface(name = "org.kde.kwin.Scripting")]
    impl Scripting {
        #[zbus(name = "loadScript")]
        fn load_script(&self, _path: String, _plugin: String) -> i32 {
            let mut state = self.0.lock().unwrap();
            if state.refuse || state.loaded {
                return -1;
            }
            state.loaded = true;
            state.next_id += 1;
            state.next_id % 8
        }

        #[zbus(name = "unloadScript")]
        fn unload_script(&self, _plugin: String) -> bool {
            std::mem::replace(&mut self.0.lock().unwrap().loaded, false)
        }
    }

    struct Script;

    #[zbus::interface(name = "org.kde.kwin.Script")]
    impl Script {
        #[zbus(name = "run")]
        fn run(&self) {}
    }

    pub struct Bus {
        daemon: Child,
        _kwin: zbus::Connection,
        pub state: Arc<Mutex<State>>,
    }

    impl Drop for Bus {
        fn drop(&mut self) {
            let _ = self.daemon.kill();
        }
    }

    /// Starts a session bus with the fake KWin on it, and points
    /// `DBUS_SESSION_BUS_ADDRESS` at it.
    pub fn start() -> (std::sync::MutexGuard<'static, ()>, Bus) {
        // One bus at a time: the address is process-wide, as is the count
        // of running games.
        static ONE: Mutex<()> = Mutex::new(());
        let one = ONE.lock().unwrap_or_else(|e| e.into_inner());
        let mut daemon = Command::new("dbus-daemon")
            .args(["--session", "--nofork", "--print-address=1"])
            .stdout(Stdio::piped())
            .spawn()
            .expect("dbus-daemon");
        let mut address = String::new();
        BufReader::new(daemon.stdout.take().unwrap())
            .read_line(&mut address)
            .unwrap();
        std::env::set_var("DBUS_SESSION_BUS_ADDRESS", address.trim());
        let state = Arc::new(Mutex::new(State::default()));
        let mut builder = zbus::connection::Builder::address(address.trim())
            .name("org.kde.KWin")
            .serve_at("/Scripting", Scripting(state.clone()));
        for id in 0..8 {
            builder = builder.serve_at(format!("/Scripting/Script{id}"), Script);
        }
        let kwin = zbus::block_on(builder.build()).unwrap();
        (
            one,
            Bus {
                daemon,
                _kwin: kwin,
                state,
            },
        )
    }
}

// A launch KWin refused in between mustn't make the first game's exit
// unload the script a later game still runs with.
#[test]
fn a_refused_kwin_script_keeps_the_count_of_running_games() {
    let (_one, bus) = fake_kwin::start();
    let dir = std::env::temp_dir().join(format!("prefixr-kwin-refused-{}", std::process::id()));
    let targets = ["DP-2".to_string()];

    let first = kwin::move_game_windows(&dir, &targets).expect("first launch");
    bus.state.lock().unwrap().refuse = true;
    assert!(kwin::move_game_windows(&dir, &targets).is_none());
    bus.state.lock().unwrap().refuse = false;
    let third = kwin::move_game_windows(&dir, &targets).expect("third launch");

    drop(first);
    assert!(
        bus.state.lock().unwrap().loaded,
        "the script was unloaded while the third game still runs"
    );
    drop(third);
    assert!(!bus.state.lock().unwrap().loaded);
    let _ = std::fs::remove_dir_all(dir);
}

// A Prefixr starting next to one that runs a game (a Steam shortcut's
// `--run` is a process of its own) mustn't take the running game's script
// away as a crashed one's leftover.
#[test]
fn a_starting_prefixr_keeps_the_kwin_script_of_a_running_one() {
    let (_one, bus) = fake_kwin::start();
    let dir = std::env::temp_dir().join(format!("prefixr-kwin-leftover-{}", std::process::id()));

    let game = kwin::move_game_windows(&dir, &["DP-2".to_string()]).expect("launch");
    // What another Prefixr process does at its start.
    kwin::unload_leftover();
    assert!(
        bus.state.lock().unwrap().loaded,
        "the running game's script was unloaded as a leftover"
    );
    drop(game);
    assert!(!bus.state.lock().unwrap().loaded);

    // A crashed one's leftover still goes.
    bus.state.lock().unwrap().loaded = true;
    kwin::unload_leftover();
    assert!(!bus.state.lock().unwrap().loaded);
    let _ = std::fs::remove_dir_all(dir);
}
