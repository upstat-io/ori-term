//! Retained draw command list for UI rendering.
//!
//! [`DrawList`] accumulates [`DrawCommand`]s in painter's order. The GPU
//! converter in oriterm walks the list to emit instance buffer records.
//! A layer stack tracks background colors for subpixel text compositing.

use crate::color::Color;
use crate::geometry::{Point, Rect};
use crate::text::ShapedText;

use super::rect_style::RectStyle;

/// A single draw operation in painter's order.
#[derive(Debug, Clone, PartialEq)]
pub enum DrawCommand {
    /// A styled rectangle.
    Rect {
        /// Bounding rectangle in logical pixels.
        rect: Rect,
        /// Visual style (fill, border, radius, shadow).
        style: RectStyle,
    },
    /// A line segment.
    Line {
        /// Start point in logical pixels.
        from: Point,
        /// End point in logical pixels.
        to: Point,
        /// Line thickness in logical pixels.
        width: f32,
        /// Line color.
        color: Color,
    },
    /// A textured image quad (deferred — logged as no-op by converter).
    Image {
        /// Bounding rectangle in logical pixels.
        rect: Rect,
        /// GPU texture identifier.
        texture_id: u32,
        /// UV coordinates `[u_left, v_top, u_right, v_bottom]`.
        uv: [f32; 4],
    },
    /// A pre-shaped text block.
    Text {
        /// Top-left position of the text block in logical pixels.
        position: Point,
        /// Shaped glyphs with layout metrics.
        shaped: ShapedText,
        /// Text color (overrides the color in the original [`TextStyle`]).
        color: Color,
        /// Background color behind this text, for subpixel compositing.
        ///
        /// Captured automatically from the layer stack at push time.
        /// The GPU subpixel shader needs the actual background color to
        /// perform per-channel `mix()` correctly.
        bg_hint: Option<Color>,
    },
    /// Push a clip rectangle onto the clip stack.
    PushClip {
        /// Clip bounds in logical pixels.
        rect: Rect,
    },
    /// Pop the most recent clip rectangle from the stack.
    PopClip,
    /// A vector icon rendered as a mono glyph from the atlas.
    ///
    /// Icons are rasterized via `tiny_skia` and cached in the monochrome
    /// glyph atlas. The shader tints the alpha mask to `color`.
    Icon {
        /// Bounding rectangle in logical pixels.
        rect: Rect,
        /// Atlas page (texture array layer) containing the icon bitmap.
        atlas_page: u32,
        /// Normalized UV coordinates `[u_left, v_top, u_width, v_height]`.
        uv: [f32; 4],
        /// Icon tint color.
        color: Color,
    },
    /// Push a background layer onto the layer stack.
    ///
    /// Widgets that draw a background rect push their bg color here
    /// so child text commands automatically capture it for subpixel
    /// compositing.
    PushLayer {
        /// Background color for this layer.
        bg: Color,
    },
    /// Pop the most recent layer from the stack.
    PopLayer,
}

/// An ordered list of draw commands for a single frame.
///
/// Commands are drawn in push order (painter's algorithm). Clip state is
/// tracked via `push_clip` / `pop_clip` pairs. Layer state is tracked via
/// `push_layer` / `pop_layer` pairs — `push_text` captures the current
/// layer's background for subpixel compositing.
pub struct DrawList {
    commands: Vec<DrawCommand>,
    /// Tracks push/pop balance for debug assertions.
    clip_stack_depth: u32,
    /// Cumulative clip stack for visibility culling during widget draw.
    clip_stack: Vec<Rect>,
    /// Background color stack for subpixel text compositing.
    bg_stack: Vec<Color>,
    /// Tracks push/pop balance for debug assertions.
    layer_stack_depth: u32,
}

