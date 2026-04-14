//! Alt screen swap operations.
//!
//! Modes 47 (legacy), 1047 (clear on enter), and 1049 (save/restore cursor)
//! each use a different swap variant. All toggle `ALT_SCREEN`, swap keyboard
//! mode stacks, and mark all lines dirty.
//!
//! The alt grid is lazily allocated on first entry — most terminals never
//! enter alt screen (only editors, pagers, etc.), so this avoids wasting
//! memory.

use crate::effect::sink::EffectSink;
use crate::grid::Grid;
use crate::image::ImageCache;

use super::{Term, TermMode};

impl<S: EffectSink> Term<S> {
    /// Switch between primary and alternate screen (mode 1049).
    ///
    /// Saves/restores cursor, toggles `TermMode::ALT_SCREEN`, swaps keyboard
    /// mode stacks, and marks all lines dirty. Also marks selection as dirty
    /// since screen content changes completely.
    pub fn swap_alt(&mut self) {
        self.selection_dirty = true;
        self.ensure_alt_grid();
        if self.mode.contains(TermMode::ALT_SCREEN) {
            // Switching back to primary: save alt cursor, restore primary cursor.
            self.alt_grid.as_mut().unwrap().save_cursor();
            self.grid.restore_cursor();
        } else {
            // Switching to alt: save primary cursor, restore alt cursor.
            self.grid.save_cursor();
            self.alt_grid.as_mut().unwrap().restore_cursor();
        }

        self.toggle_alt_common();
    }

    /// Switch alt screen without saving/restoring cursor (mode 47).
    ///
    /// Toggles `ALT_SCREEN`, swaps keyboard mode stacks, and marks all
    /// lines dirty. Does NOT save or restore the cursor position.
    pub fn swap_alt_no_cursor(&mut self) {
        self.selection_dirty = true;
        self.ensure_alt_grid();
        self.toggle_alt_common();
    }

    /// Switch to alt screen, clearing it on enter (mode 1047).
    ///
    /// When entering alt screen: clears the alt grid, then swaps.
    /// Does NOT save or restore the cursor position.
    pub fn swap_alt_clear(&mut self) {
        self.selection_dirty = true;
        self.ensure_alt_grid();
        // Clear the alt grid before entering.
        self.alt_grid.as_mut().unwrap().reset();
        self.toggle_alt_common();
    }

    /// Lazily allocate the alt grid and image cache on first use.
    fn ensure_alt_grid(&mut self) {
        if self.alt_grid.is_none() {
            let lines = self.grid.lines();
            let cols = self.grid.cols();
            self.alt_grid = Some(Grid::with_scrollback(lines, cols, 0));
            self.alt_image_cache = Some(ImageCache::new());
        }
    }

    /// Common alt screen toggle: flip flag, swap keyboard stacks, mark
    /// dirty.
    ///
    /// `grid` / `alt_grid` and `image_cache` / `alt_image_cache` are
    /// NOT swapped — they stay in their semantic fields. `grid()` /
    /// `image_cache()` route by [`TermMode::ALT_SCREEN`] to return the
    /// active screen's state. Historically this function also swapped
    /// the image-cache field contents, but that created a structural
    /// inversion where `self.image_cache` held the alt cache in alt
    /// mode and the primary grid was paired with the wrong cache in
    /// `Term::resize`.
    fn toggle_alt_common(&mut self) {
        self.mode.toggle(TermMode::ALT_SCREEN);
        std::mem::swap(
            &mut self.keyboard_mode_stack,
            &mut self.inactive_keyboard_mode_stack,
        );
        // DECSC sidecar state is per-screen (VT220 spec).
        // `saved_margins` lives on Grid itself (per SSOT — Grid owns its
        // margin state), so it is automatically per-screen via the
        // primary/alt grid split in `Term::{grid, alt_grid}`.
        std::mem::swap(&mut self.saved_charset, &mut self.inactive_saved_charset);
        std::mem::swap(
            &mut self.saved_origin_mode,
            &mut self.inactive_saved_origin_mode,
        );
        std::mem::swap(
            &mut self.saved_left_right_margin_mode,
            &mut self.inactive_saved_left_right_margin_mode,
        );
        self.grid_mut().dirty_mut().mark_all();
    }
}
