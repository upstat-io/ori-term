//! Tests for the `check` module: `--check` pass finding detection.

use std::io::Write;

use tempfile::TempDir;

use super::{CheckMode, FindingCategory, check};

// -------- Negative pins: --check rejection ---------------------------------

#[test]
fn check_rejects_wezterm_as_spec_source() {
    let (_tmp, catalog_dir) = make_fixture_catalog(&[(
        "ecma-48.md",
        row_markdown(
            "ECMA48-BAD",
            "wezterm escape-sequences.md",
            "`CSI X`",
            "implemented-unverified",
            "`Term::stub` (`oriterm_core/src/term/handler/mod.rs`)",
        ),
    )]);
    let report = check(&catalog_dir, CheckMode::Normal, None).expect("parse ok");
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.category == FindingCategory::SpecSourceCitesPeer),
        "expected SpecSourceCitesPeer; got: {:?}",
        report.findings
    );
}

#[test]
fn check_rejects_verified_status_in_bootstrap_mode() {
    let (_tmp, catalog_dir) = make_fixture_catalog(&[(
        "ecma-48.md",
        row_markdown(
            "ECMA48-BOOTSTRAP-VIOLATION",
            "ECMA-48 §8.3.21",
            "`CSI H`",
            "verified",
            "`Term::goto` (`oriterm_core/src/term/handler/mod.rs`)",
        ),
    )]);
    let bootstrap_report = check(&catalog_dir, CheckMode::Bootstrap, None).expect("parse ok");
    assert!(
        bootstrap_report
            .findings
            .iter()
            .any(|f| f.category == FindingCategory::VerifiedInBootstrap),
        "bootstrap must reject verified; got {:?}",
        bootstrap_report.findings
    );

    // And in Normal mode the same fixture passes.
    let normal_report = check(&catalog_dir, CheckMode::Normal, None).expect("parse ok");
    assert!(
        normal_report
            .findings
            .iter()
            .all(|f| f.category != FindingCategory::VerifiedInBootstrap),
        "normal mode must NOT reject verified"
    );
}

#[test]
fn check_rejects_line_number_primary_citation() {
    let (_tmp, catalog_dir) = make_fixture_catalog(&[(
        "ecma-48.md",
        row_markdown(
            "ECMA48-LN",
            "ECMA-48 §8.3.21",
            "`CSI H`",
            "implemented-unverified",
            "`crates/vte/src/ansi/dispatch/csi.rs:91 \u{2192} TermHandler::goto`",
        ),
    )]);
    let report = check(&catalog_dir, CheckMode::Normal, None).expect("parse ok");
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.category == FindingCategory::LineNumberPrimaryCitation),
        "expected LineNumberPrimaryCitation; got {:?}",
        report.findings
    );
}

#[test]
fn check_rejects_duplicate_row_id() {
    let (_tmp, catalog_dir) = make_fixture_catalog(&[
        (
            "ecma-48.md",
            row_markdown(
                "ECMA48-DUP",
                "ECMA-48 §8.3.21",
                "`CSI H`",
                "implemented-unverified",
                "`Term::goto` (`oriterm_core/src/term/handler/mod.rs`)",
            ),
        ),
        (
            "xterm-ctlseqs.md",
            row_markdown(
                "ECMA48-DUP",
                "xterm ctlseqs",
                "`CSI 14 t`",
                "implemented-unverified",
                "`Term::text_area_size_pixels` (`oriterm_core/src/term/handler/mod.rs`)",
            ),
        ),
    ]);
    let report = check(&catalog_dir, CheckMode::Normal, None).expect("parse ok");
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.category == FindingCategory::DuplicateId),
        "expected DuplicateId; got {:?}",
        report.findings
    );
}

// -------- Positive pin: full real catalog passes ---------------------------

