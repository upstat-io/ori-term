//! Prepared frame output from the Prepare phase of the render pipeline.
//!
//! [`PreparedFrame`] holds thirteen [`InstanceWriter`] buffers plus metadata
//! the Render phase needs to upload and draw. The thirteen buffers map to
//! thirteen draw calls in painter's order: backgrounds → mono glyphs →
//! subpixel glyphs → color glyphs → cursors → UI rects → UI mono glyphs →
//! UI subpixel glyphs → UI color glyphs → overlay rects → overlay mono
//! glyphs → overlay subpixel glyphs → overlay color glyphs.

use oriterm_core::Rgb;
use oriterm_core::image::ImageId;

use super::draw_list_convert::TierClips;
use super::frame_input::ViewportSize;
use super::instance_writer::InstanceWriter;
use super::prepare::dirty_skip::{RowInstanceRanges, SavedTerminalTier};
use super::srgb_to_linear;

/// Instance index ranges for a single overlay's content within the shared buffers.
///
/// Each overlay's rects and glyphs occupy a contiguous range in the shared
/// overlay instance writers. The render pass draws each overlay as a complete
/// unit (rects → mono → subpixel → color) before moving to the next, ensuring
/// correct z-ordering between stacked overlays.
#[derive(Debug, Clone, Default)]
pub struct OverlayDrawRange {
    /// `[start..end)` in `overlay_rects`.
    pub rects: (u32, u32),
    /// `[start..end)` in `overlay_glyphs` (mono).
    pub mono: (u32, u32),
    /// `[start..end)` in `overlay_subpixel_glyphs`.
    pub subpixel: (u32, u32),
    /// `[start..end)` in `overlay_color_glyphs`.
    pub color: (u32, u32),
    /// Clip segments for this overlay (absolute instance offsets).
    pub clips: TierClips,
}

/// A single image quad ready for GPU rendering.
///
/// Each image maps to one draw call because each has its own texture.
#[derive(Debug, Clone, Copy)]
pub struct ImageQuad {
    /// Image ID for texture lookup in [`ImageTextureCache`](super::image_render::ImageTextureCache).
    pub image_id: ImageId,
    /// Pixel position (top-left corner).
    pub x: f32,
    /// Pixel position (top-left corner).
    pub y: f32,
    /// Pixel width.
    pub w: f32,
    /// Pixel height.
    pub h: f32,
    /// UV source rect origin.
    pub uv_x: f32,
    /// UV source rect origin.
    pub uv_y: f32,
    /// UV source rect width.
    pub uv_w: f32,
    /// UV source rect height.
    pub uv_h: f32,
    /// Opacity (0.0–1.0).
    pub opacity: f32,
}

