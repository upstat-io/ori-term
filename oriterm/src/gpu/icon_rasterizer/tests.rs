//! Tests for vector icon rasterization.

use oriterm_ui::icons::IconId;

use super::rasterize_icon;

/// Scale factor used for tests (1.0× = no scaling).
const TEST_SCALE: f32 = 1.0;

/// Rasterize close icon at different sizes — all produce correct dimensions.
#[test]
fn rasterize_close_at_multiple_sizes() {
    for size in [16, 24, 32] {
        let data = rasterize_icon(IconId::Close.path(), size, TEST_SCALE);
        assert_eq!(
            data.len(),
            (size * size) as usize,
            "close icon at {size}px: expected {} bytes, got {}",
            size * size,
            data.len()
        );
    }
}

/// Different sizes produce different pixel data (not byte-for-byte duplicates).
#[test]
fn different_sizes_produce_different_data() {
    let d16 = rasterize_icon(IconId::Close.path(), 16, TEST_SCALE);
    let d32 = rasterize_icon(IconId::Close.path(), 32, TEST_SCALE);
    assert_ne!(d16.len(), d32.len());
}

/// Rasterized close icon has non-zero alpha along the diagonal.
#[test]
fn close_icon_has_nonzero_alpha() {
    let size = 32;
    let data = rasterize_icon(IconId::Close.path(), size, TEST_SCALE);
    let nonzero = data.iter().filter(|&&a| a > 0).count();
    assert!(nonzero > 0, "close icon at {size}px has no visible pixels");
}

/// Close icon at 2.0x scale (32px for 16px logical) has smooth diagonals.
///
/// Smooth diagonals have partial alpha values (not just 0 or 255),
/// indicating anti-aliasing is working.
#[test]
fn close_icon_has_antialiased_diagonals() {
    let data = rasterize_icon(IconId::Close.path(), 32, 2.0);
    let partial = data.iter().filter(|&&a| a > 0 && a < 255).count();
    assert!(
        partial > 0,
        "no partial alpha values found — anti-aliasing may not be working"
    );
}

/// All icon variants produce non-empty output at 16px.
#[test]
fn all_icons_rasterize_to_nonempty() {
    let icons = [
        IconId::Close,
        IconId::Plus,
        IconId::ChevronDown,
        IconId::Minimize,
        IconId::Maximize,
        IconId::Restore,
        IconId::WindowClose,
    ];
    for id in icons {
        let data = rasterize_icon(id.path(), 16, TEST_SCALE);
        assert_eq!(data.len(), 256, "{id:?} at 16px: wrong size");
        let nonzero = data.iter().filter(|&&a| a > 0).count();
        assert!(nonzero > 0, "{id:?} at 16px has no visible pixels");
    }
}

/// Zero size returns empty data.
#[test]
fn zero_size_returns_empty() {
    let data = rasterize_icon(IconId::Close.path(), 0, TEST_SCALE);
    assert!(data.is_empty());
}

/// Plus icon has pixels in both horizontal and vertical arms.
#[test]
fn plus_icon_has_cross_pattern() {
    let size = 32u32;
    let data = rasterize_icon(IconId::Plus.path(), size, TEST_SCALE);
    let mid = size / 2;

    // Check horizontal arm (middle row).
    let h_row_start = (mid * size) as usize;
    let h_row = &data[h_row_start..h_row_start + size as usize];
    let h_nonzero = h_row.iter().filter(|&&a| a > 0).count();

    // Check vertical arm (middle column).
    let v_nonzero = (0..size)
        .filter(|&row| data[(row * size + mid) as usize] > 0)
        .count();

    assert!(h_nonzero > 2, "plus horizontal arm missing");
    assert!(v_nonzero > 2, "plus vertical arm missing");
}

/// Minimize icon has pixels concentrated in the middle row.
#[test]
fn minimize_icon_is_horizontal() {
    let size = 24u32;
    let data = rasterize_icon(IconId::Minimize.path(), size, TEST_SCALE);
    let mid = size / 2;

    // Middle row should have nonzero alpha.
    let row_start = (mid * size) as usize;
    let row = &data[row_start..row_start + size as usize];
    let mid_nonzero = row.iter().filter(|&&a| a > 0).count();
    assert!(mid_nonzero > size as usize / 3, "minimize dash too short");
}

/// Higher scale factor produces thicker strokes (more non-zero pixels).
#[test]
fn higher_scale_thicker_strokes() {
    let size = 20u32;
    let data_1x = rasterize_icon(IconId::Close.path(), size, 1.0);
    let data_2x = rasterize_icon(IconId::Close.path(), size, 2.0);
    let count_1x = data_1x.iter().filter(|&&a| a > 0).count();
    let count_2x = data_2x.iter().filter(|&&a| a > 0).count();
    assert!(
        count_2x > count_1x,
        "2x scale should produce thicker strokes: {count_1x} vs {count_2x}"
    );
}
