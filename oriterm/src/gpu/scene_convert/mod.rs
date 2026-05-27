//! Converts draw primitives into GPU instance buffer records.
//!
//! Entry point: [`convert_scene`] — iterates typed Scene arrays directly.
//!
//! Each primitive becomes one or more instance buffer records:
//! - Quad/line → [`UiRectWriter::push_ui_rect`](super::ui_rect_writer::UiRectWriter::push_ui_rect)
//! - Text → [`push_glyph`](super::instance_writer::InstanceWriter::push_glyph) per shaped glyph
//! - Icon → [`push_glyph`](super::instance_writer::InstanceWriter::push_glyph) (mono atlas)
//!
//! Per-instance clip rects are resolved from each primitive's `ContentMask`.

mod text;

#[cfg(not(target_os = "macos"))]
pub(crate) use text::color_to_rgb;
use text::{IconDraw, TextDraw, convert_icon, convert_text};

use oriterm_ui::color::Color;
use oriterm_ui::draw::scene::{ContentMask, IconPrimitive, LinePrimitive, Quad, TextRun};
use oriterm_ui::draw::{RectStyle, Scene};
use oriterm_ui::geometry::{Point, Rect};

use super::instance_writer::{InstanceWriter, ScreenRect};
use super::prepare::AtlasLookup;
use super::srgb_f32_to_linear;
use super::ui_rect_writer::UiRectWriter;

/// Per-primitive paint parameters: scale, opacity, and clip rect.
///
/// Threaded through every `convert_*` helper. `scale` and `opacity` come
/// from the compositor; `clip` is the physical-pixel clip rect resolved
/// from the primitive's `ContentMask` via [`clip_from_mask`].
#[derive(Clone, Copy)]
pub struct PaintParams {
    /// Logical-to-physical pixel scale factor.
    pub scale: f32,
    /// Effective opacity (compositor opacity x mask opacity).
    pub opacity: f32,
    /// Physical-pixel clip rect `[x, y, w, h]`.
    pub clip: [f32; 4],
}

/// Line segment geometry + color for [`convert_line_clipped`].
#[derive(Clone, Copy)]
struct LineSpec {
    /// Start point (logical pixels).
    from: Point,
    /// End point (logical pixels).
    to: Point,
    /// Line thickness (logical pixels).
    width: f32,
    /// Line color.
    color: Color,
}

/// Context for converting text primitives into glyph instances.
///
/// Bundles atlas lookup, output writers, and font metrics needed for text
/// rendering. Pass to [`convert_scene`] to enable text/icon conversion.
/// When `None` is passed instead, text and icon primitives are skipped.
pub struct TextContext<'a> {
    /// Glyph atlas lookup (shared with the terminal prepare phase).
    pub atlas: &'a dyn AtlasLookup,
    /// Output writer for monochrome atlas glyphs.
    pub mono_writer: &'a mut InstanceWriter,
    /// Output writer for subpixel atlas glyphs.
    pub subpixel_writer: &'a mut InstanceWriter,
    /// Output writer for color atlas glyphs (emoji, bitmap).
    pub color_writer: &'a mut InstanceWriter,
    /// Whether hinting is enabled for [`RasterKey`] construction.
    pub hinted: bool,
    /// Whether subpixel glyph positioning is enabled.
    pub subpixel_positioning: bool,
}

/// Convert all primitives in a [`Scene`] to GPU instance buffer records.
///
/// Iterates the Scene's typed arrays directly — no command dispatch or
/// stack processing. Each primitive's `ContentMask` is resolved into a
/// per-instance clip rect. Offsets are already baked into primitive
/// positions by the Scene's push methods.
///
/// Rect and line primitives go to `ui_writer`. Text and icon primitives
/// go to the writers in `text_ctx` (routed by atlas kind). Pass `None`
/// for `text_ctx` to skip text/icon rendering.
pub fn convert_scene(
    scene: &Scene,
    ui_writer: &mut UiRectWriter,
    text_ctx: Option<&mut TextContext<'_>>,
    scale: f32,
    opacity: f32,
) {
    for quad in scene.quads() {
        let paint = paint_for(&quad.content_mask, scale, opacity);
        convert_quad(quad, ui_writer, paint);
    }
    for line in scene.lines() {
        let paint = paint_for(&line.content_mask, scale, opacity);
        convert_scene_line(line, ui_writer, paint);
    }
    if let Some(ctx) = text_ctx {
        for text in scene.text_runs() {
            let paint = paint_for(&text.content_mask, scale, opacity);
            convert_scene_text(text, ctx, paint);
        }
        for icon in scene.icons() {
            let paint = paint_for(&icon.content_mask, scale, opacity);
            convert_scene_icon(icon, ctx, paint);
        }
    }
}

