//! Canonical per-cell emit helper for the GPU prepare phase.
//!
//! [`emit_cell`] is the single source of truth for the per-cell emit body:
//! color resolution, background quad, BLINK alpha, decoration draw, glyph-y
//! super/sub offset, and glyph emission. All three fill-frame paths
//! (`fill_frame_shaped`, `fill_frame_incremental`, and the test-only
//! `fill_frame`) consume this helper so any future per-cell change has one
//! canonical site.

use oriterm_core::{CellFlags, RenderableCell, Rgb};

use crate::font::{CellMetrics, GlyphStyle};
use crate::gpu::builtin_glyphs;
use crate::gpu::instance_writer::{CLIP_UNCLIPPED, GlyphInstance, ScreenRect};
use crate::gpu::prepared_frame::PreparedFrame;

use super::AtlasLookup;
use super::decorations::{CellDecoration, DecorationContext};
use super::emit::{EmitPlacement, GlyphEmitter, ShapedSpan};
use super::resolve::{CellColorContext, resolve_cell_colors};
use super::shaped_frame::ShapedFrame;
use super::super_sub_glyph_offset;

/// Frame-level context for per-cell emit.
///
/// Bundles all frame-invariant references and mutable output buffers so
/// [`emit_cell`] takes a single context argument rather than a long positional
/// parameter list. The caller constructs one `EmitCtx` per
/// `fill_frame_*` invocation and passes `&mut ctx` to each `emit_cell` call.
pub(super) struct EmitCtx<'a> {
    /// Foreground alpha multiplier for inactive pane dimming (1.0 = focused).
    pub fg_dim: f32,
    /// Opacity for `CellFlags::BLINK` cells (0.0 = hidden, 1.0 = visible).
    pub text_blink_opacity: f32,
    /// Whether fractional subpixel X positioning is enabled.
    pub subpixel_positioning: bool,
    /// Frame-level color-resolution context consumed by [`resolve_cell_colors`].
    ///
    /// Bundles the five frame-invariant fields (selection, search, cursor,
    /// opacity, palette) that were previously individual `EmitCtx` fields.
    /// Single source of truth — new overlay/color sources add fields here
    /// rather than on `EmitCtx` directly.
    pub color_ctx: CellColorContext<'a>,
    /// Viewport cell under the mouse for hyperlink hover decoration.
    pub hovered_cell: Option<(usize, usize)>,
    /// Font cell metrics (width, height, baseline, underline offsets).
    pub cell_size: &'a CellMetrics,
    /// Atlas lookup (production: `GlyphAtlas`; tests: `HashMap`-backed mock).
    pub atlas: &'a dyn AtlasLookup,
    /// Shaper size key for builtin-glyph raster-key construction.
    ///
    /// `shaped.size_q6()` for production paths; `0` for the test-only unshaped path.
    pub size_q6: u32,
    /// Mutable output frame — receives all instance pushes.
    pub frame: &'a mut PreparedFrame,
    /// Glyph emission strategy: shaped (production) vs. unshaped (test-only).
    ///
    /// `Option<(&ShapedFrame, bool /* hinted */)>` is `Copy`, so pattern-matching
    /// copies the payload out of `ctx`, leaving `ctx.frame` freely mutable within
    /// the same scope — no split-borrow conflict.
    pub shaped: Option<(&'a ShapedFrame, bool)>,
}

/// Computed per-cell values passed to the background, decoration, and glyph helpers.
struct CellEmitState {
    col: usize,
    row: usize,
    x: f32,
    y: f32,
    bg_w: f32,
    fg: Rgb,
    bg: Rgb,
    /// Foreground/glyph alpha: pane dim × blink × SGR mode-6 fg alpha.
    cell_dim: f32,
    /// Background alpha (SGR mode-6 bg alpha, 0..1). `1.0` for opaque cells.
    bg_alpha: f32,
    /// Decoration alpha: blink × SGR mode-6 underline alpha.
    deco_alpha: f32,
    /// Cell-top y + SGR 73/74 super/sub offset — glyph only, not bg or deco.
    glyph_y: f32,
    is_hovered: bool,
}

/// Fully-opaque cell-alpha sentinel (SGR mode-6 alpha 255/255 → 1.0).
const OPAQUE_ALPHA: f32 = 1.0;

/// Push the cell background rectangle when the cell carries a non-default bg
/// color OR an explicit translucent bg alpha.
///
/// The palette background at full opacity is skipped so the window clear color
/// (carrying theme opacity for glass/acrylic) shows through for default-bg
/// cells. A default-COLORED cell with explicit SGR mode-6 `bg_alpha < 1.0`
/// still emits its bg quad — otherwise its translucent alpha would be dropped
/// and the cell would fall back to the opaque clear color.
fn emit_cell_bg(state: &CellEmitState, ctx: &mut EmitCtx<'_>) {
    if state.bg != ctx.color_ctx.palette.background || state.bg_alpha < OPAQUE_ALPHA {
        ctx.frame.backgrounds.push_rect(
            ScreenRect {
                x: state.x,
                y: state.y,
                w: state.bg_w,
                h: ctx.cell_size.height,
            },
            state.bg,
            state.bg_alpha,
        );
    }
}

/// Draw cell decorations (underlines, strikethrough, overline) anchored to cell-top y.
fn emit_cell_decorations(cell: &RenderableCell, state: &CellEmitState, ctx: &mut EmitCtx<'_>) {
    DecorationContext {
        backgrounds: &mut ctx.frame.backgrounds,
        glyphs: &mut ctx.frame.glyphs,
        atlas: ctx.atlas,
        size_q6: ctx.size_q6,
        metrics: ctx.cell_size,
        alpha: state.deco_alpha,
    }
    .draw(CellDecoration {
        flags: cell.flags,
        underline_color: cell.underline_color,
        fg: state.fg,
        x: state.x,
        y: state.y,
        cell_width: state.bg_w,
        has_hyperlink: cell.has_hyperlink,
        is_hovered: state.is_hovered,
    });
}

