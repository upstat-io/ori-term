use std::time::Duration;

use oriterm_mux::PaneId;

use crate::config::NotifyOnCommandFinish;

use super::{
    CommandCompleteAction, CommandCompleteInputs, CommandCompleteSink, CommandCompleteSurfaces,
    command_complete_action, dispatch_command_complete,
};

const TEN_S: Duration = Duration::from_secs(10);
const THIRTY_S: Duration = Duration::from_secs(30);
const FIVE_S: Duration = Duration::from_secs(5);

fn inputs(
    mode: NotifyOnCommandFinish,
    duration: Duration,
    is_in_focused_tab: bool,
    notify_command_bell: bool,
    is_audible: bool,
) -> CommandCompleteInputs {
    CommandCompleteInputs {
        mode,
        duration,
        threshold: TEN_S,
        is_in_focused_tab,
        notify_command_bell,
        is_audible,
    }
}

fn surfaces(set_bell: bool, transient_pulse: bool, audio: bool) -> CommandCompleteSurfaces {
    CommandCompleteSurfaces {
        set_bell,
        transient_pulse,
        audio,
    }
}

// Helper-level tests: command_complete_action

/// Regression: BUG-02-013 — duration < threshold must Suppress regardless
/// of mode/focus/per-surface flags.
#[test]
fn command_complete_action_below_threshold_returns_suppress() {
    let result = command_complete_action(&inputs(
        NotifyOnCommandFinish::Always,
        FIVE_S,
        false,
        true,
        true,
    ));
    assert_eq!(result, CommandCompleteAction::Suppress);
}

/// Regression: BUG-02-013 — mode=Never must Suppress.
#[test]
fn command_complete_action_with_mode_never_returns_suppress() {
    let result = command_complete_action(&inputs(
        NotifyOnCommandFinish::Never,
        THIRTY_S,
        false,
        true,
        true,
    ));
    assert_eq!(result, CommandCompleteAction::Suppress);
}

/// Regression: BUG-02-013 — mode=Unfocused with focused pane must Suppress.
#[test]
fn command_complete_action_with_mode_unfocused_and_focused_pane_returns_suppress() {
    let result = command_complete_action(&inputs(
        NotifyOnCommandFinish::Unfocused,
        THIRTY_S,
        true,
        true,
        true,
    ));
    assert_eq!(result, CommandCompleteAction::Suppress);
}

// Per-surface decision pins (Fire branch)

/// Regression: BUG-02-013 — PRIMARY REPRO FIX. Always+focused must Fire
/// transient_pulse + audio (set_bell stays background-only by design).
#[test]
fn command_complete_action_with_mode_always_focused_pane_returns_fire_with_pulse_and_audio() {
    let result = command_complete_action(&inputs(
        NotifyOnCommandFinish::Always,
        THIRTY_S,
        true,
        true,
        true,
    ));
    assert_eq!(
        result,
        CommandCompleteAction::Fire(surfaces(false, true, true))
    );
}

/// Regression: BUG-02-013 — Always+unfocused fires all four surfaces.
#[test]
fn command_complete_action_with_mode_always_unfocused_pane_returns_fire_with_all_surfaces() {
    let result = command_complete_action(&inputs(
        NotifyOnCommandFinish::Always,
        THIRTY_S,
        false,
        true,
        true,
    ));
    assert_eq!(
        result,
        CommandCompleteAction::Fire(surfaces(true, true, true))
    );
}

/// Regression: BUG-02-013 — existing Unfocused+unfocused path must not
/// regress; fires all surfaces.
#[test]
fn command_complete_action_with_mode_unfocused_unfocused_pane_returns_fire_with_all_surfaces() {
    let result = command_complete_action(&inputs(
        NotifyOnCommandFinish::Unfocused,
        THIRTY_S,
        false,
        true,
        true,
    ));
    assert_eq!(
        result,
        CommandCompleteAction::Fire(surfaces(true, true, true))
    );
}

