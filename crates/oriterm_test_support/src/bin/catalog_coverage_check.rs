//! `catalog_coverage_check` — coverage gate for the
//! spec-conformance catalog (Section 01.3).
//!
//! CLI wrapper over [`oriterm_test_support::catalog`]. Exit codes:
//!
//! - `0` — success.
//! - `1` — one or more findings; also emitted on io / parse errors.
//!
//! See §01.3.a
//! for the full subcommand surface.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use oriterm_test_support::catalog::{
    CheckMode, Classification, Tuple, build_catalog_signature_set, build_dispatch_map,
    canonical_tuple, check, classify_from_map, extract_capture_tuples, extract_dispatch_tuples,
    extract_namedprivatemode_tuples, format_report, load_all_catalog_rows, parse_catalog_markdown,
    reconcile, tuple_signature, walk_catalog_files,
};
use oriterm_test_support::paths;

/// Resolve a wrapper-resident catalog dir argument: if user passed
/// `--catalog-dir`, use it verbatim; otherwise fall back to the discovered
/// wrapper layout. Returns None when neither is available — the caller
/// graceful-skips per `tests.md §Graceful Skip Protocol`.
fn resolve_catalog_dir(cli_arg: Option<PathBuf>) -> Option<PathBuf> {
    cli_arg.or_else(paths::catalog_dir)
}

/// Resolve a wrapper-resident captures dir argument. Same shape as
/// `resolve_catalog_dir`.
fn resolve_captures_dir(cli_arg: Option<PathBuf>) -> Option<PathBuf> {
    cli_arg.or_else(paths::captures_dir)
}

/// Emit a SKIP line and return successful exit when wrapper resources are
/// absent and the user didn't override via CLI. Per `tests.md §Graceful
/// Skip Protocol` for wrapper-conditional gates.
fn skip_no_wrapper(arg_name: &str, command_name: &str) -> ExitCode {
    eprintln!(
        "SKIP: {command_name} — wrapper repo not discoverable from {} \
 (standalone term_repo checkout — pass {arg_name} explicitly to override)",
        env!("CARGO_MANIFEST_DIR"),
    );
    ExitCode::SUCCESS
}

