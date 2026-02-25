//! Tests for COLR v1 paint collection and detection.

use skrifa::GlyphId;
use skrifa::color::{Brush, ColorPainter, CompositeMode, Transform};

use super::{ClipBox, PaintCollector, ResolvedBrush, Rgba};

#[test]
fn paint_collector_solid_fill() {
    let palette = vec![
        Rgba {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        },
        Rgba {
            r: 0.0,
            g: 1.0,
            b: 0.0,
            a: 1.0,
        },
    ];
    let mut collector = PaintCollector::new(palette);

    collector.fill(Brush::Solid {
        palette_index: 0,
        alpha: 1.0,
    });

    let commands = collector.into_commands();
    assert_eq!(commands.len(), 1);
    match &commands[0] {
        super::PaintCommand::Fill(ResolvedBrush::Solid(color)) => {
            assert!((color.r - 1.0).abs() < 0.001);
            assert!(color.g.abs() < 0.001);
            assert!(color.b.abs() < 0.001);
            assert!((color.a - 1.0).abs() < 0.001);
        }
        other => panic!("expected Fill(Solid), got {other:?}"),
    }
}

#[test]
fn paint_collector_solid_with_alpha() {
    let palette = vec![Rgba {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    }];
    let mut collector = PaintCollector::new(palette);

    collector.fill(Brush::Solid {
        palette_index: 0,
        alpha: 0.5,
    });

    let commands = collector.into_commands();
    assert_eq!(commands.len(), 1);
    match &commands[0] {
        super::PaintCommand::Fill(ResolvedBrush::Solid(color)) => {
            // Premultiplied: r=1.0*0.5=0.5, a=1.0*0.5=0.5.
            assert!((color.r - 0.5).abs() < 0.001);
            assert!((color.a - 0.5).abs() < 0.001);
        }
        other => panic!("expected Fill(Solid), got {other:?}"),
    }
}

#[test]
fn paint_collector_missing_palette_index() {
    let palette = vec![Rgba {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    }];
    let mut collector = PaintCollector::new(palette);

    // Request palette index 99 which doesn't exist.
    collector.fill(Brush::Solid {
        palette_index: 99,
        alpha: 1.0,
    });

    let commands = collector.into_commands();
    assert_eq!(commands.len(), 1);
    match &commands[0] {
        super::PaintCommand::Fill(ResolvedBrush::Solid(color)) => {
            // Should return transparent black.
            assert!(color.r.abs() < 0.001);
            assert!(color.a.abs() < 0.001);
        }
        other => panic!("expected Fill(Solid), got {other:?}"),
    }
}

#[test]
fn paint_collector_fill_glyph() {
    let palette = vec![Rgba {
        r: 0.0,
        g: 0.0,
        b: 1.0,
        a: 1.0,
    }];
    let mut collector = PaintCollector::new(palette);

    collector.fill_glyph(
        GlyphId::new(42),
        None,
        Brush::Solid {
            palette_index: 0,
            alpha: 1.0,
        },
    );

    let commands = collector.into_commands();
    assert_eq!(commands.len(), 1);
    match &commands[0] {
        super::PaintCommand::FillGlyph {
            glyph_id,
            brush_transform,
            brush,
        } => {
            assert_eq!(glyph_id.to_u32(), 42);
            assert!(brush_transform.is_none());
            match brush {
                ResolvedBrush::Solid(c) => {
                    assert!((c.b - 1.0).abs() < 0.001);
                }
                _ => panic!("expected Solid brush"),
            }
        }
        other => panic!("expected FillGlyph, got {other:?}"),
    }
}

#[test]
fn paint_collector_transform_stack() {
    let palette = vec![];
    let mut collector = PaintCollector::new(palette);

    collector.push_transform(Transform {
        xx: 2.0,
        yx: 0.0,
        xy: 0.0,
        yy: 2.0,
        dx: 10.0,
        dy: 20.0,
    });
    collector.pop_transform();

    let commands = collector.into_commands();
    assert_eq!(commands.len(), 2);
    assert!(matches!(
        &commands[0],
        super::PaintCommand::PushTransform(_)
    ));
    assert!(matches!(&commands[1], super::PaintCommand::PopTransform));
}

#[test]
fn paint_collector_clip_and_layer() {
    let palette = vec![];
    let mut collector = PaintCollector::new(palette);

    collector.push_clip_glyph(GlyphId::new(10));
    collector.push_layer(CompositeMode::SrcOver);
    collector.pop_layer();
    collector.pop_clip();

    let commands = collector.into_commands();
    assert_eq!(commands.len(), 4);
    assert!(matches!(
        &commands[0],
        super::PaintCommand::PushClipGlyph(_)
    ));
    assert!(matches!(&commands[1], super::PaintCommand::PushLayer(_)));
    assert!(matches!(&commands[2], super::PaintCommand::PopLayer));
    assert!(matches!(&commands[3], super::PaintCommand::PopClip));
}

#[test]
fn clip_box_dimensions() {
    let cb = ClipBox {
        x_min: 10.0,
        y_min: -5.0,
        x_max: 60.0,
        y_max: 45.0,
    };
    assert!((cb.width() - 50.0).abs() < 0.001);
    assert!((cb.height() - 50.0).abs() < 0.001);
}

#[test]
fn rgba_premultiply() {
    let color = Rgba {
        r: 1.0,
        g: 0.5,
        b: 0.0,
        a: 0.5,
    };
    let pm = color.premultiply();
    assert!((pm.r - 0.5).abs() < 0.001);
    assert!((pm.g - 0.25).abs() < 0.001);
    assert!(pm.b.abs() < 0.001);
    assert!((pm.a - 0.5).abs() < 0.001);
}

#[test]
fn rgba_premultiply_zero_alpha() {
    // Transparent pixel: RGB channels must become zero regardless of input.
    let color = Rgba {
        r: 1.0,
        g: 0.5,
        b: 0.25,
        a: 0.0,
    };
    let pm = color.premultiply();
    assert!(pm.r.abs() < 0.001);
    assert!(pm.g.abs() < 0.001);
    assert!(pm.b.abs() < 0.001);
    assert!(pm.a.abs() < 0.001);
}

#[test]
fn clip_box_zero_area() {
    // Degenerate clip box where x_min == x_max.
    let cb = ClipBox {
        x_min: 10.0,
        y_min: 5.0,
        x_max: 10.0,
        y_max: 5.0,
    };
    assert!(cb.width().abs() < 0.001);
    assert!(cb.height().abs() < 0.001);
}

#[test]
fn clip_box_inverted_bounds() {
    // Inverted bounds (min > max) — width/height should be negative.
    let cb = ClipBox {
        x_min: 50.0,
        y_min: 50.0,
        x_max: 10.0,
        y_max: 10.0,
    };
    assert!(cb.width() < 0.0);
    assert!(cb.height() < 0.0);
}
