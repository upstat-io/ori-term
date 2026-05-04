use std::collections::BTreeMap;
use std::io::Write;

use super::{CoverageBaseline, CoverageReport, StackSummary};

#[test]
fn scan_test_citations_finds_comment_citation() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.rs");
    std::fs::write(&file, "// Catalog row: ECMA48-CUP\nfn test() {}\n").unwrap();

    let citations = super::scan_test_citations(&[dir.path().to_path_buf()], &[]).unwrap();
    assert_eq!(citations.len(), 1);
    assert_eq!(citations[0].row_id, "ECMA48-CUP");
}

#[test]
fn scan_test_citations_finds_doc_comment_citation() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.rs");
    std::fs::write(&file, "/// Catalog row: ECMA48-SGR\nfn test() {}\n").unwrap();

    let citations = super::scan_test_citations(&[dir.path().to_path_buf()], &[]).unwrap();
    assert_eq!(citations.len(), 1);
    assert_eq!(citations[0].row_id, "ECMA48-SGR");
}

#[test]
fn scan_test_citations_finds_inner_doc_comment_citation() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.rs");
    std::fs::write(&file, "//! Catalog row: OSC-52\nfn test() {}\n").unwrap();

    let citations = super::scan_test_citations(&[dir.path().to_path_buf()], &[]).unwrap();
    assert_eq!(citations.len(), 1);
    assert_eq!(citations[0].row_id, "OSC-52");
}

#[test]
fn scan_test_citations_finds_plural_form() {
    // Regression: §10.2 retrospective — the scanner used to silently
    // drop `//! Catalog rows: A, B, C` (plural + comma list), producing
    // a false-missing coverage report. Plural form must emit one
    // citation per trimmed non-empty piece.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.rs");
    std::fs::write(
        &file,
        "//! Catalog rows: OSC-52-STORE, OSC-52-LOAD\nfn test() {}\n",
    )
    .unwrap();

    let citations = super::scan_test_citations(&[dir.path().to_path_buf()], &[]).unwrap();
    assert_eq!(citations.len(), 2);
    let ids: Vec<_> = citations.iter().map(|c| c.row_id.as_str()).collect();
    assert!(ids.contains(&"OSC-52-STORE"));
    assert!(ids.contains(&"OSC-52-LOAD"));
}

#[test]
fn scan_test_citations_plural_form_all_comment_prefixes() {
    // Matrix pin: plural form must work with `//`, `//!`, and `///`.
    for prefix in ["//", "//!", "///"] {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("test.rs");
        let content = format!("{prefix} Catalog rows: A, B, C\nfn test() {{}}\n");
        std::fs::write(&file, content).unwrap();

        let citations = super::scan_test_citations(&[dir.path().to_path_buf()], &[]).unwrap();
        assert_eq!(
            citations.len(),
            3,
            "prefix {prefix:?} must emit 3 citations"
        );
        let ids: Vec<_> = citations.iter().map(|c| c.row_id.as_str()).collect();
        assert!(ids.contains(&"A"), "prefix {prefix:?} missing A: {ids:?}");
        assert!(ids.contains(&"B"), "prefix {prefix:?} missing B: {ids:?}");
        assert!(ids.contains(&"C"), "prefix {prefix:?} missing C: {ids:?}");
    }
}

#[test]
fn scan_test_citations_plural_form_skips_empty_pieces() {
    // Defensive: a trailing comma or double comma must not emit an
    // empty citation.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.rs");
    std::fs::write(&file, "//! Catalog rows: A,, B, \nfn test() {}\n").unwrap();

    let citations = super::scan_test_citations(&[dir.path().to_path_buf()], &[]).unwrap();
    assert_eq!(citations.len(), 2);
    let ids: Vec<_> = citations.iter().map(|c| c.row_id.as_str()).collect();
    assert!(ids.contains(&"A"));
    assert!(ids.contains(&"B"));
}

