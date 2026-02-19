//! Unit tests for DrawList → InstanceWriter conversion.

use oriterm_ui::color::Color;
use oriterm_ui::draw::{DrawList, RectStyle, Shadow};
use oriterm_ui::geometry::Logical;

use crate::gpu::instance_writer::InstanceWriter;

use super::convert_draw_list;

type Rect = oriterm_ui::geometry::Rect<Logical>;
type Point = oriterm_ui::geometry::Point<Logical>;

/// Read a little-endian `f32` from the given byte offset.
fn read_f32(buf: &[u8], offset: usize) -> f32 {
    f32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap())
}

/// Read a little-endian `u32` from the given byte offset.
fn read_u32(buf: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap())
}

// --- Basic rect conversion ---

#[test]
fn empty_draw_list_produces_no_instances() {
    let dl = DrawList::new();
    let mut writer = InstanceWriter::new();
    convert_draw_list(&dl, &mut writer);
    assert!(writer.is_empty());
}

#[test]
fn filled_rect_produces_one_instance() {
    let mut dl = DrawList::new();
    dl.push_rect(
        Rect::new(10.0, 20.0, 100.0, 50.0),
        RectStyle::filled(Color::WHITE),
    );

    let mut writer = InstanceWriter::new();
    convert_draw_list(&dl, &mut writer);

    assert_eq!(writer.len(), 1);

    let rec = writer.as_bytes();
    // Position.
    assert_eq!(read_f32(rec, 0), 10.0);
    assert_eq!(read_f32(rec, 4), 20.0);
    assert_eq!(read_f32(rec, 8), 100.0);
    assert_eq!(read_f32(rec, 12), 50.0);

    // Fill (bg_color) = WHITE.
    assert_eq!(read_f32(rec, 48), 1.0);
    assert_eq!(read_f32(rec, 52), 1.0);
    assert_eq!(read_f32(rec, 56), 1.0);
    assert_eq!(read_f32(rec, 60), 1.0);

    // Kind = UiRect (3).
    assert_eq!(read_u32(rec, 64), 3);

    // No corner radius or border.
    assert_eq!(read_f32(rec, 72), 0.0);
    assert_eq!(read_f32(rec, 76), 0.0);
}

#[test]
fn rect_with_border_writes_border_fields() {
    let mut dl = DrawList::new();
    let style = RectStyle::filled(Color::BLACK)
        .with_border(2.0, Color::WHITE)
        .with_radius(8.0);
    dl.push_rect(Rect::new(0.0, 0.0, 200.0, 100.0), style);

    let mut writer = InstanceWriter::new();
    convert_draw_list(&dl, &mut writer);

    assert_eq!(writer.len(), 1);

    let rec = writer.as_bytes();
    // Border color (fg_color) = WHITE.
    assert_eq!(read_f32(rec, 32), 1.0);
    assert_eq!(read_f32(rec, 36), 1.0);
    assert_eq!(read_f32(rec, 40), 1.0);
    assert_eq!(read_f32(rec, 44), 1.0);

    // Corner radius and border width.
    assert_eq!(read_f32(rec, 72), 8.0);
    assert_eq!(read_f32(rec, 76), 2.0);
}

// --- Shadow ---

#[test]
fn rect_with_shadow_produces_two_instances() {
    let mut dl = DrawList::new();
    let style = RectStyle::filled(Color::WHITE).with_shadow(Shadow {
        offset_x: 0.0,
        offset_y: 4.0,
        blur_radius: 8.0,
        spread: 2.0,
        color: Color::rgba(0.0, 0.0, 0.0, 0.5),
    });
    dl.push_rect(Rect::new(100.0, 100.0, 200.0, 150.0), style);

    let mut writer = InstanceWriter::new();
    convert_draw_list(&dl, &mut writer);

    // Shadow + main rect.
    assert_eq!(writer.len(), 2);

    let bytes = writer.as_bytes();

    // First instance is the shadow (expanded).
    let shadow_rec = &bytes[..80];
    let expand = 2.0 + 8.0; // spread + blur
    assert_eq!(read_f32(shadow_rec, 0), 100.0 - expand); // x
    assert_eq!(read_f32(shadow_rec, 4), 100.0 + 4.0 - expand); // y + offset_y
    assert_eq!(read_f32(shadow_rec, 8), 200.0 + expand * 2.0); // w
    assert_eq!(read_f32(shadow_rec, 12), 150.0 + expand * 2.0); // h

    // Second instance is the main rect.
    let main_rec = &bytes[80..160];
    assert_eq!(read_f32(main_rec, 0), 100.0);
    assert_eq!(read_f32(main_rec, 4), 100.0);
}

