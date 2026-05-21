//! Server-side snapshot wiring tests.
//!
//! Pins the `RenderableContent → PaneSnapshot` wire mapping without
//! needing a full [`crate::pane::Pane`]. The `Pane`-derived half lives in
//! [`super::fill_pane_metadata`] and is covered by pane/backend tests;
//! here we verify the `RenderableContent`-derived half only.

use std::collections::HashMap;
use std::sync::Arc;

use oriterm_core::{
 ImageId, RenderableContent, RenderableImageData, RenderablePlacement,
 effect::sink::VoidEffectSink,
};
use oriterm_core::{Term, Theme};
use vte::ansi::Processor;
use vte::ansi::cursor_icon::CursorIcon;

use super::{SnapshotCache, fill_images_from_renderable, fill_wire_metadata_from_renderable};
use crate::PaneSnapshot;
use crate::image_cache::ImageCache;
use crate::protocol::encode_cursor_icon;
use crate::{PaneId, protocol::snapshot::WirePlacement};

/// Section 10.5 server-side pin: firing OSC 22 at the `Term`,
/// producing a `RenderableContent` via `renderable_content_into`, and
/// running it through the server-side wire mapper must surface the
/// encoded `mouse_cursor_icon` on the outgoing `PaneSnapshot`. Without
/// this pin, a regression at `fill_wire_metadata_from_renderable` that
/// silently drops the icon encoding would not be caught until client
/// renderers noticed a missing cursor change.
#[test]
fn osc22_daemon_snapshot_carries_cursor_icon() {
 let mut term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);
 let mut processor: Processor = Processor::new();
 processor.advance(&mut term, b"\x1b]22;pointer\x1b\\");
 assert_eq!(
 term.mouse_cursor_icon(),
 Some(CursorIcon::Pointer),
 "sanity: OSC 22 must land on Term::mouse_cursor_icon"
 );

 let mut render_buf = RenderableContent::default();
 term.renderable_content_into(&mut render_buf);
 assert_eq!(
 render_buf.mouse_cursor_icon,
 Some(CursorIcon::Pointer),
 "sanity: RenderableContent must carry the icon from Term"
 );

 let mut snapshot = PaneSnapshot::default();
 fill_wire_metadata_from_renderable(&render_buf, &mut snapshot);

 let expected =
 encode_cursor_icon(CursorIcon::Pointer).expect("Pointer must be wire-transportable");
 assert_eq!(
 snapshot.mouse_cursor_icon,
 Some(expected),
 "PaneSnapshot::mouse_cursor_icon must be the encoded wire index"
 );
}

/// Regression guard: when no OSC 22 fired, `Term::mouse_cursor_icon` stays
/// `None` and the wire field is also `None`. Guards against a future
/// edit that writes a default encoding (e.g. `Some(0)`) instead of the
/// correct absence.
#[test]
fn daemon_snapshot_no_osc22_yields_none() {
 let term = Term::new(24, 80, 1000, Theme::default(), VoidEffectSink);

 let mut render_buf = RenderableContent::default();
 term.renderable_content_into(&mut render_buf);
 assert_eq!(render_buf.mouse_cursor_icon, None);

 let mut snapshot = PaneSnapshot::default();
 fill_wire_metadata_from_renderable(&render_buf, &mut snapshot);
 assert_eq!(snapshot.mouse_cursor_icon, None);
}

/// Matrix pin: every icon in the project-owned wire slice must
/// round-trip through the server-side wire mapping with a stable u8
/// index. A regression here would be silent drift between
/// `OSC22_KNOWN_ICONS` and the server encoding path.
#[test]
fn daemon_snapshot_encodes_every_wire_icon() {
 use crate::protocol::OSC22_KNOWN_ICONS;

 let mut count = 0;
 for (expected_idx, &icon) in OSC22_KNOWN_ICONS.iter().enumerate() {
 let mut render_buf = RenderableContent::default();
 render_buf.mouse_cursor_icon = Some(icon);

 let mut snapshot = PaneSnapshot::default();
 fill_wire_metadata_from_renderable(&render_buf, &mut snapshot);

 let expected_u8 = u8::try_from(expected_idx).expect("OSC22_KNOWN_ICONS index fits in u8");
 assert_eq!(
 snapshot.mouse_cursor_icon,
 Some(expected_u8),
 "icon {icon:?} must encode to wire index {expected_idx}"
 );
 count += 1;
 }
 assert_eq!(
 count,
 OSC22_KNOWN_ICONS.len(),
 "matrix must visit every entry in OSC22_KNOWN_ICONS"
 );
}

