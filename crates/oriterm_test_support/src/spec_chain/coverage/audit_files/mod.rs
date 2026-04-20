//! Top-down audit-file lint (`spec-coverage-report --check audit-files`).
//!
//! Enforces the 4-part lint contract documented in
//! `plans/spec-conformance/audits/README.md §Lint contract`:
//!
//! 1. **Existence** — every section in `00-overview.md` Quick Reference
//!    with status `in-progress` has a corresponding
//!    `audits/section-NN-top-down-inventory.md` file.
//! 2. **Mapping resolution** — every row with `Decision: mapped` cites a
//!    catalog row ID that exists in some `catalog/*.md` file.
//! 3. **Schema conformance** — frontmatter parses; every mapping row has
//!    exactly 4 columns; every `not-targeted` decision carries a
//!    non-empty rationale.
//! 4. **Freshness** — `last_walked` is present and parses as
//!    `YYYY-MM-DD` or the sentinel `null` (stub files use `null` until
//!    the implementer walks the spec).

mod parse;

pub use parse::parse_audit_file;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::catalog::reconcile::load_all_catalog_rows;

use self::parse::{extract_row_id, is_separator_row, split_table_row};

/// Outcome of one lint pass over the audits directory.
#[derive(Debug, Default)]
pub struct AuditFilesReport {
    /// Sections present in Quick Reference as `in-progress` but missing
    /// an `audits/section-NN-top-down-inventory.md` file.
    pub missing_audit_files: Vec<String>,
    /// `Decision: mapped` rows that cite a catalog row ID which does
    /// not resolve to any `catalog/*.md` file.
    pub unresolved_mappings: Vec<UnresolvedMapping>,
    /// Schema violations — malformed frontmatter, wrong column count,
    /// empty `not-targeted` rationale.
    pub schema_failures: Vec<SchemaFailure>,
    /// Files whose `last_walked` field is absent or malformed.
    pub freshness_failures: Vec<FreshnessFailure>,
}

impl AuditFilesReport {
    /// Returns `true` when any lint check failed.
    #[must_use]
    pub fn has_failures(&self) -> bool {
        !self.missing_audit_files.is_empty()
            || !self.unresolved_mappings.is_empty()
            || !self.schema_failures.is_empty()
            || !self.freshness_failures.is_empty()
    }

    /// Print a human-readable summary to stderr. Empty sections are
    /// omitted so a clean run produces no output.
    pub fn print_summary(&self) {
        if !self.missing_audit_files.is_empty() {
            eprintln!("MISSING AUDIT FILES (in-progress sections without audit file):");
            for section in &self.missing_audit_files {
                eprintln!("  section {section}");
            }
        }
        if !self.unresolved_mappings.is_empty() {
            eprintln!("UNRESOLVED MAPPINGS (audit cites row ID not found in any catalog):");
            for m in &self.unresolved_mappings {
                eprintln!(
                    "  {}:{}: mapped row ID {:?} does not resolve",
                    m.audit_path.display(),
                    m.line,
                    m.row_id
                );
            }
        }
        if !self.schema_failures.is_empty() {
            eprintln!("SCHEMA FAILURES (malformed audit file structure):");
            for f in &self.schema_failures {
                eprintln!("  {}:{}: {}", f.audit_path.display(), f.line, f.reason);
            }
        }
        if !self.freshness_failures.is_empty() {
            eprintln!("FRESHNESS FAILURES (last_walked malformed or absent):");
            for f in &self.freshness_failures {
                eprintln!("  {}: {}", f.audit_path.display(), f.reason);
            }
        }
    }
}

/// One mapping row whose catalog row ID does not resolve.
#[derive(Debug, Clone)]
pub struct UnresolvedMapping {
    pub audit_path: PathBuf,
    pub line: usize,
    pub row_id: String,
}

/// One schema violation.
#[derive(Debug, Clone)]
pub struct SchemaFailure {
    pub audit_path: PathBuf,
    pub line: usize,
    pub reason: String,
}

/// One freshness violation (malformed or missing `last_walked`).
#[derive(Debug, Clone)]
pub struct FreshnessFailure {
    pub audit_path: PathBuf,
    pub reason: String,
}

