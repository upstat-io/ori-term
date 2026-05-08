//! Prepare phase: convert a [`FrameInput`] into GPU-ready instance buffers.
//!
//! [`prepare_frame`] is a pure CPU function — no wgpu types, no device, no
//! queue. Given a terminal snapshot and an atlas lookup, it produces a
//! [`PreparedFrame`] containing three [`InstanceWriter`] buffers (backgrounds,
//! glyphs, cursors) ready for GPU upload.
//!
//! The [`AtlasLookup`] trait abstracts glyph lookup for testability: production
//! wraps `FontCollection::resolve` + `GlyphAtlas::lookup`; tests use a simple
//! `HashMap`.

mod decorations;
pub(crate) mod dirty_skip;
mod emit;
mod emit_cell;
mod resolve;
mod shaped_frame;
#[cfg(test)]
mod unshaped;

use oriterm_core::{CellFlags, CursorShape, RenderableCursor};

use super::atlas::AtlasEntry;

use super::frame_input::FrameInput;
use super::prepared_frame::PreparedFrame;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::font::{GlyphStyle, RasterKey};
use dirty_skip::{BufferLengths, RowInstanceRanges, fill_frame_incremental};
use emit::{draw_prompt_markers, draw_url_hover_underline, emit_cursor_for_frame};
use emit_cell::EmitCtx;
use resolve::resolve_cursor;

pub use shaped_frame::ShapedFrame;
#[cfg(test)]
pub(crate) use unshaped::{prepare_frame, prepare_frame_into};

/// Content-aware fingerprint of every frame-level input that affects per-cell
/// instance emission. SSOT for the incremental dispatch decision — one hash +
/// one comparison + one tail write replace the prior parallel sync points.
///
/// Hashed inputs:
/// - viewport (`width`, `height`) — full viewport-pixel dimensions.
/// - full [`CellMetrics`] (`width`, `height`, `baseline`, `underline_offset`,
///   `stroke_size`, `strikeout_offset`) — every metric affects cell or
///   decoration emission. Hashing only the first 3 would silently replay
///   stale decoration geometry on font/scale changes.
/// - content grid dimensions (`content_cols`, `content_rows`).
/// - origin (x, y) — saved-tier rows carry pixel positions baked at emit time.
/// - per-cell alpha multipliers (`text_blink_opacity`, `palette.opacity`,
///   `fg_dim`) — baked into per-cell instances.
/// - `subpixel_positioning` — flips between subpixel/glyphs writers.
/// - `search_fingerprint()` — already content-aware via
///   [`crate::gpu::frame_input::FrameSearch::damage_fingerprint`].
///
/// NOT hashed (handled by per-row dirty tracking in `build_dirty_set` /
/// `WindowRenderer::has_row_state_change`):
/// - selection snapshot, cursor row, hovered cell.
///
/// `f32` fields are hashed via `.to_bits()` (bitwise-exact, not epsilon-
/// tolerant). Tiny float deltas now force full rebuild instead of stale
/// replay; the effect is extra rebuilds, not stale reuse.
pub(super) fn compute_dispatch_fingerprint(input: &FrameInput, origin: (f32, f32)) -> u64 {
    let mut hasher = DefaultHasher::new();

    // Geometry — affects every cell's pixel position.
    input.viewport.width.hash(&mut hasher);
    input.viewport.height.hash(&mut hasher);

    // ALL 6 CellMetrics fields — underline/stroke/strikeout offsets affect
    // decoration emission and must invalidate on font/scale change.
    input.cell_size.width.to_bits().hash(&mut hasher);
    input.cell_size.height.to_bits().hash(&mut hasher);
    input.cell_size.baseline.to_bits().hash(&mut hasher);
    input.cell_size.underline_offset.to_bits().hash(&mut hasher);
    input.cell_size.stroke_size.to_bits().hash(&mut hasher);
    input.cell_size.strikeout_offset.to_bits().hash(&mut hasher);

    input.content_cols.hash(&mut hasher);
    input.content_rows.hash(&mut hasher);
    origin.0.to_bits().hash(&mut hasher);
    origin.1.to_bits().hash(&mut hasher);

    // Per-cell alpha multipliers — baked into per-cell instances at emit time.
    input.text_blink_opacity.to_bits().hash(&mut hasher);
    input.palette.opacity.to_bits().hash(&mut hasher);
    input.fg_dim.to_bits().hash(&mut hasher);

    // Atlas routing — flips between subpixel/glyphs writers in emit.rs.
    u8::from(input.subpixel_positioning).hash(&mut hasher);

    // INVARIANT: hash the Option, not the unwrapped tuple — the discriminant
    // distinguishes None from Some(all-zeros).
    input.search_fingerprint().hash(&mut hasher);

    hasher.finish()
}

