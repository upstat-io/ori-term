//! CPU compositing of COLR v1 paint commands via tiny-skia.
//!
//! Uses tiny-skia for proper 2D rendering: path clipping, gradient fills,
//! transform stacks, and compositing layers. Glyph outlines are extracted
//! via skrifa's [`OutlinePen`] and converted to tiny-skia paths.

mod brush;

use skrifa::color::{CompositeMode, Transform as ColrTransform};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::OutlinePen;
use skrifa::{FontRef as SkriFontRef, GlyphId, MetadataProvider};

use super::{ClipBox, PaintCommand, ResolvedBrush};
use crate::font::collection::FaceData;

use brush::{make_paint, to_blend_mode};

/// Shared compositing context passed to all rendering helpers.
///
/// Bundles bitmap dimensions, scale factor, clip box, and accumulated
/// transform so they don't need to be passed as individual arguments.
pub(super) struct ComposeCtx<'a> {
    width: u32,
    height: u32,
    scale: f32,
    clip: &'a ClipBox,
    xf: ColrTransform,
}

impl ComposeCtx<'_> {
    /// Uniform scale factor (`size_px` / `units_per_em`).
    pub(super) fn scale(&self) -> f32 {
        self.scale
    }

    /// COLR clip box in font units.
    pub(super) fn clip(&self) -> &ClipBox {
        self.clip
    }

    /// Accumulated COLR transform.
    pub(super) fn transform(&self) -> &ColrTransform {
        &self.xf
    }
}

/// Composite all paint commands onto the RGBA bitmap using tiny-skia.
///
/// Commands are replayed in order with proper transform stack, clip mask
/// management via [`tiny_skia::Mask`], and layer compositing via separate
/// pixmaps.
#[expect(
    clippy::too_many_arguments,
    reason = "module entry point — all parameters are required by the compositor"
)]
#[expect(
    clippy::too_many_lines,
    reason = "linear command dispatch — splitting would obscure the sequential flow"
)]
pub(super) fn composite_commands(
    commands: &[PaintCommand],
    bitmap: &mut [u8],
    width: u32,
    height: u32,
    clip: ClipBox,
    fd: &FaceData,
    size_px: f32,
) {
    let Some(mut pixmap) = tiny_skia::Pixmap::new(width, height) else {
        return;
    };
    let Ok(font) = SkriFontRef::from_index(&fd.bytes, fd.face_index) else {
        return;
    };
    let outlines = font.outline_glyphs();
    let m = font.metrics(Size::unscaled(), LocationRef::default());
    let upem = m.units_per_em as f32;
    let scale = if upem > 0.0 { size_px / upem } else { 1.0 };

    let mut xform_stack = vec![ColrTransform::default()];
    let mut mask_stack: Vec<Option<tiny_skia::Mask>> = Vec::new();
    let mut current_mask: Option<tiny_skia::Mask> = None;
    let mut layer_stack: Vec<(tiny_skia::Pixmap, CompositeMode)> = Vec::new();

    for cmd in commands {
        // Recompute per iteration — transform stack may have changed.
        let ctx = ComposeCtx {
            width,
            height,
            scale,
            clip: &clip,
            xf: accumulated(&xform_stack),
        };
        match cmd {
            PaintCommand::PushTransform(t) => {
                xform_stack.push(ctx.xf * *t);
            }
            PaintCommand::PopTransform => {
                if xform_stack.len() > 1 {
                    xform_stack.pop();
                }
            }
            PaintCommand::PushClipGlyph(glyph_id) => {
                mask_stack.push(current_mask.take());
                current_mask = make_glyph_mask(
                    &outlines,
                    *glyph_id,
                    &ctx,
                    mask_stack.last().and_then(|m| m.as_ref()),
                );
            }
            PaintCommand::PushClipBox(cb) => {
                mask_stack.push(current_mask.take());
                current_mask = make_box_mask(cb, &ctx, mask_stack.last().and_then(|m| m.as_ref()));
            }
            PaintCommand::PopClip => {
                current_mask = mask_stack.pop().flatten();
            }
            PaintCommand::Fill(brush) => {
                fill_brush(&mut pixmap, brush, &ctx, None, current_mask.as_ref());
            }
            PaintCommand::FillGlyph {
                glyph_id,
                brush,
                brush_transform,
            } => {
                if let Some(path) = glyph_path(&outlines, *glyph_id, &ctx) {
                    if let Some(paint) = make_paint(brush, &ctx, brush_transform.as_ref()) {
                        pixmap.fill_path(
                            &path,
                            &paint,
                            tiny_skia::FillRule::Winding,
                            tiny_skia::Transform::identity(),
                            current_mask.as_ref(),
                        );
                    } else {
                        // Direct pixel-by-pixel fill for gradients tiny-skia
                        // can't handle: sweep and two-circle radial.
                        let glyph_mask =
                            make_glyph_mask(&outlines, *glyph_id, &ctx, current_mask.as_ref());
                        let t = if let Some(bt) = brush_transform.as_ref() {
                            *ctx.transform() * *bt
                        } else {
                            *ctx.transform()
                        };
                        fill_direct_on_glyph(&mut pixmap, brush, &ctx, &t, glyph_mask.as_ref());
                    }
                }
            }
            PaintCommand::PushLayer(mode) => {
                if let Some(new_px) = tiny_skia::Pixmap::new(width, height) {
                    let base = std::mem::replace(&mut pixmap, new_px);
                    layer_stack.push((base, *mode));
                }
            }
            PaintCommand::PopLayer => {
                if let Some((mut base, mode)) = layer_stack.pop() {
                    let paint = tiny_skia::PixmapPaint {
                        blend_mode: to_blend_mode(mode),
                        ..tiny_skia::PixmapPaint::default()
                    };
                    base.draw_pixmap(
                        0,
                        0,
                        pixmap.as_ref(),
                        &paint,
                        tiny_skia::Transform::identity(),
                        None,
                    );
                    pixmap = base;
                }
            }
        }
    }

    bitmap.copy_from_slice(pixmap.data());
}

