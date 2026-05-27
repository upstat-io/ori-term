//! Convert a [`PaneSnapshot`] into a [`FrameInput`] for daemon-mode rendering.
//!
//! In daemon mode the client has no local `Term` — it renders from cached
//! wire snapshots instead. This module bridges the gap: `WireCell` →
//! `RenderableCell`, `WireCursor` → `RenderableCursor`, palette array →
//! `FramePalette`. Image pixel data resolves through the caller-supplied
//! `image_lookup` closure (`MuxClient.image_cache` keyed by `(PaneId, ImageId)`).
//! See:

use std::sync::Arc;

use oriterm_core::{
    CellFlags, Column, ImageId, RenderableCell, RenderableContent, RenderableCursor,
    RenderableImageData, RenderablePlacement, Rgb, TermMode,
};
use oriterm_mux::protocol::snapshot::{WireCell, WirePlacement};
use oriterm_mux::{PaneSnapshot, WireRgb};

use crate::font::CellMetrics;
use crate::gpu::frame_input::{FrameInput, FramePalette, ViewportSize};

/// Palette index for the foreground color (`vte::ansi::NamedColor::Foreground`).
const PALETTE_FOREGROUND: usize = 256;
/// Palette index for the background color (`vte::ansi::NamedColor::Background`).
const PALETTE_BACKGROUND: usize = 257;
/// Palette index for the cursor color (`vte::ansi::NamedColor::Cursor`).
const PALETTE_CURSOR: usize = 258;

/// Build a [`FrameInput`] from a daemon-mode [`PaneSnapshot`].
/// Wire cells already have pre-resolved RGB colors, so this is a
/// straightforward type conversion — no palette lookups needed for
/// per-cell fg/bg.
pub(crate) fn extract_frame_from_snapshot(
    snapshot: &PaneSnapshot,
    viewport: ViewportSize,
    cell_size: CellMetrics,
    image_lookup: &dyn Fn(ImageId) -> Option<Arc<RenderableImageData>>,
) -> FrameInput {
    let content = snapshot_to_renderable(snapshot, image_lookup);
    let palette = snapshot_palette(snapshot);
    let reverse_video = content.mode.contains(TermMode::REVERSE_VIDEO);

    FrameInput {
        content,
        viewport,
        cell_size,
        content_cols: snapshot.cols as usize,
        content_rows: snapshot.cells.len(),
        palette,
        selection: None,
        search: None,
        hovered_cell: None,
        hovered_url_segments: Vec::new(),
        mark_cursor: None,
        window_focused: true,
        reverse_video,
        fg_dim: 1.0,
        text_blink_opacity: 1.0,
        subpixel_positioning: true,
        prompt_marker_rows: Vec::new(),
    }
}

/// Convert a [`PaneSnapshot`] into [`RenderableContent`].
/// Wire RGB values map directly to [`Rgb`]; no palette resolution needed.
/// Image placements come straight from `snapshot.images`; image pixel data
/// comes from the wire snapshot itself (first-observation / dirty path) OR
/// from the caller-supplied `image_lookup` closure (steady-state cache hit).
/// Placements whose `ImageId` is in neither location are dropped with a
/// `log::warn!` (release-mode contract: skip the placement, never crash).
fn snapshot_to_renderable(
    snapshot: &PaneSnapshot,
    image_lookup: &dyn Fn(ImageId) -> Option<Arc<RenderableImageData>>,
) -> RenderableContent {
    let total_cells: usize = snapshot.cells.iter().map(Vec::len).sum();
    let mut cells = Vec::with_capacity(total_cells);

    for (line, row) in snapshot.cells.iter().enumerate() {
        for (col_idx, wire) in row.iter().enumerate() {
            cells.push(wire_cell_to_renderable(line, col_idx, wire));
        }
    }

    let cursor = wire_cursor_to_renderable(snapshot.cursor);

    let mut content = RenderableContent::default();
    content.cells = cells;
    content.cursor = cursor;
    content.display_offset = snapshot.display_offset as usize;
    content.stable_row_base = snapshot.stable_row_base;
    content.mode = TermMode::from_bits_truncate(snapshot.modes);
    // First-frame constructor: every cell is new from the client's
    // perspective, so a full repaint is required. Subsequent refills via
    // `snapshot_to_renderable_into` set `all_dirty` only when the wire
    // snapshot signals `images_dirty=true` (mirrors
    // `oriterm_core/src/term/snapshot/mod.rs` semantics — image-cache
    // changes don't tag per-line grid damage so the full grid must repaint).
    content.all_dirty = true;
    populate_images_from_wire(snapshot, &mut content, image_lookup);
    content.mouse_cursor_icon = snapshot
        .mouse_cursor_icon
        .and_then(oriterm_mux::protocol::snapshot::decode_cursor_icon);
    content
}

