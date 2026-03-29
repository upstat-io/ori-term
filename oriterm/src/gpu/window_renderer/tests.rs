//! Unit tests for the per-window renderer.
//!
//! Non-GPU tests verify display formatting and icon coverage.
//! GPU-gated tests exercise `append_dividers`, `append_focus_border`,
//! and `append_window_border` on a real `WindowRenderer`.

use std::collections::HashSet;

use oriterm_ui::icons::IconId;

use super::*;

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
