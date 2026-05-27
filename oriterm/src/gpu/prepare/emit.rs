//! Instance emission helpers: shaped glyph emitter, cursor, and overlay markers.
//!
//! Extracted from `prepare/mod.rs` to keep the main module under 500 lines.
//! All functions emit instances into a [`PreparedFrame`].

use oriterm_core::{CursorShape, Rgb};

use super::super::atlas::AtlasKind;
use super::super::frame_input::FrameInput;
use super::super::prepared_frame::PreparedFrame;
use super::{AtlasLookup, resolve_cursor_state};
use crate::font::{FaceIdx, FontRealm, RasterKey, SyntheticFlags, subpx_bin, subpx_offset};
use crate::gpu::instance_writer::{CLIP_UNCLIPPED, GlyphInstance, GlyphInstanceBg, ScreenRect};
use oriterm_ui::text::ShapedGlyph;

/// Prompt marker bar color: subtle blue accent.
const PROMPT_MARKER_COLOR: Rgb = Rgb {
    r: 80,
    g: 140,
    b: 220,
};

/// Prompt marker bar width in pixels.
const PROMPT_MARKER_WIDTH: f32 = 2.0;

/// Frame-level context for shaped glyph emission.
///
/// Bundles the atlas, size key, baseline, and output frame that are invariant
/// across cells. Per-cell parameters (row glyphs, column, position, color)
/// are passed to [`emit`](Self::emit).
pub(super) struct GlyphEmitter<'a> {
    pub baseline: f32,
    pub size_q6: u32,
    pub hinted: bool,
    pub fg_dim: f32,
    pub subpixel_positioning: bool,
    pub atlas: &'a dyn AtlasLookup,
    pub frame: &'a mut PreparedFrame,
}

/// Shaped-glyph source span for [`GlyphEmitter::emit`]: the row's glyph slice,
/// matching column starts, and the base-glyph start index from the col map.
#[derive(Clone, Copy)]
pub(super) struct ShapedSpan<'a> {
    pub row_glyphs: &'a [ShapedGlyph],
    pub col_starts: &'a [usize],
    pub start_idx: usize,
}

/// Per-cell placement for [`GlyphEmitter::emit`]: grid column, screen
/// position, and foreground/background colors.
#[derive(Clone, Copy)]
pub(super) struct EmitPlacement {
    pub col: usize,
    pub x: f32,
    pub y: f32,
    pub fg: Rgb,
    pub bg: Rgb,
}

