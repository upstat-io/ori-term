//! Tests for ANSI escape sequence parsing.

use core::time::Duration;

use super::*;

#[derive(Default)]
pub struct TestSyncHandler {
    is_sync: usize,
}

impl Timeout for TestSyncHandler {
    #[inline]
    fn set_timeout(&mut self, _: Duration) {
        self.is_sync += 1;
    }

    #[inline]
    fn clear_timeout(&mut self) {
        self.is_sync = 0;
    }

    #[inline]
    fn pending_timeout(&self) -> bool {
        self.is_sync != 0
    }
}

struct MockHandler {
    index: CharsetIndex,
    charset: StandardCharset,
    attr: Option<Attr>,
    identity_reported: bool,
    color: Option<Rgb>,
    reset_colors: Vec<usize>,
    title: Option<String>,
    icon_name: Option<String>,
}

impl Handler for MockHandler {
    fn terminal_attribute(&mut self, attr: Attr) {
        self.attr = Some(attr);
    }

    fn configure_charset(&mut self, index: CharsetIndex, charset: StandardCharset) {
        self.index = index;
        self.charset = charset;
    }

    fn set_active_charset(&mut self, index: CharsetIndex) {
        self.index = index;
    }

    fn identify_terminal(&mut self, _intermediate: Option<char>) {
        self.identity_reported = true;
    }

    fn reset_state(&mut self) {
        *self = Self::default();
    }

    fn set_color(&mut self, _: usize, c: Rgb) {
        self.color = Some(c);
    }

    fn reset_color(&mut self, index: usize) {
        self.reset_colors.push(index)
    }

    fn set_title(&mut self, title: Option<String>) {
        self.title = title;
    }

    fn set_icon_name(&mut self, name: Option<String>) {
        self.icon_name = name;
    }
}

impl Default for MockHandler {
    fn default() -> MockHandler {
        MockHandler {
            index: CharsetIndex::G0,
            charset: StandardCharset::Ascii,
            attr: None,
            identity_reported: false,
            color: None,
            reset_colors: Vec::new(),
            title: None,
            icon_name: None,
        }
    }
}

#[test]
fn parse_control_attribute() {
    static BYTES: &[u8] = &[0x1B, b'[', b'1', b'm'];

    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();

    parser.advance(&mut handler, BYTES);

    assert_eq!(handler.attr, Some(Attr::Bold));
}

#[test]
fn parse_terminal_identity_csi() {
    let bytes: &[u8] = &[0x1B, b'[', b'1', b'c'];

    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();

    parser.advance(&mut handler, bytes);

    assert!(!handler.identity_reported);
    handler.reset_state();

    let bytes: &[u8] = &[0x1B, b'[', b'c'];

    parser.advance(&mut handler, bytes);

    assert!(handler.identity_reported);
    handler.reset_state();

    let bytes: &[u8] = &[0x1B, b'[', b'0', b'c'];

    parser.advance(&mut handler, bytes);

    assert!(handler.identity_reported);
}

#[test]
fn parse_terminal_identity_esc() {
    let bytes: &[u8] = &[0x1B, b'Z'];

    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();

    parser.advance(&mut handler, bytes);

    assert!(handler.identity_reported);
    handler.reset_state();

    let bytes: &[u8] = &[0x1B, b'#', b'Z'];

    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();

    parser.advance(&mut handler, bytes);

    assert!(!handler.identity_reported);
    handler.reset_state();
}

#[test]
fn parse_truecolor_attr() {
    static BYTES: &[u8] = &[
        0x1B, b'[', b'3', b'8', b';', b'2', b';', b'1', b'2', b'8', b';', b'6', b'6', b';',
        b'2', b'5', b'5', b'm',
    ];

    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();

    parser.advance(&mut handler, BYTES);

    let spec = Rgb { r: 128, g: 66, b: 255 };

    assert_eq!(handler.attr, Some(Attr::Foreground(Color::Spec(spec))));
}

