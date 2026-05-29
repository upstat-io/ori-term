//! Wheel-event dispatch — trait-based side-effect surface so the wiring
//! between `parse_wheel_delta` / `classify_wheel_event` /
//! `tier2_alt_scroll_payload` and the PTY/viewport/dirty side effects is
//! matrix-testable headlessly.
//!
//! Production: `impl WheelSink for App` delegates to `&mut self` methods.
//! Tests: `RecordingSink` in `tests.rs` records every call without
//! constructing `App`.

use winit::event::MouseScrollDelta;

use oriterm_core::TermMode;
use oriterm_mux::PaneId;

use super::App;
use super::{
    MouseButton, MouseEvent, MouseEventKind, MouseModifiers, ScrollDirection, WheelTier,
    classify_wheel_event, parse_wheel_delta, tier2_alt_scroll_payload,
};

/// Side-effect surface consumed by [`dispatch_wheel`].
///
/// Extracted so the wheel-event wiring can be matrix-tested headlessly
/// (a `RecordingSink` impl in `tests.rs` records calls; production uses
/// `impl WheelSink for App`). The logic layer must not embed concrete
/// runtime resources, so the side effects flow through this trait.
pub(super) trait WheelSink {
    /// Write `bytes` to the PTY of `pane_id`.
    fn write_pane_input(&mut self, pane_id: PaneId, bytes: &[u8]);
    /// Dispatch a semantic mouse input to `pane_id` (Decision 10 apex —
    /// see [`App::write_pane_mouse_input`] for routing). Used by the
    /// Tier-1 wheel arm so wheel emissions land at the same
    /// `MuxEvent::PtyWrite { kind: PtyWriteKind::MouseEvent, .. }` apex
    /// as button/motion events.
    fn write_pane_mouse_input(&mut self, pane_id: PaneId, event: &MouseEvent);
    /// Optional: pre-set the encoding mode for sinks that encode
    /// locally (test fixtures). Production `App` impl reads the mode
    /// from `Term` on the IO thread and ignores this hook; default
    /// impl is a no-op.
    fn set_recorded_mode(&mut self, _mode: TermMode) {}
    /// Scroll the viewport of `pane_id` by `lines` (positive = up).
    fn scroll_pane(&mut self, pane_id: PaneId, lines: isize);
    /// Mark the focused window as needing a redraw.
    fn mark_dirty(&mut self);
    /// Return the grid cell to report for a Tier-1 mouse-report event,
    /// clamped to grid edges. Called only from the Tier-1 arm of
    /// [`dispatch_wheel`] so non-Tier-1 dispatches don't pay the
    /// hit-testing cost (pre-fix lazy semantics).
    fn cell_for_report(&mut self) -> Option<(usize, usize)>;
}

/// Inputs to [`dispatch_wheel`] — grouped
/// §Parameter Hygiene (>4 parameters → config struct).
///
/// `pane_id` is `Option<PaneId>` so the "no active pane → no dispatch"
/// early return is exercised by the dispatcher itself, keeping the
/// matrix cell testable without going through `App`.
#[derive(Debug, Clone, Copy)]
pub(super) struct WheelDispatch {
    pub delta: MouseScrollDelta,
    pub cell_height: f32,
    pub mode: TermMode,
    pub shift_held: bool,
    pub pane_id: Option<PaneId>,
    pub mods: MouseModifiers,
    /// PHYSICAL (device) cursor pixel coordinates for SGR-Pixel mode 1016
    /// encoding. `(None,
    /// None)` when SGR-Pixel is not active or the cursor is outside the
    /// grid; the cell encoders ignore them.
    pub px: Option<u32>,
    pub py: Option<u32>,
    /// DEVICE physical-pixel cursor coordinates for DEC Locator Pu=1
    /// observation — the SAME grid-relative physical coordinate as
    /// `px`/`py`. `None` when no usable surface. Read by
    /// `Term` Step A so a wheel dispatched while mouse-tracking +
    /// locator-Pu=1 are both active observes real pixels.
    pub physical_px: Option<(u32, u32)>,
}

