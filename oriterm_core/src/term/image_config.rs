//! Image protocol configuration and animation management.
//!
//! Extracted from `term/mod.rs` to keep the main file under the 500-line
//! limit. These methods configure image protocol behavior, cache limits,
//! cell dimensions, and advance animation frames.

use crate::event::EventListener;
use crate::grid::StableRowIndex;

use super::Term;

impl<T: EventListener> Term<T> {
    /// Set cell pixel dimensions (called by GUI after font metrics are known).
    ///
    /// Also recalculates cell coverage for fixed-pixel image placements
    /// so viewport intersection queries remain correct.
    pub fn set_cell_dimensions(&mut self, width: u16, height: u16) {
        self.cell_pixel_width = width;
        self.cell_pixel_height = height;
        self.image_cache.update_cell_coverage(width, height);
        if let Some(cache) = &mut self.alt_image_cache {
            cache.update_cell_coverage(width, height);
        }
    }

    /// Whether image protocols are enabled.
    pub fn image_protocol_enabled(&self) -> bool {
        self.image_protocol_enabled
    }

    /// Enable or disable image protocol handling (Kitty, Sixel, iTerm2).
    ///
    /// When disabled, all image protocol sequences are silently ignored.
    pub fn set_image_protocol_enabled(&mut self, enabled: bool) {
        self.image_protocol_enabled = enabled;
    }

    /// Apply image cache limits from config.
    ///
    /// If the new limit is lower than current usage, triggers immediate
    /// LRU eviction.
    pub fn set_image_limits(&mut self, memory_limit: usize, max_single: usize) {
        self.image_cache.set_memory_limit(memory_limit);
        self.image_cache.set_max_single_image(max_single);
        if let Some(cache) = &mut self.alt_image_cache {
            cache.set_memory_limit(memory_limit);
            cache.set_max_single_image(max_single);
        }
    }

    /// Enable or disable image animation.
    ///
    /// When disabled, animated images show the first frame only.
    pub fn set_image_animation_enabled(&mut self, enabled: bool) {
        self.image_cache.set_animation_enabled(enabled);
        if let Some(cache) = &mut self.alt_image_cache {
            cache.set_animation_enabled(enabled);
        }
    }

    /// Advance image animations for the active screen.
    ///
    /// Returns the next frame deadline so the event loop can schedule
    /// a redraw. Call once per frame before extracting renderable content.
    pub fn advance_animations(&mut self, now: std::time::Instant) -> Option<std::time::Instant> {
        let grid = self.grid();
        let offset = grid.display_offset().min(grid.scrollback().len());
        let lines = grid.lines();
        let base =
            grid.total_evicted() as u64 + grid.scrollback().len().saturating_sub(offset) as u64;
        let top = StableRowIndex(base);
        let bottom = StableRowIndex(base + lines.saturating_sub(1) as u64);

        self.image_cache_mut().advance_animations(now, top, bottom)
    }
}
