//! Tests for CPU compositing, gradient evaluation, blending, and color sampling.

use skrifa::color::Extend;

use super::super::{ResolvedBrush, ResolvedColorStop, Rgba};
use super::{apply_extend, blend_src_over, fill_rect, lerp_rgba, linear_gradient_t, sample_stops};

const EPS: f32 = 0.001;

/// Assert two Rgba values are approximately equal.
fn assert_rgba_eq(actual: Rgba, expected: Rgba, msg: &str) {
    assert!(
        (actual.r - expected.r).abs() < EPS
            && (actual.g - expected.g).abs() < EPS
            && (actual.b - expected.b).abs() < EPS
            && (actual.a - expected.a).abs() < EPS,
        "{msg}: expected ({:.3},{:.3},{:.3},{:.3}), got ({:.3},{:.3},{:.3},{:.3})",
        expected.r,
        expected.g,
        expected.b,
        expected.a,
        actual.r,
        actual.g,
        actual.b,
        actual.a,
    );
}

// ── blend_src_over ──

#[test]
fn blend_src_over_zero_alpha_leaves_background() {
    // Premultiplied transparent source (all channels zero) over opaque pixel
    // should leave the background unchanged. In premultiplied alpha, a=0
    // implies r=g=b=0 (the only valid premultiplied encoding of transparent).
    let mut bitmap = [255u8, 200, 100, 255]; // opaque pixel
    let src = Rgba {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    blend_src_over(&mut bitmap, 1, 0, 0, src);

    assert_eq!(bitmap, [255, 200, 100, 255]);
}

#[test]
fn blend_src_over_full_alpha_replaces_background() {
    // Fully opaque red over any background should produce red.
    let mut bitmap = [0u8, 255, 0, 255]; // opaque green
    let src = Rgba {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    blend_src_over(&mut bitmap, 1, 0, 0, src);

    assert_eq!(bitmap[0], 255); // red
    assert_eq!(bitmap[1], 0); // green gone
    assert_eq!(bitmap[2], 0); // blue
    assert_eq!(bitmap[3], 255); // opaque
}

#[test]
fn blend_src_over_half_alpha() {
    // 50% opaque red over opaque black → RGB(128, 0, 0).
    let mut bitmap = [0u8, 0, 0, 255]; // opaque black
    let src = Rgba {
        r: 0.5,
        g: 0.0,
        b: 0.0,
        a: 0.5,
    };
    blend_src_over(&mut bitmap, 1, 0, 0, src);

    // src_over: out = src + dst * (1 - src.a)
    // out_r = 0.5 + 0 * 0.5 = 0.5 → 128
    // out_a = 0.5 + 1.0 * 0.5 = 1.0 → 255
    assert_eq!(bitmap[0], 128);
    assert_eq!(bitmap[3], 255);
}

// ── sample_stops ──

#[test]
fn sample_stops_empty_returns_transparent() {
    let result = sample_stops(&[], 0.5);
    assert_rgba_eq(
        result,
        Rgba {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        },
        "empty stops",
    );
}

#[test]
fn sample_stops_single_returns_that_color() {
    let stops = [ResolvedColorStop {
        offset: 0.5,
        color: Rgba {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        },
    }];
    // Any t value should return the single stop color.
    for t in [0.0, 0.25, 0.5, 0.75, 1.0] {
        let result = sample_stops(&stops, t);
        assert_rgba_eq(
            result,
            Rgba {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
            &format!("single stop at t={t}"),
        );
    }
}

#[test]
fn sample_stops_at_exact_boundaries() {
    let red = Rgba {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    let blue = Rgba {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    };
    let stops = [
        ResolvedColorStop {
            offset: 0.0,
            color: red,
        },
        ResolvedColorStop {
            offset: 1.0,
            color: blue,
        },
    ];

    // At t=0.0 → first stop (red).
    assert_rgba_eq(sample_stops(&stops, 0.0), red, "t=0.0");
    // At t=1.0 → last stop (blue).
    assert_rgba_eq(sample_stops(&stops, 1.0), blue, "t=1.0");
    // At t=0.5 → midpoint interpolation.
    let mid = sample_stops(&stops, 0.5);
    assert!((mid.r - 0.5).abs() < EPS, "midpoint red channel");
    assert!((mid.b - 0.5).abs() < EPS, "midpoint blue channel");
}

#[test]
fn sample_stops_before_first_returns_first() {
    let stops = [
        ResolvedColorStop {
            offset: 0.25,
            color: Rgba {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        },
        ResolvedColorStop {
            offset: 0.75,
            color: Rgba {
                r: 0.0,
                g: 1.0,
                b: 0.0,
                a: 1.0,
            },
        },
    ];
    // t < first stop offset → return first stop color.
    let result = sample_stops(&stops, 0.0);
    assert!((result.r - 1.0).abs() < EPS, "before first stop");
}

#[test]
fn sample_stops_after_last_returns_last() {
    let stops = [
        ResolvedColorStop {
            offset: 0.0,
            color: Rgba {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            },
        },
        ResolvedColorStop {
            offset: 0.5,
            color: Rgba {
                r: 0.0,
                g: 1.0,
                b: 0.0,
                a: 1.0,
            },
        },
    ];
    // t > last stop offset → return last stop color.
    let result = sample_stops(&stops, 0.9);
    assert!((result.g - 1.0).abs() < EPS, "after last stop");
}

// ── apply_extend ──

#[test]
fn extend_pad_clamps_to_0_1() {
    assert!((apply_extend(-0.5, Extend::Pad) - 0.0).abs() < EPS);
    assert!((apply_extend(0.5, Extend::Pad) - 0.5).abs() < EPS);
    assert!((apply_extend(1.5, Extend::Pad) - 1.0).abs() < EPS);
}

#[test]
fn extend_repeat_wraps() {
    assert!((apply_extend(0.0, Extend::Repeat) - 0.0).abs() < EPS);
    assert!((apply_extend(0.5, Extend::Repeat) - 0.5).abs() < EPS);
    assert!((apply_extend(1.3, Extend::Repeat) - 0.3).abs() < EPS);
    assert!((apply_extend(2.7, Extend::Repeat) - 0.7).abs() < EPS);
    assert!((apply_extend(-0.3, Extend::Repeat) - 0.7).abs() < EPS);
}

#[test]
fn extend_reflect_mirrors() {
    assert!((apply_extend(0.3, Extend::Reflect) - 0.3).abs() < EPS);
    assert!((apply_extend(1.3, Extend::Reflect) - 0.7).abs() < EPS);
    assert!((apply_extend(2.3, Extend::Reflect) - 0.3).abs() < EPS);
    // Negative values.
    assert!((apply_extend(-0.3, Extend::Reflect) - 0.3).abs() < EPS);
}

// ── linear_gradient_t ──

#[test]
fn linear_gradient_degenerate_line_returns_zero() {
    // p0 == p1 (zero-length line) should return 0.0, not NaN.
    let t = linear_gradient_t([5.0, 5.0], [5.0, 5.0], 10.0, 10.0);
    assert!((t - 0.0).abs() < EPS);
    assert!(!t.is_nan());
}

#[test]
fn linear_gradient_horizontal() {
    // Horizontal gradient from x=0 to x=100. Point at x=50 → t=0.5.
    let t = linear_gradient_t([0.0, 0.0], [100.0, 0.0], 50.0, 0.0);
    assert!((t - 0.5).abs() < EPS);
}

// ── lerp_rgba ──

#[test]
fn lerp_rgba_at_zero_returns_a() {
    let a = Rgba {
        r: 1.0,
        g: 0.0,
        b: 0.5,
        a: 0.8,
    };
    let b = Rgba {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 0.2,
    };
    assert_rgba_eq(lerp_rgba(a, b, 0.0), a, "lerp t=0");
}

#[test]
fn lerp_rgba_at_one_returns_b() {
    let a = Rgba {
        r: 1.0,
        g: 0.0,
        b: 0.5,
        a: 0.8,
    };
    let b = Rgba {
        r: 0.0,
        g: 1.0,
        b: 0.0,
        a: 0.2,
    };
    assert_rgba_eq(lerp_rgba(a, b, 1.0), b, "lerp t=1");
}

// ── fill_rect ──

#[test]
fn fill_rect_solid_fills_entire_bitmap() {
    let mut bitmap = vec![0u8; 2 * 2 * 4]; // 2x2 RGBA
    let brush = ResolvedBrush::Solid(Rgba {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    });
    fill_rect(&mut bitmap, 2, 2, &brush);

    // All 4 pixels should be opaque red.
    for pixel in bitmap.chunks_exact(4) {
        assert_eq!(pixel, [255, 0, 0, 255]);
    }
}

// ── Radial gradient degenerate ──

#[test]
fn radial_gradient_coincident_circles_returns_zero() {
    use super::radial_gradient_t;

    // Both circles are identical — degenerate case.
    let t = radial_gradient_t([50.0, 50.0], 10.0, [50.0, 50.0], 10.0, 60.0, 60.0);
    // Should not be NaN or infinity.
    assert!(t.is_finite(), "coincident circles should produce finite t");
}

// ── Sweep gradient 360 degrees ──

#[test]
fn sweep_gradient_full_circle() {
    use super::sweep_gradient_t;

    // Full 360-degree sweep. Point at 90 degrees should be t=0.25.
    let center = [50.0, 50.0];
    let t = sweep_gradient_t(center, 0.0, 360.0, 100.0, 50.0); // right of center → 0°
    assert!((t - 0.0).abs() < 0.05, "0° should be ~0.0, got {t}");

    let t = sweep_gradient_t(center, 0.0, 360.0, 50.0, 0.0); // top → 90°
    assert!((t - 0.25).abs() < 0.05, "90° should be ~0.25, got {t}");
}
