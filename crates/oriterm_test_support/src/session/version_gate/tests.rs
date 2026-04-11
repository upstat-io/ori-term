//! Sibling tests for the tack version gate.
//!
//! Moved out of `session/tests.rs` in the M1 TPR cleanup
//! per `.claude/rules/test-organization.md` rule
//! "one sibling tests.rs per source file."
//!
//! All tests call the PURE helpers (`parse_tack_version`,
//! `unsupported_tack_diagnostic`, `check_tack_version_with_emit`,
//! `tack_runner_available_combine`) rather than spawning real
//! tack. The pure-helper split lets the matrix run on hosts
//! without tack installed AND in `cargo test --release`. The
//! actual `tack_version_supported()` public function only adds
//! the `tack -V` shell-out + the `eprintln!` emit closure —
//! both verified by the pure tests below.

use super::{
    TACK_PINNED_MAJOR, TACK_PINNED_MINOR, check_tack_version_with_emit, parse_tack_version,
    tack_runner_available_combine, unsupported_tack_diagnostic,
};

// ----- Family 1: parse_tack_version matrix -----

#[test]
fn parses_pinned_tack_1_08() {
    // Semantic pin: this is the EXACT version on the dev host. If
    // this test fails, the parser is broken at the only working
    // baseline.
    assert_eq!(
        parse_tack_version("tack version 1.08 (20170726)\n", ""),
        Some((1, 8))
    );
}

#[test]
fn parses_tack_with_leading_whitespace() {
    assert_eq!(
        parse_tack_version("   tack version 1.08\n", ""),
        Some((1, 8)),
        "defends against future tack builds that left-pad the banner"
    );
}

#[test]
fn parses_tack_with_only_stderr_output() {
    assert_eq!(
        parse_tack_version("", "tack version 1.08\n"),
        Some((1, 8)),
        "defends against `tack -V` future builds that move the banner to stderr"
    );
}

#[test]
fn parses_tack_with_two_digit_minor() {
    // Forces correct two-digit parsing — a leading-char-only parser
    // would return Some((1, 1)) here.
    assert_eq!(parse_tack_version("tack version 1.10\n", ""), Some((1, 10)));
}

#[test]
fn parses_tack_with_two_digit_minor_and_zero_pad() {
    // Tack 1.08 prints "1.08", not "1.8". `parse::<u32>` accepts the
    // leading zero and returns 8 — pin the behavior so a refactor that
    // adds an integer-radix-validation step doesn't accidentally reject
    // it.
    assert_eq!(parse_tack_version("tack version 1.08\n", ""), Some((1, 8)));
}

#[test]
fn parses_newer_tack_2_00() {
    assert_eq!(
        parse_tack_version("tack version 2.00 (20300101)\n", ""),
        Some((2, 0))
    );
}

#[test]
fn parses_older_tack_1_07() {
    assert_eq!(parse_tack_version("tack version 1.07\n", ""), Some((1, 7)));
}

#[test]
fn rejects_unparseable_garbage() {
    assert_eq!(parse_tack_version("hello world", ""), None);
}

#[test]
fn rejects_empty_string() {
    assert_eq!(parse_tack_version("", ""), None);
}

#[test]
fn rejects_no_version_prefix() {
    // Missing the literal `version ` token.
    assert_eq!(parse_tack_version("tack 1.08\n", ""), None);
}

#[test]
fn rejects_non_numeric_version() {
    assert_eq!(parse_tack_version("tack version foo.bar\n", ""), None);
}

#[test]
fn rejects_partial_version_missing_minor() {
    // "tack version 1\n" — split produces ["1", "", ...] — the second
    // fragment is empty and `parse::<u32>("")` returns Err.
    assert_eq!(parse_tack_version("tack version 1\n", ""), None);
}

// ----- Family 2: AND-combine pin -----

#[test]
fn tack_runner_available_combine_all_true() {
    assert!(tack_runner_available_combine(true, true, true));
}

#[test]
fn tack_runner_available_combine_version_false_short_circuits() {
    // SEMANTIC PIN for the AND-combine. If a future regression
    // flips the AND to an OR (e.g. `tack && tic || version`), this
    // assertion fails because `(true && true) || false` would be
    // true. With AND, version=false makes the whole expression
    // false even when tack and tic are both true.
    assert!(!tack_runner_available_combine(true, true, false));
}

