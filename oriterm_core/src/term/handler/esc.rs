//! ESC sequence handler implementations.
//!
//! Handles RIS (full reset) and DECALN (screen alignment test).
//! Methods are called by the `vte::ansi::Handler` trait impl on `Term<T>`.

use log::debug;

use crate::cell::Cell;
use crate::event::{Event, EventListener};
use crate::index::{Column, Line};

use super::super::{CharsetState, PromptState, Term, TermMode};

impl<T: EventListener> Term<T> {
    /// RIS (ESC c): full terminal reset.
    ///
    /// Resets both grids, mode flags, charset, palette, title, cursor shape,
    /// and keyboard mode stacks to initial state.
    pub(super) fn esc_reset_state(&mut self) {
        debug!("RIS: full terminal reset");

        self.selection_dirty = true;

        // Clear alt-screen flag without swapping — both grids are reset
        // immediately after, so cursor save/restore and dirty marking from
        // swap_alt() would be wasted work.
        self.mode.remove(TermMode::ALT_SCREEN);

        self.grid_mut().reset();
        if let Some(alt) = &mut self.alt_grid {
            alt.reset();
        }
        self.mode = TermMode::default();
        self.charset = CharsetState::default();
        self.palette = crate::color::Palette::for_theme(self.theme);
        self.cursor_shape = crate::grid::CursorShape::default();
        self.title.clear();
        self.icon_name.clear();
        self.title_stack.clear();
        self.cwd = None;
        self.keyboard_mode_stack.clear();
        self.inactive_keyboard_mode_stack.clear();

        // Shell integration state.
        self.prompt_state = PromptState::None;
        self.pending_marks = crate::term::PendingMarks::empty();
        self.prompt_markers.clear();
        self.pending_notifications.clear();
        self.command_start = None;
        self.last_command_duration = None;
        self.has_explicit_title = false;
        self.title_dirty = true;
        self.saved_private_modes.clear();

        // Image caches.
        self.image_cache.clear();
        if let Some(cache) = &mut self.alt_image_cache {
            cache.clear();
        }

        self.event_listener.send_event(Event::ResetTitle);
    }

    /// DECALN (ESC # 8): DEC Screen Alignment Test.
    ///
    /// Fills all visible cells with 'E' (default attributes), resets
    /// the scroll region to the full screen, and homes the cursor.
    pub(super) fn decaln_impl(&mut self) {
        let grid = self.grid_mut();
        let lines = grid.lines();
        let cols = grid.cols();

        // Reset scroll region to full screen.
        grid.set_scroll_region(1, None);

        // Fill every visible cell with 'E' and default attributes.
        let template = Cell::default();
        for line in 0..lines {
            for col in 0..cols {
                let cell = &mut grid[Line(line as i32)][Column(col)];
                cell.reset(&template);
                cell.ch = 'E';
            }
        }

        // Mark all lines dirty.
        grid.dirty_mut().mark_all();

        // Home the cursor.
        self.goto_origin_aware(0, 0);
    }
}