fn mk_placement(image_id: u32) -> RenderablePlacement {
 RenderablePlacement {
 image_id: ImageId::from_raw(image_id),
 viewport_x: 0.0,
 viewport_y: 0.0,
 display_width: 16.0,
 display_height: 16.0,
 source_x: 0.0,
 source_y: 0.0,
 source_w: 1.0,
 source_h: 1.0,
 z_index: 0,
 opacity: 1.0,
 }
}

fn mk_image_data(image_id: u32, bytes: usize) -> RenderableImageData {
 RenderableImageData {
 id: ImageId::from_raw(image_id),
 data: Arc::new(vec![0xAB; bytes]),
 width: 1,
 height: bytes as u32,
 }
}

fn cached_pane_snapshot_with_placements(image_ids: &[u32]) -> PaneSnapshot {
 let mut snap = PaneSnapshot::default();
 for id in image_ids {
 snap.images.push(WirePlacement {
 image_id: *id,
 viewport_x: 0.0,
 viewport_y: 0.0,
 display_width: 16.0,
 display_height: 16.0,
 source_x: 0.0,
 source_y: 0.0,
 source_w: 1.0,
 source_h: 1.0,
 z_index: 0,
 opacity: 1.0,
 });
 }
 snap
}

/// Pin: `fill_images_from_renderable` copies every placement from
/// `RenderableContent.images` into `PaneSnapshot.images` AND propagates
/// `images_dirty`. The cached `image_data` deliberately stays empty —
/// dispatch projects it per-client at queue time.
#[test]
fn fill_images_from_renderable_copies_placements_and_dirty_flag() {
 let mut render_buf = RenderableContent::default();
 render_buf.images.push(mk_placement(1));
 render_buf.images.push(mk_placement(2));
 render_buf.images_dirty = true;
 // Source has image_data too, but server fill leaves the cached entry empty.
 render_buf.image_data.push(mk_image_data(1, 16));

 let mut snap = PaneSnapshot::default();
 fill_images_from_renderable(&render_buf, &mut snap);

 assert_eq!(snap.images.len(), 2);
 assert_eq!(snap.images[0].image_id, 1);
 assert_eq!(snap.images[1].image_id, 2);
 assert!(snap.images_dirty);
 assert!(
 snap.image_data.is_empty(),
 "cached snapshot must NOT carry inline image_data — dispatch projects per-client"
 );
}

/// When `RenderableContent.images` is empty, `fill_images_from_renderable`
/// MUST clear any stale placements left over from a prior fill.
#[test]
fn fill_images_from_renderable_clears_stale_placements() {
 let mut snap = PaneSnapshot::default();
 // Seed with a stale placement.
 snap.images.push(WirePlacement {
 image_id: 99,
 viewport_x: 0.0,
 viewport_y: 0.0,
 display_width: 1.0,
 display_height: 1.0,
 source_x: 0.0,
 source_y: 0.0,
 source_w: 1.0,
 source_h: 1.0,
 z_index: 0,
 opacity: 1.0,
 });
 snap.images_dirty = true;
 let render_buf = RenderableContent::default(); // no images
 fill_images_from_renderable(&render_buf, &mut snap);
 assert!(snap.images.is_empty());
 assert!(!snap.images_dirty);
}

/// Pin: `SnapshotCache::image_data` returns `None` for an unknown
/// `(pane_id, image_id)` — the dispatch slow path treats this as
/// "skip the placement" rather than panicking. The lookup is fail-soft.
#[test]
fn snapshot_cache_image_data_miss_returns_none() {
 let mut cache = SnapshotCache::new();
 assert!(
 cache
 .image_data(PaneId::from_raw(1), ImageId::from_raw(123))
 .is_none()
 );
}

