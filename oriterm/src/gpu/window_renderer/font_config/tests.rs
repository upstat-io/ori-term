//! GPU-gated font config tests.

#![cfg(feature = "gpu-tests")]

use super::WindowRenderer;
use crate::font::collection::{FontCollection, FontSet};
use crate::font::ui_font_sizes::{self, UiFontSizes};
use crate::font::{FontRasterConfig, GlyphFormat, HintingMode};
use crate::gpu::GpuPipelines;
use crate::gpu::state::GpuState;
use crate::gpu::visual_regression::{HeadlessEnvConfig, UiFontConfig, headless_env_with};

const TEST_DPI: f32 = 96.0;
const TEST_FONT_SIZE_PT: f32 = 12.0;
const TEST_FONT_WEIGHT: u16 = 400;

/// Headless GPU state shared by every test in this file.
fn headless_gpu() -> Option<(GpuState, GpuPipelines)> {
    let gpu = GpuState::new_headless().ok()?;
    let pipelines = GpuPipelines::new(&gpu);
    Some((gpu, pipelines))
}

/// Headless environment with UI font sizes populated.
fn headless_with_ui_fonts() -> Option<(GpuState, GpuPipelines, WindowRenderer)> {
    headless_env_with(&HeadlessEnvConfig {
        ui: Some(UiFontConfig {
            font_set: FontSet::embedded(),
            hinting: HintingMode::None,
        }),
        ..Default::default()
    })
}

#[test]
fn set_hinting_and_format_preserves_ui_font_settings() {
    let Some((gpu, _pip, mut renderer)) = headless_with_ui_fonts() else {
        eprintln!("skipped: no GPU adapter or fonts available");
        return;
    };

    // UI font starts with Alpha/None.
    assert_eq!(
        renderer.ui_font_sizes().unwrap().format(),
        GlyphFormat::Alpha
    );
    assert_eq!(
        renderer.ui_font_sizes().unwrap().hinting_mode(),
        HintingMode::None
    );

    // Change terminal font to SubpixelRgb/Full.
    renderer.set_hinting_and_format(HintingMode::Full, GlyphFormat::SubpixelRgb, &gpu);

    // UI font must still be Alpha/None.
    assert_eq!(
        renderer.ui_font_sizes().unwrap().format(),
        GlyphFormat::Alpha
    );
    assert_eq!(
        renderer.ui_font_sizes().unwrap().hinting_mode(),
        HintingMode::None
    );
}

#[test]
fn set_hinting_and_format_updates_terminal_font() {
    let Some((gpu, _pip, mut renderer)) = headless_with_ui_fonts() else {
        eprintln!("skipped: no GPU adapter or fonts available");
        return;
    };

    renderer.set_hinting_and_format(HintingMode::Full, GlyphFormat::SubpixelRgb, &gpu);

    assert_eq!(renderer.font_collection().hinting_mode(), HintingMode::Full);
    assert_eq!(
        renderer.font_collection().format(),
        GlyphFormat::SubpixelRgb
    );
}

#[test]
fn set_hinting_and_format_noop_when_unchanged() {
    let Some((gpu, _pip, mut renderer)) = headless_with_ui_fonts() else {
        eprintln!("skipped: no GPU adapter or fonts available");
        return;
    };

    // Terminal starts with Alpha/Full. Pre-cache some glyphs.
    let entries_before = renderer.atlas_entry_count();

    // Call with the same values — should be a no-op (no atlas clear).
    renderer.set_hinting_and_format(HintingMode::Full, GlyphFormat::Alpha, &gpu);

    let entries_after = renderer.atlas_entry_count();
    assert_eq!(
        entries_before, entries_after,
        "atlas should not have been cleared when hinting/format unchanged"
    );
}

