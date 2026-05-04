use std::sync::Arc;

use super::*;
use crate::font::collection::loading::{FontBytes, FontData};
use crate::font::collection::{FontSet, size_key};
use crate::font::discovery::TEST_EMOJI_DATA;
use crate::font::{FaceIdx, GlyphFormat, HintingMode, parse_features};

/// Helper: build a `UiFontSizes` from the embedded font with default settings.
fn test_registry() -> UiFontSizes {
    let font_set = FontSet::embedded();
    UiFontSizes::new(
        font_set,
        96.0,
        GlyphFormat::Alpha,
        HintingMode::Full,
        400,
        550,
        PRELOAD_SIZES,
    )
    .expect("registry must build")
}

/// Helper: build a `UiFontSizes` from IBM Plex Mono (UI font) with empty
/// fallbacks, a single preloaded size, and default settings.
///
/// Used by the regression suite where the test needs a clean
/// "no-fallback" baseline so that an injected fallback is unambiguously
/// observable on every rebuild path.
fn ui_test_registry() -> UiFontSizes {
    // IBM Plex Mono embedded — `fallbacks: Vec::new()` per `FontSet::ui_embedded`.
    let font_set = FontSet::ui_embedded();
    UiFontSizes::new(
        font_set,
        96.0,
        GlyphFormat::Alpha,
        HintingMode::None,
        400,
        700,
        // Two sizes so tests can iterate across multiple collections.
        &[13.0, 16.0],
    )
    .expect("ui registry must build")
}

/// Helper: construct a `FontData` from the embedded test emoji bytes.
///
/// Returns a fresh `Arc<FontBytes>` on each call — callers that need to
/// observe `Arc::ptr_eq` identity must reuse the returned value directly.
fn test_emoji_font_data() -> FontData {
    FontData {
        data: FontBytes::owned(TEST_EMOJI_DATA.to_vec()),
        index: 0,
    }
}

/// Count the fallbacks on the registry's default collection. Used as a
/// compact "did the rebuild preserve the injected fallback" probe.
fn default_fallback_count(reg: &UiFontSizes) -> usize {
    reg.default_collection()
        .expect("default collection must exist")
        .fallback_font_data()
        .len()
}

// Construction

#[test]
fn preloaded_sizes_match_expected_count() {
    let reg = test_registry();
    assert_eq!(reg.len(), PRELOAD_SIZES.len());
}

#[test]
fn default_collection_exists() {
    let reg = test_registry();
    assert!(reg.default_collection().is_some());
}

#[test]
fn default_q6_matches_13px_at_96dpi() {
    let reg = test_registry();
    // 13px logical at scale=1 → 13px physical → size_q6 = (13 * 64).round() = 832.
    let expected = size_key(13.0);
    assert_eq!(reg.default_q6, expected);
}

// Exact-size lookup

#[test]
fn select_returns_exact_size_collection() {
    let reg = test_registry();
    // 18px logical at scale=1 → physical 18px.
    let fc = reg.select(18.0, 1.0);
    assert!(fc.is_some());
    let size_px = fc.unwrap().size_px();
    // size_px should be 18 * 96/96 = 18 (within rounding).
    assert!(
        (size_px - 18.0).abs() < 0.5,
        "expected ~18px, got {size_px}"
    );
}

#[test]
fn select_returns_none_for_missing_size() {
    let reg = test_registry();
    // 42px is not in the preload list.
    assert!(reg.select(42.0, 1.0).is_none());
}

// Lazy creation via ensure_size

#[test]
fn ensure_size_creates_collection_for_unseen_size() {
    let mut reg = test_registry();
    let initial = reg.len();
    reg.ensure_size(42.0, 1.0)
        .expect("lazy creation must succeed");
    assert_eq!(reg.len(), initial + 1);

    // Now select finds it.
    let fc = reg.select(42.0, 1.0).expect("must find ensured size");
    let size_px = fc.size_px();
    assert!(
        (size_px - 42.0).abs() < 0.5,
        "expected ~42px, got {size_px}"
    );

    // Second ensure_size is a no-op.
    reg.ensure_size(42.0, 1.0)
        .expect("cached lookup must succeed");
    assert_eq!(reg.len(), initial + 1);
}

// Q6 lookup

#[test]
fn select_by_q6_finds_preloaded_size() {
    let reg = test_registry();
    let q6 = size_key(13.0); // 13px physical at scale=1.
    assert!(reg.select_by_q6(q6).is_some());
}