impl GlyphEmitter<'_> {
    /// Emit glyph instances for a shaped cell: base glyph + any combining marks.
    ///
    /// Starts at `start_idx` in `row_glyphs` (the base glyph from the col map),
    /// then iterates forward while subsequent glyphs share the same `col_start`
    /// (combining marks are contiguous in the shaper output).
    ///
    /// Routing by [`AtlasKind`]:
    /// - `Mono` → `frame.glyphs` (R8 atlas, tinted by `fg_color`).
    /// - `Subpixel` → `frame.subpixel_glyphs` (RGBA atlas, per-channel blend).
    /// - `Color` → `frame.color_glyphs` (RGBA atlas, rendered as-is).
    pub fn emit(&mut self, span: ShapedSpan<'_>, placement: EmitPlacement) {
        let ShapedSpan {
            row_glyphs,
            col_starts,
            start_idx,
        } = span;
        let EmitPlacement { col, x, y, fg, bg } = placement;
        let mut is_first = true;
        for (sg, &cs) in row_glyphs[start_idx..].iter().zip(&col_starts[start_idx..]) {
            // Stop at the first glyph in a different column (combining marks are contiguous).
            if !is_first && cs != col {
                break;
            }
            is_first = false;

            let subpx = if self.subpixel_positioning {
                subpx_bin(sg.x_offset)
            } else {
                0
            };
            let key = RasterKey {
                glyph_id: sg.glyph_id.into(),
                face_idx: FaceIdx(sg.face_index),
                weight: 0,
                size_q6: self.size_q6,
                synthetic: SyntheticFlags::from_bits_truncate(sg.synthetic),
                hinted: self.hinted,
                subpx_x: subpx,
                font_realm: FontRealm::Terminal,
            };
            if let Some(entry) = self.atlas.lookup_key(key) {
                // Apply shaper offsets: x_offset shifts horizontally,
                // y_offset shifts vertically (positive = up in font coords = subtract in screen).
                // Subtract the absorbed subpixel offset to avoid double-counting
                // (once in the rasterized bitmap, once in positioning).
                let absorbed = subpx_offset(subpx);
                let gx = x + entry.bearing_x as f32 + sg.x_offset - absorbed;
                let gy = y + self.baseline - entry.bearing_y as f32 - sg.y_offset;
                let uv = [entry.uv_x, entry.uv_y, entry.uv_w, entry.uv_h];
                let rect = ScreenRect {
                    x: gx,
                    y: gy,
                    w: entry.width as f32,
                    h: entry.height as f32,
                };
                // Subpixel glyphs receive the cell bg for per-channel LCD
                // compositing. The shader's zero-coverage guard (Section 01)
                // prevents cross-cell bleeding from glyph overhang.
                // Mono/color glyphs use push_glyph (shaders ignore bg_color).
                let writer = match entry.kind {
                    AtlasKind::Color => &mut self.frame.color_glyphs,
                    AtlasKind::Subpixel => {
                        self.frame.subpixel_glyphs.push_glyph_with_bg(
                            rect,
                            GlyphInstanceBg {
                                uv,
                                fg,
                                bg,
                                alpha: self.fg_dim,
                                atlas_page: entry.page,
                                clip: CLIP_UNCLIPPED,
                            },
                        );
                        continue;
                    }
                    AtlasKind::Mono => &mut self.frame.glyphs,
                };
                writer.push_glyph(
                    rect,
                    GlyphInstance {
                        uv,
                        fg,
                        alpha: self.fg_dim,
                        atlas_page: entry.page,
                        clip: CLIP_UNCLIPPED,
                    },
                );
            }
        }
    }
}

/// Draw visual prompt markers as thin colored bars at the left margin.
///
/// For each viewport row in `prompt_marker_rows`, emits a 2px-wide
/// colored rectangle at the left edge of the row. Renders into the
/// cursor layer so it appears above cell backgrounds.
pub(super) fn draw_prompt_markers(input: &FrameInput, frame: &mut PreparedFrame, ox: f32, oy: f32) {
    if input.prompt_marker_rows.is_empty() {
        return;
    }
    let ch = input.cell_size.height;
    for &row in &input.prompt_marker_rows {
        let x = ox;
        let y = super::snapped_row_y(oy, row, ch);
        let rect = ScreenRect {
            x,
            y,
            w: PROMPT_MARKER_WIDTH,
            h: ch,
        };
        frame.cursors.push_cursor(rect, PROMPT_MARKER_COLOR, 0.7);
    }
}

/// Emit cursor instances for a frame, owning the visibility/opacity gate +
/// focus-effective-shape policy + `build_cursor` dispatch.
///
/// Single canonical home for the prepare-pipeline's cursor emission step.
/// Callers (`update_cursor_only` fast path, `fill_frame_shaped` full prepare,
/// `fill_frame_incremental` incremental prepare) used to duplicate this
/// 18-line block; extracted here so the visibility / opacity / focus-shape
/// policy lives in one place.
///
/// Cursor is the resolved cursor (mark-mode override applied) — the helper
/// itself calls `resolve_cursor_state(input)`. No-ops when the cursor is
/// invisible or `cursor_opacity <= 0.0`.
pub(super) fn emit_cursor_for_frame(
    input: &FrameInput,
    frame: &mut PreparedFrame,
    origin: (f32, f32),
    cursor_opacity: f32,
) {
    let cursor = resolve_cursor_state(input);
    if !cursor.visible || cursor_opacity <= 0.0 {
        return;
    }
    // resolve_cursor_state baked the focus-effective shape; read directly per
    // its SSOT contract. Re-querying effective_cursor_shape here would
    // duplicate the policy at a consumption site.
    let shape = cursor.shape;
    let cw = input.cell_size.width;
    let ch = input.cell_size.height;
    let (ox, oy) = origin;
    build_cursor(
        frame,
        shape,
        CursorGeometry {
            col: cursor.column.0,
            row: cursor.line,
            cw,
            ch,
            ox,
            oy,
        },
        input.palette.cursor_color,
        cursor_opacity,
    );
}