/// Build [`PaintParams`] for a primitive: resolve its clip rect and combine
/// the compositor opacity with the mask opacity.
fn paint_for(cm: &ContentMask, scale: f32, opacity: f32) -> PaintParams {
    PaintParams {
        scale,
        opacity: opacity * cm.opacity,
        clip: clip_from_mask(cm, scale),
    }
}

/// Convert a `ContentMask` clip rect to a physical-pixel `[f32; 4]` for the GPU.
fn clip_from_mask(cm: &ContentMask, scale: f32) -> [f32; 4] {
    [
        cm.clip.x() * scale,
        cm.clip.y() * scale,
        cm.clip.width() * scale,
        cm.clip.height() * scale,
    ]
}

/// Convert a Scene [`Quad`] to one or two UI rect instances.
///
/// Positions are in logical pixels (already offset-resolved by Scene).
/// The `clip` array is in physical pixels (pre-scaled by `clip_from_mask`).
fn convert_quad(quad: &Quad, writer: &mut UiRectWriter, paint: PaintParams) {
    convert_rect_clipped(quad.bounds, &quad.style, writer, paint);
}

/// Convert a Scene [`LinePrimitive`] to GPU rect instances with clip.
fn convert_scene_line(line: &LinePrimitive, writer: &mut UiRectWriter, paint: PaintParams) {
    let spec = LineSpec {
        from: line.from,
        to: line.to,
        width: line.width,
        color: line.color,
    };
    convert_line_clipped(spec, writer, paint);
}

/// Convert a Scene [`TextRun`] to glyph instances with clip.
fn convert_scene_text(text: &TextRun, ctx: &mut TextContext<'_>, paint: PaintParams) {
    let draw = TextDraw {
        position: text.position,
        shaped: &text.shaped,
        color: text.color,
        bg_hint: text.bg_hint,
    };
    convert_text(draw, ctx, paint);
}

/// Convert a Scene [`IconPrimitive`] to a glyph instance with clip.
fn convert_scene_icon(icon: &IconPrimitive, ctx: &mut TextContext<'_>, paint: PaintParams) {
    let draw = IconDraw {
        rect: icon.rect,
        atlas_page: icon.atlas_page,
        uv: icon.uv,
        color: icon.color,
    };
    convert_icon(draw, ctx, paint);
}

/// Convert a styled rect to one or two UI rect instances with a per-instance clip.
///
/// Populates the full 144-byte per-side border format.
fn convert_rect_clipped(
    rect: Rect,
    style: &RectStyle,
    writer: &mut UiRectWriter,
    paint: PaintParams,
) {
    let PaintParams {
        scale,
        opacity,
        clip,
    } = paint;
    // Resolve fill color: prefer gradient first stop, then solid fill.
    let fill = style
        .gradient
        .as_ref()
        .and_then(|g| g.stops.first().map(|s| s.color))
        .or(style.fill)
        .unwrap_or(Color::TRANSPARENT);

    // Shadow instance (if present): expanded rect behind the main rect.
    if let Some(shadow) = &style.shadow {
        let expand = shadow.spread + shadow.blur_radius;
        let shadow_rect = ScreenRect {
            x: rect.x() + shadow.offset_x - expand,
            y: rect.y() + shadow.offset_y - expand,
            w: rect.width() + expand * 2.0,
            h: rect.height() + expand * 2.0,
        };
        // Shadow: per-corner expanded radii, no border.
        let shadow_radii = [
            (style.corner_radius[0] + expand) * scale,
            (style.corner_radius[1] + expand) * scale,
            (style.corner_radius[2] + expand) * scale,
            (style.corner_radius[3] + expand) * scale,
        ];
        writer.push_ui_rect(
            shadow_rect.scaled(scale),
            color_to_linear_with_opacity(shadow.color, opacity),
            [0.0; 4],
            shadow_radii,
            [[0.0; 4]; 4],
            clip,
        );
    }

    // Main rect instance with full per-side border data.
    let screen = to_screen_rect(rect).scaled(scale);
    let fill_linear = color_to_linear_with_opacity(fill, opacity);

    // Border widths scaled to physical pixels.
    let widths = style.border.widths();
    let border_widths = [
        widths[0] * scale,
        widths[1] * scale,
        widths[2] * scale,
        widths[3] * scale,
    ];

    // Corner radii scaled to physical pixels.
    let corner_radii = [
        style.corner_radius[0] * scale,
        style.corner_radius[1] * scale,
        style.corner_radius[2] * scale,
        style.corner_radius[3] * scale,
    ];

    // Per-side border colors converted to linear.
    let colors = style.border.colors();
    let border_colors = [
        color_to_linear_with_opacity(colors[0], opacity),
        color_to_linear_with_opacity(colors[1], opacity),
        color_to_linear_with_opacity(colors[2], opacity),
        color_to_linear_with_opacity(colors[3], opacity),
    ];

    writer.push_ui_rect(
        screen,
        fill_linear,
        border_widths,
        corner_radii,
        border_colors,
        clip,
    );
}

