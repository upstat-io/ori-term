//! Spec-conformance coverage report generator.
//!
//! Walks catalog files via the shared `catalog::parse_catalog_markdown`
//! parser, scans test directories for catalog row ID citations, and
//! produces a per-stack absolute-verified-count table.
//!
//! The gating metric is the ABSOLUTE count of `verified` rows per stack
//! (monotonic). Percentage is advisory only — because new rows are added
//! continuously, the denominator grows and percentages can drop while
//! absolute counts hold steady.

mod scan;

pub use scan::{Citation, scan_test_citations};

use std::collections::BTreeMap;
use std::path::Path;

use crate::catalog::{Row, Verification, parse_catalog_markdown, walk_catalog_files};

/// Per-stack summary of catalog row verification status.
#[derive(Debug, Clone)]
pub struct StackSummary {
    /// Stack name (derived from catalog filename without `.md` extension).
    pub stack: String,
    /// Number of rows with `verified` or `verified-with-deviation` status.
    pub verified: u32,
    /// Number of rows with `implemented-unverified` status.
    pub implemented_unverified: u32,
    /// Number of rows with `stub` status.
    pub stub: u32,
    /// Number of rows with `missing` status.
    pub missing: u32,
    /// Number of rows with `verified-partial` status.
    pub verified_partial: u32,
    /// Total row count for this stack.
    pub total: u32,
}

/// Coverage report aggregating catalog state and test citations.
#[derive(Debug, Clone)]
pub struct CoverageReport {
    /// Per-stack summaries.
    pub stacks: Vec<StackSummary>,
    /// Row IDs marked `verified` in the catalog but not cited by any test.
    pub false_verified: Vec<String>,
    /// Row IDs cited by tests but not present in any catalog file.
    pub uncataloged: Vec<String>,
    /// Per-stack absolute verified count (the gating metric).
    pub per_stack_verified: BTreeMap<String, u32>,
}

impl CoverageReport {
    /// Build a coverage report from the catalog directory and test roots.
    ///
    /// Uses `walk_catalog_files` and `parse_catalog_markdown` (SSOT — no
    /// inline parsing). Error propagation is load-bearing: parser errors
    /// propagate via `?`, never swallowed with `.unwrap_or_default()`.
    ///
    /// `exclude_dirs` prevents the citation scanner from reading its own
    /// source code as if it were test citations.
    pub fn build(
        catalog_dir: &Path,
        test_dirs: &[std::path::PathBuf],
        exclude_dirs: &[std::path::PathBuf],
    ) -> Result<Self, CoverageError> {
        let catalog_files = walk_catalog_files(catalog_dir)?;
        let mut all_rows: Vec<(String, Row)> = Vec::new();

        for path in &catalog_files {
            let stack_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            let rows = parse_catalog_markdown(path)?;
            for row in rows {
                all_rows.push((stack_name.clone(), row));
            }
        }

        let citations = scan_test_citations(test_dirs, exclude_dirs)?;
        Ok(Self::aggregate(&all_rows, &citations))
    }

    fn aggregate(rows: &[(String, Row)], citations: &[Citation]) -> Self {
        let mut stack_map: BTreeMap<String, StackSummary> = BTreeMap::new();
        let mut verified_ids: Vec<String> = Vec::new();
        let mut all_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

        for (stack, row) in rows {
            let summary = stack_map
                .entry(stack.clone())
                .or_insert_with(|| StackSummary {
                    stack: stack.clone(),
                    verified: 0,
                    implemented_unverified: 0,
                    stub: 0,
                    missing: 0,
                    verified_partial: 0,
                    total: 0,
                });
            summary.total += 1;
            match row.verification {
                Verification::Verified | Verification::VerifiedWithDeviation => {
                    summary.verified += 1;
                    verified_ids.push(row.id.clone());
                }
                Verification::VerifiedPartial => summary.verified_partial += 1,
                Verification::ImplementedUnverified => summary.implemented_unverified += 1,
                Verification::Stub => summary.stub += 1,
                Verification::Missing => summary.missing += 1,
            }
            all_ids.insert(row.id.clone());
        }

        let cited_ids: std::collections::HashSet<String> =
            citations.iter().map(|c| c.row_id.clone()).collect();

        let false_verified: Vec<String> = verified_ids
            .iter()
            .filter(|id| !cited_ids.contains(*id))
            .cloned()
            .collect();

        // Exclude TEST-* scaffold IDs — these are harness unit test
        // fixtures, not real coverage claims.
        let uncataloged: Vec<String> = cited_ids
            .iter()
            .filter(|id| !all_ids.contains(*id) && !id.starts_with("TEST-"))
            .cloned()
            .collect();

        let per_stack_verified: BTreeMap<String, u32> = stack_map
            .iter()
            .map(|(k, v)| (k.clone(), v.verified))
            .collect();

        Self {
            stacks: stack_map.into_values().collect(),
            false_verified,
            uncataloged,
            per_stack_verified,
        }
    }

