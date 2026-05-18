//! Kitty graphics `a=d` (delete) action — per-specifier delete routing.
//!
//! Dispatches every `d=<spec>` value to the correct `ImageCache` primitive
//! per kitty graphics-protocol.rst §Deleting images. Lowercase variants
//! delete placements only; uppercase variants additionally prune image
//! data when the image has no remaining placements.

use log::debug;

use crate::effect::sink::EffectSink;
use crate::grid::StableRowIndex;
use crate::image::ImageId;
use crate::image::kitty::KittyCommand;
use crate::term::Term;

/// Whether the delete specifier is an uppercase letter. Uppercase variants
/// free image data; lowercase keeps it.
fn is_uppercase(spec: u8) -> bool {
    spec.is_ascii_uppercase()
}

/// Convert a 1-based kitty `x=` to a 0-based column, saturating at 0.
fn col_from_spec(x: u32) -> usize {
    x.saturating_sub(1) as usize
}

/// Cell coordinates derived from `cmd.source_x` / `cmd.source_y` (1-based).
struct SpecCell {
    col: usize,
    row: StableRowIndex,
}

impl<S: EffectSink> Term<S> {
    /// Delete images/placements based on the `d=` specifier.
    pub(super) fn kitty_delete(&mut self, cmd: &KittyCommand) {
        // Kitty graphics.c:2093 — abort any in-flight chunked upload
        // BEFORE dispatching the delete arm, so a late `m=0` chunk cannot
        // resurrect an image whose placements/data this command will drop.
        self.loading_image = None;

        let spec = cmd.delete_specifier.unwrap_or(b'a');
        let before_pls = self.image_cache().placement_count();

        let grid = self.grid();
        let viewport_top = StableRowIndex::from_visible(grid, 0);
        let viewport_bottom = StableRowIndex::from_visible(grid, grid.lines().saturating_sub(1));
        let cursor = grid.cursor();
        let cursor_col = cursor.col().0;
        let cursor_row = StableRowIndex::from_visible(grid, cursor.line());
        let spec_cell = SpecCell {
            col: col_from_spec(cmd.source_x),
            row: StableRowIndex::from_visible(grid, cmd.source_y.saturating_sub(1) as usize),
        };

        match spec {
            b'a' | b'A' => self.kitty_delete_visible(spec, viewport_top, viewport_bottom),
            b'i' | b'I' => self.kitty_delete_by_image_id(spec, cmd),
            b'p' | b'P' => self.kitty_delete_at_cell(spec, &spec_cell),
            b'c' | b'C' => {
                self.kitty_delete_at_position(spec, cursor_col, cursor_row);
            }
            b'x' | b'X' => self.kitty_delete_at_column(spec, spec_cell.col),
            b'y' | b'Y' => self.kitty_delete_at_row(spec, spec_cell.row),
            b'z' | b'Z' => self.kitty_delete_at_z(spec, cmd.z_index),
            b'r' | b'R' => self.kitty_delete_id_range(spec, cmd),
            b'q' | b'Q' => self.kitty_delete_at_cell_with_z(spec, &spec_cell, cmd.z_index),
            b'n' | b'N' => self.kitty_delete_by_image_number(spec, cmd),
            b'f' | b'F' => self.kitty_delete_frame(spec, cmd),
            _ => debug!("kitty delete specifier {:?} not implemented", spec as char),
        }

        let after_pls = self.image_cache().placement_count();
        // High-frequency on animated graphics (notcurses re-transmits +
        // deletes placements per frame). Kept at debug so default INFO
        // doesn't pay sync-write cost on the IO thread per frame.
        log::debug!(
            target: "oriterm_core::term::handler::image::kitty::delete",
            "kitty delete: d={} — placements {}->{}",
            spec as char,
            before_pls,
            after_pls,
        );
    }

    fn kitty_delete_visible(
        &mut self,
        spec: u8,
        viewport_top: StableRowIndex,
        viewport_bottom: StableRowIndex,
    ) {
        let ids = self
            .image_cache_mut()
            .remove_visible_placements(viewport_top, viewport_bottom);
        if is_uppercase(spec) {
            self.image_cache_mut().prune_if_orphaned(&ids);
        }
    }

