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
    // SEMANTIC PIN for the M3 fix: the parser MUST use
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
    // SEMANTIC PIN: against the actual tack v1.08 ACS test
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
