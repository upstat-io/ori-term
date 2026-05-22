//! Catalog coverage checker — shared library for `catalog_coverage_check`
//! and Section 04.8's `spec_coverage_report`.
//!
//! This module is the SSOT parser for the spec-conformance catalog
//! (`plans/spec-conformance/catalog/*.md`) and the bottom-up tuple
//! extractors that walk `crates/vte/src/ansi/dispatch/*.rs` and
//! `crates/vte/src/ansi/types.rs`. Two binaries consume it as a
//! library:
//!
//! - `catalog_coverage_check` (Section 01.3) — bootstrap-mode
//!   coverage gate. Drives CI to fail when a dispatch arm is
//!   missing a catalog row, when a catalog row references a
//!   stale symbol, when `Spec source` cites a peer terminal
//!   (wezterm et al.), or when a `Verification: verified` row
//!   is present in bootstrap mode.
//! - `spec_coverage_report` (Section 04.8) — per-stack absolute
//!   verified count + citation scan. Imports
//!   [`parse_catalog_markdown`] to populate its coverage table.
//!
//! All public parsers return [`Result`] so callers can propagate
//! schema errors via `?` at a single boundary. Consumers MUST NOT
//! swallow parse failures — the parser is the SSOT for catalog
//! schema enforcement, and silently dropping parse failures would
//! let drift in.
//!
//! See §01.3
//! for the full scope.

pub mod capture_extract;
pub mod check;
pub mod classify;
pub mod dispatch_extract;
pub mod parser;
pub mod reconcile;
pub mod row;
pub mod tuple;

use std::path::{Path, PathBuf};

pub use capture_extract::extract_capture_tuples;
pub use check::{CheckMode, CheckReport, check};
pub use classify::{Classification, build_dispatch_map, classify_from_map};
pub use dispatch_extract::{
    extract_dispatch_map, extract_dispatch_tuples, extract_namedprivatemode_tuples,
};
pub use parser::{CatalogParseError, parse_catalog_markdown};
pub use reconcile::{
    ReconciliationReport, build_catalog_signature_set, build_sig_to_row_ids, format_report,
    load_all_catalog_rows, reconcile, tuple_signature,
};
pub use row::{CATALOG_COLUMN_COUNT, CATALOG_COLUMNS, Row, Verification};
pub use tuple::TupleSig;
pub use tuple::{Category, Tuple, canonical_tuple, osc_placeholder};

/// Walk a catalog directory and return sorted paths to all `.md` data
/// files, excluding `README.md` and `_`-prefixed files.
///
/// This is the SSOT for catalog file enumeration — used by
/// [`check::check`], [`reconcile::load_all_catalog_rows`], and the
/// `extract-catalog-tuples` binary subcommand.
pub fn walk_catalog_files(catalog_dir: &Path) -> Result<Vec<PathBuf>, CatalogParseError> {
    let mut out = Vec::new();
    let read = std::fs::read_dir(catalog_dir).map_err(|source| CatalogParseError::Io {
        path: catalog_dir.to_string_lossy().into_owned(),
        source,
    })?;
    for entry in read {
        let entry = entry.map_err(|source| CatalogParseError::Io {
            path: catalog_dir.to_string_lossy().into_owned(),
            source,
        })?;
        let path = entry.path();
        // Skip the README and the tack mapping file — they are
        // schema-documentation files, not row tables.
        if path.extension().is_none_or(|ext| ext != "md") {
            continue;
        }
        let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if filename == "README.md" || filename.starts_with('_') {
            continue;
        }
        out.push(path);
    }
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests;
