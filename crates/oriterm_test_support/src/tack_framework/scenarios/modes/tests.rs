//! Sibling tests for the modes scenario module.
//!
//! Pins the `parse_modes_phase_screen` per-cap parser introduced in
//! 05.1. Tests run in BOTH debug and release per the Section 04
//! parity rule. None of the tests spawn tack — they feed synthetic
//! grids into the pure parser.
//!
//! The pre-existing `parse_modes_screen` (the always-visible
//! final-cap parser from Section 04) is exercised end-to-end by
//! `oriterm_core/tests/tack/test_menu/modes.rs::tack_modes_am`,
//! which already runs against real tack — no parallel pure test
//! is needed because the e2e test pins the same behavior.

use super::parse_modes_phase_screen;

#[test]
fn parse_modes_phase_screen_finds_all_known_caps() {
    let grid = "\
(am) auto-margins is true
(bce) back color erase is supported
(bw) auto-left-margin works
(km) has meta key
(mir) move-in-insert mode works
(msgr) safe to move in standout
(xenl) eat newline glitch
(os) over-strike is false
";
    let facts = parse_modes_phase_screen(grid);
    let labels: Vec<&str> = facts.capability_labels.iter().map(String::as_str).collect();
    assert_eq!(
        labels,
        vec!["am", "bce", "bw", "km", "mir", "msgr", "xenl", "os"]
    );
    assert_eq!(facts.header_text, "(am) auto-margins is true");
    assert!(facts.notes.is_empty());
}

#[test]
fn parse_modes_phase_screen_handles_missing_caps() {
    // Only one cap on the screen — the parser must return exactly
    // one label, not the full known set.
    let grid = "modes test\n(am) only this one is present\n";
    let facts = parse_modes_phase_screen(grid);
    assert_eq!(
        facts.capability_labels,
        vec!["am".to_string()],
        "expected only [am], got {:?}",
        facts.capability_labels
    );
}

#[test]
fn parse_modes_phase_screen_rejects_substring_collisions() {
 // Verifies: the parser MUST use grid_has_paren_token, not
    // raw `str::contains`. Words like `name` contain `am`, `xname`
    // contains `xnam`, and `bcename` contains both `bce` and `name`
    // — none of which are real cap labels (they lack the
    // `(<cap>)` wrapping). A regression that switched to plain
    // contains() would fire here.
    let grid = "\
modes test header
name xenlabel xname
bcename msgrname mirfoo
";
    let facts = parse_modes_phase_screen(grid);
    assert!(
        facts.capability_labels.is_empty(),
        "expected zero labels, got {:?} — parser is using raw contains() not grid_has_paren_token",
        facts.capability_labels
    );
}

#[test]
fn parse_modes_phase_screen_each_known_cap_in_isolation() {
    // For each known cap, feed a grid with ONLY that one
    // parenthesized cap and assert the parser returns exactly one
    // label and it is the right one. Catches a regression where the
    // parser silently swaps two cap names (e.g., `(bce)` returns
    // `am`).
    const KNOWN: &[&str] = &["am", "bce", "bw", "km", "mir", "msgr", "xenl", "os"];
    for cap in KNOWN {
        let grid = format!("modes test\n({cap}) the only cap on this grid\n");
        let facts = parse_modes_phase_screen(&grid);
        assert_eq!(
            facts.capability_labels,
            vec![(*cap).to_string()],
            "isolation pin failed for cap {cap:?}: got {:?}",
            facts.capability_labels
        );
    }
}

#[test]
fn parse_modes_phase_screen_uses_grid_has_paren_token() {
 // Verifies: feed bare cap labels (no parens) and assert
    // NONE of them are returned. The whole point of the helper is
    // that `am` matches inside `name` via plain contains() but
    // `(am)` does not. If a future regression switches the parser
    // to plain str::contains, this test fires because `am bce`
    // contains the substring `am`, `bce`, etc.
    let grid = "am bce bw km mir msgr xenl os name\n";
    let facts = parse_modes_phase_screen(grid);
    assert!(
        facts.capability_labels.is_empty(),
        "bare cap labels (without parens) MUST not match: got {:?}",
        facts.capability_labels
    );
}

#[test]
fn parse_modes_phase_screen_handles_empty_grid() {
    let facts = parse_modes_phase_screen("");
    assert!(facts.capability_labels.is_empty());
    assert_eq!(facts.header_text, "");
    assert!(facts.notes.is_empty());
}

#[test]
fn parse_modes_phase_screen_handles_all_blank_grid() {
    let facts = parse_modes_phase_screen("\n\n   \n  \n");
    assert!(facts.capability_labels.is_empty());
    assert_eq!(facts.header_text, "");
}

#[test]
fn parse_modes_phase_screen_finds_caps_on_arbitrary_lines() {
    // Caps can appear anywhere in the grid — the parser must scan
    // the whole text, not just the first line.
    let grid = "header\nfiller\nfiller\n(km) has meta key\nmore filler\n(xenl) eat newline\n";
    let facts = parse_modes_phase_screen(grid);
    assert_eq!(
        facts.capability_labels,
        vec!["km".to_string(), "xenl".to_string()]
    );
    assert_eq!(facts.header_text, "header");
}