/// Resolve the cursor row-state SSOT for the cursor-only fast-path predicate
/// AND the per-frame `prev_resolved_cursor` snapshot.
///
/// Returns the merged terminal-cursor + mark-mode override + window-focus
/// override — exactly as the emit pipeline sees it. The returned cursor's
/// `shape` is the **effective** render shape (focused or Hidden → configured
/// shape; unfocused non-Hidden → `HollowBlock`), so all downstream consumers
/// (per-cell color resolution, fast-path gating, row-dirty membership) read
/// `cursor.shape` directly without re-querying `effective_cursor_shape`. The
/// `visible` flag carries "is the cursor displayed this frame?". Storage
/// sites (`prev_resolved_cursor`) canonicalize invisible cursors to `None`
/// per the visibility-canonicalized storage rule — this helper is the SSOT
/// for the resolution itself; `Option` wrapping is the storage layer's
/// concern.
///
/// Consumers: `prepare_frame_shaped_into` (storage), `update_cursor_only`
/// (storage), `WindowRenderer::has_row_state_change` (fast-path predicate),
/// `fill_frame_shaped` / `prepare_frame_into` (`EmitCtx` build),
/// `dirty_skip::build_dirty_set` (current-frame compute).
pub(super) fn resolve_cursor_state(input: &FrameInput) -> RenderableCursor {
    let mut resolved = resolve_cursor(&input.content.cursor, input.mark_cursor.as_ref());
    resolved.shape = effective_cursor_shape(&resolved, input.window_focused);
    resolved
}

/// Effective render shape for a resolved cursor under window-focus state.
///
/// SSOT for the focused-or-`Hidden` → `cursor.shape` / unfocused-non-`Hidden` →
/// `HollowBlock` policy. Lives in `prepare/mod.rs` (not `oriterm_core`) per
/// the crate-boundaries rule that focus is render-context, not terminal-
/// emulation state. The `Hidden` carve-out preserves the explicit "draw
/// nothing" semantic — `emit.rs:275` matches `CursorShape::Hidden => {}` to
/// suppress emission; converting `Hidden` → `HollowBlock` would cause an
/// invisible cursor to render as a hollow box on focus loss.
#[inline]
pub(super) fn effective_cursor_shape(
    cursor: &RenderableCursor,
    window_focused: bool,
) -> CursorShape {
    if window_focused || cursor.shape == CursorShape::Hidden {
        cursor.shape
    } else {
        CursorShape::HollowBlock
    }
}

/// Snap a row's Y position to integer pixels.
///
/// SSOT for the integer-Y pixel-snap discipline: cell-top y is computed as
/// `(origin_y + row * cell_height).round()` and snapped to integer to
/// preserve sharp glyph edges on fractional-DPI displays. A fractional Y
/// triggers bilinear-filtering blur on cells where `cell_height` is not
/// integer-aligned (e.g. `13.0 * 0.25 = 3.25`).
///
/// Co-locates with [`super_sub_glyph_offset`] (which preserves this snap).
#[inline]
pub(super) fn snapped_row_y(origin_y: f32, row: usize, cell_height: f32) -> f32 {
    (origin_y + row as f32 * cell_height).round()
}

