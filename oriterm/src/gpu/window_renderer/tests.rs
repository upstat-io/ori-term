//! Unit tests for the per-window renderer.
//!
//! Non-GPU tests verify display formatting and icon coverage.
//! GPU-gated tests exercise `append_dividers`, `append_focus_border`,
//! and `append_window_border` on a real `WindowRenderer`.

use std::collections::HashSet;

use oriterm_ui::icons::IconId;
use oriterm_ui::text::ShapedGlyph;

use super::super::prepare::ShapedFrame;
use super::helpers::grid_raster_keys;
use super::*;
use crate::font::build_col_glyph_map;

#[test]
fn surface_error_display() {
    assert_eq!(SurfaceError::Outdated.to_string(), "surface outdated");
    assert_eq!(SurfaceError::Lost.to_string(), "surface or device lost");
    assert_eq!(SurfaceError::Timeout.to_string(), "surface timeout");
    assert_eq!(SurfaceError::OutOfMemory.to_string(), "GPU out of memory");
    assert_eq!(
        SurfaceError::Other("driver TDR".to_string()).to_string(),
        "surface error: driver TDR",
    );
}

/// Every `IconId` variant appears exactly once in `ICON_SIZES`.
///
/// Prevents drift between the pre-resolution list and actual icon definitions.
/// If a new `IconId` variant is added without a corresponding `ICON_SIZES`
/// entry, this test fails.
#[test]
fn icon_sizes_covers_all_icon_ids() {
    let resolved: HashSet<IconId> = WindowRenderer::ICON_SIZES
        .iter()
        .map(|&(id, _)| id)
        .collect();
    for &id in IconId::ALL {
        assert!(
            resolved.contains(&id),
            "{id:?} missing from ICON_SIZES — add an entry in window_renderer/icons.rs"
        );
    }
    assert_eq!(
        resolved.len(),
        IconId::ALL.len(),
        "ICON_SIZES has {} entries but IconId::ALL has {} — check for duplicates",
        resolved.len(),
        IconId::ALL.len()
    );
}

/// No duplicate `(IconId, size)` pairs in `ICON_SIZES`.
#[test]
fn icon_sizes_no_duplicates() {
    let mut seen = HashSet::new();
    for &(id, size) in &WindowRenderer::ICON_SIZES {
        assert!(
            seen.insert((id, size)),
            "duplicate ICON_SIZES entry: ({id:?}, {size})"
        );
    }
}

// --- Raster key: subpixel positioning ---

#[test]
fn grid_raster_keys_disabled_subpx_all_zero() {
    let size_q6 = 768;
    let mut sf = ShapedFrame::new(3, size_q6);
    let glyphs = vec![
        ShapedGlyph {
            glyph_id: 42,
            face_index: 0,
            synthetic: 0,
            x_advance: 8.0,
            x_offset: 0.3, // fractional — would produce non-zero subpx_x
            y_offset: 0.0,
        },
        ShapedGlyph {
            glyph_id: 43,
            face_index: 0,
            synthetic: 0,
            x_advance: 8.0,
            x_offset: 0.7,
            y_offset: 0.0,
        },
    ];
    let col_starts = vec![0, 1];
    let mut col_map = Vec::new();
    build_col_glyph_map(&col_starts, 3, &mut col_map);
    sf.push_row(&glyphs, &col_starts, &col_map);

    // subpixel_positioning = false → all keys have subpx_x == 0.
    let keys: Vec<_> = grid_raster_keys(&sf, true, false).collect();
    assert!(keys.len() >= 2);
    for key in &keys {
        assert_eq!(
            key.subpx_x, 0,
            "expected subpx_x=0 when positioning disabled"
        );
    }
}