/// One decoded audit file — frontmatter + mapping rows.
#[derive(Debug, Clone)]
pub struct AuditFile {
    pub path: PathBuf,
    pub frontmatter: Frontmatter,
    pub rows: Vec<AuditRow>,
}

/// Parsed YAML-ish frontmatter of an audit file.
#[derive(Debug, Clone, Default)]
pub struct Frontmatter {
    pub section: Option<String>,
    pub title: Option<String>,
    pub canonical_spec_sources: Vec<String>,
    pub last_walked: Option<String>,
    pub walked_by: Option<String>,
}

/// One row in the sequence-to-catalog mapping table.
#[derive(Debug, Clone)]
pub struct AuditRow {
    pub line: usize,
    pub sequence: String,
    pub spec_source: String,
    pub catalog_row_id: String,
    pub decision: Decision,
}

/// The Decision column's parsed form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// `mapped` — the sequence targets a catalog row ID.
    Mapped,
    /// `not-targeted: <rationale>` — intentionally excluded.
    NotTargeted { rationale: String },
    /// Some other value the parser couldn't interpret — captured so
    /// the schema check can report it.
    Unknown(String),
}

/// Errors from the audit-file loader (IO on inputs only).
#[derive(Debug)]
pub enum AuditFilesError {
    Io(String),
    Catalog(String),
}

impl core::fmt::Display for AuditFilesError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "audit-files I/O error: {msg}"),
            Self::Catalog(msg) => write!(f, "catalog load error: {msg}"),
        }
    }
}

impl std::error::Error for AuditFilesError {}

/// Run the audit-file lint over a spec-conformance directory.
///
/// `plan_root` is `plans/spec-conformance/`. The checker reads
/// `audits/*.md`, `catalog/*.md`, and `00-overview.md` under that root.
///
/// # Errors
///
/// Returns an error only for IO failures on the inputs; soft failures
/// (malformed audit files, unresolved mappings, missing files) populate
/// [`AuditFilesReport`] rather than short-circuiting.
pub fn check_audit_files(plan_root: &Path) -> Result<AuditFilesReport, AuditFilesError> {
    let audits_dir = plan_root.join("audits");
    let catalog_dir = plan_root.join("catalog");
    let overview_md = plan_root.join("00-overview.md");

    let catalog_row_ids = load_catalog_row_ids(&catalog_dir)?;
    let audit_files = load_audit_files(&audits_dir)?;
    let in_progress_sections = load_in_progress_sections(&overview_md)?;

    let mut report = AuditFilesReport::default();
    check_existence(&in_progress_sections, &audit_files, &mut report);

    for audit in &audit_files {
        check_schema_conformance(audit, &mut report);
        check_mapping_resolution(audit, &catalog_row_ids, &mut report);
        check_freshness(audit, &mut report);
    }

    Ok(report)
}

fn load_catalog_row_ids(catalog_dir: &Path) -> Result<HashSet<String>, AuditFilesError> {
    let rows = load_all_catalog_rows(catalog_dir).map_err(AuditFilesError::Catalog)?;
    // Normalize IDs by stripping surrounding backticks — the catalog
    // schema allows either `` `ID` `` or bare `ID` in the first column
    // (both forms appear across committed catalog files). Audit files
    // likewise cite either form; the lint compares normalized IDs so
    // both forms resolve equally.
    Ok(rows
        .into_iter()
        .map(|r| r.id.trim().trim_matches('`').to_string())
        .collect())
}

/// Return every audit file under `audits/` except `README.md`.
fn load_audit_files(audits_dir: &Path) -> Result<Vec<AuditFile>, AuditFilesError> {
    let mut out = Vec::new();
    let entries = std::fs::read_dir(audits_dir)
        .map_err(|e| AuditFilesError::Io(format!("read_dir {}: {e}", audits_dir.display())))?;
    for entry in entries {
        let entry = entry.map_err(|e| AuditFilesError::Io(format!("dir entry in audits/: {e}")))?;
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "md") {
            continue;
        }
        let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if filename == "README.md" {
            continue;
        }
        out.push(parse_audit_file(&path)?);
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// Parse the `## Quick Reference` table and return the set of section
/// IDs whose status column reads `in-progress` (case-insensitive,
/// whitespace/hyphen normalized — so `In Progress`, `in progress`, and
/// `in-progress` all match).
fn load_in_progress_sections(overview_md: &Path) -> Result<HashSet<String>, AuditFilesError> {
    let content = std::fs::read_to_string(overview_md)
        .map_err(|e| AuditFilesError::Io(format!("read {}: {e}", overview_md.display())))?;

    let mut out = HashSet::new();
    let mut saw_header = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            saw_header = false;
            continue;
        }
        let cells = split_table_row(trimmed);
        if is_separator_row(&cells) {
            continue;
        }
        if !saw_header {
            if cells.len() >= 4
                && cells[0].trim().eq_ignore_ascii_case("id")
                && cells[3].trim().eq_ignore_ascii_case("status")
            {
                saw_header = true;
            }
            continue;
        }
        if cells.len() >= 4 {
            let id = cells[0].trim().to_string();
            let status = cells[3].trim().to_lowercase().replace(['-', ' '], "");
            if status == "inprogress" {
                out.insert(id);
            }
        }
    }
    Ok(out)
}

