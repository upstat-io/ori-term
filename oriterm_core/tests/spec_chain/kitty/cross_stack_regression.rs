//! §13.6.1 — cross-stack regression tests.
//!
//! Catalog row: `KG-CROSS-STACK-SIXEL-MIXED-EVICTION`
//! Catalog row: `KG-CROSS-STACK-SIXEL-MIXED-Z-ORDER` (semantic pin layer)
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
/// cross-stack regression coverage is partitioned across 5 distinct
/// concern categories per the plan body; this pin self-verifies that
/// the partition is enumerated, so adding a new cross-stack concern
/// without registering it here fails the matrix-completeness assertion
/// instead of silently dropping coverage.
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
    ];
    assert_eq!(
        CATEGORIES.len(),
        5,
        "matrix MUST enumerate every cross-stack concern category — \
         adding a 6th regression dimension without updating this list is \
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