/// Emit cursor instances into the prepared frame.
///
/// The cursor shape determines the geometry:
/// - `Block` — full cell rectangle.
/// - `Bar` — 2px-wide vertical line at the left edge.
/// - `Underline` — 2px-tall horizontal line at the bottom.
/// - `HollowBlock` — 4 thin outline rectangles (top, bottom, left, right).
/// - `Hidden` — no instances.
///
/// Cursor cell geometry for [`build_cursor`]: grid position, cell size, and
/// origin offset (all physical pixels).
#[derive(Clone, Copy)]
pub(super) struct CursorGeometry {
    pub col: usize,
    pub row: usize,
    pub cw: f32,
    pub ch: f32,
    pub ox: f32,
    pub oy: f32,
}

/// Most callers should use [`emit_cursor_for_frame`] instead. This is
/// the lower-level shape-dispatch primitive; `emit_cursor_for_frame` owns
/// the visibility / opacity / focus-effective-shape policy.
pub(super) fn build_cursor(
    frame: &mut PreparedFrame,
    shape: CursorShape,
    geom: CursorGeometry,
    color: Rgb,
    opacity: f32,
) {
    let CursorGeometry {
        col,
        row,
        cw,
        ch,
        ox,
        oy,
    } = geom;
    let x = ox + col as f32 * cw;
    let y = super::snapped_row_y(oy, row, ch);
    let t = 2.0_f32;

    match shape {
        CursorShape::Block => {
            frame
                .cursors
                .push_cursor(ScreenRect { x, y, w: cw, h: ch }, color, opacity);
        }
        CursorShape::Bar => {
            frame
                .cursors
                .push_cursor(ScreenRect { x, y, w: t, h: ch }, color, opacity);
        }
        CursorShape::Underline => {
            let rect = ScreenRect {
                x,
                y: y + ch - t,
                w: cw,
                h: t,
            };
            frame.cursors.push_cursor(rect, color, opacity);
        }
        CursorShape::HollowBlock => {
            // Top edge.
            frame
                .cursors
                .push_cursor(ScreenRect { x, y, w: cw, h: t }, color, opacity);
            // Bottom edge.
            let rect = ScreenRect {
                x,
                y: y + ch - t,
                w: cw,
                h: t,
            };
            frame.cursors.push_cursor(rect, color, opacity);
            // Left edge.
            frame
                .cursors
                .push_cursor(ScreenRect { x, y, w: t, h: ch }, color, opacity);
            // Right edge.
            let rect = ScreenRect {
                x: x + cw - t,
                y,
                w: t,
                h: ch,
            };
            frame.cursors.push_cursor(rect, color, opacity);
        }
        CursorShape::Hidden => {}
    }
}

/// Draw implicit URL hover underlines as continuous rects per segment.
///
/// Renders into the cursor layer (on top of glyphs) so the underline is
/// not obscured by character pixels that extend into the underline zone
/// (e.g. `/` descenders in `https://`).
pub(super) fn draw_url_hover_underline(
    input: &FrameInput,
    frame: &mut PreparedFrame,
    ox: f32,
    oy: f32,
) {
    if input.hovered_url_segments.is_empty() {
        return;
    }
    let cw = input.cell_size.width;
    let ch = input.cell_size.height;
    let underline_y_offset = input.cell_size.baseline + input.cell_size.underline_offset;
    let t = input.cell_size.stroke_size;
    let fg = input.palette.foreground;

    for &(line, start_col, end_col) in &input.hovered_url_segments {
        let x = ox + start_col as f32 * cw;
        let y = super::snapped_row_y(oy, line, ch) + underline_y_offset;
        let w = (end_col - start_col + 1) as f32 * cw;
        frame
            .cursors
            .push_cursor(ScreenRect { x, y, w, h: t }, fg, 1.0);
    }
}

