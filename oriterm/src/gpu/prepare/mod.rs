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
mod gates;
mod resolve;
mod shaped_frame;
#[cfg(test)]
mod unshaped;

pub(crate) use gates::{compute_dispatch_fingerprint, evaluate_row_state_change};

use oriterm_core::{CellFlags, CursorShape, RenderableCursor};

use super::atlas::AtlasEntry;

use super::frame_input::FrameInput;
use super::prepared_frame::PreparedFrame;

use crate::font::{GlyphStyle, RasterKey};
use dirty_skip::{BufferLengths, fill_frame_incremental};
use emit::{draw_prompt_markers, draw_url_hover_underline, emit_cursor_for_frame};
use emit_cell::EmitCtx;
use resolve::{resolve_cursor, CellColorContext};

pub use shaped_frame::ShapedFrame;
#[cfg(test)]
pub(crate) use unshaped::{prepare_frame, prepare_frame_into};

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
    // INVARIANT: save_terminal_tier MUST run first — without it the incremental
    // path is unreachable and saved_tier never populates.
    out.save_terminal_tier();
    debug_assert!(
        out.backgrounds.is_empty()
            && out.glyphs.is_empty()
            && out.subpixel_glyphs.is_empty()
            && out.color_glyphs.is_empty(),
        "save_terminal_tier must leave live terminal-tier writers empty"
    );

    // INVARIANT: row-state fields excluded from fingerprint — handled per-row
    // by build_dirty_set. Full hashed-input list on `compute_dispatch_fingerprint`.
    let fingerprint = compute_dispatch_fingerprint(input, origin);
    let can_incremental = out.can_incremental(input.content.all_dirty, fingerprint);

    if can_incremental {
        // clear_ephemeral_tiers MUST run on incremental path — otherwise
        // chrome/overlay tiers accumulate and leave stale glyphs.
        out.clear_ephemeral_tiers();
        out.image_quads_below.clear();
        out.image_quads_above.clear();
        out.viewport = input.viewport;
        out.set_clear_color(input.palette.background, f64::from(input.palette.opacity));
        out.was_incremental = true;
        fill_frame_incremental(input, atlas, shaped, out, origin, cursor_opacity);
    } else {
        // Terminal-tier double-clear (save_terminal_tier + clear()) is
        // intentional — a clear_non_terminal() helper would create a second
        // sync point that drifts as PreparedFrame fields land.
        out.was_incremental = false;
        out.clear();
        out.viewport = input.viewport;
        out.set_clear_color(input.palette.background, f64::from(input.palette.opacity));
        fill_frame_shaped(input, atlas, shaped, out, origin, cursor_opacity);
    }

    finalize_frame_prepare(out, input, fingerprint, cursor_opacity);
}

/// SSOT for snapshotting "most recent rendered frame's cursor state" onto
/// `PreparedFrame`. Called from BOTH `prepare_frame_shaped_into` (full prepare)
/// AND `update_cursor_only` (cursor-blink fast path) — without this canonical
/// home, the resolve-and-write skeleton would duplicate at the two sites and
/// drift silently when a future `prev_*` cursor field lands.
///
/// Writes:
/// - `prev_resolved_cursor` — visibility-canonicalized resolved cursor;
///   invisible cursors store as `None` so hidden-to-hidden position changes
///   are a no-op for the fast-path predicate (None == None).
/// - `prev_block_cursor_color_exclusion_active` — opacity-threshold predicate
///   value for the cursor-only fast-path gate (`evaluate_row_state_change`
///   in `gates.rs`).
///
/// Both writes share the same `cur_resolved` value so they can never observe
/// different cursor states. `build_dirty_set` derives the line component from
/// `Some(c).map(|c| c.line)`.
fn write_cursor_state_snapshots(out: &mut PreparedFrame, input: &FrameInput, cursor_opacity: f32) {
    let cur_resolved = resolve_cursor_state(input);
    out.prev_resolved_cursor = cur_resolved.into_visible();
    out.prev_block_cursor_color_exclusion_active = Some(
        resolve::block_cursor_color_exclusion_active(&cur_resolved, cursor_opacity),
    );
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

    write_cursor_state_snapshots(out, input, cursor_opacity);
}

/// Write post-prepare snapshots for the next frame's dispatch + row-state gates.
///
/// Extracted from [`prepare_frame_shaped_into`] to keep that function under the
/// 50-line cap. Writes `prev_dispatch_fingerprint`, `prev_selection_snapshot`,
/// cursor state snapshots, and `prev_hovered_cell`.
fn finalize_frame_prepare(
    out: &mut PreparedFrame,
    input: &FrameInput,
    fingerprint: u64,
    cursor_opacity: f32,
) {
    out.prev_dispatch_fingerprint = Some(fingerprint);
    let num_rows = input.rows();
    out.prev_selection_snapshot = input
        .selection
        .as_ref()
        .and_then(|s| s.damage_snapshot(num_rows));
    write_cursor_state_snapshots(out, input, cursor_opacity);
    out.prev_hovered_cell = input.hovered_cell;
}

/// Iterate cells with row-transition tracking, off-screen culling, and
/// per-row range recording. Shared shape between `fill_frame_shaped` and
/// (in spirit) the incremental path's `process_incremental_cells` —
/// extracted from `fill_frame_shaped` to keep that function under the
/// 50-line size cap.
fn emit_row_tracked_cells(
    ctx: &mut EmitCtx<'_>,
    cells: &[oriterm_core::RenderableCell],
    origin: (f32, f32),
) {
    let cw = ctx.cell_size.width;
    let ch = ctx.cell_size.height;
    let viewport_h = ctx.frame.viewport.height as f32;
    let (ox, oy) = origin;

    let mut current_row = usize::MAX;
    let mut row_start = BufferLengths::capture(ctx.frame);
    let mut row_off_screen = false;

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
        if row != current_row {
            if current_row == usize::MAX {
                // First row: just initialize row_start, no range to record yet.
            } else {
                dirty_skip::push_row_range(ctx.frame, current_row, &row_start);
            }
            row_start = BufferLengths::capture(ctx.frame);
            current_row = row;

            // Skip rows entirely outside the render target.
            let row_y = snapped_row_y(oy, row, ch);
            row_off_screen = row_y + ch < 0.0 || row_y > viewport_h;
        }

        if row_off_screen {
            continue;
        }

        let x = ox + col as f32 * cw;
        let y = snapped_row_y(oy, row, ch);
        emit_cell::emit_cell(cell, x, y, ctx);
    }

    // Record the final row's range.
    if current_row != usize::MAX {
        dirty_skip::push_row_range(ctx.frame, current_row, &row_start);
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
    let (ox, oy) = origin;

    let mut ctx = EmitCtx {
        fg_dim: input.fg_dim,
        text_blink_opacity: input.text_blink_opacity,
        subpixel_positioning: input.subpixel_positioning,
        color_ctx: CellColorContext {
            palette: &input.palette,
            sel: input.selection.as_ref(),
            search: input.search.as_ref(),
            cursor: resolve_cursor_state(input),
            cursor_opacity,
        },
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
        (ox, oy),
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
