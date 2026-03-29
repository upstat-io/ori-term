//! COLR v1 paint tree collection and CPU compositing via skrifa.
//!
//! Walks the COLR v1 paint graph and collects [`PaintCommand`]s, then
//! composites them on the CPU into premultiplied RGBA bitmaps via
//! [`try_rasterize_colr_v1`]. The [`PaintCollector`] implements skrifa's
//! [`ColorPainter`] trait, recording each operation (solid fill, gradient,
//! transform, clip, layer) into a flat command list. CPAL palette colors are
//! resolved at collection time so the compositing path receives ready-to-use
//! RGBA values.

mod compose;
pub(crate) mod rasterize;

use skrifa::GlyphId;
use skrifa::color::{Brush, ColorPainter, ColorStop, CompositeMode, Extend, Transform};

/// A resolved RGBA color (sRGB, premultiplied alpha).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    /// Premultiply alpha into RGB channels.
    fn premultiply(self) -> Self {
        Self {
            r: self.r * self.a,
            g: self.g * self.a,
            b: self.b * self.a,
            a: self.a,
        }
    }
}

/// Axis-aligned clip box (font units or scaled pixels).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ClipBox {
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
}

impl ClipBox {
    /// Width of the clip box.
    pub fn width(self) -> f32 {
        self.x_max - self.x_min
    }

    /// Height of the clip box.
    pub fn height(self) -> f32 {
        self.y_max - self.y_min
    }
}

/// Gradient color stop with resolved RGBA color.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ResolvedColorStop {
    pub offset: f32,
    pub color: Rgba,
}

/// A resolved brush with CPAL palette colors already looked up.
#[derive(Debug, Clone)]
pub(crate) enum ResolvedBrush {
    /// Solid fill with a single resolved color.
    Solid(Rgba),
    /// Linear gradient between two points.
    LinearGradient {
        p0: [f32; 2],
        p1: [f32; 2],
        stops: Vec<ResolvedColorStop>,
        extend: Extend,
    },
    /// Radial gradient between two circles.
    RadialGradient {
        c0: [f32; 2],
        r0: f32,
        c1: [f32; 2],
        r1: f32,
        stops: Vec<ResolvedColorStop>,
        extend: Extend,
    },
    /// Sweep (conical) gradient around a center point.
    SweepGradient {
        center: [f32; 2],
        start_angle: f32,
        end_angle: f32,
        stops: Vec<ResolvedColorStop>,
        extend: Extend,
    },
}

/// A single operation in the collected COLR v1 paint command list.
///
/// These map 1:1 to the [`ColorPainter`] callbacks. The GPU renderer replays
/// them in order to composite emoji layers via render-to-texture.
#[derive(Debug, Clone)]
#[allow(
    dead_code,
    reason = "variant fields stored for future GPU render-to-texture compositing"
)]
pub(crate) enum PaintCommand {
    /// Push an affine transform onto the transform stack.
    PushTransform(Transform),
    /// Pop the most recent transform.
    PopTransform,
    /// Clip to the outline of the specified glyph.
    PushClipGlyph(GlyphId),
    /// Clip to an axis-aligned bounding box.
    PushClipBox(ClipBox),
    /// Pop the most recent clip.
    PopClip,
    /// Fill the current clip region with the given brush.
    Fill(ResolvedBrush),
    /// Combined clip-to-glyph + fill (with optional brush transform).
    FillGlyph {
        glyph_id: GlyphId,
        brush_transform: Option<Transform>,
        brush: ResolvedBrush,
    },
    /// Push a compositing layer.
    PushLayer(CompositeMode),
    /// Pop the compositing layer.
    PopLayer,
}

/// Result of collecting a COLR v1 glyph's paint tree.
pub(crate) struct ColrV1Glyph {
    /// Paint commands to replay for GPU compositing.
    pub commands: Vec<PaintCommand>,
    /// Bounding box in font units (before size scaling).
    pub clip_box: Option<ClipBox>,
}

/// Collects [`PaintCommand`]s by implementing skrifa's [`ColorPainter`] trait.
///
/// Create one per glyph, call [`ColorGlyph::paint`](skrifa::color::ColorGlyph::paint)
/// with it, then take the collected commands via [`into_commands`](Self::into_commands).
pub(crate) struct PaintCollector {
    commands: Vec<PaintCommand>,
    /// CPAL palette colors (palette index → RGBA).
    palette: Vec<Rgba>,
}

impl PaintCollector {
    /// Create a new collector with the given CPAL palette colors.
    pub fn new(palette: Vec<Rgba>) -> Self {
        Self {
            commands: Vec::with_capacity(32),
            palette,
        }
    }

