//! Paint methods for appending primitives to the Scene.
//!
//! Each method resolves the current state stacks (clip, offset, layer bg)
//! into the primitive's `ContentMask` and position at push time. The GPU
//! renderer reads the resolved values directly.
//!
//! Widget ID is read from `Scene::current_widget_id` (set via
//! `set_widget_id()`), not passed per-call — this keeps argument counts
//! within bounds.

use crate::color::Color;
use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::text::ShapedText;

use super::Scene;
use super::primitives::{IconPrimitive, ImagePrimitive, LinePrimitive, Quad, TextRun};

impl Scene {
    /// Appends a styled rectangle to the scene.
    pub fn push_quad(&mut self, bounds: Rect, style: RectStyle) {
        let offset_bounds = self.apply_offset(bounds);
        self.quads.push(Quad {
            bounds: offset_bounds,
            style,
            content_mask: self.current_content_mask(),
            widget_id: self.current_widget_id,
        });
    }

    /// Appends pre-shaped text to the scene.
    pub fn push_text(&mut self, position: Point, shaped: ShapedText, color: Color) {
        self.text_runs.push(TextRun {
            position: self.apply_offset_point(position),
            shaped,
            color,
            bg_hint: self.current_layer_bg(),
            content_mask: self.current_content_mask(),
            widget_id: self.current_widget_id,
        });
    }

    /// Appends a line segment to the scene.
    pub fn push_line(&mut self, from: Point, to: Point, width: f32, color: Color) {
        self.lines.push(LinePrimitive {
            from: self.apply_offset_point(from),
            to: self.apply_offset_point(to),
            width,
            color,
            content_mask: self.current_content_mask(),
            widget_id: self.current_widget_id,
        });
    }

    /// Appends a monochrome atlas icon to the scene.
    pub fn push_icon(&mut self, rect: Rect, atlas_page: u32, uv: [f32; 4], color: Color) {
        self.icons.push(IconPrimitive {
            rect: self.apply_offset(rect),
            atlas_page,
            uv,
            color,
            content_mask: self.current_content_mask(),
            widget_id: self.current_widget_id,
        });
    }

    /// Appends a texture-mapped image to the scene.
    pub fn push_image(&mut self, rect: Rect, texture_id: u32, uv: [f32; 4]) {
        self.images.push(ImagePrimitive {
            rect: self.apply_offset(rect),
            texture_id,
            uv,
            content_mask: self.current_content_mask(),
            widget_id: self.current_widget_id,
        });
    }
}

// --- Internal helpers ---

impl Scene {
    /// Resolves the current clip stack into a `ContentMask`.
    pub(super) fn current_content_mask(&self) -> ContentMask {
        self.clip_stack
            .last()
            .map_or_else(ContentMask::unclipped, |clip| ContentMask { clip: *clip })
    }

    /// Applies the cumulative offset to a rectangle.
    pub(super) fn apply_offset(&self, rect: Rect) -> Rect {
        rect.offset(self.cumulative_offset.0, self.cumulative_offset.1)
    }

    /// Applies the cumulative offset to a point.
    pub(super) fn apply_offset_point(&self, point: Point) -> Point {
        Point::new(
            point.x + self.cumulative_offset.0,
            point.y + self.cumulative_offset.1,
        )
    }
}

use super::content_mask::ContentMask;
