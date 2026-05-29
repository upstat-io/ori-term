//! DEC Locator observation dispatch (App side).
//!
//! Splits the locator-observation responsibility out of `mouse_report`
//! (which owns mouse-event ENCODING). The App routes here on the
//! locator-only path — `Term::observe_locator_input` (Step A) records the
//! position WITHOUT encoding, so the encoder cannot fire on a shift-bypass
//! / no-tracking path. Encoding stays in `mod.rs`.

use super::App;
use super::{MouseButton, MouseEvent, MouseEventKind};

/// Map a winit press/release [`ElementState`](winit::event::ElementState)
/// to the semantic [`MouseEventKind`]. Single source for the conversion
/// shared by the L/M/R dispatch arms in `handle_mouse_input`.
#[must_use]
pub(in crate::app) fn element_state_to_kind(state: winit::event::ElementState) -> MouseEventKind {
    match state {
        winit::event::ElementState::Pressed => MouseEventKind::Press,
        winit::event::ElementState::Released => MouseEventKind::Release,
    }
}

/// Map a winit mouse button to the semantic [`MouseButton`] used for DEC
/// Locator observation. Returns `None` for Back/Forward and any other
/// button DEC Locator does not report — only Left/Middle/Right carry a Pm
/// bit per xterm `button.c:944-948`.
#[must_use]
pub(in crate::app) fn locator_semantic_button(
    button: winit::event::MouseButton,
) -> Option<MouseButton> {
    match button {
        winit::event::MouseButton::Left => Some(MouseButton::Left),
        winit::event::MouseButton::Middle => Some(MouseButton::Middle),
        winit::event::MouseButton::Right => Some(MouseButton::Right),
        _ => None,
    }
}

impl App {
    /// Dispatch a mouse event to the active pane's terminal for DEC
    /// Locator observation only.
    ///
    /// Called from the additive locator-dispatch path in
    /// [`handle_mouse_input`](crate::app::App::handle_mouse_input) /
    /// motion / wheel handlers when DEC Locator reporting is active but no
    /// mouse-tracking mode is. `Term::observe_locator_input` records the
    /// position (Step A) and NEVER emits an SGR/X10 mouse report — only
    /// updates the position a subsequent DECRQLP reads.
    ///
    /// Uses UNCLAMPED [`mouse_cell`](App::mouse_cell) for the `in_grid`
    /// flag (out-of-grid → Term stores `Unavailable` → DECRQLP Pe=0) and
    /// clamped [`mouse_cell_clamped`](App::mouse_cell_clamped) for the
    /// stored cell coords. `physical_px` carries DEVICE pixels (DECELR
    /// Pu=1) — the SAME grid-relative physical coordinate the SGR-Pixel
    /// encoder now reports.
    pub(in crate::app) fn report_locator_observation(
        &mut self,
        button: MouseButton,
        kind: MouseEventKind,
    ) {
        let Some(pane_id) = self.active_pane_id() else {
            return;
        };
        // Unclamped grid membership: None (out of grid) → Unavailable.
        let in_grid = self.mouse_cell().is_some();
        // Clamped cell for the stored Known position (edge-clamped when the
        // cursor is in the grid region but past the last cell); irrelevant
        // when `!in_grid` (Term stores Unavailable regardless).
        let (col, line) = self.mouse_cell_clamped().unwrap_or((0, 0));
        let event = MouseEvent {
            button,
            kind,
            col,
            line,
            mods: self.mouse_modifiers(),
            px: None,
            py: None,
            physical_px: self.mouse_physical_px_opt(),
            in_grid,
        };
        // Observe-only dispatch: routes to `Term::observe_locator_input`
        // (Step A), NEVER the encoder. Safe on the shift-bypass path where
        // ANY_MOUSE is set but mouse-tracking is suppressed.
        self.observe_pane_locator_input(pane_id, &event);
    }

    /// Additive DEC Locator position observation via the observe-only path.
    ///
    /// Records the current cursor position for locator tracking when
    /// locator reporting is active AND the mouse-tracking encoder did NOT
    /// already dispatch this event (`!encoder_dispatched`) — when it did,
    /// Term's Step A observed in that same dispatch, so re-observing would
    /// be redundant. Routes through `report_locator_observation`
    /// (observe-only), so it NEVER fires the encoder even when `ANY_MOUSE`
    /// is set (the shift-to-select bypass path).
    ///
    /// Shared by the motion handler (passes whether `report_mouse_motion`
    /// dispatched) and the wheel handler (passes whether the Tier-1 encoder
    /// path ran). A wheel scroll does not move the cursor, so on the wheel
    /// path this re-records the current position (`Pm` unchanged: wheel
    /// carries no Pm bit; `button_mask_for_event` returns `prior` for both
    /// `None`/`Motion` and `ScrollUp`/`Press`), so one helper serves both.
    pub(in crate::app) fn maybe_observe_locator_position(&mut self, encoder_dispatched: bool) {
        if !encoder_dispatched && self.dec_locator_active() {
            self.report_locator_observation(MouseButton::None, MouseEventKind::Motion);
        }
    }

    /// DEVICE physical cursor pixels as a single `Option<(u32, u32)>` for
    /// the `MouseEvent.physical_px` field (DEC Locator Pu=1) — both-present
    /// → `Some((x, y))`, otherwise `None`.
    ///
    /// Per xterm `button.c:785-807`, DECELR Pu=1 reports raw DEVICE pixels.
    /// The SGR-Pixel encoder px/py is now ALSO physical, so this
    /// and `mouse_pixel_coords` are the SAME grid-relative device coordinate
    /// — both read the single shared [`clamped_cursor_px`](App::clamped_cursor_px)
    /// source (no parallel physical-pixel computation).
    pub(super) fn mouse_physical_px_opt(&self) -> Option<(u32, u32)> {
        match self.clamped_cursor_px() {
            (Some(x), Some(y)) => Some((x, y)),
            _ => None,
        }
    }
}