/// Regression: BUG-02-013 — independence pin. Bell-disabled focused pane
/// must still fire audio when mode authorizes.
#[test]
fn command_complete_action_focused_with_bell_disabled_fires_audio_only() {
    let result = command_complete_action(&inputs(
        NotifyOnCommandFinish::Always,
        THIRTY_S,
        true,
        false,
        true,
    ));
    assert_eq!(
        result,
        CommandCompleteAction::Fire(surfaces(false, false, true))
    );
}

/// Regression: BUG-02-013 — independence pin. Audio-disabled focused pane
/// must still fire transient pulse when mode authorizes.
#[test]
fn command_complete_action_focused_with_audio_disabled_fires_pulse_only() {
    let result = command_complete_action(&inputs(
        NotifyOnCommandFinish::Always,
        THIRTY_S,
        true,
        true,
        false,
    ));
    assert_eq!(
        result,
        CommandCompleteAction::Fire(surfaces(false, true, false))
    );
}

/// Regression: BUG-02-013 — cross-clamp for unfocused row.
#[test]
fn command_complete_action_unfocused_with_bell_disabled_fires_audio_only() {
    let result = command_complete_action(&inputs(
        NotifyOnCommandFinish::Always,
        THIRTY_S,
        false,
        false,
        true,
    ));
    assert_eq!(
        result,
        CommandCompleteAction::Fire(surfaces(false, false, true))
    );
}

/// Regression: BUG-02-013 — cross-clamp for unfocused row.
#[test]
fn command_complete_action_unfocused_with_audio_disabled_fires_visuals_only() {
    let result = command_complete_action(&inputs(
        NotifyOnCommandFinish::Always,
        THIRTY_S,
        false,
        true,
        false,
    ));
    assert_eq!(
        result,
        CommandCompleteAction::Fire(surfaces(true, true, false))
    );
}

/// Regression: BUG-02-013 — empty per-surface configuration suppresses
/// every surface but mode still authorizes the Fire variant.
#[test]
fn command_complete_action_with_no_gates_open_fires_nothing() {
    let result = command_complete_action(&inputs(
        NotifyOnCommandFinish::Always,
        THIRTY_S,
        true,
        false,
        false,
    ));
    assert_eq!(
        result,
        CommandCompleteAction::Fire(surfaces(false, false, false))
    );
}

// Semantic + negative pins

/// Regression: BUG-02-013 — semantic pin. Always+focused with all gates
/// open returns the EXACT Fire(surfaces) shape that survives the bug fix.
/// Permanent regression guard.
#[test]
fn command_complete_action_with_mode_always_focused_returns_pulse_and_audio_not_set_bell() {
    let result = command_complete_action(&inputs(
        NotifyOnCommandFinish::Always,
        THIRTY_S,
        true,
        true,
        true,
    ));
    assert_eq!(
        result,
        CommandCompleteAction::Fire(CommandCompleteSurfaces {
            set_bell: false,
            transient_pulse: true,
            audio: true,
        })
    );
}

/// Regression: BUG-02-013 — negative pin. Across ALL focused cells,
/// `set_bell` must NEVER fire. Forbid-output pin proves the suppression
/// is active, not coincidental.
#[test]
fn command_complete_action_focused_does_not_fire_set_bell() {
    for bell in [false, true] {
        for audible in [false, true] {
            let result = command_complete_action(&inputs(
                NotifyOnCommandFinish::Always,
                THIRTY_S,
                true,
                bell,
                audible,
            ));
            if let CommandCompleteAction::Fire(s) = result {
                assert!(
                    !s.set_bell,
                    "set_bell must NEVER fire on focused tab — bell={bell}, audible={audible}"
                );
            }
        }
    }
}

// Dispatch-spy tests: dispatch_command_complete

#[derive(Default)]
struct SpySink {
    pane_live: bool,
    set_bell_calls: Vec<PaneId>,
    sync_calls: Vec<PaneId>,
    pulse_calls: Vec<PaneId>,
    dirty_calls: Vec<PaneId>,
    audio_calls: usize,
}

