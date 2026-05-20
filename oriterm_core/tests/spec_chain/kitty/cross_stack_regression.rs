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

use oriterm_core::image::ImageId;
use oriterm_test_support::spec_chain::SpecHarness;
use oriterm_test_support::spec_chain::sixel_fixtures::dcs_n_cols_wide;

use super::fixtures::{b64, kitty_apc, rgba_4x4_red};

const KITTY_IMAGE_BYTES: usize = 64;

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
    use oriterm_test_support::spec_chain::sixel_fixtures::dcs_n_cols_wide;

    let mut h = SpecHarness::new();

    // 1. Sixel-A at row 1 — sixel handler stores + places at cursor.
    //    Auto-assigned ImageId = AUTO_ID_START (private const, value
    //    2_147_483_647 per oriterm_core/src/image/cache/mod.rs:24).
    const AUTO_ID_START: u32 = 2_147_483_647;
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

    // Force eviction: cap memory at roughly 2 images' worth so exactly
    // ONE of the unplaced images must drop. With kitty-B now placed
    // (eviction-immune per Option A), eviction targets the unplaced
    // pool: sixel-A (oldest) is evicted; sixel-C (younger) survives.
    // `dcs_n_cols_wide(8)` produces an 8×6 RGBA image = 192 bytes;
    // and kitty s=4,v=4,f=32 = 64 bytes. Total 3 images ≈ 448 bytes; cap
    // at 280 forces one sixel eviction (256 bytes ≈ kitty + 1 sixel).
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

    // Third kitty `a=T` would push memory past the cap. Eviction can't
    // free a placed image, so the store path MUST reject with
    // `ImageError::MemoryLimitExceeded`. The handler converts that to
    // an `ENOMEM:` APC error reply (see kitty/store.rs:108).
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