/// Populate `content.images` / `content.image_data` / `content.images_dirty`
/// from the wire snapshot, resolving cache hits via `image_lookup`.
/// Drops placements whose `ImageId` is in neither the wire snapshot's inline
/// `image_data` nor the lookup closure — that's a malformed-server signal,
/// `log::warn!` in release builds, `debug_assert!` in test/debug.
fn populate_images_from_wire(
    snapshot: &PaneSnapshot,
    content: &mut RenderableContent,
    image_lookup: &dyn Fn(ImageId) -> Option<Arc<RenderableImageData>>,
) {
    content.images.clear();
    content.image_data.clear();
    if snapshot.images.is_empty() {
        content.images_dirty = snapshot.images_dirty;
        return;
    }
    // First, capture inline image_data so we don't double-fetch.
    let mut have_inline: std::collections::HashSet<ImageId> =
        std::collections::HashSet::with_capacity(snapshot.image_data.len());
    for wid in &snapshot.image_data {
        let id = ImageId::from_raw(wid.id);
        content.image_data.push(RenderableImageData {
            id,
            data: Arc::new(wid.data.clone()),
            width: wid.width,
            height: wid.height,
            pixel_generation: wid.pixel_generation,
        });
        have_inline.insert(id);
    }
    // Now project placements; resolve cache hits for any IDs not inline.
    for wp in &snapshot.images {
        let id = ImageId::from_raw(wp.image_id);
        if have_inline.insert(id) {
            // First observation of this id in this snapshot — must resolve from
            // cache; inline path already produced the data and inserted into
            // have_inline above, so this branch fires only for steady-state.
            let Some(arc) = image_lookup(id) else {
                log::warn!("daemon snapshot: cache-miss for referenced placement {id:?}; skipping");
                debug_assert!(false, "image cache miss for referenced placement {id:?}");
                continue;
            };
            content.image_data.push((*arc).clone());
        }
        content.images.push(wire_placement_to_renderable(wp));
    }
    content.images_dirty = snapshot.images_dirty;
}

/// Convert a [`WirePlacement`] (wire schema) to a [`RenderablePlacement`].
fn wire_placement_to_renderable(wp: &WirePlacement) -> RenderablePlacement {
    RenderablePlacement {
        image_id: ImageId::from_raw(wp.image_id),
        viewport_x: wp.viewport_x,
        viewport_y: wp.viewport_y,
        display_width: wp.display_width,
        display_height: wp.display_height,
        source_x: wp.source_x,
        source_y: wp.source_y,
        source_w: wp.source_w,
        source_h: wp.source_h,
        z_index: wp.z_index,
        opacity: wp.opacity,
    }
}

/// Refill an existing [`RenderableContent`] from a [`PaneSnapshot`], reusing allocations.
/// Clears and repopulates `out.cells` and `out.damage` without freeing their
/// backing storage, avoiding per-frame allocation churn.
fn snapshot_to_renderable_into(
    snapshot: &PaneSnapshot,
    out: &mut RenderableContent,
    image_lookup: &dyn Fn(ImageId) -> Option<Arc<RenderableImageData>>,
) {
    out.cells.clear();
    let total_cells: usize = snapshot.cells.iter().map(Vec::len).sum();
    out.cells.reserve(total_cells);

    for (line, row) in snapshot.cells.iter().enumerate() {
        for (col_idx, wire) in row.iter().enumerate() {
            out.cells.push(wire_cell_to_renderable(line, col_idx, wire));
        }
    }

    out.cursor = wire_cursor_to_renderable(snapshot.cursor);
    out.display_offset = snapshot.display_offset as usize;
    out.stable_row_base = snapshot.stable_row_base;
    out.mode = TermMode::from_bits_truncate(snapshot.modes);
    // Daemon-mode wire snapshots do NOT carry per-line damage — the entire
    // grid is replaced atomically each frame — so a full repaint is required
    // unconditionally. The embedded path (`oriterm_core/src/term/snapshot/mod.rs:168`)
    // can fall back to per-line damage tracking because it has direct grid
    // access; daemon-mode clients receive only the new cell array.
    out.all_dirty = true;
    out.damage.clear();
    // Populate images from wire + cache lookup — never clear here: daemon-mode
    // clients carry image data on the wire, so clearing would drop it.
    populate_images_from_wire(snapshot, out, image_lookup);
    out.mouse_cursor_icon = snapshot
        .mouse_cursor_icon
        .and_then(oriterm_mux::protocol::snapshot::decode_cursor_icon);
}

