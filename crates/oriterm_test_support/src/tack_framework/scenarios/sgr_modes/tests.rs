//! Sibling tests for the Section 06.2 SGR modes parser.
//!
//! All tests call the pure `parse_sgr_modes_screen` helper (or its
//! extracted counter) directly so they run on hosts without tack
//! installed and in `cargo test --release`.

use super::{MIN_EXPECTED_MODES, count_mode_labels, parse_sgr_modes_screen};

/// Build a synthetic grid that mirrors tack v1.08's Mode 0..79
/// layout: 8 rows of 10 labels each, `Mode %2d` formatting, one
/// space between labels on a row, trailing newline per row.
fn synthetic_full_grid() -> String {
    let mut out = String::new();
    for row in 0..8_u32 {
        let base = row * 10;
        for col in 0..10_u32 {
            if col > 0 {
                out.push(' ');
            }
            let n = base + col;
            out.push_str(&format!("Mode {n:>2}"));
        }
        out.push('\n');
    }
    out
}

#[test]
fn parse_sgr_modes_screen_handles_empty_grid() {
    let facts = parse_sgr_modes_screen("");
    assert_eq!(facts.header_text, "");
    assert!(facts.capability_labels.is_empty());
    assert_eq!(facts.notes, vec!["found_modes_count=0".to_string()]);
}

#[test]
fn parse_sgr_modes_screen_finds_full_80_mode_grid() {
    // SEMANTIC PIN for the 06.2 contract: a canonically-formatted
    // tack SGR grid returns exactly the full 80-mode count. Pins
    // the `Mode %2d` token format against accidental parser drift.
    let grid = synthetic_full_grid();
    let facts = parse_sgr_modes_screen(&grid);
    assert_eq!(
        facts.notes,
        vec![format!("found_modes_count={MIN_EXPECTED_MODES}")],
        "expected the full {MIN_EXPECTED_MODES}-mode grid; grid:\n{grid}",
    );
}

#[test]
fn parse_sgr_modes_screen_counts_each_label_individually() {
    // Per-label isolation pin: feed ONE label at a time and verify
    // each of the 80 expected labels matches. A regression that
    // broke padding (e.g., dropping the second space for single-
    // digit modes) would surface as a per-label failure with a
    // clear diagnostic naming the specific N that stopped matching.
    for n in 0..80_u32 {
        let token = format!("Mode {n:>2}");
        let grid = format!("header line\n{token}\n");
        assert_eq!(
            count_mode_labels(&grid),
            1,
            "isolation pin failed for n={n}, token={token:?}: grid:\n{grid}",
        );
    }
}

#[test]
fn parse_sgr_modes_screen_handles_only_low_modes() {
    // Partial grid — only the first row (Mode 0..9) — returns
    // count=10. Pins that the parser does not synthesize missing
    // high-mode matches when only the low range is present.
    let mut grid = String::from("header\n");
    for n in 0..10_u32 {
        if n > 0 {
            grid.push(' ');
        }
        grid.push_str(&format!("Mode {n:>2}"));
    }
    grid.push('\n');
    assert_eq!(count_mode_labels(&grid), 10);
}

#[test]
fn parse_sgr_modes_screen_handles_only_high_modes() {
    // Mirror of the low-modes pin: the last row (Mode 70..79)
    // returns count=10. Pins that the parser's single-digit
    // padding (`Mode  0`) does not accidentally match high-mode
    // tokens.
    let mut grid = String::from("header\n");
    for n in 70..80_u32 {
        if n > 70 {
            grid.push(' ');
        }
        grid.push_str(&format!("Mode {n:>2}"));
    }
    grid.push('\n');
    assert_eq!(count_mode_labels(&grid), 10);
}

#[test]
fn parse_sgr_modes_screen_handles_header_only_grid() {
    // A header without any `Mode N` labels returns count=0 AND
    // surfaces the header in `header_text`.
    let grid = "Tack SGR modes test header\n\n";
    let facts = parse_sgr_modes_screen(grid);
    assert_eq!(facts.header_text, "Tack SGR modes test header");
    assert_eq!(facts.notes, vec!["found_modes_count=0".to_string()]);
}

#[test]
fn parse_sgr_modes_screen_does_not_confuse_mode_7_with_mode_70() {
    // SEMANTIC PIN: the parser must NOT match `Mode  7` inside
    // `Mode 70`. `grid_has_token("Mode  7")` checks that the
    // byte AFTER the matched token is ASCII whitespace. In
    // `Mode 70` the byte after the single-space + `7` is `0`,
    // which is NOT whitespace — so the token is correctly
    // rejected. This pin is the M3 token-boundary discipline
    // from Section 04 applied at the numeric-prefix level.
    let grid = "Mode 70 Mode 71\n";
    // count_mode_labels should find exactly 70 and 71 — NOT
    // 7 and 71.
    assert_eq!(count_mode_labels(grid), 2);
    // Double-check by confirming the bare `Mode  7` token is
    // NOT a substring even though `Mode  7` would be a legal
    // search pattern against `Mode 70`. The token format's
    // width-aware padding defeats it: `Mode  7` (two spaces)
    // never appears inside `Mode 70` (one space).
    assert!(!grid.contains("Mode  7"));
}

#[test]
fn parse_sgr_modes_screen_rejects_substring_collisions_in_english_text() {
    // SEMANTIC PIN: the parser uses `grid_has_token` which is
    // whitespace-bounded. A regression that swapped to raw
    // `str::contains` would false-positive on English text
    // containing "Mode 1" as a prefix of "Mode 13 is wrong" or
    // similar. Pin that a non-tack grid with English prose
    // mentioning "Mode" returns count=0.
    let grid = "\
Mode selector missing from the UI.
See Modes 0..79 in the spec.
No tack output was captured here.
";
    // Nothing in this grid matches `Mode  0`..`Mode 79` with the
    // exact `Mode %2d` padding tack emits, so count must be 0.
    // If a regression added a `grid.contains("Mode")` fast-path
    // outside the padded-token path it would false-positive here.
    assert_eq!(count_mode_labels(grid), 0);
}

#[test]
fn parse_sgr_modes_screen_extracts_first_non_blank_line_as_header() {
    let grid = "\n\n   \nSGR modes header\nMode  0\n";
    let facts = parse_sgr_modes_screen(grid);
    assert_eq!(facts.header_text, "SGR modes header");
}

#[test]
fn parse_sgr_modes_screen_handles_trailing_whitespace_padding() {
    // Pin that trailing whitespace padding (as emitted by
    // oriterm_core's `grid_text()` when the terminal pads each
    // row to full column width) does not break the token boundary
    // check on the rightmost label on a row.
    let grid =
        "Mode  0 Mode  1 Mode  2 Mode  3 Mode  4 Mode  5 Mode  6 Mode  7 Mode  8 Mode  9   \n";
    assert_eq!(count_mode_labels(grid), 10);
}

#[test]
fn parse_sgr_modes_screen_handles_minimum_expected_modes_constant() {
    // SEMANTIC PIN: `MIN_EXPECTED_MODES` is the canonical
    // threshold both the parser sibling tests and the 80x24 test
    // wrapper reference. Pin the value to 80 so a well-meaning
    // refactor that lowered the threshold to match a broken
    // capture also flips this test red as a referee.
    assert_eq!(MIN_EXPECTED_MODES, 80);
}