/// Emit glyph instances for a cell — dispatches shaped vs. unshaped path.
///
/// Shaped path: builtin-glyph branch (direct `lookup_key`, early return) then
/// [`GlyphEmitter`] for shaped atlas glyphs. Unshaped path (test-only): direct
/// `atlas.lookup` by character.
fn emit_cell_glyphs(cell: &RenderableCell, state: &CellEmitState, ctx: &mut EmitCtx<'_>) {
    if let Some((shaped, hinted)) = ctx.shaped {
        if crate::font::is_builtin(cell.ch) {
            let key = builtin_glyphs::raster_key(cell.ch, ctx.size_q6);
            if let Some(entry) = ctx.atlas.lookup_key(key) {
                let uv = [entry.uv_x, entry.uv_y, entry.uv_w, entry.uv_h];
                let rect = ScreenRect {
                    x: state.x,
                    y: state.glyph_y,
                    w: entry.width as f32,
                    h: entry.height as f32,
                };
                ctx.frame.glyphs.push_glyph(
                    rect,
                    GlyphInstance {
                        uv,
                        fg: state.fg,
                        alpha: state.cell_dim,
                        atlas_page: entry.page,
                        clip: CLIP_UNCLIPPED,
                    },
                );
            }
            return;
        }

        if state.row >= shaped.rows() || state.col >= shaped.cols() {
            return;
        }
        if let Some(start_idx) = shaped.col_map(state.row, state.col) {
            let row_glyphs = shaped.row_glyphs(state.row);
            let row_col_starts = shaped.row_col_starts(state.row);
            GlyphEmitter {
                baseline: ctx.cell_size.baseline,
                size_q6: ctx.size_q6,
                hinted,
                fg_dim: state.cell_dim,
                bg_alpha: state.bg_alpha,
                subpixel_positioning: ctx.subpixel_positioning,
                atlas: ctx.atlas,
                frame: ctx.frame,
            }
            .emit(
                ShapedSpan {
                    row_glyphs,
                    col_starts: row_col_starts,
                    start_idx,
                },
                EmitPlacement {
                    col: state.col,
                    x: state.x,
                    y: state.glyph_y,
                    fg: state.fg,
                    bg: state.bg,
                },
            );
        }
    } else {
        // Test-only unshaped path: direct atlas lookup by character.
        if cell.ch == ' ' {
            return;
        }
        let style = GlyphStyle::from_cell_flags(cell.flags);
        if let Some(entry) = ctx.atlas.lookup(cell.ch, style) {
            let glyph_x = state.x + entry.bearing_x as f32;
            let gy = state.glyph_y + ctx.cell_size.baseline - entry.bearing_y as f32;
            let uv = [entry.uv_x, entry.uv_y, entry.uv_w, entry.uv_h];
            let rect = ScreenRect {
                x: glyph_x,
                y: gy,
                w: entry.width as f32,
                h: entry.height as f32,
            };
            ctx.frame.glyphs.push_glyph(
                rect,
                GlyphInstance {
                    uv,
                    fg: state.fg,
                    alpha: state.cell_dim,
                    atlas_page: entry.page,
                    clip: CLIP_UNCLIPPED,
                },
            );
        }
    }
}

/// Emit all per-cell GPU instances for one visible cell.
///
/// Computes per-cell state (colors, bg width, BLINK alpha, glyph y-offset), then
/// delegates to [`emit_cell_bg`], [`emit_cell_decorations`], and [`emit_cell_glyphs`].
///
/// The caller is responsible for the spacer skip and the x/y computation
/// (including row-boundary y-rounding and row-off-screen checks).
pub(super) fn emit_cell(cell: &RenderableCell, x: f32, y: f32, ctx: &mut EmitCtx<'_>) {
    let col = cell.column.0;
    let row = cell.line;
    let (fg, bg) = resolve_cell_colors(cell, &ctx.color_ctx);
    let bg_w = if cell.flags.contains(CellFlags::WIDE_CHAR) {
        2.0 * ctx.cell_size.width
    } else {
        ctx.cell_size.width
    };
    let is_blink = cell.flags.contains(CellFlags::BLINK);
    // Blink × cell-alpha compose by MULTIPLY (Decision 09): blink attenuation
    // and SGR mode-6 per-channel alpha are independent factors; neither
    // clobbers the other.
    let blink_mul = if is_blink {
        ctx.text_blink_opacity
    } else {
        1.0
    };
    let fg_alpha = f32::from(cell.fg_alpha) / 255.0;
    let bg_alpha = f32::from(cell.bg_alpha) / 255.0;
    let underline_alpha = f32::from(cell.underline_alpha) / 255.0;
    let state = CellEmitState {
        col,
        row,
        x,
        y,
        bg_w,
        fg,
        bg,
        cell_dim: ctx.fg_dim * blink_mul * fg_alpha,
        bg_alpha,
        deco_alpha: ctx.fg_dim * blink_mul * underline_alpha,
        glyph_y: y + super_sub_glyph_offset(cell.flags, ctx.cell_size.height),
        is_hovered: ctx.hovered_cell == Some((row, col)),
    };
    emit_cell_bg(&state, ctx);
    emit_cell_decorations(cell, &state, ctx);
    emit_cell_glyphs(cell, &state, ctx);
}

#[cfg(test)]
mod tests;
