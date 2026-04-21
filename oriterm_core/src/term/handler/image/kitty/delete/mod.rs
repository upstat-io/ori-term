//! Kitty graphics `a=d` (delete) action — per-specifier delete routing.

use log::debug;

use crate::effect::sink::EffectSink;
use crate::grid::StableRowIndex;
use crate::image::ImageId;
use crate::image::kitty::KittyCommand;
use crate::term::Term;

impl<S: EffectSink> Term<S> {
    /// Delete images/placements based on the delete specifier.
    pub(super) fn kitty_delete(&mut self, cmd: &KittyCommand) {
        let spec = cmd.delete_specifier.unwrap_or(b'a');
        let before_pls = self.image_cache().placement_count();

        // Extract positions before borrowing cache mutably.
        let grid = self.grid();
        let cursor = grid.cursor();
        let cursor_col = cursor.col().0;
        let cursor_line = cursor.line();
        let cursor_row = StableRowIndex::from_visible(grid, cursor_line);
        // Protocol y for d=y/Y is 1-based viewport row → convert to stable.
        let protocol_row =
            StableRowIndex::from_visible(grid, cmd.source_y.saturating_sub(1) as usize);

        let cache = self.image_cache_mut();

        match spec {
            // Lowercase: delete placements only.
            // Uppercase: delete image data + all placements.
            b'a' | b'A' => cache.clear(),
            b'i' => {
                if let Some(id) = cmd.image_id {
                    cache.remove_placements_for_image(ImageId(id));
                }
            }
            b'I' => {
                if let Some(id) = cmd.image_id {
                    cache.remove_image(ImageId(id));
                }
            }
            b'p' => {
                if let Some(pid) = cmd.placement_id {
                    if let Some(id) = cmd.image_id {
                        cache.remove_placement(ImageId(id), pid);
                    }
                }
            }
            b'P' => {
                if let Some(pid) = cmd.placement_id {
                    if let Some(id) = cmd.image_id {
                        cache.remove_placement(ImageId(id), pid);
                        cache.remove_image(ImageId(id));
                    }
                }
            }
            b'c' => drop(cache.remove_placements_at_column(cursor_col)),
            b'C' => {
                let ids = cache.remove_placements_at_column(cursor_col);
                cache.prune_if_orphaned(&ids);
            }
            b'x' => drop(cache.remove_placements_at_column(cmd.source_x as usize)),
            b'X' => {
                let col = cmd.source_x as usize;
                let ids = cache.remove_placements_at_column(col);
                cache.prune_if_orphaned(&ids);
            }
            b'y' => drop(cache.remove_placements_at_row(protocol_row)),
            b'Y' => {
                let ids = cache.remove_placements_at_row(protocol_row);
                cache.prune_if_orphaned(&ids);
            }
            b'z' => drop(cache.remove_placements_by_z_index(cmd.z_index)),
            b'Z' => {
                let z = cmd.z_index;
                let ids = cache.remove_placements_by_z_index(z);
                cache.prune_if_orphaned(&ids);
            }
            b'r' | b'R' => {
                let ids = cache.remove_by_position(cursor_col, cursor_row);
                if spec == b'R' {
                    cache.prune_if_orphaned(&ids);
                }
            }
            b'n' => debug!("kitty delete d=n not yet implemented"),
            b'N' => debug!("kitty delete d=N not yet implemented"),
            _ => debug!("kitty delete specifier {:?} not implemented", spec as char),
        }

        let after_pls = self.image_cache().placement_count();
        log::info!(
            "kitty delete: d={} — placements {}->{}",
            spec as char,
            before_pls,
            after_pls,
        );
    }
}