/// Emit image quads from `RenderableContent`, splitting by z-index.
///
/// Images with negative z-index go to `image_quads_below` (drawn before text),
/// others go to `image_quads_above` (drawn after text).
pub(super) fn emit_image_quads(input: &FrameInput, frame: &mut PreparedFrame, ox: f32, oy: f32) {
    for img in &input.content.images {
        let quad = super::super::prepared_frame::ImageQuad {
            image_id: img.image_id,
            x: ox + img.viewport_x,
            y: oy + img.viewport_y,
            w: img.display_width,
            h: img.display_height,
            uv_x: img.source_x,
            uv_y: img.source_y,
            uv_w: img.source_w,
            uv_h: img.source_h,
            opacity: img.opacity,
        };
        if img.z_index < 0 {
            frame.image_quads_below.push(quad);
        } else {
            frame.image_quads_above.push(quad);
        }
    }
}

/// Emit one image quad per kitty unicode-placeholder cell.
///
/// Reads `RenderableContent::placeholder_cells` directly — no grid rescan.
/// Each placeholder cell produces a single-cell-sized quad at the cell's
/// position; the UV maps to the corresponding slice of the source image
/// based on `image_row` / `image_col` within the recorded
/// `(placement_cols × placement_rows)` grid. Multi-cell placements
/// (`c>1` / `r>1`) emit one quad per cell making up the slice; single-
/// cell placements default to `(1, 1)` and render the full image. The
/// fragment shader clips outside the framebuffer.
pub(super) fn emit_placeholder_quads(
    input: &FrameInput,
    frame: &mut PreparedFrame,
    ox: f32,
    oy: f32,
) {
    if input.content.placeholder_cells.is_empty() {
        return;
    }
    let cw = input.cell_size.width;
    let ch = input.cell_size.height;
    for pc in &input.content.placeholder_cells {
        let x = ox + pc.column.0 as f32 * cw;
        let y = oy + pc.line as f32 * ch;
        // (cols, rows) ≥ 1 invariant: snapshot defaults to (1, 1) for any
        // anchor without a recorded grid; `set_placeholder_anchor_grid`
        // rejects zero values. Division by `cols`/`rows` is therefore safe.
        let cols = pc.placement_cols.max(1);
        let rows = pc.placement_rows.max(1);
        // Clamp the diacritic-encoded `(image_row, image_col)` into the
        // recorded grid before computing UV. Well-formed clients encode
        // `image_col < cols` / `image_row < rows`; a malformed client can
        // emit out-of-range diacritics, in which case un-clamped math
        // produces UV ≥ 1.0 and relies on the wgpu sampler's ClampToEdge
        // mode (image_render bind-group) to avoid wrap-mode artifacts.
        // The clamp is defense-in-depth — render edge-pixel under
        // out-of-range input rather than depending on sampler config.
        let col = pc.image_col.min(cols - 1);
        let row = pc.image_row.min(rows - 1);
        let uv_w = 1.0 / cols as f32;
        let uv_h = 1.0 / rows as f32;
        let uv_x = col as f32 * uv_w;
        let uv_y = row as f32 * uv_h;
        let quad = super::super::prepared_frame::ImageQuad {
            image_id: pc.image_id,
            x,
            y,
            w: cw,
            h: ch,
            uv_x,
            uv_y,
            uv_w,
            uv_h,
            opacity: 1.0,
        };
        // Kitty unicode placeholders default to drawing above text.
        frame.image_quads_above.push(quad);
    }
}
