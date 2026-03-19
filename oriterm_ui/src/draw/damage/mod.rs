//! Per-widget damage tracking via primitive hashing.
//!
//! After `build_scene()`, the `DamageTracker` iterates the `Scene`,
//! hashes primitives grouped by widget ID, and compares against the
//! previous frame to identify changed regions.

mod hash_primitives;

use std::collections::HashMap;

use crate::draw::Scene;
use crate::geometry::Rect;
use crate::widget_id::WidgetId;

use hash_primitives::hash_scene_widget;

/// Per-widget frame state for damage comparison.
struct WidgetFrameState {
    /// Hash of all primitives belonging to this widget.
    hash: u64,
    /// Union of all primitive bounds for this widget.
    bounds: Rect,
}

/// Tracks per-widget damage between frames.
///
/// After each `build_scene()` call, feed the resulting `Scene` to
/// `compute_damage()`. The tracker compares per-widget primitive hashes
/// against the previous frame and produces a list of dirty regions.
pub struct DamageTracker {
    dirty_regions: Vec<Rect>,
    merge_scratch: Vec<Rect>,
    prev_state: HashMap<WidgetId, WidgetFrameState>,
    current_scratch: HashMap<WidgetId, WidgetFrameState>,
    first_frame: bool,
}

impl DamageTracker {
    /// Creates a new tracker. The first `compute_damage()` call will
    /// report the entire scene as dirty.
    pub fn new() -> Self {
        Self {
            dirty_regions: Vec::new(),
            merge_scratch: Vec::new(),
            prev_state: HashMap::new(),
            current_scratch: HashMap::new(),
            first_frame: true,
        }
    }

    /// Computes dirty regions by comparing the current scene against
    /// the previous frame's per-widget hashes.
    pub fn compute_damage(&mut self, scene: &Scene) {
        self.dirty_regions.clear();
        self.current_scratch.clear();

        // Build current per-widget state from scene.
        hash_scene_widget(scene, &mut self.current_scratch);

        if self.first_frame {
            // First frame: entire scene bounds are dirty.
            if let Some(full) = full_scene_bounds(scene) {
                self.dirty_regions.push(full);
            }
            self.first_frame = false;
            std::mem::swap(&mut self.prev_state, &mut self.current_scratch);
            return;
        }

        // Diff current vs prev.
        // Changed or new widgets -> dirty at new bounds.
        for (id, cur) in &self.current_scratch {
            match self.prev_state.get(id) {
                Some(prev) if prev.hash == cur.hash && prev.bounds == cur.bounds => {
                    // Clean — same hash and bounds.
                }
                Some(prev) => {
                    // Changed — dirty at both old and new bounds.
                    self.dirty_regions.push(prev.bounds);
                    self.dirty_regions.push(cur.bounds);
                }
                None => {
                    // New widget — dirty at new bounds.
                    self.dirty_regions.push(cur.bounds);
                }
            }
        }

        // Removed widgets -> dirty at old bounds.
        for (id, prev) in &self.prev_state {
            if !self.current_scratch.contains_key(id) {
                self.dirty_regions.push(prev.bounds);
            }
        }

        // Merge overlapping dirty rects.
        self.merge_overlapping();

        // Swap current -> prev for next frame.
        std::mem::swap(&mut self.prev_state, &mut self.current_scratch);
    }

    /// Returns `true` if any region is dirty.
    pub fn has_damage(&self) -> bool {
        !self.dirty_regions.is_empty()
    }

    /// Returns `true` if this is the first frame (no previous state).
    pub fn is_first_frame(&self) -> bool {
        self.first_frame
    }

    /// Returns the list of dirty regions from the last `compute_damage()` call.
    pub fn dirty_regions(&self) -> &[Rect] {
        &self.dirty_regions
    }

    /// Returns `true` if the given rect overlaps any dirty region.
    pub fn is_region_dirty(&self, rect: Rect) -> bool {
        self.dirty_regions.iter().any(|d| d.intersects(rect))
    }

    /// Resets the tracker, forcing the next frame to act as a first frame.
    ///
    /// Call this after resize, theme change, font change, or scale change.
    pub fn reset(&mut self) {
        self.dirty_regions.clear();
        self.prev_state.clear();
        self.current_scratch.clear();
        self.first_frame = true;
    }
}

// --- Private helpers ---

/// Computes the union of all primitive bounds in the scene.
fn full_scene_bounds(scene: &Scene) -> Option<Rect> {
    let mut result: Option<Rect> = None;
    for q in scene.quads() {
        result = Some(union_opt(result, q.bounds));
    }
    for t in scene.text_runs() {
        let r = Rect::new(t.position.x, t.position.y, t.shaped.width, t.shaped.height);
        result = Some(union_opt(result, r));
    }
    for l in scene.lines() {
        let r = line_bounds(l.from, l.to, l.width);
        result = Some(union_opt(result, r));
    }
    for i in scene.icons() {
        result = Some(union_opt(result, i.rect));
    }
    for i in scene.images() {
        result = Some(union_opt(result, i.rect));
    }
    result
}

impl DamageTracker {
    /// Greedy merge of overlapping dirty rects using pre-allocated scratch.
    #[expect(
        clippy::needless_range_loop,
        reason = "parallel indexing of `used` and `self.dirty_regions` — iterators don't help"
    )]
    fn merge_overlapping(&mut self) {
        if self.dirty_regions.len() <= 1 {
            return;
        }

        loop {
            self.merge_scratch.clear();
            let mut merged_any = false;
            let mut used = vec![false; self.dirty_regions.len()];

            for i in 0..self.dirty_regions.len() {
                if used[i] {
                    continue;
                }
                let mut r = self.dirty_regions[i];
                for j in (i + 1)..self.dirty_regions.len() {
                    if used[j] {
                        continue;
                    }
                    if r.intersects(self.dirty_regions[j]) {
                        r = r.union(self.dirty_regions[j]);
                        used[j] = true;
                        merged_any = true;
                    }
                }
                self.merge_scratch.push(r);
            }

            std::mem::swap(&mut self.dirty_regions, &mut self.merge_scratch);

            if !merged_any {
                break;
            }
        }
    }
}

impl Default for DamageTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Union of an optional accumulator with a new rect.
fn union_opt(acc: Option<Rect>, r: Rect) -> Rect {
    match acc {
        Some(a) => a.union(r),
        None => r,
    }
}

/// Bounding rect of a line segment with half-width padding.
fn line_bounds(from: crate::geometry::Point, to: crate::geometry::Point, width: f32) -> Rect {
    let half = width / 2.0;
    let min_x = from.x.min(to.x) - half;
    let min_y = from.y.min(to.y) - half;
    let max_x = from.x.max(to.x) + half;
    let max_y = from.y.max(to.y) + half;
    Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

#[cfg(test)]
mod tests;
