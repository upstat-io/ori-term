//! Sibling tests for the Section 06.3 character sets parser.
//!
//! All tests call the pure `parse_character_sets_screen` helper (or
//! its extracted distinct-counter) directly so they run on hosts
//! without tack installed and in `cargo test --release`.
//!
//! See the parent module rustdoc for the full empirical record on
//! why the parser is Unicode-only and the raw-ASCII branch is a
//! Regression guard instead of a positive test.

use super::{MIN_DEC_GRAPHICS_THRESHOLD, count_dec_graphics_chars, parse_character_sets_screen};

#[test]
fn parse_character_sets_screen_handles_empty_grid() {
    let facts = parse_character_sets_screen("");
    assert_eq!(facts.header_text, "");
    assert!(facts.capability_labels.is_empty());
    assert_eq!(
        facts.notes,
        vec!["dec_graphics_distinct_chars=0".to_string()]
    );
}

#[test]
fn parse_character_sets_unicode_box_drawing() {
    // POSITIVE PIN (a) — feed the canonical Unicode box-drawing form
    // that oriterm_core actually emits when DEC special graphics is
    // active. The parser counts DISTINCT code points: six unique
    // glyphs from the U+2500..=U+257F block.
    let grid = "┌─┐│└┘";
    let count = count_dec_graphics_chars(grid);
    assert!(
        count >= MIN_DEC_GRAPHICS_THRESHOLD,
        "expected at least {MIN_DEC_GRAPHICS_THRESHOLD} distinct \
         box-drawing chars, got {count}; grid:\n{grid}",
    );
    assert_eq!(count, 6);
}

#[test]
fn parse_character_sets_raw_ascii_line_drawing_does_not_match() {
    // REGRESSION GUARD (b) — feed the raw ASCII DEC line-drawing form
    // (the bytes a non-translating terminal would render as printable
    // letters). oriterm_core ALWAYS translates DEC special graphics
    // through `vte::ansi::StandardCharset::SpecialCharacterAndLineDrawing`
    // (verified at `oriterm_core/src/term/charset/tests.rs:14-37`),
    // so by the time bytes hit `grid_text()` they are Unicode box
    // drawing — never raw ASCII. The parser must therefore IGNORE the
    // ASCII form. A regression that swapped the parser to scan ASCII
    // bytes would flip this pin red.
    let grid = "lqkxmj";
    assert_eq!(
        count_dec_graphics_chars(grid),
        0,
        "raw ASCII line-drawing must NOT match the Unicode box-\
         drawing parser; oriterm_core translates these bytes to \
         U+2500..=U+257F before the grid is captured (see \
         `oriterm_core/src/term/charset/tests.rs:14-37`). If this \
         pin starts firing, the parser was widened to also accept \
         ASCII — that's a single-form-policy violation.",
    );
}

#[test]
fn parse_character_sets_negative_non_dec_grid() {
    // REGRESSION GUARD (d) — English prose with no box-drawing must
    // return 0. A regression that broadened the matched code point
    // range past U+2500..=U+257F would surface here.
    let grid = "Hello World, this has no graphics characters at all.";
    assert_eq!(count_dec_graphics_chars(grid), 0);
}

#[test]
fn parse_character_sets_mixed_unicode_and_ascii_only_counts_unicode() {
    // Verifies (e) — a grid containing BOTH the Unicode form
    // and the raw ASCII form must count ONLY the Unicode chars.
    // This proves the parser is single-form (Unicode-only) even
    // when the dead ASCII form is co-located. Two distinct Unicode
    // chars (`┌` and `─`) → count == 2; the trailing `lqk` ASCII
    // bytes are ignored.
    let grid = "┌─lqk";
    assert_eq!(count_dec_graphics_chars(grid), 2);
}