impl DrawList {
    /// Creates an empty draw list.
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            clip_stack_depth: 0,
            clip_stack: Vec::new(),
            bg_stack: Vec::new(),
            layer_stack_depth: 0,
        }
    }

    /// Appends a styled rectangle.
    pub fn push_rect(&mut self, rect: Rect, style: RectStyle) {
        self.commands.push(DrawCommand::Rect { rect, style });
    }

    /// Appends a line segment.
    pub fn push_line(&mut self, from: Point, to: Point, width: f32, color: Color) {
        self.commands.push(DrawCommand::Line {
            from,
            to,
            width,
            color,
        });
    }

    /// Appends a pre-shaped text block.
    ///
    /// The background color for subpixel compositing is captured automatically
    /// from the current layer stack. Widgets that draw text on a solid
    /// background should wrap in [`push_layer`](Self::push_layer) /
    /// [`pop_layer`](Self::pop_layer).
    pub fn push_text(&mut self, position: Point, shaped: ShapedText, color: Color) {
        let bg_hint = self.current_layer_bg().copied();
        self.commands.push(DrawCommand::Text {
            position,
            shaped,
            color,
            bg_hint,
        });
    }

    /// Appends a vector icon rendered as a mono atlas glyph.
    ///
    /// The `atlas_page` and `uv` must be resolved from the icon cache
    /// before calling this method. The shader tints the alpha mask to `color`.
    pub fn push_icon(&mut self, rect: Rect, atlas_page: u32, uv: [f32; 4], color: Color) {
        self.commands.push(DrawCommand::Icon {
            rect,
            atlas_page,
            uv,
            color,
        });
    }

    /// Appends a textured image quad.
    pub fn push_image(&mut self, rect: Rect, texture_id: u32, uv: [f32; 4]) {
        self.commands.push(DrawCommand::Image {
            rect,
            texture_id,
            uv,
        });
    }

    /// Pushes a clip rectangle. Must be paired with [`pop_clip`](Self::pop_clip).
    pub fn push_clip(&mut self, rect: Rect) {
        self.clip_stack_depth += 1;
        let cumulative = self
            .clip_stack
            .last()
            .copied()
            .map_or(rect, |current| current.intersection(rect));
        self.clip_stack.push(cumulative);
        self.commands.push(DrawCommand::PushClip { rect });
    }

    /// Pops the most recent clip rectangle.
    ///
    /// # Panics
    ///
    /// Panics if the clip stack is already empty.
    pub fn pop_clip(&mut self) {
        assert!(
            self.clip_stack_depth > 0,
            "pop_clip called with empty clip stack",
        );
        self.clip_stack_depth -= 1;
        self.clip_stack.pop();
        self.commands.push(DrawCommand::PopClip);
    }

    /// Pushes a background layer. Must be paired with [`pop_layer`](Self::pop_layer).
    ///
    /// The `bg` color is captured by subsequent [`push_text`](Self::push_text)
    /// calls for subpixel compositing. Layers nest — inner layers override
    /// outer layers.
    pub fn push_layer(&mut self, bg: Color) {
        self.layer_stack_depth += 1;
        self.bg_stack.push(bg);
        self.commands.push(DrawCommand::PushLayer { bg });
    }

    /// Pops the most recent background layer.
    ///
    /// # Panics
    ///
    /// Panics if the layer stack is already empty.
    pub fn pop_layer(&mut self) {
        assert!(
            self.layer_stack_depth > 0,
            "pop_layer called with empty layer stack",
        );
        self.layer_stack_depth -= 1;
        self.bg_stack.pop();
        self.commands.push(DrawCommand::PopLayer);
    }

    /// Returns the current layer's background color, if any.
    pub fn current_layer_bg(&self) -> Option<&Color> {
        self.bg_stack.last()
    }

    /// Returns the effective clip bounds after intersecting all active clips.
    pub fn current_clip_rect(&self) -> Option<Rect> {
        self.clip_stack.last().copied()
    }

    /// Returns the commands in draw order.
    pub fn commands(&self) -> &[DrawCommand] {
        &self.commands
    }

    /// Whether the list contains no commands.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Number of commands in the list.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Removes all commands and resets all stacks, retaining allocated memory.
    pub fn clear(&mut self) {
        self.commands.clear();
        self.clip_stack_depth = 0;
        self.clip_stack.clear();
        self.bg_stack.clear();
        self.layer_stack_depth = 0;
    }
}

impl Default for DrawList {
    fn default() -> Self {
        Self::new()
    }
}