/// Vertical glyph offset (in pixels) for SGR 73 (superscript) / SGR 74 (subscript).
///
/// Returns a SIGNED, INTEGER-ROUNDED pixel offset relative to the cell-top y:
/// negative shifts the glyph upward (super), positive shifts downward (sub),
/// `0.0` when neither flag is set. The `.round()` is load-bearing — it preserves
/// the integer-Y-pixel-snap discipline applied in `fill_frame_shaped` and
/// `fill_frame_incremental`, where the cell-top y is computed as
/// `(oy + row * ch).round()` and snapped to an integer; a fractional super/sub
/// offset would defeat that snap and trigger bilinear-filtering blur on cells
/// whose `cell_height * 0.25` is non-integer (e.g. `13.0 * 0.25 = 3.25`).
///
/// Backgrounds, decorations, and cursors keep the unshifted cell-top y so they
/// remain anchored to the cell rectangle — only glyph y shifts. The 25% factor
/// matches wezterm (`wezterm-gui/src/termwindow/render/screen_line.rs:437-445`).
pub(super) fn super_sub_glyph_offset(flags: CellFlags, cell_height: f32) -> f32 {
    const FACTOR: f32 = 0.25;
    let raw = if flags.contains(CellFlags::SUPERSCRIPT) {
        -cell_height * FACTOR
    } else if flags.contains(CellFlags::SUBSCRIPT) {
        cell_height * FACTOR
    } else {
        0.0
    };
    raw.round()
}

/// Abstracts glyph atlas lookup for testability.
///
/// Production: the shaped path uses [`lookup_key`](Self::lookup_key) for
/// direct `RasterKey` → `AtlasEntry` lookups. Tests may override `lookup`
/// for the per-cell unshaped path.
pub trait AtlasLookup {
    /// Look up a cached glyph entry by character and style.
    ///
    /// Used by the unshaped [`prepare_frame`] test path. Default returns
    /// `None` — production implementations only need [`lookup_key`](Self::lookup_key).
    #[allow(dead_code, reason = "used by test-only unshaped prepare_frame path")]
    fn lookup(&self, _ch: char, _style: GlyphStyle) -> Option<&AtlasEntry> {
        None
    }

    /// Look up a cached glyph entry by [`RasterKey`] (shaped path).
    fn lookup_key(&self, key: RasterKey) -> Option<&AtlasEntry>;
}

/// Convert a [`FrameInput`] into a GPU-ready [`PreparedFrame`] using shaped
/// glyph data.
///
/// Like [`prepare_frame`] but uses pre-shaped glyph positions from a
/// [`ShapedFrame`] instead of per-cell character lookups. This enables
/// ligatures, combining marks, and shaper-driven positioning.
///
/// Used by tests to get a fresh frame. Production uses
/// [`prepare_frame_shaped_into`] for buffer reuse.
#[cfg(test)]
pub fn prepare_frame_shaped(
    input: &FrameInput,
    atlas: &dyn AtlasLookup,
    shaped: &ShapedFrame,
    origin: (f32, f32),
) -> PreparedFrame {
    let cols = input.columns();
    let rows = input.rows();
    let opacity = f64::from(input.palette.opacity);
    let mut frame = PreparedFrame::with_capacity(
        input.viewport,
        cols,
        rows,
        input.palette.background,
        opacity,
    );
    fill_frame_shaped(input, atlas, shaped, &mut frame, origin, 1.0);
    frame
}