#[test]
fn tack_runner_available_combine_tack_false_short_circuits() {
    assert!(!tack_runner_available_combine(false, true, true));
}

#[test]
fn tack_runner_available_combine_tic_false_short_circuits() {
    assert!(!tack_runner_available_combine(true, false, true));
}

#[test]
fn tack_runner_available_combine_all_false() {
    assert!(!tack_runner_available_combine(false, false, false));
}

// ----- Family 3: loud-skip + silent-on-match emit pins -----

#[test]
fn check_tack_version_emits_loud_skip_on_mismatch() {
    // SEMANTIC PIN for the loud-skip diagnostic. Without this test,
    // a refactor that removed the `eprintln!` invocation in
    // `tack_version_supported` would silently break the loud-skip
    // discipline — the function would still return false but
    // operators would lose the actionable upgrade-path message.
    let mut captured: Option<String> = None;
    let result = check_tack_version_with_emit("tack version 9.99 (20300101)\n", "", &mut |msg| {
        captured = Some(msg)
    });
    assert!(!result, "9.99 must be unsupported");
    let msg = captured.expect("emit closure must be invoked on mismatch");
    assert!(
        msg.contains("INSTA_UPDATE=1"),
        "diagnostic must include the snapshot capture command, got: {msg}"
    );
    assert!(
        msg.contains("BEGIN_TESTING_INVENTORY"),
        "diagnostic must reference the inventory const, got: {msg}"
    );
    assert!(
        msg.contains("TACK_PINNED_MAJOR"),
        "diagnostic must reference the pinned-version const, got: {msg}"
    );
    assert!(
        msg.contains("9.99"),
        "diagnostic must name the observed version, got: {msg}"
    );
}

#[test]
fn check_tack_version_silent_on_pinned_match() {
    // SEMANTIC PIN: when the version DOES match the pinned values,
    // NO eprintln fires. Otherwise the loud-skip becomes noise on
    // every test run and operators learn to ignore it. The pure
    // helper makes "did the closure fire" observable without a
    // stderr capture.
    let pinned = format!(
        "tack version {pmaj}.{pmin:02} (20170726)\n",
        pmaj = TACK_PINNED_MAJOR,
        pmin = TACK_PINNED_MINOR,
    );
    let mut captured: Option<String> = None;
    let result = check_tack_version_with_emit(&pinned, "", &mut |msg| captured = Some(msg));
    assert!(result, "pinned version must be supported");
    assert!(
        captured.is_none(),
        "emit closure must NOT fire on a supported version, got: {captured:?}"
    );
}

#[test]
fn check_tack_version_returns_false_when_parse_fails() {
    // Parse failure must yield false WITHOUT calling the emit
    // closure — there's nothing actionable to say about
    // unparseable output beyond the false return.
    let mut captured: Option<String> = None;
    let result = check_tack_version_with_emit("garbage", "", &mut |msg| {
        captured = Some(msg);
    });
    assert!(!result);
    assert!(captured.is_none());
}

// ----- Family 4: unsupported_tack_diagnostic content -----

#[test]
fn unsupported_tack_diagnostic_includes_pinned_version() {
    let msg = unsupported_tack_diagnostic(2, 0);
    assert!(
        msg.contains(&format!(
            "{pmaj}.{pmin:02}",
            pmaj = TACK_PINNED_MAJOR,
            pmin = TACK_PINNED_MINOR
        )),
        "diagnostic must name the pinned (target) version, got: {msg}"
    );
}

#[test]
fn unsupported_tack_diagnostic_zero_pads_observed_minor() {
    // Observed version is printed as `<maj>.<min:02>` so 1.7 reads
    // "tack 1.07 installed" rather than "tack 1.7 installed". This
    // matches tack's own banner format and avoids visual confusion.
    let msg = unsupported_tack_diagnostic(1, 7);
    assert!(
        msg.contains("tack 1.07 installed"),
        "diagnostic must zero-pad observed minor, got: {msg}"
    );
}
