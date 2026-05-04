//! Sibling tests for the graphic-rendition (SGR) parser.
//!
//! All tests call the pure `parse_graphic_rendition_screen`
//! helper directly so they run on hosts without tack installed
//! AND in `cargo test --release`.

use super::parse_graphic_rendition_screen;

#[test]
fn parse_graphic_rendition_screen_handles_empty_grid() {
    let facts = parse_graphic_rendition_screen("");
    assert_eq!(facts.header_text, "");
    assert!(facts.capability_labels.is_empty());
    assert!(facts.notes.is_empty());
}

#[test]
fn parse_graphic_rendition_screen_finds_each_sgr_label_in_isolation() {
    // Pin every SGR label individually so a regression that swaps
    // two labels in the parser surfaces as a clear per-label
    // failure.
    const LABELS: &[&str] = &["bold", "dim", "underline", "blink", "reverse", "invis"];
    for label in LABELS {
        let grid = format!("header line\n {label} sample text\n");
        let facts = parse_graphic_rendition_screen(&grid);
        assert_eq!(
            facts.capability_labels,
            vec![(*label).to_string()],
            "isolation pin failed for label {label:?}: got {:?}",
            facts.capability_labels
        );
    }
}

#[test]
fn parse_graphic_rendition_screen_finds_all_six_labels_at_once() {
    let grid = "bold dim underline blink reverse invis\n";
    let facts = parse_graphic_rendition_screen(grid);
    assert_eq!(
        facts.capability_labels,
        vec![
            "bold".to_string(),
            "dim".to_string(),
            "underline".to_string(),
            "blink".to_string(),
            "reverse".to_string(),
            "invis".to_string(),
        ]
    );
}

#[test]
fn parse_graphic_rendition_screen_rejects_substring_collisions() {
 // Verifies for the M3 fix: the parser MUST use
    // grid_has_token, not raw `str::contains`. Words like
    // `bolder`, `dimmer`, `blinking`, `underlined`, `reversed`,
    // `invisible` all CONTAIN one of the SGR labels but are not
    // labels themselves. A regression to plain contains() would
    // fire here.
    let grid = "\
embolden bolder dimmer
underlined reversed
invisible blinking
";
    let facts = parse_graphic_rendition_screen(grid);
    assert!(
        facts.capability_labels.is_empty(),
        "expected zero labels, got {:?} — parser is using raw contains() not grid_has_token",
        facts.capability_labels
    );
}

#[test]
fn parse_graphic_rendition_screen_handles_realistic_tack_v108_output() {
 // Verifies: against the actual tack v1.08 ACS test
    // output (verified empirically — see module rustdoc), the
    // parser returns no SGR labels because tack v1.08's ACS test
    // only probes (bel) and reports Done. The parser does NOT
    // panic, false-flag, or return spurious matches.
    let grid =
        "\nTesting bell (bel)\nIf you did not hear the Bell then (bel) has failed.  (bel) Done\n";
    let facts = parse_graphic_rendition_screen(grid);
    assert!(facts.capability_labels.is_empty());
}

#[test]
fn parse_graphic_rendition_screen_extracts_first_non_blank_line_as_header() {
    let grid = "\n\n   \nReal Header\n  body\n";
    let facts = parse_graphic_rendition_screen(grid);
    assert_eq!(facts.header_text, "Real Header");
}

#[test]
fn parse_graphic_rendition_screen_handles_label_at_start_of_line() {
    let grid = "header\nbold sample\n";
    let facts = parse_graphic_rendition_screen(grid);
    assert_eq!(facts.capability_labels, vec!["bold".to_string()]);
}

#[test]
fn parse_graphic_rendition_screen_handles_label_at_end_of_line() {
    let grid = "header\nthis line ends with bold\n";
    let facts = parse_graphic_rendition_screen(grid);
    assert_eq!(facts.capability_labels, vec!["bold".to_string()]);
}

#[test]
fn parse_graphic_rendition_screen_returns_labels_in_canonical_order() {
 // Verifies for the parser walks SGR_LABELS in
    // declaration order and pushes matches in that order, so the
    // returned `capability_labels` vec MUST appear in the canonical
    // [bold, dim, underline, blink, reverse, invis] order REGARDLESS
    // of the order they appear in the grid. A regression that
    // returned labels in grid-discovery order would fire here
    // because we deliberately scramble the input.
    let grid = "invis reverse blink underline dim bold\n";
    let facts = parse_graphic_rendition_screen(grid);
    assert_eq!(
        facts.capability_labels,
        vec![
            "bold".to_string(),
            "dim".to_string(),
            "underline".to_string(),
            "blink".to_string(),
            "reverse".to_string(),
            "invis".to_string(),
        ],
        "labels must be returned in canonical SGR_LABELS order, not grid order"
    );
}

#[test]
fn parse_graphic_rendition_screen_returns_partial_subset_in_canonical_order() {
 // Verifies for when only a subset of SGR labels
    // is present, the parser returns just those labels — still in
    // canonical order, with no padding/empty entries for missing
    // labels. Pin 3 of 6 (bold, underline, reverse) to catch a
    // regression that always returned all 6 with empty placeholders.
    let grid = "header\nbold and underline and reverse\n";
    let facts = parse_graphic_rendition_screen(grid);
    assert_eq!(
        facts.capability_labels,
        vec![
            "bold".to_string(),
            "underline".to_string(),
            "reverse".to_string(),
        ],
        "partial subset must contain only matched labels in canonical order"
    );
}