/// No exactly a test; useful for debugging.
#[test]
fn parse_zsh_startup() {
    static BYTES: &[u8] = &[
        0x1B, b'[', b'1', b'm', 0x1B, b'[', b'7', b'm', b'%', 0x1B, b'[', b'2', b'7', b'm',
        0x1B, b'[', b'1', b'm', 0x1B, b'[', b'0', b'm', b' ', b' ', b' ', b' ', b' ', b' ',
        b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
        b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
        b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
        b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
        b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ', b' ',
        b' ', b' ', b' ', b'\r', b' ', b'\r', b'\r', 0x1B, b'[', b'0', b'm', 0x1B, b'[', b'2',
        b'7', b'm', 0x1B, b'[', b'2', b'4', b'm', 0x1B, b'[', b'J', b'j', b'w', b'i', b'l',
        b'm', b'@', b'j', b'w', b'i', b'l', b'm', b'-', b'd', b'e', b's', b'k', b' ', 0x1B,
        b'[', b'0', b'1', b';', b'3', b'2', b'm', 0xE2, 0x9E, 0x9C, b' ', 0x1B, b'[', b'0',
        b'1', b';', b'3', b'2', b'm', b' ', 0x1B, b'[', b'3', b'6', b'm', b'~', b'/', b'c',
        b'o', b'd', b'e',
    ];

    let mut handler = MockHandler::default();
    let mut parser = Processor::<TestSyncHandler>::new();

    parser.advance(&mut handler, BYTES);
}

#[test]
fn parse_designate_g0_as_line_drawing() {
    static BYTES: &[u8] = &[0x1B, b'(', b'0'];
    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();

    parser.advance(&mut handler, BYTES);

    assert_eq!(handler.index, CharsetIndex::G0);
    assert_eq!(handler.charset, StandardCharset::SpecialCharacterAndLineDrawing);
}

#[test]
fn parse_designate_g1_as_line_drawing_and_invoke() {
    static BYTES: &[u8] = &[0x1B, b')', b'0', 0x0E];
    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();

    parser.advance(&mut handler, &BYTES[..3]);

    assert_eq!(handler.index, CharsetIndex::G1);
    assert_eq!(handler.charset, StandardCharset::SpecialCharacterAndLineDrawing);

    let mut handler = MockHandler::default();
    parser.advance(&mut handler, &[BYTES[3]]);

    assert_eq!(handler.index, CharsetIndex::G1);
}

#[test]
fn parse_valid_rgb_colors() {
    assert_eq!(
        colors::xparse_color(b"rgb:f/e/d"),
        Some(Rgb { r: 0xFF, g: 0xEE, b: 0xDD })
    );
    assert_eq!(
        colors::xparse_color(b"rgb:11/aa/ff"),
        Some(Rgb { r: 0x11, g: 0xAA, b: 0xFF })
    );
    assert_eq!(
        colors::xparse_color(b"rgb:f/ed1/cb23"),
        Some(Rgb { r: 0xFF, g: 0xEC, b: 0xCA })
    );
    assert_eq!(
        colors::xparse_color(b"rgb:ffff/0/0"),
        Some(Rgb { r: 0xFF, g: 0x0, b: 0x0 })
    );
}

#[test]
fn parse_valid_legacy_rgb_colors() {
    assert_eq!(colors::xparse_color(b"#1af"), Some(Rgb { r: 0x10, g: 0xA0, b: 0xF0 }));
    assert_eq!(
        colors::xparse_color(b"#11aaff"),
        Some(Rgb { r: 0x11, g: 0xAA, b: 0xFF })
    );
    assert_eq!(
        colors::xparse_color(b"#110aa0ff0"),
        Some(Rgb { r: 0x11, g: 0xAA, b: 0xFF })
    );
    assert_eq!(
        colors::xparse_color(b"#1100aa00ff00"),
        Some(Rgb { r: 0x11, g: 0xAA, b: 0xFF })
    );
}

#[test]
fn parse_invalid_rgb_colors() {
    assert_eq!(colors::xparse_color(b"rgb:0//"), None);
    assert_eq!(colors::xparse_color(b"rgb://///"), None);
}

#[test]
fn parse_invalid_legacy_rgb_colors() {
    assert_eq!(colors::xparse_color(b"#"), None);
    assert_eq!(colors::xparse_color(b"#f"), None);
}

