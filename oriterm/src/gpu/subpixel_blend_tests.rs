//! Pure-math tests for subpixel blend formula.
//!
//! Verifies the CPU-side mirror of the `subpixel_fg.wgsl` per-channel blend
//! logic. No GPU adapter needed.
//!
//! The mirror operates on STRAIGHT `[0..1]` color inputs — it mirrors the
//! SHADER formula, which is unchanged in convention (the shader still owns the
//! single premultiply by `bg.a`). It does NOT exercise
//! `instance_writer::rgb_to_floats`, so it CANNOT catch an `rgb_to_floats`
//! double-premultiply; the GPU-apex pilot
//! (`visual_regression::spec_chain::pilots::cell_alpha_translucent_bg`) owns
//! that end-to-end correctness.

/// Mirror of the WGSL `subpixel_fg.wgsl` per-channel blend formula.
///
/// `fg`/`bg` are `[r, g, b, a]` in 0..1. `mask` is `[r, g, b]` coverage.
/// Returns premultiplied RGBA output.
fn subpixel_blend(fg: [f32; 4], bg: [f32; 4], mask: [f32; 3]) -> [f32; 4] {
    if bg[3] > 0.001 {
        let dim = fg[3];
        let cov_r = mask[0] * dim;
        let cov_g = mask[1] * dim;
        let cov_b = mask[2] * dim;
        let coverage = cov_r.max(cov_g).max(cov_b);
        if coverage < 0.001 {
            return [0.0, 0.0, 0.0, 0.0];
        }
        // Split on bg.a (mirrors subpixel_fg.wgsl known-bg branch). The cell
        // bg is drawn FIRST as a separate bg quad, so the framebuffer already
        // holds bg-over-dest before the glyph draw:
        // - OPAQUE bg (bg.a >= 0.999): composite-and-overwrite (out_a=1.0).
        //   The opaque result replaces the framebuffer — true per-channel AA,
        //   no double-apply. At bg.a=1.0 this is mix(bg, fg, cov), alpha 1.0.
        // - TRANSLUCENT bg: glyph-only premultiplied — do NOT re-include bg
        //   (it is already in the framebuffer via the bg quad). Re-including it
        //   here double-applied the bg at glyph edges (the corrected bug).
        if bg[3] >= 0.999 {
            let out_r = fg[0] * cov_r + bg[0] * (1.0 - cov_r);
            let out_g = fg[1] * cov_g + bg[1] * (1.0 - cov_g);
            let out_b = fg[2] * cov_b + bg[2] * (1.0 - cov_b);
            return [out_r, out_g, out_b, 1.0];
        }
        let out_r = fg[0] * cov_r;
        let out_g = fg[1] * cov_g;
        let out_b = fg[2] * cov_b;
        return [out_r, out_g, out_b, coverage];
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

/// An OPAQUE, fully-covering glyph over a translucent known bg (bg.a=0.5) must
/// OCCLUDE. On the glyph-only translucent path, coverage=1 gives out_rgb=fg*1=fg
/// and out_a=coverage=1.0 → the glyph still fully occludes [1,1,1,1]. The bg is
/// already in the framebuffer (drawn as a separate bg quad), so this glyph-only
/// output correctly replaces it where the glyph covers.
#[test]
fn subpixel_blend_opaque_glyph_over_translucent_bg_occludes() {
    let fg = [1.0, 1.0, 1.0, 1.0];
    let bg = [0.2, 0.4, 0.6, 0.5];
    let out = subpixel_blend(fg, bg, [1.0, 1.0, 1.0]);
    // coverage=1 → glyph fully occludes: premul colour == fg, alpha == 1.0.
    assert!((out[0] - 1.0).abs() < 1e-6, "R = fg.r (glyph occludes)");
    assert!((out[1] - 1.0).abs() < 1e-6, "G = fg.g (glyph occludes)");
    assert!((out[2] - 1.0).abs() < 1e-6, "B = fg.b (glyph occludes)");
    assert!(
        (out[3] - 1.0).abs() < 1e-6,
        "A = 1.0 — opaque glyph occludes the translucent bg"
    );
    // Rejection guard: reject the bg.a-cap (0.5).
    assert!(
        (out[3] - 0.5).abs() > 1e-3,
        "A must NOT be capped at bg.a=0.5"
    );
}

/// Partial glyph coverage over a translucent bg emits GLYPH-ONLY premultiplied —
/// the bg is NOT re-included here because it is already in the framebuffer (drawn
/// as a separate bg quad). out_a = coverage, out_r = fg.r*cov. Re-including bg
/// (the prior Porter-Duff `fg*cov + bg*bg.a*(1-cov)` form) double-applied the bg
/// at glyph edges — the corrected double-bg-apply bug.
#[test]
fn subpixel_blend_partial_glyph_over_translucent_bg_glyph_only() {
    let fg = [1.0, 1.0, 1.0, 1.0];
    let bg = [0.2, 0.4, 0.6, 0.5];
    let out = subpixel_blend(fg, bg, [0.5, 0.5, 0.5]);
    // Glyph-only: out_a = coverage = 0.5 (NOT the prior 0.75 with a bg term).
    assert!(
        (out[3] - 0.5).abs() < 1e-6,
        "A = coverage = 0.5 (glyph-only)"
    );
    // out_r = fg.r*cov = 0.5 (NOT the prior 0.55 with a bg term).
    assert!(
        (out[0] - 0.5).abs() < 1e-6,
        "R = fg.r*cov = 0.5 (glyph-only premul; bg comes from the bg quad)"
    );
    // Reject any bg term in out_r. The double-applying Porter-Duff form gave
    // R = 1*0.5 + 0.2*0.5*0.5 = 0.55; out_a = 0.5 + 0.5*0.5 = 0.75.
    assert!(
        (out[0] - 0.55).abs() > 1e-3,
        "R must NOT include a bg term (reject the double-apply value 0.55)"
    );
    assert!(
        (out[3] - 0.75).abs() > 1e-3,
        "A must NOT include a bg term (reject the double-apply value 0.75)"
    );
}
