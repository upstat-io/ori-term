//! Unit tests for chrome geometry helpers.

use super::grid_origin_y;

// ── grid_origin_y: integer-pixel guarantee ──

#[test]
fn origin_integer_at_100_percent_scale() {
    // 82.0 * 1.0 = 82.0 — already integer.
    let y = grid_origin_y(82.0, 1.0);
    assert_eq!(y, 82.0);
    assert_eq!(y.fract(), 0.0);
}

#[test]
fn origin_integer_at_125_percent_scale() {
    // 82.0 * 1.25 = 102.5 — fractional without rounding.
    let y = grid_origin_y(82.0, 1.25);
    assert_eq!(y, 103.0);
    assert_eq!(y.fract(), 0.0, "125% DPI must produce integer origin");
}

#[test]
fn origin_integer_at_150_percent_scale() {
    // 82.0 * 1.5 = 123.0 — already integer.
    let y = grid_origin_y(82.0, 1.5);
    assert_eq!(y, 123.0);
    assert_eq!(y.fract(), 0.0);
}

#[test]
fn origin_integer_at_175_percent_scale() {
    // 82.0 * 1.75 = 143.5 — fractional without rounding.
    let y = grid_origin_y(82.0, 1.75);
    assert_eq!(y, 144.0);
    assert_eq!(y.fract(), 0.0, "175% DPI must produce integer origin");
}

#[test]
fn origin_integer_at_200_percent_scale() {
    // 82.0 * 2.0 = 164.0 — already integer.
    let y = grid_origin_y(82.0, 2.0);
    assert_eq!(y, 164.0);
    assert_eq!(y.fract(), 0.0);
}

#[test]
fn origin_integer_at_225_percent_scale() {
    // 82.0 * 2.25 = 184.5 — fractional without rounding.
    let y = grid_origin_y(82.0, 2.25);
    assert_eq!(y, 185.0);
    assert_eq!(y.fract(), 0.0, "225% DPI must produce integer origin");
}

#[test]
fn origin_zero_chrome() {
    // No chrome — origin should be 0 at any scale.
    assert_eq!(grid_origin_y(0.0, 1.25), 0.0);
    assert_eq!(grid_origin_y(0.0, 1.75), 0.0);
}

#[test]
fn origin_caption_only_no_tab_bar() {
    // 36.0 * 1.25 = 45.0 — integer (no tab bar, the pre-bug state).
    let y = grid_origin_y(36.0, 1.25);
    assert_eq!(y, 45.0);
    assert_eq!(y.fract(), 0.0);
}

/// Exhaustive check: all common Windows DPI scale factors produce integer origins.
#[test]
fn origin_integer_for_all_common_dpi_scales() {
    let chrome_height = 82.0; // caption (36) + tab bar (46)
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