#[derive(Parser)]
#[command(
    name = "catalog_coverage_check",
    about = "Spec-conformance catalog coverage gate",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run the full check pass: schema, anti-LEAK, anti-drift.
    Check {
        /// Path to `plans/spec-conformance/catalog/`. Resolved via
        /// `paths::catalog_dir()` (wrapper repo walk-up) when omitted.
        /// Standalone `term_repo` without wrapper: SKIP + exit 0.
        #[arg(long)]
        catalog_dir: Option<PathBuf>,

        /// Enable bootstrap mode — reject `verified` rows. This is
        /// the CI gate for Section 01; it is retired after Section
        /// 04.7 freezes the schema.
        #[arg(long)]
        bootstrap_mode: bool,

        /// Workspace root for dispatch-coverage verification. When
        /// provided, asserts every dispatch tuple has a matching
        /// catalog row.
        #[arg(long)]
        workspace_root: Option<PathBuf>,
    },

    /// Extract dispatch tuples from the Rust source tree.
    ExtractDispatchTuples {
        /// Workspace root. Defaults to the current directory.
        #[arg(long, default_value = ".")]
        workspace_root: PathBuf,
    },

    /// Extract `NamedPrivateMode` tuples from `types.rs`.
    ExtractNamedPrivateModeTuples {
        #[arg(long, default_value = ".")]
        workspace_root: PathBuf,
    },

    /// Extract catalog row tuples (the canonical form of every
    /// `Sequence` column).
    ExtractCatalogTuples {
        #[arg(long)]
        catalog_dir: Option<PathBuf>,
    },

    /// Extract capture-file tuples via `vte::Parser`.
    ExtractCaptureTuples {
        /// Path to a `.cap` file.
        cap_file: PathBuf,
    },

    /// Classify one tuple: does `ori_term` dispatch it today?
    /// Returns the `Handler::*` method(s) if dispatched, or
    /// `no-dispatch` if the parser sees it but the handler drops it.
    Classify {
        /// The canonical tuple string (e.g. `(CSI, [], Ps, H)`).
        tuple: String,

        /// Workspace root for AST walking. Defaults to current dir.
        #[arg(long, default_value = ".")]
        workspace_root: PathBuf,
    },

    /// Extract top-down (spec-sourced) catalog tuples. Filters out
    /// rows with MISSING or wezterm in Spec source.
    ExtractTopDownTuples {
        #[arg(long)]
        catalog_dir: Option<PathBuf>,
    },

    /// Three-way reconciliation: bottom-up (dispatch) vs top-down
    /// (spec-sourced catalog) vs captures. Generates and prints the
    /// reconciliation report.
    Reconcile {
        #[arg(long, default_value = ".")]
        workspace_root: PathBuf,

        #[arg(long)]
        catalog_dir: Option<PathBuf>,

        #[arg(long)]
        captures_dir: Option<PathBuf>,
    },

    /// Assert that a capture's top 10 most-frequent tuples all have
    /// catalog rows. Exits 1 if any top-10 tuple is missing.
    CaptureTop10Covered {
        /// Path to a `.cap` file.
        cap_file: PathBuf,

        /// Path to `plans/spec-conformance/catalog/`.
        #[arg(long)]
        catalog_dir: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Check {
            catalog_dir,
            bootstrap_mode,
            workspace_root,
        } => {
            let Some(catalog_dir) = resolve_catalog_dir(catalog_dir) else {
                return skip_no_wrapper("--catalog-dir", "catalog_coverage_check check");
            };
            run_check(&catalog_dir, bootstrap_mode, workspace_root.as_deref())
        }
        Command::ExtractDispatchTuples { workspace_root } => run_extract_dispatch(&workspace_root),
        Command::ExtractNamedPrivateModeTuples { workspace_root } => {
            run_extract_namedprivatemode(&workspace_root)
        }
        Command::ExtractCatalogTuples { catalog_dir } => {
            let Some(catalog_dir) = resolve_catalog_dir(catalog_dir) else {
                return skip_no_wrapper("--catalog-dir", "extract-catalog-tuples");
            };
            run_extract_catalog(&catalog_dir)
        }
        Command::ExtractCaptureTuples { cap_file } => run_extract_captures(&cap_file),
        Command::Classify {
            tuple,
            workspace_root,
        } => run_classify(&tuple, &workspace_root),
        Command::ExtractTopDownTuples { catalog_dir } => {
            let Some(catalog_dir) = resolve_catalog_dir(catalog_dir) else {
                return skip_no_wrapper("--catalog-dir", "extract-top-down-tuples");
            };
            run_extract_top_down(&catalog_dir)
        }
        Command::Reconcile {
            workspace_root,
            catalog_dir,
            captures_dir,
        } => {
            let Some(catalog_dir) = resolve_catalog_dir(catalog_dir) else {
                return skip_no_wrapper("--catalog-dir", "reconcile");
            };
            let Some(captures_dir) = resolve_captures_dir(captures_dir) else {
                return skip_no_wrapper("--captures-dir", "reconcile");
            };
            run_reconcile(&workspace_root, &catalog_dir, &captures_dir)
        }
        Command::CaptureTop10Covered {
            cap_file,
            catalog_dir,
        } => {
            let Some(catalog_dir) = resolve_catalog_dir(catalog_dir) else {
                return skip_no_wrapper("--catalog-dir", "capture-top10-covered");
            };
            run_capture_top10_covered(&cap_file, &catalog_dir)
        }
    }
}

fn run_check(
    catalog_dir: &std::path::Path,
    bootstrap_mode: bool,
    workspace_root: Option<&std::path::Path>,
) -> ExitCode {
    let mode = if bootstrap_mode {
        CheckMode::Bootstrap
    } else {
        CheckMode::Normal
    };
    match check(catalog_dir, mode, workspace_root) {
        Err(e) => {
            eprintln!("catalog_coverage_check: {e}");
            ExitCode::from(1)
        }
        Ok(report) => {
            eprintln!(
                "scanned {} files, {} rows",
                report.files_scanned, report.rows_total
            );
            if report.has_findings() {
                for finding in &report.findings {
                    eprintln!(
                        "{}:{}: [{}] {}: {}",
                        finding.source_file,
                        finding.line_number,
                        finding.category,
                        finding.row_id,
                        finding.message
                    );
                }
                eprintln!("catalog_coverage_check: {} findings", report.findings.len());
                ExitCode::from(1)
            } else {
                eprintln!("catalog_coverage_check: OK");
                ExitCode::SUCCESS
            }
        }
    }
}

fn run_extract_dispatch(workspace_root: &std::path::Path) -> ExitCode {
    match extract_dispatch_tuples(workspace_root) {
        Err(e) => {
            eprintln!("extract_dispatch_tuples: {e}");
            ExitCode::from(1)
        }
        Ok(tuples) => {
            for tuple in tuples {
                println!("{tuple}");
            }
            ExitCode::SUCCESS
        }
    }
}

