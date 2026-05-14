//! Tests for XTGETTCAP (DCS `+q` Pt ST) reply emission.
//!
//! Test matrix: 13-cap coverage (notcurses + fish probes + Section 38.4
//! minimum-viable extras), edge cases (empty payload, malformed hex,
//! abort, mixed known+unknown), and a byte-exact grouped-reply pin.

use super::super::test_helpers::{feed, term_with_recorder};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Feed an XTGETTCAP query and return the recorded PtyWrite event strings.
///
/// Constructs a fresh `Term + RecordingListener`, drives the canonical
/// DCS sequence `\x1bP+q<payload>\x1b\\` through the VTE processor, and
/// returns the `PtyWrite(...)` event strings (one per ori_term reply).
fn xtgettcap_replies(query_payload: &[u8]) -> Vec<String> {
    let (mut term, listener) = term_with_recorder();
    let mut full = b"\x1bP+q".to_vec();
    full.extend_from_slice(query_payload);
    full.extend_from_slice(b"\x1b\\");
    feed(&mut term, &full);
    listener
        .events()
        .into_iter()
        .filter(|e| e.starts_with("PtyWrite("))
        .collect()
}

/// Concatenate all reply event strings for substring assertion.
fn concat(events: &[String]) -> String {
    events.join("")
}

// ---------------------------------------------------------------------------
// Handler-level reply tests — one per cap (13 total per §02 consensus)
// ---------------------------------------------------------------------------

/// Regression: BUG-06-074 §03 — query `544E` → reply contains hex of `oriterm`.
#[test]
fn xtgettcap_single_tn_returns_oriterm_hex() {
    let events = xtgettcap_replies(b"544E");
    let s = concat(&events);
    assert!(
        s.contains("\x1bP1+r544E=6F72697465726D\x1b\\"),
        "expected canonical TN=oriterm reply, got: {:?}",
        s
    );
}

/// Regression: BUG-06-074 §03 — query `524742` → reply contains hex of `8/8/8`.
#[test]
fn xtgettcap_single_rgb_returns_8_8_8_hex() {
    let events = xtgettcap_replies(b"524742");
    let s = concat(&events);
    assert!(
        s.contains("\x1bP1+r524742=382F382F38\x1b\\"),
        "expected canonical RGB=8/8/8 reply, got: {:?}",
        s
    );
}

/// Regression: BUG-06-074 §03 — query `436F` (`Co`) → reply contains hex of `256`.
#[test]
fn xtgettcap_single_co_returns_256_hex() {
    let events = xtgettcap_replies(b"436F");
    let s = concat(&events);
    assert!(
        s.contains("\x1bP1+r436F=323536\x1b\\"),
        "expected canonical Co=256 reply, got: {:?}",
        s
    );
}

/// Regression: BUG-06-074 §03 — query `colors` hex → reply hex of `256`.
#[test]
fn xtgettcap_single_colors_returns_256_hex() {
    // "colors" = 63 6F 6C 6F 72 73 → uppercase canonical hex 636F6C6F7273
    let events = xtgettcap_replies(b"636F6C6F7273");
    let s = concat(&events);
    assert!(
        s.contains("\x1bP1+r636F6C6F7273=323536\x1b\\"),
        "expected canonical colors=256 reply, got: {:?}",
        s
    );
}

/// Regression: BUG-06-074 §03 (R1 + R3) — query `687061` (`hpa`) → reply hex of `\x1b[%i%p1%dG`.
#[test]
fn xtgettcap_single_hpa_returns_csi_template_hex() {
    let events = xtgettcap_replies(b"687061");
    let s = concat(&events);
    assert!(
        s.contains("\x1bP1+r687061=1B5B2569257031256447\x1b\\"),
        "expected canonical hpa reply, got: {:?}",
        s
    );
}

/// Regression: BUG-06-074 — fish probe `indn` returns the canonical
/// `\x1b[%p1%dS` CSI template hex-encoded.
#[test]
fn xtgettcap_single_indn_returns_csi_template_hex() {
    // "indn" = 69 6E 64 6E → uppercase hex 696E646E
    let events = xtgettcap_replies(b"696E646E");
    let s = concat(&events);
    // "\x1b[%p1%dS" = 1B 5B 25 70 31 25 64 53 → "1B5B2570312564 53"
    assert!(
        s.contains("\x1bP1+r696E646E=1B5B25703125645"),
        "expected canonical indn reply prefix, got: {:?}",
        s
    );
}

