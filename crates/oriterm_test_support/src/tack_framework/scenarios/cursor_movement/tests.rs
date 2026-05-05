//! Sibling tests for the cursor movement parser.
//!
//! All tests call the pure `parse_cursor_screen` helper directly so
//! they run on hosts without tack installed AND in
//! `cargo test --release`.

use super::parse_cursor_screen;

#[test]
fn parse_cursor_screen_handles_empty_grid() {
    let facts = parse_cursor_screen("");
    assert_eq!(facts.header_text, "");
    assert!(facts.capability_labels.is_empty());
    assert!(facts.notes.is_empty());
}

#[test]
fn parse_cursor_screen_finds_all_eight_cursor_caps() {
    // Pin every cursor cap at once. Whitespace-separated tokens —
    // the canonical layout the parser expects.
    let grid = "header line\ncup hpa vpa csr cuu cud cub cuf\n";
    let facts = parse_cursor_screen(grid);
    assert_eq!(
        facts.capability_labels,
        vec![
            "cup".to_string(),
            "hpa".to_string(),
            "vpa".to_string(),
            "csr".to_string(),
            "cuu".to_string(),
            "cud".to_string(),
            "cub".to_string(),
            "cuf".to_string(),
        ]
    );
}

#[test]
fn parse_cursor_screen_finds_each_cap_in_isolation() {
    // Pin every cap individually so a regression that swaps two
    // caps in the parser surfaces as a clear per-cap failure.
    const CAPS: &[&str] = &["cup", "hpa", "vpa", "csr", "cuu", "cud", "cub", "cuf"];
    for cap in CAPS {
        let grid = format!("header line\n {cap} sample text\n");
        let facts = parse_cursor_screen(&grid);
        assert_eq!(
            facts.capability_labels,
            vec![(*cap).to_string()],
            "isolation pin failed for cap {cap:?}: got {:?}",
            facts.capability_labels
        );
    }
}

#[test]
fn parse_cursor_screen_rejects_substring_collisions() {
    // Verifies for the M3 fix: the parser MUST use
    // `grid_has_token`, not raw `str::contains`. The plan
    // explicitly calls out: `cup` matches inside
    // `cupboard`/`occupied`, `hpa`/`vpa` inside arbitrary letter
    // pairs (`hpattern`, `vpattern`, `vparams`). All 8 cursor cap
    // names are 3 characters and would silently false-positive
    // against any English word containing them. A regression to
    // `str::contains` would be invisible without this pin.
    let grid = "\
cupboard occupied hpattern vparams
cuummulus cudgel cubitus cuffed
csrubble
";
    let facts = parse_cursor_screen(grid);
    assert!(
        facts.capability_labels.is_empty(),
        "expected zero caps, got {:?} — parser is using raw contains() not grid_has_token",
        facts.capability_labels
    );
}

#[test]
fn parse_cursor_screen_handles_partial_caps() {
    // Pin that a partial subset (3 of 8) returns ONLY those caps
    // in canonical order, not all 8 with empty entries for missing
    // ones.
    let grid = "header\ncup hpa vpa\n";
    let facts = parse_cursor_screen(grid);
    assert_eq!(
        facts.capability_labels,
        vec!["cup".to_string(), "hpa".to_string(), "vpa".to_string()]
    );
}

#[test]
fn parse_cursor_screen_returns_caps_in_canonical_order() {
    // Verifies: the parser walks CURSOR_CAPS in declaration
    // order and pushes matches in that order, so the returned
    // `capability_labels` vec MUST appear in canonical
    // [cup, hpa, vpa, csr, cuu, cud, cub, cuf] order REGARDLESS
    // of grid order. Pin by deliberately scrambling the input.
    let grid = "cuf cub cud cuu csr vpa hpa cup\n";
    let facts = parse_cursor_screen(grid);
    assert_eq!(
        facts.capability_labels,
        vec![
            "cup".to_string(),
            "hpa".to_string(),
            "vpa".to_string(),
            "csr".to_string(),
            "cuu".to_string(),
            "cud".to_string(),
            "cub".to_string(),
            "cuf".to_string(),
        ],
        "labels must be returned in canonical CURSOR_CAPS order, not grid order"
    );
}

#[test]
fn parse_cursor_screen_handles_realistic_tack_v108_output() {
    // Verifies: against the actual tack v1.08 cursor movement
    // test output (verified empirically — see module rustdoc),
    // the parser returns no cursor caps because tack v1.08's
    // cursor movement test only probes (clear) cap name — it does
    // NOT emit any of the 8 cursor cap labels. The parser does
    // NOT panic, false-flag, or return spurious matches.
    let grid = "\nThis line should start in the home position.\nThe rest of the screen should be clear.  (clear) Done\n";
    let facts = parse_cursor_screen(grid);
    assert!(facts.capability_labels.is_empty());
}

#[test]
fn parse_cursor_screen_extracts_first_non_blank_line_as_header() {
    let grid = "\n\n   \nReal Header Line\n  body\n";
    let facts = parse_cursor_screen(grid);
    assert_eq!(facts.header_text, "Real Header Line");
}

#[test]
fn parse_cursor_screen_handles_cap_at_start_of_line() {
    let grid = "header\ncup sample\n";
    let facts = parse_cursor_screen(grid);
    assert_eq!(facts.capability_labels, vec!["cup".to_string()]);
}

#[test]
fn parse_cursor_screen_handles_cap_at_end_of_line() {
    let grid = "header\nthis line ends with cup\n";
    let facts = parse_cursor_screen(grid);
    assert_eq!(facts.capability_labels, vec!["cup".to_string()]);
}
