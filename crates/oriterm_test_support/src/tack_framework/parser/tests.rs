//! Sibling tests for `parser/mod.rs`.
//!
//! In 05.0.b's broken-window cleanup the `grid_has_token` /
//! `grid_has_paren_token` / `grid_line_starts_with` / `grid_find_field`
//! tests were moved out of this file into `parser/tokens/tests.rs`
//! per the test-organization rule "one tests.rs per source file".
//! Only the `default_parser` tests remain here.

use super::default_parser;

#[test]
fn default_parser_extracts_first_non_blank_line_as_header() {
    let grid = "\n\nMain Menu\n b) basic\n";
    let facts = default_parser(grid);
    assert_eq!(facts.header_text, "Main Menu");
    assert!(facts.capability_labels.is_empty());
    assert!(facts.notes.is_empty());
}

#[test]
fn default_parser_handles_empty_grid() {
    let facts = default_parser("");
    assert_eq!(facts.header_text, "");
}

#[test]
fn default_parser_handles_all_blank_grid() {
    let facts = default_parser("\n\n   \n  \n");
    assert_eq!(facts.header_text, "");
}