    /// Check if any stack's verified count dropped below the baseline.
    pub fn has_regression(&self, baseline: &CoverageBaseline) -> bool {
        for (stack, &count) in &self.per_stack_verified {
            if let Some(&baseline_count) = baseline.stacks.get(stack) {
                if count < baseline_count {
                    return true;
                }
            }
        }
        false
    }

    /// Print the per-stack table to stdout.
    pub fn print_table(&self) {
        println!(
            "{:<25} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
            "Stack", "Verified", "Partial", "Impl", "Stub", "Missing", "Total"
        );
        println!("{}", "-".repeat(85));
        for s in &self.stacks {
            println!(
                "{:<25} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
                s.stack,
                s.verified,
                s.verified_partial,
                s.implemented_unverified,
                s.stub,
                s.missing,
                s.total
            );
        }
        let total_verified: u32 = self.stacks.iter().map(|s| s.verified).sum();
        let total_all: u32 = self.stacks.iter().map(|s| s.total).sum();
        println!("{}", "-".repeat(85));
        println!("{:<25} {:>8} {:>8}", "TOTAL", total_verified, total_all);
    }
}

/// Baseline of per-stack absolute verified counts.
///
/// The gating metric is monotonic: a stack's verified count may only
/// increase. A drop from baseline fails CI.
#[derive(Debug, Clone)]
pub struct CoverageBaseline {
    /// Per-stack verified count.
    pub stacks: BTreeMap<String, u32>,
}

impl CoverageBaseline {
    /// Load a baseline from a TOML file.
    ///
    /// Format:
    /// ```toml
    /// [stacks]
    /// ecma-48 = 0
    /// osc = 0
    /// ```
    pub fn load(path: &Path) -> Result<Self, CoverageError> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            CoverageError::Io(format!("failed to read baseline {}: {e}", path.display()))
        })?;
        Self::parse(&content)
    }

    /// Parse baseline TOML content.
    pub fn parse(content: &str) -> Result<Self, CoverageError> {
        let mut stacks = BTreeMap::new();
        let mut in_stacks = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if trimmed == "[stacks]" {
                in_stacks = true;
                continue;
            }
            if trimmed.starts_with('[') {
                in_stacks = false;
                continue;
            }
            if in_stacks {
                if let Some((key, val)) = trimmed.split_once('=') {
                    let key = key.trim().to_string();
                    let val_str = val.trim();
                    let val: u32 = val_str.parse().map_err(|_e| {
                        CoverageError::Baseline(format!(
                            "invalid count for stack '{key}': {val_str}"
                        ))
                    })?;
                    stacks.insert(key, val);
                }
            }
        }
        Ok(Self { stacks })
    }
}

/// Errors from coverage report building.
#[derive(Debug)]
pub enum CoverageError {
    /// Catalog file I/O or parse error.
    Catalog(crate::catalog::CatalogParseError),
    /// Test directory scan error.
    Scan(String),
    /// I/O error.
    Io(String),
    /// Baseline parse error.
    Baseline(String),
}

impl std::fmt::Display for CoverageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Catalog(e) => write!(f, "catalog error: {e}"),
            Self::Scan(msg) => write!(f, "scan error: {msg}"),
            Self::Io(msg) => write!(f, "I/O error: {msg}"),
            Self::Baseline(msg) => write!(f, "baseline error: {msg}"),
        }
    }
}

impl std::error::Error for CoverageError {}

impl From<crate::catalog::CatalogParseError> for CoverageError {
    fn from(e: crate::catalog::CatalogParseError) -> Self {
        Self::Catalog(e)
    }
}

#[cfg(test)]
mod tests;
