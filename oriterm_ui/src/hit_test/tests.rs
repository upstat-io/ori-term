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

/// Resize border wins over interactive rect when they overlap.
///
/// Close button at top-right corner must not swallow the resize zone.
#[test]
fn resize_wins_over_interactive_rect_in_corner() {
    // Close button covering the top-right corner: x 755..800, y 0..46.
    let close_button = Rect::new(755.0, 0.0, 45.0, 46.0);
    let buttons = [close_button];
    let chrome = standard_chrome(&buttons, false);

    // Point at (798, 2) — in both the close button AND the top-right resize corner.
    let corner = Point::new(798.0, 2.0);
    assert_eq!(
        hit_test(corner, &chrome),
        HitTestResult::ResizeBorder(ResizeDirection::TopRight)
    );

    // Point at (770, 20) — in the close button but NOT in a resize zone.
    let button_center = Point::new(770.0, 20.0);
    assert_eq!(hit_test(button_center, &chrome), HitTestResult::Client);
}

/// Resize border wins over interactive rect on right edge.
#[test]
fn resize_wins_over_interactive_rect_on_edge() {
    // Button covering the right edge: x 755..800, y 100..200.
    let button = Rect::new(755.0, 100.0, 45.0, 100.0);
    let buttons = [button];
    let chrome = standard_chrome(&buttons, false);

    // Point at (798, 150) — in button AND on right edge.
    let edge = Point::new(798.0, 150.0);
    assert_eq!(
        hit_test(edge, &chrome),
        HitTestResult::ResizeBorder(ResizeDirection::Right)
    );
}

/// Maximized suppresses all resize borders — corners included.
///
/// Verifies that every corner and edge returns non-resize when maximized.
#[test]
fn maximized_suppresses_all_borders() {
    let chrome = standard_chrome(&[], true);

    // All four corners — would be ResizeBorder when not maximized.
    assert_eq!(
        hit_test(Point::new(0.0, 0.0), &chrome),
        HitTestResult::Caption,
        "top-left corner should be caption when maximized"
    );
    assert_eq!(
        hit_test(Point::new(799.0, 0.0), &chrome),
        HitTestResult::Caption,
        "top-right corner should be caption when maximized"
    );
    assert_eq!(
        hit_test(Point::new(0.0, 599.0), &chrome),
        HitTestResult::Client,
        "bottom-left corner should be client when maximized"
    );
    assert_eq!(
        hit_test(Point::new(799.0, 599.0), &chrome),
        HitTestResult::Client,
        "bottom-right corner should be client when maximized"
    );

    // All four edges.
    assert_eq!(
        hit_test(Point::new(400.0, 0.0), &chrome),
        HitTestResult::Caption,
        "top edge should be caption when maximized"
    );
    assert_eq!(
        hit_test(Point::new(0.0, 300.0), &chrome),
        HitTestResult::Client,
        "left edge below caption should be client when maximized"
    );
    assert_eq!(
        hit_test(Point::new(799.0, 300.0), &chrome),
        HitTestResult::Client,
        "right edge below caption should be client when maximized"
    );
    assert_eq!(
        hit_test(Point::new(400.0, 599.0), &chrome),
        HitTestResult::Client,
        "bottom edge should be client when maximized"
    );
}

/// Interactive rect boundary: half-open interval [x, x+w) × [y, y+h).
///
/// Points on the left/top edge are inside; points on the right/bottom edge
/// are outside (matching `Rect::contains` semantics).
#[test]
fn interactive_rect_boundary_half_open() {
    let button = Rect::new(100.0, 10.0, 40.0, 30.0);
    let buttons = [button];
    let chrome = standard_chrome(&buttons, false);

    // Left edge of button (x=100) — inside.
    assert_eq!(
        hit_test(Point::new(100.0, 20.0), &chrome),
        HitTestResult::Client
    );
    // Top edge of button (y=10) — inside.
    assert_eq!(
        hit_test(Point::new(120.0, 10.0), &chrome),
        HitTestResult::Client
    );
    // Right boundary (x=140) — outside (half-open).
    assert_eq!(
        hit_test(Point::new(140.0, 20.0), &chrome),
        HitTestResult::Caption,
        "right boundary of rect should be outside (half-open)"
    );
    // Bottom boundary (y=40) — outside (half-open).
    assert_eq!(
        hit_test(Point::new(120.0, 40.0), &chrome),
        HitTestResult::Caption,
        "bottom boundary of rect should be outside (half-open)"
    );
}

