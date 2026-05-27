//! Text and icon draw command conversion to GPU glyph instances.
//!
//! Extracted from the parent module to keep `mod.rs` under the 500-line limit.

use oriterm_ui::color::Color;
use oriterm_ui::geometry::{Point, Rect};
use oriterm_ui::text::ShapedText;

use crate::font::{FaceIdx, FontRealm, RasterKey, SyntheticFlags, subpx_bin, subpx_offset};
use crate::gpu::atlas::{AtlasEntry, AtlasKind};
use crate::gpu::instance_writer::{GlyphInstance, GlyphInstanceBg, ScreenRect};

use super::{PaintParams, TextContext};

/// Text payload for [`convert_text`]: position, shaped run, and colors.
#[derive(Clone, Copy)]
pub(super) struct TextDraw<'a> {
    /// Text anchor position in logical pixels (from widget layout).
    pub position: Point,
    /// Shaped glyph run.
    pub shaped: &'a ShapedText,
    /// Foreground color.
    pub color: Color,
    /// Optional background hint for subpixel compositing.
    pub bg_hint: Option<Color>,
}

/// Icon payload for [`convert_icon`]: rect, atlas page, uv, and color.
#[derive(Clone, Copy)]
pub(super) struct IconDraw {
    /// Icon rect in logical pixels.
    pub rect: Rect,
    /// Mono atlas page index.
    pub atlas_page: u32,
    /// Atlas UV coordinates `[x, y, w, h]`.
    pub uv: [f32; 4],
    /// Tint color.
    pub color: Color,
}

/// Glyph placement (physical-pixel cursor + baseline + subpixel bin).
#[derive(Clone, Copy)]
struct GlyphPlacement {
    /// Pen x position in physical pixels.
    cursor_x: f32,
    /// Baseline-origin y in physical pixels.
    base_y: f32,
    /// Font baseline offset.
    baseline: f32,
    /// Subpixel bin index.
    subpx: u8,
}

/// Glyph paint: foreground, optional subpixel background, alpha, clip.
#[derive(Clone, Copy)]
struct GlyphPaint {
    /// Foreground color.
    fg: oriterm_core::Rgb,
    /// Subpixel-compositing background, if known.
    subpixel_bg: Option<oriterm_core::Rgb>,
    /// Combined alpha.
    alpha: f32,
    /// Physical-pixel clip rect.
    clip: [f32; 4],
}

/// Convert a text draw command into glyph instances.
///
/// The text position is in logical pixels (from widget layout). Glyph
/// advances, offsets, bearings, and bitmap dimensions are in physical pixels
/// (from the font collection loaded at physical DPI). We scale the position
/// to physical at the start, then work entirely in physical pixel space —
/// no scaling of glyph bitmap dimensions, which would cause blurriness.
pub(super) fn convert_text(draw: TextDraw<'_>, ctx: &mut TextContext<'_>, paint: PaintParams) {
    let TextDraw {
        position,
        shaped,
        color,
        bg_hint,
    } = draw;
    let PaintParams {
        scale,
        opacity,
        clip,
    } = paint;
    let fg = color_to_rgb(color);
    let subpixel_bg = bg_hint.map(color_to_rgb);
    let alpha = color.a * opacity;
    let baseline = shaped.baseline;

    // Convert logical position to physical. All subsequent values
    // (advances, offsets, bearings, bitmap dims) are already physical.
    //
    // Round base_y to an integer pixel boundary. baseline and bearing_y
    // are already integers, so rounding base_y ensures every glyph's
    // screen rect has integer Y coordinates. Without this, fractional
    // positions (e.g. from centering a dialog in the viewport) cause the
    // bilinear atlas sampler to interpolate the bottom row of glyph pixels
    // with transparent atlas padding, producing a "cut off at bottom" artifact.
    let mut cursor_x = position.x * scale;
    let base_y = (position.y * scale).round();

    for glyph in &shaped.glyphs {
        let advance = glyph.x_advance;

        // Skip advance-only glyphs (spaces: glyph_id=0).
        if glyph.glyph_id == 0 {
            cursor_x += advance;
            continue;
        }

        let cx = if ctx.subpixel_positioning {
            cursor_x
        } else {
            cursor_x.round()
        };
        let subpx = if ctx.subpixel_positioning {
            subpx_bin(cx + glyph.x_offset)
        } else {
            0
        };
        let realm = match shaped.font_source {
            oriterm_ui::text::FontSource::Terminal => FontRealm::Terminal,
            oriterm_ui::text::FontSource::Ui => FontRealm::Ui,
        };
        let key = RasterKey {
            glyph_id: glyph.glyph_id.into(),
            face_idx: FaceIdx(glyph.face_index),
            weight: shaped.weight,
            size_q6: shaped.size_q6,
            synthetic: SyntheticFlags::from_bits_truncate(glyph.synthetic),
            hinted: ctx.hinted,
            subpx_x: subpx,
            font_realm: realm,
        };

        if let Some(entry) = ctx.atlas.lookup_key(key) {
            emit_text_glyph(
                GlyphPlacement {
                    cursor_x,
                    base_y,
                    baseline,
                    subpx,
                },
                glyph,
                entry,
                GlyphPaint {
                    fg,
                    subpixel_bg,
                    alpha,
                    clip,
                },
                ctx,
            );
        }

        cursor_x += advance;
    }
}

