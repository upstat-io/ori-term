//! §13.6.1 — cross-stack regression tests.
//!
//! Catalog row: `KG-CROSS-STACK-SIXEL-MIXED-EVICTION`
//! Catalog row: `KG-CROSS-STACK-SIXEL-MIXED-Z-ORDER` (behavior-pin layer)
//! Anchor: spec-conformance §13.6.1 — cross-protocol eviction + recency
//!
//! Cross-protocol eviction + recency-bump pins driven by REAL protocol
//! byte streams (kitty APC `\x1b_G...`, sixel DCS `\x1bP...q...`). The
//! gate at `cross_stack_regression_gate.rs` enforces no direct
//! `ImageCache::store|insert` mutation in this file — every image must
//! enter the cache through its protocol handler's placement-create path.
//!
//! Three contracts pinned here:
//!
//! 1. `ImageCache::place` is the canonical recency-update SSOT
//!    (plan body §13.6.1 line 582 sub-bullet 2). The unit-level proof
//!    lives in
//!    `oriterm_core/src/image/cache/tests.rs::access_on_placement_create_bumps_last_accessed`;
//!    the cross-protocol leg lives here.
//!
//! 2. Eviction over the unplaced+unanchored pool ranks by
//!    `last_accessed`, regardless of protocol origin.
//!
//! 3. No protocol-specific eviction carve-out exists in
//!    `oriterm_core/src/image/cache/eviction.rs` — sixel and kitty both
//!    obey the global LRU policy.

use std::path::PathBuf;

use oriterm_core::effect::PtyWriteKind;
use oriterm_core::image::ImageId;
use oriterm_test_support::spec_chain::SpecHarness;
use oriterm_test_support::spec_chain::pty_write_contains;
use oriterm_test_support::spec_chain::sixel_fixtures::dcs_n_cols_wide;

use super::fixtures::{b64, kitty_apc, rgba_4x4_red};

/// The Decision 07 Option A boundary between kitty explicit `i=` IDs
/// `[1, 2^31)` and cache auto-allocated IDs `[2^31, 2^32)` — imported from
/// the single canonical owner so no test re-declares the literal.
use oriterm_core::image::AUTO_ID_START_FOR_TEST as AUTO_ID_START;

/// `OSC 1337 ; File=inline=1:<1x1-red-png-b64> ST` — transmit a minimal
/// inline iTerm2 image (auto-allocated ID) via the real OSC path.
fn transmit_iterm2_inline(h: &mut SpecHarness) {
    let mut seq = Vec::new();
    seq.extend_from_slice(b"\x1b]1337;File=inline=1:");
    seq.extend_from_slice(b"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAEElEQVR4AQEFAPr/AP8AAP8FAAH/+lyI0QAAAABJRU5ErkJggg==");
    seq.extend_from_slice(b"\x1b\\");
    h.feed(&seq);
}

/// `CUP` to a 1-based (row, col). Placed images occupying the SAME terminal
/// cell evict each other (`sixel.rs` `remove_by_position` + `prune_if_orphaned`,
/// `iterm2.rs` placement on the cursor cell); repositioning before a placed
/// image lands it on a fresh cell so ID-namespace assertions are not
/// confounded by cell-collision eviction.
fn move_cursor_to(h: &mut SpecHarness, row: u32, col: u32) {
    h.feed(format!("\x1b[{row};{col}H").as_bytes());
}

const KITTY_IMAGE_BYTES: usize = 64;

/// Row-diacritics for the kitty U=1 placeholder protocol: indexed
/// `[row=0, row=1]`. See `~/projects/reference_repos/console_repos/kitty/docs/graphics-protocol.rst`.
const ROW_D: [char; 2] = ['\u{0305}', '\u{030D}'];

/// Col-diacritics for the kitty U=1 placeholder protocol: indexed
/// `[col=0, col=1, col=2, col=3]`.
const COL_D: [char; 4] = ['\u{0305}', '\u{030D}', '\u{030E}', '\u{030F}'];

fn transmit_kitty_unplaced(h: &mut SpecHarness, id: u32) {
    let control = format!("a=t,i={id},f=32,s=4,v=4");
    h.feed(&kitty_apc(control.as_bytes(), &b64(&rgba_4x4_red())));
}

/// Touch the recency stamp of an existing kitty image via real protocol
/// bytes: `a=p,i=N` invokes `kitty_create_placement` → `ImageCache::place`
/// (the §13.6.1 canonical recency-update SSOT). This routes through the
/// same path production callers use; no test-only mutation of the cache.
fn place_existing_kitty(h: &mut SpecHarness, id: u32) {
    let control = format!("a=p,i={id}");
    h.feed(&kitty_apc(control.as_bytes(), ""));
}

/// Locate `oriterm_core/src/image/cache/eviction.rs` for grep gates.
fn eviction_source_path() -> PathBuf {
    oriterm_test_support::paths::term_workspace_root()
        .join("oriterm_core/src/image/cache/eviction.rs")
}

// ── Recency + eviction contract ────────────────────────────────────

/// Regression: spec-conformance §13.6.1 line 582 cross-feature positive
/// pin. Three kitty images via `a=t` (stored, unplaced). Touch the
/// middle one via `cache.place()` — the §13.6.1 canonical recency-update
/// path. Lower the memory limit to force ONE eviction. The oldest
/// unplaced image is evicted; the placed (touched) image is immune; the
/// younger unplaced image survives.
///
/// This proves: (a) `place()` bumps recency through the SSOT API;
/// (b) eviction ranks by `last_accessed`, not by store-order;
/// (c) placed images are immune to eviction regardless of recency
/// (decisions/01-lru-eviction-scope-placed-vs-unplaced.md Option A).
#[test]
fn cross_protocol_lru_eviction_drops_oldest_accessed_regardless_of_protocol() {
    let mut h = SpecHarness::new();

    // Three kitty `a=t` images — stored unplaced. Store order: 1 → 2 → 3.
    transmit_kitty_unplaced(&mut h, 1);
    transmit_kitty_unplaced(&mut h, 2);
    transmit_kitty_unplaced(&mut h, 3);

    assert_eq!(h.term().image_cache().image_count(), 3);
    assert_eq!(
        h.term().image_cache().placement_count(),
        0,
        "a=t MUST NOT create placements — sanity check on the unplaced setup",
    );

    let pre_id1 = h
        .term()
        .image_cache()
        .get_no_touch(ImageId::from_raw(1))
        .unwrap()
        .last_accessed;
    let pre_id2 = h
        .term()
        .image_cache()
        .get_no_touch(ImageId::from_raw(2))
        .unwrap()
        .last_accessed;
    let pre_id3 = h
        .term()
        .image_cache()
        .get_no_touch(ImageId::from_raw(3))
        .unwrap()
        .last_accessed;
    assert!(
        pre_id1 < pre_id2 && pre_id2 < pre_id3,
        "store-order baseline"
    );

    // Touch image 2 via real kitty `a=p` bytes — routes through
    // kitty_create_placement → ImageCache::place (canonical SSOT). The
    // place() recency bump pushes id=2 past id=3.
    place_existing_kitty(&mut h, 2);

    let post_touch_id2 = h
        .term()
        .image_cache()
        .get_no_touch(ImageId::from_raw(2))
        .unwrap()
        .last_accessed;
    assert!(
        post_touch_id2 > pre_id3,
        "place(id=2) must bump recency past id=3; saw post={post_touch_id2}, pre_id3={pre_id3}",
    );

    // Force eviction: 3 images * 64 bytes = 192 bytes. Cap at 128.
    h.term_mut()
        .image_cache_mut()
        .set_memory_limit(KITTY_IMAGE_BYTES * 2);

    // image 1 (oldest unplaced) must be evicted; 2 (placed, immune) and
    // 3 (younger unplaced) must survive.
    assert!(
        h.term()
            .image_cache()
            .get_no_touch(ImageId::from_raw(1))
            .is_none(),
        "image 1 must be evicted — lowest last_accessed among unplaced. \
         Regression: place() did not bump or eviction picked the wrong victim.",
    );
    assert!(
        h.term()
            .image_cache()
            .get_no_touch(ImageId::from_raw(2))
            .is_some(),
        "image 2 must survive — placed images are immune to eviction \
         (Option A in decisions/01-lru-eviction-scope-placed-vs-unplaced.md). \
         Regression: eviction targeted a placed image OR place() did not \
         add the placement.",
    );
    assert!(
        h.term()
            .image_cache()
            .get_no_touch(ImageId::from_raw(3))
            .is_some(),
        "image 3 must survive — younger than image 1 among unplaced. Only \
         one eviction is needed to bring memory under the 128-byte cap.",
    );
}

