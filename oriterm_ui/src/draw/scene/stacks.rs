//! Internal state stacks for the Scene.
//!
//! Push/pop methods manage clip, offset, and layer background stacks.
//! State is resolved into `ContentMask` at primitive push time — stacks
//! are consumed internally, never emitted as output.

use crate::color::Color;
use crate::geometry::Rect;

use super::Scene;

// --- Clip ---

impl Scene {
    /// Pushes a clip rectangle. Primitives pushed after this call will have
    /// their `ContentMask` set to the intersection of all ancestor clips.
    pub fn push_clip(&mut self, rect: Rect) {
        let resolved = self.apply_offset(rect);
        let intersected = self
            .clip_stack
            .last()
            .map_or(resolved, |c| c.intersection(resolved));
        self.clip_stack.push(intersected);
    }

    /// Pops the most recent clip rectangle.
    pub fn pop_clip(&mut self) {
        debug_assert!(
            !self.clip_stack.is_empty(),
            "pop_clip without matching push_clip"
        );
        self.clip_stack.pop();
    }
}

// --- Offset ---

impl Scene {
    /// Pushes a 2D translation offset. All subsequent primitives will be
    /// offset by the cumulative translation.
    pub fn push_offset(&mut self, dx: f32, dy: f32) {
        self.offset_stack.push((dx, dy));
        self.cumulative_offset.0 += dx;
        self.cumulative_offset.1 += dy;
    }

    /// Pops the most recent translation offset.
    pub fn pop_offset(&mut self) {
        debug_assert!(
            !self.offset_stack.is_empty(),
            "pop_offset without matching push_offset"
        );
        if let Some((dx, dy)) = self.offset_stack.pop() {
            self.cumulative_offset.0 -= dx;
            self.cumulative_offset.1 -= dy;
        }
    }
}

// --- Layer background ---

impl Scene {
    /// Pushes a background color for subpixel text compositing.
    ///
    /// Text primitives pushed after this call will have `bg_hint` set to
    /// this color, enabling the GPU to blend subpixel-rendered text against
    /// the correct background.
    pub fn push_layer_bg(&mut self, bg: Color) {
        self.layer_bg_stack.push(bg);
    }

    /// Pops the most recent layer background color.
    pub fn pop_layer_bg(&mut self) {
        debug_assert!(
            !self.layer_bg_stack.is_empty(),
            "pop_layer_bg without matching push_layer_bg"
        );
        self.layer_bg_stack.pop();
    }
}

// --- Opacity ---

impl Scene {
    /// Pushes a subtree opacity multiplier.
    ///
    /// Values are clamped to `0.0..=1.0`. NaN and infinity normalize to
    /// `1.0` so bad inputs do not poison the scene state. Stacked opacity
    /// composes multiplicatively.
    pub fn push_opacity(&mut self, opacity: f32) {
        let clamped = normalize_opacity(opacity);
        self.opacity_stack.push(clamped);
        self.cumulative_opacity *= clamped;
    }

    /// Pops the most recent opacity multiplier.
    pub fn pop_opacity(&mut self) {
        debug_assert!(
            !self.opacity_stack.is_empty(),
            "pop_opacity without matching push_opacity"
        );
        if let Some(val) = self.opacity_stack.pop() {
            // Recompute from scratch to avoid floating-point drift.
            self.cumulative_opacity = self.opacity_stack.iter().product::<f32>().max(0.0);
            if self.opacity_stack.is_empty() {
                self.cumulative_opacity = 1.0;
            }
            let _ = val;
        }
    }

    /// Returns the current cumulative opacity (product of all pushed values).
    pub fn current_opacity(&self) -> f32 {
        self.cumulative_opacity
    }
}

/// Normalizes an opacity value: clamp to `0.0..=1.0`, NaN/infinity → `1.0`.
fn normalize_opacity(v: f32) -> f32 {
    if v.is_finite() {
        v.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

// --- Queries ---

impl Scene {
    /// Current clip rect in viewport space, if any clips are active.
    pub fn current_clip(&self) -> Option<Rect> {
        self.clip_stack.last().copied()
    }

    /// Clip rect in content space (for scroll container visibility culling).
    ///
    /// Subtracts the cumulative offset from the viewport-space clip, giving
    /// the clip rect in the coordinate space of the content being scrolled.
    pub fn current_clip_in_content_space(&self) -> Option<Rect> {
        self.clip_stack
            .last()
            .map(|clip| clip.offset(-self.cumulative_offset.0, -self.cumulative_offset.1))
    }

    /// Current layer background color for subpixel text compositing.
    pub fn current_layer_bg(&self) -> Option<Color> {
        self.layer_bg_stack.last().copied()
    }

    /// Returns `true` if the clip stack is empty.
    pub fn clip_stack_is_empty(&self) -> bool {
        self.clip_stack.is_empty()
    }

    /// Returns `true` if the offset stack is empty.
    pub fn offset_stack_is_empty(&self) -> bool {
        self.offset_stack.is_empty()
    }

    /// Returns `true` if the layer background stack is empty.
    pub fn layer_bg_stack_is_empty(&self) -> bool {
        self.layer_bg_stack.is_empty()
    }

    /// Returns `true` if the opacity stack is empty.
    pub fn opacity_stack_is_empty(&self) -> bool {
        self.opacity_stack.is_empty()
    }
}
