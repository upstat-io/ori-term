//! Snapshot building for IPC responses.
//!
//! Converts internal terminal state into wire-friendly types ([`PaneSnapshot`],
//! [`WireCell`], [`WireCursor`]) for transmission to window processes.
//!
//! Colors are pre-resolved server-side via [`Term::renderable_content()`] — the
//! wire cells carry resolved RGB values (bold-as-bright, dim, inverse already
//! applied). This eliminates the need for clients to duplicate color resolution.

use std::collections::HashMap;

use oriterm_core::index::Line;
use oriterm_core::{Column, CursorShape, Grid, RenderableCell, RenderableContent, Rgb, Term};

use crate::mux_event::MuxEventProxy;
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
    /// Shared scratch buffer for `Term::renderable_content_into()`.
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
    /// Reads the IO thread's latest snapshot via zero-lock swap. Falls back
    /// to locking the old Term for VTE content when no IO-thread snapshot
    /// is available yet. Non-VTE state (theme, cursor shape) may be stale
    /// on the fallback path since dual-Term mirror updates were removed.
    pub fn build(&mut self, pane_id: PaneId, pane: &Pane) -> &PaneSnapshot {
        let cached = self.cache.entry(pane_id).or_default();
        if pane.swap_io_snapshot(&mut self.render_buf) {
            fill_snapshot_from_renderable(pane, &self.render_buf, cached);
        } else {
            build_snapshot_locked(pane, cached, &mut self.render_buf);
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

/// Build a full snapshot of a pane's visible state.
///
/// Blocks on the terminal lock. The PTY reader uses a fairness-gate
/// lease to control when the renderer gets access — the reader yields
/// between parse cycles, keeping this wait brief even during flood output.
#[allow(dead_code, reason = "available for non-cached snapshot path")]
pub(crate) fn build_snapshot(pane: &Pane) -> PaneSnapshot {
    let term = pane.terminal().lock();
    build_snapshot_inner(&term, pane)
}

/// Build a snapshot by locking the terminal (fallback path).
///
/// Used when no IO-thread snapshot is available yet. VTE text content is
/// correct (old `PtyEventLoop` still parses). Non-VTE state (theme, cursor
/// shape) may be stale since dual-Term mirror updates were removed.
pub(crate) fn build_snapshot_locked(
    pane: &Pane,
    out: &mut PaneSnapshot,
    render_buf: &mut RenderableContent,
) {
    let mut term = pane.terminal().lock();
    build_snapshot_inner_into(&term, pane, out, render_buf);
    term.reset_damage();
}

/// Shared allocation-reusing snapshot logic.
///
/// Same work as [`build_snapshot_inner`] but mutates `out` in place,
/// reusing existing `Vec` capacities.
fn build_snapshot_inner_into(
    term: &Term<MuxEventProxy>,
    pane: &Pane,
    out: &mut PaneSnapshot,
    render_buf: &mut RenderableContent,
) {
    term.renderable_content_into(render_buf);
    let grid = term.grid();
    let cols = grid.cols();

    // Reuse row Vecs: clear and refill existing rows, push new ones if
    // the grid grew, truncate extras if it shrank.
    let offset = render_buf.display_offset;
    let mut row_idx = 0;
    let mut col_count = 0;
    for cell in &render_buf.cells {
        let hyperlink_uri = if cell.has_hyperlink {
            hyperlink_uri_at(grid, cell.line, cell.column, offset)
        } else {
            None
        };
        let wire = renderable_to_wire(cell, hyperlink_uri);

        if col_count == 0 {
            // Start of a new row — reuse or create.
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
    // Flush partial last row.
    if col_count > 0 {
        row_idx += 1;
    }
    // Truncate extra rows from a previous larger grid.
    out.cells.truncate(row_idx);

    fill_snapshot_metadata(term, pane, render_buf, out);
}

/// Fill all snapshot fields except `cells` from terminal state.
///
/// Shared by [`build_snapshot_inner_into`] (which also fills cells) and
/// [`build_snapshot_metadata_into`] (which skips cell conversion).
fn fill_snapshot_metadata(
    term: &Term<MuxEventProxy>,
    pane: &Pane,
    render_buf: &RenderableContent,
    out: &mut PaneSnapshot,
) {
    let grid = term.grid();

    // Cursor.
    out.cursor = WireCursor {
        col: u16::try_from(render_buf.cursor.column.0).unwrap_or(u16::MAX),
        row: u16::try_from(render_buf.cursor.line).unwrap_or(u16::MAX),
        shape: cursor_shape_to_wire(render_buf.cursor.shape),
        visible: render_buf.cursor.visible,
    };

    // Palette: reuse Vec capacity.
    out.palette.clear();
    let palette = term.palette();
    out.palette
        .reserve(270usize.saturating_sub(out.palette.capacity()));
    for i in 0..270 {
        let rgb = palette.color(i);
        out.palette.push([rgb.r, rgb.g, rgb.b]);
    }

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
    out.scrollback_len = u32::try_from(grid.scrollback().len()).unwrap_or(u32::MAX);
    out.display_offset = u32::try_from(render_buf.display_offset).unwrap_or(u32::MAX);
    out.stable_row_base = render_buf.stable_row_base;
    out.cols = grid.cols() as u16;

    // Search state — read from RenderableContent (populated by IO thread).
    out.search_active = render_buf.search_active;
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
}

/// Shared snapshot logic — converts terminal state to wire format.
///
/// Delegates to [`build_snapshot_inner_into`] with freshly allocated buffers.
/// Callers that need allocation reuse should call `build_snapshot_inner_into`
/// directly (via [`SnapshotCache`]).
fn build_snapshot_inner(term: &Term<MuxEventProxy>, pane: &Pane) -> PaneSnapshot {
    let mut out = PaneSnapshot::default();
    let mut render_buf = RenderableContent::default();
    build_snapshot_inner_into(term, pane, &mut out, &mut render_buf);
    out
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

/// Convert a pre-resolved [`RenderableCell`] to a [`WireCell`].
fn renderable_to_wire(cell: &RenderableCell, hyperlink_uri: Option<String>) -> WireCell {
    WireCell {
        ch: cell.ch,
        fg: rgb_to_wire(cell.fg),
        bg: rgb_to_wire(cell.bg),
        flags: cell.flags.bits(),
        underline_color: cell.underline_color.map(rgb_to_wire),
        hyperlink_uri,
        zerowidth: cell.zerowidth.clone(),
    }
}

/// Look up the hyperlink URI for a viewport cell from the grid.
///
/// Only called when `RenderableCell::has_hyperlink` is true.
fn hyperlink_uri_at(
    grid: &Grid,
    vis_line: usize,
    col: Column,
    display_offset: usize,
) -> Option<String> {
    let row = if vis_line < display_offset {
        let sb_idx = grid.scrollback().len() - display_offset + vis_line;
        grid.scrollback().get(sb_idx)?
    } else {
        let grid_line = vis_line - display_offset;
        &grid[Line(grid_line as i32)]
    };
    row[col].hyperlink().map(|h| h.uri.clone())
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
