//! Unit tests for `sixel_fixtures` helpers.

use super::{
    DCS_RED_PREFIX, DCS_TERMINATOR, PIXELS_PER_BAND, dcs_n_bands_tall, dcs_n_cols_wide,
    dcs_red_pixel_block,
};

fn expected_empty_frame() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(DCS_RED_PREFIX);
    v.extend_from_slice(DCS_TERMINATOR);
    v
}

#[test]
fn zero_dimensions_emit_empty_dcs_frame() {
    assert_eq!(dcs_red_pixel_block(0, 0), expected_empty_frame());
}

#[test]
fn zero_width_emits_empty_dcs_frame() {
    assert_eq!(dcs_red_pixel_block(0, 22), expected_empty_frame());
}

#[test]
fn zero_height_emits_empty_dcs_frame() {
    assert_eq!(dcs_red_pixel_block(80, 0), expected_empty_frame());
}

#[test]
fn one_by_one_emits_single_band_single_col() {
    let bytes = dcs_red_pixel_block(1, 1);
    let mut expected = Vec::new();
    expected.extend_from_slice(DCS_RED_PREFIX);
    expected.extend_from_slice(b"!1~");
    expected.extend_from_slice(DCS_TERMINATOR);
    assert_eq!(bytes, expected);
}

#[test]
fn typical_visual_pilot_scale_emits_correct_bands() {
    // 80 px wide × 22 px tall at PIXELS_PER_BAND=6 → ceil(22/6) = 4 bands.
    let bytes = dcs_red_pixel_block(80, 22);
    let mut expected = Vec::new();
    expected.extend_from_slice(DCS_RED_PREFIX);
    expected.extend_from_slice(b"!80~-!80~-!80~-!80~");
    expected.extend_from_slice(DCS_TERMINATOR);
    assert_eq!(bytes, expected);
}

#[test]
fn large_dimensions_keep_byte_count_bounded_via_rle() {
    // 1000 × 100: ceil(100/6) = 17 bands; each band ~7 bytes ("!1000~"); total < 250 bytes.
    let bytes = dcs_red_pixel_block(1000, 100);
    assert!(
        bytes.len() < 250,
        "RLE should keep byte count bounded; got {}",
        bytes.len()
    );
}

#[test]
fn single_band_emits_zero_separators() {
    // height_px == PIXELS_PER_BAND → exactly 1 band, zero `-` separators.
    let bytes = dcs_red_pixel_block(10, PIXELS_PER_BAND);
    assert_eq!(bytes.iter().filter(|&&b| b == b'-').count(), 0);
}

#[test]
fn multi_band_emits_correct_separator_count() {
    // height_px = 3 * PIXELS_PER_BAND → exactly 3 bands, 2 `-` separators.
    let bytes = dcs_red_pixel_block(10, 3 * PIXELS_PER_BAND);
    assert_eq!(bytes.iter().filter(|&&b| b == b'-').count(), 2);
}

#[test]
fn bands_round_up_for_partial_band_heights() {
    // height_px = PIXELS_PER_BAND + 1 → ceil(7/6) = 2 bands.
    let bytes = dcs_red_pixel_block(5, PIXELS_PER_BAND + 1);
    assert_eq!(bytes.iter().filter(|&&b| b == b'-').count(), 1);
}

#[test]
fn dcs_n_bands_tall_unchanged_by_split() {
    // Regression guard: the original helper output must be byte-identical
    // after the file-module → directory-module split.
    let bytes = dcs_n_bands_tall(4);
    let mut expected = Vec::new();
    expected.extend_from_slice(DCS_RED_PREFIX);
    expected.extend_from_slice(b"~-~-~-~");
    expected.extend_from_slice(DCS_TERMINATOR);
    assert_eq!(bytes, expected);
}

#[test]
fn dcs_n_cols_wide_unchanged_by_split() {
    // Regression guard: the original helper output must be byte-identical
    // after the file-module → directory-module split.
    let bytes = dcs_n_cols_wide(8);
    let mut expected = Vec::new();
    expected.extend_from_slice(DCS_RED_PREFIX);
    expected.extend_from_slice(b"~~~~~~~~");
    expected.extend_from_slice(DCS_TERMINATOR);
    assert_eq!(bytes, expected);
}