fn run_extract_namedprivatemode(workspace_root: &std::path::Path) -> ExitCode {
    match extract_namedprivatemode_tuples(workspace_root) {
        Err(e) => {
            eprintln!("extract_namedprivatemode_tuples: {e}");
            ExitCode::from(1)
        }
        Ok(tuples) => {
            for tuple in tuples {
                println!("{tuple}");
            }
            ExitCode::SUCCESS
        }
    }
}

fn run_extract_catalog(catalog_dir: &std::path::Path) -> ExitCode {
    let all_paths = match walk_catalog_files(catalog_dir) {
        Err(e) => {
            eprintln!("extract-catalog-tuples: {e}");
            return ExitCode::from(1);
        }
        Ok(p) => p,
    };
    for path in all_paths {
        let rows = match parse_catalog_markdown(&path) {
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(1);
            }
            Ok(r) => r,
        };
        for row in rows {
            if let Some(tuple) = canonical_tuple(&row.sequence) {
                println!("{} {tuple}", row.id);
            }
        }
    }
    ExitCode::SUCCESS
}

fn run_extract_captures(cap_file: &std::path::Path) -> ExitCode {
    match extract_capture_tuples(cap_file) {
        Err(e) => {
            eprintln!("extract_capture_tuples: {e}");
            ExitCode::from(1)
        }
        Ok(pairs) => {
            for (tuple, count) in pairs {
                println!("{count}\t{tuple}");
            }
            ExitCode::SUCCESS
        }
    }
}

fn run_classify(tuple_str: &str, workspace_root: &std::path::Path) -> ExitCode {
    let Some(tuple) = Tuple::from_display_str(tuple_str) else {
        eprintln!("classify: cannot parse tuple: {tuple_str}");
        return ExitCode::from(1);
    };
    let map = match build_dispatch_map(workspace_root) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("classify: {e}");
            return ExitCode::from(1);
        }
    };
    match classify_from_map(&map, &tuple) {
        Classification::Dispatched { handlers } => {
            println!("dispatched: {}", handlers.join(", "));
        }
        Classification::NoDispatch => {
            println!("no-dispatch");
        }
    }
    ExitCode::SUCCESS
}

fn run_capture_top10_covered(
    cap_file: &std::path::Path,
    catalog_dir: &std::path::Path,
) -> ExitCode {
    // Extract capture tuples (sorted by frequency, descending).
    let pairs = match extract_capture_tuples(cap_file) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("capture-top10-covered: {e}");
            return ExitCode::from(1);
        }
    };

    // Build the set of catalog tuple signatures — we compare on
    // (category, intermediates, final_byte) only, because params
    // differ between catalog form (`Ps`, `Ps;Ps`, per-SGR-code
    // numbers) and capture form (arity-based `Ps;Ps`).
    let catalog_sigs = match build_catalog_signature_set(catalog_dir) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("capture-top10-covered: {e}");
            return ExitCode::from(1);
        }
    };

    let top10: Vec<_> = pairs.iter().take(10).collect();
    let mut missing = Vec::new();
    for (tuple, count) in &top10 {
        let sig = tuple_signature(tuple);
        if !catalog_sigs.contains(&sig) {
            missing.push((tuple, *count));
        }
    }

    if missing.is_empty() {
        eprintln!(
            "capture-top10-covered: OK — top {} tuples all have catalog rows",
            top10.len()
        );
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "capture-top10-covered: {} of top {} tuples MISSING from catalog:",
            missing.len(),
            top10.len()
        );
        for (tuple, count) in &missing {
            eprintln!(" {count}\t{tuple}");
        }
        ExitCode::from(1)
    }
}

fn run_extract_top_down(catalog_dir: &std::path::Path) -> ExitCode {
    let rows = match load_all_catalog_rows(catalog_dir) {
        Err(e) => {
            eprintln!("extract-top-down-tuples: {e}");
            return ExitCode::from(1);
        }
        Ok(r) => r,
    };
    for row in &rows {
        if !row.has_spec_source() {
            continue;
        }
        if let Some(tuple) = canonical_tuple(&row.sequence) {
            println!("{} {tuple}", row.id);
        }
    }
    ExitCode::SUCCESS
}

fn run_reconcile(
    workspace_root: &std::path::Path,
    catalog_dir: &std::path::Path,
    captures_dir: &std::path::Path,
) -> ExitCode {
    let report = match reconcile(workspace_root, catalog_dir, captures_dir) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("reconcile: {e}");
            return ExitCode::from(1);
        }
    };
    let md = format_report(&report);
    print!("{md}");
    eprintln!(
        "reconcile: {} reconciled, {} de-facto, {} missing, {} capture-only",
        report.reconciled.len(),
        report.de_facto.len(),
        report.missing.len(),
        report.capture_only.len(),
    );
    ExitCode::SUCCESS
}
