//! Direct-VTE cap cross-check tests for terminfo caps that
//! `tack` v1.08 cannot reach.
//!
//! Section 06.5 lands one or more `#[test] fn`s per cap declared in
//! `extra/ori_term.info` that has no tack tool to probe it
//! (Smulx, Setulc, Sync, BD, BE, PS, PE, Se, Ss, XF, kxIN, kxOUT,
//! Tc, RGB, Cr, Cs, Ms, hs, dsl, fsl, tsl, AX, XT — 23 entries).
//! Each test feeds the cap's declared escape sequence directly
//! into a `Term<RecordingListener>` and asserts the correct event
//! fires or the correct mode/state transition occurs.
//!
//! Two caps are GENUINELY cross-crate (kxIN, kxOUT — focus event
//! markers emitted by `oriterm/src/app/event_loop_helpers/mod.rs`
//! in response to winit `WindowEvent::Focused`). Their real tests
//! live in `oriterm/src/app/event_loop_helpers/tests.rs`; this
//! module carries documentation-only stub fns in
//! `focus_events.rs` so the registry sweep does not falsely flag
//! them as missing.
//!
//! # Registry / SSOT
//!
//! [`NON_TACK_CAP_XCHECK_CAPS`] is the canonical list of caps
//! Section 06.5 owns. Each per-cap submodule contributes a
//! `#[test] fn` AND a corresponding entry in
//! [`XCHECK_REGISTERED_CAPS`] (a parallel sorted slice of cap
//! names declared next to the per-cap test). The
//! [`tack_cap_xcheck_covers_every_non_tack_cap`] meta-test asserts
//! both lists contain the SAME set; if a cap is added to
//! `NON_TACK_CAP_XCHECK_CAPS` without a backing test in any
//! submodule (or vice versa), the meta-test fires with a set-diff
//! diagnostic naming the missing entry.
//!
//! The dual-list pattern is necessary because Rust's test harness
//! does not expose a runtime list of `#[test] fn`s — there is no
//! way to introspect "every test function with prefix
//! `tack_cap_xcheck_`" at runtime. The parallel const list is the
//! SSOT-bridge between "what caps Section 06 owns" and "what tests
//! actually exist". Fix interference: if you ever rename a cap or
//! delete a test, update BOTH lists in the same commit so the
//! meta-test stays green.

mod bracketed_paste;
mod cursor_style;
mod focus_events;
mod osc_clipboard;
mod osc_color;
mod sgr_extensions;
mod status_line;
mod sync;
mod truecolor;
mod xterm_markers;

#[cfg(test)]
mod tests;

/// The canonical list of terminfo caps that Section 06.5 covers
/// via direct-VTE round-trip tests.
///
/// 23 entries split across 10 cap families. The order is
/// human-readable (groups together by family) — the meta-test
/// sorts both lists into `BTreeSet`s before comparing so order is
/// load-bearing for readability only.
pub(super) const NON_TACK_CAP_XCHECK_CAPS: &[&str] = &[
    // SGR extensions (kitty colon underline + truecolor underline color).
    "Smulx", "Setulc", // Synchronized output mode 2026.
    "Sync",   // Bracketed paste (mode toggle + outbound markers).
    "BD", "BE", "PS", "PE", // DECSCUSR cursor style.
    "Se", "Ss", // Focus event markers (XF bool + kxIN/kxOUT outbound bytes).
    "XF", "kxIN", "kxOUT", // Truecolor advertisement.
    "Tc", "RGB", // OSC cursor color (12 set, 112 reset) + OSC 52 clipboard.
    "Cr", "Cs", "Ms", // Status line (hs bool + tsl/fsl/dsl OSC-2 title-backed).
    "hs", "dsl", "fsl", "tsl", // xterm extension markers (bool advertisements).
    "AX", "XT",
];

