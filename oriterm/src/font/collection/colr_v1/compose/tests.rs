//! Tests for tiny-skia COLR v1 compositing helpers.

use skrifa::color::{CompositeMode, Transform as ColrTransform};

use skrifa::color::Extend;

use super::super::{ClipBox, ResolvedColorStop, Rgba};
use super::brush::{
    fill_radial_direct, fill_sweep_direct, rgba_to_color, solve_radial_t, to_blend_mode,
};
use super::{intersect_masks, to_bx, to_by, transform_radius_scale, transform_rotation_degrees};

const EPS: f32 = 0.01;

// Coordinate transform

#[test]
fn identity_transform_at_origin() {
    let clip = ClipBox {
        x_min: 0.0,
        y_min: 0.0,
        x_max: 100.0,
        y_max: 100.0,
    };
    let xf = ColrTransform::default(); // identity
    let scale = 1.0;

    // Font-unit (50, 50) → bitmap (50, 50) with Y-flip.
    let bx = to_bx(50.0, 50.0, scale, &clip, &xf);
    let by = to_by(50.0, 50.0, scale, &clip, &xf);
    assert!((bx - 50.0).abs() < EPS, "bx={bx}");
    assert!((by - 50.0).abs() < EPS, "by={by}");
}

#[test]
fn identity_transform_with_offset_clip() {
    let clip = ClipBox {
        x_min: 10.0,
        y_min: -5.0,
        x_max: 110.0,
        y_max: 95.0,
    };
    let xf = ColrTransform::default();
    let scale = 1.0;

    // Font (50, 50) → bitmap (50-10, 95-50) = (40, 45).
    let bx = to_bx(50.0, 50.0, scale, &clip, &xf);
    let by = to_by(50.0, 50.0, scale, &clip, &xf);
    assert!((bx - 40.0).abs() < EPS, "bx={bx}");
    assert!((by - 45.0).abs() < EPS, "by={by}");
}

#[test]
fn scaled_transform() {
    let clip = ClipBox {
        x_min: 0.0,
        y_min: 0.0,
        x_max: 32.0,
        y_max: 32.0,
    };
    let xf = ColrTransform::default();
    // scale = 32/1000 (e.g. 32px at 1000 upem)
    let scale = 32.0 / 1000.0;

    // Font (500, 500) → bitmap (500*0.032, 32-500*0.032) = (16, 16).
    let bx = to_bx(500.0, 500.0, scale, &clip, &xf);
    let by = to_by(500.0, 500.0, scale, &clip, &xf);
    assert!((bx - 16.0).abs() < EPS, "bx={bx}");
    assert!((by - 16.0).abs() < EPS, "by={by}");
}

#[test]
fn colr_translation_transform() {
    let clip = ClipBox {
        x_min: 0.0,
        y_min: 0.0,
        x_max: 100.0,
        y_max: 100.0,
    };
    let mut xf = ColrTransform::default();
    xf.dx = 200.0;
    xf.dy = 100.0;
    let scale = 0.1; // 10% scale

    // Font (0, 0) + translate (200, 100) → pixel (20, 10)
    // bitmap_x = 0.1*(200) - 0 = 20
    // bitmap_y = 100 - 0.1*(100) = 90
    let bx = to_bx(0.0, 0.0, scale, &clip, &xf);
    let by = to_by(0.0, 0.0, scale, &clip, &xf);
    assert!((bx - 20.0).abs() < EPS, "bx={bx}");
    assert!((by - 90.0).abs() < EPS, "by={by}");
}

// rgba_to_color

#[test]
fn transparent_black_stays_transparent() {
    let c = rgba_to_color(&Rgba {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    });
    assert!((c.alpha() - 0.0).abs() < EPS);
}

#[test]
fn opaque_premultiplied_unpremultiplies() {
    // Premultiplied: r=0.5, a=0.5 → unpremultiplied: r=1.0, a=0.5.
    let c = rgba_to_color(&Rgba {
        r: 0.5,
        g: 0.0,
        b: 0.0,
        a: 0.5,
    });
    assert!((c.red() - 1.0).abs() < EPS, "red={}", c.red());
    assert!((c.alpha() - 0.5).abs() < EPS, "alpha={}", c.alpha());
}

#[test]
fn fully_opaque_passes_through() {
    let c = rgba_to_color(&Rgba {
        r: 0.3,
        g: 0.6,
        b: 0.9,
        a: 1.0,
    });
    assert!((c.red() - 0.3).abs() < EPS);
    assert!((c.green() - 0.6).abs() < EPS);
    assert!((c.blue() - 0.9).abs() < EPS);
    assert!((c.alpha() - 1.0).abs() < EPS);
}

// intersect_masks