    /// Consume the collector and return the collected commands.
    pub fn into_commands(self) -> Vec<PaintCommand> {
        self.commands
    }

    /// Resolve a brush, looking up palette colors for all color references.
    fn resolve_brush(&self, brush: &Brush<'_>) -> ResolvedBrush {
        match *brush {
            Brush::Solid {
                palette_index,
                alpha,
            } => {
                let color = self.resolve_color(palette_index, alpha);
                ResolvedBrush::Solid(color)
            }
            Brush::LinearGradient {
                p0,
                p1,
                color_stops,
                extend,
            } => ResolvedBrush::LinearGradient {
                p0: [p0.x, p0.y],
                p1: [p1.x, p1.y],
                stops: self.resolve_stops(color_stops),
                extend,
            },
            Brush::RadialGradient {
                c0,
                r0,
                c1,
                r1,
                color_stops,
                extend,
            } => ResolvedBrush::RadialGradient {
                c0: [c0.x, c0.y],
                r0,
                c1: [c1.x, c1.y],
                r1,
                stops: self.resolve_stops(color_stops),
                extend,
            },
            Brush::SweepGradient {
                c0,
                start_angle,
                end_angle,
                color_stops,
                extend,
            } => ResolvedBrush::SweepGradient {
                center: [c0.x, c0.y],
                start_angle,
                end_angle,
                stops: self.resolve_stops(color_stops),
                extend,
            },
        }
    }

    /// Look up a palette color by index, applying an alpha multiplier.
    fn resolve_color(&self, palette_index: u16, alpha: f32) -> Rgba {
        let idx = palette_index as usize;
        if let Some(&base) = self.palette.get(idx) {
            Rgba {
                r: base.r,
                g: base.g,
                b: base.b,
                a: base.a * alpha,
            }
            .premultiply()
        } else {
            // Missing palette entry → transparent black.
            Rgba {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }
        }
    }

    /// Resolve a slice of skrifa color stops into owned stops with RGBA colors.
    fn resolve_stops(&self, stops: &[ColorStop]) -> Vec<ResolvedColorStop> {
        stops
            .iter()
            .map(|s| ResolvedColorStop {
                offset: s.offset,
                color: self.resolve_color(s.palette_index, s.alpha),
            })
            .collect()
    }
}

impl ColorPainter for PaintCollector {
    fn push_transform(&mut self, transform: Transform) {
        self.commands.push(PaintCommand::PushTransform(transform));
    }

    fn pop_transform(&mut self) {
        self.commands.push(PaintCommand::PopTransform);
    }

    fn push_clip_glyph(&mut self, glyph_id: GlyphId) {
        self.commands.push(PaintCommand::PushClipGlyph(glyph_id));
    }

    fn push_clip_box(&mut self, clip_box: skrifa::raw::types::BoundingBox<f32>) {
        self.commands.push(PaintCommand::PushClipBox(ClipBox {
            x_min: clip_box.x_min,
            y_min: clip_box.y_min,
            x_max: clip_box.x_max,
            y_max: clip_box.y_max,
        }));
    }

    fn pop_clip(&mut self) {
        self.commands.push(PaintCommand::PopClip);
    }

    fn fill(&mut self, brush: Brush<'_>) {
        let resolved = self.resolve_brush(&brush);
        self.commands.push(PaintCommand::Fill(resolved));
    }

    fn fill_glyph(
        &mut self,
        glyph_id: GlyphId,
        brush_transform: Option<Transform>,
        brush: Brush<'_>,
    ) {
        let resolved = self.resolve_brush(&brush);
        self.commands.push(PaintCommand::FillGlyph {
            glyph_id,
            brush_transform,
            brush: resolved,
        });
    }

    fn push_layer(&mut self, composite_mode: CompositeMode) {
        self.commands.push(PaintCommand::PushLayer(composite_mode));
    }

    fn pop_layer(&mut self) {
        self.commands.push(PaintCommand::PopLayer);
    }
}

/// Load CPAL palette 0 from a skrifa font as a `Vec<Rgba>`.
///
/// Falls back to an empty palette if the font has no CPAL table.
pub(super) fn load_palette(font: &skrifa::FontRef<'_>) -> Vec<Rgba> {
    use skrifa::MetadataProvider;
    let palettes = font.color_palettes();
    let Some(palette) = palettes.get(0) else {
        return Vec::new();
    };
    palette
        .colors()
        .iter()
        .map(|c| Rgba {
            r: f32::from(c.red) / 255.0,
            g: f32::from(c.green) / 255.0,
            b: f32::from(c.blue) / 255.0,
            a: f32::from(c.alpha) / 255.0,
        })
        .collect()
}

#[cfg(test)]
mod tests;