/// The aggregated list of caps covered by per-submodule test
/// fns. Each submodule contributes its caps via a `pub(super)
/// const` slice; this top-level slice concatenates them so the
/// meta-test has a single set to compare against
/// [`NON_TACK_CAP_XCHECK_CAPS`].
///
/// When a new cap test is added to a submodule, the implementer
/// MUST also add the cap name to that submodule's `REGISTERED`
/// const so the meta-test set-diff stays clean.
pub(super) const XCHECK_REGISTERED_CAPS: &[&[&str]] = &[
    sgr_extensions::REGISTERED,
    sync::REGISTERED,
    bracketed_paste::REGISTERED,
    cursor_style::REGISTERED,
    focus_events::REGISTERED,
    truecolor::REGISTERED,
    osc_color::REGISTERED,
    osc_clipboard::REGISTERED,
    status_line::REGISTERED,
    xterm_markers::REGISTERED,
];

/// Assert that `cap` is declared in `extra/ori_term.info`.
///
/// Used by per-cap tests as the first assertion BEFORE feeding
/// the literal escape sequence — if the cap declaration drifts
/// out of the terminfo source, the test fires with a
/// "cap declaration missing" diagnostic instead of a confusing
/// state assertion failure.
///
/// Delegates to the canonical [`parse_declared_caps`] helper in
/// `oriterm_test_support::tack_framework::cap_coverage` (the SSOT
/// for tic-format terminfo parsing). The earlier draft of this
/// module reimplemented the parser locally with a comment claiming
/// the canonical helper "cannot reach" the in-crate tests — that
/// claim was wrong: `oriterm_test_support` is a dev-dependency of
/// `oriterm_core` (see `oriterm_core/Cargo.toml`), so the
/// canonical helper IS reachable from this `#[cfg(test)]` module.
/// TPR-06-003 fixed the duplication by removing the local
/// reimplementation and routing through the canonical helper.
///
/// [`parse_declared_caps`]: oriterm_test_support::tack_framework::cap_coverage::parse_declared_caps
pub(super) fn assert_cap_declared(cap: &str) {
    let declared = oriterm_test_support::tack_framework::cap_coverage::parse_declared_caps();
    assert!(
        declared.contains(cap),
        "tack_cap_xcheck precondition failed: cap {cap:?} is not \
         declared in extra/ori_term.info. The Section 06.5 test \
         for {cap:?} is asserting against a cap that no longer \
         exists; either restore the declaration in the terminfo \
         source or remove the test entry from \
         NON_TACK_CAP_XCHECK_CAPS + the per-submodule REGISTERED \
         list.",
    );
}

/// Assert that the named cap is declared in `extra/ori_term.info`
/// with EXACTLY the expected string value.
///
/// The expected value is in tic source form (i.e. `\E` for escape,
/// not the literal byte `\x1b`) so the test strings stay readable.
///
/// Tests for caps that emit a specific escape sequence call this
/// helper to pin the terminfo→VTE byte mapping. A future edit to
/// `extra/ori_term.info` that changes the cap's value flips the
/// test red BEFORE the actual VTE round-trip is exercised.
///
/// Delegates to the canonical [`declared_cap_value`] helper in
/// `oriterm_test_support::tack_framework::cap_coverage` — see
/// [`assert_cap_declared`] for the SSOT rationale (TPR-06-003).
///
/// [`declared_cap_value`]: oriterm_test_support::tack_framework::cap_coverage::declared_cap_value
pub(super) fn assert_cap_value_matches(cap: &str, expected: &str) {
    let actual = oriterm_test_support::tack_framework::cap_coverage::declared_cap_value(cap);
    assert_eq!(
        actual.as_deref(),
        Some(expected),
        "tack_cap_xcheck cap value mismatch for {cap:?}.\n\
         Expected: {expected:?}\n\
         Actual:   {actual:?}\n\
         Either fix the test to match the new value or restore \
         the declaration in extra/ori_term.info."
    );
}
