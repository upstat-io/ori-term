//! Type-separated scene for UI rendering.
//!
//! Uses typed primitive arrays for draw output. Each primitive carries
//! its resolved visual state (`ContentMask`) — no stack commands in the
//! output. The GPU renderer iterates typed arrays directly.

mod content_mask;
mod paint;
mod primitives;
mod stacks;

pub use content_mask::ContentMask;
pub use primitives::{IconPrimitive, ImagePrimitive, LinePrimitive, Quad, TextRun};

use crate::color::Color;
use crate::geometry::Rect;
use crate::widget_id::WidgetId;
use crate::widgets::Widget;
use crate::widgets::contexts::DrawCtx;

/// Type-separated collection of draw primitives.
///
/// Owns typed arrays for each primitive kind and internal state stacks
/// (clip, offset, layer bg) that are resolved into each primitive's
/// `ContentMask` at push time. State stacks are never emitted as output.
pub struct Scene {
    // Typed primitive arrays (output).
    pub(crate) quads: Vec<Quad>,
    pub(crate) text_runs: Vec<TextRun>,
    pub(crate) lines: Vec<LinePrimitive>,
    pub(crate) icons: Vec<IconPrimitive>,
    pub(crate) images: Vec<ImagePrimitive>,

    // Internal state stacks (resolved into ContentMask, not in output).
    clip_stack: Vec<Rect>,
    offset_stack: Vec<(f32, f32)>,
    cumulative_offset: (f32, f32),
    layer_bg_stack: Vec<Color>,

    // Current context (set by the caller before painting).
    current_widget_id: Option<WidgetId>,
}

impl Scene {
    /// Creates an empty scene with no primitives and no active stacks.
    pub fn new() -> Self {
        Self {
            quads: Vec::new(),
            text_runs: Vec::new(),
            lines: Vec::new(),
            icons: Vec::new(),
            images: Vec::new(),
            clip_stack: Vec::new(),
            offset_stack: Vec::new(),
            cumulative_offset: (0.0, 0.0),
            layer_bg_stack: Vec::new(),
            current_widget_id: None,
        }
    }

    /// Sets the current widget ID for subsequent push operations.
    ///
    /// All primitives pushed after this call will carry this widget ID
    /// until it is changed again. Set to `None` for root-level draws.
    pub fn set_widget_id(&mut self, id: Option<WidgetId>) {
        self.current_widget_id = id;
    }

    /// Returns the current widget ID.
    pub fn widget_id(&self) -> Option<WidgetId> {
        self.current_widget_id
    }

    /// Returns the quad primitives.
    pub fn quads(&self) -> &[Quad] {
        &self.quads
    }

    /// Returns the text run primitives.
    pub fn text_runs(&self) -> &[TextRun] {
        &self.text_runs
    }

    /// Returns the line primitives.
    pub fn lines(&self) -> &[LinePrimitive] {
        &self.lines
    }

    /// Returns the icon primitives.
    pub fn icons(&self) -> &[IconPrimitive] {
        &self.icons
    }

    /// Returns the image primitives.
    pub fn images(&self) -> &[ImagePrimitive] {
        &self.images
    }

    /// Returns `true` if the scene contains no primitives.
    pub fn is_empty(&self) -> bool {
        self.quads.is_empty()
            && self.text_runs.is_empty()
            && self.lines.is_empty()
            && self.icons.is_empty()
            && self.images.is_empty()
    }

    /// Total number of primitives across all typed arrays.
    pub fn len(&self) -> usize {
        self.quads.len()
            + self.text_runs.len()
            + self.lines.len()
            + self.icons.len()
            + self.images.len()
    }

    /// Clears all primitives and resets stacks, retaining allocated memory.
    pub fn clear(&mut self) {
        self.quads.clear();
        self.text_runs.clear();
        self.lines.clear();
        self.icons.clear();
        self.images.clear();
        self.clip_stack.clear();
        self.offset_stack.clear();
        self.cumulative_offset = (0.0, 0.0);
        self.layer_bg_stack.clear();
        self.current_widget_id = None;
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

/// Paints the widget tree into a Scene. Full repaint every call.
///
/// Replaces `compose_scene()`. The caller uses `DamageTracker` to
/// detect what changed between frames.
pub fn build_scene(root: &dyn Widget, ctx: &mut DrawCtx<'_>) {
    ctx.scene.clear();
    root.paint(ctx);
    debug_assert!(
        ctx.scene.clip_stack_is_empty()
            && ctx.scene.offset_stack_is_empty()
            && ctx.scene.layer_bg_stack_is_empty(),
        "Unbalanced stacks after build_scene"
    );
}

#[cfg(test)]
mod tests;
