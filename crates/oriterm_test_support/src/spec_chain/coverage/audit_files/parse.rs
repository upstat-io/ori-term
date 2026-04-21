//! Audit-file parsing — YAML-ish frontmatter + 4-column mapping table.
//!
//! The parser is deliberately minimal: no external YAML dependency, no
//! generic markdown AST. It recognizes the exact shape documented in
//! `plans/spec-conformance/audits/README.md §Artifact format` and
//! nothing else — anything malformed surfaces as a `Decision::Unknown`
//! or a missing frontmatter field, which the lint layer flags.

use std::path::Path;

use super::{AuditFile, AuditFilesError, AuditRow, Decision, Frontmatter};

/// Parse one audit markdown file — frontmatter plus the first 4-column
/// table under `## Sequence-to-catalog mapping`.
///
/// # Errors
///
/// Returns [`AuditFilesError::Io`] when the file cannot be read.
/// Structural problems (bad frontmatter, malformed rows) surface via
/// empty / `Unknown` fields on the returned [`AuditFile`] — the lint
/// layer reports those, not this parser.
pub fn parse_audit_file(path: &Path) -> Result<AuditFile, AuditFilesError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| AuditFilesError::Io(format!("read {}: {e}", path.display())))?;

    let (frontmatter, body_start_line) = parse_frontmatter(&content);
    let rows = parse_mapping_rows(&content, body_start_line);

    Ok(AuditFile {
        path: path.to_path_buf(),
        frontmatter,
        rows,
    })
}

/// Parse a minimal YAML-ish frontmatter block between `---` markers.
///
/// Supports scalar keys (`key: value`) and one level of list values
/// (`key:` followed by `  - item` lines). Anything else is ignored.
/// Returns the parsed [`Frontmatter`] and the 1-based line number of
/// the first body line after the closing `---`.
pub(super) fn parse_frontmatter(content: &str) -> (Frontmatter, usize) {
    let mut fm = Frontmatter::default();
    let lines: Vec<&str> = content.lines().collect();

    if lines.first().map(|s| s.trim()) != Some("---") {
        return (fm, 1);
    }

    let mut current_list_key: Option<String> = None;
    let mut idx = 1;
    while idx < lines.len() {
        let raw = lines[idx];
        let trimmed = raw.trim_end();
        if trimmed.trim() == "---" {
            return (fm, idx + 2);
        }

        if let Some(item) = trimmed.strip_prefix("  - ")
            && let Some(key) = current_list_key.as_deref()
            && key == "canonical_spec_sources"
        {
            fm.canonical_spec_sources
                .push(strip_quotes(item).to_string());
            idx += 1;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("- ")
            && let Some(key) = current_list_key.as_deref()
            && key == "canonical_spec_sources"
        {
            fm.canonical_spec_sources
                .push(strip_quotes(rest).to_string());
            idx += 1;
            continue;
        }

        if let Some((key, value)) = trimmed.split_once(':') {
            let key = key.trim().to_string();
            let value = value.trim();
            current_list_key = Some(key.clone());
            if !value.is_empty() {
                let val = strip_quotes(value).to_string();
                match key.as_str() {
                    "section" => fm.section = Some(val),
                    "title" => fm.title = Some(val),
                    "last_walked" => fm.last_walked = Some(val),
                    "walked_by" => fm.walked_by = Some(val),
                    _ => {}
                }
                current_list_key = None;
            }
        }
        idx += 1;
    }

    (fm, lines.len() + 1)
}

fn strip_quotes(s: &str) -> &str {
    let s = s.trim();
    s.strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .or_else(|| {
            s.strip_prefix('\'')
                .and_then(|inner| inner.strip_suffix('\''))
        })
        .unwrap_or(s)
}

/// Parse the 4-column sequence-to-catalog mapping table.
///
/// Walks lines starting at `start_line` looking for the header
/// `| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |`.
/// Rows after the separator are yielded until the next non-table line.
/// Rows whose Sequence column is an implementer-placeholder (`_**TODO…`)
/// are skipped — they are intentional stubs, not schema failures.
pub(super) fn parse_mapping_rows(content: &str, start_line: usize) -> Vec<AuditRow> {
    let mut out = Vec::new();
    let mut saw_header = false;
    let lines: Vec<&str> = content.lines().collect();
    let start_idx = start_line.saturating_sub(1);

    for (idx, line) in lines.iter().enumerate().skip(start_idx) {
        let line_number = idx + 1;
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            if saw_header {
                break;
            }
            continue;
        }

        let cells = split_table_row(trimmed);
        if is_separator_row(&cells) {
            continue;
        }

        if !saw_header {
            if is_mapping_header(&cells) {
                saw_header = true;
            }
            continue;
        }

        if cells.len() != 4 {
            out.push(AuditRow {
                line: line_number,
                sequence: trimmed.to_string(),
                spec_source: String::new(),
                catalog_row_id: String::new(),
                decision: Decision::Unknown(format!("wrong column count: {}", cells.len())),
            });
            continue;
        }

        if cells[0].trim_start_matches(['*', '_']).starts_with("TODO") {
            continue;
        }

        let decision = parse_decision(&cells[3]);
        out.push(AuditRow {
            line: line_number,
            sequence: cells[0].clone(),
            spec_source: cells[1].clone(),
            catalog_row_id: cells[2].clone(),
            decision,
        });
    }

    out
}

fn parse_decision(cell: &str) -> Decision {
    let trimmed = cell.trim();
    if trimmed.starts_with("mapped") {
        Decision::Mapped
    } else if let Some(rest) = trimmed.strip_prefix("not-targeted") {
        let rationale = rest.trim_start_matches(':').trim().to_string();
        Decision::NotTargeted { rationale }
    } else {
        Decision::Unknown(trimmed.to_string())
    }
}

/// Extract the catalog row ID from a cell that may contain backticks or
/// trailing annotation. Accepts `` `DECRECT-DECCRA` ``, `DECRECT-DECCRA`,
/// and ignores a trailing `(NEW row …)` annotation that the audit stubs
/// use to document provenance.
pub(super) fn extract_row_id(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Some((_, rest)) = trimmed.split_once('`')
        && let Some((inside, _)) = rest.split_once('`')
    {
        return inside.trim().to_string();
    }
    trimmed
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_matches('`')
        .to_string()
}

pub(super) fn split_table_row(line: &str) -> Vec<String> {
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

pub(super) fn is_separator_row(cells: &[String]) -> bool {
    !cells.is_empty()
        && cells
            .iter()
            .all(|c| c.chars().all(|ch| ch == '-' || ch == ':'))
}

fn is_mapping_header(cells: &[String]) -> bool {
    if cells.len() != 4 {
        return false;
    }
    let normalized: Vec<String> = cells.iter().map(|c| c.trim().to_lowercase()).collect();
    normalized[0].starts_with("sequence")
        && normalized[1].starts_with("spec source")
        && normalized[2].starts_with("catalog row id")
        && normalized[3].starts_with("decision")
}