#[test]
fn grid_raster_keys_enabled_subpx_nonzero() {
    let size_q6 = 768;
    let mut sf = ShapedFrame::new(2, size_q6);
    let glyphs = vec![ShapedGlyph {
        glyph_id: 42,
        face_index: 0,
        synthetic: 0,
        x_advance: 8.0,
        x_offset: 0.3, // fractional → non-zero subpx_x with positioning enabled
        y_offset: 0.0,
    }];
    let col_starts = vec![0];
    let mut col_map = Vec::new();
    build_col_glyph_map(&col_starts, 2, &mut col_map);
    sf.push_row(&glyphs, &col_starts, &col_map);

    let keys: Vec<_> = grid_raster_keys(&sf, true, true).collect();
    assert!(!keys.is_empty());
    // 0.3 maps to subpx bin 1 (quarter-pixel: 0.25-0.5 → bin 1).
    assert_ne!(
        keys[0].subpx_x, 0,
        "expected non-zero subpx_x with 0.3 offset"
    );
}

#[test]
fn scene_raster_keys_disabled_subpx_all_zero() {
    use super::helpers::scene_raster_keys;
    use oriterm_ui::draw::Scene;
    use oriterm_ui::geometry::Point;
    use oriterm_ui::text::ShapedText;

    let mut scene = Scene::new();
    let glyphs = vec![
        ShapedGlyph {
            glyph_id: 65,
            face_index: 0,
            synthetic: 0,
            x_advance: 7.0,
            x_offset: 0.3,
            y_offset: 0.0,
        },
        ShapedGlyph {
            glyph_id: 66,
            face_index: 0,
            synthetic: 0,
            x_advance: 7.0,
            x_offset: 0.7,
            y_offset: 0.0,
        },
    ];
    let width: f32 = glyphs.iter().map(|g| g.x_advance).sum();
    let st = ShapedText::new(glyphs, width, 14.0, 12.0, 768, 400);
    scene.push_text(Point::new(10.0, 20.0), st, oriterm_ui::color::Color::WHITE);

    let mut keys = Vec::new();
    scene_raster_keys(&scene, true, 1.0, &mut keys, false);

    assert!(
        keys.len() >= 2,
        "expected at least 2 keys, got {}",
        keys.len()
    );
    for key in &keys {
        assert_eq!(
            key.subpx_x, 0,
            "expected subpx_x=0 when positioning disabled"
        );
    }
}

// --- GPU-gated multi-pane chrome tests ---
//
// These tests call the real production methods on a `WindowRenderer`
// constructed via `headless_env()` and verify instance buffer counts.

#[cfg(feature = "gpu-tests")]
mod chrome {
    use crate::gpu::ViewportSize;
    use crate::gpu::visual_regression::headless_env;
    use crate::session::compute::DividerLayout;
    use crate::session::rect::Rect;
    use crate::session::split_tree::SplitDirection;
    use oriterm_core::Rgb;
    use oriterm_mux::PaneId;

    /// Helper: construct a test `DividerLayout`.
    fn test_divider(x: f32, y: f32, w: f32, h: f32) -> DividerLayout {
        DividerLayout {
            rect: Rect {
                x,
                y,
                width: w,
                height: h,
            },
            direction: SplitDirection::Horizontal,
            pane_before: PaneId::from_raw(0),
            pane_after: PaneId::from_raw(1),
        }
    }

    #[test]
    fn divider_empty_list_pushes_nothing() {
        let (_gpu, _pip, mut renderer) = headless_env().expect("GPU available");
        let bg = Rgb { r: 0, g: 0, b: 0 };
        renderer.begin_multi_pane_frame(ViewportSize::new(800, 600), bg, 1.0);

        let color = Rgb {
            r: 42,
            g: 42,
            b: 54,
        };
        let hover = Rgb {
            r: 109,
            g: 155,
            b: 224,
        };
        renderer.append_dividers(&[], color, hover, None);

        assert_eq!(renderer.prepared.backgrounds.len(), 0);
    }

    #[test]
    fn divider_multiple_only_one_hovered() {
        let (_gpu, _pip, mut renderer) = headless_env().expect("GPU available");
        let bg = Rgb { r: 0, g: 0, b: 0 };
        renderer.begin_multi_pane_frame(ViewportSize::new(800, 600), bg, 1.0);

        let color = Rgb {
            r: 42,
            g: 42,
            b: 54,
        };
        let hover_color = Rgb {
            r: 109,
            g: 155,
            b: 224,
        };
        let d1 = test_divider(100.0, 0.0, 2.0, 600.0);
        let d2 = test_divider(300.0, 0.0, 2.0, 600.0);
        let d3 = test_divider(500.0, 0.0, 2.0, 600.0);
        renderer.append_dividers(&[d1, d2, d3], color, hover_color, Some(d2));

        assert_eq!(renderer.prepared.backgrounds.len(), 3);
    }

