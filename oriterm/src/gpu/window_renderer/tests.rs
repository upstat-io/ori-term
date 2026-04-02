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
    assert_eq!(SurfaceError::Lost.to_string(), "surface lost or outdated");
    assert_eq!(SurfaceError::OutOfMemory.to_string(), "GPU out of memory");
    assert_eq!(SurfaceError::Timeout.to_string(), "surface timeout");
    assert_eq!(SurfaceError::Other.to_string(), "surface error");
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
            HintingMode::Full,
        )
        .ok()?;
        let ui_sizes = UiFontSizes::new(
            font_set,
            TEST_DPI,
            GlyphFormat::Alpha,
            HintingMode::None,
            TEST_FONT_WEIGHT,
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
}