fn accumulated(stack: &[ColrTransform]) -> ColrTransform {
    stack.last().copied().unwrap_or_default()
}

// Outline extraction

/// Extract a glyph outline as a tiny-skia `Path` in bitmap coordinates.
fn glyph_path(
    outlines: &skrifa::outline::OutlineGlyphCollection<'_>,
    glyph_id: GlyphId,
    ctx: &ComposeCtx<'_>,
) -> Option<tiny_skia::Path> {
    let glyph = outlines.get(glyph_id)?;
    let settings =
        skrifa::outline::DrawSettings::unhinted(Size::unscaled(), LocationRef::default());
    let mut pen = SkiaPen::new(ctx.scale, ctx.clip, &ctx.xf);
    glyph.draw(settings, &mut pen).ok()?;
    pen.finish()
}

/// Build a glyph clip mask.
fn make_glyph_mask(
    outlines: &skrifa::outline::OutlineGlyphCollection<'_>,
    glyph_id: GlyphId,
    ctx: &ComposeCtx<'_>,
    parent: Option<&tiny_skia::Mask>,
) -> Option<tiny_skia::Mask> {
    let path = glyph_path(outlines, glyph_id, ctx)?;
    let mut mask = tiny_skia::Mask::new(ctx.width, ctx.height)?;
    mask.fill_path(
        &path,
        tiny_skia::FillRule::Winding,
        true,
        tiny_skia::Transform::identity(),
    );
    if let Some(p) = parent {
        intersect_masks(&mut mask, p);
    }
    Some(mask)
}

/// Build a clip-box mask.
fn make_box_mask(
    cb: &ClipBox,
    ctx: &ComposeCtx<'_>,
    parent: Option<&tiny_skia::Mask>,
) -> Option<tiny_skia::Mask> {
    let path = box_path(cb, ctx.scale, ctx.clip, &ctx.xf)?;
    let mut mask = tiny_skia::Mask::new(ctx.width, ctx.height)?;
    mask.fill_path(
        &path,
        tiny_skia::FillRule::Winding,
        true,
        tiny_skia::Transform::identity(),
    );
    if let Some(p) = parent {
        intersect_masks(&mut mask, p);
    }
    Some(mask)
}