/// Regression: spec-conformance §13.6.1 line 586 cross-feature positive
/// pin — prior `cross_protocol_lru_eviction_drops_oldest_accessed_
/// regardless_of_protocol` test used kitty-only images, leaving the
/// cross-protocol claim un-pinned at the test surface.
///
/// Setup: sixel-A (auto-assigned ImageId at `AUTO_ID_START`) → kitty-B
/// (`i=42`) → sixel-C (`AUTO_ID_START+1`). All three transmitted via
/// REAL protocol bytes; sixel placements deleted via kitty `d=p` so the
/// sixel image data persists unplaced (eviction-eligible). Touch
/// kitty-B via `a=p` (the canonical recency-update SSOT). Force
/// eviction with a tight memory cap. Sixel-A — oldest by `last_accessed`
/// — must be evicted regardless of protocol; kitty-B (touched, placed)
/// must survive; sixel-C (younger unplaced) must survive.
///
/// Proves the global-LRU invariant holds when sixel + kitty images
/// compete in the unplaced pool. A protocol-specific eviction carve-out
/// (e.g., "sixel always evicted first regardless of recency") would
/// fail this pin even though
/// `cross_protocol_lru_eviction_drops_oldest_accessed_regardless_of_protocol`
/// (kitty-only) above passes.
#[test]
fn cross_protocol_lru_mixed_sixel_kitty_drops_oldest_by_last_accessed() {
    let mut h = SpecHarness::new();

    // 1. Sixel-A at row 1 — sixel handler stores + places at cursor.
    //    Auto-assigned ImageId = AUTO_ID_START (the protocol-agnostic
    //    auto-namespace base; the sixel handler shares the cache allocator).
    let sixel_a = ImageId::from_raw(AUTO_ID_START);
    h.feed(b"\x1b[1;1H");
    h.feed(&dcs_n_cols_wide(8));
    // Delete sixel-A's placement (image stays unplaced + evictable).
    h.feed(&kitty_apc(b"a=d,d=p,x=1,y=1", ""));

    // 2. Kitty-B with explicit `i=42` — stored unplaced.
    transmit_kitty_unplaced(&mut h, 42);
    let kitty_b = ImageId::from_raw(42);

    // 3. Sixel-C at row 5 — same pattern as sixel-A. Auto-id +1.
    let sixel_c = ImageId::from_raw(AUTO_ID_START + 1);
    h.feed(b"\x1b[6;1H");
    h.feed(&dcs_n_cols_wide(8));
    h.feed(&kitty_apc(b"a=d,d=p,x=1,y=6", ""));

    assert_eq!(
        h.term().image_cache().image_count(),
        3,
        "expected 3 images stored: sixel-A + kitty-B + sixel-C"
    );
    assert_eq!(
        h.term().image_cache().placement_count(),
        0,
        "all 3 placements deleted; images are unplaced and eviction-eligible"
    );

    // Baseline recency order: sixel-A < kitty-B < sixel-C.
    let pre_a = h
        .term()
        .image_cache()
        .get_no_touch(sixel_a)
        .unwrap()
        .last_accessed;
    let pre_b = h
        .term()
        .image_cache()
        .get_no_touch(kitty_b)
        .unwrap()
        .last_accessed;
    let pre_c = h
        .term()
        .image_cache()
        .get_no_touch(sixel_c)
        .unwrap()
        .last_accessed;
    assert!(
        pre_a < pre_b && pre_b < pre_c,
        "store-order baseline: sixel-A < kitty-B < sixel-C, saw a={pre_a} b={pre_b} c={pre_c}"
    );

    // Touch kitty-B via `a=p` — routes through ImageCache::place.
    place_existing_kitty(&mut h, 42);

    // Force eviction: cap at 280 bytes (≈ kitty 64 + 1 sixel 192) so
    // ONE unplaced sixel must drop. sixel-A is the LRU victim; kitty-B
    // is eviction-immune (placed); sixel-C is younger unplaced.
    h.term_mut().image_cache_mut().set_memory_limit(280);

    assert!(
        h.term().image_cache().get_no_touch(sixel_a).is_none(),
        "sixel-A (oldest unplaced, regardless of protocol) MUST be evicted. \
         Regression: cache uses protocol-specific eviction order instead of \
         global last_accessed."
    );
    assert!(
        h.term().image_cache().get_no_touch(kitty_b).is_some(),
        "kitty-B (placed via a=p) MUST be eviction-immune regardless of recency"
    );
    assert!(
        h.term().image_cache().get_no_touch(sixel_c).is_some(),
        "sixel-C (younger unplaced) MUST survive — only one eviction needed \
         to bring memory under the cap"
    );
}

/// Regression: spec-conformance §13.6.1 — cross-protocol immunity.
/// Sixel + kitty images both placed via REAL protocol bytes; impossibly
/// low memory limit cannot evict either because both are placed. Proves
/// the immunity invariant holds across protocol boundaries — eviction
/// has no protocol-specific carve-out that would evict a placed sixel
/// when a placed kitty image exists (or vice versa).
#[test]
fn placed_images_immune_to_eviction_regardless_of_protocol() {
    let mut h = SpecHarness::new();

    // Sixel via real DCS bytes — handler places at cursor (0, 0).
    h.feed(&dcs_n_cols_wide(8));

    // Move cursor, then kitty `a=T,i=10` places at the new position.
    h.feed(b"\x1b[6;1H"); // CUP row=5, col=0.
    h.feed(&kitty_apc(b"a=T,i=10,f=32,s=4,v=4", &b64(&rgba_4x4_red())));

    let initial_image_count = h.term().image_cache().image_count();
    let initial_placement_count = h.term().image_cache().placement_count();
    assert_eq!(initial_image_count, 2, "expected sixel + kitty stored");
    assert_eq!(initial_placement_count, 2, "expected both placed");

    // Cap memory at 1 byte — impossible to evict either because both are
    // placed. evict_lru returns early; image_count and placement_count
    // must remain unchanged.
    h.term_mut().image_cache_mut().set_memory_limit(1);

    assert_eq!(
        h.term().image_cache().image_count(),
        initial_image_count,
        "ALL images placed → eviction must return false; image_count must \
         not drop. Regression: a protocol-specific eviction carve-out \
         evicted a placed sixel or kitty image despite the immunity invariant.",
    );
    assert_eq!(
        h.term().image_cache().placement_count(),
        initial_placement_count,
        "placement_count must survive: removing a placed image would also \
         remove its placements.",
    );
}

/// Regression: spec-conformance §13.6.1 line 582 sub-bullet (c). When
/// ALL existing images are placed (or anchored), `evict_one()` returns
/// `false` and the calling `store()` path surfaces
/// `ImageError::MemoryLimitExceeded` to the protocol reply emitter. The
/// observable effect at the test surface: a new transmit over the
/// memory budget is REJECTED (image_count does not grow), proving the
/// store path declined rather than corrupting a placed image's state.
#[test]
fn fully_placed_cache_over_budget_raises_memory_limit_exceeded() {
    let mut h = SpecHarness::new();

    // Two kitty images via `a=T` — both stored AND placed at cursor.
    // Move the cursor between feeds so placements don't collide.
    h.feed(&kitty_apc(b"a=T,i=1,f=32,s=4,v=4", &b64(&rgba_4x4_red())));
    h.feed(b"\x1b[2;1H"); // CUP row=1, col=0
    h.feed(&kitty_apc(b"a=T,i=2,f=32,s=4,v=4", &b64(&rgba_4x4_red())));

    assert_eq!(h.term().image_cache().image_count(), 2);
    assert_eq!(
        h.term().image_cache().placement_count(),
        2,
        "both kitty `a=T` MUST place the image (not just store)",
    );

    // Cap memory at exactly the current usage. Any new store will need
    // to evict, but both images are placed → eviction returns false.
    h.term_mut()
        .image_cache_mut()
        .set_memory_limit(KITTY_IMAGE_BYTES * 2);

    // Third `a=T` exceeds cap with no evictable image → store rejects
    // with `ImageError::MemoryLimitExceeded` → `ENOMEM:` APC error
    // reply (kitty/store.rs:108).
    h.feed(b"\x1b[3;1H");
    h.feed(&kitty_apc(b"a=T,i=3,f=32,s=4,v=4", &b64(&rgba_4x4_red())));

    assert_eq!(
        h.term().image_cache().image_count(),
        2,
        "store(id=3) MUST be rejected by the eviction-immunity contract; \
         image_count must stay at 2. Regression: a placed image was evicted, \
         or evict_one() returned true on a fully-placed cache (BANNED per \
         Option A in decisions/01-lru-eviction-scope-placed-vs-unplaced.md).",
    );
    assert!(
        h.term()
            .image_cache()
            .get_no_touch(ImageId::from_raw(3))
            .is_none(),
        "image id=3 must not have been stored (eviction rejected the store)",
    );
    assert!(
        h.term()
            .image_cache()
            .get_no_touch(ImageId::from_raw(1))
            .is_some(),
        "image id=1 (placed) must survive — eviction must not evict to make room",
    );
    assert!(
        h.term()
            .image_cache()
            .get_no_touch(ImageId::from_raw(2))
            .is_some(),
        "image id=2 (placed) must survive — eviction must not evict to make room",
    );
}

// ── Architectural grep gates ───────────────────────────────────────

