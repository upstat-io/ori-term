//! Control-character, erase, scroll, and tab handlers.
//!
//! Inherent `pub(super) fn` helpers on `Term<S>` invoked by the
//! `Handler for Term<S>` trait impl in `handler/mod.rs`. Keeps the
//! trait impl thin per the 500-line file budget.

use vte::ansi::{ClearMode, LineClearMode, TabulationClearMode};

use crate::effect::sink::EffectSink;
use crate::effect::{Effect, HostEffect};
use crate::grid::editing::{DisplayEraseMode, LineEraseMode};
use crate::grid::navigation::TabClearMode;
use crate::term::{Term, TermMode};

impl<S: EffectSink> Term<S> {
    /// Emit a `Bell` host effect, plus `UrgencyHint` when DECSET 1042 is set.
    #[inline]
    pub(super) fn bell_impl(&self) {
        self.effect_sink.push(Effect::Host(HostEffect::Bell));
        // Mode 1042 (DECSET ?1042h) — when set, BEL also requests
        // window-manager attention via the host adapter (taskbar flash on
        // Windows, dock bounce on macOS, urgency hint on X11/Wayland).
        if self.mode().contains(TermMode::URGENCY_HINTS) {
            self.effect_sink.push(Effect::Host(HostEffect::UrgencyHint));
        }
    }

    /// Line feed — newline-mode aware, with image pruning across the scrollback boundary.
    ///
    /// Snapshots `total_evicted()` BEFORE mutating the grid so the
    /// post-mutation count reflects any rows that crossed into the
    /// scrollback. `prune_images_if_evicted` MUST run AFTER the grid
    /// mutation to release graphics-protocol images anchored to those
    /// rows; reordering this pair leaks image memory.
    #[inline]
    pub(super) fn linefeed_impl(&mut self) {
        self.selection_dirty = true;
        let lnm = self.mode.contains(TermMode::LINE_FEED_NEW_LINE);
        let prev = self.grid().total_evicted();
        let grid = self.grid_mut();
        if lnm {
            grid.next_line();
        } else {
            grid.linefeed();
        }
        self.prune_images_if_evicted(prev);
    }

    /// Newline (`next_line`) with image pruning.
    ///
    /// Same `total_evicted` → mutate → `prune_images_if_evicted` ordering
    /// as `linefeed_impl`; see that method for rationale.
    pub(super) fn newline_impl(&mut self) {
        self.selection_dirty = true;
        let prev = self.grid().total_evicted();
        self.grid_mut().next_line();
        self.prune_images_if_evicted(prev);
    }

    /// Scroll up `count` lines with image pruning across the scrollback boundary.
    ///
    /// Same `total_evicted` → mutate → `prune_images_if_evicted` ordering
    /// as `linefeed_impl`; see that method for rationale.
    pub(super) fn scroll_up_impl(&mut self, count: usize) {
        self.selection_dirty = true;
        let prev = self.grid().total_evicted();
        self.grid_mut().scroll_up(count);
        self.prune_images_if_evicted(prev);
    }

    /// `ED` — erase display per `ClearMode`, with companion image cleanup.
    pub(super) fn clear_screen_impl(&mut self, mode: &ClearMode) {
        self.selection_dirty = true;
        let erase = match mode {
            ClearMode::Below => DisplayEraseMode::Below,
            ClearMode::Above => DisplayEraseMode::Above,
            ClearMode::All => DisplayEraseMode::All,
            ClearMode::Saved => DisplayEraseMode::Scrollback,
        };
        self.grid_mut().erase_display(erase);
        self.clear_images_after_ed(mode);
    }

    /// `EL` — erase line per `LineClearMode`, with companion image cleanup.
    pub(super) fn clear_line_impl(&mut self, mode: &LineClearMode) {
        self.selection_dirty = true;
        let erase = match mode {
            LineClearMode::Right => LineEraseMode::Right,
            LineClearMode::Left => LineEraseMode::Left,
            LineClearMode::All => LineEraseMode::All,
        };
        self.grid_mut().erase_line(erase);
        self.clear_images_after_el(mode);
    }

    /// `TBC` — clear tab stops (current column or all columns).
    pub(super) fn clear_tabs_impl(&mut self, mode: &TabulationClearMode) {
        let clear = match mode {
            TabulationClearMode::Current => TabClearMode::Current,
            TabulationClearMode::All => TabClearMode::All,
        };
        self.grid_mut().clear_tab_stop(clear);
    }
}