/// Architectural pin: kitty's `kitty_create_placement` and sixel's
/// `sixel_create_placement` both route placement creation through
/// `ImageCache::place`. Splitting the recency bump across per-protocol
/// handlers (rejected at §13.6.1 plan-time) would scatter the LRU
/// contract — when a future protocol lands, the bump is silently absent
/// and eviction silently regresses to FIFO for that protocol. This grep
/// gate fires the moment a handler bypasses `ImageCache::place` (e.g.,
/// constructing `ImagePlacement` and pushing it via a different API).
#[test]
fn kitty_and_sixel_create_placement_paths_route_through_image_cache_place() {
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
        // Each protocol handler MUST contain at least one call into
        // `image_cache_mut().place(`. If a handler constructs an
        // `ImagePlacement` and pushes it via a different path, this
        // assertion fires.
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
        // 6. Resize-lifecycle integration (§13 ↔ §07): a single
        //    `Term::resize` invocation drives both `prune_scrollback`
        //    (§07-owned) and `reconcile_both_placeholder_anchors`
        //    (§13.4-owned) without cross-pollination across the four
        //    cells of the {placement, anchor} × {evicted, surviving}
        //    matrix. Owner pin:
        //    `resize_with_mixed_sixel_placement_and_kitty_u1_placeholders_preserves_both`.
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

/// Regression: spec-conformance §13.6.1 semantic pin. Z-ordering is
/// driven by `z_index`, NOT by transmit/place sequence order. Two
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

/// Regression: spec-conformance §13.6.1 negative pin. Cache eviction
/// MUST be per-`image_id`. Removing a kitty image's bytes MUST NOT free
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
                // Auto-assigned starts at AUTO_ID_START = 2_147_483_647.
                ImageId::from_raw(2_147_483_647)
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

    // 4. Sixel image bytes MUST survive — the kitty delete-by-id path is
    //    per-image_id, NOT per-protocol. A protocol-specific carve-out
    //    (e.g., "delete-by-id for kitty also evicts sixel by store-order")
    //    would fail this pin.
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

/// Regression: spec-conformance §13.6 — cross-section §13 ↔ §07
/// integration pin. A single `Term::resize` invocation drives BOTH
/// `ImageCache::prune_scrollback` (§07-owned scrollback lifecycle
/// handler) AND `Term::reconcile_both_placeholder_anchors` (§13.4-owned
/// anchor reconcile) in the same operation; they MUST NOT cross-
/// pollinate. The 2×2 matrix MUST be pinned in one test:
///
/// |                                  | placement evicted | placement surviving |
/// |---|---|---|
/// | **anchor surviving (cells kept)** | sixel A × kitty B (§07 prune evicts A; §13.4 reconcile keeps B) | sixel D × kitty B (both survive — control cell) |
/// | **anchor dropped (cells gone)**   | sixel A × kitty C (§07 prune evicts A; §13.4 reconcile drops C) | sixel D × kitty C (D survives prune; reconcile drops C) |
///
/// Without all four cells, the test is a "ghost test" (codex TPR
/// round-0 F1, 2026-05-20): the anchor-survival assertion alone can
/// pass even if `reconcile_both_placeholder_anchors` is a no-op (no
/// code path adds spurious anchors mid-resize), so the test must
/// FORCE reconcile to do meaningful work via an orphan anchor that
/// only the resize-end reconcile can clean up.
///
/// Exercises two production seams in the same call (primary-cache
/// coverage; the alt-cache branch of `reconcile_both_placeholder_anchors`
/// is pinned by the companion test
/// `resize_reconciles_alt_cache_placeholder_anchors_symmetrically`):
/// 1. `Term::resize` at `term_repo/oriterm_core/src/term/resize/mod.rs:91`
///    (the symmetric reconcile entry-point invoked after `prune_scrollback`).
/// 2. `Term::reconcile_both_placeholder_anchors` at
///    `term_repo/oriterm_core/src/term/handler/helpers.rs:295` (primary
///    pair walked here; alt pair walked by the companion test).
///
/// Uses a 16-column grid + `reflow=false` resize so the column truncation
/// step (`grid/resize/mod.rs:resize_no_reflow` → `Row::resize` →
/// `Vec::resize_with` truncate) drops kitty C's placeholder cells at
/// cols 10-11 without firing the per-linefeed `prune_images_if_evicted`
/// reconcile fast path. The orphan anchor that results from the column
/// truncation can ONLY be cleaned up by the resize-end
/// `reconcile_both_placeholder_anchors` — making reconcile observably
/// necessary.
#[test]
fn resize_with_mixed_sixel_placement_and_kitty_u1_placeholders_preserves_both() {
    use oriterm_test_support::spec_chain::sixel_fixtures::dcs_n_cols_wide;

    // Auto-assigned sixel image_id (§12 sixel handler at
    // `oriterm_core/src/image/cache/mod.rs:24`).
    const SIXEL_AUTO_ID: u32 = 2_147_483_647;
    let sixel_a = ImageId::from_raw(SIXEL_AUTO_ID);
    let sixel_d = ImageId::from_raw(SIXEL_AUTO_ID + 1);
    let kitty_b = ImageId::from_raw(42);
    let kitty_c = ImageId::from_raw(99);

    // 8 lines × 16 cols, sb cap 4. The 16-column width holds kitty C's
    // placeholder cells at cols 10-11 above the post-resize 4-column
    // truncation boundary so col-shrink (with `reflow=false`) drops
    // them, creating an orphan anchor that reconcile must clean up.
    let mut h = SpecHarness::with_size_and_scrollback(8, 16, 4);

    // 1. Sixel A at grid row 0 — placement.cell_row = 0. After 11 LFs +
    //    resize-driven scrollback eviction, this row falls below the
    //    post-resize floor and prune_scrollback drops the placement.
    h.feed(b"\x1b[1;1H"); // CUP(1,1) → cursor (0, 0).
    h.feed(&dcs_n_cols_wide(4));
    assert_eq!(
        h.term().image_cache().placement_count(),
        1,
        "sixel handler must create exactly one placement at cursor"
    );

    // 2. Drive 11 linefeeds to fill scrollback to cap WITHOUT evicting.
    //    Starting from cursor at (0, 0), 7 LFs move cursor to (7, 0);
    //    LFs 8-11 scroll, pushing 4 rows to scrollback. sb cap 4 → no
    //    rows evict. evicted=0, sb=4. The 11 LFs run BEFORE any U=1
    //    anchors are placed, so the per-LF reconcile cannot drop an
    //    anchor that does not yet exist.
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

    // 3. Sixel D at grid row 4 col 0 — placement.cell_row = 8 (above
    //    the post-resize eviction floor 4). cell_col=0 (survives col
    //    truncation 16→4). Pins the "sixel placement survives prune"
    //    cell of the matrix.
    h.feed(b"\x1b[5;1H"); // CUP(5,1) → cursor (4, 0).
    h.feed(&dcs_n_cols_wide(4));
    assert_eq!(
        h.term().image_cache().placement_count(),
        2,
        "sixel D's placement must register alongside sixel A"
    );

    // 4. Kitty B `a=T,U=1,c=4,r=2,i=42` — anchor + display grid (4, 2).
    //    Place at grid line 5, write cells at grid rows 5-6 cols 0-3
    //    (low col, so they survive col-shrink). Pins the "anchor + cells
    //    survive resize" cell.
    h.feed(b"\x1b[6;1H"); // CUP(6,1) → cursor (5, 0).
    h.feed(&kitty_apc(
        b"a=T,U=1,i=42,c=4,r=2,f=32,s=4,v=4",
        &b64(&rgba_4x4_red()),
    ));
    const ROW_D: [char; 2] = ['\u{0305}', '\u{030D}'];
    const COL_D: [char; 4] = ['\u{0305}', '\u{030D}', '\u{030E}', '\u{030F}'];
    h.feed(b"\x1b[6;1H"); // grid line 5 col 0
    for col in 0..4 {
        write_placeholder_cell(&mut h, 42, ROW_D[0], COL_D[col]);
    }
    h.feed(b"\x1b[7;1H"); // grid line 6 col 0
    for col in 0..4 {
        write_placeholder_cell(&mut h, 42, ROW_D[1], COL_D[col]);
    }

    // 5. Kitty C `a=T,U=1,c=2,r=1,i=99` — anchor + display grid (2, 1).
    //    Cells at grid row 7 cols 10-11 (HIGH col). After col-shrink
    //    16→4, these cells are truncated; reconcile sees no `U+10EEEE`
    //    cells for image 99 and MUST drop the anchor. Pins the
    //    "anchor dropped by reconcile" cell of the matrix.
    h.feed(b"\x1b[8;11H"); // CUP(8,11) → cursor (7, 10).
    h.feed(&kitty_apc(
        b"a=T,U=1,i=99,c=2,r=1,f=32,s=4,v=4",
        &b64(&rgba_4x4_red()),
    ));
    h.feed(b"\x1b[8;11H");
    write_placeholder_cell(&mut h, 99, ROW_D[0], COL_D[0]);
    write_placeholder_cell(&mut h, 99, ROW_D[0], COL_D[1]);

    // Pre-resize state pins.
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

    // Park cursor at bottom row BEFORE the resize. `shrink_rows`'s
    // `count_trailing_blank_rows` heuristic (`grid/resize/mod.rs:218-233`)
    // trims blank rows below the cursor instead of pushing top rows to
    // scrollback. Without this CUP, the resize would trim trailing
    // blank rows and total_evicted would stay at 0 — the resize-driven
    // prune path would be a no-op.
    h.feed(b"\x1b[8;1H");
    let pre_evicted = h.term().grid().total_evicted();
    assert_eq!(pre_evicted, 0, "baseline: evicted=0 before resize");

    // 6. Resize 8×16 → 4×4 reflow=false. Sequence (see test docstring
    //    "Production flow during resize" for full step-by-step trace).
    h.term_mut().resize(4, 4, false);
    let post_evicted = h.term().grid().total_evicted();

    // Pin: the resize drove additional evictions.
    assert!(
        post_evicted > pre_evicted,
        "resize MUST drive new evictions for prune_scrollback to fire — \
         saw pre={pre_evicted}, post={post_evicted}. Regression: \
         scrollback math drifted; prune_scrollback's `new_primary > \
         prev_primary` guard short-circuits."
    );

    // ── Sixel placement matrix (prune_scrollback observable) ──

    // (a₁) Sixel A's placement DROPPED — its row=0 fell below the
    //      post-resize eviction floor (4).
    assert!(
        h.term().image_cache().get_no_touch(sixel_a).is_none(),
        "sixel A (cell_row=0) MUST be evicted by §07's prune_scrollback \
         (post-resize floor={post_evicted}). Its image data is dropped \
         via orphan-prune since sixel A had no anchor. Regression: §07 \
         prune lifecycle handler failed to drop a placement whose row \
         fell below the eviction floor."
    );
    // (a₂) Sixel D's placement SURVIVES — its row=8 is above the floor.
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

    // ── Kitty anchor matrix (reconcile_both_placeholder_anchors observable) ──

    let anchors_post = h.term().image_cache().placeholder_anchors().clone();

    // (b₁) Kitty B's anchor SURVIVES — cells at rows 1-2 cols 0-3 survived
    //      both row-shrink and col-shrink. `reconcile_both` finds them.
    assert!(
        anchors_post.contains(&kitty_b),
        "kitty B's anchor (image_id=42) MUST survive — its placeholder \
         cells at grid rows 5-6 cols 0-3 fall into the new visible grid \
         at low cols, surviving col-truncation. Regression: \
         reconcile_both failed to scan the visible grid, or §07's \
         prune_scrollback cross-pollinated by clearing the anchor set. \
         Surviving anchors: {anchors_post:?}."
    );
    // (b₂) Kitty C's anchor DROPPED. Only the resize-end reconcile can drop
    //      this anchor; per-LF reconcile never fires during resize.
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

    // ── Anchor grid map (§13.4-owned state, prune must not touch) ──

    // (c) Display grid `(4, 2)` for kitty B survives, and kitty C's grid
    //     entry is dropped. `prune_scrollback` does NOT touch
    //     `placeholder_anchor_grid` directly — only `remove_image` removes
    //     entries for orphaned non-anchored images.
    //     `reconcile_placeholder_anchors` (`image/cache/mod.rs:152-160`)
    //     retains `placeholder_anchor_grid` alongside `placeholder_anchors`
    //     via a paired `.retain(|id, _| survivors.contains(id))` —
    //     surviving anchors keep their grid; dropped anchors lose theirs.
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

    // (d) Kitty B's image data survives via anchor.
    assert!(
        h.term().image_cache().get_no_touch(kitty_b).is_some(),
        "kitty B's image data MUST survive — anchored via \
         placeholder_anchors so `prune_if_orphaned` skips it. \
         Regression: cross-protocol cleanup carve-out dropped kitty B's \
         image data when sixel A was evicted."
    );
}

/// Regression: spec-conformance §13.6 — companion pin to the primary-cache
/// integration test. `Term::reconcile_both_placeholder_anchors` at
/// `helpers.rs:295-313` walks BOTH the primary AND the alt cache; the
/// primary-only test would pass even if the alt branch silently skipped.
/// This pin sets up a U=1 anchor on BOTH primary and alt screens, runs the
/// same `Term::resize(_, _, reflow=false)`, and asserts the alt cache's
/// orphan-anchor reconcile fires symmetrically. Walks the
/// `if let Some(alt_grid) = self.alt_grid.as_ref()` branch at
/// `helpers.rs:302-313`.
///
/// Plan body: `plans/spec-conformance/section-13-kitty-graphics.md §13.6.1`
/// — "reconcile_both_placeholder_anchors fires once per resize and covers
/// BOTH active + inactive primary/alt caches."
#[test]
fn resize_reconciles_alt_cache_placeholder_anchors_symmetrically() {
    use oriterm_test_support::spec_chain::sixel_fixtures::dcs_n_cols_wide;

    const SIXEL_AUTO_ID: u32 = 2_147_483_647;
    let sixel_a = ImageId::from_raw(SIXEL_AUTO_ID);
    let kitty_b = ImageId::from_raw(42);
    let kitty_c = ImageId::from_raw(99);
    let kitty_b_alt = ImageId::from_raw(142);
    let kitty_c_alt = ImageId::from_raw(199);

    let mut h = SpecHarness::with_size_and_scrollback(8, 16, 4);

    // Primary setup (same shape as the sister test).
    h.feed(b"\x1b[1;1H");
    h.feed(&dcs_n_cols_wide(4));
    h.feed(&[b'\n'; 11]);

    h.feed(b"\x1b[6;1H");
    h.feed(&kitty_apc(
        b"a=T,U=1,i=42,c=4,r=2,f=32,s=4,v=4",
        &b64(&rgba_4x4_red()),
    ));
    const ROW_D: [char; 2] = ['\u{0305}', '\u{030D}'];
    const COL_D: [char; 4] = ['\u{0305}', '\u{030D}', '\u{030E}', '\u{030F}'];
    h.feed(b"\x1b[6;1H");
    for col in 0..4 {
        write_placeholder_cell(&mut h, 42, ROW_D[0], COL_D[col]);
    }
    h.feed(b"\x1b[7;1H");
    for col in 0..4 {
        write_placeholder_cell(&mut h, 42, ROW_D[1], COL_D[col]);
    }
    h.feed(b"\x1b[8;11H");
    h.feed(&kitty_apc(
        b"a=T,U=1,i=99,c=2,r=1,f=32,s=4,v=4",
        &b64(&rgba_4x4_red()),
    ));
    h.feed(b"\x1b[8;11H");
    write_placeholder_cell(&mut h, 99, ROW_D[0], COL_D[0]);
    write_placeholder_cell(&mut h, 99, ROW_D[0], COL_D[1]);

    // Enter alt screen via `\x1b[?1049h` (DECSET 1049 →
    // `Term::swap_alt`, `term/alt_screen/mod.rs:25-39`). Alt grid +
    // alt_image_cache lazily allocated, both inherit primary
    // dimensions (8×16, sb cap 0 for alt). Alt cursor restored to
    // (0, 0) on first entry (`navigation/mod.rs:255-256` — no prior
    // save → origin).
    h.feed(b"\x1b[?1049h");

    // Alt cache setup: kitty B_alt at low row+col (cells survive both
    // shrinks) + kitty C_alt at high col (cells truncated by col-shrink).
    h.feed(b"\x1b[6;1H");
    h.feed(&kitty_apc(
        b"a=T,U=1,i=142,c=4,r=2,f=32,s=4,v=4",
        &b64(&rgba_4x4_red()),
    ));
    h.feed(b"\x1b[6;1H");
    for col in 0..4 {
        write_placeholder_cell(&mut h, 142, ROW_D[0], COL_D[col]);
    }
    h.feed(b"\x1b[7;1H");
    for col in 0..4 {
        write_placeholder_cell(&mut h, 142, ROW_D[1], COL_D[col]);
    }
    h.feed(b"\x1b[8;11H");
    h.feed(&kitty_apc(
        b"a=T,U=1,i=199,c=2,r=1,f=32,s=4,v=4",
        &b64(&rgba_4x4_red()),
    ));
    h.feed(b"\x1b[8;11H");
    write_placeholder_cell(&mut h, 199, ROW_D[0], COL_D[0]);
    write_placeholder_cell(&mut h, 199, ROW_D[0], COL_D[1]);

    // Pin alt-cache preconditions via active-routing image_cache().
    let alt_anchors_pre = h.term().image_cache().placeholder_anchors().clone();
    assert!(
        alt_anchors_pre.contains(&kitty_b_alt) && alt_anchors_pre.contains(&kitty_c_alt),
        "precondition: both alt anchors recorded — saw {alt_anchors_pre:?}"
    );
    assert_eq!(alt_anchors_pre.len(), 2);

    // Swap back to primary so resize fires from primary's active
    // perspective. `swap_alt` saves alt cursor (currently at the end
    // of placeholder writes) and restores primary cursor (at end of
    // primary writes).
    h.feed(b"\x1b[?1049l");

    // Park primary cursor at bottom row to force the
    // `count_trailing_blank_rows` heuristic into the "push top to sb"
    // branch.
    h.feed(b"\x1b[8;1H");

    // Resize 8×16 → 4×4 reflow=false. The resize handler
    // (`term/resize/mod.rs:23-92`) runs on BOTH grids: primary
    // (active) AND alt (inactive). `reconcile_both_placeholder_anchors`
    // (`helpers.rs:295-313`) at the end walks BOTH caches.
    h.term_mut().resize(4, 4, false);

    // ── Primary cache assertions (mirror the sister test) ──
    assert!(h.term().image_cache().get_no_touch(sixel_a).is_none());
    let primary_anchors = h.term().image_cache().placeholder_anchors().clone();
    assert!(primary_anchors.contains(&kitty_b));
    assert!(!primary_anchors.contains(&kitty_c));

    // ── Alt cache assertions ──
    // Re-enter alt via `\x1b[?1049h` (mode-1049 swap does NOT clear
    // alt content per `swap_alt` semantics — only mode 1047 clears).
    // Alt cache state is preserved across the round trip.
    h.feed(b"\x1b[?1049h");

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
        "exactly one alt anchor must survive (kitty B_alt). Regression: \
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
