//! Mouse event reporting to the PTY.
//!
//! Encodes mouse events (clicks, motion, scroll) as escape sequences in
//! SGR, UTF-8, or Normal (X10) format, depending on the terminal mode.
//! Also handles alternate scroll (sending arrow keys in alt screen) and
//! motion deduplication.

mod encode;

use winit::dpi::PhysicalPosition;
use winit::event::MouseScrollDelta;

use oriterm_core::TermMode;
use oriterm_mux::PaneId;

use super::App;
use super::mouse_selection::{self, GridCtx};

pub(crate) use encode::{
    MouseButton, MouseEvent, MouseEventKind, MouseModifiers, encode_mouse_event,
};

/// Direction of a mouse wheel event after `parse_wheel_delta` normalization.
///
/// Replaces the historical `scroll_up: bool` pattern per
/// `impl-hygiene.md §Parameter Hygiene` — boolean parameters with
/// implicit semantics are a design smell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScrollDirection {
    Up,
    Down,
}

/// Which of the three `handle_mouse_wheel` tiers should consume this event.
///
/// Tier 1 wins if any mouse-tracking flag is set without shift-bypass;
/// Tier 2 wins if alt-screen + alternate-scroll without shift-bypass;
/// Tier 3 (viewport scroll) is the default. The exhaustive match in
/// `handle_mouse_wheel` makes regressions a compile error rather than
/// a doc-comment violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WheelTier {
    /// Tier 1: report mouse event via SGR/UTF-8/etc encoding.
    MouseReport,
    /// Tier 2: synthesize arrow keys for alt-scroll.
    AltScroll,
    /// Tier 3: viewport scroll.
    ViewportScroll,
}

/// Bytes + repeat count for a Tier-2 alt-scroll wheel translation.
///
/// `bytes` is one of four cursor sequences chosen by `(direction, app_cursor)`:
/// `\x1bOA` / `\x1bOB` (SS3) when DECCKM is set (application cursor mode),
/// `\x1b[A` / `\x1b[B` (CSI) when DECCKM is clear (normal cursor mode).
/// `repeat` is the number of times the caller should write `bytes` to the PTY.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AltScrollPayload {
    bytes: &'static [u8],
    repeat: usize,
}

/// Side-effect surface consumed by [`dispatch_wheel`].
///
/// Extracted so the wheel-event wiring can be matrix-tested headlessly
/// (a `RecordingSink` impl in `tests.rs` records calls; production uses
/// `impl WheelSink for App`). Per `impl-hygiene.md §Platform & External
/// Resource Abstraction` — the logic layer must not embed concrete
/// runtime resources, so the side effects flow through this trait.
pub(super) trait WheelSink {
    /// Write `bytes` to the PTY of `pane_id`.
    fn write_pane_input(&mut self, pane_id: PaneId, bytes: &[u8]);
    /// Scroll the viewport of `pane_id` by `lines` (positive = up).
    fn scroll_pane(&mut self, pane_id: PaneId, lines: isize);
    /// Mark the focused window as needing a redraw.
    fn mark_dirty(&mut self);
}

/// Inputs to [`dispatch_wheel`] — grouped per `impl-hygiene.md
/// §Parameter Hygiene` (>4 parameters → config struct).
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
    pub cell_for_report: Option<(usize, usize)>,
    pub mods: MouseModifiers,
}

/// Pure decision function: should mouse events be reported to the PTY for
/// the given mode + Shift state?
///
/// Single source of truth for the Tier-1 mouse-reporting gate — consumed
/// by both [`App::should_report_mouse`] (the `&self` method) and
/// [`classify_wheel_event`] (free function). The free function and method
/// share the same name; this is intentional — `App::should_report_mouse`
/// is a thin `&self` wrapper that calls THIS free function with
/// `self.modifiers.shift_key()`. Callers in `&self` scope use the method;
/// callers in free-function scope (e.g., `classify_wheel_event`) use this
/// free function directly.
#[must_use]
pub(super) fn should_report_mouse(mode: TermMode, shift_held: bool) -> bool {
    !shift_held && mode.intersects(TermMode::ANY_MOUSE)
}