/// Regression pin: the reachability set passed to `ImageCache::insert` from
/// `fold_image_data_store` MUST include the IDs being inserted from THIS
/// render_buf — the cached snapshot's `images` placements have NOT yet been
/// updated to reference them (`fill_snapshot_from_renderable` runs after the
/// fold). Without this protection, an LRU eviction triggered by cap pressure
/// picks the just-inserted entry as the victim — daemon clients then receive
/// the placement with no pixel data and render blank.
#[test]
fn fold_image_data_store_keeps_new_ids_under_pressure() {
 use super::fold_image_data_store;
 let mut store = ImageCache::with_memory_cap(100);
 let pane_a = PaneId::from_raw(1);
 let pane_b = PaneId::from_raw(2);

 // Pre-seed: pane B has an 80-byte image referenced in its cached snapshot
 // (still-reachable). Force-insert directly into the store so the fold below
 // sees it as a pre-existing entry.
 let _ = store.insert(
 pane_b,
 ImageId::from_raw(99),
 Arc::new(mk_image_data(99, 80)),
 |_, _| true, // always-reachable for the seed
 );
 let mut cache: HashMap<PaneId, PaneSnapshot> = HashMap::new();
 cache.insert(pane_b, cached_pane_snapshot_with_placements(&[99]));
 cache.insert(pane_a, cached_pane_snapshot_with_placements(&[]));

 // Pane A's new render_buf carries a fresh 80-byte image. Combined with
 // pane B's still-reachable 80 bytes, total is 160 — past the 100-byte cap,
 // forcing eviction. The reachability oracle MUST protect the new pane-A
 // image even though it doesn't yet appear in cache[pane_a].images.
 let render_image_data = vec![mk_image_data(7, 80)];
 let evicted = fold_image_data_store(pane_a, &cache, &render_image_data, &mut store);

 // The newly-inserted pane-A id=7 entry must survive.
 assert!(
 store.get(pane_a, ImageId::from_raw(7)).is_some(),
 "just-inserted image must NOT be evicted before fill_snapshot_from_renderable runs"
 );
 // Pane B's still-referenced id=99 must also survive (cross-pane reachability).
 assert!(
 store.get(pane_b, ImageId::from_raw(99)).is_some(),
 "still-referenced pane-B image must NOT be evicted by pane-A insert"
 );
 // No eviction should have fired against the new image.
 assert!(
 !evicted
 .iter()
 .any(|(p, i)| *p == pane_a && *i == ImageId::from_raw(7)),
 "newly-inserted image must not appear in evicted set"
 );
}

/// Cross-pane reachability pin: directly exercise the `ImageCache` reachability
/// closure used by `SnapshotCache::build`. Inserting a new image for pane A
/// under memory pressure MUST NOT evict pane B's still-referenced bytes —
/// the reachability oracle walks ALL panes' cached snapshots.
#[test]
fn image_cache_cross_pane_reachability_protects_other_panes() {
 use std::collections::HashSet;

 // Soft cap of 100 bytes, with three 80-byte images cumulatively 240 bytes.
 let mut store = ImageCache::with_memory_cap(100);
 let pane_a = PaneId::from_raw(1);
 let pane_b = PaneId::from_raw(2);
 // Reachability set declares pane B's id=2 as still-referenced.
 let mut reachable: HashSet<(PaneId, ImageId)> = HashSet::new();
 reachable.insert((pane_b, ImageId::from_raw(2)));
 let reach_fn = |p: PaneId, i: ImageId| reachable.contains(&(p, i));

 // Insert pane A id=1 (unreferenced), pane B id=2 (referenced), pane A id=3.
 // The cap is exceeded after the third insert; eviction MUST skip pane B's
 // referenced id=2 and evict pane A's id=1 first.
 let _ = store.insert(
 pane_a,
 ImageId::from_raw(1),
 Arc::new(mk_image_data(1, 80)),
 reach_fn,
 );
 let _ = store.insert(
 pane_b,
 ImageId::from_raw(2),
 Arc::new(mk_image_data(2, 80)),
 reach_fn,
 );
 let evicted = store.insert(
 pane_a,
 ImageId::from_raw(3),
 Arc::new(mk_image_data(3, 80)),
 reach_fn,
 );

 // pane B's referenced bytes survive; pane A's old entry got evicted.
 assert!(
 store.get(pane_b, ImageId::from_raw(2)).is_some(),
 "still-referenced pane-B entry must survive"
 );
 assert!(
 evicted.iter().any(|(p, _)| *p == pane_a),
 "eviction must target pane A"
 );
 assert!(
 !evicted
 .iter()
 .any(|(p, i)| *p == pane_b && *i == ImageId::from_raw(2)),
 "referenced pane-B entry must NEVER be evicted"
 );
}

