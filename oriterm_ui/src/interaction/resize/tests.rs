//! Tests for resize geometry: hit testing, cursor mapping, and resize computation.

use winit::window::CursorIcon;

use super::{
    HitTestConfig, HitZone, ResizeEdge, ResizeRect, ResizeResult, compute_resize,
    hit_test_floating_zone, resize_cursor,
};

const CFG: HitTestConfig = HitTestConfig {
    edge_threshold: 5.0,
    corner_size: 10.0,
    title_bar_height: 24.0,
};

const RECT: ResizeRect = ResizeRect {
    x: 100.0,
    y: 100.0,
    width: 200.0,
    height: 200.0,
};

// -- Hit testing --

#[test]
fn outside_rect_returns_none() {
    assert!(hit_test_floating_zone(50.0, 50.0, &RECT, &CFG).is_none());
    assert!(hit_test_floating_zone(400.0, 200.0, &RECT, &CFG).is_none());
}

#[test]
fn top_left_corner() {
    let zone = hit_test_floating_zone(102.0, 102.0, &RECT, &CFG);
    assert_eq!(zone, Some(HitZone::Edge(ResizeEdge::TopLeft)));
}

#[test]
fn top_right_corner() {
    let zone = hit_test_floating_zone(298.0, 102.0, &RECT, &CFG);
    assert_eq!(zone, Some(HitZone::Edge(ResizeEdge::TopRight)));
}

#[test]
fn bottom_left_corner() {
    let zone = hit_test_floating_zone(102.0, 298.0, &RECT, &CFG);
    assert_eq!(zone, Some(HitZone::Edge(ResizeEdge::BottomLeft)));
}

#[test]
fn bottom_right_corner() {
    let zone = hit_test_floating_zone(298.0, 298.0, &RECT, &CFG);
    assert_eq!(zone, Some(HitZone::Edge(ResizeEdge::BottomRight)));
}

#[test]
fn left_edge() {
    // Near left edge, but not in corner zone.
    let zone = hit_test_floating_zone(102.0, 200.0, &RECT, &CFG);
    assert_eq!(zone, Some(HitZone::Edge(ResizeEdge::Left)));
}

#[test]
fn right_edge() {
    let zone = hit_test_floating_zone(298.0, 200.0, &RECT, &CFG);
    assert_eq!(zone, Some(HitZone::Edge(ResizeEdge::Right)));
}

#[test]
fn top_edge() {
    let zone = hit_test_floating_zone(200.0, 102.0, &RECT, &CFG);
    assert_eq!(zone, Some(HitZone::Edge(ResizeEdge::Top)));
}

#[test]
fn bottom_edge() {
    let zone = hit_test_floating_zone(200.0, 298.0, &RECT, &CFG);
    assert_eq!(zone, Some(HitZone::Edge(ResizeEdge::Bottom)));
}

#[test]
fn title_bar_zone() {
    // Inside rect, past edge threshold, but within title bar height.
    let zone = hit_test_floating_zone(200.0, 115.0, &RECT, &CFG);
    assert_eq!(zone, Some(HitZone::TitleBar));
}

#[test]
fn interior_zone() {
    let zone = hit_test_floating_zone(200.0, 200.0, &RECT, &CFG);
    assert_eq!(zone, Some(HitZone::Interior));
}

// -- Cursor mapping --

#[test]
fn cursor_for_each_edge() {
    assert_eq!(resize_cursor(ResizeEdge::Top), CursorIcon::NsResize);
    assert_eq!(resize_cursor(ResizeEdge::Bottom), CursorIcon::NsResize);
    assert_eq!(resize_cursor(ResizeEdge::Left), CursorIcon::EwResize);
    assert_eq!(resize_cursor(ResizeEdge::Right), CursorIcon::EwResize);
    assert_eq!(resize_cursor(ResizeEdge::TopLeft), CursorIcon::NwseResize);
    assert_eq!(
        resize_cursor(ResizeEdge::BottomRight),
        CursorIcon::NwseResize
    );
    assert_eq!(resize_cursor(ResizeEdge::TopRight), CursorIcon::NeswResize);
    assert_eq!(
        resize_cursor(ResizeEdge::BottomLeft),
        CursorIcon::NeswResize
    );
}