#[test]
fn parse_character_sets_boundary_count() {
    // Verifies (f) — a grid with EXACTLY
    // `MIN_DEC_GRAPHICS_THRESHOLD - 1` distinct Unicode box-
    // drawing chars returns exactly that count. Pins the
    // threshold as a comparison (`>=`) rather than an off-by-one
    // exact match, so a regression that swapped `>=` for `>`
    // would let the test wrapper accept three chars when the
    // contract demands four.
    let mut grid = String::new();
    for offset in 0..(MIN_DEC_GRAPHICS_THRESHOLD - 1) {
        let code = 0x2500_u32 + offset as u32;
        grid.push(char::from_u32(code).expect("U+2500..U+257F is valid"));
    }
    assert_eq!(
        count_dec_graphics_chars(&grid),
        MIN_DEC_GRAPHICS_THRESHOLD - 1,
        "boundary count must equal {} - 1 distinct chars; grid bytes: {:?}",
        MIN_DEC_GRAPHICS_THRESHOLD,
        grid.as_bytes(),
    );
}

#[test]
fn parse_character_sets_screen_extracts_first_non_blank_line_as_header() {
    // Header extraction parity with other Section 04/05/06 parsers:
    // skip leading blank lines and trim the first content line.
    let grid = "\n\n   \nDEC graphics test header\n┌─┐\n";
    let facts = parse_character_sets_screen(grid);
    assert_eq!(facts.header_text, "DEC graphics test header");
}

#[test]
fn parse_character_sets_screen_records_full_count_in_notes() {
    // SSOT pin: the parser writes EXACTLY one `dec_graphics_distinct_chars=`
    // note. The test wrapper parses this note prefix to extract the
    // count for its `>= MIN_DEC_GRAPHICS_THRESHOLD` assertion, so a
    // rename or reformat here would silently break the wrapper.
    let grid = "header\n┌─┐│└┘";
    let facts = parse_character_sets_screen(grid);
    assert_eq!(facts.notes.len(), 1);
    assert_eq!(facts.notes[0], "dec_graphics_distinct_chars=6");
}

#[test]
fn parse_character_sets_screen_counts_distinct_not_total() {
    // Verifies — a grid full of REPEATED `─` chars scores 1,
    // not N. The parser asserts diversity (corners + edges + lines),
    // not bulk repetition. A regression that swapped distinct-count
    // for total-count would flip this red.
    let grid = "──────────────────────────────────";
    assert_eq!(count_dec_graphics_chars(grid), 1);
}

#[test]
fn parse_character_sets_screen_handles_minimum_threshold_constant() {
    // Verifies: `MIN_DEC_GRAPHICS_THRESHOLD` is the canonical
    // threshold both the parser sibling tests and the 80x24 test
    // wrapper reference. Pin the value to 4 so a refactor that
    // lowered the threshold to match a broken capture also flips
    // this test red as a referee.
    assert_eq!(MIN_DEC_GRAPHICS_THRESHOLD, 4);
}

#[test]
fn parse_character_sets_screen_accepts_full_box_drawing_block_range() {
    // RANGE PIN — every code point in U+2500..=U+257F counts as
    // distinct. Iterates the full block boundary so a regression
    // that narrowed the accepted range (e.g. only U+2500..=U+253F)
    // surfaces here.
    let mut grid = String::new();
    for code in 0x2500_u32..=0x257F_u32 {
        grid.push(char::from_u32(code).expect("U+2500..=U+257F is valid"));
    }
    assert_eq!(
        count_dec_graphics_chars(&grid),
        0x80,
        "the full Box Drawing block (U+2500..=U+257F) is 128 chars",
    );
}

#[test]
fn parse_character_sets_screen_rejects_chars_just_outside_block() {
    // EDGE PIN — code points immediately before and after the
    // U+2500..=U+257F range must NOT count. Catches an off-by-one
    // boundary regression on either side of the inclusive range.
    let before = char::from_u32(0x24FF).expect("valid scalar");
    let after = char::from_u32(0x2580).expect("valid scalar");
    let mut grid = String::new();
    grid.push(before);
    grid.push(after);
    assert_eq!(count_dec_graphics_chars(&grid), 0);
}