/// Pure decision function: should a mouse wheel event be translated to
/// arrow key sequences in the PTY?
///
/// Returns `true` when the terminal is on the alternate screen AND has
/// `ALTERNATE_SCROLL` enabled AND the Shift modifier is NOT held.
/// Shift-bypass lets users scroll the viewport even in alternate screen.
///
/// Extracted from `handle_mouse_wheel` (Tier 2) so Section 09.1's
/// bridge test can verify the full parser → mode flag → decision path
/// without constructing a real `App`.
#[must_use]
pub(super) fn should_translate_wheel_to_arrows(mode: TermMode, shift_held: bool) -> bool {
    !shift_held && mode.contains(TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL)
}

/// Pure decision function: which tier consumes this wheel event?
///
/// Mirrors the order in [`App::handle_mouse_wheel`]. SSOT for the
/// dispatch invariant — consults the canonical [`should_report_mouse`]
/// (Tier-1 gate) and [`should_translate_wheel_to_arrows`] (Tier-2 gate)
/// rather than inlining their predicates.
#[must_use]
fn classify_wheel_event(mode: TermMode, shift_held: bool) -> WheelTier {
    if should_report_mouse(mode, shift_held) {
        return WheelTier::MouseReport;
    }
    if should_translate_wheel_to_arrows(mode, shift_held) {
        return WheelTier::AltScroll;
    }
    WheelTier::ViewportScroll
}

/// Pure decision function: byte selection + repeat count for the
/// Tier-2 alt-scroll wheel translation.
///
/// Returns `Some(payload)` iff [`should_translate_wheel_to_arrows`] is
/// true. Otherwise returns `None`.
///
/// DECCKM-aware byte selection per xterm spec (xterm `ctlseqs.txt`
/// §"The cursor keys transmit the following escape sequences depending
/// on the mode specified via the DECCKM escape sequence"; xterm
/// `scrollbar.c` `MODE_DECCKM ? ANSI_SS3 : ANSI_CSI` selection).
/// See BUG-08-015 root cause analysis §1B.
///
/// NOTE: The (DECCKM × direction) → bytes mapping mirrors the regular
/// cursor-key encoder in `oriterm/src/key_encoding/legacy.rs`. This
/// duplication is intentional — alt-scroll synthesis is on the
/// mouse-event path, key encoding is on the keyboard path, and unifying
/// them via a shared cursor-key helper is tracked as BUG-08-033.
/// Both paths share the same xterm spec (`ctlseqs.txt:2465-2473`)
/// so semantic drift between them would be a spec violation.
#[must_use]
fn tier2_alt_scroll_payload(
    mode: TermMode,
    shift_held: bool,
    lines: usize,
    direction: ScrollDirection,
) -> Option<AltScrollPayload> {
    if !should_translate_wheel_to_arrows(mode, shift_held) {
        return None;
    }
    let app_cursor = mode.contains(TermMode::APP_CURSOR);
    let bytes: &'static [u8] = match (direction, app_cursor) {
        (ScrollDirection::Up, true) => b"\x1bOA",
        (ScrollDirection::Up, false) => b"\x1b[A",
        (ScrollDirection::Down, true) => b"\x1bOB",
        (ScrollDirection::Down, false) => b"\x1b[B",
    };
    Some(AltScrollPayload {
        bytes,
        repeat: lines,
    })
}

/// Wire a wheel event through the 3-tier dispatcher to the side-effect
/// sink. Generic over the sink type for static dispatch per
/// `impl-hygiene.md §Dispatch Choice`.
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
pub(super) fn dispatch_wheel<S: WheelSink>(input: WheelDispatch, sink: &mut S) {
    let Some((lines, scroll_up)) = parse_wheel_delta(input.delta, input.cell_height) else {
        return;
    };
    let Some(pane_id) = input.pane_id else {
        return;
    };
    let direction = if scroll_up {
        ScrollDirection::Up
    } else {
        ScrollDirection::Down
    };

    match classify_wheel_event(input.mode, input.shift_held) {
        WheelTier::MouseReport => {
            let button = match direction {
                ScrollDirection::Up => MouseButton::ScrollUp,
                ScrollDirection::Down => MouseButton::ScrollDown,
            };
            let Some((col, line)) = input.cell_for_report else {
                return;
            };
            let event = MouseEvent {
                button,
                kind: MouseEventKind::Press,
                col,
                line,
                mods: input.mods,
            };
            let report = encode_mouse_event(&event, input.mode);
            let bytes = report.as_bytes();
            if !bytes.is_empty() {
                for _ in 0..lines {
                    sink.write_pane_input(pane_id, bytes);
                }
            }
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
        }
        WheelTier::ViewportScroll => {
            let scroll_lines = match direction {
                ScrollDirection::Up => lines as isize,
                ScrollDirection::Down => -(lines as isize),
            };
            sink.scroll_pane(pane_id, scroll_lines);
        }
    }

    sink.mark_dirty();
}