/// Zero border width: no resize borders anywhere.
#[test]
fn zero_border_width_no_resize() {
    let chrome = WindowChrome {
        window_size: Size::new(800.0, 600.0),
        border_width: 0.0,
        caption_height: 46.0,
        interactive_rects: &[],
        is_maximized: false,
    };

    // Corners and edges that would normally be resize.
    assert_eq!(
        hit_test(Point::new(0.0, 0.0), &chrome),
        HitTestResult::Caption
    );
    assert_eq!(
        hit_test(Point::new(799.0, 0.0), &chrome),
        HitTestResult::Caption
    );
    assert_eq!(
        hit_test(Point::new(0.0, 599.0), &chrome),
        HitTestResult::Client
    );
    assert_eq!(
        hit_test(Point::new(400.0, 300.0), &chrome),
        HitTestResult::Client
    );
}

/// Zero caption height: no caption area — everything is client.
#[test]
fn zero_caption_height_no_caption() {
    let chrome = WindowChrome {
        window_size: Size::new(800.0, 600.0),
        border_width: 5.0,
        caption_height: 0.0,
        interactive_rects: &[],
        is_maximized: false,
    };

    // Point that would be caption with non-zero caption_height.
    assert_eq!(
        hit_test(Point::new(400.0, 20.0), &chrome),
        HitTestResult::Client,
        "no caption when caption_height is zero"
    );
    // Resize borders still work.
    assert_eq!(
        hit_test(Point::new(0.0, 300.0), &chrome),
        HitTestResult::ResizeBorder(ResizeDirection::Left)
    );
}

/// Zero-size window: everything is resize border (all points within border_width).
#[test]
fn zero_size_window() {
    let chrome = WindowChrome {
        window_size: Size::new(0.0, 0.0),
        border_width: 5.0,
        caption_height: 46.0,
        interactive_rects: &[],
        is_maximized: false,
    };

    // Origin is within border_width of all edges.
    let result = hit_test(Point::new(0.0, 0.0), &chrome);
    assert!(
        matches!(result, HitTestResult::ResizeBorder(_)),
        "zero-size window: origin should be a resize border, got {result:?}"
    );
}

/// Regression test: exact top-right pixel (width-1, 0).
///
/// This is the specific coordinate where the close button and resize corner
/// overlap — the bug that motivated the WM_NCHITTEST fix in Section 01.
/// Resize must win so the corner remains resizable.
#[test]
fn top_right_pixel_regression() {
    // Close button spanning the top-right corner.
    let close_button = Rect::new(754.0, 0.0, 46.0, 46.0);
    let buttons = [close_button];
    let chrome = standard_chrome(&buttons, false);

    // (799, 0) — the exact top-right pixel.
    let point = Point::new(799.0, 0.0);
    assert_eq!(
        hit_test(point, &chrome),
        HitTestResult::ResizeBorder(ResizeDirection::TopRight),
        "(width-1, 0) must be resize, not client (close button)"
    );

    // When maximized, the same point falls back to the interactive rect → Client.
    let chrome_max = standard_chrome(&buttons, true);
    assert_eq!(
        hit_test(point, &chrome_max),
        HitTestResult::Client,
        "(width-1, 0) maximized: close button wins (no resize borders)"
    );
}

/// Tab-bar-shaped interactive rects: tabs + buttons + controls.
///
/// Simulates the unified chrome where interactive rects include tab rects,
/// new-tab button, dropdown button, and 3 control buttons.
#[test]
fn tab_bar_shaped_interactive_rects() {
    // 3 tabs at 150px each starting at x=16, plus new-tab, dropdown, 3 controls.
    let tab_width = 150.0;
    let caption_h = 46.0;
    let mut rects = Vec::new();

    // Tab rects.
    for i in 0..3 {
        rects.push(Rect::new(
            16.0 + i as f32 * tab_width,
            0.0,
            tab_width,
            caption_h,
        ));
    }
    // New-tab button.
    rects.push(Rect::new(466.0, 0.0, 38.0, caption_h));
    // Dropdown button.
    rects.push(Rect::new(504.0, 0.0, 30.0, caption_h));
    // 3 control buttons (right-aligned).
    rects.push(Rect::new(662.0, 0.0, 46.0, caption_h));
    rects.push(Rect::new(708.0, 0.0, 46.0, caption_h));
    rects.push(Rect::new(754.0, 0.0, 46.0, caption_h));

    let chrome = standard_chrome(&rects, false);

    // Click on tab 1 body → Client (interactive rect).
    let tab1_center = Point::new(16.0 + tab_width * 1.5, 20.0);
    assert_eq!(hit_test(tab1_center, &chrome), HitTestResult::Client);

    // Click between tabs and controls (gap) → Caption (draggable).
    let gap = Point::new(600.0, 20.0);
    assert_eq!(hit_test(gap, &chrome), HitTestResult::Caption);

    // Click on a control button → Client.
    let control = Point::new(730.0, 20.0);
    assert_eq!(hit_test(control, &chrome), HitTestResult::Client);

    // Click below the tab bar → Client (grid area).
    let below = Point::new(400.0, 300.0);
    assert_eq!(hit_test(below, &chrome), HitTestResult::Client);
}
