//! Reproduce the duplicate-wordmark bug observed in operator runs of
//! `notcurses-info` inside ori_term, and bisect which pipeline stage is
//! emitting the duplicate.
//!
//! Operator anchor: `notcurses-info` exits leaving TWO copies of the
//! `notCURSEs` pixel-blit wordmark visible — one at the intended
//! position (upper-right per `info/main.c:366-367`: `.y = y - 3, .x = 55`)
//! and one duplicate further left/down. The reference rendering (a
//! correct terminal running notcurses-info) shows exactly ONE wordmark.
//!
//! Bisect strategy (per-stage assertions; first-failing stage names the
//! root):
//!
//! | Stage | Check | Failure means |
//! |---|---|---|
//! | A. cache  | `term.image_cache().placement_count()` after `\.cap` replay | Term::kitty_transmit_and_place creates >1 placement per single APC `a=T` |
//! | B. snapshot | `renderable_content.images.len()` after `renderable_content_into()` | Snapshot extraction duplicates one cache placement into multiple `RenderablePlacement` entries |
//! | C. GPU emit | `emit_image_quads` per-placement call count | GPU emit phase emits multiple quads for one snapshot placement (out of scope here — exercised by the GPU pilot at `oriterm/src/gpu/visual_regression/spec_chain/pilots/notcurses_info_visual.rs`) |
//!
//! These tests pin the EXPECTED behavior (exactly 1 placement at each
//! stage). When the bug is present they FAIL with a precise message
//! naming the doubling stage. When the bug is fixed they pass.

use oriterm_test_support::spec_chain::SpecHarness;

/// Load the captured `notcurses-info` full byte stream — the same bytes
/// operator-launched `notcurses-info` emits during init + render +
/// shutdown. Wrapper absent → graceful SKIP.
fn captured_bytes() -> Option<Vec<u8>> {
    let captures = oriterm_test_support::paths::captures_dir()?;
    let path = captures.join("notcurses-info-full.cap");
    std::fs::read(&path).ok()
}

/// Stage A — kitty image cache placement count.
///
/// `notcurses-info` emits exactly one kitty `_G a=T,...` chunked
/// transmit for `display_logo()` (a=T == transmit-and-place; spec
/// requires creating exactly ONE placement). If our cache shows >1
/// placement at the end of the byte stream, `Term::kitty_transmit_and_place`
/// is emitting a duplicate.
#[test]
fn notcurses_info_creates_exactly_one_kitty_placement_in_cache() {
    let Some(bytes) = captured_bytes() else {
        eprintln!("SKIP: plans/spec-conformance/captures/notcurses-info-full.cap missing");
        return;
    };
    let mut h = SpecHarness::with_size(54, 142);
    h.feed(&bytes);
    let count = h.term().image_cache().placement_count();
    assert_eq!(
        count, 1,
        "Expected exactly 1 kitty placement in image cache after notcurses-info \
         transmit (display_logo emits ONE `_G a=T,...` per info/main.c:524-530). \
         Observed {count}. \
         If >1, Term::kitty_transmit_and_place is creating duplicate cache entries \
         for a single APC — root cause is at oriterm_core/src/term/handler/image/kitty/transmit.rs \
         OR oriterm_core/src/term/handler/image/kitty/place.rs."
    );
}

/// Stage B — snapshot RenderablePlacement count.
///
/// One cache placement MUST snapshot to exactly one
/// `RenderablePlacement` entry in `RenderableContent.images`. If the
/// snapshot has >1 entries with the same image_id, the snapshot
/// extraction at `oriterm_core/src/term/snapshot/mod.rs::fill_image_snapshot`
/// is duplicating.
#[test]
fn notcurses_info_snapshot_emits_exactly_one_renderable_placement() {
    let Some(bytes) = captured_bytes() else {
        eprintln!("SKIP: plans/spec-conformance/captures/notcurses-info-full.cap missing");
        return;
    };
    let mut h = SpecHarness::with_size(54, 142);
    h.feed(&bytes);

    let snapshot = h.term().renderable_content();
    let total_placements = snapshot.images.len();
    assert_eq!(
        total_placements, 1,
        "Expected exactly 1 RenderablePlacement in snapshot after notcurses-info \
         transmit. Observed {total_placements}. \
         If >1 AND Stage-A test passed: snapshot extraction at \
         oriterm_core/src/term/snapshot/mod.rs::fill_image_snapshot is emitting \
         multiple snapshot entries for a single cache placement. \
         Snapshot dump (image_id, placement_id per entry):\n{:#?}",
        snapshot.images,
    );
}