#[test]
fn check_with_workspace_root_runs_dispatch_coverage() {
    let repo_root = std::env::var("CARGO_MANIFEST_DIR")
        .ok()
        .and_then(|mpd| {
            std::path::PathBuf::from(mpd)
                .parent()?
                .parent()
                .map(std::path::Path::to_path_buf)
        })
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let catalog_dir = repo_root.join("plans/spec-conformance/catalog");
    let vte_dispatch = repo_root.join("crates/vte/src/ansi/dispatch");
    if !catalog_dir.exists() || !vte_dispatch.exists() {
        eprintln!(
            "SKIP check_with_workspace_root_runs_dispatch_coverage: {} or {} missing",
            catalog_dir.display(),
            vte_dispatch.display()
        );
        return;
    }
    let report =
        check(&catalog_dir, CheckMode::Normal, Some(&repo_root)).expect("real catalog parses");
    assert!(
        report.files_scanned > 0,
        "expected at least one catalog file scanned"
    );
    assert!(report.rows_total > 0, "expected at least one catalog row");
    // Assert the dispatch-coverage path actually ran by checking for
    // DispatchWithoutCatalogRow findings. With `workspace_root = Some(...)`,
    // the check should have produced these findings (dispatch arms exist
    // that aren't covered by catalog rows). Without workspace_root, the
    // report would have zero of these. This pins the path distinction.
    let dispatch_findings: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.category == FindingCategory::DispatchWithoutCatalogRow)
        .collect();
    // Compare against a run WITHOUT workspace_root to pin the path
    // distinction. Without workspace_root, no dispatch-coverage findings
    // should exist (dispatch extraction is skipped).
    let report_no_ws =
        check(&catalog_dir, CheckMode::Normal, None).expect("real catalog parses without ws");
    let dispatch_findings_no_ws: Vec<_> = report_no_ws
        .findings
        .iter()
        .filter(|f| f.category == FindingCategory::DispatchWithoutCatalogRow)
        .collect();
    assert!(
        dispatch_findings_no_ws.is_empty(),
        "expected zero DispatchWithoutCatalogRow findings when workspace_root is None"
    );
    // With workspace_root, dispatch-coverage findings MUST exist (uncovered
    // dispatch arms always exist during active catalog development).
    assert!(
        !dispatch_findings.is_empty(),
        "expected at least one DispatchWithoutCatalogRow finding \
         when workspace_root is provided — the dispatch-coverage path \
         may not have run"
    );
}

// The original `real_catalog_passes_bootstrap_mode_check` test was a
// Section 01 gate that asserted the real catalog had zero `verified`
// rows (because bootstrap mode forbids them). The comment on
// `CheckMode::Bootstrap` explicitly bounds its lifetime: "Used by
// Section 01's gate until Section 04.7 freezes the schema."
//
// Section 04.7 froze the schema, and Section 08+ now drives rows to
// `verified` as the verification-chain harness lands. Asserting the
// real catalog still passes bootstrap mode is incompatible with the
// catalog's intended evolution — it would break on every future
// verification.
//
// The fixture-based `check_rejects_verified_status_in_bootstrap_mode`
// test above still exercises the bootstrap-mode rejection API itself,
// and `check_with_workspace_root_runs_dispatch_coverage` runs the real
// catalog through Normal mode (the post-bootstrap mode), so the
// real-catalog smoke coverage is preserved.

// -------- Fixture helpers --------------------------------------------------

fn row_markdown(
    id: &str,
    spec_source: &str,
    sequence: &str,
    verification: &str,
    implementation: &str,
) -> String {
    format!(
        r#"---
schema_version: "0.1-provisional"
---

# Test Fixture

| ID | Spec source | Sequence | Description | Implementation | Apex layer | Test chain | Verification | De-facto ref | Notes |
|---|---|---|---|---|---|---|---|---|---|
| {id} | {spec_source} | {sequence} | Test row | {implementation} | state-snapshot | parser:pending | {verification} | — | fixture |
"#,
    )
}

fn make_fixture_catalog(files: &[(&str, String)]) -> (TempDir, std::path::PathBuf) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let catalog_dir = tmp.path().join("catalog");
    std::fs::create_dir_all(&catalog_dir).expect("create catalog dir");
    for (name, body) in files {
        let path = catalog_dir.join(name);
        let mut file = std::fs::File::create(&path).expect("create fixture file");
        file.write_all(body.as_bytes()).expect("write fixture");
    }
    (tmp, catalog_dir)
}
