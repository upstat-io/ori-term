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

/// Pins that `dcs_red_pixel_block(0, 0)` emits a valid empty DCS frame
/// (prefix + terminator) rather than degenerate bytes the sixel parser
/// would silently clamp via `!0~` → 1 sixel column.
#[test]
fn zero_dimensions_emit_empty_dcs_frame() {
    assert_eq!(dcs_red_pixel_block(0, 0), expected_empty_frame());
}

/// Pins that zero width with non-zero height still emits an empty DCS
/// frame — the zero-dimension guard applies if EITHER axis is zero.
#[test]
fn zero_width_emits_empty_dcs_frame() {
    assert_eq!(dcs_red_pixel_block(0, 22), expected_empty_frame());
}

/// Pins that zero height with non-zero width still emits an empty DCS
/// frame — the zero-dimension guard applies if EITHER axis is zero.
#[test]
fn zero_height_emits_empty_dcs_frame() {
    assert_eq!(dcs_red_pixel_block(80, 0), expected_empty_frame());
}

/// Pins the minimal viable non-empty input — `(1, 1)` emits a single
/// band with a single `!1~` sixel column.
#[test]
fn one_by_one_emits_single_band_single_col() {
    let bytes = dcs_red_pixel_block(1, 1);
    let mut expected = Vec::new();
    expected.extend_from_slice(DCS_RED_PREFIX);
    expected.extend_from_slice(b"!1~");
    expected.extend_from_slice(DCS_TERMINATOR);
    assert_eq!(bytes, expected);
}

/// Pins the typical visual-pilot scale: 80 px × 22 px at
/// `PIXELS_PER_BAND = 6` → ceil(22 / 6) = 4 bands of `!80~`
/// separated by 3 `-` band terminators. Catalog row:
/// `KG-CROSS-STACK-SIXEL-PLACEHOLDER-COEXIST`.
#[test]
fn typical_visual_pilot_scale_emits_correct_bands() {
    let bytes = dcs_red_pixel_block(80, 22);
    let mut expected = Vec::new();
    expected.extend_from_slice(DCS_RED_PREFIX);
    expected.extend_from_slice(b"!80~-!80~-!80~-!80~");
    expected.extend_from_slice(DCS_TERMINATOR);
    assert_eq!(bytes, expected);
}

/// Pins that sixel run-length encoding keeps byte count bounded for
/// large dimensions — 1000 × 100 → 17 bands of `!1000~` ≈ 7 bytes per
/// band; total stays under 250 bytes (vs ~17000 bytes for an unencoded
/// repeat-glyph form).
#[test]
fn large_dimensions_keep_byte_count_bounded_via_rle() {
    let bytes = dcs_red_pixel_block(1000, 100);
    assert!(
        bytes.len() < 250,
        "RLE should keep byte count bounded; got {}",
        bytes.len()
    );
}

/// Pins single-band shape: when `height_px == PIXELS_PER_BAND`, the
/// helper emits exactly 1 sixel band with zero `-` separators between
/// bands.
#[test]
fn single_band_emits_zero_separators() {
    let bytes = dcs_red_pixel_block(10, PIXELS_PER_BAND);
    assert_eq!(bytes.iter().filter(|&&b| b == b'-').count(), 0);
}

/// Pins multi-band shape: when `height_px = N * PIXELS_PER_BAND`, the
/// helper emits exactly N bands separated by (N - 1) `-` separators.
#[test]
fn multi_band_emits_correct_separator_count() {
    let bytes = dcs_red_pixel_block(10, 3 * PIXELS_PER_BAND);
    assert_eq!(bytes.iter().filter(|&&b| b == b'-').count(), 2);
}

/// Pins partial-band rounding: when `height_px = PIXELS_PER_BAND + 1`,
/// the helper rounds up to `ceil(7 / 6) = 2` bands (1 `-` separator).
#[test]
fn bands_round_up_for_partial_band_heights() {
    let bytes = dcs_red_pixel_block(5, PIXELS_PER_BAND + 1);
    assert_eq!(bytes.iter().filter(|&&b| b == b'-').count(), 1);
}

/// Regression guard: `dcs_n_bands_tall` byte output must stay
/// byte-identical after the `sixel_fixtures.rs` → `sixel_fixtures/`
/// directory-module split. Prevents inadvertent contract change to the
/// helper's existing call sites (`oriterm_core/tests/spec_chain/sixel/grid_integration.rs`).
#[test]
fn dcs_n_bands_tall_unchanged_by_split() {
    let bytes = dcs_n_bands_tall(4);
    let mut expected = Vec::new();
    expected.extend_from_slice(DCS_RED_PREFIX);
    expected.extend_from_slice(b"~-~-~-~");
    expected.extend_from_slice(DCS_TERMINATOR);
    assert_eq!(bytes, expected);
}

/// Regression guard: `dcs_n_cols_wide` byte output must stay
/// byte-identical after the directory-module split. Prevents
/// inadvertent contract change to the helper's existing call sites
/// (`oriterm_core/tests/spec_chain/sixel/*`, `cross_stack_handoff.rs`,
/// `cross_stack_regression.rs`, `cross_protocol_rss.rs`).
#[test]
fn dcs_n_cols_wide_unchanged_by_split() {
    let bytes = dcs_n_cols_wide(8);
    let mut expected = Vec::new();
    expected.extend_from_slice(DCS_RED_PREFIX);
    expected.extend_from_slice(b"~~~~~~~~");
    expected.extend_from_slice(DCS_TERMINATOR);
    assert_eq!(bytes, expected);
}