/// Stage A+B sanity composition — the same cache placement_id appears
/// in the snapshot exactly once. Belt-and-suspenders pin for the case
/// where cache_count == 1 and snapshot_count == 1 but they reference
/// different IDs (would indicate a placement-replacement bug rather
/// than a duplication bug; unexpected shape but worth pinning).
#[test]
fn notcurses_info_snapshot_placement_id_matches_cache_placement_id() {
    let Some(bytes) = captured_bytes() else {
        eprintln!("SKIP: plans/spec-conformance/captures/notcurses-info-full.cap missing");
        return;
    };
    let mut h = SpecHarness::with_size(54, 142);
    h.feed(&bytes);

    let snapshot = h.term().renderable_content();
    assert_eq!(
        snapshot.images.len(),
        1,
        "precondition failed: snapshot must have exactly one image placement \
         (see notcurses_info_snapshot_emits_exactly_one_renderable_placement)"
    );

    let cache = h.term().image_cache();
    assert_eq!(
        cache.placement_count(),
        1,
        "precondition failed: cache must have exactly one placement \
         (see notcurses_info_creates_exactly_one_kitty_placement_in_cache)"
    );
}

// ---------------------------------------------------------------------------
// Matrix tests — clamp the cure (action-inheritance for chunked uploads).
// Each test exercises a different chunked-upload action so the cure
// doesn't accidentally regress `a=T` / `a=f` / single-shot-fallback paths.
// ---------------------------------------------------------------------------

use oriterm_test_support::spec_chain::kitty_fixtures::{b64, kitty_apc, rgba_4x4_red};

/// Chunked `a=T` (transmit-and-place) MUST create exactly 1 placement.
/// Pins that the cure's action-inheritance preserves the in-flight
/// action when it's already `TransmitAndPlace`.
#[test]
fn kitty_chunked_transmit_and_place_creates_exactly_one_placement() {
    let mut h = SpecHarness::with_size(24, 80);
    let payload = b64(&rgba_4x4_red());
    let mid = payload.len() / 2;
    // First chunk: a=T explicit.
    h.feed(&kitty_apc(b"a=T,i=42,f=32,s=4,v=4,m=1", &payload[..mid]));
    // Terminator chunk: no a=, m=0 — defaults to TransmitAndPlace via
    // `decode_action(None)`. Cure: inherits a=T from first chunk
    // (correct outcome since action was already TransmitAndPlace).
    h.feed(&kitty_apc(b"m=0", &payload[mid..]));

    assert_eq!(
        h.term().image_cache().placement_count(),
        1,
        "chunked a=T MUST create exactly 1 placement after terminator chunk"
    );
    assert_eq!(
        h.term().image_cache().image_count(),
        1,
        "chunked a=T MUST also store exactly 1 image"
    );
}

/// Chunked `a=t` (transmit-only) followed by separate `a=p` (place)
/// MUST create exactly 1 placement. This is the exact pattern
/// `notcurses-info` uses and the one the cure repairs.
#[test]
fn kitty_chunked_transmit_then_separate_place_creates_exactly_one_placement() {
    let mut h = SpecHarness::with_size(24, 80);
    let payload = b64(&rgba_4x4_red());
    let mid = payload.len() / 2;
    // First chunk: a=t explicit (transmit-ONLY).
    h.feed(&kitty_apc(b"a=t,i=43,f=32,s=4,v=4,m=1", &payload[..mid]));
    // Terminator chunk: no a=, m=0 — defaults to TransmitAndPlace via
    // `decode_action(None)`. Cure: inherits a=t from first chunk →
    // dispatches to `kitty_transmit` (NOT `kitty_transmit_and_place`)
    // → finalizes image storage WITHOUT creating a placement.
    h.feed(&kitty_apc(b"m=0", &payload[mid..]));
    // After transmit-only completes: image stored, ZERO placements.
    assert_eq!(
        h.term().image_cache().placement_count(),
        0,
        "chunked a=t (transmit-only) MUST NOT create a placement after \
         terminator. Cure must inherit a=t — defaulting to TransmitAndPlace \
         is the bug this regression pins."
    );
    assert_eq!(h.term().image_cache().image_count(), 1);

    // Now place via separate a=p APC.
    h.feed(&kitty_apc(b"a=p,i=43", ""));
    assert_eq!(
        h.term().image_cache().placement_count(),
        1,
        "a=p after chunked a=t MUST create exactly 1 placement (was 2 pre-cure)"
    );
}

/// Single-shot APC with no `a=` field MUST still default to
/// `TransmitAndPlace` (KG-ACTION-FALLBACK-TRANSMITANDPLACE). The cure
/// only overrides when a chunked upload is in progress
/// (`loading_image.is_some()`), so the non-chunked fallback path is
/// unchanged.
#[test]
fn kitty_single_shot_no_action_still_falls_back_to_transmit_and_place() {
    let mut h = SpecHarness::with_size(24, 80);
    h.feed(&kitty_apc(b"i=44,f=32,s=4,v=4", &b64(&rgba_4x4_red())));
    assert_eq!(
        h.term().image_cache().placement_count(),
        1,
        "single-shot APC with no a= MUST fall back to TransmitAndPlace and \
         create 1 placement (KG-ACTION-FALLBACK-TRANSMITANDPLACE). Cure \
         must NOT change this path."
    );
}
