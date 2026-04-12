//! Sibling tests for the Section 06.4 ENQ/ACK parser.
//!
//! All tests call `parse_enq_ack_screen` directly so they run on
//! hosts without tack installed and in `cargo test --release`.

use super::parse_enq_ack_screen;

/// Build a synthetic post-`u` screen that mirrors tack v1.08's
/// success-path output: header, blank line, then the
/// `ACK terminating character: c` line.
fn synthetic_success_screen() -> String {
    "Testing ENQ/ACK, standby...\n\nACK terminating character: c\n".to_string()
}

#[test]
fn parse_enq_ack_screen_handles_empty_grid() {
    let facts = parse_enq_ack_screen("");
    assert_eq!(facts.header_text, "");
    assert!(facts.capability_labels.is_empty());
    assert!(facts.notes.is_empty());
}

#[test]
fn parse_enq_ack_screen_extracts_terminator_from_success_screen() {
    let grid = synthetic_success_screen();
    let facts = parse_enq_ack_screen(&grid);
    assert_eq!(facts.header_text, "Testing ENQ/ACK, standby...");
    assert_eq!(facts.notes, vec!["ack_terminator=c".to_string()]);
}

#[test]
fn parse_enq_ack_screen_extracts_first_non_blank_line_as_header() {
    // Header extraction parity with other Section 06 parsers.
    let grid = "\n\n   \nTesting ENQ/ACK, standby...\nACK terminating character: c\n";
    let facts = parse_enq_ack_screen(grid);
    assert_eq!(facts.header_text, "Testing ENQ/ACK, standby...");
}

#[test]
fn parse_enq_ack_screen_handles_screen_without_terminator_label() {
    // NEGATIVE PIN — a grid that has the header but NOT the
    // success-path terminator line returns no notes. Catches a
    // regression where the parser would default-emit a stub note
    // with an empty value.
    let grid = "Testing ENQ/ACK, standby...\nUnrelated text\n";
    let facts = parse_enq_ack_screen(grid);
    assert_eq!(facts.header_text, "Testing ENQ/ACK, standby...");
    assert!(facts.notes.is_empty());
}

#[test]
fn parse_enq_ack_screen_does_not_match_substring_inside_word() {
    // SEMANTIC PIN — `grid_find_field` is whitespace-bounded on
    // both sides, so a word containing `character:` as a substring
    // (e.g., `subcharacter:value`) must NOT match. This pins the
    // M3 token-boundary discipline at the parser layer.
    let grid = "Testing ENQ/ACK, standby...\nMisleadingsubcharacter: x\n";
    let facts = parse_enq_ack_screen(grid);
    assert!(
        facts.notes.is_empty(),
        "expected NO notes for substring-only match, got: {:?}",
        facts.notes,
    );
}

#[test]
fn parse_enq_ack_screen_records_terminator_with_punctuation() {
    // Some terminals report ACK terminators that are not bare
    // letters. The parser must record whatever single token follows
    // the `character:` label, including punctuation.
    let grid = "Testing ENQ/ACK, standby...\nACK terminating character: $\n";
    let facts = parse_enq_ack_screen(grid);
    assert_eq!(facts.notes, vec!["ack_terminator=$".to_string()]);
}