// -- Resize computation --

const MIN: f32 = 100.0;

fn rect(x: f32, y: f32, w: f32, h: f32) -> ResizeRect {
    ResizeRect {
        x,
        y,
        width: w,
        height: h,
    }
}

#[test]
fn resize_right_grows_width() {
    let r = compute_resize(
        &rect(0.0, 0.0, 200.0, 200.0),
        ResizeEdge::Right,
        50.0,
        0.0,
        MIN,
    );
    assert_eq!(r.width, 250.0);
    assert!(!r.needs_move);
}

#[test]
fn resize_bottom_grows_height() {
    let r = compute_resize(
        &rect(0.0, 0.0, 200.0, 200.0),
        ResizeEdge::Bottom,
        0.0,
        50.0,
        MIN,
    );
    assert_eq!(r.height, 250.0);
    assert!(!r.needs_move);
}

#[test]
fn resize_left_shifts_origin() {
    let r = compute_resize(
        &rect(100.0, 0.0, 200.0, 200.0),
        ResizeEdge::Left,
        -50.0,
        0.0,
        MIN,
    );
    assert_eq!(r.x, 50.0);
    assert_eq!(r.width, 250.0);
    assert!(r.needs_move);
}

#[test]
fn resize_top_shifts_origin() {
    let r = compute_resize(
        &rect(0.0, 100.0, 200.0, 200.0),
        ResizeEdge::Top,
        0.0,
        -50.0,
        MIN,
    );
    assert_eq!(r.y, 50.0);
    assert_eq!(r.height, 250.0);
    assert!(r.needs_move);
}

#[test]
fn resize_top_left_shifts_both() {
    let r = compute_resize(
        &rect(100.0, 100.0, 200.0, 200.0),
        ResizeEdge::TopLeft,
        -30.0,
        -40.0,
        MIN,
    );
    assert_eq!(r.x, 70.0);
    assert_eq!(r.y, 60.0);
    assert_eq!(r.width, 230.0);
    assert_eq!(r.height, 240.0);
    assert!(r.needs_move);
}

#[test]
fn resize_bottom_right_no_move() {
    let r = compute_resize(
        &rect(0.0, 0.0, 200.0, 200.0),
        ResizeEdge::BottomRight,
        30.0,
        40.0,
        MIN,
    );
    assert_eq!(
        r,
        ResizeResult {
            x: 0.0,
            y: 0.0,
            width: 230.0,
            height: 240.0,
            needs_move: false,
        }
    );
}

#[test]
fn min_size_clamping() {
    // Shrink right edge below min.
    let r = compute_resize(
        &rect(0.0, 0.0, 200.0, 200.0),
        ResizeEdge::Right,
        -150.0,
        0.0,
        MIN,
    );
    assert_eq!(r.width, MIN);

    // Shrink left edge below min — origin adjusts.
    let r = compute_resize(
        &rect(0.0, 0.0, 200.0, 200.0),
        ResizeEdge::Left,
        150.0,
        0.0,
        MIN,
    );
    assert_eq!(r.width, MIN);
    assert_eq!(r.x, 100.0); // 0 + 200 - 100
}

#[test]
fn top_right_corner_resize() {
    let r = compute_resize(
        &rect(50.0, 50.0, 200.0, 200.0),
        ResizeEdge::TopRight,
        20.0,
        -30.0,
        MIN,
    );
    assert_eq!(r.width, 220.0);
    assert_eq!(r.height, 230.0);
    assert_eq!(r.x, 50.0); // x unchanged for TopRight
    assert_eq!(r.y, 20.0); // y shifts up
    assert!(r.needs_move);
}

#[test]
fn bottom_left_corner_resize() {
    let r = compute_resize(
        &rect(50.0, 50.0, 200.0, 200.0),
        ResizeEdge::BottomLeft,
        -20.0,
        30.0,
        MIN,
    );
    assert_eq!(r.width, 220.0);
    assert_eq!(r.height, 230.0);
    assert_eq!(r.x, 30.0); // x shifts left
    assert_eq!(r.y, 50.0); // y unchanged for BottomLeft
    assert!(r.needs_move);
}
