//! Snapshot building for IPC responses.
//!
//! Converts IO-thread-produced [`RenderableContent`] into wire-friendly types
//! ([`PaneSnapshot`], [`WireCell`], [`WireCursor`]) for transmission to window
//! processes.
//!
//! Colors are pre-resolved by the IO thread via `Term::renderable_content_into()`
//! — the wire cells carry resolved RGB values (bold-as-bright, dim, inverse
//! already applied). This eliminates the need for clients to duplicate color
//! resolution.

use std::collections::HashMap;

use oriterm_core::{CursorShape, RenderableContent, Rgb};

use crate::pane::Pane;
use crate::protocol::WireSearchMatch;
use crate::{PaneId, PaneSnapshot, WireCell, WireCursor, WireCursorShape, WireRgb};

/// Cached snapshots with reusable allocation buffers.
///
/// Encapsulates the per-pane snapshot cache and the shared
/// [`RenderableContent`] scratch buffer used during snapshot building.
/// The server layer interacts with this type instead of touching
/// `RenderableContent` directly.
pub(crate) struct SnapshotCache {
    /// Per-pane cached snapshots — buffers reused across frames.
    cache: HashMap<PaneId, PaneSnapshot>,
    /// Shared scratch buffer for IO-thread snapshot swap.
    render_buf: RenderableContent,
}

impl SnapshotCache {
    /// Create an empty cache.
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            render_buf: RenderableContent::default(),
        }
    }

    /// Build a snapshot for a pane, reusing cached allocations.
    ///
    /// Reads the IO thread's latest snapshot via zero-lock swap.
    pub fn build(&mut self, pane_id: PaneId, pane: &Pane) -> &PaneSnapshot {
        let cached = self.cache.entry(pane_id).or_default();
        if pane.swap_io_snapshot(&mut self.render_buf) {
            fill_snapshot_from_renderable(pane, &self.render_buf, cached);
        }
        &self.cache[&pane_id]
    }

    /// Clone the cached snapshot for a pane (for sending over IPC).
    ///
    /// Builds a fresh snapshot if none is cached.
    pub fn build_clone(&mut self, pane_id: PaneId, pane: &Pane) -> PaneSnapshot {
        self.build(pane_id, pane).clone()
    }

    /// Build a snapshot and move it out of the cache.
    ///
    /// Avoids the `clone()` in [`build_clone`] by taking ownership via
    /// `mem::take`. The cache entry is left empty (default) — the next
    /// `build` call will re-populate it (losing one frame of allocation
    /// reuse, which is acceptable for the synchronous RPC path).
    pub fn build_and_take(&mut self, pane_id: PaneId, pane: &Pane) -> PaneSnapshot {
        self.build(pane_id, pane);
        std::mem::take(self.cache.get_mut(&pane_id).expect("just built"))
    }

    /// Remove a pane's cached snapshot.
    pub fn remove(&mut self, pane_id: PaneId) {
        self.cache.remove(&pane_id);
    }
}

/// Fill snapshot metadata and wire cells from a pre-built [`RenderableContent`].
///
/// Used when the IO thread has already produced a snapshot — no terminal lock
/// needed. Palette and grid dimensions come from `RenderableContent`'s metadata
/// fields (`cols`, `lines`, `scrollback_len`, `palette_snapshot`).
pub(crate) fn fill_snapshot_from_renderable(
    pane: &Pane,
    render_buf: &RenderableContent,
    out: &mut PaneSnapshot,
) {
    fill_wire_cells_from_renderable(render_buf, out);
    fill_metadata_from_renderable(pane, render_buf, out);
}

