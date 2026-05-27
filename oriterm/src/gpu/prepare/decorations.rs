//! Text decoration rendering: underlines (single, double, curly, dotted, dashed),
//! strikethrough, and overline.
//!
//! Simple decorations (single, double, strikethrough, overline) are emitted as
//! solid-color rectangles into the background buffer. Patterned decorations
//! (curly, dotted, dashed) are rendered as atlas-cached glyph instances — one
//! instance per cell instead of O(`cell_width`) rect instances.
//!
//! Geometry is derived from font metrics in [`CellMetrics`] — underline
//! position and thickness come from the font's OS/2 and post tables. Overline
//! sits at the cell-top edge with the same stroke thickness as underline.

use oriterm_core::{CellFlags, Rgb};

use crate::font::CellMetrics;
use crate::gpu::builtin_glyphs::decorations::{
    CURLY_GLYPH_ID, DASHED_GLYPH_ID, DOTTED_GLYPH_ID, curly_amplitude, decoration_key,
};
use crate::gpu::instance_writer::{CLIP_UNCLIPPED, GlyphInstance, InstanceWriter, ScreenRect};

use super::AtlasLookup;

/// Frame-level context for decoration rendering.
///
/// Bundles the instance writers, atlas, size key, and font metrics that are
/// invariant across cells within a single frame. Per-cell parameters (flags,
/// colors, position) are passed to [`draw`](Self::draw).
pub(super) struct DecorationContext<'a> {
    pub(super) backgrounds: &'a mut InstanceWriter,
    pub(super) glyphs: &'a mut InstanceWriter,
    pub(super) atlas: &'a dyn AtlasLookup,
    pub(super) size_q6: u32,
    pub(super) metrics: &'a CellMetrics,
    /// Alpha multiplier for all decoration output.
    ///
    /// 1.0 for normal cells. For cells with `CellFlags::BLINK`, set to the
    /// current text blink opacity so decorations fade alongside glyphs.
    pub(super) alpha: f32,
}

/// Per-cell decoration parameters for [`DecorationContext::draw`].
///
/// Bundles the cell's decoration flags, colors, position, width, and
/// hyperlink state — everything that varies cell-to-cell within a frame.
#[derive(Clone, Copy)]
pub(super) struct CellDecoration {
    /// Cell decoration flags (underlines, strikethrough, overline).
    pub(super) flags: CellFlags,
    /// Explicit SGR underline color, if any. Falls back to `fg`.
    pub(super) underline_color: Option<Rgb>,
    /// Foreground color (decoration color when no explicit underline color).
    pub(super) fg: Rgb,
    /// Cell origin x in pixels.
    pub(super) x: f32,
    /// Cell origin y in pixels (cell-top edge).
    pub(super) y: f32,
    /// Cell width in pixels.
    pub(super) cell_width: f32,
    /// Whether the cell carries an OSC 8 hyperlink.
    pub(super) has_hyperlink: bool,
    /// Whether the hyperlink is currently hovered.
    pub(super) is_hovered: bool,
}

/// Stroke geometry + color for a single decoration line.
///
/// Shared by [`DecorationContext::draw_underline`] and the rect-based
/// fallbacks. All fields are `Copy`.
#[derive(Clone, Copy)]
struct StrokeSpec {
    /// Stroke color.
    color: Rgb,
    /// Stroke origin x in pixels.
    x: f32,
    /// Stroke baseline y in pixels.
    y: f32,
    /// Stroke width in pixels.
    w: f32,
    /// Stroke thickness in pixels.
    t: f32,
    /// Alpha multiplier.
    alpha: f32,
}