/// Build a rectangular path from a COLR clip box.
fn box_path(
    cb: &ClipBox,
    scale: f32,
    clip: &ClipBox,
    xf: &ColrTransform,
) -> Option<tiny_skia::Path> {
    let corners = [
        (cb.x_min, cb.y_min),
        (cb.x_max, cb.y_min),
        (cb.x_max, cb.y_max),
        (cb.x_min, cb.y_max),
    ];
    let mut b = tiny_skia::PathBuilder::new();
    for (i, &(fx, fy)) in corners.iter().enumerate() {
        let bx = to_bx(fx, fy, scale, clip, xf);
        let by = to_by(fx, fy, scale, clip, xf);
        if i == 0 {
            b.move_to(bx, by);
        } else {
            b.line_to(bx, by);
        }
    }
    b.close();
    b.finish()
}

// Coordinate transform

/// Font-unit X → bitmap X.
pub(super) fn to_bx(fx: f32, fy: f32, scale: f32, clip: &ClipBox, t: &ColrTransform) -> f32 {
    scale * (t.xx * fx + t.xy * fy + t.dx) - clip.x_min
}

/// Font-unit Y → bitmap Y (Y-flipped).
pub(super) fn to_by(fx: f32, fy: f32, scale: f32, clip: &ClipBox, t: &ColrTransform) -> f32 {
    clip.y_max - scale * (t.yx * fx + t.yy * fy + t.dy)
}

// Outline pen

/// Converts skrifa outline commands to a tiny-skia `PathBuilder`.
///
/// Applies the combined COLR transform + scale + Y-flip + bitmap offset
/// to each point during collection.
struct SkiaPen {
    builder: tiny_skia::PathBuilder,
    scale: f32,
    clip_x_min: f32,
    clip_y_max: f32,
    xx: f32,
    xy: f32,
    dx: f32,
    yx: f32,
    yy: f32,
    dy: f32,
}

impl SkiaPen {
    fn new(scale: f32, clip: &ClipBox, xf: &ColrTransform) -> Self {
        Self {
            builder: tiny_skia::PathBuilder::new(),
            scale,
            clip_x_min: clip.x_min,
            clip_y_max: clip.y_max,
            xx: xf.xx,
            xy: xf.xy,
            dx: xf.dx,
            yx: xf.yx,
            yy: xf.yy,
            dy: xf.dy,
        }
    }

    fn bx(&self, fx: f32, fy: f32) -> f32 {
        self.scale * (self.xx * fx + self.xy * fy + self.dx) - self.clip_x_min
    }

    fn by(&self, fx: f32, fy: f32) -> f32 {
        self.clip_y_max - self.scale * (self.yx * fx + self.yy * fy + self.dy)
    }

    fn finish(self) -> Option<tiny_skia::Path> {
        self.builder.finish()
    }
}

impl OutlinePen for SkiaPen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.builder.move_to(self.bx(x, y), self.by(x, y));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.builder.line_to(self.bx(x, y), self.by(x, y));
    }

    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.builder.quad_to(
            self.bx(cx0, cy0),
            self.by(cx0, cy0),
            self.bx(x, y),
            self.by(x, y),
        );
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.builder.cubic_to(
            self.bx(cx0, cy0),
            self.by(cx0, cy0),
            self.bx(cx1, cy1),
            self.by(cx1, cy1),
            self.bx(x, y),
            self.by(x, y),
        );
    }

    fn close(&mut self) {
        self.builder.close();
    }
}

// Brush fill

