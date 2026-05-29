//! Mouse event reporting to the PTY.
//!
//! Encodes mouse events (clicks, motion, scroll) as escape sequences in
//! SGR, UTF-8, or Normal (X10) format, depending on the terminal mode.
//! Also handles alternate scroll (sending arrow keys in alt screen) and
//! motion deduplication.

mod locator;
mod wheel_dispatch;

use winit::dpi::PhysicalPosition;
use winit::event::MouseScrollDelta;

use oriterm_core::TermMode;
pub(crate) use oriterm_core::encode::mouse::{
    MouseButton, MouseEvent, MouseEventKind, MouseModifiers,
};

use crate::key_encoding::cursor_keys::{CursorKey, cursor_key_bytes};

use super::App;
use super::mouse_selection::{self, GridCtx};

use wheel_dispatch::{WheelDispatch, dispatch_wheel};

// DEC Locator observation dispatch lives in `locator.rs` (separate
// responsibility from mouse-event encoding). Re-export the winit→semantic
// mappers so `handle_mouse_input` (app::mouse_input) reaches them as
// `mouse_report::{element_state_to_kind, locator_semantic_button}`.
pub(super) use locator::{element_state_to_kind, locator_semantic_button};

/// Direction of a mouse wheel event after `parse_wheel_delta` normalization.
///
/// Replaces the historical `scroll_up: bool` pattern per
/// Hygiene — boolean parameters with
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
/// See root cause analysis §1B. The (DECCKM × direction) →
/// bytes mapping is the SSOT defined in [`crate::key_encoding::cursor_keys`];
/// keyboard cursor-arrow encoding (`legacy.rs::encode_legacy`) and Kitty's
/// CSI-u terminator selection (`kitty.rs::legacy_csi_info`) route through
/// the same helper to prevent semantic drift.
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
    let cursor = match direction {
        ScrollDirection::Up => CursorKey::Up,
        ScrollDirection::Down => CursorKey::Down,
    };
    Some(AltScrollPayload {
        bytes: cursor_key_bytes(cursor, app_cursor),
        repeat: lines,
    })
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
    /// Builds the semantic [`MouseEvent`] from winit state + cell
    /// coordinates and dispatches via [`App::write_pane_mouse_input`]
    /// which routes through the Decision 10 Option A apex (mode 1016
    /// pixel coords + `Effect::Pty` + `PtyWriteKind::MouseEvent`). The
    /// caller gates on [`should_report_mouse`](App::should_report_mouse)
    /// before calling; the encoder reads `Term::mode` on the IO thread.
    ///
    /// Returns `true` when the event was dispatched to Term (which also
    /// runs Step A locator observation), `false` when it was dropped
    /// because the cursor is out of grid (no usable cell) — the caller's
    /// DEC Locator observe-only path then records `Unavailable`.
    pub(super) fn report_mouse_button(
        &mut self,
        button: MouseButton,
        kind: MouseEventKind,
    ) -> bool {
        let Some((col, line)) = self.mouse_cell() else {
            return false;
        };

        let Some(pane_id) = self.active_pane_id() else {
            return false;
        };
        // Read the shared physical-pixel source ONCE; the DEC Locator Pu=1
        // `physical_px` is the same pair zipped (one grid-bounds clamp).
        let (px, py) = self.clamped_cursor_px();
        let physical_px = px.zip(py);
        let event = MouseEvent {
            button,
            kind,
            col,
            line,
            mods: self.mouse_modifiers(),
            px,
            py,
            // Cursor is in-grid here (guarded by the `mouse_cell()` Some
            // above). physical_px carries DEVICE pixels so the combined
            // mouse-tracking + DEC-Locator-Pu=1 case observes real pixels
            // (Term Step A reads it on this same dispatch).
            physical_px,
            in_grid: true,
        };
        self.write_pane_mouse_input(pane_id, &event);
        true
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
        // Read the shared physical-pixel source ONCE; the DEC Locator Pu=1
        // `physical_px` is the same pair zipped (one grid-bounds clamp).
        let (px, py) = self.clamped_cursor_px();
        let physical_px = px.zip(py);
        let event = MouseEvent {
            button,
            kind: MouseEventKind::Motion,
            col,
            line,
            mods: self.mouse_modifiers(),
            px,
            py,
            // Cursor is in-grid (guarded by `pixel_to_cell()` Some above).
            // physical_px carries DEVICE pixels for combined tracking +
            // DEC-Locator-Pu=1 observation on this dispatch.
            physical_px,
            in_grid: true,
        };
        if let Some(pane_id) = self.active_pane_id() {
            self.write_pane_mouse_input(pane_id, &event);
            self.mouse.set_last_reported_cell(Some((col, line)));
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
        // Cheap context only — `mouse_cell_clamped` (the expensive hit-test)
        // is queried lazily by `dispatch_wheel` via `WheelSink::cell_for_report`,
        // and only on the Tier-1 path. `parse_wheel_delta` runs once inside
        // the dispatcher.
        let cell_height = self
            .focused_renderer()
            .map_or(16.0, |r| r.cell_metrics().height);
        let pane_id = self.active_pane_id();
        let mods = self.mouse_modifiers();
        let shift_held = self.modifiers.shift_key();
        // Read the shared physical-pixel source ONCE; the DEC Locator Pu=1
        // `physical_px` is the same pair zipped (one grid-bounds clamp).
        let (px, py) = self.clamped_cursor_px();
        let physical_px = px.zip(py);
        let encoder_dispatched = dispatch_wheel(
            WheelDispatch {
                delta,
                cell_height,
                mode,
                shift_held,
                pane_id,
                mods,
                px,
                py,
                physical_px,
            },
            self,
        );
        // Additive DEC Locator wheel observation: when the Tier-1 encoder
        // did NOT dispatch (not mouse-reporting, shift-bypass, or out-of-grid
        // drop), refresh the locator position via the observe-only path. A
        // wheel scroll does not move the cursor, so this re-records the
        // current position (Unavailable when out-of-grid).
        self.maybe_observe_locator_position(encoder_dispatched);
    }

    /// Convert the current cursor position to a grid cell.
    pub(super) fn mouse_cell(&self) -> Option<(usize, usize)> {
        self.pixel_to_cell(self.mouse.cursor_pos())
    }

    /// Convert the current cursor position to a grid cell, clamping to edges.
    ///
    /// Unlike [`mouse_cell`], this never returns `None` when the grid and
    /// renderer are available — positions outside the grid are clamped to
    /// the nearest edge cell. Returns `None` only if the grid widget or
    /// renderer is missing.
    pub(super) fn mouse_cell_clamped(&self) -> Option<(usize, usize)> {
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
    pub(super) fn mouse_modifiers(&self) -> MouseModifiers {
        MouseModifiers {
            shift: self.modifiers.shift_key(),
            alt: self.modifiers.alt_key(),
            ctrl: self.modifiers.control_key(),
        }
    }

    /// PHYSICAL (device) cursor pixels relative to the grid origin, clamped
    /// to grid bounds — the single physical-pixel source shared by the
    /// SGR-Pixel (DEC mode 1016) encoder and DEC Locator Pu=1 observation.
    /// Per-event dispatch reads it once via `clamped_cursor_px()` and derives
    /// the `physical_px` shape with `px.zip(py)`; the locator path reads
    /// [`mouse_physical_px_opt`](Self::mouse_physical_px_opt).
    ///
    /// SGR-Pixel and CSI 14t both report PHYSICAL (device) pixels, so
    /// 1016-aware clients (notcurses) decode the correct cell at every DPI
    /// scale. Dividing by `scale_factor()` (logical/CSS pixels) was the unit
    /// mismatch that broke notcurses' `cellpxy` decode at scale > 1.0.
    ///
    /// Returns `(Some, Some)` for every event where the surface checks pass
    /// (focused context + grid bounds). Cursor positions outside the grid
    /// widget's bounds are clamped to the nearest valid device pixel via
    /// [`clamp_mouse_pixel_coords`] — mirroring
    /// [`mouse_cell_clamped`](Self::mouse_cell_clamped). Returns
    /// `(None, None)` ONLY when there is no usable surface. The encoder treats
    /// `None` pixel coords under `MOUSE_SGR_PIXEL` as an upstream invariant
    /// violation, not a drop signal — this clamping is what prevents the
    /// violation for legitimate clicks.
    ///
    /// See: bug-tracker/plans/BUG-08-057/
    pub(super) fn clamped_cursor_px(&self) -> (Option<u32>, Option<u32>) {
        let Some(wctx) = self.focused_ctx() else {
            return (None, None);
        };
        let Some(bounds) = wctx.terminal_grid.bounds() else {
            return (None, None);
        };
        clamp_mouse_pixel_coords(
            self.mouse.cursor_pos(),
            (f64::from(bounds.x()), f64::from(bounds.y())),
            (f64::from(bounds.width()), f64::from(bounds.height())),
        )
    }
}

/// Pure clamping helper for [`App::clamped_cursor_px`] — the pixel-
/// coordinate surface predicate factored out so it can be tested
/// headlessly without constructing `App` / `TermWindow` / `wgpu::Surface`.
/// Mirrors the clamping shape of `mouse_selection::pixel_to_cell` for
/// cell coords.
///
/// Returns `(Some, Some)` for every position when the surface inputs are
/// valid (both `bounds_size` dimensions ≥ 1.0, all inputs finite). Cursor
/// positions outside `[origin, origin + size)` clamp to the nearest valid
/// PHYSICAL (device) pixel. Returns `(None, None)` for the "no usable
/// surface" cases — sub-pixel or zero bounds, or any non-finite input
/// (NaN / infinity). Extracted from
/// `App::clamped_cursor_px` so it is testable without constructing
/// `App` / `TermWindow` / `wgpu::Surface`, mirroring the existing
/// `RecordingSink` / `WheelDispatch` headless-seam pattern.
///
/// The `bounds_size < 1.0` reject is load-bearing: `f64::clamp(min, max)`
/// panics when `min > max`, so a bounds width of `0.5` would compute
/// `max = bounds_size.0 - 1.0 = -0.5` and trip the panic. Sub-pixel
/// bounds are never a usable surface; reject them before the clamp.
///
/// See: bug-tracker/plans/BUG-08-057/
pub(crate) fn clamp_mouse_pixel_coords(
    pos: PhysicalPosition<f64>,
    bounds_origin: (f64, f64),
    bounds_size: (f64, f64),
) -> (Option<u32>, Option<u32>) {
    // Reject non-finite inputs before any arithmetic — propagating NaN
    // through clamp + `as u32` would fabricate origin coords silently.
    if !pos.x.is_finite()
        || !pos.y.is_finite()
        || !bounds_origin.0.is_finite()
        || !bounds_origin.1.is_finite()
        || !bounds_size.0.is_finite()
        || !bounds_size.1.is_finite()
    {
        return (None, None);
    }
    // Reject sub-pixel bounds. The < 1.0 reject avoids the
    // `f64::clamp(0.0, bounds_size.0 - 1.0)` panic when `bounds_size.0`
    // is between 0.0 (exclusive) and 1.0 (exclusive).
    if bounds_size.0 < 1.0 || bounds_size.1 < 1.0 {
        return (None, None);
    }
    let rel_x = (pos.x - bounds_origin.0).clamp(0.0, bounds_size.0 - 1.0);
    let rel_y = (pos.y - bounds_origin.1).clamp(0.0, bounds_size.1 - 1.0);
    // PHYSICAL (device) pixels relative to the grid origin — consistent with
    // the CSI 14t report, which is also physical. notcurses
    // derives `cellpxy = csi14t_pixy / rows` (physical cell height) and
    // decodes `(wire_y - 1) / cellpxy = physical_y / cellpxy`, recovering the
    // row exactly at every DPI scale. Dividing by `scale_factor` here (logical
    // pixels) was the prior logical-pixel unit mismatch — notcurses mis-decoded at
    // scale > 1.0.
    let physical_x = rel_x as u32;
    let physical_y = rel_y as u32;
    (Some(physical_x), Some(physical_y))
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