impl WheelSink for App {
    fn write_pane_input(&mut self, pane_id: PaneId, bytes: &[u8]) {
        Self::write_pane_input(self, pane_id, bytes);
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
}

impl App {
    /// Whether mouse events should be reported to the PTY for the given mode.
    ///
    /// True when any mouse reporting mode is active and Shift is NOT held.
    /// Shift-bypass lets users select text even when the terminal app has
    /// requested mouse reporting.
    ///
    /// Pure check — does not lock the terminal. Caller reads mode once via
    /// [`terminal_mode`](App::terminal_mode) and passes it through.
    ///
    /// Thin `&self` wrapper over the free [`should_report_mouse`] function.
    pub(super) fn should_report_mouse(&self, mode: TermMode) -> bool {
        should_report_mouse(mode, self.modifiers.shift_key())
    }

    /// Encode and send a mouse button event to the PTY.
    ///
    /// Encodes the event using the provided terminal mode, then writes to
    /// the PTY. No-op if the cursor is outside the grid.
    pub(super) fn report_mouse_button(
        &mut self,
        button: MouseButton,
        kind: MouseEventKind,
        mode: TermMode,
    ) {
        let Some((col, line)) = self.mouse_cell() else {
            return;
        };

        let Some(pane_id) = self.active_pane_id() else {
            return;
        };
        let event = MouseEvent {
            button,
            kind,
            col,
            line,
            mods: self.mouse_modifiers(),
        };
        let report = encode_mouse_event(&event, mode);
        let bytes = report.as_bytes();
        if !bytes.is_empty() {
            self.write_pane_input(pane_id, bytes);
        }
    }

    /// Report mouse motion to the PTY when tracking mode is active.
    ///
    /// Performs motion deduplication: only sends a report when the cell
    /// changes. Returns `true` if motion was reported (caller should
    /// skip selection handling).
    pub(super) fn report_mouse_motion(
        &mut self,
        position: PhysicalPosition<f64>,
        mode: TermMode,
    ) -> bool {
        // X10 mode reports presses only — no motion, no drag.
        if mode.contains(TermMode::MOUSE_X10) {
            return false;
        }

        let has_drag = mode.contains(TermMode::MOUSE_DRAG) && self.mouse.any_button_down();
        let has_motion = mode.contains(TermMode::MOUSE_MOTION);

        if !has_drag && !has_motion {
            return false;
        }

        // Shift-bypass: let user select text.
        if self.modifiers.shift_key() {
            return false;
        }

        let Some((col, line)) = self.pixel_to_cell(position) else {
            return false;
        };

        // Motion deduplication: skip if same cell as last report.
        if self.mouse.last_reported_cell() == Some((col, line)) {
            return false;
        }

        // Drag (button held) uses the actual button code; mode 1003 motion
        // without a button uses None (code 3+32 = 35).
        // Priority: left > middle > right (matches Alacritty).
        let button = if self.mouse.left_down() {
            MouseButton::Left
        } else if self.mouse.middle_down() {
            MouseButton::Middle
        } else if self.mouse.right_down() {
            MouseButton::Right
        } else {
            MouseButton::None
        };
        let event = MouseEvent {
            button,
            kind: MouseEventKind::Motion,
            col,
            line,
            mods: self.mouse_modifiers(),
        };
        let report = encode_mouse_event(&event, mode);
        let bytes = report.as_bytes();
        if !bytes.is_empty() {
            if let Some(pane_id) = self.active_pane_id() {
                self.write_pane_input(pane_id, bytes);
                self.mouse.set_last_reported_cell(Some((col, line)));
            }
        }
        true
    }

    /// Handle mouse wheel with 3-tier priority dispatched via [`WheelTier`].
    ///
    /// Extracts pure context (cell height, pane id, cell-for-report, mods,
    /// shift state) from `&mut self` and delegates the wiring to
    /// [`dispatch_wheel`]. Side effects flow through `WheelSink for App`
    /// so the wiring is matrix-testable headlessly with a `RecordingSink`
    /// in `tests.rs`.
    pub(super) fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta, mode: TermMode) {
        let cell_height = self
            .focused_renderer()
            .map_or(16.0, |r| r.cell_metrics().height);
        let pane_id = self.active_pane_id();
        let cell_for_report = self.mouse_cell_clamped();
        let mods = self.mouse_modifiers();
        let shift_held = self.modifiers.shift_key();
        dispatch_wheel(
            WheelDispatch {
                delta,
                cell_height,
                mode,
                shift_held,
                pane_id,
                cell_for_report,
                mods,
            },
            self,
        );
    }