#[test]
fn select_by_q6_returns_none_for_unknown() {
    let reg = test_registry();
    // Fabricate a q6 that doesn't correspond to any preloaded size.
    let q6 = size_key(99.0);
    assert!(reg.select_by_q6(q6).is_none());
}

// DPI rebuild

#[test]
fn set_dpi_rebuilds_all_collections() {
    let mut reg = test_registry();
    let old_default_q6 = reg.default_q6;
    let old_count = reg.len();

    // Double the DPI (simulates moving to a 2× display).
    reg.set_dpi(192.0).expect("DPI rebuild must succeed");

    // Same number of collections, but keys changed.
    assert_eq!(reg.len(), old_count);
    assert_ne!(
        reg.default_q6, old_default_q6,
        "q6 keys must change with DPI"
    );

    // The 13px collection at 2× should have physical size ~26px.
    let fc = reg
        .default_collection()
        .expect("default must exist after rebuild");
    assert!(
        (fc.size_px() - 26.0).abs() < 0.5,
        "expected ~26px at 2×, got {}",
        fc.size_px()
    );
}

#[test]
fn set_dpi_noop_when_unchanged() {
    let mut reg = test_registry();
    let q6_before = reg.default_q6;
    reg.set_dpi(96.0).expect("noop must succeed");
    assert_eq!(reg.default_q6, q6_before);
}

// Standalone default creation

#[test]
fn create_default_collection_matches_registry_size() {
    let reg = test_registry();
    let standalone = reg.create_default_collection().expect("must succeed");
    let registry_fc = reg.default_collection().unwrap();
    assert!(
        (standalone.size_px() - registry_fc.size_px()).abs() < 0.01,
        "standalone size {} must match registry size {}",
        standalone.size_px(),
        registry_fc.size_px(),
    );
}

// Post-rebuild hook

/// Helper: check if a feature list contains a "smcp" feature.
fn has_smcp(features: &[rustybuzz::Feature]) -> bool {
    let smcp = parse_features(&["smcp"]);
    features.iter().any(|f| f.tag == smcp[0].tag)
}

#[test]
fn set_dpi_reapplies_post_rebuild_hook() {
    let mut reg = test_registry();

    // Install a hook that sets a custom feature ("smcp" — small caps).
    let features = parse_features(&["smcp"]);
    reg.set_post_rebuild_hook(Box::new(move |fc| {
        fc.set_features(features.clone());
    }));

    // Before DPI change: default collection still has original features.
    let before = reg
        .default_collection()
        .unwrap()
        .features_for_face(FaceIdx::REGULAR);
    assert!(
        !has_smcp(before),
        "smcp should not be present before hook runs"
    );

    // DPI change triggers rebuild_all → hook reapplied.
    reg.set_dpi(192.0).expect("DPI rebuild must succeed");

    let after = reg
        .default_collection()
        .unwrap()
        .features_for_face(FaceIdx::REGULAR);
    assert!(has_smcp(after), "smcp must be present after DPI rebuild");
}

#[test]
fn ensure_size_applies_post_rebuild_hook() {
    let mut reg = test_registry();

    let features = parse_features(&["smcp"]);
    reg.set_post_rebuild_hook(Box::new(move |fc| {
        fc.set_features(features.clone());
    }));

    reg.ensure_size(42.0, 1.0).expect("must succeed");
    let fc = reg.select(42.0, 1.0).expect("42px must exist");
    assert!(
        has_smcp(fc.features_for_face(FaceIdx::REGULAR)),
        "smcp must be present on newly ensured size"
    );
}

#[test]
fn create_default_collection_applies_post_rebuild_hook() {
    let mut reg = test_registry();

    let features = parse_features(&["smcp"]);
    reg.set_post_rebuild_hook(Box::new(move |fc| {
        fc.set_features(features.clone());
    }));

    let standalone = reg.create_default_collection().expect("must succeed");
    assert!(
        has_smcp(standalone.features_for_face(FaceIdx::REGULAR)),
        "smcp must be present on standalone default collection"
    );
}

// regression: emoji fallback must survive every UiFontSizes
// rebuild path (set_dpi, ensure_size, create_default_collection) and
// inject_fallbacks must be idempotent so DPI changes + GPU-recovery
// re-injection cannot grow the fallback chain unboundedly.
// See bug-tracker/plans/completed/00-overview.md.

