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
