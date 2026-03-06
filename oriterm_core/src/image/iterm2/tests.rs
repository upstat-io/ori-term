//! Tests for iTerm2 image protocol parsing and handler execution.

use std::sync::{Arc, Mutex};

use vte::ansi::Processor;

use super::{Iterm2Error, SizeSpec, parse_iterm2_file};
use crate::event::{Event, EventListener};
use crate::term::Term;
use crate::theme::Theme;

// --- Parsing tests ---

#[test]
fn parse_width_height_auto() {
    assert_eq!(super::parse_size_spec(b"auto"), SizeSpec::Auto);
    assert_eq!(super::parse_size_spec(b"Auto"), SizeSpec::Auto);
    assert_eq!(super::parse_size_spec(b"AUTO"), SizeSpec::Auto);
    assert_eq!(super::parse_size_spec(b""), SizeSpec::Auto);
}

#[test]
fn parse_width_height_cells() {
    assert_eq!(super::parse_size_spec(b"80"), SizeSpec::Cells(80));
    assert_eq!(super::parse_size_spec(b"1"), SizeSpec::Cells(1));
}

#[test]
fn parse_width_height_pixels() {
    assert_eq!(super::parse_size_spec(b"100px"), SizeSpec::Pixels(100));
    assert_eq!(super::parse_size_spec(b"640px"), SizeSpec::Pixels(640));
}

#[test]
fn parse_width_height_percent() {
    assert_eq!(super::parse_size_spec(b"50%"), SizeSpec::Percent(50));
    assert_eq!(super::parse_size_spec(b"100%"), SizeSpec::Percent(100));
}

#[test]
fn parse_basic_inline_image() {
    // Minimal inline image: File=inline=1:base64data
    // Base64 of 4 bytes (0x89 0x50 0x4E 0x47 = PNG magic... but it won't be valid PNG)
    // Just test parsing, not decode.
    let params: &[&[u8]] = &[b"File=inline=1:AQID"];
    let img = parse_iterm2_file(params).unwrap();
    assert!(img.inline);
    assert_eq!(img.width, SizeSpec::Auto);
    assert_eq!(img.height, SizeSpec::Auto);
    assert!(img.preserve_aspect_ratio);
    assert_eq!(img.data, &[1, 2, 3]);
}

#[test]
fn parse_with_all_args() {
    // Simulating VTE splitting on ';':
    // File=name=dGVzdC5wbmc=;size=1234;width=80;height=50%;inline=1;preserveAspectRatio=0:AQID
    let params: &[&[u8]] = &[
        b"File=name=dGVzdC5wbmc=",
        b"size=1234",
        b"width=80",
        b"height=50%",
        b"inline=1",
        b"preserveAspectRatio=0:AQID",
    ];
    let img = parse_iterm2_file(params).unwrap();
    assert_eq!(img.name, Some("test.png".to_string()));
    assert_eq!(img.size, Some(1234));
    assert_eq!(img.width, SizeSpec::Cells(80));
    assert_eq!(img.height, SizeSpec::Percent(50));
    assert!(img.inline);
    assert!(!img.preserve_aspect_ratio);
    assert_eq!(img.data, &[1, 2, 3]);
}

#[test]
fn parse_preserves_aspect_ratio_by_default() {
    let params: &[&[u8]] = &[b"File=inline=1:AQID"];
    let img = parse_iterm2_file(params).unwrap();
    assert!(img.preserve_aspect_ratio);
}

#[test]
fn parse_missing_payload() {
    // No colon separator means no payload.
    let params: &[&[u8]] = &[b"File=inline=1"];
    let err = parse_iterm2_file(params).unwrap_err();
    assert_eq!(err, Iterm2Error::MissingPayload);
}

#[test]
fn parse_empty_payload() {
    // Colon present but nothing after it.
    let params: &[&[u8]] = &[b"File=inline=1:"];
    let err = parse_iterm2_file(params).unwrap_err();
    assert_eq!(err, Iterm2Error::MissingPayload);
}

#[test]
fn parse_invalid_base64() {
    // `@` is not a valid base64 character.
    let params: &[&[u8]] = &[b"File=inline=1:@@@"];
    let err = parse_iterm2_file(params).unwrap_err();
    assert_eq!(err, Iterm2Error::InvalidBase64);
}