impl CommandCompleteSink for SpySink {
    fn pane_is_live(&self, _pane_id: PaneId) -> bool {
        self.pane_live
    }
    fn set_bell(&mut self, pane_id: PaneId) {
        self.set_bell_calls.push(pane_id);
    }
    fn sync_tab_bar(&mut self, pane_id: PaneId) {
        self.sync_calls.push(pane_id);
    }
    fn ring_pulse(&mut self, pane_id: PaneId) {
        self.pulse_calls.push(pane_id);
    }
    fn mark_dirty(&mut self, pane_id: PaneId) {
        self.dirty_calls.push(pane_id);
    }
    fn play_audio(&mut self) {
        self.audio_calls += 1;
    }
    fn log_completion(&self, _pane_id: PaneId, _duration: Duration) {
        // Spy doesn't need to record; covered by dedicated test below.
    }
}

#[derive(Default)]
struct SpySinkWithLog {
    pane_live: bool,
    log_calls: std::cell::RefCell<Vec<(PaneId, Duration)>>,
    audio_calls: usize,
}

impl CommandCompleteSink for SpySinkWithLog {
    fn pane_is_live(&self, _pane_id: PaneId) -> bool {
        self.pane_live
    }
    fn set_bell(&mut self, _pane_id: PaneId) {}
    fn sync_tab_bar(&mut self, _pane_id: PaneId) {}
    fn ring_pulse(&mut self, _pane_id: PaneId) {}
    fn mark_dirty(&mut self, _pane_id: PaneId) {}
    fn play_audio(&mut self) {
        self.audio_calls += 1;
    }
    fn log_completion(&self, pane_id: PaneId, duration: Duration) {
        self.log_calls.borrow_mut().push((pane_id, duration));
    }
}

/// Regression: BUG-02-013 — set_bell fires on live pane, also triggers sync.
#[test]
fn dispatch_command_complete_with_set_bell_true_calls_set_bell_and_sync() {
    let pane = PaneId::from_raw(1);
    let mut sink = SpySink {
        pane_live: true,
        ..Default::default()
    };
    dispatch_command_complete(pane, THIRTY_S, surfaces(true, false, false), &mut sink);
    assert_eq!(sink.set_bell_calls, vec![pane]);
    assert_eq!(sink.sync_calls, vec![pane]);
    assert!(sink.pulse_calls.is_empty());
    assert_eq!(sink.audio_calls, 0);
}

/// Regression: BUG-02-013 — orphan-pane guard. set_bell MUST NOT fire on
/// a closed pane (would reinsert into EmbeddedMux::bell_panes).
#[test]
fn dispatch_command_complete_with_set_bell_true_skips_when_pane_orphan() {
    let pane = PaneId::from_raw(1);
    let mut sink = SpySink {
        pane_live: false,
        ..Default::default()
    };
    dispatch_command_complete(pane, THIRTY_S, surfaces(true, false, false), &mut sink);
    assert!(sink.set_bell_calls.is_empty());
    assert!(sink.sync_calls.is_empty());
}

/// Regression: BUG-02-013 — transient pulse fires on live pane with mark_dirty.
#[test]
fn dispatch_command_complete_with_transient_pulse_true_calls_ring_and_mark_dirty() {
    let pane = PaneId::from_raw(1);
    let mut sink = SpySink {
        pane_live: true,
        ..Default::default()
    };
    dispatch_command_complete(pane, THIRTY_S, surfaces(false, true, false), &mut sink);
    assert_eq!(sink.pulse_calls, vec![pane]);
    assert_eq!(sink.dirty_calls, vec![pane]);
    assert!(sink.set_bell_calls.is_empty());
    assert_eq!(sink.audio_calls, 0);
}