    fn kitty_delete_by_image_id(&mut self, spec: u8, cmd: &KittyCommand) {
        let Some(id) = cmd.image_id else { return };
        let image_id = ImageId::from_raw(id);
        let cache = self.image_cache_mut();
        if let Some(pid) = cmd.placement_id {
            cache.remove_placement(image_id, pid);
        } else {
            cache.remove_placements_for_image(image_id);
        }
        // Uppercase `d=I` is targeted by image ID — remove the image and
        // its anchor unconditionally, bypassing the orphan check. The
        // orphan-check path is for region/visibility-targeted variants
        // (`d=p/c/q/x/y/z/a/A`) where anchored U=1 images must survive.
        if is_uppercase(spec) {
            cache.remove_image(image_id);
        }
    }

    fn kitty_delete_at_cell(&mut self, spec: u8, cell: &SpecCell) {
        self.kitty_delete_at_position(spec, cell.col, cell.row);
    }

    fn kitty_delete_at_position(&mut self, spec: u8, col: usize, row: StableRowIndex) {
        let ids = self
            .image_cache_mut()
            .remove_placements_intersecting_cell(col, row);
        if is_uppercase(spec) {
            self.image_cache_mut().prune_if_orphaned(&ids);
        }
    }

    fn kitty_delete_at_column(&mut self, spec: u8, col: usize) {
        let ids = self.image_cache_mut().remove_placements_at_column(col);
        if is_uppercase(spec) {
            self.image_cache_mut().prune_if_orphaned(&ids);
        }
    }

    fn kitty_delete_at_row(&mut self, spec: u8, row: StableRowIndex) {
        let ids = self.image_cache_mut().remove_placements_at_row(row);
        if is_uppercase(spec) {
            self.image_cache_mut().prune_if_orphaned(&ids);
        }
    }

    fn kitty_delete_at_z(&mut self, spec: u8, z: i32) {
        let ids = self.image_cache_mut().remove_placements_by_z_index(z);
        if is_uppercase(spec) {
            self.image_cache_mut().prune_if_orphaned(&ids);
        }
    }

    fn kitty_delete_id_range(&mut self, spec: u8, cmd: &KittyCommand) {
        let lo = ImageId::from_raw(cmd.source_x);
        let hi = ImageId::from_raw(cmd.source_y);
        let ids = self.image_cache_mut().remove_placements_in_id_range(lo, hi);
        // ID-targeted: uppercase removes image data unconditionally
        // (anchored U=1 images do NOT survive a targeted `d=R`).
        if is_uppercase(spec) {
            let cache = self.image_cache_mut();
            for id in ids {
                cache.remove_image(id);
            }
        }
    }

    fn kitty_delete_at_cell_with_z(&mut self, spec: u8, cell: &SpecCell, z: i32) {
        let ids = self
            .image_cache_mut()
            .remove_placements_at_cell_with_z(cell.col, cell.row, z);
        if is_uppercase(spec) {
            self.image_cache_mut().prune_if_orphaned(&ids);
        }
    }

    fn kitty_delete_by_image_number(&mut self, spec: u8, cmd: &KittyCommand) {
        let Some(number) = cmd.image_number else {
            return;
        };
        let Some(image_id) = self.image_cache().newest_by_image_number(number) else {
            return;
        };
        let cache = self.image_cache_mut();
        if let Some(pid) = cmd.placement_id {
            cache.remove_placement(image_id, pid);
        } else {
            cache.remove_placements_for_image(image_id);
        }
        // Image-number is an ID-targeted alias — uppercase removes image
        // data unconditionally so anchored U=1 images do NOT survive.
        if is_uppercase(spec) {
            cache.remove_image(image_id);
        }
    }

    fn kitty_delete_frame(&mut self, spec: u8, cmd: &KittyCommand) {
        // Kitty graphics.c:1685-1689 — d=f/F accepts `i=` OR `I=`; when
        // only `I=` is given, resolve to the newest image with that number.
        let image_id = match cmd.image_id {
            Some(id) => ImageId::from_raw(id),
            None => match cmd
                .image_number
                .and_then(|n| self.image_cache().newest_by_image_number(n))
            {
                Some(id) => id,
                None => return,
            },
        };
        if self.image_cache().get_no_touch(image_id).is_none() {
            return;
        }
        if self.image_cache().has_extra_animation_frames(image_id) {
            let frame_number = cmd.display_rows.unwrap_or(1);
            let now_static = self
                .image_cache_mut()
                .remove_animation_frame(image_id, frame_number);
            if is_uppercase(spec) && now_static {
                self.image_cache_mut().remove_image(image_id);
            }
        } else {
            // Static image: lowercase is a no-op; uppercase removes the image.
            if is_uppercase(spec) {
                self.image_cache_mut().remove_image(image_id);
            }
        }
    }
}

#[cfg(test)]
mod tests;
