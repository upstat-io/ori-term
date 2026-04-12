//! Sibling tests for the color screen parser.
//!
//! All tests call the pure `parse_color_screen` helper directly so
//! they run on hosts without tack installed AND in
//! `cargo test --release`.

use super::parse_color_screen;

#[test]
fn parse_color_screen_handles_empty_grid() {
    let facts = parse_color_screen("");
    assert_eq!(facts.header_text, "");
    assert!(facts.capability_labels.is_empty());
    assert!(facts.notes.is_empty());
}

#[test]
fn parse_color_screen_finds_all_eight_named_colors() {
    // Pin every ANSI 16-color name at once. Whitespace-separated
    // tokens — the canonical layout the parser expects.
    let grid = "header line\nblack red green yellow blue magenta cyan white\n";
    let facts = parse_color_screen(grid);
    assert_eq!(
        facts.capability_labels,
        vec![
            "black".to_string(),
            "red".to_string(),
            "green".to_string(),
            "yellow".to_string(),
            "blue".to_string(),
            "magenta".to_string(),
            "cyan".to_string(),
            "white".to_string(),
        ]
    );
}

#[test]
fn parse_color_screen_finds_each_color_in_isolation() {
    // Pin every color individually so a regression that swaps two
    // colors in the parser surfaces as a clear per-color failure.
    const COLORS: &[&str] = &[
        "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white",
    ];
    for color in COLORS {
        let grid = format!("header line\n {color} sample text\n");
        let facts = parse_color_screen(&grid);
        assert_eq!(
            facts.capability_labels,
            vec![(*color).to_string()],
            "isolation pin failed for color {color:?}: got {:?}",
            facts.capability_labels
        );
    }
}

#[test]
fn parse_color_screen_rejects_substring_collisions() {
    // SEMANTIC PIN for the M3 fix: the parser MUST use
    // `grid_has_token`, not raw `str::contains`. The plan
    // explicitly calls out: `red` matches inside
    // `redirect`/`rendered`/`reduce`, `blue` inside
    // `bluefoot`/`bluebird`, `yellow` inside
    // `yellowish`/`yellowstone`, etc. A regression that switched
    // to `str::contains` would silently make every color test
    // pass-with-false-positives — invisible without this pin.
    let grid = "\
redirect rendered yellowish bluefoot
greener blacksmith magentastyle
cyanide whitewash redneck
";
    let facts = parse_color_screen(grid);
    assert!(
        facts.capability_labels.is_empty(),
        "expected zero colors, got {:?} — parser is using raw contains() not grid_has_token",
        facts.capability_labels
    );
}

#[test]
fn parse_color_screen_handles_partial_palette() {
    // Pin that a partial subset (3 of 8) returns ONLY those colors
    // in canonical order, not all 8 with empty entries for missing
    // ones.
    let grid = "header\nred green blue\n";
    let facts = parse_color_screen(grid);
    assert_eq!(
        facts.capability_labels,
        vec!["red".to_string(), "green".to_string(), "blue".to_string()]
    );
}

#[test]
fn parse_color_screen_returns_colors_in_canonical_order() {
    // SEMANTIC PIN: the parser walks NAMED_COLORS in declaration
    // order and pushes matches in that order, so the returned
    // `capability_labels` vec MUST appear in canonical
    // [black, red, green, yellow, blue, magenta, cyan, white] order
    // REGARDLESS of grid order. Pin by deliberately scrambling
    // the input.
    let grid = "white cyan magenta blue yellow green red black\n";
    let facts = parse_color_screen(grid);
    assert_eq!(
        facts.capability_labels,
        vec![
            "black".to_string(),
            "red".to_string(),
            "green".to_string(),
            "yellow".to_string(),
            "blue".to_string(),
            "magenta".to_string(),
            "cyan".to_string(),
            "white".to_string(),
        ],
        "labels must be returned in canonical NAMED_COLORS order, not grid order"
    );
}

#[test]
fn parse_color_screen_handles_realistic_tack_v108_output() {
    // SEMANTIC PIN: against the actual tack v1.08 color test
    // output (verified empirically — see module rustdoc), the
    // parser returns no named colors because tack v1.08's color
    // test only probes (colors) and (pairs) cap names — it does
    // NOT emit any of the 8 ANSI named colors. The parser does
    // NOT panic, false-flag, or return spurious matches.
    let grid = "\nThis terminal can display 256 colors and 32767 color pairs.  (colors) (pairs)\n(colors) (pairs) Done\n";
    let facts = parse_color_screen(grid);
    assert!(facts.capability_labels.is_empty());
}

#[test]
fn parse_color_screen_extracts_first_non_blank_line_as_header() {
    let grid = "\n\n   \nReal Header Line\n  body\n";
    let facts = parse_color_screen(grid);
    assert_eq!(facts.header_text, "Real Header Line");
}

#[test]
fn parse_color_screen_handles_color_at_start_of_line() {
    let grid = "header\nred sample\n";
    let facts = parse_color_screen(grid);
    assert_eq!(facts.capability_labels, vec!["red".to_string()]);
}

#[test]
fn parse_color_screen_handles_color_at_end_of_line() {
    let grid = "header\nthis line ends with red\n";
    let facts = parse_color_screen(grid);
    assert_eq!(facts.capability_labels, vec!["red".to_string()]);
}
