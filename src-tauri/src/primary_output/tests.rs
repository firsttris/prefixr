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