/// Regression: emoji vanishes from tab titles after DPI change.
/// Pins that the default collection retains its injected fallback across
/// `rebuild_all` (triggered by `set_dpi`).
#[test]
fn set_dpi_preserves_injected_fallbacks() {
    let mut reg = ui_test_registry();
    assert_eq!(
        default_fallback_count(&reg),
        0,
        "ui_test_registry should have zero fallbacks before injection"
    );

    reg.inject_fallbacks(&[test_emoji_font_data()]);
    assert_eq!(
        default_fallback_count(&reg),
        1,
        "inject_fallbacks should land on existing collections"
    );

    reg.set_dpi(192.0).expect("DPI rebuild must succeed");
    assert_eq!(
        default_fallback_count(&reg),
        1,
        "DPI rebuild must preserve injected fallback"
    );
}

/// Regression: every preloaded size (not only the default)
/// must retain the injected fallback after rebuild.
#[test]
fn set_dpi_preserves_injected_fallbacks_all_collections() {
    let mut reg = ui_test_registry();
    reg.inject_fallbacks(&[test_emoji_font_data()]);
    reg.set_dpi(192.0).expect("DPI rebuild must succeed");

    for &logical in &[13.0_f32, 16.0] {
        // At 2× DPI the physical scale is 2.0.
        let fc = reg
            .select(logical, 2.0)
            .unwrap_or_else(|| panic!("collection for {logical}px at 2× must exist"));
        assert_eq!(
            fc.fallback_font_data().len(),
            1,
            "collection for {logical}px at 2× must retain injected fallback"
        );
    }
}

/// Regression guard: `set_dpi` that is a no-op (same DPI) must not duplicate or
/// drop fallbacks. Guards against a regression that re-enters the rebuild
/// path unnecessarily.
#[test]
fn set_dpi_noop_does_not_change_fallbacks() {
    let mut reg = ui_test_registry();
    reg.inject_fallbacks(&[test_emoji_font_data()]);
    let before = default_fallback_count(&reg);

    reg.set_dpi(96.0).expect("noop DPI must succeed");

    assert_eq!(
        default_fallback_count(&reg),
        before,
        "no-op set_dpi must leave fallback count unchanged"
    );
}

/// `inject_fallbacks(&[])` is a no-op for both storage and existing
/// collections. Pins the edge case so no-op inject cannot accidentally
/// trigger work (allocation, bind-group churn, etc.).
#[test]
fn inject_fallbacks_empty_data_is_noop() {
    let mut reg = ui_test_registry();
    let before = default_fallback_count(&reg);
    reg.inject_fallbacks(&[]);
    assert_eq!(default_fallback_count(&reg), before);
}

/// Regression: `ensure_size` path.
/// A size added AFTER `inject_fallbacks` must still carry the injected
/// fallback. Pins the "new-collection factory produces a complete
/// collection" invariant on the runtime-size-registration path.
#[test]
fn ensure_size_applies_injected_fallbacks() {
    let mut reg = ui_test_registry();
    reg.inject_fallbacks(&[test_emoji_font_data()]);

    // 42px is deliberately not in the preload list (`ui_test_registry`
    // preloads 13px + 16px).
    reg.ensure_size(42.0, 1.0)
        .expect("ensure_size must succeed");
    let fc = reg
        .select(42.0, 1.0)
        .expect("42px must exist after ensure_size");
    assert_eq!(
        fc.fallback_font_data().len(),
        1,
        "ensure_size must apply injected fallbacks to the new collection"
    );
}

/// Regression: `create_default_collection` path.
/// The standalone default collection returned by `create_default_collection`
/// must carry injected fallbacks. Used by `WindowRenderer::new_ui_only`
/// for dialog/tab-bar flows where a standalone terminal-font slot is built
/// from the UI registry.
#[test]
fn create_default_collection_applies_injected_fallbacks() {
    let mut reg = ui_test_registry();
    reg.inject_fallbacks(&[test_emoji_font_data()]);
    let standalone = reg.create_default_collection().expect("must succeed");
    assert_eq!(
        standalone.fallback_font_data().len(),
        1,
        "create_default_collection must apply injected fallbacks"
    );
}