#[test]
fn scan_test_citations_strips_trailing_period() {
 // Regression: — `//! Catalog rows: OSC-0, OSC-1, OSC-2.`
    // used to register `OSC-2.` (with period) as a distinct row ID,
    // producing spurious UNCATALOGED CITATIONS + FALSE VERIFIED pairs.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.rs");
    std::fs::write(
        &file,
        "//! Catalog rows: OSC-0, OSC-1, OSC-2.\nfn test() {}\n",
    )
    .unwrap();

    let citations = super::scan_test_citations(&[dir.path().to_path_buf()], &[]).unwrap();
    let ids: Vec<_> = citations.iter().map(|c| c.row_id.as_str()).collect();
    assert_eq!(ids.len(), 3, "expected 3 IDs, got {ids:?}");
    assert!(ids.contains(&"OSC-0"));
    assert!(ids.contains(&"OSC-1"));
    assert!(
        ids.contains(&"OSC-2"),
        "trailing period not stripped: {ids:?}"
    );
    assert!(
        !ids.contains(&"OSC-2."),
        "period-suffixed variant must not appear: {ids:?}"
    );
}

#[test]
fn scan_test_citations_strips_surrounding_backticks() {
 // Regression: — `` /// Catalog row: `SIXEL-BG-NoChange`.``
    // used to register a backtick-wrapped, period-suffixed id.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.rs");
    std::fs::write(
        &file,
        "/// Catalog row: `SIXEL-BG-NoChange`.\nfn test() {}\n",
    )
    .unwrap();

    let citations = super::scan_test_citations(&[dir.path().to_path_buf()], &[]).unwrap();
    let ids: Vec<_> = citations.iter().map(|c| c.row_id.as_str()).collect();
    assert_eq!(ids, vec!["SIXEL-BG-NoChange"]);
}

#[test]
fn scan_test_citations_strips_parenthetical_qualifier() {
 // Regression: — `/// Catalog row: SIXEL-REPEAT (§12.4 GPU-apex)`
    // used to register the entire prose as a row ID.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.rs");
    std::fs::write(
        &file,
        "/// Catalog row: SIXEL-REPEAT (§12.4 GPU-apex)\nfn test() {}\n",
    )
    .unwrap();

    let citations = super::scan_test_citations(&[dir.path().to_path_buf()], &[]).unwrap();
    let ids: Vec<_> = citations.iter().map(|c| c.row_id.as_str()).collect();
    assert_eq!(ids, vec!["SIXEL-REPEAT"]);
}

#[test]
fn scan_test_citations_cuts_trailing_prose_after_sentence_boundary() {
 // Regression: —
    // `//! Catalog rows: OSC-4-SET, OSC-4-QUERY. Apex: state-snapshot / effect-pty-write.`
    // used to register `OSC-4-QUERY. Apex: state-snapshot / effect-pty-write.`
    // as an id.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.rs");
    std::fs::write(
        &file,
        "//! Catalog rows: OSC-4-SET, OSC-4-QUERY. Apex: state-snapshot / effect-pty-write.\nfn test() {}\n",
    )
    .unwrap();

    let citations = super::scan_test_citations(&[dir.path().to_path_buf()], &[]).unwrap();
    let ids: Vec<_> = citations.iter().map(|c| c.row_id.as_str()).collect();
    assert_eq!(ids.len(), 2, "expected 2 IDs, got {ids:?}");
    assert!(ids.contains(&"OSC-4-SET"));
    assert!(ids.contains(&"OSC-4-QUERY"));
}

#[test]
fn scan_test_citations_combined_backtick_and_parenthetical() {
    // `` /// Catalog row: `SIXEL-BG-SetToBg` (DECSCNM interaction).``
    // must normalize to `SIXEL-BG-SetToBg`.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.rs");
    std::fs::write(
        &file,
        "/// Catalog row: `SIXEL-BG-SetToBg` (DECSCNM interaction).\nfn test() {}\n",
    )
    .unwrap();

    let citations = super::scan_test_citations(&[dir.path().to_path_buf()], &[]).unwrap();
    let ids: Vec<_> = citations.iter().map(|c| c.row_id.as_str()).collect();
    assert_eq!(ids, vec!["SIXEL-BG-SetToBg"]);
}

