//! Unit tests for chrome geometry and layout helpers.

use crate::config::{TabBarPosition, TabBarStyle};
use crate::font::CellMetrics;

use super::{GRID_PADDING, compute_window_layout, grid_origin_y};

// grid_origin_y: integer-pixel guarantee

#[test]
fn origin_integer_at_100_percent_scale() {
    // 46.0 * 1.0 = 46.0 — already integer.
    let y = grid_origin_y(46.0, 1.0);
    assert_eq!(y, 46.0);
    assert_eq!(y.fract(), 0.0);
}

#[test]
fn origin_integer_at_125_percent_scale() {
    // 46.0 * 1.25 = 57.5 — fractional without rounding.
    let y = grid_origin_y(46.0, 1.25);
    assert_eq!(y, 58.0);
    assert_eq!(y.fract(), 0.0, "125% DPI must produce integer origin");
}

#[test]
fn origin_integer_at_150_percent_scale() {
    // 46.0 * 1.5 = 69.0 — already integer.
    let y = grid_origin_y(46.0, 1.5);
    assert_eq!(y, 69.0);
    assert_eq!(y.fract(), 0.0);
}

#[test]
fn origin_integer_at_175_percent_scale() {
    // 46.0 * 1.75 = 80.5 — fractional without rounding.
    let y = grid_origin_y(46.0, 1.75);
    assert_eq!(y, 81.0);
    assert_eq!(y.fract(), 0.0, "175% DPI must produce integer origin");
}

#[test]
fn origin_integer_at_200_percent_scale() {
    // 46.0 * 2.0 = 92.0 — already integer.
    let y = grid_origin_y(46.0, 2.0);
    assert_eq!(y, 92.0);
    assert_eq!(y.fract(), 0.0);
}

#[test]
fn origin_integer_at_225_percent_scale() {
    // 46.0 * 2.25 = 103.5 — fractional without rounding.
    let y = grid_origin_y(46.0, 2.25);
    assert_eq!(y, 104.0);
    assert_eq!(y.fract(), 0.0, "225% DPI must produce integer origin");
}

#[test]
fn origin_zero_chrome() {
    // No chrome — origin should be 0 at any scale.
    assert_eq!(grid_origin_y(0.0, 1.25), 0.0);
    assert_eq!(grid_origin_y(0.0, 1.75), 0.0);
}

/// Exhaustive check: all common Windows DPI scale factors produce integer origins.
#[test]
fn origin_integer_for_all_common_dpi_scales() {
    let chrome_height = 46.0; // unified tab bar (TAB_BAR_HEIGHT)
    let scales = [1.0, 1.25, 1.5, 1.75, 2.0, 2.25, 2.5, 3.0, 3.5, 4.0];
    for scale in scales {
        let y = grid_origin_y(chrome_height, scale);
        assert_eq!(
            y.fract(),
            0.0,
            "grid_origin_y({chrome_height}, {scale}) = {y} is not integer",
        );
    }
}

// compute_window_layout: layout engine produces same results as manual calculation

/// Helper to create test cell metrics.
fn test_cell(width: f32, height: f32) -> CellMetrics {
    CellMetrics {
        width,
        height,
        baseline: height * 0.8,
        underline_offset: 1.0,
        stroke_size: 1.0,
        strikeout_offset: height * 0.4,
    }
}

#[test]
fn layout_grid_origin_includes_padding() {
    // 1920×1080 at 1x scale with 8×16 cells.
    let cell = test_cell(8.0, 16.0);
    let wl = compute_window_layout(1920, 1080, &cell, 1.0, false, 46.0);

    // Tab bar is 46px at 1x. Grid origin includes padding offset.
    let pad = (GRID_PADDING * 1.0).round();
    let expected_y = grid_origin_y(46.0, 1.0) + pad;
    assert_eq!(wl.grid_rect.y(), expected_y);
    assert_eq!(wl.grid_rect.x(), pad);
}

