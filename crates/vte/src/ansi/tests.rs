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
    decic_counts: Vec<u16>,
    decdc_counts: Vec<u16>,
    decbi_calls: u32,
    decfi_calls: u32,
    decrqss_queries: Vec<Vec<u8>>,
    decrsps_calls: Vec<(u16, Vec<u8>)>,
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

    fn decic(&mut self, count: u16) {
        self.decic_counts.push(count);
    }

    fn decdc(&mut self, count: u16) {
        self.decdc_counts.push(count);
    }

    fn decbi(&mut self) {
        self.decbi_calls += 1;
    }

    fn decfi(&mut self) {
        self.decfi_calls += 1;
    }

    fn decrqss(&mut self, query: &[u8]) {
        self.decrqss_queries.push(query.to_vec());
    }

    fn decrsps(&mut self, ps: u16, pt: &[u8]) {
        self.decrsps_calls.push((ps, pt.to_vec()));
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
            decic_counts: Vec::new(),
            decdc_counts: Vec::new(),
            decbi_calls: 0,
            decfi_calls: 0,
            decrqss_queries: Vec::new(),
            decrsps_calls: Vec::new(),
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

/// Mode 2026 sync window arms / disarms the parser-side timer across
/// chunked advance() calls — and intra-window bytes dispatch inline.
///
/// `TestSyncHandler::is_sync` increments on every `set_timeout` call
/// and resets to 0 on `clear_timeout`, giving a count of timer arms.
/// BSU re-arms; ESU disarms.
#[test]
fn partial_sync_updates() {
    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();

    assert_eq!(parser.state.sync_state.timeout.is_sync, 0);
    assert!(handler.attr.is_none());

    // BSU split across two advance() calls — timer arms only when the
    // full sequence is consumed.
    parser.advance(&mut handler, b"\x1b[?20");
    assert_eq!(parser.state.sync_state.timeout.is_sync, 0);
    parser.advance(&mut handler, b"26h");
    assert_eq!(parser.state.sync_state.timeout.is_sync, 1);

    // SGR mid-sync dispatches INLINE (Mode 2026 gates snapshots, not
    // byte processing).
    parser.advance(&mut handler, b"random \x1b[31m stuff");
    assert_eq!(parser.state.sync_state.timeout.is_sync, 1);
    assert!(
        handler.attr.is_some(),
        "SGR mid-sync must dispatch inline (Mode 2026 does not buffer)"
    );
    handler.attr = None;

    // Second BSU re-arms the timer (counter increments again).
    parser.advance(&mut handler, b"\x1b[?20");
    parser.advance(&mut handler, b"26h");
    assert_eq!(parser.state.sync_state.timeout.is_sync, 2);

    // ESU split across two advance() calls — timer disarms when the
    // full sequence is consumed.
    parser.advance(&mut handler, b"\x1b[?20");
    assert_eq!(parser.state.sync_state.timeout.is_sync, 2);
    parser.advance(&mut handler, b"26l");
    assert_eq!(parser.state.sync_state.timeout.is_sync, 0);
}

#[test]
fn mixed_sync_escape() {
    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();

    assert_eq!(parser.state.sync_state.timeout.is_sync, 0);
    assert!(handler.attr.is_none());

    // BSU + SGR in one advance() — both dispatch inline.
    parser.advance(&mut handler, b"\x1b[?2026h\x1b[31m");
    assert_eq!(parser.state.sync_state.timeout.is_sync, 1);
    assert!(
        handler.attr.is_some(),
        "SGR after BSU must dispatch inline (Mode 2026 does not buffer)"
    );

    // ESU disarms the timer.
    parser.advance(&mut handler, b"\x1b[?2026l");
    assert_eq!(parser.state.sync_state.timeout.is_sync, 0);
}

#[test]
fn sync_bsu_with_esu() {
    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();

    // BSU + SGR(Bold) in one chunk — Bold dispatches inline.
    parser.advance(&mut handler, b"\x1b[?2026h\x1b[1m");
    assert_eq!(parser.state.sync_state.timeout.is_sync, 1);
    assert_eq!(handler.attr.take(), Some(Attr::Bold));

    // ESU + BSU + SGR(Underline) in one chunk — ESU disarms, BSU
    // re-arms, Underline dispatches inline.
    parser.advance(&mut handler, b"\x1b[?2026l\x1b[?2026h\x1b[4m");
    assert_eq!(parser.state.sync_state.timeout.is_sync, 1);
    assert_eq!(handler.attr.take(), Some(Attr::Underline));

    // Final ESU clears the timer.
    parser.advance(&mut handler, b"\x1b[?2026l");
    assert_eq!(parser.state.sync_state.timeout.is_sync, 0);
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

#[test]
fn parse_esc_6_dispatches_decbi() {
    // DECBI: ESC 6
    let bytes: &[u8] = b"\x1b6";
    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();
    parser.advance(&mut handler, bytes);
    assert_eq!(handler.decbi_calls, 1);
    assert_eq!(handler.decfi_calls, 0);
}

#[test]
fn parse_esc_9_dispatches_decfi() {
    // DECFI: ESC 9
    let bytes: &[u8] = b"\x1b9";
    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();
    parser.advance(&mut handler, bytes);
    assert_eq!(handler.decfi_calls, 1);
    assert_eq!(handler.decbi_calls, 0);
}

#[test]
fn parse_csi_apostrophe_right_brace_dispatches_decic() {
    // DECIC: CSI Ps ' }
    let bytes: &[u8] = b"\x1b[3'}";
    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();
    parser.advance(&mut handler, bytes);
    assert_eq!(handler.decic_counts, vec![3]);
    assert!(handler.decdc_counts.is_empty());
}

#[test]
fn parse_csi_apostrophe_tilde_dispatches_decdc() {
    // DECDC: CSI Ps ' ~
    let bytes: &[u8] = b"\x1b[2'~";
    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();
    parser.advance(&mut handler, bytes);
    assert_eq!(handler.decdc_counts, vec![2]);
    assert!(handler.decic_counts.is_empty());
}

#[test]
fn parse_decic_default_count_is_one() {
    // DECIC with no explicit Ps → default 1.
    let bytes: &[u8] = b"\x1b['}";
    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();
    parser.advance(&mut handler, bytes);
    assert_eq!(handler.decic_counts, vec![1]);
}

#[test]
fn parse_decrqss_decscusr_routes_to_handler() {
    // DECRQSS DECSCUSR query: DCS $ q q ST.
    let bytes: &[u8] = b"\x1bP$qq\x1b\\";
    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();
    parser.advance(&mut handler, bytes);
    assert_eq!(handler.decrqss_queries, vec![b"q".to_vec()]);
}

#[test]
fn parse_decrqss_decsca_routes_to_handler() {
    // DECRQSS DECSCA query: DCS $ q " q ST.
    let bytes: &[u8] = b"\x1bP$q\"q\x1b\\";
    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();
    parser.advance(&mut handler, bytes);
    assert_eq!(handler.decrqss_queries, vec![b"\"q".to_vec()]);
}

#[test]
fn parse_decrsps_ps1_routes_to_handler() {
    // DECRSPS with Ps=1 (DECCIR cursor-info) and a small payload.
    let bytes: &[u8] = b"\x1bP1$tABC\x1b\\";
    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();
    parser.advance(&mut handler, bytes);
    assert_eq!(handler.decrsps_calls, vec![(1, b"ABC".to_vec())]);
}

#[test]
fn parse_decrsps_ps2_routes_to_handler() {
    // DECRSPS with Ps=2 (DECTABSR tab-stop vector).
    let bytes: &[u8] = b"\x1bP2$t1/9/17\x1b\\";
    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();
    parser.advance(&mut handler, bytes);
    assert_eq!(handler.decrsps_calls, vec![(2, b"1/9/17".to_vec())]);
}

#[test]
fn parse_decrsps_default_ps_is_zero() {
    // DECRSPS with no explicit Ps → default 0 (unrecognized format).
    let bytes: &[u8] = b"\x1bP$t\x1b\\";
    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();
    parser.advance(&mut handler, bytes);
    assert_eq!(handler.decrsps_calls, vec![(0, Vec::<u8>::new())]);
}

// =====================================================================
// BUG-11-027 §03 matrix — Mode 2026 inline dispatch (vte-layer pins).
//
// These pins enforce that the parser's Mode 2026 handling dispatches
// the handler INLINE within the same `advance()` call as the BSU —
// no buffering, no deferred replay. The only sync state the parser
// holds is the deadline timer used by the run loop's `select!`
// deadline arm to bound a sync window. See bug-tracker/plans/BUG-11-027/.
// =====================================================================

/// Negative pin: `SyncState` carries no byte buffer.
///
/// Asserts that `SyncState<TestSyncHandler>` has the SAME size as
/// `TestSyncHandler` alone (the only field is `timeout: T`). If a
/// future refactor re-introduces a `buffer: Vec<u8>` field (24 bytes
/// on 64-bit), the sizes diverge and this assertion fires.
///
/// Regression: BUG-11-027 §03 negative-pin (no byte-level buffering).
#[test]
fn processor_no_sync_buffer_during_active_sync() {
    use core::mem::size_of;

    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();

    // Activate sync — gives the timer something non-default to hold.
    parser.advance(&mut handler, b"\x1b[?2026h");
    assert!(
        parser.state.sync_state.timeout.pending_timeout(),
        "BSU should arm the parser-side sync timer"
    );

    // SyncState should be a thin wrapper around just `timeout: T`.
    // On HEAD it also carries a `buffer: Vec<u8>` (24 bytes on 64-bit
    // platforms), so the sizes diverge.
    assert_eq!(
        size_of::<super::processor::SyncState<TestSyncHandler>>(),
        size_of::<TestSyncHandler>(),
        "SyncState must contain only the timeout field; presence of a byte buffer adds Vec overhead"
    );
}

/// Inline-dispatch pin: BSU + DA1 + grid-mutating SGR all dispatch
/// within the SAME `advance()` call.
///
/// Asserting both the DA1 `identify_terminal` callback AND the SGR
/// `terminal_attribute` callback fired pins inline dispatch from two
/// angles — if a regression re-introduced parser termination after BSU,
/// DA1 and SGR would not reach the handler in this chunk.
///
/// Regression: BUG-11-027 §03 inline-dispatch pin.
#[test]
fn processor_dispatches_handler_inline_during_sync() {
    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();

    parser.advance(&mut handler, b"\x1b[?2026h\x1b[c\x1b[31m");

    assert!(
        handler.identity_reported,
        "DA1 must dispatch inline during sync — buffered on HEAD"
    );
    assert!(
        handler.attr.is_some(),
        "SGR must dispatch inline during sync — buffered on HEAD"
    );
}

/// ESU without a preceding BSU is a safe no-op.
///
/// After the fix the ESU dispatch arm calls `sync_timeout.clear_timeout()`
/// unconditionally for `?2026 l`. This pin proves that calling
/// `clear_timeout` on an already-disarmed timer does NOT panic and does
/// NOT leave the timer in a weird state (still disarmed).
///
/// Regression: BUG-11-027 §03 ESU-without-BSU edge case.
#[test]
fn esu_without_bsu_is_noop() {
    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();

    parser.advance(&mut handler, b"\x1b[?2026l");

    assert!(
        !parser.state.sync_state.timeout.pending_timeout(),
        "ESU on a disarmed timer must remain disarmed (no-op)"
    );
}

/// Two BSUs in one feed re-arm the timer (TestSyncHandler increments
/// its `is_sync` counter on each `set_timeout`).
///
/// Both BSUs dispatch inline through the same `advance()` call — the
/// counter advances to 2, demonstrating the second BSU reached the
/// dispatch arm rather than being deferred. The intra-chunk DA1 also
/// dispatches (`handler.identity_reported == true`), pinning the same
/// inline-dispatch invariant from a different angle.
///
/// Regression: BUG-11-027 §03 nested-BSU pin.
#[test]
fn bsu_twice_in_one_feed_extends_timeout() {
    let mut parser = Processor::<TestSyncHandler>::new();
    let mut handler = MockHandler::default();

    parser.advance(&mut handler, b"\x1b[?2026h\x1b[c\x1b[?2026h");

    assert_eq!(
        parser.state.sync_state.timeout.is_sync, 2,
        "both BSUs must reach the dispatch arm and re-arm the timer (is_sync == 2)"
    );
    assert!(
        handler.identity_reported,
        "DA1 between the two BSUs must dispatch inline (HEAD: buffered after first BSU's terminated=true)"
    );
}
