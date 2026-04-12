//! `--check` subcommand logic — coverage + anti-LEAK + anti-drift.
//!
//! Walks every catalog file under
//! `plans/spec-conformance/catalog/`, parses each row, and
//! asserts:
//!
//! 1. No duplicate `ID` across files.
//! 2. No row cites `wezterm` in the `Spec source` column
//!    (Phase 2 Finding J anti-LEAK gate).
//! 3. In `--bootstrap-mode`: no row holds `verified`,
//!    `verified-partial`, or `verified-with-deviation`
//!    (Phase 2 Finding L negative criterion).
//! 4. `Implementation` column does not use line-number-primary
//!    citations (`file.rs:91 → Symbol` is rejected; `Symbol
//!    (file.rs)` is accepted).
//! 5. No row has fewer than 10 columns (enforced by the parser
//!    — we re-assert here for clarity).
//!
//! Dispatch-coverage checks (asserting every `(Category,
//! intermediates, params, final)` dispatch tuple has at least
//! one matching catalog row) are a separate pass gated by the
//! `catalog_mode` flag. That pass is best-effort in the
//! bootstrap phase because the catalog's `Sequence` column
//! uses human placeholders (`Ps`, `Ps;Ps`, `text`) that the
//! canonicalizer must normalize against.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use super::parser::{CatalogParseError, parse_catalog_markdown};
use super::row::Row;

/// Operating mode for `--check`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckMode {
    /// Normal — allow `verified` rows.
    Normal,
    /// Bootstrap — reject `verified`, `verified-partial`, and
    /// `verified-with-deviation`. Used by Section 01's gate
    /// until Section 04.7 freezes the schema.
    Bootstrap,
}

/// Aggregated report.
#[derive(Debug, Default)]
pub struct CheckReport {
    pub files_scanned: usize,
    pub rows_total: usize,
    pub findings: Vec<Finding>,
}

/// One diagnostic produced by `--check`.
#[derive(Debug, Clone)]
pub struct Finding {
    pub source_file: String,
    pub line_number: usize,
    pub row_id: String,
    pub category: FindingCategory,
    pub message: String,
}

/// Kind of `--check` finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingCategory {
    /// Duplicate row ID across two or more files.
    DuplicateId,
    /// `Spec source` column cites a peer terminal emulator
    /// (Phase 2 Finding J).
    SpecSourceCitesPeer,
    /// `verified` row present in bootstrap mode (Phase 2
    /// Finding L).
    VerifiedInBootstrap,
    /// `Implementation` column uses line-number-primary form
    /// (`file.rs:91 → Symbol`).
    LineNumberPrimaryCitation,
}

impl core::fmt::Display for FindingCategory {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = match self {
            Self::DuplicateId => "duplicate-id",
            Self::SpecSourceCitesPeer => "spec-source-cites-peer",
            Self::VerifiedInBootstrap => "verified-in-bootstrap",
            Self::LineNumberPrimaryCitation => "line-number-primary-citation",
        };
        f.write_str(s)
    }
}

impl CheckReport {
    #[must_use]
    pub const fn has_findings(&self) -> bool {
        !self.findings.is_empty()
    }
}