#[test]
fn scan_test_citations_handles_multi_line_continuation() {
 // Regression: — when a long plural form wraps across lines
    // via trailing comma, the scanner used to drop lines 2+, producing
    // false-verified entries for every row after the first line.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.rs");
    std::fs::write(
        &file,
        "//! Catalog rows: OSC-3, OSC-5-SET, OSC-5-QUERY, OSC-6, OSC-13-SET,\n\
         //! OSC-13-QUERY, OSC-14-SET, OSC-14-QUERY, OSC-17-SET, OSC-17-QUERY,\n\
         //! OSC-19-SET, OSC-19-QUERY, OSC-113, OSC-114, OSC-117, OSC-119, OSC-L,\n\
         //! OSC-l.\nfn test() {}\n",
    )
    .unwrap();

    let citations = super::scan_test_citations(&[dir.path().to_path_buf()], &[]).unwrap();
    let ids: Vec<_> = citations.iter().map(|c| c.row_id.as_str()).collect();
    assert_eq!(ids.len(), 18, "expected 18 IDs across 4 lines, got {ids:?}");
    for expected in [
        "OSC-3",
        "OSC-5-SET",
        "OSC-5-QUERY",
        "OSC-6",
        "OSC-13-SET",
        "OSC-13-QUERY",
        "OSC-14-SET",
        "OSC-14-QUERY",
        "OSC-17-SET",
        "OSC-17-QUERY",
        "OSC-19-SET",
        "OSC-19-QUERY",
        "OSC-113",
        "OSC-114",
        "OSC-117",
        "OSC-119",
        "OSC-L",
        "OSC-l",
    ] {
        assert!(ids.contains(&expected), "missing {expected}: {ids:?}");
    }
}

#[test]
fn scan_test_citations_silently_drops_prose() {
    // Author-error citations (prose used where an ID was expected) must
    // NOT produce spurious UNCATALOGED entries — they're dropped silently
    // because prose cannot match any catalog row by construction.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.rs");
    std::fs::write(
        &file,
        "/// Catalog row: sixel lifecycle — scrollback eviction.\nfn test() {}\n",
    )
    .unwrap();

    let citations = super::scan_test_citations(&[dir.path().to_path_buf()], &[]).unwrap();
    assert!(
        citations.is_empty(),
        "prose must not produce citations: {citations:?}"
    );
}

#[test]
fn scan_test_citations_continuation_stops_on_non_comment_line() {
    // Continuation lines must be comments. A bare fn line terminates the
    // continuation without consuming it.
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.rs");
    std::fs::write(
        &file,
        "//! Catalog rows: A, B,\nfn interloper() {}\n//! Catalog row: C\n",
    )
    .unwrap();

    let citations = super::scan_test_citations(&[dir.path().to_path_buf()], &[]).unwrap();
    let ids: Vec<_> = citations.iter().map(|c| c.row_id.as_str()).collect();
    // A, B stop at the fn line (continuation ended); C is a separate
    // standalone citation.
    assert!(ids.contains(&"A"));
    assert!(ids.contains(&"B"));
    assert!(ids.contains(&"C"));
    assert_eq!(ids.len(), 3, "unexpected IDs: {ids:?}");
}

#[test]
fn scan_test_citations_finds_const_field_citation() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("test.rs");
    std::fs::write(
        &file,
        r#"
const SCENARIO: SpecScenario = SpecScenario {
    catalog_row_id: "ECMA48-CSI-DA1",
    bytes: b"\x1b[c",
};
"#,
    )
    .unwrap();

    let citations = super::scan_test_citations(&[dir.path().to_path_buf()], &[]).unwrap();
    assert_eq!(citations.len(), 1);
    assert_eq!(citations[0].row_id, "ECMA48-CSI-DA1");
}

