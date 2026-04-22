//! Tests for the `effect_filters` helper surface.
//!
//! Every helper gets a positive test (exercises the happy path) and a
//! negative pin (proves the filter does not over-match or silently
//! aggregate unrelated writes). DA1 (`CSI c`) and DSR cursor-position
//! (`CSI 6 n`) are the fixture sequences — both drive the production
//! handler path and emit distinct `PtyWriteKind` variants, letting the
//! tests verify kind-based filtering without contrived setup.

use oriterm_core::effect::PtyWriteKind;

use super::{
    assert_no_pty_writes, count_exact_pty_writes, expect_single_pty_write, last_pty_write,
    pty_write_concat, pty_write_contains, pty_writes, pty_writes_of_kind,
};
use crate::spec_chain::SpecHarness;

/// DA1 reply bytes — VT420 + ANSI color + sixel attribute string.
const DA1_REPLY: &[u8] = b"\x1b[?64;6;4c";

/// DSR cursor-position reply for an 80×24 terminal sitting at the home
/// cell (row 1, column 1). Matches the default `SpecHarness::new()`
/// dimensions.
const DSR_CPR_HOME: &[u8] = b"\x1b[1;1R";

fn harness_with_da1() -> SpecHarness {
    let mut h = SpecHarness::new();
    h.feed(b"\x1b[c");
    h
}

fn harness_with_da1_and_dsr() -> SpecHarness {
    let mut h = SpecHarness::new();
    h.feed(b"\x1b[c"); // DA1 → DeviceAttribute
    h.feed(b"\x1b[6n"); // DSR CPR → CursorReport
    h
}

/// Pins: `pty_writes` yields every `Effect::Pty(PtyEffect::Write)` in arrival
/// order with its `(bytes, PtyWriteKind)` pair intact, skipping non-`Pty`
/// effects. Anchor: HYG-13.1-002 §13.1 close-out.
#[test]
fn pty_writes_yields_all_pty_write_effects_with_kind() {
    let h = harness_with_da1_and_dsr();
    let writes: Vec<(Vec<u8>, PtyWriteKind)> =
        pty_writes(&h).map(|(b, k)| (b.to_vec(), k)).collect();
    assert_eq!(writes.len(), 2, "two PTY writes emitted (DA1 + DSR CPR)");
    assert_eq!(writes[0].0, DA1_REPLY);
    assert_eq!(writes[0].1, PtyWriteKind::DeviceAttribute);
    assert_eq!(writes[1].0, DSR_CPR_HOME);
    assert_eq!(writes[1].1, PtyWriteKind::CursorReport);
}

/// Pins: a fresh harness with no sequences fed yields an empty `pty_writes`
/// iterator — proves the helper reads the current transcript, not cached
/// state. Anchor: HYG-13.1-002 §13.1 close-out.
#[test]
fn pty_writes_yields_empty_when_harness_emitted_no_writes() {
    let h = SpecHarness::new();
    assert_eq!(pty_writes(&h).count(), 0);
}

/// Pins: `pty_writes_of_kind` applies the kind filter AND yields bytes only
/// (not the tuple), so two commands emitting distinct kinds partition into
/// two disjoint per-kind iterators. Anchor: HYG-13.1-002 §13.1 close-out.
#[test]
fn pty_writes_of_kind_filters_by_kind() {
    let h = harness_with_da1_and_dsr();
    let da: Vec<&[u8]> = pty_writes_of_kind(&h, PtyWriteKind::DeviceAttribute).collect();
    assert_eq!(da, vec![DA1_REPLY]);
    let cpr: Vec<&[u8]> = pty_writes_of_kind(&h, PtyWriteKind::CursorReport).collect();
    assert_eq!(cpr, vec![DSR_CPR_HOME]);
}

/// Negative pin: a kind with no matching writes yields an empty
/// iterator. Proves the filter is not leaking unrelated writes through.
/// Anchor: HYG-13.1-002 §13.1 close-out.
#[test]
fn pty_writes_of_kind_yields_empty_for_unrelated_kind() {
    let h = harness_with_da1();
    let count = pty_writes_of_kind(&h, PtyWriteKind::ImageProtocolReply).count();
    assert_eq!(count, 0, "DA1 must not leak as an ImageProtocolReply");
}

/// Pins: `pty_write_concat` joins every filtered write's bytes into one
/// `Vec<u8>` — the canonical "full transcript for assertion messages" use
/// case HYG-13.1-002 called out. Anchor: HYG-13.1-002 §13.1 close-out.
#[test]
fn pty_write_concat_joins_all_writes_of_kind() {
    let h = harness_with_da1();
    let concat = pty_write_concat(&h, PtyWriteKind::DeviceAttribute);
    assert_eq!(concat, DA1_REPLY);
}

/// Negative pin: concat returns empty when no writes of the requested
/// kind exist. Proves an unrelated kind does not dump DA1's bytes into
/// the output buffer. Anchor: HYG-13.1-002 §13.1 close-out.
#[test]
fn pty_write_concat_returns_empty_for_unrelated_kind() {
    let h = harness_with_da1();
    let concat = pty_write_concat(&h, PtyWriteKind::ImageProtocolReply);
    assert!(
        concat.is_empty(),
        "unrelated-kind concat must be empty; got {concat:?}",
    );
}

/// Pins: `pty_write_contains` locates a needle as a byte subsequence via
/// windowed search (`slice::windows(needle.len()).any(== needle)`). Anchor:
/// HYG-13.1-002 §13.1 close-out.
#[test]
fn pty_write_contains_finds_needle_via_windowed_search() {
    let h = harness_with_da1();
    assert!(pty_write_contains(
        &h,
        PtyWriteKind::DeviceAttribute,
        b"?64;6;4",
    ));
}

