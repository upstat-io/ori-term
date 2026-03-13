//! Guillotine bin packer for atlas page allocation.
//!
//! Maintains a list of free rectangles within a fixed-size page. When a glyph
//! is packed, the best-fitting free rectangle is split into two smaller ones
//! along the shorter leftover axis (best-short-side-fit).
//!
//! Reference: Jukka Jylanki, "A Thousand Ways to Pack the Bin" (2010).

/// Axis-aligned rectangle for free-space tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// 2D rectangle bin packer using the Guillotine best-short-side-fit algorithm.
///
/// Maintains a list of free rectangles within a fixed-size page. When a glyph
/// is packed, the best-fitting free rectangle is split into two smaller ones
/// along the shorter leftover axis.
pub(crate) struct RectPacker {
    width: u32,
    height: u32,
    free_rects: Vec<Rect>,
}

impl RectPacker {
    /// Create a packer with one free rect covering the full page.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            free_rects: vec![Rect {
                x: 0,
                y: 0,
                w: width,
                h: height,
            }],
        }
    }

    /// Find space for a rectangle of the given dimensions.
    ///
    /// Returns the top-left position `(x, y)` within the page, or `None`
    /// if no free rectangle can fit the request.
    ///
    /// Uses best-short-side-fit: chooses the free rectangle where the shorter
    /// leftover side after placement is minimized, breaking ties by the longer
    /// leftover side. After placement, the chosen rectangle is split via the
    /// Guillotine method (split along the shorter leftover axis).
    pub fn pack(&mut self, glyph_w: u32, glyph_h: u32) -> Option<(u32, u32)> {
        let mut best_idx = None;
        let mut best_short = u32::MAX;
        let mut best_long = u32::MAX;

        for (i, r) in self.free_rects.iter().enumerate() {
            if r.w >= glyph_w && r.h >= glyph_h {
                let leftover_w = r.w - glyph_w;
                let leftover_h = r.h - glyph_h;
                let short = leftover_w.min(leftover_h);
                let long = leftover_w.max(leftover_h);
                if short < best_short || (short == best_short && long < best_long) {
                    best_idx = Some(i);
                    best_short = short;
                    best_long = long;
                }
            }
        }

        let idx = best_idx?;
        let r = self.free_rects[idx];
        let pos = (r.x, r.y);

        // Guillotine split: remove the chosen rect and add up to two children.
        self.free_rects.swap_remove(idx);
        let leftover_w = r.w - glyph_w;
        let leftover_h = r.h - glyph_h;

        // Split along the shorter leftover axis for better packing.
        if leftover_w < leftover_h {
            // Horizontal split: right strip is glyph_h tall, bottom strip is full width.
            if leftover_w > 0 {
                self.free_rects.push(Rect {
                    x: r.x + glyph_w,
                    y: r.y,
                    w: leftover_w,
                    h: glyph_h,
                });
            }
            if leftover_h > 0 {
                self.free_rects.push(Rect {
                    x: r.x,
                    y: r.y + glyph_h,
                    w: r.w,
                    h: leftover_h,
                });
            }
        } else {
            // Vertical split: bottom strip is glyph_w wide, right strip is full height.
            if leftover_h > 0 {
                self.free_rects.push(Rect {
                    x: r.x,
                    y: r.y + glyph_h,
                    w: glyph_w,
                    h: leftover_h,
                });
            }
            if leftover_w > 0 {
                self.free_rects.push(Rect {
                    x: r.x + glyph_w,
                    y: r.y,
                    w: leftover_w,
                    h: r.h,
                });
            }
        }

        Some(pos)
    }

    /// Total free area remaining on this page (sum of free rectangle areas).
    pub fn free_area(&self) -> u64 {
        self.free_rects
            .iter()
            .map(|r| u64::from(r.w) * u64::from(r.h))
            .sum()
    }

    /// Reset the packer to a single free rectangle covering the full page.
    pub fn reset(&mut self) {
        self.free_rects.clear();
        self.free_rects.push(Rect {
            x: 0,
            y: 0,
            w: self.width,
            h: self.height,
        });
    }
}

#[cfg(test)]
mod tests;