/// Fill the entire pixmap with a brush (respecting current mask).
fn fill_brush(
    pixmap: &mut tiny_skia::Pixmap,
    brush: &ResolvedBrush,
    ctx: &ComposeCtx<'_>,
    brush_xf: Option<&ColrTransform>,
    mask: Option<&tiny_skia::Mask>,
) {
    // Sweep gradients bypass the shader system — fill directly.
    let t = if let Some(bt) = brush_xf {
        *ctx.transform() * *bt
    } else {
        *ctx.transform()
    };

    // Direct pixel-by-pixel fills for gradients tiny-skia can't handle.
    match brush {
        ResolvedBrush::SweepGradient {
            center,
            start_angle,
            end_angle,
            stops,
            extend,
        } => {
            let cx = to_bx(center[0], center[1], ctx.scale(), ctx.clip(), &t);
            let cy = to_by(center[0], center[1], ctx.scale(), ctx.clip(), &t);
            brush::fill_sweep_direct(
                pixmap,
                cx,
                cy,
                *start_angle,
                *end_angle,
                stops,
                *extend,
                mask,
            );
            return;
        }
        ResolvedBrush::RadialGradient { r0, .. } if *r0 > 0.0 => {
            fill_radial_brush(pixmap, brush, ctx, &t, mask);
            return;
        }
        _ => {}
    }

    let Some(paint) = make_paint(brush, ctx, brush_xf) else {
        return;
    };
    let w = pixmap.width() as f32;
    let h = pixmap.height() as f32;
    let Some(rect) = tiny_skia::Rect::from_xywh(0.0, 0.0, w, h) else {
        return;
    };
    pixmap.fill_rect(rect, &paint, tiny_skia::Transform::identity(), mask);
}

/// Fill with a two-circle radial gradient via `fill_radial_direct`.
fn fill_radial_brush(
    pixmap: &mut tiny_skia::Pixmap,
    brush: &ResolvedBrush,
    ctx: &ComposeCtx<'_>,
    t: &ColrTransform,
    mask: Option<&tiny_skia::Mask>,
) {
    if let ResolvedBrush::RadialGradient {
        c0,
        r0,
        c1,
        r1,
        stops,
        extend,
    } = brush
    {
        let bx0 = to_bx(c0[0], c0[1], ctx.scale(), ctx.clip(), t);
        let by0 = to_by(c0[0], c0[1], ctx.scale(), ctx.clip(), t);
        let bx1 = to_bx(c1[0], c1[1], ctx.scale(), ctx.clip(), t);
        let by1 = to_by(c1[0], c1[1], ctx.scale(), ctx.clip(), t);
        brush::fill_radial_direct(
            pixmap,
            bx0,
            by0,
            r0 * ctx.scale(),
            bx1,
            by1,
            r1 * ctx.scale(),
            stops,
            *extend,
            mask,
        );
    }
}

/// Direct pixel-by-pixel fill on a glyph (sweep or two-circle radial).
fn fill_direct_on_glyph(
    pixmap: &mut tiny_skia::Pixmap,
    brush: &ResolvedBrush,
    ctx: &ComposeCtx<'_>,
    t: &ColrTransform,
    mask: Option<&tiny_skia::Mask>,
) {
    match brush {
        ResolvedBrush::SweepGradient {
            center,
            start_angle,
            end_angle,
            stops,
            extend,
        } => {
            let cx = to_bx(center[0], center[1], ctx.scale(), ctx.clip(), t);
            let cy = to_by(center[0], center[1], ctx.scale(), ctx.clip(), t);
            brush::fill_sweep_direct(
                pixmap,
                cx,
                cy,
                *start_angle,
                *end_angle,
                stops,
                *extend,
                mask,
            );
        }
        ResolvedBrush::RadialGradient { r0, .. } if *r0 > 0.0 => {
            fill_radial_brush(pixmap, brush, ctx, t, mask);
        }
        _ => {}
    }
}

// Mask helpers

/// AND-intersect two masks (pixel-wise alpha multiply).
fn intersect_masks(mask: &mut tiny_skia::Mask, parent: &tiny_skia::Mask) {
    for (m, &p) in mask.data_mut().iter_mut().zip(parent.data().iter()) {
        *m = ((u16::from(*m) * u16::from(p)) / 255) as u8;
    }
}

#[cfg(test)]
mod tests;
