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
use oriterm_test_support::spec_chain::SpecHarness;

const NOTCURSES_STARTUP: &[u8] =
    include_bytes!("../../../../plans/spec-conformance/captures/notcurses-demo-intro.cap");

/// Feed the recorded notcurses startup bytes and return the concatenated
/// PTY reply stream.
fn replay_notcurses_startup() -> Vec<u8> {
    let mut h = SpecHarness::new();
    h.feed(NOTCURSES_STARTUP);
    let mut out = Vec::new();
    for eff in &h.outcome().effects_emitted {
        if let Effect::Pty(PtyEffect::Write { bytes, .. }) = eff {
            out.extend_from_slice(bytes);
        }
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

/// Dump the reply stream to stderr — run with `-- --nocapture` to inspect.
/// Not a pin; purely diagnostic for future BUG-06-017 / notcurses debugging.
#[test]
fn dump_notcurses_startup_reply_stream() {
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
}

/// Scan the stream and return every byte index that falls outside a
/// recognised escape-sequence frame. A byte is considered "in frame" if it
/// sits between an opening `ESC [` / `ESC ]` / `ESC _` / `ESC P` and its
/// terminator (final char for CSI, `\x07`/`ESC \\` for OSC, `ESC \\` for
/// APC/DCS).
fn find_out_of_frame_bytes(stream: &[u8]) -> Vec<(usize, u8)> {
    let mut errs = Vec::new();
    let mut i = 0;
    while i < stream.len() {
        let b = stream[i];
        if b == 0x1b {
            i += 1;
            if i >= stream.len() {
                errs.push((i - 1, 0x1b));
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
            errs.push((i, b));
            i += 1;
        }
    }
    errs
}

/// Scan the stream and return every `q` byte that is outside any recognised
/// escape-sequence frame. `q` bytes INSIDE a `\x1bP+q...\x1b\\` DCS request
/// or `\x1b[>0q` XTVERSION reply are fine; stray `q` in raw output is not.
fn find_stray_q_bytes(stream: &[u8]) -> Vec<usize> {
    let mut stray = Vec::new();
    let mut i = 0;
    while i < stream.len() {
        let b = stream[i];
        if b == 0x1b && i + 1 < stream.len() {
            let start_kind = stream[i + 1];
            i += 2;
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
            if b == b'q' {
                stray.push(i);
            }
            i += 1;
        }
    }
    stray
}
