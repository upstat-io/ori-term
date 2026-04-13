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

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use oriterm_test_support::spec_chain::coverage::{CoverageBaseline, CoverageReport};

fn main() -> ExitCode {
    let workspace_root = find_workspace_root();
    let catalog_dir = workspace_root.join("plans/spec-conformance/catalog");
    let test_roots: Vec<PathBuf> = vec![
        workspace_root.join("oriterm_core/tests"),
        workspace_root.join("oriterm_core/src"),
        workspace_root.join("oriterm/tests"),
        workspace_root.join("oriterm/src"),
        workspace_root.join("oriterm_ui/tests"),
        workspace_root.join("oriterm_mux/tests"),
        workspace_root.join("crates/oriterm_test_support/src"),
        workspace_root.join("crates/oriterm_test_support/tests"),
    ];

    let report = match CoverageReport::build(&catalog_dir, &test_roots) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    report.print_table();

    if std::env::args().any(|a| a == "--check") {
        let baseline_path = workspace_root.join("plans/spec-conformance/coverage-baseline.toml");
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

        if has_false_verified || has_uncataloged || has_regression {
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

/// Find the workspace root by walking up from `CARGO_MANIFEST_DIR`.
fn find_workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // oriterm_test_support is at crates/oriterm_test_support/
    // workspace root is two levels up.
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .map(Path::to_path_buf)
        .unwrap_or(manifest_dir)
}
