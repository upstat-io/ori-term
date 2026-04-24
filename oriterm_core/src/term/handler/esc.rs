//! ESC sequence handler implementations.
//!
//! Handles RIS (full reset), DECALN (screen alignment test), DECSC/DECRC
//! (cursor save/restore), and DECSLRM (left/right margin set).
//! Methods are called by the `vte::ansi::Handler` trait impl on `Term<S>`.

use std::collections::VecDeque;

use log::debug;
use vte::ansi::KeyboardModes;

use crate::cell::{Cell, CellFlags};
use crate::effect::sink::EffectSink;
use crate::effect::{Effect, HostEffect};
use crate::index::{Column, Line};

use super::super::{CharsetState, PromptState, Term, TermMode};

impl<S: EffectSink> Term<S> {
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
        self.saved_charset = None;
        self.saved_origin_mode = None;
        self.inactive_saved_charset = None;
        self.inactive_saved_origin_mode = None;
        self.palette = crate::color::Palette::for_theme(self.theme);
        self.cursor_shape = crate::grid::CursorShape::default();
        self.title.clear();
        self.icon_name.clear();
        self.title_stack.clear();
        self.cwd = None;
        self.keyboard_mode_stack.clear();
        self.inactive_keyboard_mode_stack.clear();
        // Seed paired snapshots to `Some(empty)` (not `None`) so that if a
        // kitty-aware child later pushes keyboard modes and crashes before
        // popping, the next OSC 133 ; A / ; D still restores to an empty
        // stack rather than leaving child-pushed modes active. Bits
        // snapshot matches: `Some(NO_MODE)` means "restore to no kitty
        // bits set". Inactive-screen live bits reset to NO_MODE. BUG-08-12.
        self.pre_command_kb_stack_snapshot = Some(VecDeque::new());
        self.inactive_pre_command_kb_stack_snapshot = Some(VecDeque::new());
        self.pre_command_kb_mode_bits_snapshot = Some(KeyboardModes::NO_MODE);
        self.inactive_keyboard_mode_bits = KeyboardModes::NO_MODE;

        // Shell integration state.
        self.prompt_state = PromptState::None;
        self.pending_marks = crate::term::PendingMarks::empty();
        self.prompt_markers.clear();
        self.effect_sink
            .push(Effect::Host(HostEffect::ClearPendingNotifications));
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

        self.effect_sink
            .push(Effect::Host(HostEffect::TitleSet { value: None }));

