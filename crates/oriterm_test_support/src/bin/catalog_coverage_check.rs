//! `catalog_coverage_check` — coverage gate for the
//! spec-conformance catalog (Section 01.3).
//!
//! CLI wrapper over [`oriterm_test_support::catalog`]. Exit codes:
//!
//! - `0` — success.
//! - `1` — one or more findings; also emitted on io / parse errors.
//!
//! See `plans/spec-conformance/section-01-catalog-bootstrap.md §01.3.a`
//! for the full subcommand surface.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use oriterm_test_support::catalog::{
    CheckMode, canonical_tuple, check, extract_capture_tuples, extract_dispatch_tuples,
    extract_namedprivatemode_tuples, parse_catalog_markdown,
};

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
        /// Path to `plans/spec-conformance/catalog/` (defaults to
        /// the path relative to the workspace root).
        #[arg(long, default_value = "plans/spec-conformance/catalog")]
        catalog_dir: PathBuf,

        /// Enable bootstrap mode — reject `verified` rows. This is
        /// the CI gate for Section 01; it is retired after Section
        /// 04.7 freezes the schema.
        #[arg(long)]
        bootstrap_mode: bool,
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
        #[arg(long, default_value = "plans/spec-conformance/catalog")]
        catalog_dir: PathBuf,
    },

    /// Extract capture-file tuples via `vte::Parser`.
    ExtractCaptureTuples {
        /// Path to a `.cap` file.
        cap_file: PathBuf,
    },

    /// Classify one tuple: does `ori_term` dispatch it today?
    Classify {
        /// The canonical tuple string (e.g. `(CSI, [], Ps, H)`).
        tuple: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Check {
            catalog_dir,
            bootstrap_mode,
        } => run_check(&catalog_dir, bootstrap_mode),
        Command::ExtractDispatchTuples { workspace_root } => run_extract_dispatch(&workspace_root),
        Command::ExtractNamedPrivateModeTuples { workspace_root } => {
            run_extract_namedprivatemode(&workspace_root)
        }
        Command::ExtractCatalogTuples { catalog_dir } => run_extract_catalog(&catalog_dir),
        Command::ExtractCaptureTuples { cap_file } => run_extract_captures(&cap_file),
        Command::Classify { tuple } => run_classify(&tuple),
    }
}

fn run_check(catalog_dir: &std::path::Path, bootstrap_mode: bool) -> ExitCode {
    let mode = if bootstrap_mode {
        CheckMode::Bootstrap
    } else {
        CheckMode::Normal
    };
    match check(catalog_dir, mode) {
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
    let files = match std::fs::read_dir(catalog_dir) {
        Err(e) => {
            eprintln!("read_dir {}: {e}", catalog_dir.display());
            return ExitCode::from(1);
        }
        Ok(f) => f,
    };
    let mut all_paths: Vec<PathBuf> = Vec::new();
    for entry in files {
        let entry = match entry {
            Err(e) => {
                eprintln!("dir entry: {e}");
                return ExitCode::from(1);
            }
            Ok(e) => e,
        };
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "md") {
            let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if filename != "README.md" && !filename.starts_with('_') {
                all_paths.push(path);
            }
        }
    }
    all_paths.sort();
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

fn run_classify(tuple: &str) -> ExitCode {
    println!("classify: {tuple} (not yet implemented — see Section 01.5)");
    ExitCode::SUCCESS
}
