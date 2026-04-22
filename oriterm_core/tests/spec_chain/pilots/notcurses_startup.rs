//! Replay the notcurses-demo startup handshake capture and dump the
//! response stream ori_term produces.
//!
//! Regression anchor for BUG-06-017 investigation — the captured startup
//! handshake at `plans/spec-conformance/captures/notcurses-demo-intro.cap`
//! exercises every identification query (DA1/DA2/DA3, XTVERSION, XTGETTCAP,
//! DSR, OSC 4/10/11, DECRQM 2026/1016, XTSMGRAPHICS, KITTYQUERY, CSI 14t/18t,
//! DECSET 1049/1/? etc.). We assert the full reply stream is well-framed:
//! every response is a complete, terminated escape sequence — no raw
//! printable bytes that notcurses's input layer would interpret as
//! keypresses (the mechanism by which a malformed reply can quit a demo
//! via the `interrupt_demo()` path in `hud.c:288`).

use oriterm_core::effect::{Effect, PtyEffect};
use oriterm_test_support::spec_chain::{SpecHarness, pty_writes};

const NOTCURSES_STARTUP: &[u8] =
    include_bytes!("../../../../plans/spec-conformance/captures/notcurses-demo-intro.cap");

/// Feed the recorded notcurses startup bytes and return the concatenated
/// PTY reply stream.
fn replay_notcurses_startup() -> Vec<u8> {
    let mut h = SpecHarness::new();
    h.feed(NOTCURSES_STARTUP);
    let mut out = Vec::new();
    for (bytes, _) in pty_writes(&h) {
        out.extend_from_slice(bytes);
    }
    out
}

/// Every byte ori_term writes back in response to the notcurses startup
/// handshake MUST sit inside a well-formed escape sequence. Any raw
/// printable byte (especially `q`) would leak into notcurses's input
/// queue via the `ultramegaok_demo` thread in `input.c:120` and trigger
/// `interrupt_demo()` through `menu_or_hud_key`.
///
/// This is the canonical framing pin for the BUG-06-017 hypothesis
/// ("malformed reply interpreted by demo_getc as spurious user input").
#[test]
fn notcurses_startup_reply_stream_has_no_bare_printable_bytes() {
    let stream = replay_notcurses_startup();
    let framing_errors = find_out_of_frame_bytes(&stream);
    assert!(
        framing_errors.is_empty(),
        "reply stream has {} out-of-frame byte(s): {:?}\nfull stream: {:?}",
        framing_errors.len(),
        framing_errors,
        String::from_utf8_lossy(&stream),
    );
}

/// Every DA response must terminate with a final `c` (CSI) or `\x1b\\`
/// (DCS ST) and contain no `q` outside of `\x1bP+q...\x1b\\` DCS framing.
/// A stray `q` in a CSI reply would be parsed by notcurses as a key press.
#[test]
fn notcurses_startup_reply_stream_contains_no_stray_q_bytes() {
    let stream = replay_notcurses_startup();
    let stray = find_stray_q_bytes(&stream);
    assert!(
        stray.is_empty(),
        "reply stream contains {} stray 'q' byte(s) outside framing: {:?}\nfull stream: {:?}",
        stray.len(),
        stray,
        String::from_utf8_lossy(&stream),
    );
}

/// Pins: notcurses-demo startup must emit at least one PTY reply.
/// A zero-reply run would indicate the VT parser silently dropped every
/// identification query (DA1/DA2/DA3/DSR/XTVERSION/...) which is the same
/// failure mode BUG-06-017 investigates. Run with `-- --nocapture` to
/// inspect the per-effect dump. Anchor: BUG-06-017 / HYG-13.1-010.
#[test]
fn notcurses_startup_emits_at_least_one_pty_reply() {
    let mut h = SpecHarness::new();
    h.feed(NOTCURSES_STARTUP);
    let mut total = 0usize;
    let mut pty_count = 0usize;
    eprintln!("--- notcurses startup reply stream ---");
    for (i, eff) in h.outcome().effects_emitted.iter().enumerate() {
        if let Effect::Pty(PtyEffect::Write { bytes, kind }) = eff {
            pty_count += 1;
            total += bytes.len();
            eprintln!(
                "  [{i}] kind={:?} ({} bytes): {:?}",
                kind,
                bytes.len(),
                String::from_utf8_lossy(bytes),
            );
        } else {
            eprintln!("  [{i}] non-PTY: {eff:?}");
        }
    }
    eprintln!("--- {pty_count} PTY replies, {total} total reply bytes ---");
    assert!(
        pty_count > 0,
        "notcurses startup must emit at least one PTY reply ({total} total bytes)"
    );
}