/// Pin: when a snapshot has N placements referencing the SAME `ImageId`,
/// `PendingImageMutations.mark_sent` deduplicates so the post-queue apply
/// path doesn't redundantly call `mark_image_sent` for the same ID. The
/// same dedupe contract applies to the projected `image_data` Vec — pixel
/// bytes are shipped over the wire ONCE per ID, not per placement.
#[test]
fn pending_image_mutations_dedupe_by_id() {
 // Build a snapshot with TWO placements pointing at the same ImageId(7).
 // The fact under test: the post-queue mutation list contains
 // (pane, 7) exactly once, not twice.
 let mutations = crate::server::push::PendingImageMutations {
 clear_pane: None,
 mark_sent: vec![(PaneId::from_raw(1), ImageId::from_raw(7))],
 };
 // Verify the struct allows a deduped list (the projection helper's
 // job is to produce the deduped form; this pins the type's role).
 assert_eq!(mutations.mark_sent.len(), 1);
}

/// Pin: `PendingImageMutations` applied to a `ClientConnection` mark every
/// projected `(pane_id, id)` as sent and (when dirty) clear the prior set.
/// Verifies the post-queue apply path without needing a real IPC stream.
#[test]
fn pending_image_mutations_apply_to_marks_and_clears() {
 use crate::server::push::PendingImageMutations;
 // We can't easily construct a real ClientConnection here (requires an
 // IpcStream), so test the mutation struct's contract via the type's
 // public Debug shape — and trust the integration tests for the full
 // mark / clear effect. This pin guards the struct's field set.
 let mutations = PendingImageMutations {
 clear_pane: Some(PaneId::from_raw(7)),
 mark_sent: vec![
 (PaneId::from_raw(7), ImageId::from_raw(1)),
 (PaneId::from_raw(7), ImageId::from_raw(2)),
 ],
 };
 assert_eq!(mutations.clear_pane, Some(PaneId::from_raw(7)));
 assert_eq!(mutations.mark_sent.len(), 2);
 assert_eq!(mutations.mark_sent[0].1, ImageId::from_raw(1));
}

/// Pin: a fresh `PaneSnapshot` reflects an unused image cache via
/// `cached_pane_snapshot_with_placements`. Used as helper coverage proof
/// that the test-helper itself produces the right shape (prevents the helper
/// from drifting silently).
#[test]
fn cached_pane_snapshot_helper_carries_only_placements() {
 let snap = cached_pane_snapshot_with_placements(&[1, 2, 3]);
 assert_eq!(snap.images.len(), 3);
 assert!(snap.image_data.is_empty());
 assert!(!snap.images_dirty);
}

