//! Alt screen swap operations.
//!
//! Modes 47 (legacy), 1047 (clear on enter), and 1049 (save/restore cursor)
//! each use a different swap variant. All toggle `ALT_SCREEN`, swap keyboard
//! mode stacks, and mark all lines dirty.
//!
//! The alt grid is lazily allocated on first entry — most terminals never
//! enter alt screen (only editors, pagers, etc.), so this avoids wasting
//! memory.

use crate::event::EventListener;
use crate::grid::Grid;
use crate::image::ImageCache;

use super::{Term, TermMode};

impl<T: EventListener> Term<T> {
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

    /// Common alt screen toggle: flip flag, swap keyboard stacks, swap
    /// image caches, mark dirty.
    fn toggle_alt_common(&mut self) {
        self.mode.toggle(TermMode::ALT_SCREEN);
        std::mem::swap(
            &mut self.keyboard_mode_stack,
            &mut self.inactive_keyboard_mode_stack,
        );
        // Swap image caches. Alt cache is guaranteed Some after ensure_alt_grid.
        let alt_cache = self.alt_image_cache.take().unwrap();
        let primary_cache = std::mem::replace(&mut self.image_cache, alt_cache);
        self.alt_image_cache = Some(primary_cache);
        self.grid_mut().dirty_mut().mark_all();
    }
}