    #[test]
    fn focus_border_pushes_four_rects() {
        let (_gpu, _pip, mut renderer) = headless_env().expect("GPU available");
        let bg = Rgb { r: 0, g: 0, b: 0 };
        renderer.begin_multi_pane_frame(ViewportSize::new(800, 600), bg, 1.0);

        let color = Rgb {
            r: 109,
            g: 155,
            b: 224,
        };
        let rect = Rect {
            x: 100.0,
            y: 100.0,
            width: 200.0,
            height: 150.0,
        };
        renderer.append_focus_border(&rect, color, 2.0);

        assert_eq!(renderer.prepared.cursors.len(), 4);
    }

    #[test]
    fn focus_border_scaled_width() {
        let (_gpu, _pip, mut renderer) = headless_env().expect("GPU available");
        let bg = Rgb { r: 0, g: 0, b: 0 };
        renderer.begin_multi_pane_frame(ViewportSize::new(800, 600), bg, 1.0);

        let color = Rgb {
            r: 109,
            g: 155,
            b: 224,
        };
        let rect = Rect {
            x: 100.0,
            y: 100.0,
            width: 200.0,
            height: 150.0,
        };
        renderer.append_focus_border(&rect, color, 4.0); // 2x DPI

        assert_eq!(renderer.prepared.cursors.len(), 4);
    }

    #[test]
    fn window_border_pushes_four_rects() {
        let (_gpu, _pip, mut renderer) = headless_env().expect("GPU available");
        let bg = Rgb { r: 0, g: 0, b: 0 };
        renderer.begin_multi_pane_frame(ViewportSize::new(800, 600), bg, 1.0);

        let color = Rgb {
            r: 58,
            g: 58,
            b: 72,
        };
        renderer.append_window_border(800, 600, color, 2.0);

        assert_eq!(renderer.prepared.cursors.len(), 4);
    }

    #[test]
    fn window_border_scaled() {
        let (_gpu, _pip, mut renderer) = headless_env().expect("GPU available");
        let bg = Rgb { r: 0, g: 0, b: 0 };
        renderer.begin_multi_pane_frame(ViewportSize::new(800, 600), bg, 1.0);

        let color = Rgb {
            r: 58,
            g: 58,
            b: 72,
        };
        renderer.append_window_border(800, 600, color, 4.0); // 2x DPI

        assert_eq!(renderer.prepared.cursors.len(), 4);
    }
}

// --- GPU-gated font config tests ---

#[cfg(feature = "gpu-tests")]
mod font_config {
    use crate::font::collection::{FontCollection, FontSet};
    use crate::font::ui_font_sizes::{self, UiFontSizes};
    use crate::font::{GlyphFormat, HintingMode};
    use crate::gpu::state::GpuState;
    use crate::gpu::{GpuPipelines, WindowRenderer};

    const TEST_DPI: f32 = 96.0;
    const TEST_FONT_SIZE_PT: f32 = 12.0;
    const TEST_FONT_WEIGHT: u16 = 400;