// --- Line conversion ---

#[test]
fn horizontal_line_converts_to_rect() {
    let mut dl = DrawList::new();
    dl.push_line(
        Point::new(10.0, 50.0),
        Point::new(110.0, 50.0),
        2.0,
        Color::BLACK,
    );

    let mut writer = InstanceWriter::new();
    convert_draw_list(&dl, &mut writer);

    assert_eq!(writer.len(), 1);

    let rec = writer.as_bytes();
    // Width should be ~100px, height ~2px.
    let w = read_f32(rec, 8);
    let h = read_f32(rec, 12);
    assert!((w - 100.0).abs() < 0.01);
    assert!((h - 2.0).abs() < 0.01);
}

#[test]
fn zero_length_line_produces_nothing() {
    let mut dl = DrawList::new();
    dl.push_line(
        Point::new(50.0, 50.0),
        Point::new(50.0, 50.0),
        2.0,
        Color::BLACK,
    );

    let mut writer = InstanceWriter::new();
    convert_draw_list(&dl, &mut writer);

    assert!(writer.is_empty());
}

// --- Deferred commands ---

#[test]
fn image_command_is_noop() {
    let mut dl = DrawList::new();
    dl.push_image(Rect::new(0.0, 0.0, 64.0, 64.0), 1, [0.0, 0.0, 1.0, 1.0]);

    let mut writer = InstanceWriter::new();
    convert_draw_list(&dl, &mut writer);

    assert!(writer.is_empty());
}

#[test]
fn clip_commands_are_noop() {
    let mut dl = DrawList::new();
    dl.push_clip(Rect::new(0.0, 0.0, 100.0, 100.0));
    dl.push_rect(
        Rect::new(10.0, 10.0, 50.0, 50.0),
        RectStyle::filled(Color::WHITE),
    );
    dl.pop_clip();

    let mut writer = InstanceWriter::new();
    convert_draw_list(&dl, &mut writer);

    // Only the rect should produce an instance; clips are no-ops.
    assert_eq!(writer.len(), 1);
}

// --- Multiple commands ---

#[test]
fn multiple_rects_accumulate() {
    let mut dl = DrawList::new();
    dl.push_rect(
        Rect::new(0.0, 0.0, 50.0, 50.0),
        RectStyle::filled(Color::BLACK),
    );
    dl.push_rect(
        Rect::new(60.0, 0.0, 50.0, 50.0),
        RectStyle::filled(Color::WHITE),
    );
    dl.push_rect(
        Rect::new(120.0, 0.0, 50.0, 50.0),
        RectStyle::filled(Color::BLACK),
    );

    let mut writer = InstanceWriter::new();
    convert_draw_list(&dl, &mut writer);

    assert_eq!(writer.len(), 3);
}

// --- Invisible rect ---

#[test]
fn invisible_rect_still_writes_instance() {
    let mut dl = DrawList::new();
    dl.push_rect(Rect::new(0.0, 0.0, 50.0, 50.0), RectStyle::default());

    let mut writer = InstanceWriter::new();
    convert_draw_list(&dl, &mut writer);

    // An unstyled rect writes a transparent instance (the GPU will discard it).
    assert_eq!(writer.len(), 1);
    let rec = writer.as_bytes();
    assert_eq!(read_f32(rec, 48), 0.0); // fill alpha = 0 (transparent)
}