#[test]
fn parse_unknown_keys_ignored() {
    let params: &[&[u8]] = &[b"File=inline=1", b"foo=bar:AQID"];
    let img = parse_iterm2_file(params).unwrap();
    assert!(img.inline);
    assert_eq!(img.data, &[1, 2, 3]);
}

#[test]
fn parse_non_inline_default() {
    let params: &[&[u8]] = &[b"File=:AQID"];
    let img = parse_iterm2_file(params).unwrap();
    assert!(!img.inline);
}

#[test]
fn parse_pixel_width_spec() {
    let params: &[&[u8]] = &[b"File=width=200px:AQID"];
    let img = parse_iterm2_file(params).unwrap();
    assert_eq!(img.width, SizeSpec::Pixels(200));
}

// --- Handler integration tests ---

struct RecordingListener {
    events: Arc<Mutex<Vec<String>>>,
}

impl RecordingListener {
    fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Clone for RecordingListener {
    fn clone(&self) -> Self {
        Self {
            events: Arc::clone(&self.events),
        }
    }
}

impl EventListener for RecordingListener {
    fn send_event(&self, event: Event) {
        let s = match &event {
            Event::PtyWrite(data) => format!("pty:{data}"),
            other => format!("{other:?}"),
        };
        self.events.lock().expect("lock poisoned").push(s);
    }
}

fn term_with_recorder() -> (Term<RecordingListener>, RecordingListener) {
    let listener = RecordingListener::new();
    let term = Term::new(24, 80, 100, Theme::default(), listener.clone());
    (term, listener)
}

fn feed(term: &mut impl vte::ansi::Handler, bytes: &[u8]) {
    let mut processor: Processor = Processor::new();
    processor.advance(term, bytes);
}

/// Build an OSC 1337 File= sequence.
///
/// `args` is the key=value pairs (e.g., "inline=1").
/// `b64_data` is the base64-encoded image data.
fn iterm2_osc(args: &str, b64_data: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(b"\x1b]1337;File=");
    buf.extend_from_slice(args.as_bytes());
    buf.push(b':');
    buf.extend_from_slice(b64_data.as_bytes());
    buf.push(0x07); // BEL terminator.
    buf
}

/// Create a minimal valid 1x1 red PNG as base64.
fn tiny_png_b64() -> String {
    use base64::Engine;
    let png_data = create_tiny_png();
    base64::engine::general_purpose::STANDARD.encode(&png_data)
}

/// Create a minimal valid 1x1 red PNG.
fn create_tiny_png() -> Vec<u8> {
    // Use the image crate to create a 1x1 PNG.
    let mut buf = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buf);
    let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]));
    img.write_to(&mut cursor, image::ImageFormat::Png).unwrap();
    buf
}

/// Create a 10x5 PNG.
fn create_sized_png(w: u32, h: u32) -> Vec<u8> {
    let mut buf = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buf);
    let img = image::RgbaImage::from_pixel(w, h, image::Rgba([0, 128, 255, 255]));
    img.write_to(&mut cursor, image::ImageFormat::Png).unwrap();
    buf
}

fn png_to_b64(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}

#[test]
fn handler_inline_image_placed_at_cursor() {
    let (mut term, _listener) = term_with_recorder();
    let b64 = tiny_png_b64();
    let osc = iterm2_osc("inline=1", &b64);
    feed(&mut term, &osc);

    // Image should be stored in cache.
    let placements = term.image_cache().placements_in_viewport(
        crate::grid::StableRowIndex(0),
        crate::grid::StableRowIndex(1000),
    );
    assert_eq!(placements.len(), 1, "expected 1 placement");
    assert_eq!(placements[0].cell_col, 0);
}

#[test]
fn handler_cursor_advances_below_image() {
    let (mut term, _listener) = term_with_recorder();
    let png = create_sized_png(8, 48); // 48px tall with 16px cells = 3 rows.
    let b64 = png_to_b64(&png);
    let osc = iterm2_osc("inline=1", &b64);

    let line_before = term.grid().cursor().line();
    feed(&mut term, &osc);
    let line_after = term.grid().cursor().line();

    // Should advance by rows - 1 = 2 lines (image is 3 cell rows tall).
    assert!(
        line_after > line_before,
        "cursor should advance: before={line_before}, after={line_after}"
    );
}

