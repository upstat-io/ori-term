//! Sibling tests for the ACS / graphic-character-set parser.
//!
//! All tests call the pure `parse_acs_screen` helper directly so
//! they run on hosts without tack installed AND in
//! `cargo test --release`.

use super::parse_acs_screen;

#[test]
fn parse_acs_screen_handles_empty_grid() {
    let facts = parse_acs_screen("");
    assert_eq!(facts.header_text, "");
    assert!(facts.capability_labels.is_empty());
    assert_eq!(
        facts.notes,
        vec!["distinct_line_drawing_chars=0".to_string()]
    );
}

#[test]
fn parse_acs_screen_counts_line_drawing_chars_when_present() {
    // Box-drawing characters: U+2500 ─, U+2502 │, U+250C ┌,
    // U+2510 ┐, U+2514 └, U+2518 ┘. Six distinct codepoints.
    let grid = "header\n┌─────┐\n│ box │\n└─────┘\n";
    let facts = parse_acs_screen(grid);
    assert_eq!(
        facts.notes,
        vec!["distinct_line_drawing_chars=6".to_string()]
    );
    assert_eq!(facts.header_text, "header");
}

#[test]
fn parse_acs_screen_dedupes_repeated_chars() {
    // The grid contains 8 instances of `─` and 2 of `│` — but
    // only 2 DISTINCT codepoints. The set-based count must
    // dedupe.
    let grid = "────────\n│      │\n────────\n";
    let facts = parse_acs_screen(grid);
    assert_eq!(
        facts.notes,
        vec!["distinct_line_drawing_chars=2".to_string()]
    );
}

#[test]
fn parse_acs_screen_ignores_non_line_drawing_chars() {
    // ASCII text + emoji + Latin extended — all OUTSIDE the
    // U+2500..=U+257F range. Count must be 0.
    let grid = "The quick brown fox 🦊 jumps over the lazy dog. ñ ü β\n";
    let facts = parse_acs_screen(grid);
    assert_eq!(
        facts.notes,
        vec!["distinct_line_drawing_chars=0".to_string()]
    );
}

#[test]
fn parse_acs_screen_counts_chars_at_block_boundaries() {
    // U+2500 (the first char in the block) and U+257F (the last
    // char in the block) — pin both inclusive boundaries.
    let grid = "\u{2500}\u{257F}";
    let facts = parse_acs_screen(grid);
    assert_eq!(
        facts.notes,
        vec!["distinct_line_drawing_chars=2".to_string()]
    );
}

#[test]
fn parse_acs_screen_excludes_chars_just_outside_block() {
    // U+24FF is just below the block; U+2580 is just above.
    // Neither should count.
    let grid = "\u{24FF}\u{2580}";
    let facts = parse_acs_screen(grid);
    assert_eq!(
        facts.notes,
        vec!["distinct_line_drawing_chars=0".to_string()]
    );
}

#[test]
fn parse_acs_screen_handles_realistic_tack_v108_output() {
    // Verifies: against the actual tack v1.08 ACS test
    // output (verified empirically — see module rustdoc), the
    // parser returns 0 distinct line-drawing chars because tack
    // v1.08's ACS test only probes (bel) and reports Done. The
    // parser does NOT panic, return wrong notes, or false-flag
    // any characters.
    let grid =
        "\nTesting bell (bel)\nIf you did not hear the Bell then (bel) has failed.  (bel) Done\n";
    let facts = parse_acs_screen(grid);
    assert_eq!(
        facts.notes,
        vec!["distinct_line_drawing_chars=0".to_string()]
    );
    assert!(facts.capability_labels.is_empty());
}

#[test]
fn parse_acs_screen_extracts_first_non_blank_line_as_header() {
    let grid = "\n\n   \nReal Header Line\n  body\n";
    let facts = parse_acs_screen(grid);
    assert_eq!(facts.header_text, "Real Header Line");
}

#[test]
fn parse_acs_screen_full_block_sweep_counts_all_128_codepoints() {
    // Verifies for the parser must count EVERY
    // codepoint in the inclusive U+2500..=U+257F block as a distinct
    // line-drawing char. The block is 128 codepoints (0x2500 through
    // 0x257F). A regression that off-by-one-clips the upper bound
    // (e.g., `..` instead of `..=`) or that uses a narrower range
    // (e.g., U+2500..=U+254F for box-drawing only) would surface as
    // a count below 128.
    let mut grid = String::new();
    for code in 0x2500u32..=0x257Fu32 {
        if let Some(ch) = char::from_u32(code) {
            grid.push(ch);
        }
    }
    let facts = parse_acs_screen(&grid);
    assert_eq!(
        facts.notes,
        vec!["distinct_line_drawing_chars=128".to_string()],
        "full sweep over U+2500..=U+257F must count all 128 distinct codepoints"
    );
}

#[test]
fn parse_acs_screen_preserves_count_across_multiple_lines() {
    // Verifies for line breaks must NOT affect the
    // distinct-char count. A regression that processes only the
    // first line, or that resets per-line, would fire here.
    let grid = "─\n│\n┌\n┐\n└\n┘\n";
    let facts = parse_acs_screen(grid);
    assert_eq!(
        facts.notes,
        vec!["distinct_line_drawing_chars=6".to_string()],
        "multi-line input must preserve cumulative distinct count"
    );
}
