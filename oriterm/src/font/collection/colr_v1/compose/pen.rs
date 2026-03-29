//! Glyph outline extraction via skrifa's `OutlinePen` to tiny-skia `Path`.
//!
//! Extracts glyph outlines from skrifa and converts them to tiny-skia paths
//! with the combined COLR transform, font scale, and Y-flip applied to each
//! point during collection.

use skrifa::GlyphId;
use skrifa::color::Transform as ColrTransform;
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::OutlinePen;

use super::super::ClipBox;
use super::ComposeCtx;

/// Extract a glyph outline as a tiny-skia `Path` in bitmap coordinates.
pub(super) fn glyph_path(
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
