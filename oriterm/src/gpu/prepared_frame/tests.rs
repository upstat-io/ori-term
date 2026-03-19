//! Unit tests for prepared frame.

use oriterm_core::Rgb;

use super::PreparedFrame;
use crate::gpu::frame_input::ViewportSize;
use crate::gpu::instance_writer::ScreenRect;

const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };
const WHITE: Rgb = Rgb {
    r: 255,
    g: 255,
    b: 255,
};

/// Default test viewport (not meaningful for these tests).
const VP: ViewportSize = ViewportSize {
    width: 640,
    height: 480,
};

// --- Construction ---

#[test]
fn new_frame_is_empty() {
    let frame = PreparedFrame::new(VP, BLACK, 1.0);
    assert!(frame.is_empty());
    assert_eq!(frame.total_instances(), 0);
}

#[test]
fn with_capacity_starts_empty() {
    let frame = PreparedFrame::with_capacity(VP, 80, 24, BLACK, 1.0);
    assert!(frame.is_empty());
    assert_eq!(frame.total_instances(), 0);
}

// --- Clear color ---

#[test]
fn clear_color_opaque_black() {
    let frame = PreparedFrame::new(VP, BLACK, 1.0);
    assert_eq!(frame.clear_color, [0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn clear_color_opaque_white() {
    let frame = PreparedFrame::new(VP, WHITE, 1.0);
    assert_eq!(frame.clear_color, [1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn clear_color_half_transparent() {
    let frame = PreparedFrame::new(VP, WHITE, 0.5);
    // Premultiplied: each channel * 0.5.
    assert_eq!(frame.clear_color, [0.5, 0.5, 0.5, 0.5]);
}

#[test]
fn clear_color_fully_transparent() {
    let frame = PreparedFrame::new(VP, WHITE, 0.0);
    assert_eq!(frame.clear_color, [0.0, 0.0, 0.0, 0.0]);
}

#[test]
fn set_clear_color_updates() {
    let mut frame = PreparedFrame::new(VP, BLACK, 1.0);
    frame.set_clear_color(WHITE, 0.5);
    assert_eq!(frame.clear_color, [0.5, 0.5, 0.5, 0.5]);
}

// --- Lifecycle ---

#[test]
fn populate_and_count() {
    let mut frame = PreparedFrame::new(VP, BLACK, 1.0);

    let r1 = ScreenRect {
        x: 0.0,
        y: 0.0,
        w: 8.0,
        h: 16.0,
    };
    let r2 = ScreenRect {
        x: 8.0,
        y: 0.0,
        w: 8.0,
        h: 16.0,
    };
    frame.backgrounds.push_rect(r1, BLACK, 1.0);
    frame.backgrounds.push_rect(r2, BLACK, 1.0);
    frame.glyphs.push_glyph(
        r1,
        [0.0; 4],
        WHITE,
        1.0,
        0,
        crate::gpu::instance_writer::CLIP_UNCLIPPED,
    );
    frame.cursors.push_cursor(r1, WHITE, 1.0);

    assert!(!frame.is_empty());
    assert_eq!(frame.total_instances(), 4);
    assert_eq!(frame.backgrounds.len(), 2);
    assert_eq!(frame.glyphs.len(), 1);
    assert_eq!(frame.cursors.len(), 1);
}

#[test]
fn clear_resets_all_buffers() {
    let mut frame = PreparedFrame::new(VP, BLACK, 1.0);
    let r = ScreenRect {
        x: 0.0,
        y: 0.0,
        w: 8.0,
        h: 16.0,
    };
    frame.backgrounds.push_rect(r, BLACK, 1.0);
    frame.glyphs.push_glyph(
        r,
        [0.0; 4],
        WHITE,
        1.0,
        0,
        crate::gpu::instance_writer::CLIP_UNCLIPPED,
    );
    frame.cursors.push_cursor(r, WHITE, 1.0);

    frame.clear();

    assert!(frame.is_empty());
    assert_eq!(frame.total_instances(), 0);
}

#[test]
fn clear_and_reuse() {
    let mut frame = PreparedFrame::new(VP, BLACK, 1.0);

    // First frame.
    let r = ScreenRect {
        x: 0.0,
        y: 0.0,
        w: 8.0,
        h: 16.0,
    };
    frame.backgrounds.push_rect(r, BLACK, 1.0);
    assert_eq!(frame.total_instances(), 1);

    // Clear for next frame.
    frame.clear();
    assert!(frame.is_empty());

    // Second frame.
    frame.glyphs.push_glyph(
        r,
        [0.0; 4],
        WHITE,
        1.0,
        0,
        crate::gpu::instance_writer::CLIP_UNCLIPPED,
    );
    let r2 = ScreenRect {
        x: 8.0,
        y: 0.0,
        w: 8.0,
        h: 16.0,
    };
    frame.glyphs.push_glyph(
        r2,
        [0.0; 4],
        WHITE,
        1.0,
        0,
        crate::gpu::instance_writer::CLIP_UNCLIPPED,
    );
    assert_eq!(frame.total_instances(), 2);
}