/// Wire a wheel event through the 3-tier dispatcher to the side-effect
/// sink. Generic over the sink type for static dispatch.
///
/// The dispatcher exits early (without firing `mark_dirty`) on three
/// conditions:
/// - [`parse_wheel_delta`] returned `None` (delta below threshold);
/// - `input.pane_id == None` (no active pane);
/// - Tier-1 path with `input.cell_for_report == None` (cursor outside
///   grid, mouse-report mode set).
///
/// On any other path, exactly one of the three tier arms runs, then
/// `sink.mark_dirty()` fires once. Tier-1 + Tier-2 issue
/// `sink.write_pane_input` `lines` (or `payload.repeat`) times when the
/// resulting bytes are nonempty; Tier-3 issues a single `sink.scroll_pane`
/// with the signed `lines` count.
pub(super) fn dispatch_wheel<S: WheelSink>(input: WheelDispatch, sink: &mut S) -> bool {
    let Some((lines, scroll_up)) = parse_wheel_delta(input.delta, input.cell_height) else {
        return false;
    };
    let Some(pane_id) = input.pane_id else {
        return false;
    };
    let direction = if scroll_up {
        ScrollDirection::Up
    } else {
        ScrollDirection::Down
    };

    sink.set_recorded_mode(input.mode);
    // `dispatched` records whether the Tier-1 mouse-report encoder actually
    // sent the event to Term (which also runs Step A locator observation).
    // The caller observes the DEC Locator position via the observe-only path
    // when this returns false (no Tier-1, or out-of-grid Tier-1 drop).
    let dispatched = match classify_wheel_event(input.mode, input.shift_held) {
        WheelTier::MouseReport => {
            let button = match direction {
                ScrollDirection::Up => MouseButton::ScrollUp,
                ScrollDirection::Down => MouseButton::ScrollDown,
            };
            let Some((col, line)) = sink.cell_for_report() else {
                // Out-of-grid Tier-1 drop: no dispatch, no mark_dirty (per
                // the early-exit contract). Caller's observe-only path
                // records Unavailable.
                return false;
            };
            let event = MouseEvent {
                button,
                kind: MouseEventKind::Press,
                col,
                line,
                mods: input.mods,
                px: input.px,
                py: input.py,
                // In-grid (guarded by `cell_for_report()` Some above).
                // physical_px carries DEVICE pixels for combined tracking +
                // DEC-Locator-Pu=1 observation on this dispatch.
                physical_px: input.physical_px,
                in_grid: true,
            };
            for _ in 0..lines {
                sink.write_pane_mouse_input(pane_id, &event);
            }
            true
        }
        WheelTier::AltScroll => {
            // tier2_alt_scroll_payload re-checks should_translate_wheel_to_arrows
            // even though classify_wheel_event already confirmed it returned true
            // here; the re-check preserves the helper's standalone-testability
            // contract.
            if let Some(payload) =
                tier2_alt_scroll_payload(input.mode, input.shift_held, lines, direction)
                && !payload.bytes.is_empty()
            {
                for _ in 0..payload.repeat {
                    sink.write_pane_input(pane_id, payload.bytes);
                }
            }
            false
        }
        WheelTier::ViewportScroll => {
            let scroll_lines = match direction {
                ScrollDirection::Up => lines as isize,
                ScrollDirection::Down => -(lines as isize),
            };
            sink.scroll_pane(pane_id, scroll_lines);
            false
        }
    };

    sink.mark_dirty();
    dispatched
}

impl WheelSink for App {
    fn write_pane_input(&mut self, pane_id: PaneId, bytes: &[u8]) {
        // SAFETY: `Self::write_pane_input` resolves to the inherent
        // `App::write_pane_input` (pane_accessors.rs), not the trait
        // method — Rust's UFCS prefers inherent methods when both exist
        // with the same name. Renaming the inherent method without
        // updating this call site would silently route through the
        // trait and recurse.
        Self::write_pane_input(self, pane_id, bytes);
    }
    fn write_pane_mouse_input(&mut self, pane_id: PaneId, event: &MouseEvent) {
        // Same UFCS contract as write_pane_input: resolve to the
        // inherent `App::write_pane_mouse_input` (pane_accessors.rs).
        Self::write_pane_mouse_input(self, pane_id, event);
    }
    fn scroll_pane(&mut self, pane_id: PaneId, lines: isize) {
        if let Some(mux) = self.mux.as_mut() {
            mux.scroll_display(pane_id, lines);
        }
    }
    fn mark_dirty(&mut self) {
        if let Some(ctx) = self.focused_ctx_mut() {
            ctx.root.mark_dirty();
        }
    }
    fn cell_for_report(&mut self) -> Option<(usize, usize)> {
        Self::mouse_cell_clamped(self)
    }
}
