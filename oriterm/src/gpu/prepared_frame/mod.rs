//! Prepared frame output from the Prepare phase of the render pipeline.
//!
//! [`PreparedFrame`] holds nine [`InstanceWriter`] buffers plus metadata
//! the Render phase needs to upload and draw. The nine buffers map to nine
//! draw calls in painter's order: backgrounds → mono glyphs → subpixel
//! glyphs → color glyphs → cursors → UI rects → UI mono glyphs →
//! UI subpixel glyphs → UI color glyphs.

use oriterm_core::Rgb;

use super::frame_input::ViewportSize;
use super::instance_writer::InstanceWriter;
use super::srgb_to_linear;

/// GPU-ready frame data produced by the Prepare phase.
///
/// Contains nine instance buffers for the nine rendering layers
/// (drawn in order: backgrounds → mono glyphs → subpixel glyphs →
/// color glyphs → cursors → UI rects → UI mono glyphs → UI subpixel
/// glyphs → UI color glyphs) plus the clear color and total instance
/// count for the Render phase.
///
/// UI text glyphs (draws 7–9) are separate from terminal glyphs (draws
/// 2–4) so they render AFTER UI rect backgrounds (draw 6) instead of
/// being hidden behind them.
pub struct PreparedFrame {
    /// Background rectangle instances (solid-color cell fills).
    pub backgrounds: InstanceWriter,
    /// Monochrome glyph instances (`R8Unorm` atlas, tinted by `fg_color`).
    pub glyphs: InstanceWriter,
    /// LCD subpixel glyph instances (`Rgba8Unorm` atlas, per-channel blend).
    pub subpixel_glyphs: InstanceWriter,
    /// Color glyph instances (`Rgba8Unorm` atlas, rendered as-is).
    pub color_glyphs: InstanceWriter,
    /// Cursor instances (block, bar, underline shapes).
    pub cursors: InstanceWriter,
    /// UI rect instances (SDF rounded rectangles with optional border).
    pub ui_rects: InstanceWriter,
    /// UI monochrome glyph instances (dialog/overlay text, drawn after UI rects).
    pub ui_glyphs: InstanceWriter,
    /// UI subpixel glyph instances (dialog/overlay text, drawn after UI rects).
    pub ui_subpixel_glyphs: InstanceWriter,
    /// UI color glyph instances (dialog/overlay text, drawn after UI rects).
    pub ui_color_glyphs: InstanceWriter,
    /// Viewport pixel dimensions for uniform buffer update.
    pub viewport: ViewportSize,
    /// Window clear color (alpha-premultiplied).
    pub clear_color: [f64; 4],
}

impl PreparedFrame {
    /// Create an empty frame with the given clear color.
    pub fn new(viewport: ViewportSize, background: Rgb, opacity: f64) -> Self {
        Self {
            backgrounds: InstanceWriter::new(),
            glyphs: InstanceWriter::new(),
            subpixel_glyphs: InstanceWriter::new(),
            color_glyphs: InstanceWriter::new(),
            cursors: InstanceWriter::new(),
            ui_rects: InstanceWriter::new(),
            ui_glyphs: InstanceWriter::new(),
            ui_subpixel_glyphs: InstanceWriter::new(),
            ui_color_glyphs: InstanceWriter::new(),
            viewport,
            clear_color: rgb_to_clear(background, opacity),
        }
    }

    /// Create an empty frame pre-allocated for the given grid dimensions.
    ///
    /// `cols * rows` instances are reserved for backgrounds (one per cell),
    /// and the same for glyphs. Cursors are always small (typically 1–2).
    #[cfg(test)]
    pub fn with_capacity(
        viewport: ViewportSize,
        cols: usize,
        rows: usize,
        background: Rgb,
        opacity: f64,
    ) -> Self {
        let cells = cols * rows;
        Self {
            backgrounds: InstanceWriter::with_capacity(cells),
            glyphs: InstanceWriter::with_capacity(cells),
            subpixel_glyphs: InstanceWriter::new(),
            color_glyphs: InstanceWriter::new(),
            cursors: InstanceWriter::with_capacity(4),
            ui_rects: InstanceWriter::new(),
            ui_glyphs: InstanceWriter::new(),
            ui_subpixel_glyphs: InstanceWriter::new(),
            ui_color_glyphs: InstanceWriter::new(),
            viewport,
            clear_color: rgb_to_clear(background, opacity),
        }
    }

    /// Total instance count across all nine buffers.
    #[allow(dead_code, reason = "frame management methods for later sections")]
    pub fn total_instances(&self) -> usize {
        self.backgrounds.len()
            + self.glyphs.len()
            + self.subpixel_glyphs.len()
            + self.color_glyphs.len()
            + self.cursors.len()
            + self.ui_rects.len()
            + self.ui_glyphs.len()
            + self.ui_subpixel_glyphs.len()
            + self.ui_color_glyphs.len()
    }

    /// Whether all nine buffers are empty.
    #[allow(dead_code, reason = "frame management methods for later sections")]
    pub fn is_empty(&self) -> bool {
        self.backgrounds.is_empty()
            && self.glyphs.is_empty()
            && self.subpixel_glyphs.is_empty()
            && self.color_glyphs.is_empty()
            && self.cursors.is_empty()
            && self.ui_rects.is_empty()
            && self.ui_glyphs.is_empty()
            && self.ui_subpixel_glyphs.is_empty()
            && self.ui_color_glyphs.is_empty()
    }

    /// Reset all buffers for the next frame, retaining allocated memory.
    pub fn clear(&mut self) {
        self.backgrounds.clear();
        self.glyphs.clear();
        self.subpixel_glyphs.clear();
        self.color_glyphs.clear();
        self.cursors.clear();
        self.ui_rects.clear();
        self.ui_glyphs.clear();
        self.ui_subpixel_glyphs.clear();
        self.ui_color_glyphs.clear();
    }

    /// Update the clear color (e.g. after a palette change).
    pub fn set_clear_color(&mut self, background: Rgb, opacity: f64) {
        self.clear_color = rgb_to_clear(background, opacity);
    }
}

/// Convert an `Rgb` + opacity to the `[f64; 4]` wgpu expects for clear color.
///
/// Each sRGB byte is decoded via [`srgb_to_linear`] before premultiplication
/// so the clear color is truly linear for the `*Srgb` render target.
fn rgb_to_clear(c: Rgb, opacity: f64) -> [f64; 4] {
    [
        f64::from(srgb_to_linear(c.r)) * opacity,
        f64::from(srgb_to_linear(c.g)) * opacity,
        f64::from(srgb_to_linear(c.b)) * opacity,
        opacity,
    ]
}

#[cfg(test)]
mod tests;
