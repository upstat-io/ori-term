use super::default_parser;
use super::tokens::{grid_find_field, grid_has_token, grid_line_starts_with};

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
