//! PTY write filter helpers for `spec_chain` tests.
//!
//! Canonical home for the `effects_emitted.iter().filter_map(|e| matches!(e,
//! Effect::Pty(PtyEffect::Write { bytes,.. }))...)` skeleton — a pattern
//! that was independently re-implemented across the `spec_chain` suite
//! (§08 baseline, §09 DEC private modes, §09A DEC presentation, §10 OSC,
//! §12 sixel, §13 kitty, DEC rect ops, pilots). Consolidated per
//! §Algorithmic DRY — 3+ instances
//! mandate extraction.
//!
//! All helpers borrow the harness; callers that need owning storage
//! collect the iterator output explicitly.

use oriterm_core::effect::{Effect, PtyEffect, PtyWriteKind};

use crate::spec_chain::SpecHarness;

/// Iterate every `Effect::Pty(PtyEffect::Write { bytes, kind })` in
/// arrival order, yielding `(bytes, kind)` pairs.
///
/// Non-`Pty` effects (`Host`, `HostRequest`, `Ui`, `Presentation`) are
/// skipped. Callers that need a specific `PtyWriteKind` should use
/// [`pty_writes_of_kind`] instead.
pub fn pty_writes(h: &SpecHarness) -> impl Iterator<Item = (&[u8], PtyWriteKind)> {
    h.outcome()
        .effects_emitted
        .iter()
        .filter_map(|eff| match eff {
            Effect::Pty(PtyEffect::Write { bytes, kind }) => Some((bytes.as_slice(), *kind)),
            _ => None,
        })
}

/// Iterate every PTY write of `kind` in arrival order, yielding bytes
/// only.
///
/// Equivalent to `pty_writes(h).filter(|(_, k)| *k == kind).map(|(b, _)| b)`
/// but with the filter baked into the iterator so callers do not have to
/// re-implement the matches arm.
pub fn pty_writes_of_kind(h: &SpecHarness, kind: PtyWriteKind) -> impl Iterator<Item = &[u8]> {
    pty_writes(h).filter_map(move |(bytes, k)| (k == kind).then_some(bytes))
}

/// Concatenate every PTY write of `kind` into a single `Vec<u8>`.
///
/// Useful when assertion messages need the full transcript on failure,
/// or when a single logical reply was split across multiple writes.
pub fn pty_write_concat(h: &SpecHarness, kind: PtyWriteKind) -> Vec<u8> {
    let mut out = Vec::new();
    for bytes in pty_writes_of_kind(h, kind) {
        out.extend_from_slice(bytes);
    }
    out
}

/// True iff any PTY write of `kind` contains `needle` as a byte
/// subsequence (windowed search).
///
/// An empty `needle` short-circuits to "any write of `kind` exists" —
/// the vacuous-match reading ("every byte string contains the empty
/// string"). `std::slice::windows(0)` panics; this helper explicitly
/// does not, so callers that pass a runtime-computed empty needle get
/// a defined result instead of a test-time crash.
pub fn pty_write_contains(h: &SpecHarness, kind: PtyWriteKind, needle: &[u8]) -> bool {
    if needle.is_empty() {
        return pty_writes_of_kind(h, kind).next().is_some();
    }
    pty_writes_of_kind(h, kind).any(|bytes| bytes.windows(needle.len()).any(|w| w == needle))
}

/// Count the PTY writes of `kind` whose bytes equal `expected` exactly.
///
/// Distinguishes "this command produced its own reply" from "a prior
/// command's reply is already in the transcript" — see §13.1 TPR
/// round 0 finding on multi-step tests satisfied by aggregated
/// transcripts.
pub fn count_exact_pty_writes(h: &SpecHarness, kind: PtyWriteKind, expected: &[u8]) -> usize {
    pty_writes_of_kind(h, kind)
        .filter(|bytes| *bytes == expected)
        .count()
}

/// Return the most recent `PtyEffect::Write` on the transcript as a
/// cloned `(bytes, kind)` pair, or `None` if the transcript is empty.
///
/// Canonical home for the "give me the last reply emitted" test pattern.
/// Callers that require the write to be present should pair with
/// `.expect("...")`.
pub fn last_pty_write(h: &SpecHarness) -> Option<(Vec<u8>, PtyWriteKind)> {
    pty_writes(h)
        .last()
        .map(|(bytes, kind)| (bytes.to_vec(), kind))
}

/// Assert exactly one `PtyEffect::Write` is on the transcript and return
/// its cloned bytes. Panics on zero or more than one.
///
/// Canonical home for the "single reply only" test pattern used by OSC
/// handlers whose contract is one reply per command. The kind is not
/// returned — callers that need kind discrimination should use
/// [`last_pty_write`] + assert the returned kind.
pub fn expect_single_pty_write(h: &SpecHarness) -> Vec<u8> {
    let mut iter = pty_writes(h);
    let (bytes, _) = iter
        .next()
        .expect("expected exactly one PtyEffect::Write; got none");
    let first = bytes.to_vec();
    assert!(
        iter.next().is_none(),
        "expected exactly one PtyEffect::Write; got multiple",
    );
    first
}

/// Assert the transcript contains zero `PtyEffect::Write` effects.
///
/// Canonical home for the "silent path" test pattern (OSC handlers that
/// mutate state without emitting a reply). Panics on any `PtyEffect::Write`.
pub fn assert_no_pty_writes(h: &SpecHarness) {
    assert_eq!(
        pty_writes(h).count(),
        0,
        "expected no PtyEffect::Write on transcript; got at least one",
    );
}

#[cfg(test)]
mod tests;