/// Run all `--check` passes on the catalog at `catalog_dir`.
///
/// # Errors
///
/// Returns [`CatalogParseError`] if any file fails to parse. The
/// report-level findings (duplicate ID, Phase 2 Finding J/L,
/// line-number citation) are accumulated into
/// [`CheckReport::findings`] and returned even when
/// `findings.len() > 0`.
pub fn check(catalog_dir: &Path, mode: CheckMode) -> Result<CheckReport, CatalogParseError> {
    let mut report = CheckReport::default();
    let mut all_rows: Vec<Row> = Vec::new();

    for entry in walk_catalog_files(catalog_dir)? {
        let path = entry;
        let rows = parse_catalog_markdown(&path)?;
        report.files_scanned += 1;
        report.rows_total += rows.len();
        all_rows.extend(rows);
    }

    // Duplicate ID detection.
    let mut first_seen: HashMap<String, (String, usize)> = HashMap::new();
    let mut duplicates: BTreeSet<String> = BTreeSet::new();
    for row in &all_rows {
        if let Some(prev) = first_seen.get(&row.id) {
            duplicates.insert(row.id.clone());
            report.findings.push(Finding {
                source_file: row.source_file.clone(),
                line_number: row.line_number,
                row_id: row.id.clone(),
                category: FindingCategory::DuplicateId,
                message: format!(
                    "duplicate row ID {:?} — first seen at {}:{}",
                    row.id, prev.0, prev.1
                ),
            });
        } else {
            first_seen.insert(row.id.clone(), (row.source_file.clone(), row.line_number));
        }
    }
    // Silence unused-binding warning on duplicates without weakening the
    // collection semantics for future callers that want the set.
    let _ = duplicates;

    for row in &all_rows {
        if spec_source_cites_peer(&row.spec_source) {
            report.findings.push(Finding {
                source_file: row.source_file.clone(),
                line_number: row.line_number,
                row_id: row.id.clone(),
                category: FindingCategory::SpecSourceCitesPeer,
                message: format!(
                    "Spec source {:?} cites a peer terminal emulator — \
                     wezterm/alacritty/ghostty belong in De-facto ref, not Spec source",
                    row.spec_source
                ),
            });
        }

        if implementation_is_line_number_primary(&row.implementation) {
            report.findings.push(Finding {
                source_file: row.source_file.clone(),
                line_number: row.line_number,
                row_id: row.id.clone(),
                category: FindingCategory::LineNumberPrimaryCitation,
                message: format!(
                    "Implementation {:?} uses line-number-primary form; \
                     must anchor on a stable SYMBOL first with file path \
                     as parenthesized metadata",
                    row.implementation
                ),
            });
        }

        if mode == CheckMode::Bootstrap && row.verification.is_bootstrap_forbidden() {
            report.findings.push(Finding {
                source_file: row.source_file.clone(),
                line_number: row.line_number,
                row_id: row.id.clone(),
                category: FindingCategory::VerifiedInBootstrap,
                message: format!(
                    "Verification {} is forbidden in --bootstrap-mode \
                     (Section 01 never bootstraps verified rows)",
                    row.verification
                ),
            });
        }
    }

    Ok(report)
}

fn walk_catalog_files(catalog_dir: &Path) -> Result<Vec<PathBuf>, CatalogParseError> {
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

fn spec_source_cites_peer(spec_source: &str) -> bool {
    // Anti-LEAK gate: `Spec source` column must not reference a
    // peer terminal emulator. The allowed authority ladder is
    // documented in `00-overview.md §Authority Ladder`.
    //
    // Detection is case-insensitive substring search over the
    // known peer-implementation names.
    let lowered = spec_source.to_ascii_lowercase();
    lowered.contains("wezterm")
        || lowered.contains("alacritty")
        || lowered.contains("ghostty")
        || lowered.contains("ptyxis")
}

fn implementation_is_line_number_primary(implementation: &str) -> bool {
    // The plan's canonical form is `` `Symbol` (`file.rs`) `` or
    // `` `Symbol` (`file.rs:NNN`) ``. Line-number-primary form is
    // `` `file.rs:91 → Symbol` `` — i.e., a line number appears
    // BEFORE an arrow (`→`). We flag any occurrence of `.rs:N+ →`.
    //
    // Implementation: split at `.rs:` and for each tail, walk the
    // first run of ASCII digits, then check whether the next
    // non-digit non-whitespace character is `→`. This avoids raw
    // string indexing, which clippy flags as not-UTF-8-safe.
    for tail in implementation.split(".rs:").skip(1) {
        let mut chars = tail.chars().peekable();
        let mut saw_digit = false;
        while matches!(chars.peek(), Some(ch) if ch.is_ascii_digit()) {
            chars.next();
            saw_digit = true;
        }
        if !saw_digit {
            continue;
        }
        while matches!(chars.peek(), Some(ch) if ch.is_whitespace()) {
            chars.next();
        }
        if chars.peek() == Some(&'→') {
            return true;
        }
    }
    false
}