/// Event emitted by [`scan_escape_stream`] for each boundary encountered
/// outside a recognised escape frame.
enum ScanEvent {
    /// A byte at `index` that fell outside any escape sequence frame.
    OutOfFrame { index: usize, byte: u8 },
    /// A bare `ESC` (0x1b) at `index` with no discriminator byte because
    /// the stream terminated. Reported once per stream at most.
    DanglingEsc { index: usize },
}

/// Walk `stream` and invoke `visitor` for every byte that sits outside a
/// recognised `ESC [` / `ESC ]` / `ESC _` / `ESC P` frame, plus a final
/// `DanglingEsc` event if the stream terminates on a bare ESC.
///
/// Framing arms:
/// - `ESC [` CSI — consume until the first 0x40..=0x7e final-byte (inclusive).
/// - `ESC ]` OSC — consume until `BEL` (0x07) or `ESC \\` terminator.
/// - `ESC _` APC / `ESC P` DCS — consume until `ESC \\` terminator.
/// - `ESC <other>` — return to out-of-frame state without emitting events for
///   the two framing bytes (matches the original scanners' behaviour for
///   unrecognised two-byte ESC sequences).
///
/// Extracted at §13.1 close-out (HYG-13.1-008) to de-duplicate the identical
/// walker skeleton that previously lived in `find_out_of_frame_bytes` and
/// `find_stray_q_bytes`.
fn scan_escape_stream<F: FnMut(ScanEvent)>(stream: &[u8], mut visitor: F) {
    let mut i = 0;
    while i < stream.len() {
        let b = stream[i];
        if b == 0x1b {
            i += 1;
            if i >= stream.len() {
                visitor(ScanEvent::DanglingEsc { index: i - 1 });
                break;
            }
            let start_kind = stream[i];
            i += 1;
            match start_kind {
                b'[' => {
                    while i < stream.len() && !(0x40..=0x7e).contains(&stream[i]) {
                        i += 1;
                    }
                    if i < stream.len() {
                        i += 1;
                    }
                }
                b']' => {
                    while i < stream.len() {
                        if stream[i] == 0x07 {
                            i += 1;
                            break;
                        }
                        if stream[i] == 0x1b && i + 1 < stream.len() && stream[i + 1] == b'\\' {
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                }
                b'_' | b'P' => {
                    while i < stream.len() {
                        if stream[i] == 0x1b && i + 1 < stream.len() && stream[i + 1] == b'\\' {
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                }
                _ => {}
            }
        } else {
            visitor(ScanEvent::OutOfFrame { index: i, byte: b });
            i += 1;
        }
    }
}

/// Scan the stream and return every byte index that falls outside a
/// recognised escape-sequence frame. A byte is considered "in frame" if it
/// sits between an opening `ESC [` / `ESC ]` / `ESC _` / `ESC P` and its
/// terminator (final char for CSI, `\x07`/`ESC \\` for OSC, `ESC \\` for
/// APC/DCS). A trailing bare `ESC` is reported as `(index, 0x1b)`.
fn find_out_of_frame_bytes(stream: &[u8]) -> Vec<(usize, u8)> {
    let mut errs = Vec::new();
    scan_escape_stream(stream, |evt| match evt {
        ScanEvent::OutOfFrame { index, byte } => errs.push((index, byte)),
        ScanEvent::DanglingEsc { index } => errs.push((index, 0x1b)),
    });
    errs
}

/// Scan the stream and return every `q` byte that is outside any recognised
/// escape-sequence frame. `q` bytes INSIDE a `\x1bP+q...\x1b\\` DCS request
/// or `\x1b[>0q` XTVERSION reply are fine; stray `q` in raw output is not.
fn find_stray_q_bytes(stream: &[u8]) -> Vec<usize> {
    let mut stray = Vec::new();
    scan_escape_stream(stream, |evt| {
        if let ScanEvent::OutOfFrame { index, byte: b'q' } = evt {
            stray.push(index);
        }
    });
    stray
}

// ============================================================================
// Unit tests for scan_escape_stream (HYG-13.1-008 extraction)
// ============================================================================

/// Pins: scan_escape_stream skips bytes inside a CSI frame (no OutOfFrame
/// events emitted for parameter bytes or the final letter). Anchor: HYG-13.1-008.
#[test]
fn scan_escape_stream_skips_csi_frame_content() {
    let stream = b"\x1b[2;3R"; // CSI 2;3R — a DSR cursor reply
    let mut events = Vec::new();
    scan_escape_stream(stream, |evt| match evt {
        ScanEvent::OutOfFrame { index, byte } => events.push((index, byte)),
        ScanEvent::DanglingEsc { .. } => panic!("unexpected dangling ESC"),
    });
    assert!(
        events.is_empty(),
        "CSI frame content should not emit events: {events:?}"
    );
}

/// Pins: scan_escape_stream skips bytes inside an OSC frame terminated by
/// BEL (0x07). Anchor: HYG-13.1-008.
#[test]
fn scan_escape_stream_skips_osc_bel_frame_content() {
    let stream = b"\x1b]11;rgb:00/00/00\x07";
    let mut events = Vec::new();
    scan_escape_stream(stream, |evt| match evt {
        ScanEvent::OutOfFrame { index, byte } => events.push((index, byte)),
        ScanEvent::DanglingEsc { .. } => panic!("unexpected dangling ESC"),
    });
    assert!(
        events.is_empty(),
        "OSC BEL-terminated content should not emit events: {events:?}"
    );
}

/// Pins: scan_escape_stream skips bytes inside an OSC frame terminated by
/// ESC \\ (ST). Anchor: HYG-13.1-008.
#[test]
fn scan_escape_stream_skips_osc_st_frame_content() {
    let stream = b"\x1b]10;rgb:ff/ff/ff\x1b\\";
    let mut events = Vec::new();
    scan_escape_stream(stream, |evt| match evt {
        ScanEvent::OutOfFrame { index, byte } => events.push((index, byte)),
        ScanEvent::DanglingEsc { .. } => panic!("unexpected dangling ESC"),
    });
    assert!(
        events.is_empty(),
        "OSC ST-terminated content should not emit events: {events:?}"
    );
}

/// Pins: scan_escape_stream skips bytes inside an APC frame terminated by
/// ESC \\. Anchor: HYG-13.1-008.
#[test]
fn scan_escape_stream_skips_apc_frame_content() {
    let stream = b"\x1b_Gi=1;OK\x1b\\"; // kitty graphics reply shape
    let mut events = Vec::new();
    scan_escape_stream(stream, |evt| match evt {
        ScanEvent::OutOfFrame { index, byte } => events.push((index, byte)),
        ScanEvent::DanglingEsc { .. } => panic!("unexpected dangling ESC"),
    });
    assert!(
        events.is_empty(),
        "APC frame content should not emit events: {events:?}"
    );
}

/// Pins: scan_escape_stream skips bytes inside a DCS frame terminated by
/// ESC \\. Anchor: HYG-13.1-008.
#[test]
fn scan_escape_stream_skips_dcs_frame_content() {
    let stream = b"\x1bP>|ori_term 0.1.0\x1b\\"; // XTVERSION-style reply shape
    let mut events = Vec::new();
    scan_escape_stream(stream, |evt| match evt {
        ScanEvent::OutOfFrame { index, byte } => events.push((index, byte)),
        ScanEvent::DanglingEsc { .. } => panic!("unexpected dangling ESC"),
    });
    assert!(
        events.is_empty(),
        "DCS frame content should not emit events: {events:?}"
    );
}

/// Pins: scan_escape_stream emits OutOfFrame for every raw byte before the
/// first ESC framing byte. Anchor: HYG-13.1-008.
#[test]
fn scan_escape_stream_emits_out_of_frame_for_raw_bytes() {
    let stream = b"abc\x1b[0m";
    let mut events = Vec::new();
    scan_escape_stream(stream, |evt| match evt {
        ScanEvent::OutOfFrame { index, byte } => events.push((index, byte)),
        ScanEvent::DanglingEsc { .. } => panic!("unexpected dangling ESC"),
    });
    assert_eq!(events, vec![(0, b'a'), (1, b'b'), (2, b'c')]);
}

/// Pins: scan_escape_stream emits OutOfFrame for a stray `q` byte after a
/// terminated CSI frame. Negative pin for find_stray_q_bytes. Anchor: HYG-13.1-008.
#[test]
fn scan_escape_stream_emits_out_of_frame_for_stray_q_after_csi() {
    let stream = b"\x1b[0mq"; // CSI SGR reset, then stray q
    let mut events = Vec::new();
    scan_escape_stream(stream, |evt| match evt {
        ScanEvent::OutOfFrame { index, byte } => events.push((index, byte)),
        ScanEvent::DanglingEsc { .. } => panic!("unexpected dangling ESC"),
    });
    assert_eq!(events, vec![(4, b'q')]);
}

/// Pins: scan_escape_stream emits DanglingEsc for a trailing bare ESC and
/// does NOT also emit an OutOfFrame for the ESC byte. Anchor: HYG-13.1-008.
#[test]
fn scan_escape_stream_emits_dangling_esc_at_end_of_stream() {
    let stream = b"\x1b[0m\x1b";
    let mut dangling = None;
    let mut out_of_frame = Vec::new();
    scan_escape_stream(stream, |evt| match evt {
        ScanEvent::OutOfFrame { index, byte } => out_of_frame.push((index, byte)),
        ScanEvent::DanglingEsc { index } => {
            assert!(dangling.is_none(), "should emit at most one DanglingEsc");
            dangling = Some(index);
        }
    });
    assert_eq!(dangling, Some(4));
    assert!(
        out_of_frame.is_empty(),
        "dangling ESC path should not also emit OutOfFrame: {out_of_frame:?}"
    );
}

/// Pins: find_out_of_frame_bytes reports the dangling ESC as `(index, 0x1b)`
/// — the `Vec<(usize, u8)>` shape must survive the HYG-13.1-008 refactor.
#[test]
fn find_out_of_frame_bytes_reports_dangling_esc_as_0x1b() {
    let stream = b"\x1b[0m\x1b";
    let errs = find_out_of_frame_bytes(stream);
    assert_eq!(errs, vec![(4, 0x1b)]);
}

/// Pins: find_stray_q_bytes ignores a dangling ESC (not a q byte). Negative
/// pin for the HYG-13.1-008 refactor — DanglingEsc must not leak into the
/// stray-q output.
#[test]
fn find_stray_q_bytes_ignores_dangling_esc() {
    let stream = b"\x1b[0m\x1b";
    let stray = find_stray_q_bytes(stream);
    assert!(stray.is_empty(), "dangling ESC is not a stray q: {stray:?}");
}

/// Pins: find_stray_q_bytes flags a bare `q` between frames but ignores a
/// `q` inside a DCS frame. Regression pin for the HYG-13.1-008 refactor.
#[test]
fn find_stray_q_bytes_separates_in_frame_vs_out_of_frame_q() {
    // DCS request `\x1bP+q...\x1b\\` carries an in-frame `q`; the trailing
    // standalone `q` is the only stray byte.
    let stream = b"\x1bP+qTN\x1b\\q";
    let stray = find_stray_q_bytes(stream);
    assert_eq!(stray, vec![stream.len() - 1]);
}