    /// Headless environment with UI font sizes populated.
    fn headless_with_ui_fonts() -> Option<(GpuState, GpuPipelines, WindowRenderer)> {
        let gpu = GpuState::new_headless().ok()?;
        let pipelines = GpuPipelines::new(&gpu);
        let font_set = FontSet::embedded();
        let font_collection = FontCollection::new(
            font_set.clone(),
            TEST_FONT_SIZE_PT,
            TEST_DPI,
            GlyphFormat::Alpha,
            TEST_FONT_WEIGHT,
            550,
            HintingMode::Full,
        )
        .ok()?;
        let ui_sizes = UiFontSizes::new(
            font_set,
            TEST_DPI,
            GlyphFormat::Alpha,
            HintingMode::None,
            TEST_FONT_WEIGHT,
            550,
            ui_font_sizes::PRELOAD_SIZES,
        )
        .ok()?;
        let renderer = WindowRenderer::new(&gpu, &pipelines, font_collection, Some(ui_sizes));
        Some((gpu, pipelines, renderer))
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

    /// Regression: round-1 TPR finding TPR-04-004-.
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
        let Some(gpu) = GpuState::new_headless().ok() else {
            eprintln!("skipped: no GPU adapter available");
            return;
        };
        let pipelines = GpuPipelines::new(&gpu);

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
            GlyphFormat::Alpha,
            TEST_FONT_WEIGHT,
            550,
            HintingMode::Full,
        )
        .expect("terminal FontCollection must build")
    }

    /// Build a fresh `UiFontSizes` with empty fallbacks (mirrors production).
    fn fresh_empty_ui_sizes() -> UiFontSizes {
        UiFontSizes::new(
            FontSet::ui_embedded(),
            TEST_DPI,
            GlyphFormat::Alpha,
            HintingMode::None,
            TEST_FONT_WEIGHT,
            550,
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
}

// --- Chrome render SSOT consolidation pins ---
//
// Pin the `cache_invalidated_this_frame` SSOT contract:
// - Constructor-init: both `WindowRenderer::new` and `new_ui_only` start
//   with the flag false (no frame prepared yet).
// - Multi-pane setter: `begin_multi_pane_frame` unconditionally sets the
//   flag true (multi-pane always rebuilds the aggregate prepared frame).
// - Chrome SSOT consumer: source-scan archaeology asserts `chrome.rs`
//   reads from `renderer.cache_invalidated_this_frame()` and does NOT
//   maintain a parallel `ChromeParams` predicate.

/// Regression: chrome.rs must read from
/// `WindowRenderer::cache_invalidated_this_frame()` and NOT carry a
/// parallel `ChromeParams::{content_dirty, selection_changed, blink_changed}`
/// predicate. Drift would re-introduce the SSOT violation; this test
/// fails at compile time if `chrome.rs` regresses.
///
/// See: bug-tracker/plans/BUG-06-033/section-03-tdd-matrix.md §"Parallel-
/// predicate regression pin (renderer-level surrogate + automated archaeology)"
#[test]
fn chrome_render_queries_renderer_ssot() {
    let chrome_src = include_str!("../../app/redraw/chrome.rs");

    // Positive: chrome reads from the SSOT query.
    assert!(
        chrome_src.contains("renderer.cache_invalidated_this_frame() || ctx.ui_stale"),
        "chrome.rs must read needs_full_render from renderer.cache_invalidated_this_frame() \
         (SSOT consolidation regression)"
    );

    // Negative: the old parallel-predicate fields must be gone from chrome.
    let banned = [
        "params.content_dirty",
        "params.selection_changed",
        "params.blink_changed",
    ];
    for forbidden in &banned {
        assert!(
            !chrome_src.contains(forbidden),
            "chrome.rs must NOT reference `{forbidden}` (parallel predicate removed in SSOT consolidation)"
        );
    }
}

#[cfg(feature = "gpu-tests")]
mod cache_invalidated_pins {
    use crate::gpu::ViewportSize;
    use crate::gpu::pipelines::GpuPipelines;
    use crate::gpu::state::GpuState;
    use crate::gpu::window_renderer::WindowRenderer;
    use oriterm_core::Rgb;

    use crate::gpu::visual_regression::headless_env;

    /// Constructor pin: fresh `WindowRenderer::new` reports cache-NOT-invalidated.
    ///
    /// No frame has been prepared yet, so the flag must default to false.
    #[test]
    fn new_constructor_initializes_cache_invalidated_to_false() {
        let Some((_gpu, _pip, renderer)) = headless_env() else {
            eprintln!("SKIP: GPU adapter unavailable");
            return;
        };
        assert!(
            !renderer.cache_invalidated_this_frame(),
            "WindowRenderer::new must initialize cache_invalidated_this_frame to false"
        );
    }

    /// Constructor pin: fresh `WindowRenderer::new_ui_only` reports cache-NOT-invalidated.
    #[test]
    fn new_ui_only_constructor_initializes_cache_invalidated_to_false() {
        let Some(gpu) = GpuState::new_headless().ok() else {
            eprintln!("SKIP: GPU adapter unavailable");
            return;
        };
        let pipelines = GpuPipelines::new(&gpu);
        let font_set = crate::font::collection::FontSet::ui_embedded();
        let Some(ui_sizes) = crate::font::ui_font_sizes::UiFontSizes::new(
            font_set,
            96.0,
            crate::font::GlyphFormat::Alpha,
            crate::font::HintingMode::None,
            400,
            550,
            crate::font::ui_font_sizes::PRELOAD_SIZES,
        )
        .ok() else {
            eprintln!("SKIP: UI font setup failed");
            return;
        };
        let renderer = WindowRenderer::new_ui_only(&gpu, &pipelines, ui_sizes);
        assert!(
            !renderer.cache_invalidated_this_frame(),
            "WindowRenderer::new_ui_only must initialize cache_invalidated_this_frame to false"
        );
    }

    /// Multi-pane setter pin: `begin_multi_pane_frame` unconditionally sets
    /// the flag true. Multi-pane always rebuilds the aggregate prepared
    /// frame (no incremental fast path), so the SSOT must report invalidation.
    ///
    /// Note: this is a setter-correctness pin, not a regression pin —
    /// `multi_pane/mod.rs:122-127` already triggers `any_content_changed`
    /// via `is_focused`, so the chrome stale-blit case the parallel
    /// predicate missed was unreachable in production. The unconditional
    /// `true` here matches the cleared-cache state, not closes a real bug.
    #[test]
    fn begin_multi_pane_frame_sets_cache_invalidated_true() {
        let Some((_gpu, _pip, mut renderer)) = headless_env() else {
            eprintln!("SKIP: GPU adapter unavailable");
            return;
        };
        let bg = Rgb { r: 0, g: 0, b: 0 };
        renderer.begin_multi_pane_frame(ViewportSize::new(800, 600), bg, 1.0);
        assert!(
            renderer.cache_invalidated_this_frame(),
            "begin_multi_pane_frame must set cache_invalidated_this_frame=true \
             (multi-pane always rebuilds, no incremental fast path)"
        );
    }

    #[test]
    fn opacity_change_invalidates_cache() {
        let Some((_gpu, _pip, mut renderer)) = headless_env() else {
            eprintln!("SKIP: GPU adapter unavailable");
            return;
        };

        let mut input = crate::gpu::frame_input::FrameInput::test_grid(10, 10, "");
        let origin = (0.0_f32, 0.0_f32);

        // Align input so only palette.opacity is the variable.
        input.text_blink_opacity = 1.0;
        input.fg_dim = 1.0;
        input.subpixel_positioning = false;
        input.selection = None;
        input.hovered_cell = None;

        // Seed prev_dispatch_fingerprint by computing for the baseline input.
        // Default PreparedFrame has prev_dispatch_fingerprint=None, so we
        // must publish a fingerprint first (mimicking what prepare does).
        input.palette.opacity = 1.0;
        let baseline = crate::gpu::prepare::compute_dispatch_fingerprint(&input, origin);
        renderer.prepared.prev_dispatch_fingerprint = Some(baseline);

        // Same opacity: no change.
        assert!(
            !renderer.has_dispatch_change(&input, origin),
            "opacity 1.0 to 1.0 must not invalidate fingerprint"
        );

        // Delta of 0.5 changes the fingerprint.
        input.palette.opacity = 0.5;
        assert!(
            renderer.has_dispatch_change(&input, origin),
            "opacity change from 1.0 to 0.5 must invalidate dispatch fingerprint"
        );

        // Bitwise-exact comparison replaces the prior `> 0.001` threshold.
        // Sub-EPSILON deltas now invalidate (extra rebuilds, never stale reuse).
        input.palette.opacity = 0.9995;
        assert!(
            renderer.has_dispatch_change(&input, origin),
            "bitwise-exact comparison must invalidate on any non-zero delta"
        );
    }

    /// Regression: BUG-06-030 — selection change MUST gate the cursor-only
    /// fast path. The dispatch fingerprint excludes `prev_selection_snapshot`
    /// because selection damage is handled per-row by `build_dirty_set` inside
    /// the incremental prepare pass — but the fast path BYPASSES prepare
    /// entirely, so selection changes there would silently leave stale
    /// selection decorations without `has_row_state_change`.
    /// See: bug-tracker/plans/BUG-06-030/
    #[test]
    fn fast_path_skipped_when_selection_changes() {
        use oriterm_core::{Selection, Side, StableRowIndex};

        let Some((_gpu, _pip, mut renderer)) = headless_env() else {
            eprintln!("SKIP: GPU adapter unavailable");
            return;
        };

        let mut input = crate::gpu::frame_input::FrameInput::test_grid(10, 10, "");
        let origin = (0.0_f32, 0.0_f32);
        input.selection = None;
        input.hovered_cell = None;

        // Seed both fingerprint and selection snapshot to baseline ("no selection").
        let baseline_fp = crate::gpu::prepare::compute_dispatch_fingerprint(&input, origin);
        renderer.prepared.prev_dispatch_fingerprint = Some(baseline_fp);
        renderer.prepared.prev_selection_snapshot = None;

        // Add a selection — fingerprint stays the same, selection snapshot changes.
        let mut sel = Selection::new_char(StableRowIndex(0), 0, Side::Left);
        sel.end = oriterm_core::SelectionPoint {
            row: StableRowIndex(0),
            col: 5,
            side: Side::Right,
        };
        input.selection = Some(crate::gpu::frame_input::FrameSelection::new(&sel, 0));
        assert!(
            !renderer.has_dispatch_change(&input, origin),
            "selection change must NOT alter the dispatch fingerprint"
        );
        assert!(
            renderer.has_row_state_change(&input),
            "selection change MUST trigger has_row_state_change to gate the fast path"
        );
    }

    #[test]
    fn fast_path_skipped_when_hovered_cell_changes() {
        let Some((_gpu, _pip, mut renderer)) = headless_env() else {
            eprintln!("SKIP: GPU adapter unavailable");
            return;
        };

        let mut input = crate::gpu::frame_input::FrameInput::test_grid(10, 10, "");
        let origin = (0.0_f32, 0.0_f32);
        input.selection = None;
        input.hovered_cell = None;

        let baseline_fp = crate::gpu::prepare::compute_dispatch_fingerprint(&input, origin);
        renderer.prepared.prev_dispatch_fingerprint = Some(baseline_fp);
        renderer.prepared.prev_hovered_cell = None;

        input.hovered_cell = Some((1, 2));
        assert!(
            !renderer.has_dispatch_change(&input, origin),
            "hovered_cell change must NOT alter the dispatch fingerprint"
        );
        assert!(
            renderer.has_row_state_change(&input),
            "hovered_cell change MUST trigger has_row_state_change to gate the fast path"
        );
    }

    #[test]
    fn fast_path_taken_when_selection_unchanged() {
        let Some((_gpu, _pip, mut renderer)) = headless_env() else {
            eprintln!("SKIP: GPU adapter unavailable");
            return;
        };

        let mut input = crate::gpu::frame_input::FrameInput::test_grid(10, 10, "");
        let origin = (0.0_f32, 0.0_f32);
        input.selection = None;
        input.hovered_cell = None;

        let baseline_fp = crate::gpu::prepare::compute_dispatch_fingerprint(&input, origin);
        renderer.prepared.prev_dispatch_fingerprint = Some(baseline_fp);
        renderer.prepared.prev_selection_snapshot = None;
        renderer.prepared.prev_hovered_cell = None;

        // No state change — both helpers must report no change.
        assert!(!renderer.has_dispatch_change(&input, origin));
        assert!(!renderer.has_row_state_change(&input));
    }
}
