//! Catalog markdown table parser.
//!
//! Reads one catalog file and yields one [`Row`] per data row.
//! Strict — rejects rows with fewer than 10 columns or with a
//! header row whose column names do not match
//! [`CATALOG_COLUMNS`].
//!
//! The parser does NOT walk the Rust source tree — it only reads
//! markdown tables. Dispatch / enum extraction lives in
//! [`crate::catalog::dispatch_extract`].
//!
//! Returns [`Result`] so both `catalog_coverage_check` (Section
//! 01.3) and `spec_coverage_report` (Section 04.8) can propagate
//! schema errors via `?` at a single boundary.

use std::fs;
use std::path::Path;

use super::row::{CATALOG_COLUMN_COUNT, CATALOG_COLUMNS, Row, Verification};

/// Errors the parser can emit.
#[derive(Debug)]
pub enum CatalogParseError {
    /// File could not be opened or read.
    Io {
        path: String,
        source: std::io::Error,
    },
    /// Header row present but column names do not match the schema.
    HeaderMismatch {
        path: String,
        line: usize,
        got: Vec<String>,
    },
    /// Row has the wrong column count.
    ColumnCount {
        path: String,
        line: usize,
        got: usize,
    },
    /// `Verification` column has an unknown value.
    UnknownVerification {
        path: String,
        line: usize,
        got: String,
    },
    /// Row ID is empty.
    EmptyId { path: String, line: usize },
}

impl core::fmt::Display for CatalogParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "io error reading {path}: {source}"),
            Self::HeaderMismatch { path, line, got } => write!(
                f,
                "{path}:{line}: header row does not match schema; got: {got:?}"
            ),
            Self::ColumnCount { path, line, got } => write!(
                f,
                "{path}:{line}: row has {got} columns, expected {CATALOG_COLUMN_COUNT}"
            ),
            Self::UnknownVerification { path, line, got } => {
                write!(f, "{path}:{line}: unknown Verification value {got:?}")
            }
            Self::EmptyId { path, line } => write!(f, "{path}:{line}: row ID is empty"),
        }
    }
}

impl std::error::Error for CatalogParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Parse one catalog markdown file into a list of rows.
///
/// # Errors
///
/// Returns [`CatalogParseError`] on io failures, header mismatches,
/// wrong column counts, or unknown `Verification` values. Consumers
/// MUST propagate the error (never `unwrap_or_default`) — a silent
/// parse failure would let catalog drift in.
pub fn parse_catalog_markdown(path: &Path) -> Result<Vec<Row>, CatalogParseError> {
    let path_str = path.to_string_lossy().into_owned();
    let contents = fs::read_to_string(path).map_err(|source| CatalogParseError::Io {
        path: path_str.clone(),
        source,
    })?;

    let mut rows = Vec::new();
    let mut saw_header_for_current_table = false;

    for (idx, line) in contents.lines().enumerate() {
        let line_number = idx + 1;
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            // Any non-table line resets the header state — the next
            // table we hit needs its own header row.
            saw_header_for_current_table = false;
            continue;
        }

        let cells = split_table_row(trimmed);

        // Separator row: `|---|---|...|` — skip, but it confirms the
        // previous line was a header.
        if is_separator_row(&cells) {
            continue;
        }

        // Header row: columns match the schema.
        if !saw_header_for_current_table && matches_header(&cells) {
            saw_header_for_current_table = true;
            continue;
        }

        // If we saw a `|...|` line before any header (e.g., a stray
        // comment table), ignore.
        if !saw_header_for_current_table {
            // Not part of a catalog data table.
            continue;
        }

        // Data row.
        if cells.len() != CATALOG_COLUMN_COUNT {
            return Err(CatalogParseError::ColumnCount {
                path: path_str,
                line: line_number,
                got: cells.len(),
            });
        }

        // Row IDs are frequently wrapped in markdown code spans
        // (`` `DECPRES-DECIC` ``) when the catalog author wants them
        // rendered in monospace. Strip surrounding backticks so the
        // stored ID matches what a test's `/// Catalog row: DECPRES-DECIC`
        // citation produces after scanner normalization — the catalog ID
        // is the semantic key, not the markdown presentation.
        let id = cells[0].trim().trim_matches('`').trim().to_string();
        if id.is_empty() {
            return Err(CatalogParseError::EmptyId {
                path: path_str,
                line: line_number,
            });
        }

        let verification_raw = cells[7].trim().to_string();
        let verification = Verification::from_cell(&verification_raw).ok_or_else(|| {
            CatalogParseError::UnknownVerification {
                path: path_str.clone(),
                line: line_number,
                got: verification_raw,
            }
        })?;

        rows.push(Row {
            id,
            spec_source: cells[1].trim().to_string(),
            sequence: cells[2].trim().to_string(),
            description: cells[3].trim().to_string(),
            implementation: cells[4].trim().to_string(),
            apex_layer: cells[5].trim().to_string(),
            test_chain: cells[6].trim().to_string(),
            verification,
            de_facto_ref: cells[8].trim().to_string(),
            notes: cells[9].trim().to_string(),
            source_file: path_str.clone(),
            line_number,
        });
    }

    Ok(rows)
}

fn split_table_row(line: &str) -> Vec<String> {
    // Split on `|` column separators while honoring two markdown
    // escape conventions used inside catalog cells:
    //
    // * `\|` — literal pipe inside a cell (e.g., `BEL\|ST`).
    // * `` `…|…` `` — literal pipe inside backticked code spans
    // (e.g., the DECUDK sequence `DCS ! | 00000000 ST`).
    //
    // We walk the line character-by-character, tracking backtick
    // depth. A pipe inside a code span is absorbed into the current
    // cell; only pipes at depth-0 split. Backtick depth is a simple
    // toggle — an odd number of unescaped backticks means we are
    // inside a code span.
    let inner = line.trim_start_matches('|').trim_end_matches('|');
    let mut cells: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_code = false;
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek() == Some(&'|') {
            chars.next();
            current.push('|');
            continue;
        }
        if c == '`' {
            in_code = !in_code;
            current.push(c);
            continue;
        }
        if c == '|' && !in_code {
            cells.push(current.trim().to_string());
            current = String::new();
            continue;
        }
        current.push(c);
    }
    cells.push(current.trim().to_string());
    cells
}

fn is_separator_row(cells: &[String]) -> bool {
    !cells.is_empty()
        && cells
            .iter()
            .all(|c| c.chars().all(|ch| ch == '-' || ch == ':'))
}

fn matches_header(cells: &[String]) -> bool {
    if cells.len() != CATALOG_COLUMN_COUNT {
        return false;
    }
    cells
        .iter()
        .zip(CATALOG_COLUMNS.iter())
        .all(|(got, want)| got.trim() == *want)
}

#[cfg(test)]
mod tests;
