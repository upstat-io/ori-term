//! Terminal input controller — claims all input events.
//!
//! Trivial "catch-all" controller that returns `true` for every mouse and
//! keyboard event, preventing them from bubbling past the terminal grid.
//! Actual terminal input dispatch (sending to PTY, updating grid) stays
//! at the app layer.

use oriterm_ui::controllers::{ControllerCtx, EventController};
use oriterm_ui::input::{EventPhase, InputEvent};

/// Claims all mouse and keyboard events for the terminal grid.
///
/// The terminal grid is a sink: no input event should escape to parent
/// widgets. The app layer handles actual terminal dispatch.
// Used in Section 08.3 Wave 4 migration.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct TerminalInputController;

impl TerminalInputController {
    /// Creates a new terminal input controller.
    #[allow(dead_code)]
    pub(crate) fn new() -> Self {
        Self
    }
}

impl EventController for TerminalInputController {
    fn phase(&self) -> EventPhase {
        EventPhase::Target
    }

    fn handle_event(&mut self, event: &InputEvent, _ctx: &mut ControllerCtx<'_>) -> bool {
        // Claim all input events — the app layer dispatches to the PTY.
        matches!(
            event,
            InputEvent::MouseDown { .. }
                | InputEvent::MouseUp { .. }
                | InputEvent::MouseMove { .. }
                | InputEvent::KeyDown { .. }
                | InputEvent::KeyUp { .. }
                | InputEvent::Scroll { .. }
        )
    }
}
