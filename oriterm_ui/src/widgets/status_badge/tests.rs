use crate::color::Color;
use crate::draw::Scene;
use crate::geometry::{Insets, Point};
use crate::widgets::tests::MockMeasurer;

use super::{StatusBadge, StatusBadgeStyle};

fn test_style() -> StatusBadgeStyle {
    StatusBadgeStyle {
        bg: Color::rgba(0.1, 0.1, 0.1, 0.9),
        fg: Color::WHITE,
        font_size: 11.0,
        corner_radius: 4.0,
        padding: Insets::vh(5.0, 8.0),
    }
}

#[test]
fn measure_includes_padding() {
    let m = MockMeasurer::STANDARD;
    let badge = StatusBadge::new("hello").with_style(test_style());
    let (w, h) = badge.measure(&m, f32::INFINITY);

    // "hello" = 5 chars * 8px = 40px text + 16px horizontal padding = 56px.
    assert_eq!(w, 56.0);
    // 16px line height + 10px vertical padding = 26px.
    assert_eq!(h, 26.0);
}

#[test]
fn measure_empty_text() {
    let m = MockMeasurer::STANDARD;
    let badge = StatusBadge::new("").with_style(test_style());
    let (w, h) = badge.measure(&m, f32::INFINITY);

    // No text, just padding.
    assert_eq!(w, 16.0);
    assert_eq!(h, 26.0);
}

#[test]
fn draw_produces_layer_rect_text_commands() {
    let m = MockMeasurer::STANDARD;
    let badge = StatusBadge::new("test").with_style(test_style());
    let mut scene = Scene::new();
    let pos = Point::new(100.0, 50.0);

    let bounds = badge.draw(&mut scene, &m, pos, f32::INFINITY);

    // Badge bounds: 4 chars * 8px + 16px padding = 48px wide.
    assert_eq!(bounds.x(), 100.0);
    assert_eq!(bounds.y(), 50.0);
    assert_eq!(bounds.width(), 48.0);
    assert_eq!(bounds.height(), 26.0);

    // Scene produces: 1 quad (background) + 1 text run (label).
    assert_eq!(scene.quads().len(), 1);
    assert_eq!(scene.text_runs().len(), 1);
}

#[test]
fn draw_text_position_respects_padding() {
    let m = MockMeasurer::STANDARD;
    let style = StatusBadgeStyle {
        padding: Insets::vh(10.0, 20.0),
        ..test_style()
    };
    let badge = StatusBadge::new("x").with_style(style);
    let mut scene = Scene::new();
    let pos = Point::new(0.0, 0.0);

    let _ = badge.draw(&mut scene, &m, pos, f32::INFINITY);

    // Text run should be at (padding_left, padding_top) = (20, 10).
    let text = &scene.text_runs()[0];
    assert_eq!(text.position.x, 20.0);
    assert_eq!(text.position.y, 10.0);
}

#[test]
fn default_style_matches_dark_theme() {
    let style = StatusBadgeStyle::default();
    let theme = crate::theme::UiTheme::dark();

    assert_eq!(style.font_size, theme.font_size_small);
    assert_eq!(style.corner_radius, theme.corner_radius);
    assert_eq!(style.fg, theme.fg_primary);
}
