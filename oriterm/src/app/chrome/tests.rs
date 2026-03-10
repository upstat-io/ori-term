//! Unit tests for chrome geometry helpers.

use super::grid_origin_y;

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
