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
use super::ui_rect_writer::UiRectWriter;

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
        let clip = clip_from_mask(&quad.content_mask, scale);
        let eff = opacity * quad.content_mask.opacity;
        convert_quad(quad, ui_writer, scale, eff, clip);
    }
    for line in scene.lines() {
        let clip = clip_from_mask(&line.content_mask, scale);
        let eff = opacity * line.content_mask.opacity;
        convert_scene_line(line, ui_writer, scale, eff, clip);
    }
    if let Some(ctx) = text_ctx {
        for text in scene.text_runs() {
            let clip = clip_from_mask(&text.content_mask, scale);
            let eff = opacity * text.content_mask.opacity;
            convert_scene_text(text, ctx, scale, eff, clip);
        }
        for icon in scene.icons() {
            let clip = clip_from_mask(&icon.content_mask, scale);
            let eff = opacity * icon.content_mask.opacity;
            convert_scene_icon(icon, ctx, scale, eff, clip);
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
fn convert_quad(quad: &Quad, writer: &mut UiRectWriter, scale: f32, opacity: f32, clip: [f32; 4]) {
    convert_rect_clipped(quad.bounds, &quad.style, writer, scale, opacity, clip);
}

/// Convert a Scene [`LinePrimitive`] to GPU rect instances with clip.
fn convert_scene_line(
    line: &LinePrimitive,
    writer: &mut UiRectWriter,
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
///
/// Populates the full 144-byte per-side border format.
#[expect(
    clippy::too_many_arguments,
    reason = "rect conversion: bounds, style, output, scale, opacity, clip"
)]
fn convert_rect_clipped(
    rect: Rect,
    style: &RectStyle,
    writer: &mut UiRectWriter,
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
#[expect(
    clippy::too_many_arguments,
    reason = "line conversion: endpoints, thickness, color, output, scale, opacity, clip"
)]
fn convert_line_clipped(
    from: Point,
    to: Point,
    width: f32,
    color: Color,
    writer: &mut UiRectWriter,
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
