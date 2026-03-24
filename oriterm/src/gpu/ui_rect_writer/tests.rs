//! Unit tests for the dedicated UI rect writer.

use super::*;
use crate::gpu::instance_writer::ScreenRect;

#[test]
fn ui_rect_instance_stride_is_144_bytes() {
    assert_eq!(UI_RECT_INSTANCE_SIZE, 144);
}

#[test]
fn ui_rect_writer_count_matches_instances() {
    let mut w = UiRectWriter::new();
    assert_eq!(w.len(), 0);
    push_test_rect(&mut w);
    assert_eq!(w.len(), 1);
    push_test_rect(&mut w);
    assert_eq!(w.len(), 2);
    assert_eq!(w.byte_len(), 2 * UI_RECT_INSTANCE_SIZE);
}

#[test]
fn ui_rect_writer_clear_resets_to_zero() {
    let mut w = UiRectWriter::new();
    push_test_rect(&mut w);
    push_test_rect(&mut w);
    let cap_before = w.buf.capacity();
    w.clear();
    assert_eq!(w.len(), 0);
    assert!(w.is_empty());
    assert!(w.buf.capacity() >= cap_before);
}

#[test]
fn ui_rect_writer_maybe_shrink_honors_4x_threshold() {
    let mut w = UiRectWriter::new();
    // Push many instances to grow capacity.
    for _ in 0..100 {
        push_test_rect(&mut w);
    }
    w.clear();
    // Now capacity >> usage. Push one instance so len=1.
    push_test_rect(&mut w);
    let cap_before = w.buf.capacity();
    assert!(cap_before > 4 * w.buf.len());
    w.maybe_shrink();
    assert!(w.buf.capacity() < cap_before);
}

#[test]
fn ui_rect_writer_push_writes_correct_offsets() {
    let mut w = UiRectWriter::new();
    let rect = ScreenRect {
        x: 10.0,
        y: 20.0,
        w: 100.0,
        h: 50.0,
    };
    let fill = [0.1, 0.2, 0.3, 1.0];
    let bw = [1.0, 2.0, 3.0, 4.0];
    let cr = [5.0, 6.0, 7.0, 8.0];
    let bc = [
        [0.5, 0.0, 0.0, 1.0],
        [0.0, 0.5, 0.0, 1.0],
        [0.0, 0.0, 0.5, 1.0],
        [0.5, 0.5, 0.0, 1.0],
    ];
    let clip = [0.0, 0.0, 800.0, 600.0];
    w.push_ui_rect(rect, fill, bw, cr, bc, clip);

    let buf = w.as_bytes();

    // pos
    assert_eq!(read_f32(buf, 0), 10.0);
    assert_eq!(read_f32(buf, 4), 20.0);
    // size
    assert_eq!(read_f32(buf, 8), 100.0);
    assert_eq!(read_f32(buf, 12), 50.0);
    // clip
    assert_eq!(read_f32(buf, 16), 0.0);
    assert_eq!(read_f32(buf, 20), 0.0);
    assert_eq!(read_f32(buf, 24), 800.0);
    assert_eq!(read_f32(buf, 28), 600.0);
    // fill
    assert_eq!(read_f32(buf, 32), 0.1);
    assert_eq!(read_f32(buf, 36), 0.2);
    assert_eq!(read_f32(buf, 40), 0.3);
    assert_eq!(read_f32(buf, 44), 1.0);
    // border_widths
    assert_eq!(read_f32(buf, 48), 1.0);
    assert_eq!(read_f32(buf, 52), 2.0);
    assert_eq!(read_f32(buf, 56), 3.0);
    assert_eq!(read_f32(buf, 60), 4.0);
    // corner_radii
    assert_eq!(read_f32(buf, 64), 5.0);
    assert_eq!(read_f32(buf, 68), 6.0);
    assert_eq!(read_f32(buf, 72), 7.0);
    assert_eq!(read_f32(buf, 76), 8.0);
    // border_top
    assert_eq!(read_f32(buf, 80), 0.5);
    assert_eq!(read_f32(buf, 84), 0.0);
    // border_right
    assert_eq!(read_f32(buf, 96), 0.0);
    assert_eq!(read_f32(buf, 100), 0.5);
    // border_bottom
    assert_eq!(read_f32(buf, 112), 0.0);
    assert_eq!(read_f32(buf, 116), 0.0);
    assert_eq!(read_f32(buf, 120), 0.5);
    // border_left
    assert_eq!(read_f32(buf, 128), 0.5);
    assert_eq!(read_f32(buf, 132), 0.5);
}

#[test]
fn ui_rect_writer_push_uniform_border_writes_equal_sides() {
    let mut w = UiRectWriter::new();
    let bc = [[1.0, 0.0, 0.0, 1.0]; 4];
    w.push_ui_rect(
        ScreenRect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
        },
        [0.0; 4],
        [2.0; 4],
        [0.0; 4],
        bc,
        [0.0, 0.0, 100.0, 100.0],
    );
    let buf = w.as_bytes();
    // All four border widths equal.
    for i in 0..4 {
        assert_eq!(read_f32(buf, OFF_BORDER_WIDTHS + i * 4), 2.0);
    }
    // All four border colors equal.
    for side in 0..4 {
        let base = OFF_BORDER_TOP + side * 16;
        assert_eq!(read_f32(buf, base), 1.0);
        assert_eq!(read_f32(buf, base + 4), 0.0);
    }
}

#[test]
fn ui_rect_writer_push_zero_border_all_transparent() {
    let mut w = UiRectWriter::new();
    w.push_ui_rect(
        ScreenRect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
        },
        [1.0, 1.0, 1.0, 1.0],
        [0.0; 4],
        [0.0; 4],
        [[0.0; 4]; 4],
        [0.0, 0.0, 100.0, 100.0],
    );
    let buf = w.as_bytes();
    for i in 0..4 {
        assert_eq!(read_f32(buf, OFF_BORDER_WIDTHS + i * 4), 0.0);
    }
    for side in 0..4 {
        for ch in 0..4 {
            assert_eq!(read_f32(buf, OFF_BORDER_TOP + side * 16 + ch * 4), 0.0);
        }
    }
}

#[test]
fn ui_rect_writer_extend_from_appends_correctly() {
    let mut a = UiRectWriter::new();
    let mut b = UiRectWriter::new();
    push_test_rect(&mut a);
    push_test_rect(&mut b);
    push_test_rect(&mut b);
    a.extend_from(&b);
    assert_eq!(a.len(), 3);
    assert_eq!(a.byte_len(), 3 * UI_RECT_INSTANCE_SIZE);
}

fn push_test_rect(w: &mut UiRectWriter) {
    w.push_ui_rect(
        ScreenRect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: 10.0,
        },
        [0.0; 4],
        [0.0; 4],
        [0.0; 4],
        [[0.0; 4]; 4],
        [0.0, 0.0, 100.0, 100.0],
    );
}