/// Regression: BUG-06-074 — fish probe `query-os-name` returns the HOST OS
/// NAME (Linux/Darwin/Windows), NOT the terminal name.
#[test]
fn xtgettcap_single_query_os_name_returns_host_os_hex() {
    // "query-os-name" = 71 75 65 72 79 2D 6F 73 2D 6E 61 6D 65 → uppercase hex
    // 71756572792D6F732D6E616D65
    let events = xtgettcap_replies(b"71756572792D6F732D6E616D65");
    let s = concat(&events);
    let expected_os = match std::env::consts::OS {
        "linux" => "Linux",
        "macos" => "Darwin",
        "windows" => "Windows",
        _ => "Unknown",
    };
    let expected_hex = hex_encode_upper(expected_os.as_bytes());
    let expected_prefix =
        format!("\x1bP1+r71756572792D6F732D6E616D65={}\x1b\\", expected_hex);
    assert!(
        s.contains(&expected_prefix),
        "expected query-os-name={} reply ({:?}), got: {:?}",
        expected_os,
        expected_prefix,
        s
    );
}

/// Regression: BUG-06-074 §03 — query `Smulx` → reply hex of `\x1b[4:%p1%dm`.
#[test]
fn xtgettcap_single_smulx_returns_sgr_4_colon_template() {
    // "Smulx" = 53 6D 75 6C 78 → uppercase hex 536D756C78
    let events = xtgettcap_replies(b"536D756C78");
    let s = concat(&events);
    // "\x1b[4:%p1%dm" = 1B 5B 34 3A 25 70 31 25 64 6D → "1B5B343A25703125 64 6D"
    assert!(
        s.contains("\x1bP1+r536D756C78=1B5B343A25703125646D\x1b\\"),
        "expected canonical Smulx reply, got: {:?}",
        s
    );
}

/// Regression: BUG-06-074 §03 — query `Ms` (clipboard OSC 52).
#[test]
fn xtgettcap_single_ms_returns_osc_52_hex() {
    // "Ms" = 4D 73 → uppercase hex 4D73
    let events = xtgettcap_replies(b"4D73");
    let s = concat(&events);
    assert!(
        s.contains("\x1bP1+r4D73="),
        "expected canonical Ms reply prefix, got: {:?}",
        s
    );
}

