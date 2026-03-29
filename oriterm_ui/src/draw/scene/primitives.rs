//! Type-separated draw primitives.
//!
//! Each primitive carries its resolved visual state (position, color,
//! clip rect). The GPU renderer reads typed arrays directly — no
//! command dispatch or stack processing at consumption time.

use crate::color::Color;
use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::text::ShapedText;
use crate::widget_id::WidgetId;

use super::content_mask::ContentMask;

/// Filled, bordered, or shadowed rectangle.
#[derive(Debug, Clone, PartialEq)]
pub struct Quad {
    /// Position and size in viewport space.
    pub bounds: Rect,
    /// Visual style (fill, border, radius, shadow, gradient).
    pub style: RectStyle,
    /// Resolved clip rect from ancestor clips.
    pub content_mask: ContentMask,
    /// Widget that produced this primitive.
    pub widget_id: Option<WidgetId>,
}

/// Pre-shaped text at a position.
#[derive(Debug, Clone, PartialEq)]
pub struct TextRun {
    /// Baseline origin in viewport space.
    pub position: Point,
    /// Pre-shaped glyph run.
    pub shaped: ShapedText,
    /// Text color.
    pub color: Color,
    /// Background hint for subpixel compositing (from layer bg stack).
    pub bg_hint: Option<Color>,
    /// Resolved clip rect from ancestor clips.
    pub content_mask: ContentMask,
    /// Widget that produced this primitive.
    pub widget_id: Option<WidgetId>,
}

/// Line segment with thickness.
#[derive(Debug, Clone, PartialEq)]
pub struct LinePrimitive {
    /// Start point in viewport space.
    pub from: Point,
    /// End point in viewport space.
    pub to: Point,
    /// Line width in pixels.
    pub width: f32,
    /// Line color.
    pub color: Color,
    /// Resolved clip rect from ancestor clips.
    pub content_mask: ContentMask,
    /// Widget that produced this primitive.
    pub widget_id: Option<WidgetId>,
}

/// Monochrome atlas icon.
#[derive(Debug, Clone, PartialEq)]
pub struct IconPrimitive {
    /// Position and size in viewport space.
    pub rect: Rect,
    /// Atlas texture page index.
    pub atlas_page: u32,
    /// UV coordinates `[u_left, v_top, u_width, v_height]`.
    pub uv: [f32; 4],
    /// Tint color.
    pub color: Color,
    /// Resolved clip rect from ancestor clips.
    pub content_mask: ContentMask,
    /// Widget that produced this primitive.
    pub widget_id: Option<WidgetId>,
}

/// Texture-mapped rectangle.
#[derive(Debug, Clone, PartialEq)]
pub struct ImagePrimitive {
    /// Position and size in viewport space.
    pub rect: Rect,
    /// Texture resource identifier.
    pub texture_id: u32,
    /// UV coordinates `[u_left, v_top, u_width, v_height]`.
    pub uv: [f32; 4],
    /// Resolved clip rect from ancestor clips.
    pub content_mask: ContentMask,
    /// Widget that produced this primitive.
    pub widget_id: Option<WidgetId>,
}