    /// Convert the current cursor position to a grid cell.
    fn mouse_cell(&self) -> Option<(usize, usize)> {
        self.pixel_to_cell(self.mouse.cursor_pos())
    }

    /// Convert the current cursor position to a grid cell, clamping to edges.
    ///
    /// Unlike [`mouse_cell`], this never returns `None` when the grid and
    /// renderer are available — positions outside the grid are clamped to
    /// the nearest edge cell. Returns `None` only if the grid widget or
    /// renderer is missing.
    fn mouse_cell_clamped(&self) -> Option<(usize, usize)> {
        let wctx = self.focused_ctx()?;
        let cell = wctx.renderer.as_ref()?.cell_metrics();
        let ctx = GridCtx {
            widget: &wctx.terminal_grid,
            cell,
            word_delimiters: &self.config.behavior.word_delimiters,
        };
        let pos = self.mouse.cursor_pos();

        // Fast path: position is inside the grid.
        if let Some(cell) = mouse_selection::pixel_to_cell(pos, &ctx) {
            return Some(cell);
        }

        // Clamp to edge: compute the nearest valid cell.
        let bounds = ctx.widget.bounds()?;
        let cw = f64::from(ctx.cell.width);
        let ch = f64::from(ctx.cell.height);
        if cw <= 0.0 || ch <= 0.0 {
            return None;
        }
        let max_col = ((f64::from(bounds.width()) / cw) as usize).saturating_sub(1);
        let max_line = ((f64::from(bounds.height()) / ch) as usize).saturating_sub(1);

        let col = if pos.x < f64::from(bounds.x()) {
            0
        } else {
            (((pos.x - f64::from(bounds.x())) / cw) as usize).min(max_col)
        };
        let line = if pos.y < f64::from(bounds.y()) {
            0
        } else {
            (((pos.y - f64::from(bounds.y())) / ch) as usize).min(max_line)
        };
        Some((col, line))
    }

    /// Convert a pixel position to a grid cell, using grid context.
    fn pixel_to_cell(&self, pos: PhysicalPosition<f64>) -> Option<(usize, usize)> {
        let wctx = self.focused_ctx()?;
        let cell = wctx.renderer.as_ref()?.cell_metrics();
        let ctx = GridCtx {
            widget: &wctx.terminal_grid,
            cell,
            word_delimiters: &self.config.behavior.word_delimiters,
        };
        mouse_selection::pixel_to_cell(pos, &ctx)
    }

    /// Build modifier state from the current winit modifiers.
    fn mouse_modifiers(&self) -> MouseModifiers {
        MouseModifiers {
            shift: self.modifiers.shift_key(),
            alt: self.modifiers.alt_key(),
            ctrl: self.modifiers.control_key(),
        }
    }
}

/// Parse a mouse wheel delta into `(line_count, scroll_up)`.
///
/// Winit's `LineDelta` reports raw notches (1.0 per notch) without applying
/// the OS scroll lines setting. We multiply by the platform's configured
/// lines-per-notch (e.g. 3 on Windows) so scrolling respects the user's
/// system preference.
///
/// Returns `None` if the delta is too small to register.
fn parse_wheel_delta(delta: MouseScrollDelta, cell_height: f32) -> Option<(usize, bool)> {
    let (lines, scroll_up) = match delta {
        MouseScrollDelta::LineDelta(_, y) => {
            if y == 0.0 {
                return None;
            }
            let os_lines = crate::platform::scroll::wheel_scroll_lines() as f32;
            let scaled = y.abs() * os_lines;
            ((scaled.ceil() as usize).max(1), y > 0.0)
        }
        MouseScrollDelta::PixelDelta(pos) => {
            let y = pos.y;
            if y.abs() < f64::from(cell_height) / 2.0 {
                return None;
            }
            ((y.abs() / f64::from(cell_height)).ceil() as usize, y > 0.0)
        }
    };
    Some((lines, scroll_up))
}

#[cfg(test)]
mod tests;