/// Pins: a needle not present in any write of the given kind returns false.
/// Paired with `pty_write_contains_finds_needle_via_windowed_search` to
/// clamp both directions of the subsequence check. Anchor: HYG-13.1-002
/// §13.1 close-out.
#[test]
fn pty_write_contains_rejects_needle_absent_from_transcript() {
    let h = harness_with_da1();
    assert!(!pty_write_contains(
        &h,
        PtyWriteKind::DeviceAttribute,
        b"?9001;",
    ));
}

/// Negative pin: a needle that matches a DIFFERENT kind's bytes must
/// not falsely match under the filtered kind. Proves the kind filter
/// runs BEFORE the byte search. Anchor: HYG-13.1-002 §13.1 close-out.
#[test]
fn pty_write_contains_kind_filter_runs_before_byte_search() {
    let h = harness_with_da1_and_dsr();
    // `1;1R` bytes exist in the CursorReport reply — but asking
    // `DeviceAttribute` must return false because DA1's bytes do not
    // contain that substring.
    assert!(!pty_write_contains(
        &h,
        PtyWriteKind::DeviceAttribute,
        b"1;1R",
    ));
    // The same needle DOES match under CursorReport.
    assert!(pty_write_contains(&h, PtyWriteKind::CursorReport, b"1;1R"));
}

/// Empty needle short-circuits to "any write of kind exists" per the
/// helper's documented vacuous-match contract (`std::slice::windows(0)`
/// panics; this helper explicitly does not). Anchor: HYG-13.1-002 §13.1
/// close-out.
#[test]
fn pty_write_contains_empty_needle_matches_when_any_write_of_kind_exists() {
    let h = harness_with_da1();
    assert!(pty_write_contains(&h, PtyWriteKind::DeviceAttribute, b""));
    assert!(!pty_write_contains(
        &h,
        PtyWriteKind::ImageProtocolReply,
        b"",
    ));
}

/// Pins: `count_exact_pty_writes` tallies writes whose bytes equal
/// `expected` byte-for-byte — the §13.1 TPR round 0 "multi-step tests
/// satisfied by aggregated transcripts" regression guard. Three DA1
/// queries produce three distinct DA1 reply writes, so exact-count must
/// return 3 (not 1-because-concat, not 0-because-not-found). Anchor:
/// HYG-13.1-002 §13.1 close-out.
#[test]
fn count_exact_pty_writes_counts_byte_for_byte_matches() {
    let mut h = SpecHarness::new();
    h.feed(b"\x1b[c\x1b[c\x1b[c"); // three DA1 queries
    let got = count_exact_pty_writes(&h, PtyWriteKind::DeviceAttribute, DA1_REPLY);
    assert_eq!(got, 3, "three DA1 queries produce three exact replies");
}

/// Negative pin: a near-match (same kind, different bytes) must NOT
/// count. Proves the count uses exact byte comparison, not a prefix or
/// loose match. Anchor: HYG-13.1-002 §13.1 close-out.
#[test]
fn count_exact_pty_writes_rejects_near_match_bytes() {
    let h = harness_with_da1();
    let got = count_exact_pty_writes(&h, PtyWriteKind::DeviceAttribute, b"\x1b[?64;6c");
    assert_eq!(
        got, 0,
        "prefix-only match must not count; exact byte equality required",
    );
}

/// Pins: `last_pty_write` returns the cloned (bytes, kind) of the most
/// recent write, or `None` when the transcript is empty. Anchor:
/// HYG-13.1-002 §13.1 close-out.
#[test]
fn last_pty_write_returns_most_recent_write_cloned() {
    let h = harness_with_da1_and_dsr();
    let (bytes, kind) = last_pty_write(&h).expect("transcript has two writes");
    assert_eq!(bytes, DSR_CPR_HOME);
    assert_eq!(kind, PtyWriteKind::CursorReport);
}

/// Negative pin: `last_pty_write` returns `None` on an empty transcript.
/// Proves the helper does not panic when no writes have been emitted.
/// Anchor: HYG-13.1-002 §13.1 close-out.
#[test]
fn last_pty_write_returns_none_on_empty_transcript() {
    let h = SpecHarness::new();
    assert!(last_pty_write(&h).is_none());
}

/// Pins: `expect_single_pty_write` returns the cloned bytes when exactly
/// one write is on the transcript. Anchor: HYG-13.1-002 §13.1 close-out.
#[test]
fn expect_single_pty_write_returns_bytes_when_exactly_one_write() {
    let h = harness_with_da1();
    let bytes = expect_single_pty_write(&h);
    assert_eq!(bytes, DA1_REPLY);
}

/// Pins: `assert_no_pty_writes` passes when the transcript is empty.
/// Anchor: HYG-13.1-002 §13.1 close-out.
#[test]
fn assert_no_pty_writes_passes_on_empty_transcript() {
    let h = SpecHarness::new();
    assert_no_pty_writes(&h);
}

/// Negative pin: an expected payload that matches a different kind's
/// bytes must return zero under the filtered kind. Pairs with the
/// `pty_write_contains_kind_filter_runs_before_byte_search` pin for
/// the exact-count path. Anchor: HYG-13.1-002 §13.1 close-out.
#[test]
fn count_exact_pty_writes_kind_filter_runs_before_byte_compare() {
    let h = harness_with_da1_and_dsr();
    let got = count_exact_pty_writes(&h, PtyWriteKind::DeviceAttribute, DSR_CPR_HOME);
    assert_eq!(
        got, 0,
        "DSR bytes under DA kind filter must not count — kind filter precedes byte compare",
    );
}
