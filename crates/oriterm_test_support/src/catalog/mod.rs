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
//! See `plans/spec-conformance/section-01-catalog-bootstrap.md §01.3`
//! for the full scope.

pub mod capture_extract;
pub mod check;
pub mod classify;
pub mod dispatch_extract;
pub mod parser;
pub mod row;
pub mod tuple;

pub use capture_extract::extract_capture_tuples;
pub use check::{CheckMode, CheckReport, check};
pub use classify::{Classification, build_dispatch_map, classify, classify_from_map};
pub use dispatch_extract::{extract_dispatch_tuples, extract_namedprivatemode_tuples};
pub use parser::{CatalogParseError, parse_catalog_markdown};
pub use row::{CATALOG_COLUMN_COUNT, CATALOG_COLUMNS, Row, Verification};
pub use tuple::{Category, Tuple, canonical_tuple};

#[cfg(test)]
mod tests;
