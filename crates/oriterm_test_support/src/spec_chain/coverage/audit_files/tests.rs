//! Unit tests for the audit-file lint — matrix coverage for all four
//! lint checks (existence, mapping-resolution, schema-conformance,
//! freshness) using synthetic plan-root fixtures.

use std::fs;
use std::path::Path;

use super::{Decision, check_audit_files, parse_audit_file};

/// Write a minimal 4-file spec-conformance fixture under `root`:
///   root/catalog/test-stack.md  — two valid catalog rows
///   root/00-overview.md         — Quick Reference with statuses
///   root/audits/                — empty audits dir
/// Returns the root path (backed by the caller's tempdir).
fn write_fixture(root: &Path, quick_ref_rows: &[(&str, &str)]) {
    fs::create_dir_all(root.join("catalog")).unwrap();
    fs::create_dir_all(root.join("audits")).unwrap();

    // Minimal valid catalog file with the schema's 10-col header plus
    // two data rows whose IDs we use in tests below.
    let catalog = "\
# Test Catalog

| ID | Spec source | Sequence | Description | Implementation | Apex layer | Test chain | Verification | De-facto ref | Notes |
|---|---|---|---|---|---|---|---|---|---|
| `TEST-FOO` | test spec | `` `CSI A` `` | foo | MISSING | state-snapshot | parser:pending | missing | — | test |
| `TEST-BAR` | test spec | `` `CSI B` `` | bar | MISSING | state-snapshot | parser:pending | missing | — | test |
";
    fs::write(root.join("catalog/test-stack.md"), catalog).unwrap();

    // Quick Reference table with caller-specified row data.
    let mut overview = String::from(
        "# Overview

## Quick Reference

| ID | Title | File | Status |
|----|---    |---   |---     |
",
    );
    for (id, status) in quick_ref_rows {
        overview.push_str(&format!("| {id} | Title | file.md | {status} |\n"));
    }
    fs::write(root.join("00-overview.md"), overview).unwrap();
}

fn write_audit(root: &Path, filename: &str, contents: &str) {
    fs::write(root.join("audits").join(filename), contents).unwrap();
}

const VALID_MINIMAL_AUDIT: &str = "---
section: \"99\"
title: \"Test Section\"
canonical_spec_sources:
  - \"Test spec\"
last_walked: 2026-04-19
walked_by: \"tester\"
---

# Audit

## Sequence-to-catalog mapping

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| `CSI A` | test spec | `TEST-FOO` | mapped |
| `CSI C` | test spec | — | not-targeted: deprecated in VT5xx |
";

#[test]
fn clean_fixture_lints_without_failures() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_fixture(root, &[("99", "Complete")]);
    write_audit(
        root,
        "section-99-top-down-inventory.md",
        VALID_MINIMAL_AUDIT,
    );

    let report = check_audit_files(root).unwrap();
    assert!(
        !report.has_failures(),
        "clean fixture lint failed: {report:#?}"
    );
}

#[test]
fn mapping_to_nonexistent_catalog_row_fails_lint() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_fixture(root, &[("99", "Complete")]);
    let audit = VALID_MINIMAL_AUDIT.replace("`TEST-FOO`", "`TEST-DOES-NOT-EXIST`");
    write_audit(root, "section-99-top-down-inventory.md", &audit);

    let report = check_audit_files(root).unwrap();
    assert_eq!(report.unresolved_mappings.len(), 1);
    assert_eq!(report.unresolved_mappings[0].row_id, "TEST-DOES-NOT-EXIST");
}

#[test]
fn empty_not_targeted_rationale_fails_lint() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_fixture(root, &[("99", "Complete")]);
    let audit = VALID_MINIMAL_AUDIT.replace("not-targeted: deprecated in VT5xx", "not-targeted:");
    write_audit(root, "section-99-top-down-inventory.md", &audit);

    let report = check_audit_files(root).unwrap();
    assert!(
        report
            .schema_failures
            .iter()
            .any(|f| f.reason.contains("empty rationale")),
        "expected empty-rationale schema failure, got {report:#?}"
    );
}

#[test]
fn missing_section_frontmatter_fails_lint() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_fixture(root, &[("99", "Complete")]);
    let audit = VALID_MINIMAL_AUDIT.replace("section: \"99\"\n", "");
    write_audit(root, "section-99-top-down-inventory.md", &audit);

    let report = check_audit_files(root).unwrap();
    assert!(
        report
            .schema_failures
            .iter()
            .any(|f| f.reason.contains("section")),
        "expected missing-section schema failure, got {report:#?}"
    );
}

#[test]
fn missing_last_walked_fails_freshness() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_fixture(root, &[("99", "Complete")]);
    let audit = VALID_MINIMAL_AUDIT.replace("last_walked: 2026-04-19\n", "");
    write_audit(root, "section-99-top-down-inventory.md", &audit);

    let report = check_audit_files(root).unwrap();
    assert_eq!(report.freshness_failures.len(), 1);
    assert!(
        report.freshness_failures[0].reason.contains("last_walked"),
        "unexpected freshness failure reason: {:?}",
        report.freshness_failures[0].reason
    );
}

