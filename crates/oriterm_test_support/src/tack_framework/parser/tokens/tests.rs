//! Sibling tests for the tokenized grid match helpers.
//!
//! Moved out of `parser/tests.rs` in 05.0.b's broken-window cleanup
//! per the test-organization rule "one tests.rs per source file".
//! `parser/tests.rs` retains only the `default_parser` tests; all
//! `grid_has_token` / `grid_has_paren_token` / `grid_line_starts_with` /
//! `grid_find_field` tests live here.

use super::{grid_find_field, grid_has_paren_token, grid_has_token, grid_line_starts_with};

#[test]
fn grid_has_token_finds_whitespace_bounded_match() {
    assert!(grid_has_token("am bce bw", "am"));
    assert!(grid_has_token("am bce bw", "bce"));
    assert!(grid_has_token("am bce bw", "bw"));
}

#[test]
fn grid_has_token_rejects_substring_collision() {
    // Semantic pin for the M3 fix: `am` is a substring of `name` and
    // `xenl` is a substring of `xenlabel`. Blind `str::contains` would
    // false-positive both. The whitespace-bounded check rejects them.
    assert!(!grid_has_token("name bce bw", "am"));
    assert!(!grid_has_token("xenlabel", "xenl"));
}

#[test]
fn grid_has_token_handles_line_edges_as_boundaries() {
    assert!(grid_has_token("am\nbce", "am"));
    assert!(grid_has_token("am\nbce", "bce"));
}

#[test]
fn grid_has_token_rejects_empty_token() {
    assert!(!grid_has_token("anything", ""));
}

#[test]
fn grid_line_starts_with_finds_prompt_marker() {
    let grid = "header\n  tack [m] > waiting\n";
    assert!(grid_line_starts_with(grid, "tack [m]"));
    assert!(!grid_line_starts_with(grid, "tack [n]"));
}

#[test]
fn grid_find_field_returns_trailing_value() {
    let grid = "header\nsetaf \\E[3%dm\nsetab \\E[4%dm\n";
    assert_eq!(grid_find_field(grid, "setaf"), Some("\\E[3%dm"));
    assert_eq!(grid_find_field(grid, "setab"), Some("\\E[4%dm"));
    assert_eq!(grid_find_field(grid, "missing"), None);
}

#[test]
fn grid_find_field_rejects_substring_collision_on_right_boundary() {
    // SEMANTIC PIN for the TPR-04-001 right-boundary fix. The
    // earlier draft only checked the LEFT boundary, so this would
    // false-positive and return "oreground". With both boundaries
    // enforced, the helper correctly returns None.
    let grid = "setaforeground \\E[3%dm\n";
    assert_eq!(grid_find_field(grid, "setaf"), None);
}

#[test]
fn grid_find_field_finds_real_token_after_substring_collision_on_same_line() {
    // SEMANTIC PIN for the TPR-04-001 all-hits-per-line fix. The
    // earlier draft only inspected the FIRST find() hit per line,
    // so the leading substring `xsetaf` (which fails the left
    // boundary) hid the real `setaf` token later on the same line.
    // With the all-hits scan, the helper finds the second
    // occurrence and returns its value.
    let grid = "xsetaf setaf \\E[3%dm\n";
    assert_eq!(grid_find_field(grid, "setaf"), Some("\\E[3%dm"));
}

#[test]
fn grid_find_field_rejects_empty_label() {
    assert_eq!(grid_find_field("anything", ""), None);
}

#[test]
fn grid_has_paren_token_finds_parenthesized_cap_label() {
    // Tack tags every modes-test result with `(cap_name)`.
    let grid = "(am) auto-margins is true\n(os) over-strike is false\n";
    assert!(grid_has_paren_token(grid, "am"));
    assert!(grid_has_paren_token(grid, "os"));
}

#[test]
fn grid_has_paren_token_rejects_bare_cap_without_parens() {
    // The whole point of the helper is that bare `am` (matched by
    // raw `contains`) collides with `name`, while `(am)` does not.
    let grid = "name is am\nover-strike is os\n";
    assert!(!grid_has_paren_token(grid, "am"));
    assert!(!grid_has_paren_token(grid, "os"));
}

#[test]
fn grid_has_paren_token_rejects_empty_cap() {
    assert!(!grid_has_paren_token("anything", ""));
}