fn check_existence(
    in_progress: &HashSet<String>,
    audits: &[AuditFile],
    report: &mut AuditFilesReport,
) {
    let present: HashSet<String> = audits
        .iter()
        .filter_map(|a| a.frontmatter.section.clone())
        .collect();
    let mut missing: Vec<String> = in_progress
        .iter()
        .filter(|id| !present.contains(*id))
        .cloned()
        .collect();
    missing.sort();
    report.missing_audit_files.extend(missing);
}

fn check_schema_conformance(audit: &AuditFile, report: &mut AuditFilesReport) {
    if audit.frontmatter.section.is_none() {
        report.schema_failures.push(SchemaFailure {
            audit_path: audit.path.clone(),
            line: 1,
            reason: "frontmatter missing `section:` field".to_string(),
        });
    }
    if audit.frontmatter.title.is_none() {
        report.schema_failures.push(SchemaFailure {
            audit_path: audit.path.clone(),
            line: 1,
            reason: "frontmatter missing `title:` field".to_string(),
        });
    }

    for row in &audit.rows {
        match &row.decision {
            Decision::Mapped => {
                if row.catalog_row_id.trim().is_empty() {
                    report.schema_failures.push(SchemaFailure {
                        audit_path: audit.path.clone(),
                        line: row.line,
                        reason: "`mapped` decision with empty catalog row ID cell".to_string(),
                    });
                }
            }
            Decision::NotTargeted { rationale } => {
                if rationale.trim().is_empty() {
                    report.schema_failures.push(SchemaFailure {
                        audit_path: audit.path.clone(),
                        line: row.line,
                        reason: "`not-targeted` decision with empty rationale".to_string(),
                    });
                }
            }
            Decision::Unknown(raw) => {
                report.schema_failures.push(SchemaFailure {
                    audit_path: audit.path.clone(),
                    line: row.line,
                    reason: format!("unrecognized Decision value: {raw:?}"),
                });
            }
        }
    }
}

fn check_mapping_resolution(
    audit: &AuditFile,
    catalog_row_ids: &HashSet<String>,
    report: &mut AuditFilesReport,
) {
    for row in &audit.rows {
        if !matches!(row.decision, Decision::Mapped) {
            continue;
        }
        let id = extract_row_id(&row.catalog_row_id);
        if id.is_empty() {
            continue;
        }
        if !catalog_row_ids.contains(&id) {
            report.unresolved_mappings.push(UnresolvedMapping {
                audit_path: audit.path.clone(),
                line: row.line,
                row_id: id,
            });
        }
    }
}

fn check_freshness(audit: &AuditFile, report: &mut AuditFilesReport) {
    match audit.frontmatter.last_walked.as_deref() {
        None => report.freshness_failures.push(FreshnessFailure {
            audit_path: audit.path.clone(),
            reason: "frontmatter missing `last_walked:` field".to_string(),
        }),
        Some(val) => {
            let v = val.trim();
            if v == "null" {
                return;
            }
            if !is_valid_date(v) {
                report.freshness_failures.push(FreshnessFailure {
                    audit_path: audit.path.clone(),
                    reason: format!("`last_walked` does not parse as YYYY-MM-DD: {v:?}"),
                });
            }
        }
    }
}

fn is_valid_date(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.len() != 10 {
        return false;
    }
    if bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    bytes
        .iter()
        .enumerate()
        .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
}

#[cfg(test)]
mod tests;