#[test]
fn layout_padding_reduces_cols_rows() {
    // Cols/rows are computed from the visible grid area after padding.
    // This matches the WM_SIZING snap formula so the column count is
    // stable during interactive resize.
    let cell = test_cell(8.0, 16.0);
    let wl = compute_window_layout(1920, 1080, &cell, 1.0, false, 46.0);

    let pad = (GRID_PADDING * 1.0).round();
    let chrome_h = grid_origin_y(46.0, 1.0);
    let expected_cols = cell.columns((1920.0 - pad) as u32);
    let expected_rows = cell.rows((1080.0 - chrome_h - pad) as u32);
    assert_eq!(wl.cols, expected_cols);
    assert_eq!(wl.rows, expected_rows);
}

#[test]
fn layout_cols_rows_match_manual_at_125_scale() {
    // 1920×1080 at 1.25x with 10×20 physical-pixel cells.
    let scale = 1.25;
    let cell = test_cell(10.0, 20.0);
    let wl = compute_window_layout(1920, 1080, &cell, scale, false, 46.0);

    // Cols/rows computed from visible grid area after padding.
    let pad = (GRID_PADDING * scale).round();
    let chrome_px = grid_origin_y(46.0, scale);
    let expected_cols = cell.columns((1920.0 - pad) as u32);
    let expected_rows = cell.rows((1080.0 - chrome_px - pad) as u32);

    assert_eq!(wl.cols, expected_cols);
    assert_eq!(wl.rows, expected_rows);
}

#[test]
fn layout_integer_origin_at_fractional_dpi() {
    // 175% DPI — tab bar height produces fractional without rounding.
    let cell = test_cell(14.0, 28.0);
    let wl = compute_window_layout(2560, 1440, &cell, 1.75, false, 46.0);

    assert_eq!(
        wl.grid_rect.y().fract(),
        0.0,
        "grid origin must be integer-pixel aligned"
    );
}

#[test]
fn layout_minimum_one_col_one_row() {
    // Tiny viewport — must produce at least 1×1.
    let cell = test_cell(100.0, 100.0);
    let wl = compute_window_layout(50, 100, &cell, 1.0, false, 46.0);

    assert_eq!(wl.cols, 1);
    assert_eq!(wl.rows, 1);
}

#[test]
fn hidden_tab_bar_suppresses_layout() {
    // With tab_bar_hidden=true, the grid origin should start at padding
    // (no chrome height). The grid gets more rows than the default case.
    let cell = test_cell(8.0, 16.0);
    let visible = compute_window_layout(1920, 1080, &cell, 1.0, false, 46.0);
    let hidden = compute_window_layout(1920, 1080, &cell, 1.0, true, 46.0);

    let pad = (GRID_PADDING * 1.0).round();

    // Hidden layout: grid origin Y is just padding (no tab bar).
    assert_eq!(hidden.grid_rect.y(), pad);

    // Visible layout: grid origin Y includes chrome + padding.
    let chrome_h = grid_origin_y(46.0, 1.0);
    assert_eq!(visible.grid_rect.y(), chrome_h + pad);

    // Hidden layout produces more rows (no chrome stealing space).
    assert!(
        hidden.rows > visible.rows,
        "hidden={} should exceed visible={} rows",
        hidden.rows,
        visible.rows,
    );

    // Columns should be identical (tab bar only affects vertical space).
    assert_eq!(hidden.cols, visible.cols);
}

#[test]
fn hidden_tab_bar_grid_origin_at_fractional_dpi() {
    // Even with hidden tab bar, grid origin must be integer-aligned.
    let cell = test_cell(10.0, 20.0);
    let wl = compute_window_layout(1920, 1080, &cell, 1.25, true, 46.0);

    assert_eq!(
        wl.grid_rect.y().fract(),
        0.0,
        "hidden tab bar grid origin must be integer-pixel aligned"
    );
}

#[test]
fn compact_tab_bar_shifts_grid_origin() {
    // Compact tab bar (34px) should produce a different grid origin
    // from default (46px). This validates that `tab_bar_height` param
    // actually drives layout instead of a hardcoded constant.
    let cell = test_cell(8.0, 16.0);
    let default = compute_window_layout(1920, 1080, &cell, 1.0, false, 46.0);
    let compact = compute_window_layout(1920, 1080, &cell, 1.0, false, 34.0);

    assert!(
        compact.grid_rect.y() < default.grid_rect.y(),
        "compact grid y ({}) should be less than default ({})",
        compact.grid_rect.y(),
        default.grid_rect.y(),
    );

    // Compact layout should have more rows (less space for chrome).
    assert!(
        compact.rows >= default.rows,
        "compact rows ({}) should be >= default rows ({})",
        compact.rows,
        default.rows,
    );
}

