//! Converts draw primitives into GPU instance buffer records.
//!
//! Entry point: [`convert_scene`] — iterates typed Scene arrays directly.
//!
//! Each primitive becomes one or more instance buffer records:
//! - Quad/line → [`push_ui_rect`](super::instance_writer::InstanceWriter::push_ui_rect)
//! - Text → [`push_glyph`](super::instance_writer::InstanceWriter::push_glyph) per shaped glyph
//! - Icon → [`push_glyph`](super::instance_writer::InstanceWriter::push_glyph) (mono atlas)
//!
//! Per-instance clip rects are resolved from each primitive's `ContentMask`.

mod text;

#[cfg(test)]
use text::color_to_rgb;
use text::{convert_icon, convert_text};

use oriterm_ui::color::Color;
use oriterm_ui::draw::scene::{ContentMask, IconPrimitive, LinePrimitive, Quad, TextRun};
use oriterm_ui::draw::{RectStyle, Scene};
use oriterm_ui::geometry::{Point, Rect};

use super::instance_writer::{InstanceWriter, ScreenRect};
use super::prepare::AtlasLookup;
use super::srgb_f32_to_linear;

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
    /// Font size in 26.6 fixed-point for [`RasterKey`] construction.
    pub size_q6: u32,
    /// Whether hinting is enabled for [`RasterKey`] construction.
    pub hinted: bool,
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
    ui_writer: &mut InstanceWriter,
    text_ctx: Option<&mut TextContext<'_>>,
    scale: f32,
    opacity: f32,
) {
    for quad in scene.quads() {
        let clip = clip_from_mask(&quad.content_mask, scale);
        convert_quad(quad, ui_writer, scale, opacity, clip);
    }
    for line in scene.lines() {
        let clip = clip_from_mask(&line.content_mask, scale);
        convert_scene_line(line, ui_writer, scale, opacity, clip);
    }
    if let Some(ctx) = text_ctx {
        for text in scene.text_runs() {
            let clip = clip_from_mask(&text.content_mask, scale);
            convert_scene_text(text, ctx, scale, opacity, clip);
        }
        for icon in scene.icons() {
            let clip = clip_from_mask(&icon.content_mask, scale);
            convert_scene_icon(icon, ctx, scale, opacity, clip);
        }
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
fn convert_quad(
    quad: &Quad,
    writer: &mut InstanceWriter,
    scale: f32,
    opacity: f32,
    clip: [f32; 4],
) {
    convert_rect_clipped(quad.bounds, &quad.style, writer, scale, opacity, clip);
}

/// Convert a Scene [`LinePrimitive`] to GPU rect instances with clip.
fn convert_scene_line(
    line: &LinePrimitive,
    writer: &mut InstanceWriter,
    scale: f32,
    opacity: f32,
    clip: [f32; 4],
) {
    convert_line_clipped(
        line.from, line.to, line.width, line.color, writer, scale, opacity, clip,
    );
}

/// Convert a Scene [`TextRun`] to glyph instances with clip.
fn convert_scene_text(
    text: &TextRun,
    ctx: &mut TextContext<'_>,
    scale: f32,
    opacity: f32,
    clip: [f32; 4],
) {
    convert_text(
        text.position,
        &text.shaped,
        text.color,
        text.bg_hint,
        ctx,
        scale,
        opacity,
        clip,
    );
}

/// Convert a Scene [`IconPrimitive`] to a glyph instance with clip.
fn convert_scene_icon(
    icon: &IconPrimitive,
    ctx: &mut TextContext<'_>,
    scale: f32,
    opacity: f32,
    clip: [f32; 4],
) {
    convert_icon(
        icon.rect,
        icon.atlas_page,
        icon.uv,
        icon.color,
        ctx,
        scale,
        opacity,
        clip,
    );
}

/// Convert a styled rect to one or two UI rect instances with a per-instance clip.
#[expect(
    clippy::too_many_arguments,
    reason = "rect conversion: bounds, style, output, scale, opacity, clip"
)]
fn convert_rect_clipped(
    rect: Rect,
    style: &RectStyle,
    writer: &mut InstanceWriter,
    scale: f32,
    opacity: f32,
    clip: [f32; 4],
) {
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
        writer.push_ui_rect(
            shadow_rect.scaled(scale),
            color_to_linear_with_opacity(shadow.color, opacity),
            [0.0; 4],
            (uniform_radius(&style.corner_radius) + expand) * scale,
            0.0,
            clip,
        );
    }

    // Main rect instance.
    let screen = to_screen_rect(rect).scaled(scale);
    let (border_color, border_width) = style.border.map_or(([0.0; 4], 0.0), |b| {
        (color_to_linear_with_opacity(b.color, opacity), b.width)
    });

    writer.push_ui_rect(
        screen,
        color_to_linear_with_opacity(fill, opacity),
        border_color,
        uniform_radius(&style.corner_radius) * scale,
        border_width * scale,
        clip,
    );
}

/// Convert a line segment to GPU rect instances with a per-instance clip.
///
/// Axis-aligned lines (horizontal or vertical) produce a single thin rect.
/// Diagonal lines are decomposed into pixel-stepping rects along the major
/// axis — one `width x width` rect per step — to avoid the AABB problem
/// where a single bounding box fills a solid square for 45-degree lines.
#[expect(
    clippy::too_many_arguments,
    reason = "line conversion: endpoints, thickness, color, output, scale, opacity, clip"
)]
fn convert_line_clipped(
    from: Point,
    to: Point,
    width: f32,
    color: Color,
    writer: &mut InstanceWriter,
    scale: f32,
    opacity: f32,
    clip: [f32; 4],
) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = dx.hypot(dy);
    if len < f32::EPSILON {
        return;
    }

    let fill = color_to_linear_with_opacity(color, opacity);
    let hw = width * 0.5;

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
        writer.push_ui_rect(rect, fill, [0.0; 4], 0.0, 0.0, clip);
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
        writer.push_ui_rect(rect, fill, [0.0; 4], 0.0, 0.0, clip);
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
        writer.push_ui_rect(rect, fill, [0.0; 4], 0.0, 0.0, clip);
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

/// Pick a uniform radius from the per-corner array.
///
/// The SDF shader currently supports a single radius value. When per-corner
/// radii differ, use the maximum (visually reasonable until a 4-corner SDF
/// is implemented).
fn uniform_radius(radii: &[f32; 4]) -> f32 {
    radii[0].max(radii[1]).max(radii[2]).max(radii[3])
}

#[cfg(test)]
mod tests;