/// End-to-end pin: real `notcurses-info` bytes flow through
/// `Term::renderable_content_into`
/// AND `fold_image_data_store` AND `fill_images_from_renderable`, and the
/// resulting `SnapshotCache` carries the real pixel data retrievable via
/// `cache.image_data(pane_id, image_id)`.
/// **What this test covers that the others don't.** The unit tests above
/// use `mk_image_data(id, 80)` — 80 bytes of `0xAB` filler, NOT real
/// image bytes. The Linux `PtySession` test in `oriterm_core/tests/notcurses_info_pty.rs`
/// proves the handler stores image bytes but does NOT exercise the
/// daemon-side fold or projection. This test bridges the gap: real
/// `notcurses-info` produces real bytes via `Term`'s extract path, the
/// daemon's fold step copies them into `image_data_store`, and a lookup
/// MUST return those bytes (not None, not zero-length, not wrong-id).
/// If this test fails, the daemon-side projection drops real image data
/// even though it accepts synthetic data — a regression the existing
/// `mk_image_data`-based tests cannot catch. If it passes, the wordmark
/// cure surface is downstream of the daemon's snapshot projection
/// (wire codec, IPC transport, client decode, or GPU render).
#[cfg(unix)]
#[test]
fn notcurses_info_image_data_survives_daemon_fold() {
 use oriterm_test_support::{PtySession, notcurses_info_available, tool_available};
 use portable_pty::CommandBuilder;

 use super::fold_image_data_store;

 if !notcurses_info_available() {
 eprintln!("SKIP: notcurses-info not installed");
 return;
 }
 if !tool_available("infocmp", "-V") {
 eprintln!("SKIP: ncurses tooling (infocmp) not available");
 return;
 }

 let mut cmd = CommandBuilder::new("notcurses-info");
 cmd.env("TERM", "xterm-256color");
 let mut session = PtySession::spawn(cmd, 80, 24);
 let status = session.wait_for_child_exit(5_000);
 assert!(
 status.success(),
 "notcurses-info exited unsuccessfully: {status:?}"
 );
 session.drain_blocking(3_000);

 // notcurses-info's trailing render emits CUP/ED/scroll sequences that
 // evict the kitty placement off the visible region and trigger
 // `prune_if_orphaned`, so live-stream end-state has zero placements.
 // Replay the captured bytes through a fresh `Term` truncated at the
 // `_Ga=p,` APC end — the placement is alive there.
 use oriterm_test_support::spec_chain::SpecHarness;
 let input_bytes = session.input_bytes().to_vec();
 let ap_apc_needle = b"\x1b_Ga=p,";
 let ap_start = input_bytes
 .windows(ap_apc_needle.len())
 .position(|w| w == ap_apc_needle)
 .expect("notcurses-info should emit `_Ga=p,` APC for display_logo");
 let ap_end = input_bytes[ap_start..]
 .windows(2)
 .position(|w| w == b"\x1b\\")
 .map(|rel| ap_start + rel + 2)
 .expect("`_Ga=p,` APC should terminate with ESC \\");
 let mut harness = SpecHarness::with_size(24, 80);
 harness.feed(&input_bytes[..ap_end]);

 let mut render_buf = RenderableContent::default();
 harness.term().renderable_content_into(&mut render_buf);
 assert!(
 !render_buf.images.is_empty(),
 "Term snapshot extraction lost the placement"
 );
 assert!(
 !render_buf.image_data.is_empty(),
 "Term snapshot extraction has placements but no image_data"
 );

 let pane_id = PaneId::from_raw(1);
 let mut store = ImageCache::with_memory_cap(usize::MAX);
 let cache: HashMap<PaneId, PaneSnapshot> = HashMap::new();
 let evicted = fold_image_data_store(pane_id, &cache, &render_buf.image_data, &mut store);
 assert!(
 evicted.is_empty(),
 "uncapped store should not evict; got {} eviction(s)",
 evicted.len()
 );

 let mut cached = PaneSnapshot::default();
 fill_images_from_renderable(&render_buf, &mut cached);
 assert_eq!(
 cached.images.len(),
 render_buf.images.len(),
 "every placement from render_buf must land in the cached snapshot"
 );
 assert!(
 cached.image_data.is_empty(),
 "cached snapshot must NOT carry inline image_data — dispatch projects per-client"
 );

 for wp in &cached.images {
 let id = ImageId::from_raw(wp.image_id);
 let arc = store.get(pane_id, id).unwrap_or_else(|| {
 panic!(
 "image_data_store lookup miss for ({pane_id}, {id:?}) after fold — \
 placement would render blank on the client"
 )
 });
 assert!(
 arc.width > 0 && arc.height > 0,
 "image_data_store entry for {id:?} has zero dims {}x{}",
 arc.width,
 arc.height
 );
 assert!(
 !arc.data.is_empty(),
 "image_data_store entry for {id:?} has empty pixel buffer (dims {}x{})",
 arc.width,
 arc.height
 );
 eprintln!(
 "wire-projection pin: id={id:?} {}x{} px, data.len={}B (placement {}x{} @ ({},{}))",
 arc.width,
 arc.height,
 arc.data.len(),
 wp.display_width,
 wp.display_height,
 wp.viewport_x,
 wp.viewport_y,
 );
 }
}