/// Emit a single text glyph instance, routing by atlas kind.
///
/// All coordinates are in physical pixels — no scale factor needed. The
/// glyph bitmap dimensions come directly from the atlas entry (rasterized
/// at the font's physical pixel size).
fn emit_text_glyph(
    placement: GlyphPlacement,
    glyph: &oriterm_ui::text::ShapedGlyph,
    entry: &AtlasEntry,
    paint: GlyphPaint,
    ctx: &mut TextContext<'_>,
) {
    let GlyphPlacement {
        cursor_x,
        base_y,
        baseline,
        subpx,
    } = placement;
    let GlyphPaint {
        fg,
        subpixel_bg,
        alpha,
        clip,
    } = paint;
    let absorbed = subpx_offset(subpx);
    let gx = cursor_x + glyph.x_offset - absorbed + entry.bearing_x as f32;
    let gy = base_y + baseline - entry.bearing_y as f32 - glyph.y_offset;
    let uv = [entry.uv_x, entry.uv_y, entry.uv_w, entry.uv_h];
    // All values are physical pixels — no scaling needed.
    let rect = ScreenRect {
        x: gx,
        y: gy,
        w: entry.width as f32,
        h: entry.height as f32,
    };

    match entry.kind {
        AtlasKind::Subpixel => {
            if let Some(bg) = subpixel_bg {
                // Known background — per-channel compositing in the shader.
                ctx.subpixel_writer.push_glyph_with_bg(
                    rect,
                    GlyphInstanceBg {
                        uv,
                        fg,
                        bg,
                        alpha,
                        // UI text has an opaque known background; no SGR
                        // mode-6 per-channel alpha applies here.
                        bg_alpha: 1.0,
                        atlas_page: entry.page,
                        clip,
                    },
                );
            } else {
                // No background hint — fall back to alpha blending.
                ctx.subpixel_writer.push_glyph(
                    rect,
                    GlyphInstance {
                        uv,
                        fg,
                        alpha,
                        atlas_page: entry.page,
                        clip,
                    },
                );
            }
        }
        AtlasKind::Mono => {
            ctx.mono_writer.push_glyph(
                rect,
                GlyphInstance {
                    uv,
                    fg,
                    alpha,
                    atlas_page: entry.page,
                    clip,
                },
            );
        }
        AtlasKind::Color => {
            ctx.color_writer.push_glyph(
                rect,
                GlyphInstance {
                    uv,
                    fg,
                    alpha,
                    atlas_page: entry.page,
                    clip,
                },
            );
        }
    }
}

/// Convert an icon draw command into a mono glyph instance.
///
/// The icon bitmap lives in the mono atlas and is tinted to `color`
/// by the `fg.wgsl` shader (same as monochrome text glyphs).
pub(super) fn convert_icon(draw: IconDraw, ctx: &mut TextContext<'_>, paint: PaintParams) {
    let IconDraw {
        rect,
        atlas_page,
        uv,
        color,
    } = draw;
    let PaintParams {
        scale,
        opacity,
        clip,
    } = paint;
    let fg = color_to_rgb(color);
    let alpha = color.a * opacity;

    // Snap icon quad to integer physical pixels so each texel maps 1:1 to a
    // screen pixel. Without this, bilinear atlas sampling blurs the icon.
    let x = (rect.x() * scale).round();
    let y = (rect.y() * scale).round();
    let w = (rect.width() * scale).round();
    let h = (rect.height() * scale).round();
    let screen = ScreenRect { x, y, w, h };
    ctx.mono_writer.push_glyph(
        screen,
        GlyphInstance {
            uv,
            fg,
            alpha,
            atlas_page,
            clip,
        },
    );
}

/// Convert an [`oriterm_ui::color::Color`] (f32 RGBA) to [`oriterm_core::Rgb`] (u8 RGB).
pub(crate) fn color_to_rgb(c: Color) -> oriterm_core::Rgb {
    oriterm_core::Rgb {
        r: (c.r * 255.0).round() as u8,
        g: (c.g * 255.0).round() as u8,
        b: (c.b * 255.0).round() as u8,
    }
}