/// Regression: BUG-06-074 §03 — unknown cap → DCS 0+r reply.
#[test]
fn xtgettcap_unknown_cap_returns_dcs_0_plus_r() {
    // "99" is not a valid cap name (it's hex digits, decodes to byte 0x99 which is no known cap)
    let events = xtgettcap_replies(b"99");
    let s = concat(&events);
    assert!(
        s.contains("\x1bP0+r99\x1b\\"),
        "expected DCS 0+r for unknown cap '99', got: {:?}",
        s
    );
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

/// Regression: BUG-06-074 §03 — exact notcurses query `544e;524742;687061` (lowercase TN).
/// Reply is single grouped DCS 1+r with all three known caps in order.
#[test]
fn xtgettcap_notcurses_full_query_all_known() {
    // Lowercase hex as notcurses actually sends per the capture
    let events = xtgettcap_replies(b"544e;524742;687061");
    let s = concat(&events);
    assert!(
        s.contains("\x1bP1+r544E=6F72697465726D;524742=382F382F38;687061=1B5B2569257031256447\x1b\\"),
        "expected canonical TN;RGB;hpa grouped reply with UPPERCASE hex names, got: {:?}",
        s
    );
    assert_eq!(events.len(), 1, "expected exactly one PtyWrite reply, got {}: {:?}", events.len(), events);
}

/// Regression: BUG-06-074 §03 — mixed known+unknown caps split into 2 replies.
#[test]
fn xtgettcap_multi_cap_mixed_known_unknown() {
    // TN (known); 99 (unknown); RGB (known); AB (unknown)
    let events = xtgettcap_replies(b"544E;99;524742;AB");
    assert_eq!(
        events.len(),
        2,
        "expected 2 PtyWrite replies (known + unknown), got {}: {:?}",
        events.len(),
        events
    );
    let known_reply = &events[0];
    let unknown_reply = &events[1];
    assert!(
        known_reply.contains("\x1bP1+r"),
        "first reply should be DCS 1+r, got: {:?}",
        known_reply
    );
    assert!(
        known_reply.contains("544E="),
        "known reply should contain TN, got: {:?}",
        known_reply
    );
    assert!(
        known_reply.contains("524742="),
        "known reply should contain RGB, got: {:?}",
        known_reply
    );
    assert!(
        unknown_reply.contains("\x1bP0+r"),
        "second reply should be DCS 0+r, got: {:?}",
        unknown_reply
    );
    assert!(
        unknown_reply.contains("99"),
        "unknown reply should contain '99', got: {:?}",
        unknown_reply
    );
    assert!(
        unknown_reply.contains("AB"),
        "unknown reply should contain 'AB', got: {:?}",
        unknown_reply
    );
}

/// Regression: BUG-06-074 §03 — empty payload → no reply (graceful no-op).
#[test]
fn xtgettcap_empty_payload() {
    let events = xtgettcap_replies(b"");
    assert!(events.is_empty(), "expected zero replies, got: {:?}", events);
}

/// Regression: BUG-06-074 §03 — empty chunk in multi-cap query is skipped.
#[test]
fn xtgettcap_empty_chunk_in_multi() {
    let events = xtgettcap_replies(b"544E;;524742");
    let s = concat(&events);
    assert!(
        s.contains("\x1bP1+r544E=6F72697465726D;524742=382F382F38\x1b\\"),
        "expected TN + RGB reply (empty chunk skipped), got: {:?}",
        s
    );
}

/// Regression: BUG-06-074 §03 — malformed odd-length hex → cap skipped.
#[test]
fn xtgettcap_malformed_hex_odd_length() {
    // "544" is 3 hex chars — odd length, can't decode
    let events = xtgettcap_replies(b"544");
    assert!(events.is_empty(), "expected zero replies, got: {:?}", events);
}

/// Regression: BUG-06-074 §03 — malformed non-hex char → cap skipped.
#[test]
fn xtgettcap_malformed_hex_non_hex_char() {
    // "GG" contains 'G' which is not a hex digit
    let events = xtgettcap_replies(b"GG");
    assert!(events.is_empty(), "expected zero replies, got: {:?}", events);
}

// ---------------------------------------------------------------------------
// Hex codec case-insensitivity
// ---------------------------------------------------------------------------

/// Regression: BUG-06-074 §03 — uppercase hex decode (xterm canon).
#[test]
fn hex_decode_tn_uppercase_input_yields_reply() {
    let events = xtgettcap_replies(b"544E");
    assert!(!events.is_empty(), "uppercase TN should decode");
}

/// Regression: BUG-06-074 §03 — lowercase hex decode (notcurses sends lowercase).
#[test]
fn hex_decode_tn_lowercase_input_yields_reply() {
    let events = xtgettcap_replies(b"544e");
    assert!(!events.is_empty(), "lowercase TN should also decode");
}

// ---------------------------------------------------------------------------
// Byte-exact grouped reply (the load-bearing regression guard)
// ---------------------------------------------------------------------------

/// Regression: BUG-06-074 — byte-exact grouped reply for the exact
/// notcurses query. Pins reply ORDERING + grouped envelope + per-cap hex
/// encoding correctness in one assertion. A reply with reversed ordering,
/// missing trailing ST, or wrong hex case fails this assertion.
#[test]
fn xtgettcap_notcurses_query_emits_canonical_grouped_reply() {
    let events = xtgettcap_replies(b"544e;524742;687061");
    let s = concat(&events);
    let expected =
        "\x1bP1+r544E=6F72697465726D;524742=382F382F38;687061=1B5B2569257031256447\x1b\\";
    assert!(
        s.contains(expected),
        "Byte-exact grouped reply mismatch.\nExpected: {:?}\nGot: {:?}",
        expected,
        s
    );
}

// ---------------------------------------------------------------------------
// Local helpers
// ---------------------------------------------------------------------------

fn hex_encode_upper(input: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(input.len() * 2);
    for &b in input {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0F) as usize] as char);
    }
    out
}
