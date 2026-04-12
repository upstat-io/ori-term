//! Three-way reconciliation of bottom-up, top-down, and capture tuple
//! sets (Section 01.8).
//!
//! Compares dispatch-extracted tuples (bottom-up from source code),
//! spec-sourced catalog tuples (top-down from 01.7), and real-app
//! capture tuples to categorize every sequence as `reconciled`,
//! `de-facto`, `missing`, or `capture-only`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::capture_extract::extract_capture_tuples;
use super::dispatch_extract::{extract_dispatch_tuples, extract_namedprivatemode_tuples};
use super::parser::parse_catalog_markdown;
use super::row::Row;
use super::tuple::{Category, Tuple, canonical_tuple};

/// Tuple signature for coverage comparison: (`category`, `sorted
/// intermediates`, `final_byte`). Params are excluded because they
/// differ between catalog, dispatch, and capture canonical forms.
pub type TupleSig = (String, Vec<u8>, String);

/// Normalize a [`Tuple`] to its comparison signature.
pub fn tuple_signature(tuple: &Tuple) -> TupleSig {
    let final_byte = match tuple.category {
        Category::Osc | Category::Dcs => {
            if tuple.final_byte == "ST" {
                "BEL".to_string()
            } else {
                tuple.final_byte.clone()
            }
        }
        _ => tuple.final_byte.clone(),
    };
    (
        format!("{}", tuple.category),
        tuple.intermediates.clone(),
        final_byte,
    )
}

/// A catalog row paired with its parsed tuple signature.
pub struct CatalogEntry {
    pub row_id: String,
    pub source_file: String,
    pub spec_source: String,
    pub implementation: String,
    pub sig: TupleSig,
}

/// Categorized reconciliation result.
pub struct ReconciliationReport {
    pub bottom_up_count: usize,
    pub top_down_count: usize,
    pub capture_count: usize,
    pub reconciled: Vec<CatalogEntry>,
    pub de_facto: Vec<TupleSig>,
    pub missing: Vec<CatalogEntry>,
    pub capture_only: Vec<TupleSig>,
}

/// Load all catalog rows from a directory (excluding README.md and
/// `_`-prefixed files).
pub fn load_all_catalog_rows(catalog_dir: &Path) -> Result<Vec<Row>, String> {
    let entries = std::fs::read_dir(catalog_dir)
        .map_err(|e| format!("read_dir {}: {e}", catalog_dir.display()))?;
    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("dir entry: {e}"))?;
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "md") {
            continue;
        }
        let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if filename == "README.md" || filename.starts_with('_') {
            continue;
        }
        paths.push(path);
    }
    paths.sort();
    let mut all_rows = Vec::new();
    for path in paths {
        let rows = parse_catalog_markdown(&path).map_err(|e| format!("{e}"))?;
        all_rows.extend(rows);
    }
    Ok(all_rows)
}

/// Build the set of all catalog tuple signatures.
pub fn build_catalog_signature_set(catalog_dir: &Path) -> Result<BTreeSet<TupleSig>, String> {
    let rows = load_all_catalog_rows(catalog_dir)?;
    let mut sigs = BTreeSet::new();
    for row in &rows {
        if let Some(tuple) = canonical_tuple(&row.sequence) {
            sigs.insert(tuple_signature(&tuple));
        }
    }
    Ok(sigs)
}

/// Filter catalog rows to those with a real spec source (not MISSING,
/// not wezterm-sourced, not `—` placeholder). These form the "top-down"
/// tuple set.
fn is_spec_sourced(row: &Row) -> bool {
    let src = row.spec_source.to_lowercase();
    !src.contains("missing")
        && !src.contains("wezterm")
        && !row.spec_source.starts_with('—')
        && !row.spec_source.starts_with("—") // em dash (UTF-8)
}

/// Run the full three-way reconciliation.
pub fn reconcile(
    workspace_root: &Path,
    catalog_dir: &Path,
    captures_dir: &Path,
) -> Result<ReconciliationReport, String> {
    // 1. Bottom-up: dispatch + named private modes → signatures.
    let dispatch = extract_dispatch_tuples(workspace_root).map_err(|e| format!("dispatch: {e}"))?;
    let modes = extract_namedprivatemode_tuples(workspace_root)
        .map_err(|e| format!("named-private-modes: {e}"))?;
    let bottom_up_sigs: BTreeSet<TupleSig> = dispatch
        .iter()
        .chain(modes.iter())
        .map(tuple_signature)
        .collect();

    // 2. Top-down: filtered catalog rows → entries with signatures.
    let all_rows = load_all_catalog_rows(catalog_dir)?;
    let top_down_entries: Vec<CatalogEntry> = all_rows
        .iter()
        .filter(|r| is_spec_sourced(r))
        .filter_map(|r| {
            canonical_tuple(&r.sequence).map(|t| CatalogEntry {
                row_id: r.id.clone(),
                source_file: r.source_file.clone(),
                spec_source: r.spec_source.clone(),
                implementation: r.implementation.clone(),
                sig: tuple_signature(&t),
            })
        })
        .collect();
    let top_down_sigs: BTreeSet<TupleSig> =
        top_down_entries.iter().map(|e| e.sig.clone()).collect();

    // 3. Captures: all .cap files → signatures.
    let mut capture_sigs: BTreeSet<TupleSig> = BTreeSet::new();
    if captures_dir.exists() {
        let entries = std::fs::read_dir(captures_dir)
            .map_err(|e| format!("read_dir {}: {e}", captures_dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("dir entry: {e}"))?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "cap") {
                let pairs = extract_capture_tuples(&path)
                    .map_err(|e| format!("{}: {e}", path.display()))?;
                for (tuple, _) in pairs {
                    capture_sigs.insert(tuple_signature(&tuple));
                }
            }
        }
    }

    // 4. Categorize.
    let bottom_up_or_captures: BTreeSet<TupleSig> =
        bottom_up_sigs.union(&capture_sigs).cloned().collect();

    let reconciled: Vec<CatalogEntry> = top_down_entries
        .into_iter()
        .filter(|e| bottom_up_sigs.contains(&e.sig) || capture_sigs.contains(&e.sig))
        .collect();

    // Re-derive top-down entries for missing (those not in bottom-up).
    let all_rows2 = load_all_catalog_rows(catalog_dir)?;
    let missing: Vec<CatalogEntry> = all_rows2
        .iter()
        .filter(|r| is_spec_sourced(r))
        .filter_map(|r| {
            canonical_tuple(&r.sequence).map(|t| CatalogEntry {
                row_id: r.id.clone(),
                source_file: r.source_file.clone(),
                spec_source: r.spec_source.clone(),
                implementation: r.implementation.clone(),
                sig: tuple_signature(&t),
            })
        })
        .filter(|e| !bottom_up_sigs.contains(&e.sig))
        .collect();

    let de_facto: Vec<TupleSig> = bottom_up_or_captures
        .iter()
        .filter(|sig| !top_down_sigs.contains(sig))
        .cloned()
        .collect();

    let capture_only: Vec<TupleSig> = capture_sigs
        .iter()
        .filter(|sig| !bottom_up_sigs.contains(sig) && !top_down_sigs.contains(sig))
        .cloned()
        .collect();

    Ok(ReconciliationReport {
        bottom_up_count: bottom_up_sigs.len(),
        top_down_count: top_down_sigs.len(),
        capture_count: capture_sigs.len(),
        reconciled,
        de_facto,
        missing,
        capture_only,
    })
}

