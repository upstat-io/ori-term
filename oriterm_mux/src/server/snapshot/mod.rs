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

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use oriterm_core::{CursorShape, ImageId, RenderableContent, RenderableImageData, Rgb};

use crate::image_cache::ImageCache;
use crate::pane::Pane;
use crate::protocol::snapshot::WirePlacement;
use crate::protocol::{WireSearchMatch, encode_cursor_icon};
use crate::{PaneId, PaneSnapshot, WireCell, WireCursor, WireCursorShape, WireRgb};

/// Cached snapshots with reusable allocation buffers.
/// Encapsulates the per-pane snapshot cache and the shared
/// [`RenderableContent`] scratch buffer used during snapshot building.
/// The server layer interacts with this type instead of touching
/// `RenderableContent` directly.
pub(crate) struct SnapshotCache {
 /// Per-pane cached snapshots — buffers reused across frames.
 cache: HashMap<PaneId, PaneSnapshot>,
 /// Shared scratch buffer for IO-thread snapshot swap.
 render_buf: RenderableContent,
 /// Bounded LRU store of image pixel data, keyed by `(PaneId, ImageId)`.
 /// Server-side SSOT for daemon-mode image bytes. The dispatch slow path
 /// reads from this to build per-client `WireImageData` entries on demand
 /// (so cached `PaneSnapshot` entries can stay image-data-free, and per-
 /// subscriber filtering doesn't re-extract from `Term`).
 image_data_store: ImageCache,
}

/// Pixel-data evictions surfaced by the most recent `build*` call.
/// Callers must propagate these to every client's `sent_images` so that a
/// snapshot later referencing the evicted ID re-includes its `WireImageData`.
pub(crate) type EvictedImageKeys = Vec<(PaneId, ImageId)>;

impl SnapshotCache {
 /// Create an empty cache.
 pub fn new() -> Self {
 Self {
 cache: HashMap::new(),
 render_buf: RenderableContent::default(),
 image_data_store: ImageCache::new(),
 }
 }

 /// Build a snapshot for a pane, reusing cached allocations.
 /// Reads the IO thread's latest snapshot via zero-lock swap. Also folds
 /// any new `RenderableImageData` from `render_buf` into the server-side
 /// `image_data_store` (server-side SSOT for daemon-mode image bytes).
 /// Returns the list of `(PaneId, ImageId)` keys evicted from
 /// `image_data_store` during this insert sweep — callers MUST propagate
 /// these to every client's `sent_images` to maintain the server-driven
 /// invalidation contract.
 pub fn build(&mut self, pane_id: PaneId, pane: &Pane) -> (&PaneSnapshot, EvictedImageKeys) {
 let mut evicted = EvictedImageKeys::new();
 let swapped = pane.swap_io_snapshot(&mut self.render_buf);
 if swapped {
 // Fold image data into image_data_store BEFORE filling the cached
 // snapshot (so the latest pane snapshot still has its prior images
 // for the reachability oracle during eviction decisions).
 evicted = fold_image_data_store(
 pane_id,
 &self.cache,
 &self.render_buf.image_data,
 &mut self.image_data_store,
 );
 let cached = self.cache.entry(pane_id).or_default();
 fill_snapshot_from_renderable(pane, &self.render_buf, cached);
 } else {
 // No IO-thread snapshot to fold; ensure cache entry exists so
 // downstream `build_and_take` can mem::take it.
 self.cache.entry(pane_id).or_default();
 }
 (&self.cache[&pane_id], evicted)
 }

 /// Look up `(pane_id, image_id)` in the server-side pixel-data store.
 /// Used by per-client snapshot projection (`push_snapshot_to_subscribers`,
 /// `GetPaneSnapshot` RPC) to fetch the `Arc<RenderableImageData>` to attach
 /// inline as `WireImageData` when a client needs it.
 pub fn image_data(
 &mut self,
 pane_id: PaneId,
 image_id: ImageId,
 ) -> Option<Arc<RenderableImageData>> {
 self.image_data_store.get(pane_id, image_id)
 }

 /// Clone the cached snapshot for a pane (for sending over IPC).
 /// Builds a fresh snapshot if none is cached. Returns the snapshot plus
 /// any evicted `(PaneId, ImageId)` keys from the image-data store.
 pub fn build_clone(
 &mut self,
 pane_id: PaneId,
 pane: &Pane,
 ) -> (PaneSnapshot, EvictedImageKeys) {
 let (snap_ref, evicted) = self.build(pane_id, pane);
 (snap_ref.clone(), evicted)
 }

 /// Build a snapshot and move it out of the cache.
 /// Avoids the `clone()` in [`build_clone`] by taking ownership via
 /// `mem::take`. The cache entry is left empty (default) — the next
 /// `build` call will re-populate it (losing one frame of allocation
 /// reuse, which is acceptable for the synchronous RPC path). Returns
 /// the snapshot plus any evicted `(PaneId, ImageId)` keys.
 pub fn build_and_take(
 &mut self,
 pane_id: PaneId,
 pane: &Pane,
 ) -> (PaneSnapshot, EvictedImageKeys) {
 let (_, evicted) = self.build(pane_id, pane);
 let taken = std::mem::take(self.cache.get_mut(&pane_id).expect("just built"));
 (taken, evicted)
 }

 /// Remove a pane's cached snapshot and image pixel data.
 pub fn remove(&mut self, pane_id: PaneId) {
 self.cache.remove(&pane_id);
 self.image_data_store.drop_pane(pane_id);
 }
}