impl DecorationContext<'_> {
    /// Emit underline and strikethrough decorations for a single cell.
    ///
    /// Fast-path: returns immediately when no decoration flags are set and
    /// the cell has no hyperlink.
    ///
    /// Hyperlink underlines: cells with OSC 8 hyperlinks that lack an explicit
    /// SGR underline get a dotted underline (solid when hovered). Explicit SGR
    /// underlines take priority over the hyperlink decoration.
    ///
    /// Patterned underlines (curly, dotted, dashed) are emitted as glyph
    /// instances from the atlas. If the atlas entry is missing (e.g. in tests),
    /// falls back to per-pixel rect emission.
    pub(super) fn draw(&mut self, cell: CellDecoration) {
        let CellDecoration {
            flags,
            underline_color,
            fg,
            x,
            y,
            cell_width,
            has_hyperlink,
            is_hovered,
        } = cell;
        let has_explicit_underline = flags.intersects(CellFlags::ALL_UNDERLINES);
        let has_strikethrough = flags.contains(CellFlags::STRIKETHROUGH);
        let has_overline = flags.contains(CellFlags::OVERLINE);

        if !has_explicit_underline && !has_strikethrough && !has_hyperlink && !has_overline {
            return;
        }

        let t = self.metrics.stroke_size;
        let underline_y = y + self.metrics.baseline + self.metrics.underline_offset;

        // Hyperlink underline: dotted when idle, solid when hovered.
        // Only emitted when the cell has no explicit SGR underline.
        if has_hyperlink && !has_explicit_underline {
            if is_hovered {
                self.backgrounds.push_rect(
                    ScreenRect {
                        x,
                        y: underline_y,
                        w: cell_width,
                        h: t,
                    },
                    fg,
                    self.alpha,
                );
            } else {
                // Dotted underline for non-hovered hyperlinks.
                if !self.try_atlas_decoration(DOTTED_GLYPH_ID, fg, x, underline_y) {
                    draw_dotted_underline_rects(
                        self.backgrounds,
                        StrokeSpec {
                            color: fg,
                            x,
                            y: underline_y,
                            w: cell_width,
                            t,
                            alpha: self.alpha,
                        },
                    );
                }
            }
        }

        if has_explicit_underline {
            let color = underline_color.unwrap_or(fg);
            self.draw_underline(
                flags,
                StrokeSpec {
                    color,
                    x,
                    y: underline_y,
                    w: cell_width,
                    t,
                    alpha: self.alpha,
                },
            );
        }

        if has_strikethrough {
            let strike_y = y + self.metrics.baseline - self.metrics.strikeout_offset;
            let rect = ScreenRect {
                x,
                y: strike_y,
                w: cell_width,
                h: t,
            };
            self.backgrounds.push_rect(rect, fg, self.alpha);
        }

        if has_overline {
            // Stroke-thickness rect at the cell-top edge. Color = fg (no SGR
            // for "colored overline"). Matches wezterm
            // (wezterm-gui/src/glyphcache.rs:1312-1328).
            let rect = ScreenRect {
                x,
                y,
                w: cell_width,
                h: t,
            };
            self.backgrounds.push_rect(rect, fg, self.alpha);
        }
    }

    /// Dispatch to the appropriate underline style.
    ///
    /// Priority: curly > double > dotted > dashed > single.
    fn draw_underline(&mut self, flags: CellFlags, stroke: StrokeSpec) {
        let StrokeSpec {
            color,
            x,
            y,
            w,
            t,
            alpha,
        } = stroke;
        if flags.contains(CellFlags::CURLY_UNDERLINE) {
            if !self.try_atlas_decoration(CURLY_GLYPH_ID, color, x, y) {
                draw_curly_underline_rects(self.backgrounds, stroke);
            }
        } else if flags.contains(CellFlags::DOUBLE_UNDERLINE) {
            draw_double_underline(self.backgrounds, stroke);
        } else if flags.contains(CellFlags::DOTTED_UNDERLINE) {
            if !self.try_atlas_decoration(DOTTED_GLYPH_ID, color, x, y) {
                draw_dotted_underline_rects(self.backgrounds, stroke);
            }
        } else if flags.contains(CellFlags::DASHED_UNDERLINE) {
            if !self.try_atlas_decoration(DASHED_GLYPH_ID, color, x, y) {
                draw_dashed_underline_rects(self.backgrounds, stroke);
            }
        } else {
            // Single underline (plain UNDERLINE flag).
            self.backgrounds
                .push_rect(ScreenRect { x, y, w, h: t }, color, alpha);
        }
    }

    /// Try to emit a patterned decoration as a single atlas glyph instance.
    ///
    /// Returns `true` if the atlas had the entry and the glyph was emitted,
    /// `false` to signal the caller should fall back to rect emission.
    fn try_atlas_decoration(&mut self, glyph_id: u16, color: Rgb, x: f32, y: f32) -> bool {
        let key = decoration_key(glyph_id, self.size_q6);
        if let Some(entry) = self.atlas.lookup_key(key) {
            // Curly decorations are taller than the underline position —
            // center the bitmap vertically on the underline Y coordinate.
            let glyph_y = if glyph_id == CURLY_GLYPH_ID {
                y - curly_amplitude(self.metrics.stroke_size)
            } else {
                y
            };
            let uv = [entry.uv_x, entry.uv_y, entry.uv_w, entry.uv_h];
            let rect = ScreenRect {
                x,
                y: glyph_y,
                w: entry.width as f32,
                h: entry.height as f32,
            };
            self.glyphs.push_glyph(
                rect,
                GlyphInstance {
                    uv,
                    fg: color,
                    alpha: self.alpha,
                    atlas_page: entry.page,
                    clip: CLIP_UNCLIPPED,
                },
            );
            true
        } else {
            false
        }
    }
}