/// Refill an existing [`FrameInput`] from a [`PaneSnapshot`], reusing allocations.
/// Like [`extract_frame_from_snapshot`] but refills `out` in place, reusing
/// the `Vec` allocations inside `out.content`. Avoids per-frame allocation
/// for the `cells` and `damage` vectors.
pub(crate) fn extract_frame_from_snapshot_into(
    snapshot: &PaneSnapshot,
    out: &mut FrameInput,
    viewport: ViewportSize,
    cell_size: CellMetrics,
    image_lookup: &dyn Fn(ImageId) -> Option<Arc<RenderableImageData>>,
) {
    snapshot_to_renderable_into(snapshot, &mut out.content, image_lookup);
    out.viewport = viewport;
    out.cell_size = cell_size;
    out.content_cols = snapshot.cols as usize;
    out.content_rows = snapshot.cells.len();
    out.palette = snapshot_palette(snapshot);
    out.reverse_video = out.content.mode.contains(TermMode::REVERSE_VIDEO);
    out.selection = None;
    out.search = None;
    out.hovered_cell = None;
    out.hovered_url_segments.clear();
    out.mark_cursor = None;
    out.window_focused = true;
    out.fg_dim = 1.0;
    out.subpixel_positioning = true;
    out.prompt_marker_rows.clear();
}

/// Convert a [`WireCursor`] to a [`RenderableCursor`].
fn wire_cursor_to_renderable(wire: oriterm_mux::WireCursor) -> RenderableCursor {
    RenderableCursor {
        line: wire.row as usize,
        column: Column(wire.col as usize),
        shape: wire.shape.into(),
        visible: wire.visible,
    }
}

/// Extract [`FramePalette`] from the snapshot's 270-entry palette array.
pub(crate) fn snapshot_palette(snapshot: &PaneSnapshot) -> FramePalette {
    let get = |idx: usize| -> Rgb {
        snapshot
            .palette
            .get(idx)
            .map_or(Rgb { r: 0, g: 0, b: 0 }, |&[r, g, b]| Rgb { r, g, b })
    };

    FramePalette {
        foreground: get(PALETTE_FOREGROUND),
        background: get(PALETTE_BACKGROUND),
        cursor_color: get(PALETTE_CURSOR),
        opacity: 1.0,
        selection_fg: None,
        selection_bg: None,
    }
}

/// Convert a [`WireRgb`] to an [`Rgb`].
fn wire_rgb_to_rgb(w: WireRgb) -> Rgb {
    Rgb {
        r: w.r,
        g: w.g,
        b: w.b,
    }
}

/// Build a [`RenderableCell`] from one wire cell at the given grid position.
///
/// Single source for the `WireCell` → `RenderableCell` field mapping: BOTH the
/// fresh ([`snapshot_to_renderable`]) and refill ([`snapshot_to_renderable_into`])
/// client paths call here, so a new wire field is mapped in exactly ONE place.
/// `from_bits_truncate` is intentionally lossy — unknown future flag bits drop
/// to opaque/absent rather than corrupting the cell (forward-compat contract).
fn wire_cell_to_renderable(line: usize, col_idx: usize, wire: &WireCell) -> RenderableCell {
    RenderableCell {
        line,
        column: Column(col_idx),
        ch: wire.ch,
        fg: wire_rgb_to_rgb(wire.fg),
        bg: wire_rgb_to_rgb(wire.bg),
        flags: CellFlags::from_bits_truncate(wire.flags),
        underline_color: wire.underline_color.map(wire_rgb_to_rgb),
        fg_alpha: wire.fg_alpha,
        bg_alpha: wire.bg_alpha,
        underline_alpha: wire.underline_alpha,
        has_hyperlink: wire.hyperlink_uri.is_some(),
        hyperlink_uri: None,
        zerowidth: if wire.zerowidth.is_empty() {
            Vec::new()
        } else {
            wire.zerowidth.clone()
        },
    }
}

#[cfg(test)]
mod tests;