        // §09A.8 presentation state.
        self.char_protection = false;
        self.conformance_level = 64;
        self.c1_7bit = true;
        self.active_status_display = 0;
        self.status_line_type = 0;
        // §09A.6: DECSACE attribute-change extent mode.
        self.ace_mode = crate::term::AceMode::default();
    }

    /// DECSTR (CSI ! p): Soft Terminal Reset.
    ///
    /// Resets modes, SGR attributes, cursor position, scroll region,
    /// tab stops, and charsets. Does NOT clear screen contents,
    /// scrollback, title, palette, or image caches — that is RIS (ESC c).
    pub(super) fn soft_reset(&mut self) {
        debug!("DECSTR: soft terminal reset");

        self.selection_dirty = true;

        // Reset state on BOTH screens. `self.mode = TermMode::default()`
        // below drops ALT_SCREEN, so the primary grid becomes active on
        // return — it must be reset regardless of which screen was active
        // when DECSTR fired. Per WezTerm (terminalstate/mod.rs:1273-1276).
        Self::soft_reset_grid(&mut self.grid);
        if let Some(alt) = &mut self.alt_grid {
            Self::soft_reset_grid(alt);
        }

        self.mode = TermMode::default();
        self.charset = CharsetState::default();
        self.saved_charset = None;
        self.saved_origin_mode = None;
        self.inactive_saved_charset = None;
        self.inactive_saved_origin_mode = None;
        self.cursor_shape = crate::grid::CursorShape::default();
        self.keyboard_mode_stack.clear();
        self.inactive_keyboard_mode_stack.clear();
        // Same paired-snapshot seeding as RIS — see BUG-08-12.
        self.pre_command_kb_stack_snapshot = Some(VecDeque::new());
        self.inactive_pre_command_kb_stack_snapshot = Some(VecDeque::new());
        self.pre_command_kb_mode_bits_snapshot = Some(KeyboardModes::NO_MODE);
        self.inactive_keyboard_mode_bits = KeyboardModes::NO_MODE;
        // DECSTR clears the DECSCA protection state — cursor template
        // is reset above via `soft_reset_grid`, but `char_protection`
        // mirrors the same state on Term and must follow suit.
        self.char_protection = false;
    }

    /// Reset SGR-relevant state on a single grid (DECSTR scope).
    ///
    /// Resets cursor position, template (SGR), scroll region, margins,
    /// saved cursor, and SGR stack. Does NOT touch screen contents.
    fn soft_reset_grid(grid: &mut crate::grid::Grid) {
        grid.cursor_mut().template = Cell::default();
        grid.cursor_mut().set_line(0);
        grid.cursor_mut().set_col(Column(0));
        grid.set_scroll_region(0, None);
        let cols = grid.cols();
        grid.set_left_right_margins(0, cols.saturating_sub(1));
        grid.clear_saved_cursor();
        grid.clear_sgr_stack();
    }

    /// DECALN (ESC # 8): DEC Screen Alignment Test.
    ///
    /// Fills all visible cells with 'E' (default attributes), resets
    /// the scroll region to the full screen, and homes the cursor.
    pub(super) fn decaln_impl(&mut self) {
        let grid = self.grid_mut();
        let lines = grid.lines();
        let cols = grid.cols();

        // Reset scroll region and horizontal margins to full screen.
        grid.set_scroll_region(1, None);
        grid.reset_left_right_margins();

        // Fill every visible cell with 'E' and default attributes.
        // DECALN cells are application-written per xterm semantics —
        // set DRAWN so DECRQCRA treats them as drawn (BUG-08-17).
        let template = Cell::default();
        for line in 0..lines {
            for col in 0..cols {
                let cell = &mut grid[Line(line as i32)][Column(col)];
                cell.reset(&template);
                cell.ch = 'E';
                cell.flags.insert(CellFlags::DRAWN);
            }
        }

        // Mark all lines dirty.
        grid.dirty_mut().mark_all();

        // Home the cursor.
        self.goto_origin_aware(0, 0);
    }

    /// DECSC (ESC 7): save cursor position + attributes + charset + wrap + DECOM.
    ///
    /// Per DEC STD 070 §5.6.1, the save set includes cursor position,
    /// character attributes, charset state, wrap flag, and DECOM flag.
    /// DECLRMM mode and margin values are NOT saved.
    pub(super) fn save_cursor_impl(&mut self) {
        self.grid_mut().save_cursor();
        self.saved_charset = Some(self.charset.clone());
        self.saved_origin_mode = Some(self.mode.contains(TermMode::ORIGIN));
    }

    /// DECRC (ESC 8): restore cursor position + attributes + charset + DECOM.
    pub(super) fn restore_cursor_impl(&mut self) {
        self.grid_mut().restore_cursor();
        if let Some(charset) = self.saved_charset.take() {
            self.charset = charset;
            self.saved_charset = Some(self.charset.clone());
        }
        if let Some(origin) = self.saved_origin_mode {
            if origin {
                self.mode.insert(TermMode::ORIGIN);
            } else {
                self.mode.remove(TermMode::ORIGIN);
            }
        }
    }

    /// CSI s: DECSLRM (if mode 69 active) or save cursor (backward compat).
    pub(super) fn decslrm_or_save_cursor_impl(&mut self, has_params: bool, left: u16, right: u16) {
        if has_params {
            if self.mode.contains(TermMode::LEFT_RIGHT_MARGIN) {
                let cols = self.grid().cols();
                let l = (left.max(1) as usize).saturating_sub(1);
                let r = if right == 0 {
                    cols.saturating_sub(1)
                } else {
                    (right as usize).saturating_sub(1)
                };
                self.grid_mut().set_left_right_margins(l, r);
            }
        } else if self.mode.contains(TermMode::LEFT_RIGHT_MARGIN) {
            self.grid_mut().reset_left_right_margins();
        } else {
            self.save_cursor_impl();
        }
    }
}