// ── Rect-based fallbacks (used when atlas entries are unavailable) ──

/// Curly underline fallback: per-pixel sine wave rects.
fn draw_curly_underline_rects(bg: &mut InstanceWriter, stroke: StrokeSpec) {
    let StrokeSpec {
        color,
        x,
        y,
        w,
        t,
        alpha,
    } = stroke;
    let amplitude = curly_amplitude(t);
    let steps = w as usize;
    for dx in 0..steps {
        let phase = (dx as f32 / w) * std::f32::consts::TAU;
        let offset = (phase.sin() * amplitude).round();
        let rect = ScreenRect {
            x: x + dx as f32,
            y: y + offset,
            w: 1.0,
            h: t,
        };
        bg.push_rect(rect, color, alpha);
    }
}

/// Double underline: two lines separated by a gap that scales with thickness.
fn draw_double_underline(bg: &mut InstanceWriter, stroke: StrokeSpec) {
    let StrokeSpec {
        color,
        x,
        y,
        w,
        t,
        alpha,
    } = stroke;
    let gap = (t + 1.0).ceil();
    bg.push_rect(ScreenRect { x, y, w, h: t }, color, alpha);
    bg.push_rect(
        ScreenRect {
            x,
            y: y - gap,
            w,
            h: t,
        },
        color,
        alpha,
    );
}

/// Dotted underline fallback: per-pixel alternating rects.
fn draw_dotted_underline_rects(bg: &mut InstanceWriter, stroke: StrokeSpec) {
    let StrokeSpec {
        color,
        x,
        y,
        w,
        t,
        alpha,
    } = stroke;
    let steps = w as usize;
    for dx in (0..steps).step_by(2) {
        let rect = ScreenRect {
            x: x + dx as f32,
            y,
            w: 1.0,
            h: t,
        };
        bg.push_rect(rect, color, alpha);
    }
}

/// Dashed underline fallback: per-pixel 3-on-2-off rects.
fn draw_dashed_underline_rects(bg: &mut InstanceWriter, stroke: StrokeSpec) {
    let StrokeSpec {
        color,
        x,
        y,
        w,
        t,
        alpha,
    } = stroke;
    let steps = w as usize;
    for dx in 0..steps {
        if dx % 5 < 3 {
            let rect = ScreenRect {
                x: x + dx as f32,
                y,
                w: 1.0,
                h: t,
            };
            bg.push_rect(rect, color, alpha);
        }
    }
}
