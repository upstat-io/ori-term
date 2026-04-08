//! Sibling tests for the padding-and-strings parser.
//!
//! All tests call the pure `parse_padding_screen` helper directly so
//! they run on hosts without tack installed AND in
//! `cargo test --release`.
//!
//! Note: the parser uses `grid_has_paren_token`, so test inputs
//! wrap cap names in parens (`(rs1)` not `rs1`) to match tack's
//! canonical output format.

use super::parse_padding_screen;

#[test]
fn parse_padding_screen_handles_empty_grid() {
    let facts = parse_padding_screen("");
    assert_eq!(facts.header_text, "");
    assert!(facts.capability_labels.is_empty());
    assert!(facts.notes.is_empty());
}

#[test]
fn parse_padding_screen_finds_all_string_caps() {
    // Pin every string cap at once. Cap names wrapped in parens
    // — the canonical tack output format.
    let grid = "header line\n(rs1) (rs2) (rs3) (is1) (is2) (is3) (smcup) (rmcup) (smkx) (rmkx)\n";
    let facts = parse_padding_screen(grid);
    assert_eq!(
        facts.capability_labels,
        vec![
            "rs1".to_string(),
            "rs2".to_string(),
            "rs3".to_string(),
            "is1".to_string(),
            "is2".to_string(),
            "is3".to_string(),
            "smcup".to_string(),
            "rmcup".to_string(),
            "smkx".to_string(),
            "rmkx".to_string(),
        ]
    );
}

#[test]
fn parse_padding_screen_finds_each_cap_in_isolation() {
    // Pin every cap individually so a regression that swaps two
    // caps in the parser surfaces as a clear per-cap failure.
    const CAPS: &[&str] = &[
        "rs1", "rs2", "rs3", "is1", "is2", "is3", "smcup", "rmcup", "smkx", "rmkx",
    ];
    for cap in CAPS {
        let grid = format!("header line\n ({cap}) sample text\n");
        let facts = parse_padding_screen(&grid);
        assert_eq!(
            facts.capability_labels,
            vec![(*cap).to_string()],
            "isolation pin failed for cap {cap:?}: got {:?}",
            facts.capability_labels
        );
    }
}

#[test]
fn parse_padding_screen_rejects_substring_collisions() {
    // SEMANTIC PIN for the M3 fix: the parser MUST require the
    // parenthesized form via `grid_has_paren_token`, not raw
    // `str::contains`. The shorter cap names (`is1`, `is2`, `is3`,
    // `rs1`, `rs2`, `rs3`) would false-positive against any
    // sequence containing them — e.g. `is1` matches inside `is15`,
    // `rs2` inside `users2`. ALSO `reset_1string` contains the
    // substring `s1` and `string` contains `rs1`-as-substring
    // (no, wait, `string` contains `tring` not `rs1` — but the
    // point stands for other names). A regression to `str::contains`
    // would match every substring; a regression to
    // `grid_has_token` (whitespace-bounded only) would still
    // match the cap name if it appeared as a bare word but would
    // miss tack's actual `(cap)` parenthesized output. Requiring
    // parenthesized tokens is the strongest collision resistance.
    let grid = "\
is15 rs2nd users2 mismatched
rmcupboard smcuprite is3rd
rs3lemur smkxnonsense rmkxbluefoot
reset_1string contains s1 not (rs1)
";
    let facts = parse_padding_screen(grid);
    // Note: the LAST line contains `(rs1)` — that DOES match.
    // The point of this test is that NONE of the bare-word
    // substrings false-positive. Only the explicitly
    // parenthesized form on the last line should be detected.
    assert_eq!(
        facts.capability_labels,
        vec!["rs1".to_string()],
        "expected only the parenthesized (rs1) to match, got {:?} — \
         parser is using bare contains() not grid_has_paren_token",
        facts.capability_labels
    );
}

#[test]
fn parse_padding_screen_returns_caps_in_canonical_order() {
    // SEMANTIC PIN: the parser walks STRING_CAPS in declaration
    // order and pushes matches in that order, so the returned
    // `capability_labels` vec MUST appear in canonical order
    // [rs1, rs2, rs3, is1, is2, is3, smcup, rmcup, smkx, rmkx]
    // REGARDLESS of grid order. Pin by deliberately scrambling
    // the input.
    let grid = "(rmkx) (smkx) (rmcup) (smcup) (is3) (is2) (is1) (rs3) (rs2) (rs1)\n";
    let facts = parse_padding_screen(grid);
    assert_eq!(
        facts.capability_labels,
        vec![
            "rs1".to_string(),
            "rs2".to_string(),
            "rs3".to_string(),
            "is1".to_string(),
            "is2".to_string(),
            "is3".to_string(),
            "smcup".to_string(),
            "rmcup".to_string(),
            "smkx".to_string(),
            "rmkx".to_string(),
        ],
        "labels must be returned in canonical STRING_CAPS order, not grid order"
    );
}

#[test]
fn parse_padding_screen_handles_realistic_tack_v108_output() {
    // SEMANTIC PIN: against the actual tack v1.08 padding test
    // output (verified empirically — see module rustdoc), the
    // parser returns ["rs1"] because tack v1.08 emits the
    // `(rs1) reset_1string, not present.  (rs1) Done` line. The
    // "not present" part of tack's output reflects the current
    // state of extra/ori_term.info, which declares NO reset-string
    // capabilities at all (neither rs1, rs2, nor rs3) — see
    // TPR-05-021 for the empirical correction. Other caps in
    // STRING_CAPS are absent from the captured grid because tack
    // only probes caps that exist in the terminfo entry, and
    // none of them are declared.
    let grid = "(rs1) reset_1string, not present.  (rs1) Done\n";
    let facts = parse_padding_screen(grid);
    assert_eq!(facts.capability_labels, vec!["rs1".to_string()]);
}

#[test]
fn parse_padding_screen_handles_partial_caps() {
    // Pin that a partial subset (3 of 10) returns ONLY those caps
    // in canonical order, not all 10 with empty entries for
    // missing ones.
    let grid = "header\n(rs1) (smcup) (smkx)\n";
    let facts = parse_padding_screen(grid);
    assert_eq!(
        facts.capability_labels,
        vec!["rs1".to_string(), "smcup".to_string(), "smkx".to_string()]
    );
}

#[test]
fn parse_padding_screen_extracts_first_non_blank_line_as_header() {
    let grid = "\n\n   \nReal Header Line\n  body\n";
    let facts = parse_padding_screen(grid);
    assert_eq!(facts.header_text, "Real Header Line");
}

#[test]
fn parse_padding_screen_handles_cap_at_start_of_line() {
    let grid = "header\n(rs1) sample\n";
    let facts = parse_padding_screen(grid);
    assert_eq!(facts.capability_labels, vec!["rs1".to_string()]);
}

#[test]
fn parse_padding_screen_handles_cap_at_end_of_line() {
    let grid = "header\nthis line ends with (rs1)\n";
    let facts = parse_padding_screen(grid);
    assert_eq!(facts.capability_labels, vec!["rs1".to_string()]);
}