#[test]
fn malformed_last_walked_date_fails_freshness() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_fixture(root, &[("99", "Complete")]);
    let audit = VALID_MINIMAL_AUDIT.replace("last_walked: 2026-04-19", "last_walked: yesterday");
    write_audit(root, "section-99-top-down-inventory.md", &audit);

    let report = check_audit_files(root).unwrap();
    assert_eq!(report.freshness_failures.len(), 1);
    assert!(report.freshness_failures[0].reason.contains("YYYY-MM-DD"));
}

#[test]
fn null_sentinel_last_walked_passes_freshness() {
    // Stub audit files (committed alongside verbiage rewrites) use
    // `last_walked: null` until the implementer walks the spec. The
    // lint must NOT flag this as a freshness failure.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_fixture(root, &[("99", "Complete")]);
    let audit = VALID_MINIMAL_AUDIT.replace("last_walked: 2026-04-19", "last_walked: null");
    write_audit(root, "section-99-top-down-inventory.md", &audit);

    let report = check_audit_files(root).unwrap();
    assert!(
        report.freshness_failures.is_empty(),
        "null sentinel unexpectedly triggered freshness failure: {report:#?}"
    );
}

#[test]
fn in_progress_section_without_audit_file_fails_existence() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_fixture(root, &[("42", "In Progress")]);
    // No audit file written.

    let report = check_audit_files(root).unwrap();
    assert_eq!(report.missing_audit_files, vec!["42".to_string()]);
}

#[test]
fn not_started_section_without_audit_file_is_exempt() {
    // README §Lint contract: not-started sections are exempted until
    // §NN.0 execution. A clean lint must not flag them.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_fixture(root, &[("42", "Not Started")]);

    let report = check_audit_files(root).unwrap();
    assert!(
        report.missing_audit_files.is_empty(),
        "not-started section incorrectly flagged: {report:#?}"
    );
}

#[test]
fn complete_section_without_audit_file_is_exempt() {
    // Complete sections have their audit file permanently committed —
    // the existence check does not apply.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_fixture(root, &[("42", "Complete")]);

    let report = check_audit_files(root).unwrap();
    assert!(
        report.missing_audit_files.is_empty(),
        "complete section incorrectly flagged: {report:#?}"
    );
}

#[test]
fn wrong_column_count_row_is_flagged() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_fixture(root, &[("99", "Complete")]);
    let audit = VALID_MINIMAL_AUDIT.replace(
        "| `CSI A` | test spec | `TEST-FOO` | mapped |",
        "| `CSI A` | test spec | `TEST-FOO` |",
    );
    write_audit(root, "section-99-top-down-inventory.md", &audit);

    let report = check_audit_files(root).unwrap();
    assert!(
        report
            .schema_failures
            .iter()
            .any(|f| f.reason.contains("wrong column count")),
        "expected wrong-column-count failure, got {report:#?}"
    );
}

#[test]
fn malformed_frontmatter_does_not_panic() {
 // Regression guard: the parser must survive arbitrary junk inside the
    // frontmatter block without panicking.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_fixture(root, &[("99", "Complete")]);
    let audit = "---\n::::nonsense::::\n---\n\n## Sequence-to-catalog mapping\n\n| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |\n|---|---|---|---|\n";
    write_audit(root, "section-99-top-down-inventory.md", audit);

    let report = check_audit_files(root).unwrap();
    // Should have schema failure (missing section field) but not panic.
    assert!(report.has_failures());
}

#[test]
fn parse_decision_handles_both_forms() {
    let fixture = "---
section: \"99\"
title: \"Test\"
last_walked: 2026-04-19
---

## Sequence-to-catalog mapping

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| `A` | spec | `ID-1` | mapped |
| `B` | spec | — | not-targeted: intentionally skipped |
";
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("audit.md");
    fs::write(&path, fixture).unwrap();
    let parsed = parse_audit_file(&path).unwrap();
    assert_eq!(parsed.rows.len(), 2);
    assert_eq!(parsed.rows[0].decision, Decision::Mapped);
    match &parsed.rows[1].decision {
        Decision::NotTargeted { rationale } => {
            assert_eq!(rationale, "intentionally skipped");
        }
        other => panic!("expected NotTargeted, got {other:?}"),
    }
}

#[test]
fn todo_placeholder_rows_are_skipped() {
    // Stub audit files use `_**TODO…**_` rows as implementer
    // placeholders. The parser must skip these — they are not
    // schema failures.
    let fixture = "---
section: \"99\"
title: \"Test\"
last_walked: null
---

## Sequence-to-catalog mapping

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| _**TODO: implementer populates this table.**_ | | | |
";
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("audit.md");
    fs::write(&path, fixture).unwrap();
    let parsed = parse_audit_file(&path).unwrap();
    assert!(
        parsed.rows.is_empty(),
        "TODO placeholder rows must be skipped: {:?}",
        parsed.rows
    );
}