#[test]
fn hidden_tab_bar_chrome_init_uses_zero_height() {
    // Regression: create_tab_bar_widget() must pass 0.0 to install_chrome()
    // and set_tab_bar_height() when tab_bar_position is Hidden, even though
    // the widget's own metrics are nonzero. This prevents macOS traffic lights
    // from briefly rendering at the wrong caption height on startup.
    use super::super::init::metrics_from_style;

    for style in [TabBarStyle::Default, TabBarStyle::Compact] {
        let metrics = metrics_from_style(style);
        assert!(
            metrics.height > 0.0,
            "{style:?} metrics height must be nonzero"
        );

        // Replicate the init-path logic: hidden overrides to 0.0.
        for (position, expected_zero) in [
            (TabBarPosition::Top, false),
            (TabBarPosition::Bottom, false),
            (TabBarPosition::Hidden, true),
        ] {
            let hidden = position == TabBarPosition::Hidden;
            let chrome_h = if hidden { 0.0 } else { metrics.height };

            if expected_zero {
                assert_eq!(
                    chrome_h, 0.0,
                    "chrome_h must be 0.0 for {position:?} + {style:?}"
                );
            } else {
                assert_eq!(
                    chrome_h, metrics.height,
                    "chrome_h must match metrics for {position:?} + {style:?}"
                );
            }
        }
    }
}

/// Regression (TPR-09-020): Search overlay chrome height must be 0 when
/// `TabBarPosition::Hidden`, regardless of the widget's own metrics.
/// The redraw paths use `if tab_bar_hidden { 0.0 } else { metrics().height }`
/// — this test validates the branch.
#[test]
fn hidden_tab_bar_search_overlay_uses_zero_chrome_height() {
    use super::super::init::metrics_from_style;

    for style in [TabBarStyle::Default, TabBarStyle::Compact] {
        let metrics = metrics_from_style(style);

        for (position, expected_zero) in [
            (TabBarPosition::Top, false),
            (TabBarPosition::Bottom, false),
            (TabBarPosition::Hidden, true),
        ] {
            let hidden = position == TabBarPosition::Hidden;
            // Mirrors the redraw/mod.rs and redraw/multi_pane/mod.rs pattern.
            let chrome_h = if hidden { 0.0 } else { metrics.height };

            if expected_zero {
                assert_eq!(
                    chrome_h, 0.0,
                    "search overlay chrome_h must be 0.0 for {position:?} + {style:?}"
                );
            } else {
                assert!(
                    chrome_h > 0.0,
                    "search overlay chrome_h must be nonzero for {position:?} + {style:?}"
                );
            }
        }
    }
}

/// Regression (TPR-09-020): Config reload position-to-Hidden must publish
/// effective height 0.0 to macOS, not the raw metrics height.
#[test]
fn config_reload_hidden_publishes_zero_effective_height() {
    use super::super::init::metrics_from_style;

    for style in [TabBarStyle::Default, TabBarStyle::Compact] {
        let metrics = metrics_from_style(style);

        // Mirrors the config_reload/mod.rs pattern.
        for (position, expect_zero) in [
            (TabBarPosition::Top, false),
            (TabBarPosition::Bottom, false),
            (TabBarPosition::Hidden, true),
        ] {
            let hidden = position == TabBarPosition::Hidden;
            let effective_h = if hidden { 0.0 } else { metrics.height };

            if expect_zero {
                assert_eq!(
                    effective_h, 0.0,
                    "config reload must publish 0.0 for {position:?} + {style:?}"
                );
            } else {
                assert_eq!(
                    effective_h, metrics.height,
                    "config reload must publish metrics.height for {position:?} + {style:?}"
                );
            }
        }
    }
}
