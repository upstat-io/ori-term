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

use oriterm_core::{CellFlags, CursorShape};

use super::atlas::AtlasEntry;

use super::frame_input::FrameInput;
use super::prepared_frame::PreparedFrame;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::font::{GlyphStyle, RasterKey};
use dirty_skip::{BufferLengths, RowInstanceRanges, fill_frame_incremental};
use emit::{build_cursor, draw_prompt_markers, draw_url_hover_underline};
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

    // Search highlight state — search match colors baked at emit time.
    // Hash via the Option directly so the discriminant (None vs Some) is
    // structurally distinguished from the inner tuple's bytes, even when the
    // inner tuple happens to be all-zeros.
    input.search_fingerprint().hash(&mut hasher);

    hasher.finish()
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
    // Publish the previous frame's terminal-tier into saved_tier BEFORE
    // the dispatch decision. Without this bootstrap, the incremental path
    // is unreachable — saved_tier would only populate INSIDE the
    // incremental branch itself, so frame 0 leaves saved_tier empty and
    // every subsequent frame falls through to full rebuild. First call:
    // terminal-tier and saved_tier both empty, swap is a no-op.
    // Subsequent calls: previous frame's terminal-tier moves into
    // saved_tier, making it visible to the can_incremental check.
    out.save_terminal_tier();
    // Invariant: save_terminal_tier swaps the live terminal-tier writers
    // out into saved_tier and clears the live writers — the live writers
    // MUST be empty here for the dispatch branches below to populate
    // them from a clean baseline.
    debug_assert!(
        out.backgrounds.is_empty()
            && out.glyphs.is_empty()
            && out.subpixel_glyphs.is_empty()
            && out.color_glyphs.is_empty(),
        "save_terminal_tier must leave live terminal-tier writers empty"
    );

    // Single content-aware fingerprint replaces the prior 11-clause
    // enumerated predicate. Every frame-level input that
    // affects per-cell instance emission is hashed into one u64 via
    // `compute_dispatch_fingerprint` — viewport, full CellMetrics,
    // content dims, origin, blink/palette/dim opacities, subpixel
    // positioning, search state. Adding a new invalidating input now
    // requires updating exactly one location (the helper) instead of
    // three parallel sync points (predicate + tail + struct field).
    //
    // Excluded from the fingerprint (handled by per-row `build_dirty_set`,
    // not full rebuild):
    //   - selection / cursor / hovered_cell — row-localized changes
    //   - palette colors beyond opacity — color drift via grid `damage`
    //
    // The fingerprint is bitwise-exact (`.to_bits()` for f32 fields);
    // small float deltas now force full rebuild rather than incremental
    // replay. The effect is extra rebuilds, not stale reuse.
    let fingerprint = compute_dispatch_fingerprint(input, origin);
    let can_incremental = !input.content.all_dirty
        && out.saved_tier.has_cached_rows()
        && out.prev_dispatch_fingerprint == Some(fingerprint);

    if can_incremental {
        // Incremental path: saved_tier is already populated by the
        // unconditional save_terminal_tier above. clear_ephemeral_tiers
        // drops cursor, chrome, and overlay tiers — without it those
        // buffers accumulate frame-after-frame on this path, leaving
        // stale glyphs visible when chrome or overlay content shrinks
        // (e.g., shorter tab title after OSC 0/2 updates during
        // high-throughput PTY output).
        out.clear_ephemeral_tiers();
        out.image_quads_below.clear();
        out.image_quads_above.clear();
        out.viewport = input.viewport;
        out.set_clear_color(input.palette.background, f64::from(input.palette.opacity));
        out.was_incremental = true;
        fill_frame_incremental(input, atlas, shaped, out, origin, cursor_opacity);
    } else {
        // Full rebuild path. The redundant terminal-tier double-clear
        // (after save_terminal_tier) is accepted: a clear_non_terminal()
        // helper would create a second sync point that drifts when new
        // fields are added to PreparedFrame. The double-clear is O(1) on
        // empty buffers, has no observable cost, and keeps clear() /
        // clear_ephemeral_tiers() as the SSOT for buffer-clearing field
        // lists.
        out.was_incremental = false;
        out.clear();
        out.viewport = input.viewport;
        out.set_clear_color(input.palette.background, f64::from(input.palette.opacity));
        fill_frame_shaped(input, atlas, shaped, out, origin, cursor_opacity);
    }

    // Update post-prepare snapshots for next frame's dispatch.
    // The dispatch fingerprint replaces the prior 9 enumerated tail updates;
    // per-row dirty-tracking fields (selection, cursor, hovered) stay because
    // they feed `build_dirty_set` inside the incremental pass AND
    // `WindowRenderer::has_row_state_change` for the fast-path gate.
    out.prev_dispatch_fingerprint = Some(fingerprint);

    let num_rows = input.rows();
    out.prev_selection_snapshot = input
        .selection
        .as_ref()
        .and_then(|s| s.damage_snapshot(num_rows));
    // Use the resolved cursor (mark-mode override applied) so the next
    // frame's build_dirty_set dirties the correct previous-cursor row
    // when the cursor moves.
    let resolved_cursor = resolve_cursor(&input.content.cursor, input.mark_cursor.as_ref());
    out.prev_cursor_line = if resolved_cursor.visible {
        Some(resolved_cursor.line)
    } else {
        None
    };
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

    let cursor = resolve_cursor(&input.content.cursor, input.mark_cursor.as_ref());
    if cursor.visible && cursor_opacity > 0.0 {
        let shape = if input.window_focused {
            cursor.shape
        } else {
            CursorShape::HollowBlock
        };
        let cw = input.cell_size.width;
        let ch = input.cell_size.height;
        let (ox, oy) = origin;
        build_cursor(
            out,
            shape,
            cursor.column.0,
            cursor.line,
            cw,
            ch,
            ox,
            oy,
            input.palette.cursor_color,
            cursor_opacity,
        );
    }

    // URL hover underline + prompt markers also live on the cursors
    // buffer (per `emit::draw_url_hover_underline` /
    // `emit::draw_prompt_markers`); the cursors.clear() above drops them,
    // so re-emit them on every cursor-only refresh.
    let (ox, oy) = origin;
    draw_url_hover_underline(input, out, ox, oy);
    draw_prompt_markers(input, out, ox, oy);
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
        cursor: resolve_cursor(&input.content.cursor, input.mark_cursor.as_ref()),
        cursor_opacity,
        hovered_cell: input.hovered_cell,
        cell_size: &input.cell_size,
        atlas,
        size_q6: shaped.size_q6(),
        frame,
        shaped: Some((shaped, shaped.hinted())),
    };

    for cell in &input.content.cells {
        if cell
            .flags
            .intersects(CellFlags::WIDE_CHAR_SPACER | CellFlags::LEADING_WIDE_CHAR_SPACER)
        {
            continue;
        }

        let col = cell.column.0;
        let row = cell.line;

        // Record row range on row transition.
        if row != current_row {
            if current_row == usize::MAX {
                row_start = BufferLengths::capture(ctx.frame);
            } else {
                let now = BufferLengths::capture(ctx.frame);
                let ranges = now.range_since(&row_start);
                // Fill gaps if rows were skipped (shouldn't happen but defensive).
                while ctx.frame.row_ranges.len() < current_row {
                    ctx.frame.row_ranges.push(RowInstanceRanges::default());
                }
                ctx.frame.row_ranges.push(ranges);
                row_start = now;
            }
            current_row = row;

            // Skip rows entirely outside the render target.
            // Round to match the integer-snapped Y used for rendering.
            let row_y = (oy + row as f32 * ch).round();
            row_off_screen = row_y + ch < 0.0 || row_y > viewport_h;
        }

        if row_off_screen {
            continue;
        }

        let x = ox + col as f32 * cw;
        // Round Y to integer pixels to prevent bilinear interpolation from
        // softening glyph edges on fractional-DPI displays (1.25x, 1.5x).
        // UI text already does this (scene_convert/text.rs:51).
        let y = (oy + row as f32 * ch).round();

        emit_cell::emit_cell(cell, x, y, &mut ctx);
    }

    // Record the final row's range.
    if current_row != usize::MAX {
        let now = BufferLengths::capture(ctx.frame);
        let ranges = now.range_since(&row_start);
        while ctx.frame.row_ranges.len() < current_row {
            ctx.frame.row_ranges.push(RowInstanceRanges::default());
        }
        ctx.frame.row_ranges.push(ranges);
    }

    draw_url_hover_underline(input, ctx.frame, ox, oy);
    draw_prompt_markers(input, ctx.frame, ox, oy);

    // Cursor (gated by terminal visibility AND application blink state).
    // Unfocused windows always render a steady hollow block cursor.
    if ctx.cursor.visible && cursor_opacity > 0.0 {
        let shape = if input.window_focused {
            ctx.cursor.shape
        } else {
            CursorShape::HollowBlock
        };
        build_cursor(
            ctx.frame,
            shape,
            ctx.cursor.column.0,
            ctx.cursor.line,
            cw,
            ch,
            ox,
            oy,
            input.palette.cursor_color,
            cursor_opacity,
        );
    }

    emit::emit_image_quads(input, ctx.frame, ox, oy);
}

#[cfg(test)]
mod tests;
