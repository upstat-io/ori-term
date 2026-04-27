//! Spec-conformance coverage report binary.
//!
//! Walks `plans/spec-conformance/catalog/*.md` and scans test/source
//! directories for catalog row ID citations. Prints a per-stack
//! absolute-verified-count table. In `--check` mode, fails on
//! false-verified rows, uncataloged citations, or regression below
//! baseline.
//!
//! Run: `cargo run -p oriterm_test_support --bin spec-coverage-report`
//! Check: `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check`
//! Audit-files lint: `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files`
//! Explain citations in one file: `cargo run -p oriterm_test_support --bin spec-coverage-report -- --explain <path>`

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use oriterm_test_support::catalog::build_catalog_signature_set;
use oriterm_test_support::paths;
use oriterm_test_support::spec_chain::coverage::{
    CoverageBaseline, CoverageReport, check_audit_files, explain_file,
};
use oriterm_test_support::spec_chain::uncataloged;

fn main() -> ExitCode {
    let term_root = paths::term_workspace_root();
    let args: Vec<String> = std::env::args().collect();

    // `--explain <path>` walks one file's citation lines and prints per-
    // piece normalizer trace (accepted vs. dropped, with drop reason).
    // The path argument is term-repo-relative — test/source files live under
    // term_repo/, NOT the wrapper.
    if let Some(explain_idx) = args.iter().position(|a| a == "--explain") {
        return run_explain(&args, explain_idx, term_root);
    }

    // Wrapper-resident gate: catalog, baseline, audit-files all live at the
    // wrapper root. Standalone term_repo checkout = graceful skip + exit 0
    // per `.claude/rules/tests.md §Graceful Skip Protocol`. Explicit gates
    // (catalog/baseline/audits) require the wrapper; without it there is
    // nothing this binary can do. `paths::catalog_dir()` is the SSOT entry
    // point — every wrapper-relative subpath below derives from it.
    let Some(catalog_dir) = paths::catalog_dir() else {
        eprintln!(
            "SKIP: spec-coverage-report — wrapper repo not discoverable from {}",
            env!("CARGO_MANIFEST_DIR")
        );
        eprintln!(
            "       (standalone term_repo checkout — wrapper-only `plans/spec-conformance/` absent)"
        );
        return ExitCode::SUCCESS;
    };

    // `--check audit-files` runs the top-down audit-file lint ONLY
    // (separate gate from the coverage report). Wire point for the
    // audits/ SSOT introduced in Section 09A.
    if args.iter().any(|a| a == "audit-files") && args.iter().any(|a| a == "--check") {
        let plan_root =
            paths::spec_conformance_dir().expect("wrapper present (catalog_dir resolved above)");
        return run_audit_files_lint(&plan_root);
    }

    let test_roots: Vec<PathBuf> = vec![
        term_root.join("oriterm_core/tests"),
        term_root.join("oriterm_core/src"),
        term_root.join("oriterm/tests"),
        term_root.join("oriterm/src"),
        term_root.join("oriterm_ui/tests"),
        term_root.join("oriterm_ui/src"),
        term_root.join("oriterm_mux/tests"),
        term_root.join("oriterm_mux/src"),
        term_root.join("crates/oriterm_test_support/src"),
        term_root.join("crates/oriterm_test_support/tests"),
        term_root.join("crates/vte/src"),
    ];

    // Exclude the scanner's own source and binary to prevent it from
    // reading doc-comment examples as real catalog citations.
    let exclude_dirs: Vec<PathBuf> = vec![
        term_root.join("crates/oriterm_test_support/src/bin"),
        term_root.join("crates/oriterm_test_support/src/spec_chain/coverage"),
    ];

    let report = match CoverageReport::build(&catalog_dir, &test_roots, &exclude_dirs) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    report.print_table();

    if std::env::args().any(|a| a == "--check") {
        let baseline_path =
            paths::coverage_baseline_path().expect("wrapper present (catalog_dir resolved above)");
        let baseline = match CoverageBaseline::load(&baseline_path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("error loading baseline: {e}");
                return ExitCode::FAILURE;
            }
        };

        let has_false_verified = !report.false_verified.is_empty();
        let has_uncataloged = !report.uncataloged.is_empty();
        let has_regression = report.has_regression(&baseline);

        if has_false_verified {
            eprintln!("FALSE VERIFIED (catalog says verified but no test cites):");
            for row in &report.false_verified {
                eprintln!("  {row}");
            }
        }

        if has_uncataloged {
            eprintln!("UNCATALOGED CITATIONS (test cites row ID not in catalog):");
            for row in &report.uncataloged {
                eprintln!("  {row}");
            }
        }

        if has_regression {
            eprintln!("REGRESSION: absolute verified count dropped for one or more stacks");
        }

        // Gate 4: uncataloged backlog — sequences observed during test
        // execution that have no matching catalog row. Subtracts the
        // known catalog signature set so already-cataloged sequences
        // don't trigger false failures.
        let spool_dir = term_root.join("target/spec-chain-uncataloged");
        let catalog_sigs = build_catalog_signature_set(&catalog_dir).unwrap_or_default();
        let has_backlog = report_uncataloged_backlog(&spool_dir, &catalog_sigs);

        if has_false_verified || has_uncataloged || has_regression || has_backlog {
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

/// Report the UNCATALOGED BACKLOG gate — sequences observed during
/// test execution that have no matching catalog row. Returns `true`
/// when the gate should fail the run.
///
/// Output shape: count line, per-category histogram (`BTreeMap` so
/// categories are alphabetically ordered), then the full sorted list
/// (by category, then `final_byte`). The histogram surfaces at a
/// glance which categories drive the backlog; without it, an
/// unsorted dump (BUG-07-019 retrospective: 49 entries pre-fix) is
/// hard to scan.
fn report_uncataloged_backlog(
    spool_dir: &Path,
    catalog_sigs: &std::collections::BTreeSet<oriterm_test_support::catalog::TupleSig>,
) -> bool {
    let tuples = match uncataloged::read_accumulated_tuples(&spool_dir.to_path_buf()) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("warning: failed to read uncataloged backlog: {e}");
            return false;
        }
    };

    let unknown: Vec<_> = tuples
        .iter()
        .filter(|sig| !catalog_sigs.contains(*sig))
        .collect();
    if unknown.is_empty() {
        return false;
    }

    eprintln!(
        "UNCATALOGED BACKLOG ({} distinct tuples observed but not in catalog):",
        unknown.len()
    );

    let mut by_category: std::collections::BTreeMap<&str, usize> =
        std::collections::BTreeMap::new();
    for (cat, _, _) in &unknown {
        *by_category.entry(cat.as_str()).or_insert(0) += 1;
    }
    eprintln!("  by category:");
    for (cat, count) in &by_category {
        eprintln!("    {cat}: {count}");
    }

    eprintln!("  full list (sorted by category, then final_byte):");
    let mut sorted: Vec<_> = unknown.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0).then(a.2.cmp(&b.2)));
    for (cat, intermediates, final_byte) in sorted {
        let hex: Vec<String> = intermediates.iter().map(|b| format!("{b:02x}")).collect();
        eprintln!(
            "    [{cat}] intermediates=[{}] final={final_byte}",
            hex.join(",")
        );
    }

    true
}

/// Resolve the `--explain <path>` flag and run the per-file citation
/// trace via `explain_file`. Returns a process exit code.
fn run_explain(args: &[String], explain_idx: usize, workspace_root: &Path) -> ExitCode {
    let Some(path_arg) = args.get(explain_idx + 1) else {
        eprintln!("error: --explain requires a file path argument");
        eprintln!("usage: spec-coverage-report --explain <path-to-test-file.rs>");
        return ExitCode::FAILURE;
    };
    let path = PathBuf::from(path_arg);
    let path = if path.is_absolute() {
        path
    } else {
        workspace_root.join(&path)
    };
    if !path.exists() {
        eprintln!("error: file not found: {}", path.display());
        return ExitCode::FAILURE;
    }
    match explain_file(&path) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Run the top-down audit-file lint. Fails (non-zero exit) when any of
/// the four lint checks surface findings. Clean runs produce no output.
fn run_audit_files_lint(plan_root: &Path) -> ExitCode {
    let report = match check_audit_files(plan_root) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };
    report.print_summary();
    if report.has_failures() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