/// Convert [`RenderableContent`] cells to wire format without `&Term`.
///
/// Hyperlink URIs come from `RenderableCell::hyperlink_uri`, populated
/// during `renderable_content_into()`.
fn fill_wire_cells_from_renderable(render_buf: &RenderableContent, out: &mut PaneSnapshot) {
    let cols = render_buf.cols;
    if cols == 0 {
        out.cells.clear();
        return;
    }

    let mut row_idx = 0;
    let mut col_count = 0;
    for cell in &render_buf.cells {
        let wire = WireCell {
            ch: cell.ch,
            fg: rgb_to_wire(cell.fg),
            bg: rgb_to_wire(cell.bg),
            flags: cell.flags.bits(),
            underline_color: cell.underline_color.map(rgb_to_wire),
            hyperlink_uri: cell.hyperlink_uri.clone(),
            zerowidth: cell.zerowidth.clone(),
        };

        if col_count == 0 {
            if row_idx < out.cells.len() {
                out.cells[row_idx].clear();
            } else {
                out.cells.push(Vec::with_capacity(cols));
            }
        }

        if row_idx < out.cells.len() {
            out.cells[row_idx].push(wire);
        }

        col_count += 1;
        if col_count == cols {
            col_count = 0;
            row_idx += 1;
        }
    }
    if col_count > 0 {
        row_idx += 1;
    }
    out.cells.truncate(row_idx);
}

/// Fill all snapshot fields except `cells` from a pre-built [`RenderableContent`].
///
/// Reads palette, grid dimensions, and scrollback length from the snapshot's
/// metadata fields. Pane-local data (title, CWD, search) comes from `&Pane`.
fn fill_metadata_from_renderable(
    pane: &Pane,
    render_buf: &RenderableContent,
    out: &mut PaneSnapshot,
) {
    // Cursor.
    out.cursor = WireCursor {
        col: u16::try_from(render_buf.cursor.column.0).unwrap_or(u16::MAX),
        row: u16::try_from(render_buf.cursor.line).unwrap_or(u16::MAX),
        shape: cursor_shape_to_wire(render_buf.cursor.shape),
        visible: render_buf.cursor.visible,
    };

    // Palette from snapshot metadata (no Term lock needed).
    out.palette.clear();
    out.palette
        .reserve(270usize.saturating_sub(out.palette.capacity()));
    out.palette.extend_from_slice(&render_buf.palette_snapshot);

    // Title.
    out.title.clear();
    out.title.push_str(pane.effective_title());

    // Icon name.
    out.icon_name = pane.icon_name().map(str::to_owned);

    // CWD.
    out.cwd = pane.cwd().map(str::to_owned);

    // Scalar fields.
    out.has_unseen_output = pane.has_unseen_output();
    out.modes = render_buf.mode.bits();
    out.scrollback_len = u32::try_from(render_buf.scrollback_len).unwrap_or(u32::MAX);
    out.display_offset = u32::try_from(render_buf.display_offset).unwrap_or(u32::MAX);
    out.stable_row_base = render_buf.stable_row_base;
    out.cols = render_buf.cols as u16;

    // Search state from RenderableContent (filled by IO thread).
    fill_search_from_renderable(render_buf, out);
}

/// Fill search state in a [`PaneSnapshot`] from [`RenderableContent`] fields.
///
/// Reads search data that the IO thread populated during snapshot production.
fn fill_search_from_renderable(render_buf: &RenderableContent, out: &mut PaneSnapshot) {
    if render_buf.search_active {
        out.search_active = true;
        out.search_query.clear();
        out.search_query.push_str(&render_buf.search_query);
        out.search_matches.clear();
        for m in &render_buf.search_matches {
            out.search_matches.push(WireSearchMatch {
                start_row: m.start_row.0,
                start_col: u16::try_from(m.start_col).unwrap_or(u16::MAX),
                end_row: m.end_row.0,
                end_col: u16::try_from(m.end_col).unwrap_or(u16::MAX),
            });
        }
        out.search_total_matches = render_buf.search_total_matches;
        out.search_focused = render_buf.search_focused;
    } else {
        out.search_active = false;
        out.search_query.clear();
        out.search_matches.clear();
        out.search_focused = None;
        out.search_total_matches = 0;
    }
}

/// Convert an [`Rgb`] to a [`WireRgb`].
fn rgb_to_wire(rgb: Rgb) -> WireRgb {
    WireRgb {
        r: rgb.r,
        g: rgb.g,
        b: rgb.b,
    }
}

/// Map [`CursorShape`] enum to [`WireCursorShape`].
fn cursor_shape_to_wire(shape: CursorShape) -> WireCursorShape {
    WireCursorShape::from(shape)
}