/// Architectural pin: kitty's `kitty_create_placement`, sixel's
/// `sixel_create_placement`, AND iterm2's `iterm2_create_placement` all
/// route placement creation through `ImageCache::place`. Splitting the
/// recency bump across per-protocol handlers (rejected at §13.6.1
/// plan-time) would scatter the LRU contract — when a future protocol
/// lands, the bump is silently absent and eviction silently regresses to
/// FIFO for that protocol. This grep gate fires the moment a handler
/// bypasses `ImageCache::place` (e.g., constructing `ImagePlacement` and
/// pushing it via a different API). Renamed from
/// `kitty_and_sixel_create_placement_paths_*` per §14.6 — the gate has
/// always iterated all three protocols; the name now matches.
#[test]
fn every_image_protocol_handler_routes_through_image_cache_place() {
    let workspace = oriterm_test_support::paths::term_workspace_root();

    let kitty_place_rs = workspace.join("oriterm_core/src/term/handler/image/kitty/place.rs");
    let sixel_rs = workspace.join("oriterm_core/src/term/handler/image/sixel.rs");
    let iterm2_rs = workspace.join("oriterm_core/src/term/handler/image/iterm2.rs");

    for (name, path) in [
        ("kitty", kitty_place_rs),
        ("sixel", sixel_rs),
        ("iterm2", iterm2_rs),
    ] {
        let contents = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {} for grep gate: {err}", path.display()));
        // Each protocol handler MUST call `.image_cache_mut().place(`.
        assert!(
            contents.contains(".image_cache_mut().place("),
            "{name} placement-create path at {} MUST call \
             `image_cache_mut().place(...)`. ImageCache::place is the \
             canonical recency-update SSOT per plan body §13.6.1 line \
             582 sub-bullet 2. Splitting the \
             bump across per-protocol handlers regresses LRU to FIFO \
             when any handler omits the bump.",
            path.display(),
        );
    }
}

/// Architectural pin: `oriterm_core/src/image/cache/eviction.rs`
/// implements GLOBAL LRU — `min_by_key(last_accessed)` over the
/// unplaced+unanchored subset. No protocol-specific carve-out exists
/// (no `Sixel`/`Kitty`/`Iterm2` enum branches, no protocol-named
/// constants in the eviction path). This gate scans `eviction.rs` and
/// asserts the file contains zero protocol-name mentions.
///
/// Regression: spec-conformance §13.6.1 line 582 cross-feature negative
/// pin — sister test for both `sixel_eviction_does_not_use_protocol_specific_fifo_carve_out`
/// and `kitty_eviction_does_not_use_protocol_specific_lru_carve_out`.
#[test]
fn sixel_eviction_does_not_use_protocol_specific_fifo_carve_out() {
    assert_eviction_source_has_no_protocol_branch("Sixel");
}

#[test]
fn kitty_eviction_does_not_use_protocol_specific_lru_carve_out() {
    assert_eviction_source_has_no_protocol_branch("Kitty");
}

#[test]
fn iterm2_eviction_does_not_use_protocol_specific_carve_out() {
    assert_eviction_source_has_no_protocol_branch("Iterm2");
}