/// Convert a line segment to GPU rect instances with a per-instance clip.
///
/// Axis-aligned lines (horizontal or vertical) produce a single thin rect.
/// Diagonal lines are decomposed into pixel-stepping rects along the major
/// axis — one `width x width` rect per step — to avoid the AABB problem
/// where a single bounding box fills a solid square for 45-degree lines.
fn convert_line_clipped(spec: LineSpec, writer: &mut UiRectWriter, paint: PaintParams) {
    let LineSpec {
        from,
        to,
        width,
        color,
    } = spec;
    let PaintParams {
        scale,
        opacity,
        clip,
    } = paint;
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = dx.hypot(dy);
    if len < f32::EPSILON {
        return;
    }

    let fill = color_to_linear_with_opacity(color, opacity);
    let hw = width * 0.5;

    // Lines have no border — zero widths and transparent colors.
    let no_bw = [0.0; 4];
    let no_cr = [0.0; 4];
    let no_bc = [[0.0; 4]; 4];

    // Axis-aligned fast paths: single rect.
    if dx.abs() < f32::EPSILON {
        // Vertical line.
        let (min_y, max_y) = if from.y < to.y {
            (from.y, to.y)
        } else {
            (to.y, from.y)
        };
        let rect = ScreenRect {
            x: from.x - hw,
            y: min_y,
            w: width,
            h: max_y - min_y,
        }
        .scaled(scale);
        writer.push_ui_rect(rect, fill, no_bw, no_cr, no_bc, clip);
        return;
    }
    if dy.abs() < f32::EPSILON {
        // Horizontal line.
        let (min_x, max_x) = if from.x < to.x {
            (from.x, to.x)
        } else {
            (to.x, from.x)
        };
        let rect = ScreenRect {
            x: min_x,
            y: from.y - hw,
            w: max_x - min_x,
            h: width,
        }
        .scaled(scale);
        writer.push_ui_rect(rect, fill, no_bw, no_cr, no_bc, clip);
        return;
    }

    // Diagonal line: step along the major axis and emit one rect per step.
    let steps = dx.abs().max(dy.abs()).ceil() as usize;
    if steps == 0 {
        return;
    }
    let sx = dx / steps as f32;
    let sy = dy / steps as f32;

    for i in 0..=steps {
        let x = from.x + sx * i as f32;
        let y = from.y + sy * i as f32;
        let rect = ScreenRect {
            x: x - hw,
            y: y - hw,
            w: width,
            h: width,
        }
        .scaled(scale);
        writer.push_ui_rect(rect, fill, no_bw, no_cr, no_bc, clip);
    }
}

/// Convert a geometry [`Rect`] to a [`ScreenRect`] for the instance writer.
fn to_screen_rect(rect: Rect) -> ScreenRect {
    ScreenRect {
        x: rect.x(),
        y: rect.y(),
        w: rect.width(),
        h: rect.height(),
    }
}

/// Convert an sRGB [`Color`] to a linear-light `[f32; 4]` for the GPU,
/// multiplying alpha by the compositor `opacity`.
///
/// The `*Srgb` render target applies hardware sRGB encoding on output, so
/// all colors passed to shaders must be in linear space. UI `Color` values
/// are stored as sRGB; this decodes each RGB channel and applies the
/// compositor opacity to the alpha channel.
fn color_to_linear_with_opacity(c: Color, opacity: f32) -> [f32; 4] {
    [
        srgb_f32_to_linear(c.r),
        srgb_f32_to_linear(c.g),
        srgb_f32_to_linear(c.b),
        c.a * opacity,
    ]
}

#[cfg(test)]
mod tests;
