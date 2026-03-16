//! Pure-math tests for subpixel blend formula.
//!
//! Verifies the CPU-side mirror of the `subpixel_fg.wgsl` per-channel blend
//! logic. No GPU adapter needed.

/// Mirror of the WGSL `subpixel_fg.wgsl` per-channel blend formula.
///
/// `fg`/`bg` are `[r, g, b, a]` in 0..1. `mask` is `[r, g, b]` coverage.
/// Returns premultiplied RGBA output.
fn subpixel_blend(fg: [f32; 4], bg: [f32; 4], mask: [f32; 3]) -> [f32; 4] {
    fn mix(a: f32, b: f32, t: f32) -> f32 {
        a * (1.0 - t) + b * t
    }
    if bg[3] > 0.001 {
        let dim = fg[3];
        let r = mix(bg[0], fg[0], mask[0] * dim);
        let g = mix(bg[1], fg[1], mask[1] * dim);
        let b = mix(bg[2], fg[2], mask[2] * dim);
        let coverage = mask[0].max(mask[1]).max(mask[2]) * dim;
        if coverage < 0.001 {
            return [0.0, 0.0, 0.0, 0.0];
        }
        return [r, g, b, 1.0];
    }
    let coverage = mask[0].max(mask[1]).max(mask[2]);
    let a = coverage * fg[3];
    [fg[0] * a, fg[1] * a, fg[2] * a, a]
}

#[test]
fn subpixel_blend_full_mask_returns_fg() {
    let fg = [1.0, 0.5, 0.0, 1.0];
    let bg = [0.0, 0.0, 0.0, 1.0];
    let out = subpixel_blend(fg, bg, [1.0, 1.0, 1.0]);
    assert!((out[0] - 1.0).abs() < 1e-6, "R should be fg.r");
    assert!((out[1] - 0.5).abs() < 1e-6, "G should be fg.g");
    assert!((out[2] - 0.0).abs() < 1e-6, "B should be fg.b");
    assert!((out[3] - 1.0).abs() < 1e-6, "A should be 1.0");
}

#[test]
fn subpixel_blend_zero_mask_known_bg_returns_transparent() {
    let fg = [1.0, 1.0, 1.0, 1.0];
    let bg = [0.2, 0.4, 0.6, 1.0];
    let out = subpixel_blend(fg, bg, [0.0, 0.0, 0.0]);
    assert!((out[0]).abs() < 1e-6, "R should be 0 (transparent)");
    assert!((out[1]).abs() < 1e-6, "G should be 0 (transparent)");
    assert!((out[2]).abs() < 1e-6, "B should be 0 (transparent)");
    assert!((out[3]).abs() < 1e-6, "A should be 0 (transparent)");
}

#[test]
fn subpixel_blend_partial_mask_interpolates() {
    let fg = [1.0, 1.0, 1.0, 1.0];
    let bg = [0.0, 0.0, 0.0, 1.0];
    let out = subpixel_blend(fg, bg, [0.5, 0.5, 0.5]);
    assert!((out[0] - 0.5).abs() < 1e-6, "R should be 0.5");
    assert!((out[1] - 0.5).abs() < 1e-6, "G should be 0.5");
    assert!((out[2] - 0.5).abs() < 1e-6, "B should be 0.5");
    assert!((out[3] - 1.0).abs() < 1e-6, "A should be 1.0");
}

#[test]
fn subpixel_blend_per_channel_independence() {
    let fg = [1.0, 0.5, 0.8, 1.0];
    let bg = [0.0, 1.0, 0.0, 1.0];
    let out = subpixel_blend(fg, bg, [1.0, 0.0, 0.5]);
    assert!((out[0] - 1.0).abs() < 1e-6, "R: mask=1 → fg.r");
    assert!((out[1] - 1.0).abs() < 1e-6, "G: mask=0 → bg.g");
    assert!((out[2] - 0.4).abs() < 1e-6, "B: mask=0.5 → midpoint");
    assert!((out[3] - 1.0).abs() < 1e-6, "A: max channel coverage");
}

#[test]
fn subpixel_blend_semitransparent_fg() {
    let fg = [1.0, 1.0, 1.0, 0.5];
    let bg = [0.0, 0.0, 0.0, 0.0];
    let out = subpixel_blend(fg, bg, [1.0, 1.0, 1.0]);
    assert!((out[0] - 0.5).abs() < 1e-6);
    assert!((out[1] - 0.5).abs() < 1e-6);
    assert!((out[2] - 0.5).abs() < 1e-6);
    assert!((out[3] - 0.5).abs() < 1e-6);
}

#[test]
fn subpixel_blend_unknown_bg_falls_back_to_grayscale() {
    let fg = [1.0, 0.5, 0.25, 1.0];
    let bg = [0.0, 0.0, 0.0, 0.0];
    let out = subpixel_blend(fg, bg, [1.0, 0.0, 0.5]);
    assert!((out[0] - 1.0).abs() < 1e-6, "R should be fg.r * coverage");
    assert!((out[1] - 0.5).abs() < 1e-6, "G should be fg.g * coverage");
    assert!((out[2] - 0.25).abs() < 1e-6, "B should be fg.b * coverage");
    assert!(
        (out[3] - 1.0).abs() < 1e-6,
        "A should be grayscale coverage"
    );
}

#[test]
fn subpixel_blend_known_bg_dim_reduces_coverage() {
    let fg = [1.0, 1.0, 1.0, 0.5];
    let bg = [0.0, 0.0, 0.0, 1.0];
    let out = subpixel_blend(fg, bg, [1.0, 1.0, 1.0]);
    assert!((out[0] - 0.5).abs() < 1e-6, "R should be 0.5 (dimmed)");
    assert!((out[1] - 0.5).abs() < 1e-6, "G should be 0.5 (dimmed)");
    assert!((out[2] - 0.5).abs() < 1e-6, "B should be 0.5 (dimmed)");
    assert!((out[3] - 1.0).abs() < 1e-6, "A should be 1.0 (opaque)");
}

#[test]
fn subpixel_blend_known_bg_zero_coverage_transparent() {
    let fg = [0.8, 0.2, 0.5, 1.0];
    let bg = [0.1, 0.9, 0.3, 1.0];
    let out = subpixel_blend(fg, bg, [0.0, 0.0, 0.0]);
    assert_eq!(out, [0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn subpixel_blend_known_bg_full_dim_returns_transparent() {
    let fg = [1.0, 1.0, 1.0, 0.0];
    let bg = [0.5, 0.5, 0.5, 1.0];
    let out = subpixel_blend(fg, bg, [1.0, 1.0, 1.0]);
    assert_eq!(out, [0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn subpixel_blend_known_bg_below_epsilon_transparent() {
    let fg = [1.0, 1.0, 1.0, 1.0];
    let bg = [0.5, 0.5, 0.5, 1.0];
    let out = subpixel_blend(fg, bg, [0.0005, 0.0003, 0.0001]);
    assert_eq!(out, [0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn subpixel_blend_known_bg_above_epsilon_composited() {
    let fg = [1.0, 1.0, 1.0, 1.0];
    let bg = [0.0, 0.0, 0.0, 1.0];
    let out = subpixel_blend(fg, bg, [0.002, 0.001, 0.001]);
    assert!((out[0] - 0.002).abs() < 1e-5, "R: mix(0, 1, 0.002)");
    assert!((out[3] - 1.0).abs() < 1e-6, "A: opaque");
}