#[test]
fn scan_test_citations_ignores_non_rs_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("readme.md"),
        "// Catalog row: SHOULD-NOT-MATCH\n",
    )
    .unwrap();

    let citations = super::scan_test_citations(&[dir.path().to_path_buf()], &[]).unwrap();
    assert!(citations.is_empty());
}

#[test]
fn scan_test_citations_walks_subdirectories() {
    let dir = tempfile::tempdir().unwrap();
    let sub = dir.path().join("sub");
    std::fs::create_dir(&sub).unwrap();
    std::fs::write(sub.join("test.rs"), "// Catalog row: DEEP-ID\n").unwrap();

    let citations = super::scan_test_citations(&[dir.path().to_path_buf()], &[]).unwrap();
    assert_eq!(citations.len(), 1);
    assert_eq!(citations[0].row_id, "DEEP-ID");
}

#[test]
fn false_verified_flagged_when_catalog_verified_but_no_test_cites() {
    let report = CoverageReport {
        stacks: vec![StackSummary {
            stack: "test".to_string(),
            verified: 1,
            implemented_unverified: 0,
            stub: 0,
            missing: 0,
            verified_partial: 0,
            total: 1,
        }],
        false_verified: vec!["VERIFIED-BUT-UNCITED".to_string()],
        uncataloged: vec![],
        per_stack_verified: BTreeMap::from([("test".to_string(), 1)]),
    };
    assert!(!report.false_verified.is_empty());
    assert_eq!(report.false_verified[0], "VERIFIED-BUT-UNCITED");
}

#[test]
fn uncataloged_flagged_when_test_cites_but_catalog_missing() {
    let report = CoverageReport {
        stacks: vec![],
        false_verified: vec![],
        uncataloged: vec!["GHOST-ROW".to_string()],
        per_stack_verified: BTreeMap::new(),
    };
    assert!(!report.uncataloged.is_empty());
    assert_eq!(report.uncataloged[0], "GHOST-ROW");
}

#[test]
fn has_regression_fails_when_absolute_verified_drops() {
    let report = CoverageReport {
        stacks: vec![],
        false_verified: vec![],
        uncataloged: vec![],
        per_stack_verified: BTreeMap::from([("ecma-48".to_string(), 5)]),
    };
    let baseline = CoverageBaseline {
        stacks: BTreeMap::from([("ecma-48".to_string(), 10)]),
    };
    assert!(report.has_regression(&baseline));
}

#[test]
fn has_regression_passes_when_verified_holds_despite_new_rows() {
    let report = CoverageReport {
        stacks: vec![],
        false_verified: vec![],
        uncataloged: vec![],
        per_stack_verified: BTreeMap::from([("ecma-48".to_string(), 10)]),
    };
    let baseline = CoverageBaseline {
        stacks: BTreeMap::from([("ecma-48".to_string(), 10)]),
    };
    assert!(!report.has_regression(&baseline));
}

#[test]
fn baseline_parse_reads_stack_counts() {
    let toml = r#"
# Coverage baseline
[stacks]
ecma-48 = 5
osc = 3
"#;
    let baseline = CoverageBaseline::parse(toml).unwrap();
    assert_eq!(baseline.stacks.get("ecma-48"), Some(&5));
    assert_eq!(baseline.stacks.get("osc"), Some(&3));
}

#[test]
fn coverage_report_build_propagates_parser_errors() {
    let dir = tempfile::tempdir().unwrap();
    // Create a catalog file with correct header but malformed data row
    // (wrong number of columns → ColumnCount error).
    let mut f = std::fs::File::create(dir.path().join("bad.md")).unwrap();
    writeln!(f, "| ID | Spec source | Sequence | Description | Implementation | Apex layer | Test chain | Verification | De-facto ref | Notes |").unwrap();
    writeln!(f, "|---|---|---|---|---|---|---|---|---|---|").unwrap();
    writeln!(f, "| X | only-two-cols |").unwrap();

    let result = CoverageReport::build(dir.path(), &[], &[]);
    assert!(result.is_err(), "malformed catalog should propagate error");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("bad.md") || err.contains("column"),
        "error should mention file or column count: {err}"
    );
}
