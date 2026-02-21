//! Unit tests for sRGB-to-linear conversion.

use super::srgb_to_linear;

// IEC 61966-2-1 reference values (sRGB byte → linear float).

#[test]
fn boundary_zero() {
    assert_eq!(srgb_to_linear(0), 0.0);
}

#[test]
fn boundary_max() {
    assert_eq!(srgb_to_linear(255), 1.0);
}

#[test]
fn linear_region_threshold() {
    // sRGB 10 → 10/255 ≈ 0.03922, below 0.04045 threshold → linear path.
    let s = 10.0 / 255.0;
    let expected = s / 12.92;
    assert!((srgb_to_linear(10) - expected).abs() < 1e-7);
}

#[test]
fn gamma_region_mid_gray() {
    // sRGB 128 → linear ≈ 0.2158605 (perceptual mid-gray).
    assert!((srgb_to_linear(128) - 0.215_860_5).abs() < 1e-5);
}

#[test]
fn gamma_region_quarter() {
    // sRGB 64 → linear ≈ 0.05126946.
    assert!((srgb_to_linear(64) - 0.051_269_46).abs() < 1e-5);
}

#[test]
fn gamma_region_three_quarter() {
    // sRGB 192 → linear ≈ 0.52711513.
    assert!((srgb_to_linear(192) - 0.527_115_13).abs() < 1e-5);
}

#[test]
fn near_threshold_below() {
    // sRGB 10 (s ≈ 0.0392) is below 0.04045 → linear path.
    let s = 10.0_f32 / 255.0;
    assert!(s < 0.04045);
    assert!((srgb_to_linear(10) - s / 12.92).abs() < 1e-7);
}

#[test]
fn near_threshold_above() {
    // sRGB 11 (s ≈ 0.0431) is above 0.04045 → gamma path.
    let s = 11.0_f32 / 255.0;
    assert!(s > 0.04045);
    let expected = ((s + 0.055) / 1.055).powf(2.4);
    assert!((srgb_to_linear(11) - expected).abs() < 1e-7);
}

#[test]
fn monotonically_increasing() {
    let mut prev = srgb_to_linear(0);
    for i in 1..=255 {
        let cur = srgb_to_linear(i);
        assert!(
            cur > prev,
            "srgb_to_linear({i}) = {cur} should be > srgb_to_linear({}) = {prev}",
            i - 1,
        );
        prev = cur;
    }
}
