//! Alt screen swap operations.
//!
//! Modes 47 (legacy), 1047 (clear on enter), and 1049 (save/restore cursor)
//! each use a different swap variant. All toggle `ALT_SCREEN`, swap keyboard
//! mode stacks, and mark all lines dirty.
//!
//! The alt grid is lazily allocated on first entry — most terminals never
//! enter alt screen (only editors, pagers, etc.), so this avoids wasting
//! memory.

use vte::ansi::{KeyboardModes, KeyboardModesApplyBehavior};

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
    /// When entering alt screen: clears the alt grid AND the alt image
    /// cache, then swaps. Does NOT save or restore the cursor position.
    ///
    /// The alt image cache must clear alongside the grid — DECSET 1047/1049
    /// semantics say "clear the alt screen on enter", and alt-screen
    /// placements live outside the grid. Leaving the cache intact would
    /// resurrect kitty/sixel placements on re-entry.
    pub fn swap_alt_clear(&mut self) {
        self.selection_dirty = true;
        self.ensure_alt_grid();
        // Clear the alt grid AND alt image cache before entering.
        self.alt_grid.as_mut().unwrap().reset();
        if let Some(cache) = self.alt_image_cache.as_mut() {
            cache.clear();
        }
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
        // Swap the paired pre-command stack snapshots alongside the
        // stacks so a command-boundary snapshot taken on one screen
        // only fires restore on that screen. BUG-08-12.
        std::mem::swap(
            &mut self.pre_command_kb_stack_snapshot,
            &mut self.inactive_pre_command_kb_stack_snapshot,
        );
        // Paired bits snapshot swaps alongside the stack snapshot so
        // the snapshot travels with its owning screen — without this
        // swap, an alt-side `;A` restore applied primary's bits (wrong
        // screen). BUG-08-12 TPR round-4.
        std::mem::swap(
            &mut self.pre_command_kb_mode_bits_snapshot,
            &mut self.inactive_pre_command_kb_mode_bits_snapshot,
        );
        // DECSC sidecar state is per-screen (VT220 spec). DECLRMM is
        // NOT in the DECSC save set (see `Grid::save_cursor`), so there
        // is nothing margin-related to swap here — the primary/alt grid
        // split in `Term::{grid, alt_grid}` carries its own margin state.
        std::mem::swap(&mut self.saved_charset, &mut self.inactive_saved_charset);
        std::mem::swap(
            &mut self.saved_origin_mode,
            &mut self.inactive_saved_origin_mode,
        );
        // Swap live per-screen kitty bits: capture the active bits
        // (subset of `self.mode`), install the inactive screen's live
        // bits as the new active, and store the old active in the
        // inactive slot. Covers set-only `CSI = Ps u` state that never
        // enters the stack, and child mid-command mutations that would
        // otherwise be lost by deriving reapply from stack top. See
        // BUG-08-12 TPR round-3 F1/F2.
        let active_bits_before_swap = KeyboardModes::from(self.mode);
        let new_active_bits = self.inactive_keyboard_mode_bits;
        self.inactive_keyboard_mode_bits = active_bits_before_swap;
        self.dcs_set_keyboard_mode(new_active_bits, KeyboardModesApplyBehavior::Replace);
        self.grid_mut().dirty_mut().mark_all();
    }
}

#[cfg(test)]
mod tests;