#[test]
fn parse_invalid_number() {
    assert_eq!(colors::parse_number(b"1abc"), None);
}

#[test]
fn parse_valid_number() {
    assert_eq!(colors::parse_number(b"123"), Some(123));
}

#[test]
fn parse_number_too_large() {
    assert_eq!(colors::parse_number(b"321"), None);
}

#[test]
fn parse_osc4_set_color() {
    let bytes: &[u8] = b"\x1b]4;0;#fff\x1b\\";

    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();

    parser.advance(&mut handler, bytes);

    assert_eq!(handler.color, Some(Rgb { r: 0xF0, g: 0xF0, b: 0xF0 }));
}

#[test]
fn parse_osc104_reset_color() {
    let bytes: &[u8] = b"\x1b]104;1;\x1b\\";

    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();

    parser.advance(&mut handler, bytes);

    assert_eq!(handler.reset_colors, vec![1]);
}

#[test]
fn parse_osc104_reset_all_colors() {
    let bytes: &[u8] = b"\x1b]104;\x1b\\";

    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();

    parser.advance(&mut handler, bytes);

    let expected: Vec<usize> = (0..256).collect();
    assert_eq!(handler.reset_colors, expected);
}

#[test]
fn parse_osc104_reset_all_colors_no_semicolon() {
    let bytes: &[u8] = b"\x1b]104\x1b\\";

    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();

    parser.advance(&mut handler, bytes);

    let expected: Vec<usize> = (0..256).collect();
    assert_eq!(handler.reset_colors, expected);
}

#[test]
fn partial_sync_updates() {
    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();

    assert_eq!(parser.state.sync_state.timeout.is_sync, 0);
    assert!(handler.attr.is_none());

    // Start synchronized update.

    parser.advance(&mut handler, b"\x1b[?20");
    assert_eq!(parser.state.sync_state.timeout.is_sync, 0);
    assert!(handler.attr.is_none());

    parser.advance(&mut handler, b"26h");
    assert_eq!(parser.state.sync_state.timeout.is_sync, 1);
    assert!(handler.attr.is_none());

    // Dispatch some data.

    parser.advance(&mut handler, b"random \x1b[31m stuff");
    assert_eq!(parser.state.sync_state.timeout.is_sync, 1);
    assert!(handler.attr.is_none());

    // Extend synchronized update.

    parser.advance(&mut handler, b"\x1b[?20");
    assert_eq!(parser.state.sync_state.timeout.is_sync, 1);
    assert!(handler.attr.is_none());

    parser.advance(&mut handler, b"26h");
    assert_eq!(parser.state.sync_state.timeout.is_sync, 2);
    assert!(handler.attr.is_none());

    // Terminate synchronized update.

    parser.advance(&mut handler, b"\x1b[?20");
    assert_eq!(parser.state.sync_state.timeout.is_sync, 2);
    assert!(handler.attr.is_none());

    parser.advance(&mut handler, b"26l");
    assert_eq!(parser.state.sync_state.timeout.is_sync, 0);
    assert!(handler.attr.is_some());
}

#[test]
fn sync_bursts_buffer() {
    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();

    assert_eq!(parser.state.sync_state.timeout.is_sync, 0);
    assert!(handler.attr.is_none());

    // Repeat test twice to ensure internal state is reset properly.
    for _ in 0..2 {
        // Start synchronized update.
        parser.advance(&mut handler, b"\x1b[?2026h");
        assert_eq!(parser.state.sync_state.timeout.is_sync, 1);
        assert!(handler.attr.is_none());

        // Ensure sync works.
        parser.advance(&mut handler, b"\x1b[31m");
        assert_eq!(parser.state.sync_state.timeout.is_sync, 1);
        assert!(handler.attr.is_none());

        // Exceed sync buffer dimensions.
        parser.advance(&mut handler, "a".repeat(SYNC_BUFFER_SIZE).as_bytes());
        assert_eq!(parser.state.sync_state.timeout.is_sync, 0);
        assert!(handler.attr.take().is_some());

        // Ensure new events are dispatched directly.
        parser.advance(&mut handler, b"\x1b[31m");
        assert_eq!(parser.state.sync_state.timeout.is_sync, 0);
        assert!(handler.attr.take().is_some());
    }
}