#[test]
fn handler_aspect_ratio_preserved() {
    let (mut term, _listener) = term_with_recorder();
    // 20x10 image with width=40px, height=auto, preserveAspectRatio=1.
    // Width is 40px, aspect 2:1, so height should be 20px.
    let png = create_sized_png(20, 10);
    let b64 = png_to_b64(&png);
    let osc = iterm2_osc("width=40px;inline=1;preserveAspectRatio=1", &b64);
    feed(&mut term, &osc);

    let placements = term.image_cache().placements_in_viewport(
        crate::grid::StableRowIndex(0),
        crate::grid::StableRowIndex(1000),
    );
    assert_eq!(placements.len(), 1);
    // 40px / 8px cell_width = 5 cols.
    assert_eq!(placements[0].cols, 5);
    // 20px / 16px cell_height = 2 rows (div_ceil).
    assert_eq!(placements[0].rows, 2);
}

#[test]
fn handler_aspect_ratio_not_preserved() {
    let (mut term, _listener) = term_with_recorder();
    // 20x10 image with width=40px, height=32px, preserveAspectRatio=0.
    let png = create_sized_png(20, 10);
    let b64 = png_to_b64(&png);
    let osc = iterm2_osc(
        "width=40px;height=32px;inline=1;preserveAspectRatio=0",
        &b64,
    );
    feed(&mut term, &osc);

    let placements = term.image_cache().placements_in_viewport(
        crate::grid::StableRowIndex(0),
        crate::grid::StableRowIndex(1000),
    );
    assert_eq!(placements.len(), 1);
    // 40px / 8px = 5 cols.
    assert_eq!(placements[0].cols, 5);
    // 32px / 16px = 2 rows.
    assert_eq!(placements[0].rows, 2);
}

#[test]
fn handler_oversized_payload_rejected() {
    let (mut term, _listener) = term_with_recorder();
    // The default max_single_image_bytes is 64 MB. Create oversized test
    // by temporarily setting a low limit.
    term.image_cache_mut().set_max_single_image(100);

    // Create a PNG that's larger than 100 bytes.
    let png = create_sized_png(50, 50);
    let b64 = png_to_b64(&png);
    let osc = iterm2_osc("inline=1", &b64);
    feed(&mut term, &osc);

    let placements = term.image_cache().placements_in_viewport(
        crate::grid::StableRowIndex(0),
        crate::grid::StableRowIndex(1000),
    );
    assert!(placements.is_empty(), "oversized image should be rejected");
}

#[test]
fn handler_invalid_base64_no_crash() {
    let (mut term, _listener) = term_with_recorder();
    // `@@@` is invalid base64 — should be handled gracefully.
    let osc = iterm2_osc("inline=1", "@@@");
    feed(&mut term, &osc);

    let placements = term.image_cache().placements_in_viewport(
        crate::grid::StableRowIndex(0),
        crate::grid::StableRowIndex(1000),
    );
    assert!(placements.is_empty());
}

#[test]
fn handler_invalid_image_format_no_crash() {
    let (mut term, _listener) = term_with_recorder();
    // Valid base64 but invalid image data.
    let osc = iterm2_osc("inline=1", "AQIDBA==");
    feed(&mut term, &osc);

    let placements = term.image_cache().placements_in_viewport(
        crate::grid::StableRowIndex(0),
        crate::grid::StableRowIndex(1000),
    );
    assert!(placements.is_empty());
}

#[test]
fn handler_non_inline_not_displayed() {
    let (mut term, _listener) = term_with_recorder();
    let b64 = tiny_png_b64();
    // inline=0 (or absent) means download, not display.
    let osc = iterm2_osc("inline=0", &b64);
    feed(&mut term, &osc);

    let placements = term.image_cache().placements_in_viewport(
        crate::grid::StableRowIndex(0),
        crate::grid::StableRowIndex(1000),
    );
    assert!(
        placements.is_empty(),
        "non-inline images should not be placed"
    );
}