#[test]
fn intersect_opaque_with_opaque_stays_opaque() {
    let mut mask = tiny_skia::Mask::new(2, 2).unwrap();
    mask.data_mut().fill(255);
    let parent = {
        let mut p = tiny_skia::Mask::new(2, 2).unwrap();
        p.data_mut().fill(255);
        p
    };
    intersect_masks(&mut mask, &parent);
    assert!(mask.data().iter().all(|&b| b == 255));
}

#[test]
fn intersect_with_transparent_clears() {
    let mut mask = tiny_skia::Mask::new(2, 2).unwrap();
    mask.data_mut().fill(255);
    let parent = tiny_skia::Mask::new(2, 2).unwrap(); // all zeros
    intersect_masks(&mut mask, &parent);
    assert!(mask.data().iter().all(|&b| b == 0));
}

#[test]
fn intersect_half_alpha() {
    let mut mask = tiny_skia::Mask::new(1, 1).unwrap();
    mask.data_mut()[0] = 200;
    let mut parent = tiny_skia::Mask::new(1, 1).unwrap();
    parent.data_mut()[0] = 128;
    intersect_masks(&mut mask, &parent);
    // (200 * 128) / 255 ≈ 100.
    let result = mask.data()[0];
    assert!((result as i32 - 100).abs() <= 1, "got {result}");
}

// to_blend_mode

#[test]
fn src_over_maps_correctly() {
    assert!(matches!(
        to_blend_mode(CompositeMode::SrcOver),
        tiny_skia::BlendMode::SourceOver
    ));
}

#[test]
fn multiply_maps_correctly() {
    assert!(matches!(
        to_blend_mode(CompositeMode::Multiply),
        tiny_skia::BlendMode::Multiply
    ));
}

#[test]
fn screen_maps_correctly() {
    assert!(matches!(
        to_blend_mode(CompositeMode::Screen),
        tiny_skia::BlendMode::Screen
    ));
}

// transform_radius_scale

#[test]
fn radius_scale_identity_transform() {
    let t = ColrTransform::default();
    let rs = transform_radius_scale(2.0, &t);
    // Identity det = 1, so radius_scale = scale * 1 = 2.0.
    assert!((rs - 2.0).abs() < EPS, "rs={rs}");
}

#[test]
fn radius_scale_uniform_2x() {
    let t = ColrTransform {
        xx: 2.0,
        yy: 2.0,
        ..ColrTransform::default()
    };
    let rs = transform_radius_scale(1.0, &t);
    // det = 4, sqrt(4) = 2.
    assert!((rs - 2.0).abs() < EPS, "rs={rs}");
}

#[test]
fn radius_scale_rotation_preserves_radius() {
    // 45° rotation: cos=0.707, sin=0.707, det = cos²+sin² = 1.
    let c = std::f32::consts::FRAC_PI_4.cos();
    let s = std::f32::consts::FRAC_PI_4.sin();
    let t = ColrTransform {
        xx: c,
        xy: -s,
        yx: s,
        yy: c,
        ..ColrTransform::default()
    };
    let rs = transform_radius_scale(3.0, &t);
    assert!((rs - 3.0).abs() < EPS, "rs={rs}");
}

#[test]
fn radius_scale_non_uniform() {
    // Scale X by 2, Y by 3 → det = 6, sqrt(6) ≈ 2.449.
    let t = ColrTransform {
        xx: 2.0,
        yy: 3.0,
        ..ColrTransform::default()
    };
    let rs = transform_radius_scale(1.0, &t);
    let expected = 6.0_f32.sqrt();
    assert!((rs - expected).abs() < EPS, "rs={rs}, expected={expected}");
}

// transform_rotation_degrees

#[test]
fn rotation_identity_is_zero() {
    let t = ColrTransform::default();
    let deg = transform_rotation_degrees(&t);
    assert!(deg.abs() < EPS, "deg={deg}");
}

#[test]
fn rotation_90_degrees() {
    // 90° CCW: xx=0, yx=1.
    let t = ColrTransform {
        xx: 0.0,
        xy: -1.0,
        yx: 1.0,
        yy: 0.0,
        ..ColrTransform::default()
    };
    let deg = transform_rotation_degrees(&t);
    assert!((deg - 90.0).abs() < EPS, "deg={deg}");
}

#[test]
fn rotation_45_degrees() {
    let c = std::f32::consts::FRAC_PI_4.cos();
    let s = std::f32::consts::FRAC_PI_4.sin();
    let t = ColrTransform {
        xx: c,
        xy: -s,
        yx: s,
        yy: c,
        ..ColrTransform::default()
    };
    let deg = transform_rotation_degrees(&t);
    assert!((deg - 45.0).abs() < EPS, "deg={deg}");
}