/// Convert a [`FrameInput`] into a pre-existing [`PreparedFrame`], reusing
/// its buffer allocations (shaped path).
///
/// Like [`prepare_frame_shaped`] but clears and refills `out` instead of
/// allocating a new frame. The `origin` offset shifts all cell positions
/// (from layout), and `cursor_opacity` gates cursor emission (from
/// application-level blink state).
///
/// When the previous frame's row ranges are available and not all rows are
/// dirty, uses the incremental path: saves the old terminal-tier instances,
/// copies clean rows from the cache, and only regenerates dirty rows.
#[expect(
    clippy::too_many_arguments,
    reason = "origin + cursor opacity are pipeline context, not FrameInput concerns"
)]
pub fn prepare_frame_shaped_into(
    input: &FrameInput,
    atlas: &dyn AtlasLookup,
    shaped: &ShapedFrame,
    out: &mut PreparedFrame,
    origin: (f32, f32),
    cursor_opacity: f32,
) {
    // INVARIANT: save_terminal_tier MUST run before the dispatch decision —
    // without this pre-publish, saved_tier never populates and the
    // incremental path is unreachable. Subsequent calls move the previous
    // frame's terminal-tier into saved_tier for can_incremental.
    out.save_terminal_tier();
    // INVARIANT: save_terminal_tier leaves the live terminal-tier writers
    // empty so the dispatch branches below populate from a clean baseline.
    debug_assert!(
        out.backgrounds.is_empty()
            && out.glyphs.is_empty()
            && out.subpixel_glyphs.is_empty()
            && out.color_glyphs.is_empty(),
        "save_terminal_tier must leave live terminal-tier writers empty"
    );

    // INVARIANT: row-state fields (selection, cursor, hovered_cell) are
    // intentionally excluded from the fingerprint — they're handled per-row
    // by build_dirty_set inside this incremental pass. Full rationale and
    // hashed-input list live on `compute_dispatch_fingerprint`.
    let fingerprint = compute_dispatch_fingerprint(input, origin);
    let can_incremental = !input.content.all_dirty
        && out.saved_tier.has_cached_rows()
        && out.prev_dispatch_fingerprint == Some(fingerprint);

    if can_incremental {
        // INVARIANT: clear_ephemeral_tiers must run on the incremental path —
        // without it, cursor/chrome/overlay tiers accumulate across frames
        // and leave stale glyphs when chrome/overlay content shrinks.
        out.clear_ephemeral_tiers();
        out.image_quads_below.clear();
        out.image_quads_above.clear();
        out.viewport = input.viewport;
        out.set_clear_color(input.palette.background, f64::from(input.palette.opacity));
        out.was_incremental = true;
        fill_frame_incremental(input, atlas, shaped, out, origin, cursor_opacity);
    } else {
        // INVARIANT: terminal-tier double-clear (after save_terminal_tier)
        // is accepted. A clear_non_terminal() helper would create a
        // second sync point that drifts as new fields land on PreparedFrame.
        out.was_incremental = false;
        out.clear();
        out.viewport = input.viewport;
        out.set_clear_color(input.palette.background, f64::from(input.palette.opacity));
        fill_frame_shaped(input, atlas, shaped, out, origin, cursor_opacity);
    }

    // Post-prepare snapshots for the next frame's dispatch + row-state gates.
    out.prev_dispatch_fingerprint = Some(fingerprint);

    let num_rows = input.rows();
    out.prev_selection_snapshot = input
        .selection
        .as_ref()
        .and_then(|s| s.damage_snapshot(num_rows));
    // INVARIANT: `prev_resolved_cursor` is the SSOT for "previous rendered
    // frame's resolved cursor state". Visibility-canonicalized: invisible
    // cursors store as None so hidden-to-hidden position changes are a no-op
    // for the fast-path predicate (None == None). build_dirty_set derives the
    // line component from `Some(c).map(|c| c.line)`.
    out.prev_resolved_cursor = resolve_cursor_state(input).into_visible();
    out.prev_hovered_cell = input.hovered_cell;
}

/// Cursor-blink-only fast path: rebuild only cursor instances.
///
/// All non-cursor content (backgrounds, glyphs, images, chrome, overlays)
/// is already rendered into the content cache texture. This function
/// rebuilds the cursor instance buffer plus other cursor-tier overlays
/// (URL hover underline, prompt markers) that the full prepare path
/// emits to the cursor buffer — without re-emitting them here, those
/// overlays would disappear on every cursor blink.
pub fn update_cursor_only(
    input: &FrameInput,
    out: &mut PreparedFrame,
    origin: (f32, f32),
    cursor_opacity: f32,
) {
    out.cursors.clear();

    emit_cursor_for_frame(input, out, origin, cursor_opacity);

    // INVARIANT: re-emit URL hover underline + prompt markers — they live on
    // the cursors buffer that cursors.clear() above drops.
    let (ox, oy) = origin;
    draw_url_hover_underline(input, out, ox, oy);
    draw_prompt_markers(input, out, ox, oy);

    // SSOT semantics: prev_resolved_cursor MUST reflect the most recent
    // rendered frame, regardless of which prepare path produced it. Without
    // this write, the field semantics drift to "last full-prepare frame" — a
    // foot-gun for any future predicate consumer. Visibility-canonicalized
    // via RenderableCursor::into_visible (invisible → None).
    out.prev_resolved_cursor = resolve_cursor_state(input).into_visible();
}