/// Fold pixel data from a freshly-swapped `render_buf` into the server-side
/// `image_data_store`, using cross-pane reachability for eviction decisions.
/// Computed reachability set walks ALL panes' latest cached snapshots — an
/// image bytes entry for pane B may be evicted by pane A's insert under
/// memory pressure, so we cannot evict bytes still referenced by B.
fn fold_image_data_store(
 pane_id: PaneId,
 cache: &HashMap<PaneId, PaneSnapshot>,
 image_data: &[RenderableImageData],
 store: &mut ImageCache,
) -> EvictedImageKeys {
 if image_data.is_empty() {
 return Vec::new();
 }
 // Compute upfront reachability set: every `(PaneId, ImageId)` referenced
 // by a placement in any pane's latest cached snapshot — PLUS every ID
 // about to be inserted from THIS render_buf for THIS pane.
 // The new IDs must be marked reachable because the cached snapshot's
 // placements (`cache[pane_id].images`) are NOT yet updated to reference
 // them — `fill_snapshot_from_renderable` runs AFTER this fold. Without
 // them in the reachable set, an LRU eviction triggered by cap pressure
 // would pick the just-inserted entry (the only one not appearing in any
 // cached snapshot yet) as the victim, immediately dropping pixel data
 // that the about-to-be-cached snapshot will reference. Daemon clients
 // then receive `images: [N]` with `image_data: []` and render blank.
 let mut reachable: HashSet<(PaneId, ImageId)> = cache
 .iter()
 .flat_map(|(p, snap)| {
 snap.images
 .iter()
 .map(move |wp| (*p, ImageId::from_raw(wp.image_id)))
 })
 .collect();
 for img in image_data {
 reachable.insert((pane_id, img.id));
 }
 let mut total_evicted = Vec::new();
 for img in image_data {
 let arc = Arc::new(img.clone());
 let evicted = store.insert(pane_id, img.id, arc, |p, i| reachable.contains(&(p, i)));
 total_evicted.extend(evicted);
 }
 total_evicted
}

/// Fill snapshot metadata and wire cells from a pre-built [`RenderableContent`].
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
 fill_images_from_renderable(render_buf, out);
}

/// Fill the `images` placement list + `images_dirty` flag from `render_buf`.
/// `image_data` deliberately stays empty in the CACHED `PaneSnapshot` — the
/// dispatch layer projects per-client `image_data` at queue time, reading
/// pixels from `SnapshotCache.image_data_store` (kept in sync separately
/// by `fold_image_data_store`).
fn fill_images_from_renderable(render_buf: &RenderableContent, out: &mut PaneSnapshot) {
 out.images.clear();
 out.images.reserve(
 render_buf
 .images
 .len()
 .saturating_sub(out.images.capacity()),
 );
 for placement in &render_buf.images {
 out.images.push(WirePlacement {
 image_id: placement.image_id.as_u32(),
 viewport_x: placement.viewport_x,
 viewport_y: placement.viewport_y,
 display_width: placement.display_width,
 display_height: placement.display_height,
 source_x: placement.source_x,
 source_y: placement.source_y,
 source_w: placement.source_w,
 source_h: placement.source_h,
 z_index: placement.z_index,
 opacity: placement.opacity,
 });
 }
 out.images_dirty = render_buf.images_dirty;
 // `image_data` stays empty in the cached snapshot — dispatch projects it
 // per-client at queue time.
 out.image_data.clear();
}

/// Convert [`RenderableContent`] cells to wire format without `&Term`.
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
/// Reads palette, grid dimensions, and scrollback length from the snapshot's
/// metadata fields. Pane-local data (title, CWD, search) comes from `&Pane`.
fn fill_metadata_from_renderable(
 pane: &Pane,
 render_buf: &RenderableContent,
 out: &mut PaneSnapshot,
) {
 fill_wire_metadata_from_renderable(render_buf, out);
 fill_pane_metadata(pane, out);
}

/// Fill the `RenderableContent`-derived portion of snapshot metadata.
/// All fields here are scalars or wire-index encodings of IO-thread state
/// (cursor, palette, modes, dimensions, OSC 22 mouse cursor icon, search).
/// Pane-specific fields (title, `icon_name`, cwd, `has_unseen_output`) are
/// filled separately by [`fill_pane_metadata`]. Splitting keeps this
/// half independently testable — the daemon wire contract between
/// `RenderableContent` and `PaneSnapshot` is verified without needing
/// a real [`Pane`].
pub(crate) fn fill_wire_metadata_from_renderable(
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

 // Scalar fields derived from RenderableContent.
 out.modes = render_buf.mode.bits();
 out.scrollback_len = u32::try_from(render_buf.scrollback_len).unwrap_or(u32::MAX);
 out.display_offset = u32::try_from(render_buf.display_offset).unwrap_or(u32::MAX);
 out.stable_row_base = render_buf.stable_row_base;
 out.cols = render_buf.cols as u16;
 out.mouse_cursor_icon = render_buf.mouse_cursor_icon.and_then(encode_cursor_icon);

 // Search state from RenderableContent (filled by IO thread).
 fill_search_from_renderable(render_buf, out);
}

/// Fill the [`Pane`]-derived portion of snapshot metadata.
/// Separated from [`fill_wire_metadata_from_renderable`] so the wire
/// mapping of `RenderableContent` → `PaneSnapshot` can be unit-tested
/// without needing a full `Pane` (PTY + threads).
fn fill_pane_metadata(pane: &Pane, out: &mut PaneSnapshot) {
 out.title.clear();
 out.title.push_str(pane.effective_title());
 out.icon_name = pane.icon_name().map(str::to_owned);
 out.cwd = pane.cwd().map(str::to_owned);
 out.has_unseen_output = pane.has_unseen_output();
}

/// Fill search state in a [`PaneSnapshot`] from [`RenderableContent`] fields.
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

#[cfg(test)]
mod tests;