#[test]
fn mixed_sync_escape() {
    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();

    assert_eq!(parser.state.sync_state.timeout.is_sync, 0);
    assert!(handler.attr.is_none());

    // Start synchronized update with immediate SGR.
    parser.advance(&mut handler, b"\x1b[?2026h\x1b[31m");
    assert_eq!(parser.state.sync_state.timeout.is_sync, 1);
    assert!(handler.attr.is_none());

    // Terminate synchronized update and check for SGR.
    parser.advance(&mut handler, b"\x1b[?2026l");
    assert_eq!(parser.state.sync_state.timeout.is_sync, 0);
    assert!(handler.attr.is_some());
}

#[test]
fn sync_bsu_with_esu() {
    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();

    assert_eq!(parser.state.sync_state.timeout.is_sync, 0);
    assert!(handler.attr.is_none());

    // Start synchronized update with immediate SGR.
    parser.advance(&mut handler, b"\x1b[?2026h\x1b[1m");
    assert_eq!(parser.state.sync_state.timeout.is_sync, 1);
    assert!(handler.attr.is_none());

    // Terminate synchronized update, but immediately start a new one.
    parser.advance(&mut handler, b"\x1b[?2026l\x1b[?2026h\x1b[4m");
    assert_eq!(parser.state.sync_state.timeout.is_sync, 2);
    assert_eq!(handler.attr.take(), Some(Attr::Bold));

    // Terminate again, expecting one buffered SGR.
    parser.advance(&mut handler, b"\x1b[?2026l");
    assert_eq!(parser.state.sync_state.timeout.is_sync, 0);
    assert_eq!(handler.attr.take(), Some(Attr::Underline));
}

#[test]
#[cfg(feature = "std")]
fn contrast() {
    let rgb1 = Rgb { r: 0xFF, g: 0xFF, b: 0xFF };
    let rgb2 = Rgb { r: 0x00, g: 0x00, b: 0x00 };
    assert!((rgb1.contrast(rgb2) - 21.).abs() < f64::EPSILON);

    let rgb1 = Rgb { r: 0xFF, g: 0xFF, b: 0xFF };
    assert!((rgb1.contrast(rgb1) - 1.).abs() < f64::EPSILON);

    let rgb1 = Rgb { r: 0xFF, g: 0x00, b: 0xFF };
    let rgb2 = Rgb { r: 0x00, g: 0xFF, b: 0x00 };
    assert!((rgb1.contrast(rgb2) - 2.285_543_608_124_253_3).abs() < f64::EPSILON);

    let rgb1 = Rgb { r: 0x12, g: 0x34, b: 0x56 };
    let rgb2 = Rgb { r: 0xFE, g: 0xDC, b: 0xBA };
    assert!((rgb1.contrast(rgb2) - 9.786_558_997_257_74).abs() < f64::EPSILON);
}

// --- OSC title/icon dispatch tests ---

#[test]
fn osc_0_sets_both_title_and_icon_name() {
    // OSC 0 ; text ST
    let bytes: &[u8] = b"\x1b]0;hello\x1b\\";
    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();
    parser.advance(&mut handler, bytes);
    assert_eq!(handler.title.as_deref(), Some("hello"));
    assert_eq!(handler.icon_name.as_deref(), Some("hello"));
}

#[test]
fn osc_1_sets_only_icon_name() {
    // OSC 1 ; text ST
    let bytes: &[u8] = b"\x1b]1;icon\x1b\\";
    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();
    parser.advance(&mut handler, bytes);
    assert_eq!(handler.title, None);
    assert_eq!(handler.icon_name.as_deref(), Some("icon"));
}

#[test]
fn osc_2_sets_only_title() {
    // OSC 2 ; text ST
    let bytes: &[u8] = b"\x1b]2;title\x1b\\";
    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();
    parser.advance(&mut handler, bytes);
    assert_eq!(handler.title.as_deref(), Some("title"));
    assert_eq!(handler.icon_name, None);
}
