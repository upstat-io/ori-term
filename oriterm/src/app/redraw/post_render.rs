//! Post-render cleanup: surface error recovery, blink state, IME.

use super::App;
use crate::gpu::SurfaceError;

impl App {
    /// Finalize a render pass: recover from surface errors, update blink
    /// state, and reposition the IME candidate window.
    ///
    /// Called by both the single-pane and multi-pane redraw paths after
    /// GPU submission completes. `cursor_pos` is `Some` only on the
    /// single-pane path where cursor-move detection resets the blink timer.
    pub(super) fn finish_render(
        &mut self,
        render_result: Result<(), SurfaceError>,
        blinking_now: bool,
        cursor_pos: Option<(usize, usize)>,
    ) {
        // Surface error recovery.
        match render_result {
            Ok(()) => log::trace!("render ok"),
            Err(SurfaceError::Lost) => {
                log::warn!("surface lost, reconfiguring");
                let Some(gpu) = self.gpu.as_ref() else { return };
                if let Some(ctx) = self
                    .focused_window_id
                    .and_then(|id| self.windows.get_mut(&id))
                {
                    let (w, h) = ctx.window.size_px();
                    ctx.window.resize_surface(w, h, gpu);
                    ctx.window.apply_pending_surface_resize(gpu);
                }
            }
            Err(e) => log::error!("render error: {e}"),
        }

        // Reset blink to visible when the cursor moves (PTY output moved it).
        if let Some(pos) = cursor_pos {
            if pos != self.last_cursor_pos {
                self.last_cursor_pos = pos;
                self.reset_cursor_blink();
            }
        }

        // Blink state transition: reset on off→on edge.
        if blinking_now && !self.blinking_active {
            self.reset_cursor_blink();
        }
        self.blinking_active = self.config.terminal.cursor_blink && blinking_now;

        // Keep the IME candidate window positioned at the terminal cursor.
        self.update_ime_cursor_area();
    }
}