/// Build a row-ID lookup keyed by tuple signature from all catalog
/// rows (not just spec-sourced ones).
pub fn build_sig_to_row_ids(catalog_dir: &Path) -> Result<BTreeMap<TupleSig, Vec<String>>, String> {
    let rows = load_all_catalog_rows(catalog_dir)?;
    let mut map: BTreeMap<TupleSig, Vec<String>> = BTreeMap::new();
    for row in &rows {
        if let Some(tuple) = canonical_tuple(&row.sequence) {
            map.entry(tuple_signature(&tuple))
                .or_default()
                .push(row.id.clone());
        }
    }
    Ok(map)
}

/// Format the reconciliation report as markdown.
pub fn format_report(report: &ReconciliationReport) -> String {
    use std::fmt::Write;

    let mut out = String::new();
    out.push_str("# Section 01 Reconciliation Report\n\n");
    out.push_str("schema_version: \"0.1-provisional\"\n");
    let _ = writeln!(out, "generated: {}\n", chrono_date());

    out.push_str("## Summary\n\n");
    let _ = writeln!(out, "- Bottom-up tuple count: {}", report.bottom_up_count);
    let _ = writeln!(out, "- Top-down tuple count: {}", report.top_down_count);
    let _ = writeln!(out, "- Capture tuple count: {}", report.capture_count);
    let _ = writeln!(
        out,
        "- Reconciled (in spec + code/captures): {}",
        report.reconciled.len()
    );
    let _ = writeln!(
        out,
        "- De-facto (in code, not in spec): {}",
        report.de_facto.len()
    );
    let _ = writeln!(
        out,
        "- MISSING (in spec, not in code): {}",
        report.missing.len()
    );
    let _ = writeln!(
        out,
        "- Capture-only (in captures, not in code or spec): {}\n",
        report.capture_only.len()
    );

    out.push_str("## De-facto rows (in code/captures, no spec source)\n\n");
    out.push_str("| Signature | Reason |\n|---|---|\n");
    if report.de_facto.is_empty() {
        out.push_str("| — | No de-facto-only tuples |\n");
    } else {
        for sig in &report.de_facto {
            let _ = writeln!(
                out,
                "| ({}, {:?}, {}) | In dispatch but no primary spec row |",
                sig.0, sig.1, sig.2
            );
        }
    }
    out.push('\n');

    out.push_str("## MISSING rows (in spec, not dispatched)\n\n");
    out.push_str("| Row ID | Catalog file | Spec source | Owner |\n|---|---|---|---|\n");
    if report.missing.is_empty() {
        out.push_str("| — | — | — | — |\n");
    } else {
        for entry in &report.missing {
            let owner = if entry.implementation.contains("Section") {
                entry
                    .implementation
                    .split("Section")
                    .nth(1)
                    .unwrap_or("—")
                    .trim()
                    .split(|c: char| !c.is_ascii_digit())
                    .next()
                    .unwrap_or("—")
            } else {
                "—"
            };
            let _ = writeln!(
                out,
                "| {} | {} | {} | Section {} |",
                entry.row_id, entry.source_file, entry.spec_source, owner
            );
        }
    }
    out.push('\n');

    out.push_str("## Capture-only rows (in captures, not in code or spec)\n\n");
    out.push_str("| Signature | Reason |\n|---|---|\n");
    if report.capture_only.is_empty() {
        out.push_str("| — | No capture-only tuples |\n");
    } else {
        for sig in &report.capture_only {
            let _ = writeln!(
                out,
                "| ({}, {:?}, {}) | Seen in captures only |",
                sig.0, sig.1, sig.2
            );
        }
    }
    out.push('\n');

    out
}

fn chrono_date() -> String {
    // Simple date without chrono dependency — parse from system.
    let output = std::process::Command::new("date").arg("+%Y-%m-%d").output();
    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => "YYYY-MM-DD".to_string(),
    }
}