/// Regression: emoji reinject ordering.
///
/// Pins the source-side emoji-reinject ordering that mirrors the
/// production config-reload call sequence
/// (`oriterm/src/app/config_reload/mod.rs:187-197`):
///
/// - `WindowRenderer::replace_ui_font_sizes` is storage-only — it MUST
/// NOT inject emoji, because `rebuild_ui_font_sizes` calls it BEFORE
/// `replace_font_collection` installs the new terminal font. If
/// injection fired here it would pull stale emoji from the previous
/// terminal font.
/// - `WindowRenderer::replace_font_collection` is the canonical
/// reinject trigger — it installs the new terminal font AND
/// re-wires emoji into whatever UI registry is currently stored,
/// so the post-reload state carries the NEW terminal font's emoji.
///
/// Uses `FontSet::ui_embedded()` (empty fallbacks) for the UI side
/// to mirror production — the embedded test helper uses
/// `FontSet::embedded()` which has its own emoji fallback and would
/// mask the reinject path under a font_set-sourced fallback.
#[test]
fn replace_font_collection_reinjects_emoji_into_current_ui_registry() {
    let Some((gpu, pipelines)) = headless_gpu() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    // Terminal font: FontSet::embedded() — TEST_EMOJI_DATA is its
    // one fallback. Source of the emoji that reinject propagates.
    let terminal_fc = build_terminal_fc();
    // UI registry: FontSet::ui_embedded() — empty fallbacks. Mirrors
    // production so the only emoji in the UI chain comes through
    // reinject_emoji_fallback, not font_set.fallbacks.
    let mut renderer =
        WindowRenderer::new(&gpu, &pipelines, terminal_fc, Some(fresh_empty_ui_sizes()));

    // WindowRenderer::new calls reinject_emoji_fallback — UI registry
    // should now carry the terminal font's emoji.
    assert_eq!(
        ui_fallback_count(&renderer),
        1,
        "WindowRenderer::new should have injected the terminal font's emoji fallback"
    );

    // Step 1 (config-reload sequence): install a fresh UI registry
    // with NO fallbacks. replace_ui_font_sizes is storage-only —
    // it MUST NOT inject emoji here, because the terminal font
    // has not been replaced yet.
    renderer.replace_ui_font_sizes(fresh_empty_ui_sizes());
    assert_eq!(
        ui_fallback_count(&renderer),
        0,
        "replace_ui_font_sizes must NOT inject — it is storage-only"
    );

    // Step 2: install a new terminal font collection. This is the
    // canonical trigger for emoji reinject — the new collection's
    // fallback data must now land on the current (fresh) UI registry.
    // Snapshot the new collection's fallback Arc BEFORE moving it so
    // the identity assertion below proves the UI fallback came from
    // THIS collection, not from a stale or unrelated source.
    let new_terminal_fc = build_terminal_fc();
    let expected_fallback_arc = std::sync::Arc::clone(
        &new_terminal_fc
            .fallback_font_data()
            .first()
            .expect("new terminal FontCollection must have emoji fallback")
            .data,
    );

    renderer.replace_font_collection(new_terminal_fc, &gpu);

    let ui_fallback = renderer
        .ui_font_sizes()
        .unwrap()
        .default_collection()
        .unwrap()
        .fallback_font_data();
    assert_eq!(
        ui_fallback.len(),
        1,
        "replace_font_collection must reinject emoji from the NEW terminal font"
    );
    assert!(
        std::sync::Arc::ptr_eq(&ui_fallback[0].data, &expected_fallback_arc),
        "UI registry fallback must share the NEW terminal font's Arc — proves the reinject pulled from the newly-installed collection, not from a stale or unrelated source"
    );
}

/// Build a terminal `FontCollection` from the embedded test fonts.
fn build_terminal_fc() -> FontCollection {
    FontCollection::new(
        FontSet::embedded(),
        TEST_FONT_SIZE_PT,
        TEST_DPI,
        FontRasterConfig {
            format: GlyphFormat::Alpha,
            weight: TEST_FONT_WEIGHT,
            bold_weight: 550,
            hinting: HintingMode::Full,
        },
    )
    .expect("terminal FontCollection must build")
}

/// Build a fresh `UiFontSizes` with empty fallbacks (mirrors production).
fn fresh_empty_ui_sizes() -> UiFontSizes {
    UiFontSizes::new(
        FontSet::ui_embedded(),
        TEST_DPI,
        FontRasterConfig {
            format: GlyphFormat::Alpha,
            weight: TEST_FONT_WEIGHT,
            bold_weight: 550,
            hinting: HintingMode::None,
        },
        ui_font_sizes::PRELOAD_SIZES,
    )
    .expect("fresh UiFontSizes must build")
}

/// Count the fallbacks on the renderer's current UI default collection.
fn ui_fallback_count(renderer: &WindowRenderer) -> usize {
    renderer
        .ui_font_sizes()
        .unwrap()
        .default_collection()
        .unwrap()
        .fallback_font_data()
        .len()
}