// solve_radial_t

#[test]
fn solve_radial_concentric_circles() {
    // Concentric circles: c0==c1, r0=10, r1=50. Pixel at distance 30 from
    // center → t = (30-10)/(50-10) = 0.5.
    // With concentric circles: dx=dy=0, dr=40, a_coeff = -dr² = -1600.
    // For pixel at (30, 0) from c0: sx=30, sy=0.
    // b = -2*(0 + 0 + 10*40) = -800.
    // c = 900 - 100 = 800.
    let t = solve_radial_t(-1600.0, -800.0, 800.0, 10.0, 40.0);
    assert!(t.is_some(), "expected solution");
    assert!((t.unwrap() - 0.5).abs() < EPS, "t={}", t.unwrap());
}

#[test]
fn solve_radial_no_solution_negative_radius() {
    // Force both roots to yield r(t) < 0.
    // r0=1, dr=-10 → r(t)=1-10t < 0 for t > 0.1.
    // If both roots are > 0.1, no valid solution.
    let t = solve_radial_t(1.0, -20.0, 100.0, 1.0, -10.0);
    assert!(t.is_none(), "expected None, got {t:?}");
}

#[test]
fn solve_radial_linear_case() {
    // a ≈ 0 (linear): Bt + C = 0 → t = -C/B.
    let t = solve_radial_t(0.0, -4.0, 2.0, 1.0, 1.0);
    assert!(t.is_some());
    assert!((t.unwrap() - 0.5).abs() < EPS, "t={}", t.unwrap());
}

// fill_sweep_direct — wraparound

#[test]
fn sweep_full_circle_covers_all_quadrants() {
    // 4x4 pixmap, center at (2,2), full 360° sweep, 2 stops (red→blue).
    let mut pixmap = tiny_skia::Pixmap::new(4, 4).unwrap();
    let stops = vec![
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
            offset: 1.0,
            color: Rgba {
                r: 0.0,
                g: 0.0,
                b: 1.0,
                a: 1.0,
            },
        },
    ];
    fill_sweep_direct(&mut pixmap, 2.0, 2.0, 0.0, 360.0, &stops, Extend::Pad, None);

    // Every pixel should be non-transparent (the gradient covers the full circle).
    for y in 0..4 {
        for x in 0..4 {
            if x == 2 && y == 2 {
                continue; // center pixel is degenerate (atan2(0,0))
            }
            let idx = ((y * 4 + x) * 4 + 3) as usize;
            assert!(pixmap.data()[idx] > 0, "pixel ({x},{y}) is transparent");
        }
    }
}

// fill_radial_direct — basic two-circle

#[test]
fn radial_two_circle_midrange_pixel_colored() {
    // Concentric circles at (4,4), r0=1, r1=4. Pixel at (6,4) is distance 2
    // from center — within the gradient band [r0, r1].
    let mut pixmap = tiny_skia::Pixmap::new(8, 8).unwrap();
    let stops = vec![
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
            offset: 1.0,
            color: Rgba {
                r: 0.0,
                g: 0.0,
                b: 1.0,
                a: 1.0,
            },
        },
    ];
    fill_radial_direct(
        &mut pixmap,
        4.0,
        4.0,
        1.0,
        4.0,
        4.0,
        4.0,
        &stops,
        Extend::Pad,
        None,
    );

    // Pixel (6,4) at distance 2 from center should be colored (t ≈ 0.33).
    let idx = ((4 * 8 + 6) * 4 + 3) as usize;
    assert!(
        pixmap.data()[idx] > 0,
        "midrange pixel is transparent, alpha={}",
        pixmap.data()[idx]
    );
}

#[test]
fn radial_two_circle_outside_is_padded() {
    // Large outer circle: pixels well outside should still get the end-stop color
    // due to Pad extend mode.
    let mut pixmap = tiny_skia::Pixmap::new(8, 8).unwrap();
    let stops = vec![
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
            offset: 1.0,
            color: Rgba {
                r: 0.0,
                g: 0.0,
                b: 1.0,
                a: 1.0,
            },
        },
    ];
    fill_radial_direct(
        &mut pixmap,
        4.0,
        4.0,
        0.5,
        4.0,
        4.0,
        2.0,
        &stops,
        Extend::Pad,
        None,
    );

    // Corner pixel (0,0) is far outside r1=2. With Pad, it gets end-stop (blue).
    let idx = 2; // byte index for blue channel of pixel (0,0)
    let alpha = pixmap.data()[3];
    assert!(alpha > 0, "corner pixel is transparent");
    // Blue channel should dominate.
    let b = pixmap.data()[idx];
    assert!(b > 200, "expected blue > 200, got {b}");
}
