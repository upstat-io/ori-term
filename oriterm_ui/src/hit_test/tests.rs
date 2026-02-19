use crate::geometry::Logical;

type Point = crate::geometry::Point<Logical>;
type Size = crate::geometry::Size<Logical>;
type Rect = crate::geometry::Rect<Logical>;

use super::{HitTestResult, ResizeDirection, WindowChrome, hit_test};

/// Standard test window: 800x600, 5px border, 46px caption.
fn standard_chrome(interactive_rects: &[Rect], is_maximized: bool) -> WindowChrome<'_> {
    WindowChrome {
        window_size: Size::new(800.0, 600.0),
        border_width: 5.0,
        caption_height: 46.0,
        interactive_rects,
        is_maximized,
    }
}

#[test]
fn client_area_in_grid() {
    let chrome = standard_chrome(&[], false);
    let point = Point::new(400.0, 300.0);
    assert_eq!(hit_test(point, &chrome), HitTestResult::Client);
}

#[test]
fn caption_area_in_tab_bar() {
    let chrome = standard_chrome(&[], false);
    // Point in the caption area, past the border width.
    let point = Point::new(400.0, 20.0);
    assert_eq!(hit_test(point, &chrome), HitTestResult::Caption);
}

#[test]
fn resize_top_edge() {
    let chrome = standard_chrome(&[], false);
    let point = Point::new(400.0, 2.0);
    assert_eq!(
        hit_test(point, &chrome),
        HitTestResult::ResizeBorder(ResizeDirection::Top)
    );
}

#[test]
fn resize_bottom_edge() {
    let chrome = standard_chrome(&[], false);
    let point = Point::new(400.0, 598.0);
    assert_eq!(
        hit_test(point, &chrome),
        HitTestResult::ResizeBorder(ResizeDirection::Bottom)
    );
}

#[test]
fn resize_left_edge() {
    let chrome = standard_chrome(&[], false);
    let point = Point::new(2.0, 300.0);
    assert_eq!(
        hit_test(point, &chrome),
        HitTestResult::ResizeBorder(ResizeDirection::Left)
    );
}

#[test]
fn resize_right_edge() {
    let chrome = standard_chrome(&[], false);
    let point = Point::new(798.0, 300.0);
    assert_eq!(
        hit_test(point, &chrome),
        HitTestResult::ResizeBorder(ResizeDirection::Right)
    );
}

#[test]
fn resize_top_left_corner() {
    let chrome = standard_chrome(&[], false);
    let point = Point::new(2.0, 2.0);
    assert_eq!(
        hit_test(point, &chrome),
        HitTestResult::ResizeBorder(ResizeDirection::TopLeft)
    );
}

#[test]
fn resize_top_right_corner() {
    let chrome = standard_chrome(&[], false);
    let point = Point::new(798.0, 2.0);
    assert_eq!(
        hit_test(point, &chrome),
        HitTestResult::ResizeBorder(ResizeDirection::TopRight)
    );
}

#[test]
fn resize_bottom_left_corner() {
    let chrome = standard_chrome(&[], false);
    let point = Point::new(2.0, 598.0);
    assert_eq!(
        hit_test(point, &chrome),
        HitTestResult::ResizeBorder(ResizeDirection::BottomLeft)
    );
}

#[test]
fn resize_bottom_right_corner() {
    let chrome = standard_chrome(&[], false);
    let point = Point::new(798.0, 598.0);
    assert_eq!(
        hit_test(point, &chrome),
        HitTestResult::ResizeBorder(ResizeDirection::BottomRight)
    );
}

#[test]
fn corner_priority_over_edge() {
    let chrome = standard_chrome(&[], false);
    // Point at (0, 0) — both on left edge and top edge.
    // Corner should win.
    let point = Point::new(0.0, 0.0);
    assert_eq!(
        hit_test(point, &chrome),
        HitTestResult::ResizeBorder(ResizeDirection::TopLeft)
    );
}

#[test]
fn maximized_suppresses_resize_borders() {
    let chrome = standard_chrome(&[], true);
    // Top edge when not maximized would be resize.
    let point = Point::new(400.0, 2.0);
    // When maximized, resize borders are suppressed -> caption.
    assert_eq!(hit_test(point, &chrome), HitTestResult::Caption);
}

#[test]
fn maximized_no_resize_on_edges() {
    let chrome = standard_chrome(&[], true);
    // Bottom edge.
    let point = Point::new(400.0, 598.0);
    assert_eq!(hit_test(point, &chrome), HitTestResult::Client);
}

#[test]
fn interactive_rect_in_caption_returns_client() {
    // A close button rect in the caption area.
    let button = Rect::new(750.0, 5.0, 40.0, 36.0);
    let buttons = [button];
    let chrome = standard_chrome(&buttons, false);
    let point = Point::new(770.0, 20.0);
    assert_eq!(hit_test(point, &chrome), HitTestResult::Client);
}

#[test]
fn interactive_rect_outside_point_is_caption() {
    // A close button rect that does NOT contain the point.
    let button = Rect::new(750.0, 5.0, 40.0, 36.0);
    let buttons = [button];
    let chrome = standard_chrome(&buttons, false);
    let point = Point::new(100.0, 20.0);
    assert_eq!(hit_test(point, &chrome), HitTestResult::Caption);
}

#[test]
fn point_on_border_width_boundary() {
    let chrome = standard_chrome(&[], false);
    // Exactly at x = border_width (5.0) — should NOT be on the left edge.
    let point = Point::new(5.0, 300.0);
    assert_eq!(hit_test(point, &chrome), HitTestResult::Client);
}

#[test]
fn point_just_inside_border() {
    let chrome = standard_chrome(&[], false);
    // x = 4.9 — just inside the left border.
    let point = Point::new(4.9, 300.0);
    assert_eq!(
        hit_test(point, &chrome),
        HitTestResult::ResizeBorder(ResizeDirection::Left)
    );
}

#[test]
fn multiple_interactive_rects() {
    let buttons = vec![
        Rect::new(700.0, 5.0, 30.0, 36.0),
        Rect::new(735.0, 5.0, 30.0, 36.0),
        Rect::new(770.0, 5.0, 25.0, 36.0),
    ];
    let chrome = standard_chrome(&buttons, false);
    // Click on second button.
    let point = Point::new(750.0, 20.0);
    assert_eq!(hit_test(point, &chrome), HitTestResult::Client);
}