/// The post-rebuild hook AND the injected fallback must both apply after
/// a rebuild — neither lost. Pins the interaction, not just the two legs
/// in isolation.
#[test]
fn hook_and_injection_both_apply_after_rebuild() {
    let mut reg = ui_test_registry();

    let features = parse_features(&["smcp"]);
    reg.set_post_rebuild_hook(Box::new(move |fc| {
        fc.set_features(features.clone());
    }));

    reg.inject_fallbacks(&[test_emoji_font_data()]);
    reg.set_dpi(192.0).expect("DPI rebuild must succeed");

    let fc = reg
        .default_collection()
        .expect("default collection must exist after rebuild");
    assert!(
        has_smcp(fc.features_for_face(FaceIdx::REGULAR)),
        "smcp feature (post_rebuild_hook) must be present after rebuild"
    );
    assert_eq!(
        fc.fallback_font_data().len(),
        1,
        "injected fallback must be present after rebuild"
    );
}

/// Production init order (per `oriterm/src/app/init/mod.rs:169-179`): the
/// config hook is installed BEFORE `inject_fallbacks` runs. A later DPI
/// change must preserve BOTH the hook's work and the injected fallback.
#[test]
fn init_order_hook_first_then_injection_survives_dpi() {
    let mut reg = ui_test_registry();

    // Step 1: install hook (mirrors `apply_font_config_to_ui_sizes`).
    let features = parse_features(&["smcp"]);
    reg.set_post_rebuild_hook(Box::new(move |fc| {
        fc.set_features(features.clone());
    }));

    // Step 2: inject (mirrors `WindowRenderer::new`'s post-construction step).
    reg.inject_fallbacks(&[test_emoji_font_data()]);

    // Step 3: DPI rebuild (the trigger).
    reg.set_dpi(192.0).expect("DPI rebuild must succeed");

    let fc = reg.default_collection().unwrap();
    assert!(has_smcp(fc.features_for_face(FaceIdx::REGULAR)));
    assert_eq!(fc.fallback_font_data().len(), 1);
}

/// Idempotency pin: calling `inject_fallbacks` twice with the same
/// `Arc`-backed `FontData` must be a no-op on the second call. Guards the
/// GPU-recovery path (`app/gpu_recovery.rs:102-103`), which reuses the
/// same `UiFontSizes` across a `WindowRenderer::new` rebuild — a
/// non-idempotent `inject_fallbacks` would double the fallback chain on
/// every recovery cycle.
#[test]
fn inject_fallbacks_is_idempotent_same_arc() {
    let mut reg = ui_test_registry();
    let fd = test_emoji_font_data();

    reg.inject_fallbacks(&[fd.clone()]);
    reg.inject_fallbacks(&[fd.clone()]);

    assert_eq!(
        default_fallback_count(&reg),
        1,
        "same-Arc re-injection must not duplicate the fallback"
    );
}

/// Repeated DPI transitions must not grow the fallback count — pins
/// "no-unbounded-growth" across the dominant user flow (dragging a window
/// between monitors of different DPI many times).
#[test]
fn repeated_dpi_transitions_do_not_grow_fallbacks() {
    let mut reg = ui_test_registry();
    reg.inject_fallbacks(&[test_emoji_font_data()]);

    for i in 0..10 {
        let dpi = if i % 2 == 0 { 192.0 } else { 96.0 };
        reg.set_dpi(dpi).expect("DPI rebuild must succeed");
        assert_eq!(
            default_fallback_count(&reg),
            1,
            "iteration {i}: fallback count must stay at 1 across DPI changes"
        );
    }
}

/// Property: after a DPI rebuild, the fallback's underlying `Arc`
/// matches the originally-injected `Arc` (bytes are the same data, not a
/// fresh load). This distinguishes the `append_fallback_data` replay path
/// (which clones `Arc` references) from alternative (A) — mutating
/// `font_set.fallbacks` — which would force `FontCollection::new` to
/// process the fallback through cap-height normalization, which would
/// still `Arc::clone` the same bytes but would also change the scale
/// factor. The Arc identity preserves the bytes; the `append_fallback_data`
/// path preserves the `scale_factor: 1.0` contract by construction.
#[test]
fn rebuild_all_preserves_exact_fallback_bytes() {
    let mut reg = ui_test_registry();
    let fd = test_emoji_font_data();
    let original_arc = Arc::clone(&fd.data);

    reg.inject_fallbacks(&[fd]);
    reg.set_dpi(192.0).expect("DPI rebuild must succeed");

    let exported = reg.default_collection().unwrap().fallback_font_data();
    assert_eq!(exported.len(), 1);
    assert!(
        Arc::ptr_eq(&exported[0].data, &original_arc),
        "fallback bytes after rebuild must share the originally-injected Arc"
    );
}
