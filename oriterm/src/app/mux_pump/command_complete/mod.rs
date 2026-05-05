//! Pure decision + dispatch helpers for `MuxNotification::CommandComplete`.
//!
//! Owns the full authorization lattice (threshold + mode + focus + per-surface
//! gates) so the caller in `mux_pump/mod.rs` is reduced to: (1) computing
//! `is_in_focused_tab`, (2) calling [`command_complete_action`], (3) on
//! `Fire`, dispatching surfaces via the [`CommandCompleteSink`] trait. Both
//! the helper and the dispatch are testable as free functions — the helper
//! against a pure decision lattice, the dispatch against a spy implementing
//! the sink.

use std::time::{Duration, Instant};

use oriterm_mux::PaneId;

use crate::app::App;
use crate::config::NotifyOnCommandFinish;
use crate::platform::audio;

/// Inputs to [`command_complete_action`]. Wraps the full threshold,
/// mode, focus, and per-surface state into one struct so the helper
/// signature stays under the four-parameter cap from
/// `.claude/rules/impl-hygiene.md` §Clean Code Patterns / Parameter Hygiene.
pub(super) struct CommandCompleteInputs {
    pub mode: NotifyOnCommandFinish,
    pub duration: Duration,
    pub threshold: Duration,
    pub is_in_focused_tab: bool,
    pub notify_command_bell: bool,
    pub is_audible: bool,
}

/// Authorization decision for a `CommandComplete` notification.
///
/// `Suppress` covers all cases where the user's `notify_on_command_finish`
/// mode (or the per-pane threshold gate) opts out: `Never`, `Unfocused`
/// when the pane is in the focused tab, and any duration below threshold.
/// `Fire(surfaces)` carries the per-surface decisions the caller dispatches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CommandCompleteAction {
    Suppress,
    Fire(CommandCompleteSurfaces),
}

/// Per-surface decisions for `Fire`. `set_bell` co-fires with the
/// `sync_tab_bar` mutation at the call site — they are NOT independent
/// surfaces but two sides of one persistent-indicator state change.
///
/// Persistent indicator (`set_bell`) is background-only — pointless on a
/// focused tab. Transient pulse and audio are user-perceptible alerts the
/// `Always` mode opts the user in to even on a focused tab.
///
/// `Always+unfocused` deliberately fires BOTH `set_bell` (persistent
/// background-pane indicator) AND `transient_pulse` (immediate flash) —
/// different purposes for a background pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CommandCompleteSurfaces {
    pub set_bell: bool,
    pub transient_pulse: bool,
    pub audio: bool,
}

pub(super) fn command_complete_action(inputs: &CommandCompleteInputs) -> CommandCompleteAction {
    if inputs.duration < inputs.threshold {
        return CommandCompleteAction::Suppress;
    }
    match inputs.mode {
        NotifyOnCommandFinish::Never => return CommandCompleteAction::Suppress,
        NotifyOnCommandFinish::Unfocused if inputs.is_in_focused_tab => {
            return CommandCompleteAction::Suppress;
        }
        _ => {}
    }
    let background = !inputs.is_in_focused_tab;
    CommandCompleteAction::Fire(CommandCompleteSurfaces {
        // Persistent indicator + the tab-bar sync that follows are
        // background-only by UX design: a focused tab shouldn't carry an
        // "unread alert" badge.
        set_bell: inputs.notify_command_bell && background,
        // Transient pulse + audio fire whenever their per-surface gate
        // allows — `Always+focused` is the user opting in to be alerted
        // even when looking at the pane.
        transient_pulse: inputs.notify_command_bell,
        audio: inputs.is_audible,
    })
}

/// Side-effect sink for [`dispatch_command_complete`]. The real sink in
/// `App::handle_command_complete` wires these to `mux.set_bell`,
/// `self.sync_tab_bar_for_pane`, `self.ring_owning_window_tab_bell`,
/// `self.mark_pane_window_dirty`, and `audio::play_bell`. Tests pass a
/// spy recording each call so the dispatch-layer mapping (surface bool →
/// sink-method call) is pinned without an `App` fixture.
///
/// `pane_is_live` lets the sink suppress persistent surfaces for orphan
/// panes — guards against `EmbeddedMux::set_bell` reinserting a closed
/// pane into `bell_panes` AND against `mark_pane_window_dirty` falling
/// back to `mark_all_windows_dirty()`.
pub(super) trait CommandCompleteSink {
    fn pane_is_live(&self, pane_id: PaneId) -> bool;
    fn set_bell(&mut self, pane_id: PaneId);
    fn sync_tab_bar(&mut self, pane_id: PaneId);
    fn ring_pulse(&mut self, pane_id: PaneId);
    fn mark_dirty(&mut self, pane_id: PaneId);
    fn play_audio(&mut self);
    fn log_completion(&self, pane_id: PaneId, duration: Duration);
}

/// Dispatch the per-surface side effects authorized by the helper.
///
/// Audio fires regardless of pane liveness — the user's command genuinely
/// finished; an audio cue is harmless even for a recently-closed pane.
/// All persistent / window-touching surfaces short-circuit when the pane
/// is no longer live to avoid the `EmbeddedMux::bell_panes` reinsertion
/// leak and the redraw `mark_all_windows_dirty` fallback.
pub(super) fn dispatch_command_complete<S: CommandCompleteSink>(
    pane_id: PaneId,
    duration: Duration,
    surfaces: CommandCompleteSurfaces,
    sink: &mut S,
) {
    sink.log_completion(pane_id, duration);
    let pane_live = sink.pane_is_live(pane_id);
    if surfaces.set_bell && pane_live {
        sink.set_bell(pane_id);
        sink.sync_tab_bar(pane_id);
    }
    if surfaces.transient_pulse && pane_live {
        sink.ring_pulse(pane_id);
        sink.mark_dirty(pane_id);
    }
    if surfaces.audio {
        sink.play_audio();
    }
}

/// Adapter wiring the `App`'s real side-effect methods to the
/// [`CommandCompleteSink`] trait so [`dispatch_command_complete`] can
/// run against either real `App` state or a test spy without diverging.
pub(super) struct AppCommandCompleteSink<'a> {
    pub app: &'a mut App,
}

impl CommandCompleteSink for AppCommandCompleteSink<'_> {
    fn pane_is_live(&self, pane_id: PaneId) -> bool {
        self.app.session.window_for_pane(pane_id).is_some()
    }

    fn set_bell(&mut self, pane_id: PaneId) {
        if let Some(mux) = self.app.mux.as_mut() {
            mux.set_bell(pane_id);
        }
    }

    fn sync_tab_bar(&mut self, pane_id: PaneId) {
        self.app.sync_tab_bar_for_pane(pane_id);
    }

    fn ring_pulse(&mut self, pane_id: PaneId) {
        self.app
            .ring_owning_window_tab_bell(pane_id, Instant::now());
    }

    fn mark_dirty(&mut self, pane_id: PaneId) {
        self.app.mark_pane_window_dirty(pane_id);
    }

    fn play_audio(&mut self) {
        audio::play_bell();
    }

    fn log_completion(&self, pane_id: PaneId, duration: Duration) {
        log::info!(
            "command completed in {pane_id} after {:.1}s",
            duration.as_secs_f64()
        );
    }
}

#[cfg(test)]
mod tests;
