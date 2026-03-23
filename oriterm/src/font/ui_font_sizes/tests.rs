use super::*;
use crate::font::collection::{FontSet, size_key};
use crate::font::{GlyphFormat, HintingMode};

/// Helper: build a `UiFontSizes` from the embedded font with default settings.
fn test_registry() -> UiFontSizes {
    let font_set = FontSet::embedded();
    UiFontSizes::new(
        font_set,
        96.0,
        GlyphFormat::Alpha,
        HintingMode::Full,
        400,
        PRELOAD_SIZES,
    )
    .expect("registry must build")
}

// ── Construction ──

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

// ── Exact-size lookup ──

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

// ── Lazy creation ──

#[test]
fn select_mut_creates_collection_for_unseen_size() {
    let mut reg = test_registry();
    let initial = reg.len();
    let fc = reg
        .select_mut(42.0, 1.0)
        .expect("lazy creation must succeed");
    let size_px = fc.size_px();
    assert!(
        (size_px - 42.0).abs() < 0.5,
        "expected ~42px, got {size_px}"
    );
    assert_eq!(reg.len(), initial + 1);

    // Second call returns the cached collection, no growth.
    let _ = reg
        .select_mut(42.0, 1.0)
        .expect("cached lookup must succeed");
    assert_eq!(reg.len(), initial + 1);
}

// ── Q6 lookup ──

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

// ── DPI rebuild ──

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

// ── Hinting and format propagation ──

#[test]
fn set_hinting_propagates_to_all_collections() {
    let mut reg = test_registry();
    reg.set_hinting(HintingMode::None);
    assert_eq!(reg.hinting_mode(), HintingMode::None);
    let fc = reg.default_collection().unwrap();
    assert_eq!(fc.hinting_mode(), HintingMode::None);
}

#[test]
fn set_format_propagates_to_all_collections() {
    let mut reg = test_registry();
    reg.set_format(GlyphFormat::SubpixelRgb);
    assert_eq!(reg.format(), GlyphFormat::SubpixelRgb);
    let fc = reg.default_collection().unwrap();
    assert_eq!(fc.format(), GlyphFormat::SubpixelRgb);
}

// ── Standalone default creation ──

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