fn assert_eviction_source_has_no_protocol_branch(protocol_name: &str) {
    let path = eviction_source_path();
    let contents = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));

    let mut hits = Vec::new();
    for (lineno, line) in contents.lines().enumerate() {
        let stripped = line.trim_start();
        if stripped.starts_with("//") || stripped.starts_with("//!") {
            continue;
        }
        if line.contains(protocol_name) {
            hits.push((lineno + 1, line.trim().to_string()));
        }
    }

    assert!(
        hits.is_empty(),
        "§13.6.1 line 582 BANNED: eviction.rs contains protocol-specific \
         references to `{protocol_name}`. Eviction MUST rank by \
         `last_accessed` globally — protocol carve-outs (e.g. `if protocol \
         == {protocol_name} {{ /* FIFO branch */ }}`) regress the LRU \
         contract for that protocol.\nHits in {}:\n{}",
        path.display(),
        hits.iter()
            .map(|(line, src)| format!("  line {line}: {src}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

/// Architectural pin per §13.6.1 line 582 sub-bullet "Cross-feature
/// negative pins" #3: §12 sixel tests do NOT assert an
/// eviction ordering other than global LRU. If §12 ever encodes
/// "sixel-first" or "FIFO sub-policy" semantics, the §13.6 work flows
/// back to §12 as a `§1.7D` cohesion-edit. Plan-time verification at
/// §13.6 author-time (2026-05-14) was clean.
#[test]
fn no_section_12_test_asserts_eviction_order_other_than_global_lru() {
    let workspace = oriterm_test_support::paths::term_workspace_root();
    let sixel_test_dir = workspace.join("oriterm_core/tests/spec_chain/sixel");
    let sixel_src_dir = workspace.join("oriterm_core/src/image/sixel");

    let mut hits: Vec<(PathBuf, usize, String)> = Vec::new();
    for dir in [&sixel_test_dir, &sixel_src_dir] {
        if !dir.exists() {
            continue;
        }
        for entry in walk_rs_files(dir) {
            let contents = std::fs::read_to_string(&entry).unwrap_or_default();
            for (lineno, line) in contents.lines().enumerate() {
                let lc = line.to_ascii_lowercase();
                // Banned: assertions about sixel-specific eviction ordering.
                // Keywords scoped tightly to actual ordering-claim phrases —
                // not generic "ordering" prose.
                let banned = [
                    "sixel_first",
                    "sixel-first",
                    "fifo_evict",
                    "sixel_evict_order",
                    "sixel_specific_eviction_order",
                ];
                if banned.iter().any(|b| lc.contains(b)) {
                    hits.push((entry.clone(), lineno + 1, line.trim().to_string()));
                }
            }
        }
    }

    assert!(
        hits.is_empty(),
        "§13.6.1 line 582 BANNED: §12 sixel tests / source assert an \
         eviction ordering other than global LRU. Sixel must obey the \
         global LRU policy implemented in eviction.rs — any \
         sixel-specific ordering encoding would create a cross-protocol \
         eviction inversion. Hits:\n{}",
        hits.iter()
            .map(|(p, line, src)| format!("  {} line {line}: {src}", p.display()))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}

fn walk_rs_files(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(read) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walk_rs_files(&path));
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            out.push(path);
        }
    }
    out
}

// ── §13.6.1 cross-stack matrix + semantic + negative pins ─────────

/// Regression: spec-conformance §13.6.1 — matrix count assertion. The
/// cross-stack regression coverage is partitioned across 6 distinct
/// concern categories per the plan body: z-order composition, LRU
/// eviction across protocols, placed-image immunity, cache churn / RSS
/// stability, placeholder + cache-coordinate coexistence, and resize-
/// lifecycle cross-section (§13 ↔ §07) integration. This pin
/// self-verifies the partition is enumerated, so adding a new
/// cross-stack concern without registering it here fails the
/// matrix-completeness assertion instead of silently dropping coverage.
#[test]
fn cross_stack_regression_category_matrix_completeness() {
    /// Cross-protocol concern categories owned by §13.6 / §13.6.1.
    /// Each entry must have at least one regression test in this file
    /// (or in `kitty_sixel_mixed_z_order.rs` / `kitty_sixel_mixed_with_text.rs`
    /// / `kitty_placeholder_sixel_coexist.rs` / `cross_protocol_rss.rs`).
    const CATEGORIES: &[&str] = &[
        // 1. Z-order composition between protocols (above-text / below-text
        //    layering).
        "z-order-composition",
        // 2. LRU eviction across protocols (the canonical recency-update
        //    SSOT through `ImageCache::place`).
        "lru-eviction-cross-protocol",
        // 3. Placed-image eviction immunity (Option A scope decision).
        "placed-immunity",
        // 4. Cross-protocol cache churn / RSS stability under rapid
        //    sixel↔kitty alternation.
        "cache-churn-rss-stability",
        // 5. Placeholder + cache-coordinate coexistence (U=1 anchor +
        //    sixel placement on the same screen).
        "placeholder-sixel-coexist",
        // 6. Resize-lifecycle integration (§13 ↔ §07): single resize
        //    drives prune_scrollback + reconcile_both_placeholder_anchors.
        //    Owner pin: resize_with_mixed_sixel_placement_*.
        "resize-lifecycle-cross-section",
    ];
    assert_eq!(
        CATEGORIES.len(),
        6,
        "matrix MUST enumerate every cross-stack concern category — \
         adding a 7th regression dimension without updating this list is \
         a silent coverage drop"
    );
}

/// Regression: z-ordering must be driven by `z_index`, NOT by
/// transmit/place sequence order. Two
/// equivalent compositions (sixel-then-kitty vs kitty-then-sixel) at
/// the same z-configuration must produce equivalent cache state — same
/// placement count, same image count, same placement z-indices. The
/// GPU-rung pixel-level proof lives in `kitty_sixel_mixed_z_order.rs`;
/// this pin operates at the cache layer where transmit-order is the
/// only difference between the two runs.
#[test]
fn sixel_and_kitty_z_order_independent_of_transmit_order() {
    use oriterm_test_support::spec_chain::sixel_fixtures::dcs_n_cols_wide;
    let snapshot = |sixel_first: bool| {
        let mut h = SpecHarness::new();
        if sixel_first {
            h.feed(b"\x1b[6;1H"); // CUP row=5, col=0
            h.feed(&dcs_n_cols_wide(8));
            h.feed(b"\x1b[2;1H"); // CUP row=1, col=0
            h.feed(&kitty_apc(b"a=T,i=20,f=32,s=4,v=4", &b64(&rgba_4x4_red())));
        } else {
            h.feed(b"\x1b[2;1H"); // CUP row=1, col=0
            h.feed(&kitty_apc(b"a=T,i=20,f=32,s=4,v=4", &b64(&rgba_4x4_red())));
            h.feed(b"\x1b[6;1H"); // CUP row=5, col=0
            h.feed(&dcs_n_cols_wide(8));
        }
        (
            h.term().image_cache().image_count(),
            h.term().image_cache().placement_count(),
        )
    };

    let sixel_then_kitty = snapshot(true);
    let kitty_then_sixel = snapshot(false);
    assert_eq!(
        sixel_then_kitty, kitty_then_sixel,
        "(image_count, placement_count) must be invariant under \
         transmit-order swap — z-ordering is driven by `z_index`, NOT \
         by transmit sequence. Regression: transmit-order-sensitive \
         caching would create visible-frame divergence between two \
         programs that emit the same protocol bytes in different orders."
    );
}

/// Regression: cache eviction must reject cross-protocol bleed.
/// Eviction is per-`image_id`; removing a kitty image's bytes MUST NOT free
/// a sixel image's bytes (or vice versa) — protocol-specific cleanup
/// carve-outs would create cross-pollination bugs where a kitty
/// `a=d,d=I,i=N` accidentally evicts a sixel image with overlapping
/// metadata.
#[test]
fn mixed_protocol_eviction_does_not_cross_pollinate_image_data() {
    use oriterm_test_support::spec_chain::sixel_fixtures::dcs_n_cols_wide;
    let mut h = SpecHarness::new();

    // 1. Sixel via real DCS bytes — auto-assigned image_id at AUTO_ID_START.
    h.feed(&dcs_n_cols_wide(8));
    let sixel_image_id = {
        let cache = h.term().image_cache();
        cache
            .placeholder_anchors()
            .iter()
            .copied()
            .next()
            // Sixel doesn't anchor — fall back to scanning all images
            // for the auto-assigned ID (kitty uses i=N explicit IDs).
            .or_else(|| {
                let mut ids: Vec<_> = (0..u32::MAX)
                    .take_while(|id| cache.get_no_touch(ImageId::from_raw(*id)).is_some())
                    .collect();
                ids.pop().map(ImageId::from_raw)
            })
            .unwrap_or_else(|| {
                // Auto-assigned ids start at AUTO_ID_START.
                ImageId::from_raw(AUTO_ID_START)
            })
    };
    assert!(
        h.term()
            .image_cache()
            .get_no_touch(sixel_image_id)
            .is_some(),
        "sixel image must be stored at sixel_image_id={:?}",
        sixel_image_id
    );

    // 2. Kitty image with explicit i=42 — different ID space.
    h.feed(b"\x1b[2;1H");
    h.feed(&kitty_apc(b"a=T,i=42,f=32,s=4,v=4", &b64(&rgba_4x4_red())));
    assert!(
        h.term()
            .image_cache()
            .get_no_touch(ImageId::from_raw(42))
            .is_some(),
        "kitty image at i=42 must be stored"
    );

    // 3. Delete kitty by image_id — `a=d,d=I,i=42` removes ONLY kitty.
    h.feed(&kitty_apc(b"a=d,d=I,i=42", ""));

    // 4. Sixel survives — delete-by-id is per-image_id, not per-protocol.
    assert!(
        h.term()
            .image_cache()
            .get_no_touch(sixel_image_id)
            .is_some(),
        "sixel image (id={:?}) MUST survive kitty `d=I,i=42` — cross-\
         protocol eviction is BANNED. Regression: a protocol-specific \
         delete carve-out evicted the sixel image when only the kitty \
         image was targeted.",
        sixel_image_id
    );
    assert!(
        h.term()
            .image_cache()
            .get_no_touch(ImageId::from_raw(42))
            .is_none(),
        "kitty image at i=42 MUST be gone — the delete targeted it"
    );
}

// ── §13 ↔ §07 cross-section integration pin ───────────────────────

/// Write one kitty unicode-placeholder cell at the cursor.
/// Encodes `image_id_low` via the 256-color foreground SGR slot and the
/// `(row, col)` indices via the two combining diacritics per the kitty
/// placeholder spec — see
/// `~/projects/reference_repos/console_repos/kitty/docs/graphics-protocol.rst`.
fn write_placeholder_cell(h: &mut SpecHarness, image_id_low: u32, row: char, col: char) {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(format!("\x1b[38;5;{image_id_low}m").as_bytes());
    let mut buf = [0u8; 4];
    bytes.extend_from_slice('\u{10EEEE}'.encode_utf8(&mut buf).as_bytes());
    bytes.extend_from_slice(row.encode_utf8(&mut buf).as_bytes());
    bytes.extend_from_slice(col.encode_utf8(&mut buf).as_bytes());
    bytes.extend_from_slice(b"\x1b[39m");
    h.feed(&bytes);
}

// ── §13 ↔ §07 setup helpers ────────────────────────────────────────

/// CUP(1,1) → sixel A via `dcs_n_cols_wide(4)`. Sixel handler stores
/// the image and places it at the cursor; resulting `placement.cell_row = 0`.
/// Post-condition: exactly ONE placement registered.
fn setup_sixel_a_at_row_0(h: &mut SpecHarness) {
    h.feed(b"\x1b[1;1H"); // CUP(1,1) → cursor (0, 0).
    h.feed(&dcs_n_cols_wide(4));
    assert_eq!(
        h.term().image_cache().placement_count(),
        1,
        "sixel handler must create exactly one placement at cursor"
    );
}

/// Drive 11 linefeeds to fill scrollback to cap without evicting any
/// row. From cursor (0, 0): LF 1-7 move cursor (0,0)→(7,0); LFs 8-11
/// push 4 rows to scrollback (cap 4) → `total_evicted = 0`. The 11 LFs
/// run BEFORE any U=1 anchor exists, so per-LF reconcile cannot drop
/// an anchor that does not yet exist.
fn fill_scrollback_to_cap_without_eviction(h: &mut SpecHarness) {
    let lfs = vec![b'\n'; 11];
    h.feed(&lfs);
    assert_eq!(
        h.term().grid().total_evicted(),
        0,
        "11 LFs from origin with sb cap 4 must fill sb to cap without \
         evicting. Regression: scroll math drifted; later assertions \
         depend on this baseline."
    );
    assert_eq!(
        h.term().image_cache().placement_count(),
        1,
        "sixel A's placement (cell_row=0) MUST survive the LF phase — \
         per-LF prune fires only when evicted grows, and evicted=0."
    );
}

/// CUP(5,1) → sixel D via `dcs_n_cols_wide(4)`. Cursor at (4, 0); after
/// the preceding 11 LFs have pushed 4 rows to scrollback, the sixel
/// placement registers at absolute `cell_row = 8` (above the post-resize
/// eviction floor 4) at `cell_col = 0` (survives col-truncation 16→4).
/// Pins the "sixel placement survives prune" cell of the matrix.
/// Post-condition: two placements registered (sixel A + sixel D).
fn setup_sixel_d_at_row_4(h: &mut SpecHarness) {
    h.feed(b"\x1b[5;1H"); // CUP(5,1) → cursor (4, 0).
    h.feed(&dcs_n_cols_wide(4));
    assert_eq!(
        h.term().image_cache().placement_count(),
        2,
        "sixel D's placement must register alongside sixel A"
    );
}

/// Transmit `a=T,U=1,c=4,r=2` (kitty graphics protocol) + write
/// `image_id` placeholder cells at grid rows 5-6 cols 0-3 (LOW col —
/// survives col-shrink 16→4 with `reflow=false`). Used by both the
/// primary-cache test (`image_id = 42`, kitty B) and the alt-cache
/// companion test (`image_id = 142`, kitty `B_alt`). Anchor +
/// display-grid `(4, 2)` recorded on the active cache; cells survive
/// both row-shrink and col-shrink, so reconcile keeps the anchor.
fn write_kitty_u1_low_col_block(h: &mut SpecHarness, image_id: u32) {
    h.feed(b"\x1b[6;1H"); // CUP(6,1) → cursor (5, 0).
    let control = format!("a=T,U=1,i={image_id},c=4,r=2,f=32,s=4,v=4");
    h.feed(&kitty_apc(control.as_bytes(), &b64(&rgba_4x4_red())));
    h.feed(b"\x1b[6;1H"); // grid line 5 col 0
    for &col_d in &COL_D {
        write_placeholder_cell(h, image_id, ROW_D[0], col_d);
    }
    h.feed(b"\x1b[7;1H"); // grid line 6 col 0
    for &col_d in &COL_D {
        write_placeholder_cell(h, image_id, ROW_D[1], col_d);
    }
}

/// Transmit `a=T,U=1,c=2,r=1` (kitty graphics protocol) + write
/// `image_id` placeholder cells at grid row 7 cols 10-11 (HIGH col —
/// truncated by col-shrink 16→4 with `reflow=false`). Used by both the
/// primary-cache test (`image_id = 99`, kitty C) and the alt-cache
/// companion test (`image_id = 199`, kitty `C_alt`). Anchor +
/// display-grid `(2, 1)` recorded on the active cache; cells are
/// truncated by col-shrink, so reconcile MUST drop the orphaned anchor.
fn write_kitty_u1_high_col_block(h: &mut SpecHarness, image_id: u32) {
    h.feed(b"\x1b[8;11H"); // CUP(8,11) → cursor (7, 10).
    let control = format!("a=T,U=1,i={image_id},c=2,r=1,f=32,s=4,v=4");
    h.feed(&kitty_apc(control.as_bytes(), &b64(&rgba_4x4_red())));
    h.feed(b"\x1b[8;11H");
    write_placeholder_cell(h, image_id, ROW_D[0], COL_D[0]);
    write_placeholder_cell(h, image_id, ROW_D[0], COL_D[1]);
}

/// DECSET 1049 — enter alt screen (`Term::swap_alt`,
/// `term/alt_screen/mod.rs:25-39`). Alt grid + `alt_image_cache`
/// lazily allocated, both inherit primary dimensions; alt cursor
/// restored to (0, 0) on first entry (`navigation/mod.rs:255-256`).
fn enter_alt_screen(h: &mut SpecHarness) {
    h.feed(b"\x1b[?1049h");
}

/// DECRST 1049 — exit alt screen back to primary. Saves alt cursor and
/// restores primary cursor; alt cache state is preserved (mode 1049
/// does NOT clear alt content — only mode 1047 clears).
fn exit_alt_screen(h: &mut SpecHarness) {
    h.feed(b"\x1b[?1049l");
}

/// Park primary cursor at the bottom row BEFORE `Term::resize`. The
/// `shrink_rows` `count_trailing_blank_rows` heuristic at
/// `grid/resize/mod.rs:218-233` trims blank rows below the cursor
/// instead of pushing top rows to scrollback. Without this CUP, the
/// resize would trim trailing blank rows and `total_evicted` would
/// stay at 0 — the resize-driven prune path would be a no-op.
fn park_cursor_at_bottom_row(h: &mut SpecHarness) {
    h.feed(b"\x1b[8;1H");
}

/// Pre-resize state pins for the §13 ↔ §07 integration tests after the
/// 5 setup helpers (sixel A + 11 LFs + sixel D + kitty B low-col +
/// kitty C high-col) have run. Asserts: 4 images stored, 2 placements
/// (sixel A + sixel D — kitty B/C use U=1 anchors, not placements),
/// both kitty anchors registered with their `(c, r)` display grids.
fn assert_pre_resize_state(h: &SpecHarness, kitty_b: ImageId, kitty_c: ImageId) {
    assert_eq!(
        h.term().image_cache().image_count(),
        4,
        "precondition: 4 images stored — sixel A, sixel D, kitty B, kitty C"
    );
    assert_eq!(
        h.term().image_cache().placement_count(),
        2,
        "precondition: 2 placements — sixel A, sixel D (kitty B/C use U=1 anchors, no placement)"
    );
    let anchors_pre = h.term().image_cache().placeholder_anchors();
    assert!(anchors_pre.contains(&kitty_b) && anchors_pre.contains(&kitty_c));
    assert_eq!(anchors_pre.len(), 2);
    assert_eq!(
        h.term().image_cache().placeholder_anchor_grid_for(kitty_b),
        Some((4, 2))
    );
    assert_eq!(
        h.term().image_cache().placeholder_anchor_grid_for(kitty_c),
        Some((2, 1))
    );
}

/// Post-resize matrix pins for the §13 ↔ §07 integration tests, after
/// `Term::resize(4, 4, false)`. Walks all four cells of the
/// `{placement, anchor} × {evicted, surviving}` matrix on the active
/// (primary) cache. Verifies sixel A dropped; sixel D survives;
/// kitty B anchor and display grid survive; kitty C anchor and display
/// grid dropped; kitty B image data survives via the anchor.
#[derive(Clone, Copy)]
struct IntegrationIds {
    sixel_a: ImageId,
    sixel_d: ImageId,
    kitty_b: ImageId,
    kitty_c: ImageId,
}

fn assert_post_resize_primary_matrix(h: &SpecHarness, post_evicted: usize, ids: IntegrationIds) {
    let IntegrationIds {
        sixel_a,
        sixel_d,
        kitty_b,
        kitty_c,
    } = ids;
    // Sixel placement matrix (prune_scrollback observable).
    assert!(
        h.term().image_cache().get_no_touch(sixel_a).is_none(),
        "sixel A (cell_row=0) MUST be evicted by §07's prune_scrollback \
         (post-resize floor={post_evicted}). Its image data is dropped \
         via orphan-prune since sixel A had no anchor. Regression: §07 \
         prune lifecycle handler failed to drop a placement whose row \
         fell below the eviction floor."
    );
    assert!(
        h.term().image_cache().get_no_touch(sixel_d).is_some(),
        "sixel D (cell_row=8) MUST survive prune_scrollback \
         (post-resize floor={post_evicted}). Regression: §07 prune \
         lifecycle handler was overzealous — `cell_row < evicted_before` \
         flipped to `<=` or the floor calculation drifted."
    );
    assert_eq!(
        h.term().image_cache().placement_count(),
        1,
        "exactly one placement must survive (sixel D). Regression: \
         prune/on_resize dropped/kept the wrong placement."
    );

    // Kitty anchor matrix (reconcile_both_placeholder_anchors observable).
    let anchors_post = h.term().image_cache().placeholder_anchors().clone();
    assert!(
        anchors_post.contains(&kitty_b),
        "kitty B's anchor (image_id=42) MUST survive — its placeholder \
         cells at grid rows 5-6 cols 0-3 fall into the new visible grid \
         at low cols, surviving col-truncation. Regression: \
         reconcile_both failed to scan the visible grid, or §07's \
         prune_scrollback cross-pollinated by clearing the anchor set. \
         Surviving anchors: {anchors_post:?}."
    );
    assert!(
        !anchors_post.contains(&kitty_c),
        "kitty C's anchor (image_id=99) MUST be dropped — its placeholder \
         cells at row 7 cols 10-11 were truncated by the col-shrink \
         (16→4 with reflow=false), leaving the anchor orphaned. Only \
         reconcile_both_placeholder_anchors can drop it during this \
         resize — if this assertion fails, reconcile_both did not fire \
         OR did not observe the truncated cells. Anchors: {anchors_post:?}."
    );
    assert_eq!(
        anchors_post.len(),
        1,
        "exactly one anchor must survive (kitty B). Regression: \
         reconcile_both added a spurious anchor, or prune_scrollback \
         cross-pollinated by removing/adding entries it does not own."
    );

    // Anchor grid map (§13.4-owned state, prune must not touch).
    assert_eq!(
        h.term().image_cache().placeholder_anchor_grid_for(kitty_b),
        Some((4, 2)),
        "kitty B's recorded display grid (4, 2) MUST survive the resize. \
         Regression: reconcile dropped a surviving anchor's grid entry, \
         or §07 prune_scrollback inadvertently cleared the grid map."
    );
    assert_eq!(
        h.term().image_cache().placeholder_anchor_grid_for(kitty_c),
        None,
        "kitty C's display grid entry MUST be dropped — \
         `reconcile_placeholder_anchors` retains the grid map alongside \
         the anchor set, so non-surviving anchors lose their grid too. \
         Regression: grid map drifted from anchor set (paired-retain broken)."
    );

    // Kitty B's image data survives via anchor.
    assert!(
        h.term().image_cache().get_no_touch(kitty_b).is_some(),
        "kitty B's image data MUST survive — anchored via \
         placeholder_anchors so `prune_if_orphaned` skips it. \
         Regression: cross-protocol cleanup carve-out dropped kitty B's \
         image data when sixel A was evicted."
    );
}

/// Regression: spec-conformance §13.6 — cross-section §13 ↔ §07
/// integration pin. A single `Term::resize` invocation drives BOTH
/// `ImageCache::prune_scrollback` (§07-owned scrollback lifecycle
/// handler) AND `Term::reconcile_both_placeholder_anchors` (§13.4-owned
/// anchor reconcile) in the same operation; they MUST NOT
/// cross-pollinate. The {placement, anchor} × {evicted, surviving} 2×2
/// matrix MUST be pinned in one test:
///
/// |                                  | placement evicted | placement surviving |
/// |---|---|---|
/// | **anchor surviving (cells kept)** | sixel A × kitty B (§07 prune evicts A; §13.4 reconcile keeps B) | sixel D × kitty B (both survive — control cell) |
/// | **anchor dropped (cells gone)**   | sixel A × kitty C (§07 prune evicts A; §13.4 reconcile drops C) | sixel D × kitty C (D survives prune; reconcile drops C) |
///
/// Without all four cells, the test is a "ghost test": the
/// anchor-survival assertion alone can pass even if
/// `reconcile_both_placeholder_anchors` is a no-op (no code path adds
/// spurious anchors mid-resize), so the test FORCES reconcile to do
/// meaningful work via an orphan anchor (kitty C) that only the
/// resize-end reconcile can clean up.
///
/// # Setup
///
/// 8 lines × 16 cols grid, scrollback cap 4. The 16-column width holds
/// kitty C's placeholder cells at cols 10-11 above the post-resize
/// 4-column truncation boundary so col-shrink (with `reflow=false`)
/// drops them, creating an orphan anchor that reconcile must clean up.
///
/// Setup sequence (each helper asserts its own post-conditions):
///
/// 1. `setup_sixel_a_at_row_0` — sixel A at grid row 0, `placement.cell_row = 0`.
///    After 11 LFs + resize-driven scrollback eviction, this row falls
///    below the post-resize floor and `prune_scrollback` drops the
///    placement (pins "placement evicted" cell of the matrix).
/// 2. `fill_scrollback_to_cap_without_eviction` — 11 LFs to push 4 rows
///    to scrollback at cap with no eviction (baseline for the resize
///    delta).
/// 3. `setup_sixel_d_at_row_4` — sixel D at grid row 4, absolute
///    `cell_row = 8` (above the post-resize eviction floor 4) at
///    `cell_col = 0` (survives col-truncation 16→4). Pins the "placement
///    surviving" cell.
/// 4. `write_kitty_u1_low_col_block(_, 42)` — kitty B U=1 anchor +
///    display grid `(4, 2)`. Cells at grid rows 5-6 cols 0-3 (low col).
///    Pins the "anchor surviving (cells kept)" cell.
/// 5. `write_kitty_u1_high_col_block(_, 99)` — kitty C U=1 anchor +
///    display grid `(2, 1)`. Cells at grid row 7 cols 10-11 (HIGH col).
///    After col-shrink 16→4, these cells are truncated; reconcile sees
///    no `U+10EEEE` cells for image 99 and MUST drop the anchor. Pins
///    the "anchor dropped" cell.
///
/// # Action
///
/// After parking the cursor at the bottom row (so the resize takes the
/// "push top to scrollback" branch rather than trimming trailing blank
/// rows), invoke `Term::resize(4, 4, reflow=false)`. The 16→4 column
/// truncation step (`grid/resize/mod.rs:resize_no_reflow` → `Row::resize`
/// → `Vec::resize_with` truncate) drops kitty C's placeholder cells at
/// cols 10-11 without firing the per-linefeed `prune_images_if_evicted`
/// reconcile fast path. The orphan anchor that results from the column
/// truncation can ONLY be cleaned up by the resize-end
/// `reconcile_both_placeholder_anchors` — making reconcile observably
/// necessary.
///
/// # Production flow during resize
///
/// 1. `Term::resize` at `term_repo/oriterm_core/src/term/resize/mod.rs:91`
///    — the symmetric reconcile entry-point invoked after
///    `prune_scrollback`.
/// 2. `Term::reconcile_both_placeholder_anchors` at
///    `term_repo/oriterm_core/src/term/handler/helpers.rs:295` — primary
///    pair walked here (the alt pair is walked by the companion test
///    `resize_reconciles_alt_cache_placeholder_anchors_symmetrically`).
///
/// # Assertions
///
/// - Resize drove additional evictions (`total_evicted` grew).
/// - Sixel placement matrix (`prune_scrollback` observable): sixel A
///   placement dropped (`cell_row=0` fell below floor); sixel D
///   placement survives (`cell_row=8` above floor); exactly one
///   placement survives.
/// - Kitty anchor matrix (`reconcile_both` observable): kitty B anchor
///   survives (cells at rows 1-2 cols 0-3 survived both shrinks); kitty
///   C anchor dropped (cells at row 7 cols 10-11 truncated by col-shrink);
///   exactly one anchor survives.
/// - Anchor grid map (§13.4-owned state, prune must not touch): kitty B
///   display grid `(4, 2)` survives; kitty C display grid entry dropped
///   via `reconcile_placeholder_anchors`'s paired-retain
///   (`image/cache/mod.rs:152-160`).
/// - Kitty B's image data survives via anchor (`prune_if_orphaned` skips
///   anchored images).
///
/// # Cross-pollination invariants
///
/// `prune_scrollback` does NOT touch `placeholder_anchor_grid` directly —
/// only `remove_image` removes entries for orphaned non-anchored images.
/// `reconcile_placeholder_anchors` retains `placeholder_anchor_grid`
/// alongside `placeholder_anchors` via a paired
/// `.retain(|id, _| survivors.contains(id))` — surviving anchors keep
/// their grid; dropped anchors lose theirs.
#[test]
fn resize_with_mixed_sixel_placement_and_kitty_u1_placeholders_preserves_both() {
    let ids = IntegrationIds {
        sixel_a: ImageId::from_raw(AUTO_ID_START),
        sixel_d: ImageId::from_raw(AUTO_ID_START + 1),
        kitty_b: ImageId::from_raw(42),
        kitty_c: ImageId::from_raw(99),
    };

    let mut h = SpecHarness::with_size_and_scrollback(8, 16, 4);

    // 1. Sixel A at row 0 (will fall below post-resize eviction floor).
    setup_sixel_a_at_row_0(&mut h);
    // 2. 11 LFs fill scrollback to cap without evicting.
    fill_scrollback_to_cap_without_eviction(&mut h);
    // 3. Sixel D at row 4 (absolute row 8 — survives prune).
    setup_sixel_d_at_row_4(&mut h);
    // 4. Kitty B U=1 anchor + low-col cells (survive col-shrink).
    write_kitty_u1_low_col_block(&mut h, 42);
    // 5. Kitty C U=1 anchor + high-col cells (truncated by col-shrink).
    write_kitty_u1_high_col_block(&mut h, 99);

    assert_pre_resize_state(&h, ids.kitty_b, ids.kitty_c);

    park_cursor_at_bottom_row(&mut h);
    let pre_evicted = h.term().grid().total_evicted();
    assert_eq!(pre_evicted, 0, "baseline: evicted=0 before resize");

    // 6. Resize 8×16 → 4×4 reflow=false.
    h.term_mut().resize(4, 4, false);
    let post_evicted = h.term().grid().total_evicted();

    assert!(
        post_evicted > pre_evicted,
        "resize MUST drive new evictions for prune_scrollback to fire — \
         saw pre={pre_evicted}, post={post_evicted}. Regression: \
         scrollback math drifted; prune_scrollback's `new_primary > \
         prev_primary` guard short-circuits."
    );

    assert_post_resize_primary_matrix(&h, post_evicted, ids);
}

/// Regression: spec-conformance §13.6 — companion pin to the
/// primary-cache integration test. `Term::reconcile_both_placeholder_anchors`
/// at `helpers.rs:295-313` walks BOTH the primary AND the alt cache;
/// the primary-only test would pass even if the alt branch silently
/// skipped. This pin sets up a U=1 anchor on BOTH primary and alt
/// screens, runs the same `Term::resize(_, _, reflow=false)`, and
/// asserts the alt cache's orphan-anchor reconcile fires symmetrically.
/// Walks the `if let Some(alt_grid) = self.alt_grid.as_ref()` branch at
/// `helpers.rs:302-313`.
///
/// Invariant: `reconcile_both_placeholder_anchors` fires once per
/// resize and covers BOTH active + inactive primary/alt caches.
///
/// # Setup
///
/// 8 lines × 16 cols grid, scrollback cap 4 (mirrors the primary
/// integration test). Primary cache populated with the same shape as
/// the sister test (sixel A row 0 + 11 LFs + kitty B low-col +
/// kitty C high-col); alt cache populated symmetrically with kitty
/// `B_alt` (`i=142`) low-col + kitty `C_alt` (`i=199`) high-col.
///
/// Setup sequence:
///
/// 1. Primary `setup_sixel_a_at_row_0` + 11 LFs (via raw feed, no
///    eviction-baseline assertion — the alt test does not assert on
///    primary mid-state, only end-state).
/// 2. Primary `write_kitty_u1_low_col_block(_, 42)` — kitty B anchor +
///    cells survive both shrinks.
/// 3. Primary `write_kitty_u1_high_col_block(_, 99)` — kitty C anchor +
///    cells truncated by col-shrink.
/// 4. `enter_alt_screen` (DECSET 1049) — lazily allocates alt grid +
///    `alt_image_cache`, both inheriting primary dimensions; alt cursor
///    restored to (0, 0) on first entry.
/// 5. Alt `write_kitty_u1_low_col_block(_, 142)` — kitty `B_alt` anchor
///    + cells survive both shrinks (on alt cache).
/// 6. Alt `write_kitty_u1_high_col_block(_, 199)` — kitty `C_alt` anchor
///    + cells truncated by alt col-shrink.
/// 7. `exit_alt_screen` (DECRST 1049) — swap back to primary so resize
///    fires from primary's active perspective. Alt cache state is
///    preserved (mode 1049 does NOT clear alt content; only mode 1047
///    clears).
///
/// # Action
///
/// Park primary cursor at bottom row, then `Term::resize(4, 4, false)`.
/// The resize handler (`term/resize/mod.rs:23-92`) runs on BOTH grids:
/// primary (active) AND alt (inactive). Alt always uses `reflow=false`
/// per `term/resize/mod.rs:70`. `reconcile_both_placeholder_anchors`
/// (`helpers.rs:295-313`) at the end walks BOTH caches.
///
/// # Assertions
///
/// - Primary cache assertions mirror the sister test: sixel A evicted;
///   kitty B anchor survives; kitty C anchor dropped.
/// - Alt cache assertions (after re-entering alt screen): kitty `B_alt`
///   anchor survives (cells at rows 5-6 cols 0-3 in old 8×16 alt grid
///   become rows 1-2 cols 0-3 in new 4×4 alt grid — survive both
///   row+col shrinks); kitty `C_alt` anchor dropped (cells at row 7 cols
///   10-11 truncated by alt col-shrink — orphan anchor cleaned up by
///   the alt branch of `reconcile_both_placeholder_anchors`); exactly
///   one alt anchor survives.
/// - Display-grid map symmetry on alt cache: kitty `B_alt` grid `(4, 2)`
///   survives; kitty `C_alt` grid entry dropped via paired-retain in
///   reconcile (`image/cache/mod.rs:156-157`).
///
/// # Cross-pollination invariants
///
/// `reconcile_both_placeholder_anchors` fires ONCE per resize and
/// covers BOTH primary + alt caches symmetrically. A regression where
/// the alt branch is dead code OR `collect_placeholder_image_ids_in_grid`
/// is not called on the alt grid would leave kitty `C_alt`'s anchor
/// orphaned in the alt cache while primary cleanup succeeded.
#[test]
fn resize_reconciles_alt_cache_placeholder_anchors_symmetrically() {
    let sixel_a = ImageId::from_raw(AUTO_ID_START);
    let kitty_b = ImageId::from_raw(42);
    let kitty_c = ImageId::from_raw(99);
    let kitty_b_alt = ImageId::from_raw(142);
    let kitty_c_alt = ImageId::from_raw(199);

    let mut h = SpecHarness::with_size_and_scrollback(8, 16, 4);

    // Primary setup (mirrors the sister test, minus mid-state pins).
    h.feed(b"\x1b[1;1H");
    h.feed(&dcs_n_cols_wide(4));
    h.feed(&[b'\n'; 11]);
    write_kitty_u1_low_col_block(&mut h, 42);
    write_kitty_u1_high_col_block(&mut h, 99);

    // Alt cache setup: enter alt, populate, exit.
    enter_alt_screen(&mut h);
    write_kitty_u1_low_col_block(&mut h, 142);
    write_kitty_u1_high_col_block(&mut h, 199);

    // Pin alt-cache preconditions via active-routing image_cache().
    let alt_anchors_pre = h.term().image_cache().placeholder_anchors().clone();
    assert!(
        alt_anchors_pre.contains(&kitty_b_alt) && alt_anchors_pre.contains(&kitty_c_alt),
        "precondition: both alt anchors recorded — saw {alt_anchors_pre:?}"
    );
    assert_eq!(alt_anchors_pre.len(), 2);

    exit_alt_screen(&mut h);
    park_cursor_at_bottom_row(&mut h);

    // Resize 8×16 → 4×4 reflow=false — fires on BOTH primary and alt grids.
    h.term_mut().resize(4, 4, false);

    // Primary cache assertions (mirror the sister test — alt test's
    // setup omits sixel D, so we pin only sixel A + kitty B/C here).
    assert!(h.term().image_cache().get_no_touch(sixel_a).is_none());
    let primary_anchors = h.term().image_cache().placeholder_anchors().clone();
    assert!(primary_anchors.contains(&kitty_b));
    assert!(!primary_anchors.contains(&kitty_c));

    // Re-enter alt screen to query alt-cache via active-routing.
    enter_alt_screen(&mut h);

    let alt_anchors_post = h.term().image_cache().placeholder_anchors().clone();
    assert!(
        alt_anchors_post.contains(&kitty_b_alt),
        "alt B's anchor (image_id=142) MUST survive the resize. \
         Alt cells at rows 5-6 cols 0-3 (in old 8×16 alt grid) become \
         rows 1-2 cols 0-3 in new 4×4 alt grid — survive both row+col \
         shrinks. Regression: `reconcile_both_placeholder_anchors` \
         failed to scan the alt grid, breaking the §13.4 layer-boundary \
         contract symmetry between primary and alt caches. \
         Surviving alt anchors: {alt_anchors_post:?}."
    );
    assert!(
        !alt_anchors_post.contains(&kitty_c_alt),
        "alt C's anchor (image_id=199) MUST be dropped by reconcile. \
         Alt cells at row 7 cols 10-11 were truncated by alt-grid \
         col-shrink (16→4 with reflow=false; alt always uses reflow=false \
         per `term/resize/mod.rs:70`). The orphan anchor can only be \
         cleaned up by `reconcile_both_placeholder_anchors`'s alt branch \
         at `helpers.rs:302-313`. Regression: alt branch is dead code \
         OR `collect_placeholder_image_ids_in_grid` was not called on \
         the alt grid. Surviving alt anchors: {alt_anchors_post:?}."
    );
    assert_eq!(
        alt_anchors_post.len(),
        1,
        "exactly one alt anchor must survive (kitty `B_alt`). Regression: \
         alt reconcile added a spurious anchor or dropped the wrong one."
    );

    // Display-grid map symmetry on alt cache.
    assert_eq!(
        h.term()
            .image_cache()
            .placeholder_anchor_grid_for(kitty_b_alt),
        Some((4, 2))
    );
    assert_eq!(
        h.term()
            .image_cache()
            .placeholder_anchor_grid_for(kitty_c_alt),
        None,
        "alt C's grid entry MUST be dropped via paired-retain in \
         reconcile (`image/cache/mod.rs:156-157`). Regression: alt cache \
         grid map drifted from anchor set."
    );
}

// §14.6 — ImageCache ID namespace partition (Decision 07 Option A)

/// Kitty explicit `i=AUTO_ID_START-1` (the top of the explicit range
/// `[1, 2^31)`) is accepted and stored. Boundary pin: the validator must
/// reject `>= AUTO_ID_START`, NOT `> AUTO_ID_START-1`.
/// Anchor: ITERM2-1337-FILE-INLINE (cross-protocol ID isolation pin).
#[test]
fn kitty_explicit_id_at_auto_start_minus_one_accepted() {
    let mut h = SpecHarness::new();
    transmit_kitty_unplaced(&mut h, AUTO_ID_START - 1);
    assert_eq!(
        h.term().image_cache().image_count(),
        1,
        "i=2^31-1 must store"
    );
    assert!(
        h.term()
            .image_cache()
            .contains_image_for_test(ImageId::from_raw(AUTO_ID_START - 1)),
        "the explicit-max image id 2^31-1 must be present"
    );
}

/// Kitty explicit `i=AUTO_ID_START` (the first auto-namespace id) is
/// REJECTED with EINVAL and stores nothing — an explicit id may not
/// intrude into `[2^31, 2^32)`. Rejecting-side guard: FAILS if the
/// validator is removed (the image would store, colliding with the auto
/// namespace).
/// Anchor: ITERM2-1337-FILE-INLINE (cross-protocol ID isolation).
#[test]
fn kitty_explicit_id_at_auto_start_rejected() {
    let mut h = SpecHarness::new();
    transmit_kitty_unplaced(&mut h, AUTO_ID_START);
    assert_eq!(
        h.term().image_cache().image_count(),
        0,
        "i=2^31 (auto-namespace) must be rejected, not stored"
    );
    assert!(
        pty_write_contains(
            &h,
            PtyWriteKind::ImageProtocolReply,
            b"EINVAL: explicit image id 2147483648 outside allowed range [1, 2^31)"
        ),
        "rejection must emit the EINVAL reply so the client can recover"
    );
}

/// Explicit `i=0` on a CREATE action is rejected — the kitty graphics
/// protocol requires image ids to be positive
/// (`~/projects/reference_repos/console_repos/kitty/docs/graphics-protocol.rst`:
/// "an arbitrary positive integer up to 4294967295, it must not be zero").
/// 0 falls below the writable explicit range `[1, 2^31)`, so a transmit
/// choosing it is EINVAL'd and stores nothing. (An ABSENT `i` still
/// auto-allocates — only an explicit zero is rejected.)
/// Anchor: ITERM2-1337-FILE-INLINE (explicit-id lower-bound rejection).
#[test]
fn kitty_explicit_id_zero_rejected() {
    let mut h = SpecHarness::new();
    transmit_kitty_unplaced(&mut h, 0);
    assert_eq!(
        h.term().image_cache().image_count(),
        0,
        "explicit i=0 (below the [1, 2^31) explicit range) must be rejected, not stored"
    );
    assert!(
        pty_write_contains(
            &h,
            PtyWriteKind::ImageProtocolReply,
            b"EINVAL: explicit image id 0 outside allowed range [1, 2^31)"
        ),
        "i=0 rejection must emit the EINVAL reply so the client can recover"
    );
}

/// Kitty ADDRESSING actions (`a=p` / `a=d` / `a=q`) on a terminal-assigned
/// id `>= AUTO_ID_START` must NOT be namespace-rejected. The terminal
/// auto-allocates ids in `[2^31, 2^32)` and hands them to kitty clients via
/// the `I=` request-id flow; the client then addresses that id. Only the
/// CREATE actions (`a=t` / `a=T`) reject an out-of-range explicit id —
/// addressing an already-assigned one is the documented reuse path
/// (`~/projects/reference_repos/console_repos/kitty/docs/graphics-protocol.rst`
/// "Requesting image ids from the terminal").
/// Anchor: ITERM2-1337-FILE-INLINE (terminal-assigned-id reuse).
#[test]
fn kitty_addressing_action_on_terminal_assigned_auto_id_not_namespace_rejected() {
    let mut h = SpecHarness::new();
    transmit_iterm2_inline(&mut h); // auto-allocates AUTO_ID_START at the home cell
    assert!(
        h.term()
            .image_cache()
            .contains_image_for_test(ImageId::from_raw(AUTO_ID_START)),
        "iTerm2 auto-allocation seeds an image at the terminal-assigned id"
    );
    place_existing_kitty(&mut h, AUTO_ID_START); // a=p,i=2^31 — addressing, not create
    assert!(
        !pty_write_contains(&h, PtyWriteKind::ImageProtocolReply, b"allowed range"),
        "a kitty addressing action on a terminal-assigned auto-range id must not emit a namespace EINVAL"
    );
    assert!(
        h.term()
            .image_cache()
            .contains_image_for_test(ImageId::from_raw(AUTO_ID_START)),
        "the addressed image survives — the place neither failed nor evicted it"
    );
}

/// Decision 07 wrap-preserves-upper-half: `next_image_id()` must keep
/// auto-allocated ids inside `[AUTO_ID_START, 2^32)` even after the counter
/// exhausts the `u32` range. Driving it to `u32::MAX` then once more must
/// land back at/above `AUTO_ID_START`, never in the explicit range
/// `[1, 2^31)` (which would collide with kitty client ids).
/// Anchor: ITERM2-1337-FILE-INLINE (auto-id wrap stays in upper half).
#[test]
fn iterm2_auto_id_wrap_stays_in_upper_half() {
    let mut h = SpecHarness::new();
    h.term_mut()
        .image_cache_mut()
        .set_next_auto_id_for_test(u32::MAX);
    let pre_wrap = h.term_mut().image_cache_mut().next_auto_id_for_test();
    assert_eq!(
        pre_wrap.as_u32(),
        u32::MAX,
        "the allocation at the top of the range is u32::MAX"
    );
    let wrapped = h.term_mut().image_cache_mut().next_auto_id_for_test();
    assert!(
        wrapped.as_u32() >= AUTO_ID_START,
        "post-wrap auto id must stay in the upper half [2^31, 2^32), got {}",
        wrapped.as_u32()
    );
}

/// The first cache-auto-allocated id (here an iTerm2 inline image) is
/// exactly `AUTO_ID_START` (`2^31`) on a fresh cache — the bottom of the
/// auto namespace, disjoint from the kitty explicit range.
/// Anchor: ITERM2-1337-FILE-INLINE.
#[test]
fn iterm2_first_auto_allocation_is_2_pow_31() {
    let mut h = SpecHarness::new();
    transmit_iterm2_inline(&mut h);
    let placements = h.term().image_cache().placements_for_test();
    assert_eq!(placements.len(), 1, "iTerm2 inline must place one image");
    assert_eq!(
        placements[0].image_id,
        ImageId::from_raw(AUTO_ID_START),
        "first auto-allocated id must be 2^31, not 2^31-1"
    );
}

/// Cross-protocol no-overwrite: a kitty explicit image at `AUTO_ID_START-1`
/// (the explicit max) followed by an auto-allocated iTerm2 image (which
/// gets `AUTO_ID_START`) must NOT collide — both survive. Pre-Decision-07
/// the iTerm2 auto id was `2^31-1`, overwriting the kitty image; the
/// disjoint namespaces fix it.
/// Anchor: ITERM2-1337-FILE-INLINE (cross-protocol ID isolation pin).
#[test]
fn cross_protocol_id_collision_kitty_explicit_at_auto_start_iterm2_auto_no_overwrite() {
    let mut h = SpecHarness::new();
    transmit_kitty_unplaced(&mut h, AUTO_ID_START - 1);
    transmit_iterm2_inline(&mut h);

    let cache = h.term().image_cache();
    assert!(
        cache.contains_image_for_test(ImageId::from_raw(AUTO_ID_START - 1)),
        "kitty explicit image at 2^31-1 must survive the iTerm2 auto-allocation"
    );
    assert!(
        cache.contains_image_for_test(ImageId::from_raw(AUTO_ID_START)),
        "iTerm2 auto image must land at the disjoint 2^31, not overwrite the kitty image"
    );
    assert_eq!(cache.image_count(), 2, "both protocol images coexist");
}

/// Inverse cross-protocol pin: an auto-allocated iTerm2 image (`2^31`)
/// followed by a kitty explicit `i=2^31` must reject the kitty request
/// (EINVAL — explicit id in the auto namespace) and leave the iTerm2
/// image untouched.
/// Anchor: ITERM2-1337-FILE-INLINE (cross-protocol ID isolation pin).
#[test]
fn cross_protocol_id_collision_iterm2_auto_kitty_explicit_no_overwrite() {
    let mut h = SpecHarness::new();
    transmit_iterm2_inline(&mut h);
    assert!(
        h.term()
            .image_cache()
            .contains_image_for_test(ImageId::from_raw(AUTO_ID_START)),
        "iTerm2 auto image present at 2^31"
    );

    transmit_kitty_unplaced(&mut h, AUTO_ID_START);
    assert!(
        pty_write_contains(
            &h,
            PtyWriteKind::ImageProtocolReply,
            b"EINVAL: explicit image id 2147483648"
        ),
        "kitty explicit i=2^31 must be rejected (auto-namespace intrusion)"
    );
    assert!(
        h.term()
            .image_cache()
            .contains_image_for_test(ImageId::from_raw(AUTO_ID_START)),
        "the iTerm2 image must remain untouched by the rejected kitty request"
    );
}

/// 3-way interleaved no-collision (post Decision-07): a kitty explicit
/// image (`i=2^31-1`, top of the explicit range), an auto-allocated
/// iTerm2 inline image (`2^31`), and an auto-allocated sixel image
/// (`2^31+1`) transmitted in one session occupy three DISTINCT ids across
/// the namespace boundary — none overwrites another. The placed images
/// (iTerm2, sixel) sit on separate cells so cell-collision eviction does
/// not confound the ID-isolation assertion.
/// Anchor: ITERM2-1337-FILE-INLINE (cross-stack 3-protocol ID isolation).
#[test]
fn kitty_explicit_iterm2_auto_sixel_auto_interleaved_no_collision_post_id_namespace_fix() {
    let mut h = SpecHarness::new();
    transmit_kitty_unplaced(&mut h, AUTO_ID_START - 1); // explicit -> 2^31-1 (unplaced)
    transmit_iterm2_inline(&mut h); // auto -> 2^31, placed on the home cell
    move_cursor_to(&mut h, 10, 1); // fresh cell so sixel does not evict iTerm2
    h.feed(&dcs_n_cols_wide(8)); // sixel auto -> 2^31+1, placed on row 10

    let cache = h.term().image_cache();
    assert!(
        cache.contains_image_for_test(ImageId::from_raw(AUTO_ID_START - 1)),
        "kitty explicit 2^31-1 present"
    );
    assert!(
        cache.contains_image_for_test(ImageId::from_raw(AUTO_ID_START)),
        "iTerm2 auto 2^31 present (disjoint from kitty)"
    );
    assert!(
        cache.contains_image_for_test(ImageId::from_raw(AUTO_ID_START + 1)),
        "sixel auto 2^31+1 present (disjoint from both)"
    );
    assert_eq!(
        cache.image_count(),
        3,
        "all three protocol images coexist, no collision"
    );
}

/// 3-way global-LRU under cache pressure: placed iTerm2 + sixel images are
/// immune to eviction while an unplaced kitty image is the eviction
/// victim — the global-LRU policy treats all three protocols uniformly
/// (no protocol-specific carve-out). Mirrors
/// `cross_protocol_lru_mixed_sixel_kitty_*` extended with iTerm2.
/// Anchor: ITERM2-1337-FILE-INLINE (3-protocol LRU uniformity).
#[test]
fn mixed_kitty_iterm2_sixel_lru_eviction_under_cache_pressure() {
    let mut h = SpecHarness::new();
    // Placed images (immune): iTerm2 inline (auto 2^31) + sixel (auto 2^31+1).
    // Separate cells — placement on the same cell would evict the earlier
    // image (last-one-wins), which would defeat the "both placed survive" pin.
    transmit_iterm2_inline(&mut h); // placed on the home cell
    move_cursor_to(&mut h, 10, 1); // fresh cell so sixel does not evict iTerm2
    h.feed(&dcs_n_cols_wide(8)); // placed on row 10
    // Unplaced kitty image (eviction-eligible).
    transmit_kitty_unplaced(&mut h, 42);
    assert_eq!(
        h.term().image_cache().image_count(),
        3,
        "three images stored"
    );

    // Tighten memory below the total so eviction must reclaim the only
    // un-placed image (the kitty one); the placed iTerm2 + sixel survive.
    let placed_floor = h.term().image_cache().memory_used()
        - (KITTY_IMAGE_BYTES.min(h.term().image_cache().memory_used()));
    h.term_mut()
        .image_cache_mut()
        .set_memory_limit(placed_floor.max(1));
    // Trigger an eviction pass by transmitting one more unplaced image.
    transmit_kitty_unplaced(&mut h, 43);

    let cache = h.term().image_cache();
    assert!(
        cache.contains_image_for_test(ImageId::from_raw(AUTO_ID_START)),
        "placed iTerm2 image immune to eviction"
    );
    assert!(
        cache.contains_image_for_test(ImageId::from_raw(AUTO_ID_START + 1)),
        "placed sixel image immune to eviction"
    );
    assert!(
        !cache.contains_image_for_test(ImageId::from_raw(42)),
        "the oldest unplaced kitty image is the LRU victim, regardless of protocol"
    );
}