/// Regression: BUG-02-013 — orphan-pane guard. Transient pulse + dirty
/// mark MUST NOT fire on closed pane (avoids mark_all_windows_dirty
/// fallback in redraw/mod.rs:49-50).
#[test]
fn dispatch_command_complete_with_transient_pulse_true_skips_when_pane_orphan() {
    let pane = PaneId::from_raw(1);
    let mut sink = SpySink {
        pane_live: false,
        ..Default::default()
    };
    dispatch_command_complete(pane, THIRTY_S, surfaces(false, true, false), &mut sink);
    assert!(sink.pulse_calls.is_empty());
    assert!(sink.dirty_calls.is_empty());
}

/// Regression: BUG-02-013 — audio fires even for orphan panes (documented
/// benign behavior; the user's command genuinely finished).
#[test]
fn dispatch_command_complete_with_audio_true_calls_play_audio_regardless_of_pane_liveness() {
    let pane = PaneId::from_raw(1);
    let mut sink_orphan = SpySink {
        pane_live: false,
        ..Default::default()
    };
    dispatch_command_complete(
        pane,
        THIRTY_S,
        surfaces(false, false, true),
        &mut sink_orphan,
    );
    assert_eq!(sink_orphan.audio_calls, 1);

    let mut sink_live = SpySink {
        pane_live: true,
        ..Default::default()
    };
    dispatch_command_complete(pane, THIRTY_S, surfaces(false, false, true), &mut sink_live);
    assert_eq!(sink_live.audio_calls, 1);
}

/// Regression: BUG-02-013 — log_completion fires on every Fire dispatch
/// (preserves the Phase-1.75 log-preservation invariant).
#[test]
fn dispatch_command_complete_logs_completion_unconditionally_when_action_is_fire() {
    let pane = PaneId::from_raw(1);
    let mut sink = SpySinkWithLog {
        pane_live: true,
        ..Default::default()
    };
    dispatch_command_complete(pane, THIRTY_S, surfaces(false, false, false), &mut sink);
    let log_calls = sink.log_calls.borrow();
    assert_eq!(log_calls.len(), 1);
    assert_eq!(log_calls[0].0, pane);
    assert_eq!(log_calls[0].1, THIRTY_S);
}

/// Regression: BUG-02-013 — empty surfaces dispatch only logs completion.
#[test]
fn dispatch_command_complete_with_no_surfaces_only_logs_completion() {
    let pane = PaneId::from_raw(1);
    let mut sink = SpySinkWithLog {
        pane_live: true,
        ..Default::default()
    };
    dispatch_command_complete(pane, THIRTY_S, surfaces(false, false, false), &mut sink);
    assert_eq!(sink.audio_calls, 0);
    assert_eq!(sink.log_calls.borrow().len(), 1);
}

// Combined helper × dispatch end-to-end pin

/// Regression: BUG-02-013 — PRIMARY REPRO FIX END-TO-END. Composes
/// command_complete_action(Always+focused) → Fire(surfaces) →
/// dispatch_command_complete with live pane. Spy must record:
/// 1 log + 0 set_bell + 0 sync + 1 ring + 1 mark_dirty + 1 audio.
#[test]
fn command_complete_action_then_dispatch_with_mode_always_focused_pane_calls_pulse_and_audio_only()
{
    let pane = PaneId::from_raw(1);
    let action = command_complete_action(&inputs(
        NotifyOnCommandFinish::Always,
        THIRTY_S,
        true,
        true,
        true,
    ));
    let CommandCompleteAction::Fire(s) = action else {
        panic!("expected Fire, got Suppress");
    };
    let mut sink = SpySink {
        pane_live: true,
        ..Default::default()
    };
    dispatch_command_complete(pane, THIRTY_S, s, &mut sink);

    assert!(sink.set_bell_calls.is_empty());
    assert!(sink.sync_calls.is_empty());
    assert_eq!(sink.pulse_calls, vec![pane]);
    assert_eq!(sink.dirty_calls, vec![pane]);
    assert_eq!(sink.audio_calls, 1);
}