/// Iterate cells with row-transition tracking, off-screen culling, and
/// per-row range recording. Shared shape between `fill_frame_shaped` and
/// (in spirit) the incremental path's `process_incremental_cells` —
/// extracted from `fill_frame_shaped` to keep that function under the
/// 50-line size cap.
#[expect(
    clippy::too_many_arguments,
    reason = "row-transition state machine exposes all needed pixel geometry + accumulators"
)]
fn emit_row_tracked_cells(
    ctx: &mut EmitCtx<'_>,
    cells: &[oriterm_core::RenderableCell],
    cw: f32,
    ch: f32,
    ox: f32,
    oy: f32,
    viewport_h: f32,
    current_row: &mut usize,
    row_start: &mut BufferLengths,
    row_off_screen: &mut bool,
) {
    for cell in cells {
        if cell
            .flags
            .intersects(CellFlags::WIDE_CHAR_SPACER | CellFlags::LEADING_WIDE_CHAR_SPACER)
        {
            continue;
        }

        let col = cell.column.0;
        let row = cell.line;

        // Record row range on row transition.
        if row != *current_row {
            if *current_row == usize::MAX {
                *row_start = BufferLengths::capture(ctx.frame);
            } else {
                let now = BufferLengths::capture(ctx.frame);
                let ranges = now.range_since(row_start);
                // Fill gaps if rows were skipped (shouldn't happen but defensive).
                while ctx.frame.row_ranges.len() < *current_row {
                    ctx.frame.row_ranges.push(RowInstanceRanges::default());
                }
                ctx.frame.row_ranges.push(ranges);
                *row_start = now;
            }
            *current_row = row;

            // Skip rows entirely outside the render target.
            let row_y = snapped_row_y(oy, row, ch);
            *row_off_screen = row_y + ch < 0.0 || row_y > viewport_h;
        }

        if *row_off_screen {
            continue;
        }

        let x = ox + col as f32 * cw;
        let y = snapped_row_y(oy, row, ch);
        emit_cell::emit_cell(cell, x, y, ctx);
    }

    // Record the final row's range.
    if *current_row != usize::MAX {
        let now = BufferLengths::capture(ctx.frame);
        let ranges = now.range_since(row_start);
        while ctx.frame.row_ranges.len() < *current_row {
            ctx.frame.row_ranges.push(RowInstanceRanges::default());
        }
        ctx.frame.row_ranges.push(ranges);
    }
}

/// Shaped rendering: emit background, glyph, and cursor instances from shaped data.
///
/// Backgrounds and cursors use the same per-cell logic as the unshaped path.
/// Glyphs are driven by the [`ShapedFrame`] col-to-glyph map instead of
/// per-cell character lookups, enabling ligatures and combining marks.
#[expect(
    clippy::too_many_arguments,
    reason = "origin + cursor opacity are pipeline context passed from renderer"
)]
pub(crate) fn fill_frame_shaped(
    input: &FrameInput,
    atlas: &dyn AtlasLookup,
    shaped: &ShapedFrame,
    frame: &mut PreparedFrame,
    origin: (f32, f32),
    cursor_opacity: f32,
) {
    let cw = input.cell_size.width;
    let ch = input.cell_size.height;
    let (ox, oy) = origin;

    // Row-range tracking: snapshot buffer lengths before frame is moved into ctx.
    let viewport_h = frame.viewport.height as f32;
    let mut current_row = usize::MAX;
    let mut row_start = BufferLengths::capture(frame);
    let mut row_off_screen = false;

    let mut ctx = EmitCtx {
        fg_dim: input.fg_dim,
        text_blink_opacity: input.text_blink_opacity,
        subpixel_positioning: input.subpixel_positioning,
        palette: &input.palette,
        sel: input.selection.as_ref(),
        search: input.search.as_ref(),
        cursor: resolve_cursor_state(input),
        cursor_opacity,
        hovered_cell: input.hovered_cell,
        cell_size: &input.cell_size,
        atlas,
        size_q6: shaped.size_q6(),
        frame,
        shaped: Some((shaped, shaped.hinted())),
    };

    emit_row_tracked_cells(
        &mut ctx,
        &input.content.cells,
        cw,
        ch,
        ox,
        oy,
        viewport_h,
        &mut current_row,
        &mut row_start,
        &mut row_off_screen,
    );

    draw_url_hover_underline(input, ctx.frame, ox, oy);
    draw_prompt_markers(input, ctx.frame, ox, oy);

    // Cursor: visibility/opacity gate + focus-effective-shape + build_cursor
    // dispatch all owned by the canonical home in `prepare/emit.rs`.
    emit_cursor_for_frame(input, ctx.frame, origin, cursor_opacity);

    emit::emit_image_quads(input, ctx.frame, ox, oy);
}

#[cfg(test)]
mod tests;