/// GPU-ready frame data produced by the Prepare phase.
///
/// Contains thirteen instance buffers in four tiers, drawn in painter's order:
///
/// **Terminal tier** (draws 1–5): backgrounds, mono/subpixel/color glyphs, cursors.
/// **Chrome tier** (draws 6–9): UI rects, UI mono/subpixel/color glyphs.
/// **Overlay tier** (draws 10–13): overlay rects, overlay mono/subpixel/color glyphs.
///
/// The overlay tier is separate from the chrome tier so that overlay content
/// (context menus, dialogs) renders ON TOP of all chrome text (tab bar titles).
/// Without this separation, chrome text from draws 7–9 would paint over overlay
/// rect backgrounds from draw 6, since all UI rects shared a single buffer.
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
    /// UI rect instances (SDF rounded rectangles — chrome layer).
    pub ui_rects: InstanceWriter,
    /// UI monochrome glyph instances (chrome text, drawn after UI rects).
    pub ui_glyphs: InstanceWriter,
    /// UI subpixel glyph instances (chrome text, drawn after UI rects).
    pub ui_subpixel_glyphs: InstanceWriter,
    /// UI color glyph instances (chrome text, drawn after UI rects).
    pub ui_color_glyphs: InstanceWriter,
    /// Overlay rect instances (SDF rounded rectangles — overlay layer, above chrome text).
    pub overlay_rects: InstanceWriter,
    /// Overlay monochrome glyph instances (drawn after overlay rects).
    pub overlay_glyphs: InstanceWriter,
    /// Overlay subpixel glyph instances (drawn after overlay rects).
    pub overlay_subpixel_glyphs: InstanceWriter,
    /// Overlay color glyph instances (drawn after overlay rects).
    pub overlay_color_glyphs: InstanceWriter,
    /// Clip segments for the chrome tier (draws 6–9), one per writer.
    pub ui_clips: TierClips,
    /// Clip segments for the overlay tier (draws 10–13), one per writer.
    pub overlay_clips: TierClips,
    /// Per-overlay draw ranges for correct z-ordering between stacked overlays.
    ///
    /// Each entry corresponds to one overlay (back-to-front order). The render
    /// pass draws each overlay as a complete unit before moving to the next.
    pub overlay_draw_ranges: Vec<OverlayDrawRange>,
    /// Image quads below text (`z_index` < 0).
    pub image_quads_below: Vec<ImageQuad>,
    /// Image quads above text (`z_index` >= 0).
    pub image_quads_above: Vec<ImageQuad>,
    /// Per-row instance byte ranges in the terminal-tier buffers.
    ///
    /// Index = viewport line. Used by the incremental prepare path to copy
    /// clean rows' instances from the previous frame without regenerating them.
    pub row_ranges: Vec<RowInstanceRanges>,
    /// Saved terminal-tier data from the previous frame for incremental updates.
    ///
    /// Swapped out at the start of each prepare pass; clean rows copy from here.
    pub(crate) saved_tier: SavedTerminalTier,
    /// Selection line range from the previous frame for damage tracking.
    ///
    /// `(start_line, end_line)` inclusive viewport lines. Used by the
    /// incremental path to detect which rows changed selection state.
    /// Persists across `clear()` and `save_terminal_tier()`.
    pub(crate) prev_selection_range: Option<(usize, usize)>,
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
            overlay_rects: InstanceWriter::new(),
            overlay_glyphs: InstanceWriter::new(),
            overlay_subpixel_glyphs: InstanceWriter::new(),
            overlay_color_glyphs: InstanceWriter::new(),
            ui_clips: TierClips::default(),
            overlay_clips: TierClips::default(),
            overlay_draw_ranges: Vec::new(),
            image_quads_below: Vec::new(),
            image_quads_above: Vec::new(),
            row_ranges: Vec::new(),
            saved_tier: SavedTerminalTier::new(),
            prev_selection_range: None,
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
            overlay_rects: InstanceWriter::new(),
            overlay_glyphs: InstanceWriter::new(),
            overlay_subpixel_glyphs: InstanceWriter::new(),
            overlay_color_glyphs: InstanceWriter::new(),
            ui_clips: TierClips::default(),
            overlay_clips: TierClips::default(),
            overlay_draw_ranges: Vec::new(),
            image_quads_below: Vec::new(),
            image_quads_above: Vec::new(),
            row_ranges: Vec::new(),
            saved_tier: SavedTerminalTier::new(),
            prev_selection_range: None,
            viewport,
            clear_color: rgb_to_clear(background, opacity),
        }
    }

    /// Total instance count across all thirteen buffers.
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
            + self.overlay_rects.len()
            + self.overlay_glyphs.len()
            + self.overlay_subpixel_glyphs.len()
            + self.overlay_color_glyphs.len()
    }

    /// Whether all thirteen buffers are empty.
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
            && self.overlay_rects.is_empty()
            && self.overlay_glyphs.is_empty()
            && self.overlay_subpixel_glyphs.is_empty()
            && self.overlay_color_glyphs.is_empty()
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
        self.overlay_rects.clear();
        self.overlay_glyphs.clear();
        self.overlay_subpixel_glyphs.clear();
        self.overlay_color_glyphs.clear();
        self.ui_clips.clear();
        self.overlay_clips.clear();
        self.overlay_draw_ranges.clear();
        self.image_quads_below.clear();
        self.image_quads_above.clear();
        self.row_ranges.clear();
    }

    /// Append all instances from `other` into this frame.
    ///
    /// Copies instances from each of the thirteen buffers. Viewport and
    /// clear color are NOT copied — they belong to the target frame.
    pub fn extend_from(&mut self, other: &Self) {
        self.backgrounds.extend_from(&other.backgrounds);
        self.glyphs.extend_from(&other.glyphs);
        self.subpixel_glyphs.extend_from(&other.subpixel_glyphs);
        self.color_glyphs.extend_from(&other.color_glyphs);
        self.cursors.extend_from(&other.cursors);
        self.ui_rects.extend_from(&other.ui_rects);
        self.ui_glyphs.extend_from(&other.ui_glyphs);
        self.ui_subpixel_glyphs
            .extend_from(&other.ui_subpixel_glyphs);
        self.ui_color_glyphs.extend_from(&other.ui_color_glyphs);
        self.overlay_rects.extend_from(&other.overlay_rects);
        self.overlay_glyphs.extend_from(&other.overlay_glyphs);
        self.overlay_subpixel_glyphs
            .extend_from(&other.overlay_subpixel_glyphs);
        self.overlay_color_glyphs
            .extend_from(&other.overlay_color_glyphs);
        self.ui_clips.extend_from(
            &other.ui_clips,
            [
                self.ui_rects.len() as u32,
                self.ui_glyphs.len() as u32,
                self.ui_subpixel_glyphs.len() as u32,
                self.ui_color_glyphs.len() as u32,
            ],
        );
        self.overlay_clips.extend_from(
            &other.overlay_clips,
            [
                self.overlay_rects.len() as u32,
                self.overlay_glyphs.len() as u32,
                self.overlay_subpixel_glyphs.len() as u32,
                self.overlay_color_glyphs.len() as u32,
            ],
        );
        // Shift per-overlay draw ranges by current buffer lengths.
        let bases = [
            self.overlay_rects.len() as u32,
            self.overlay_glyphs.len() as u32,
            self.overlay_subpixel_glyphs.len() as u32,
            self.overlay_color_glyphs.len() as u32,
        ];
        for range in &other.overlay_draw_ranges {
            let mut shifted = range.clone();
            shifted.rects = (range.rects.0 + bases[0], range.rects.1 + bases[0]);
            shifted.mono = (range.mono.0 + bases[1], range.mono.1 + bases[1]);
            shifted.subpixel = (range.subpixel.0 + bases[2], range.subpixel.1 + bases[2]);
            shifted.color = (range.color.0 + bases[3], range.color.1 + bases[3]);
            shifted.clips.shift_offsets(bases);
            self.overlay_draw_ranges.push(shifted);
        }
        self.image_quads_below
            .extend_from_slice(&other.image_quads_below);
        self.image_quads_above
            .extend_from_slice(&other.image_quads_above);
    }

    /// Shrink all instance buffers and scratch Vecs if capacity vastly exceeds usage.
    ///
    /// Called after rendering to bound memory waste. See [`InstanceWriter::maybe_shrink`].
    pub fn maybe_shrink(&mut self) {
        self.backgrounds.maybe_shrink();
        self.glyphs.maybe_shrink();
        self.subpixel_glyphs.maybe_shrink();
        self.color_glyphs.maybe_shrink();
        self.cursors.maybe_shrink();
        self.ui_rects.maybe_shrink();
        self.ui_glyphs.maybe_shrink();
        self.ui_subpixel_glyphs.maybe_shrink();
        self.ui_color_glyphs.maybe_shrink();
        self.overlay_rects.maybe_shrink();
        self.overlay_glyphs.maybe_shrink();
        self.overlay_subpixel_glyphs.maybe_shrink();
        self.overlay_color_glyphs.maybe_shrink();
        maybe_shrink_vec(&mut self.overlay_draw_ranges);
        maybe_shrink_vec(&mut self.image_quads_below);
        maybe_shrink_vec(&mut self.image_quads_above);
        maybe_shrink_vec(&mut self.row_ranges);
    }

    /// Update the clear color (e.g. after a palette change).
    pub fn set_clear_color(&mut self, background: Rgb, opacity: f64) {
        self.clear_color = rgb_to_clear(background, opacity);
    }

    /// Swap the terminal-tier buffers into `saved_tier` for incremental updates.
    ///
    /// After this call the terminal-tier writers are empty (backed by the
    /// previous `saved_tier`'s Vecs, cleared to zero length). The old
    /// instances live in `saved_tier` and can be copied row-by-row for
    /// clean rows. This is O(1) — three pointer swaps, no data copied.
    pub(crate) fn save_terminal_tier(&mut self) {
        self.backgrounds.swap_buf(&mut self.saved_tier.backgrounds);
        self.glyphs.swap_buf(&mut self.saved_tier.glyphs);
        self.subpixel_glyphs
            .swap_buf(&mut self.saved_tier.subpixel_glyphs);
        self.color_glyphs
            .swap_buf(&mut self.saved_tier.color_glyphs);
        std::mem::swap(&mut self.row_ranges, &mut self.saved_tier.row_ranges);

        // Clear the now-swapped-in buffers (they held the old saved_tier data).
        self.backgrounds.clear();
        self.glyphs.clear();
        self.subpixel_glyphs.clear();
        self.color_glyphs.clear();
        self.row_ranges.clear();
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

/// Shrink a Vec if capacity vastly exceeds usage (> 4× len and > 4096 elements).
fn maybe_shrink_vec<T>(v: &mut Vec<T>) {
    let cap = v.capacity();
    let len = v.len();
    if cap > 4 * len && cap > 4096 {
        v.shrink_to(len * 2);
    }
}

#[cfg(test)]
mod tests;
