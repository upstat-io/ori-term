//! Tests for tiny-skia COLR v1 compositing helpers.

use skrifa::color::{CompositeMode, Transform as ColrTransform};

use super::super::{ClipBox, Rgba};
use super::brush::{rgba_to_color, to_blend_mode};
use super::{intersect_masks, to_bx, to_by};

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
