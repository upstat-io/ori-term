//! Tests for scene snapshot formatting.

use crate::color::Color;
use crate::draw::scene::ContentMask;
use crate::draw::{BorderSides, LinePrimitive, Quad, RectStyle, Scene, TextRun};
use crate::geometry::Point;
use crate::text::ShapedText;

use super::{color_hex, scene_to_snapshot};

#[test]
fn color_hex_opaque() {
    let c = Color::hex(0xff0000);
    assert_eq!(color_hex(&c), "#ff0000");
}

#[test]
fn color_hex_with_alpha() {
    let c = Color::hex_alpha(0xff000080);
    assert_eq!(color_hex(&c), "#ff000080");
}

#[test]
fn color_hex_transparent() {
    let c = Color::hex_alpha(0x00000000);
    assert_eq!(color_hex(&c), "transparent");
}

#[test]
fn color_hex_white() {
    let c = Color::hex(0xffffff);
    assert_eq!(color_hex(&c), "#ffffff");
}

#[test]
fn empty_scene_snapshot() {
    let scene = Scene::new();
    let snap = scene_to_snapshot(&scene);
    assert!(snap.starts_with("Scene: 0 quads, 0 text, 0 lines, 0 icons, 0 images"));
}

#[test]
fn quad_with_fill() {
    let mut scene = Scene::new();
    scene.quads.push(Quad {
        bounds: crate::geometry::Rect::new(10.0, 20.0, 100.0, 50.0),
        style: RectStyle::filled(Color::hex(0x1e1e2e)),
        content_mask: ContentMask::unclipped(),
        widget_id: None,
    });
    let snap = scene_to_snapshot(&scene);
    assert!(snap.contains("Q00 (10.0, 20.0, 100.0\u{00d7}50.0) fill=#1e1e2e"));
}

#[test]
fn quad_with_radius_uniform() {
    let mut scene = Scene::new();
    let mut style = RectStyle::filled(Color::hex(0x000000));
    style.corner_radius = [6.0; 4];
    scene.quads.push(Quad {
        bounds: crate::geometry::Rect::new(0.0, 0.0, 50.0, 50.0),
        style,
        content_mask: ContentMask::unclipped(),
        widget_id: None,
    });
    let snap = scene_to_snapshot(&scene);
    assert!(snap.contains("radius=6.0"));
    // Uniform radius should NOT use array syntax.
    assert!(!snap.contains("radius=["));
}

#[test]
fn quad_with_radius_per_corner() {
    let mut scene = Scene::new();
    let mut style = RectStyle::filled(Color::hex(0x000000));
    style.corner_radius = [4.0, 4.0, 0.0, 0.0];
    scene.quads.push(Quad {
        bounds: crate::geometry::Rect::new(0.0, 0.0, 50.0, 50.0),
        style,
        content_mask: ContentMask::unclipped(),
        widget_id: None,
    });
    let snap = scene_to_snapshot(&scene);
    assert!(snap.contains("radius=[4.0,4.0,0.0,0.0]"));
}

#[test]
fn quad_with_border() {
    let mut scene = Scene::new();
    let mut style = RectStyle::filled(Color::hex(0x000000));
    style.border = BorderSides::only_bottom(2.0, Color::hex(0x45475a));
    scene.quads.push(Quad {
        bounds: crate::geometry::Rect::new(0.0, 0.0, 800.0, 36.0),
        style,
        content_mask: ContentMask::unclipped(),
        widget_id: None,
    });
    let snap = scene_to_snapshot(&scene);
    assert!(snap.contains("border-b=2.0/#45475a"));
}

#[test]
fn text_run_with_source() {
    let mut scene = Scene::new();
    let shaped = ShapedText::new(Vec::new(), 40.0, 16.0, 12.0, 0, 400).with_source("Tab 1");
    scene.text_runs.push(TextRun {
        position: Point::new(14.0, 10.0),
        shaped,
        color: Color::hex(0xcdd6f4),
        bg_hint: None,
        content_mask: ContentMask::unclipped(),
        widget_id: None,
    });
    let snap = scene_to_snapshot(&scene);
    assert!(snap.contains("T00 (14.0, 10.0) \"Tab 1\" 40.0\u{00d7}16.0 #cdd6f4"));
}

#[test]
fn text_run_without_source_shows_glyph_count() {
    let mut scene = Scene::new();
    let shaped = ShapedText::new(
        vec![crate::text::ShapedGlyph {
            glyph_id: 1,
            face_index: 0,
            synthetic: 0,
            x_advance: 8.0,
            x_offset: 0.0,
            y_offset: 0.0,
        }],
        8.0,
        16.0,
        12.0,
        0,
        400,
    );
    scene.text_runs.push(TextRun {
        position: Point::new(0.0, 0.0),
        shaped,
        color: Color::hex(0xffffff),
        bg_hint: None,
        content_mask: ContentMask::unclipped(),
        widget_id: None,
    });
    let snap = scene_to_snapshot(&scene);
    assert!(snap.contains("[1g]"));
}

#[test]
fn text_run_bold_shows_weight() {
    let mut scene = Scene::new();
    let shaped = ShapedText::new(Vec::new(), 40.0, 16.0, 12.0, 0, 700).with_source("Bold");
    scene.text_runs.push(TextRun {
        position: Point::new(0.0, 0.0),
        shaped,
        color: Color::hex(0xffffff),
        bg_hint: None,
        content_mask: ContentMask::unclipped(),
        widget_id: None,
    });
    let snap = scene_to_snapshot(&scene);
    assert!(snap.contains("w700"));
}

#[test]
fn line_primitive() {
    let mut scene = Scene::new();
    scene.lines.push(LinePrimitive {
        from: Point::new(200.0, 0.0),
        to: Point::new(200.0, 36.0),
        width: 1.0,
        color: Color::hex(0x45475a),
        content_mask: ContentMask::unclipped(),
        widget_id: None,
    });
    let snap = scene_to_snapshot(&scene);
    assert!(snap.contains("L00 (200.0, 0.0)\u{2192}(200.0, 36.0) w=1.0 #45475a"));
}

#[test]
fn snapshot_is_deterministic() {
    let mut scene = Scene::new();
    scene.quads.push(Quad {
        bounds: crate::geometry::Rect::new(0.0, 0.0, 100.0, 100.0),
        style: RectStyle::filled(Color::hex(0xaabbcc)),
        content_mask: ContentMask::unclipped(),
        widget_id: None,
    });
    let snap1 = scene_to_snapshot(&scene);
    let snap2 = scene_to_snapshot(&scene);
    assert_eq!(snap1, snap2);
}
