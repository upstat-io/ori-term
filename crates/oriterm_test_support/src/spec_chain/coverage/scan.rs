//! Citation scanner — walks test directories for catalog row ID references.
//!
//! Recognizes these citation forms in `.rs` files:
//! - `// Catalog row: <ID>` (line comment)
//! - `//! Catalog row: <ID>` (inner doc comment)
//! - `/// Catalog row: <ID>` (outer doc comment)
//! - `Catalog rows: <ID>, <ID>, …` (plural form — comma-separated list
//!   after any of the three comment prefixes). Each trimmed non-empty
//!   piece becomes its own citation. The plural form exists so a test
//!   file that covers multiple catalog rows can cite them on one doc
//!   line without a silent-miss when the natural-English plural is used.
//! - Multi-line continuation: when a citation line ends with a trailing
//!   comma, subsequent lines that start with `//`, `//!`, or `///` are
//!   appended until a line without a trailing comma is consumed. This
//!   supports the natural wrap when a long list of row IDs doesn't fit
//!   on one line.
//! - `catalog_row_id: "<ID>"` (const field in `SpecScenario`)
//!
//! Each piece is normalized before becoming a citation:
//! - Trailing periods dropped (`OSC-2.` → `OSC-2`)
//! - Trailing parenthetical qualifier dropped (`SIXEL-REPEAT (§12.4 GPU-apex)` → `SIXEL-REPEAT`)
//! - Trailing prose after a sentence-boundary `. ` dropped (`OSC-4-QUERY. Apex: …` → `OSC-4-QUERY`)
//! - Surrounding backticks dropped (`` `SIXEL-BG-NoChange` `` → `SIXEL-BG-NoChange`)
//!
//! Pieces that still contain whitespace after normalization are silently
//! dropped — they are prose (author error), not row IDs, and cannot match
//! any catalog row. Validation rejects anything that isn't a bare
//! identifier (ASCII alphanumeric + `-_`).
//!
//! Also scans `src/` directories because visual `spec_chain` tests live
//! under `oriterm/src/gpu/visual_regression/spec_chain/` as unit tests.

use std::path::{Path, PathBuf};

use super::CoverageError;

/// A citation of a catalog row ID found in a test file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citation {
    /// The catalog row ID (e.g. `"ECMA48-CUP"`).
    pub row_id: String,
    /// Path to the file containing the citation.
    pub file_path: PathBuf,
}

/// Scan test directories for catalog row ID citations.
///
/// Recursively walks each directory, reading every `.rs` file and
/// extracting citations via string matching. Directories in
/// `exclude` are skipped (prevents the scanner from reading its own
/// source code as if it were test citations).
pub fn scan_test_citations(
    dirs: &[PathBuf],
    exclude: &[PathBuf],
) -> Result<Vec<Citation>, CoverageError> {
    let mut citations = Vec::new();
    for dir in dirs {
        if dir.exists() {
            walk_dir_recursive(dir, exclude, &mut citations)?;
        }
    }
    Ok(citations)
}

fn walk_dir_recursive(
    dir: &Path,
    exclude: &[PathBuf],
    citations: &mut Vec<Citation>,
) -> Result<(), CoverageError> {
    if exclude.iter().any(|e| dir.starts_with(e)) {
        return Ok(());
    }
    let entries = std::fs::read_dir(dir)
        .map_err(|e| CoverageError::Scan(format!("failed to read {}: {e}", dir.display())))?;

    for entry in entries {
        let entry = entry.map_err(|e| CoverageError::Scan(format!("dir entry error: {e}")))?;
        let path = entry.path();

        if path.is_dir() {
            walk_dir_recursive(&path, exclude, citations)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            scan_file(&path, citations)?;
        } else {
            // Non-.rs files are ignored.
        }
    }
    Ok(())
}

const CITATION_PREFIXES: &[&str] = &[
    "// Catalog row: ",
    "//! Catalog row: ",
    "/// Catalog row: ",
    "// Catalog rows: ",
    "//! Catalog rows: ",
    "/// Catalog rows: ",
];

fn scan_file(path: &Path, citations: &mut Vec<Citation>) -> Result<(), CoverageError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| CoverageError::Scan(format!("failed to read {}: {e}", path.display())))?;

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();

        // Const-field citation — `catalog_row_id: "ID"`.
        if let Some(id) = extract_const_field_id(trimmed) {
            citations.push(Citation {
                row_id: id,
                file_path: path.to_path_buf(),
            });
        }

        // Comment citation — `// Catalog row[s]: ...` with optional
        // multi-line continuation when a line ends in a trailing comma.
        if let Some(rest) = find_citation_prefix(trimmed) {
            let mut collected = rest.to_string();
            while trailing_comma(&collected) {
                let next_idx = i + 1;
                if next_idx >= lines.len() {
                    break;
                }
                let trimmed_next = lines[next_idx].trim();
                let Some(next_text) = strip_comment_marker(trimmed_next) else {
                    // Non-comment line terminates the continuation without
                    // consuming it — the outer loop will re-process line
                    // `next_idx` normally.
                    break;
                };
                i = next_idx;
                collected.push(' ');
                collected.push_str(next_text);
            }

            for piece in collected.split(',') {
                if let Some(id) = normalize_row_id(piece) {
                    citations.push(Citation {
                        row_id: id,
                        file_path: path.to_path_buf(),
                    });
                }
            }
        }

        i += 1;
    }
    Ok(())
}

/// Match any of the six citation prefixes and return the text after the
/// prefix.
fn find_citation_prefix(line: &str) -> Option<&str> {
    CITATION_PREFIXES.iter().find_map(|p| line.strip_prefix(p))
}

/// Check whether a citation fragment ends with a trailing comma (allowing
/// trailing whitespace). Signals that the list continues on the next
/// comment line.
fn trailing_comma(s: &str) -> bool {
    s.trim_end().ends_with(',')
}

/// Strip the comment marker (`//`, `///`, or `//!`) from a line, returning
/// the text after the marker with leading whitespace trimmed. Returns
/// `None` if the line is not a comment.
///
/// Order matters: `///` and `//!` must be tried before `//` because `//`
/// is a prefix of both.
fn strip_comment_marker(line: &str) -> Option<&str> {
    let rest = line
        .strip_prefix("///")
        .or_else(|| line.strip_prefix("//!"))
        .or_else(|| line.strip_prefix("//"))?;
    Some(rest.trim_start())
}

/// Normalize a split citation piece into a bare row ID, or return `None`
/// if the piece is not a valid identifier after normalization.
///
/// Normalization order:
/// 1. Trim surrounding whitespace.
/// 2. Drop everything after the first sentence-boundary `. ` (trailing
///    prose like `OSC-4-QUERY. Apex: state-snapshot` → `OSC-4-QUERY`).
/// 3. Drop trailing parenthetical qualifier (` (§12.4 GPU-apex)`).
/// 4. Drop trailing periods.
/// 5. Trim surrounding backticks.
/// 6. Trim surrounding whitespace again.
///
/// Validation: the result must be non-empty and consist only of ASCII
/// alphanumeric + `-` / `_`. Prose (anything containing whitespace or
/// other characters) is silently dropped; the author used the citation
/// prefix on non-ID text, which cannot match a catalog row.
fn normalize_row_id(raw: &str) -> Option<String> {
    let s = raw.trim();
    // Sentence-boundary prose drop: `ID. Extra prose.` → `ID`.
    // `split_once` returns `Option<(&str, &str)>` with valid UTF-8
    // boundaries, sidestepping clippy's `string_slice` lint on raw byte
    // indexing.
    let s = s.split_once(". ").map_or(s, |(before, _)| before);
    // Trailing parenthetical qualifier drop.
    let s = s.split_once(" (").map_or(s, |(before, _)| before);
    // Trailing period(s) drop.
    let s = s.trim_end_matches('.');
    // Surrounding backticks.
    let s = s.trim_matches('`').trim();

    if s.is_empty() {
        return None;
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }
    Some(s.to_string())
}

/// Extract a catalog row ID from a `catalog_row_id: "ID"` pattern.
///
/// Anchored to lines that START with `catalog_row_id:` (after trimming)
/// to avoid matching doc-comment examples or code that mentions the
/// pattern inline.
fn extract_const_field_id(line: &str) -> Option<String> {
    if !line.starts_with("catalog_row_id:") {
        return None;
    }
    let after = line.split("catalog_row_id:").nth(1)?;
    let after_first_quote = after.split('"').nth(1)?;
    let id = after_first_quote.split('"').next()?;
    if id.is_empty() {
        return None;
    }
    Some(id.to_string())
}

/// Print a per-line trace of every citation prefix encountered in `path`.
///
/// For each matching line, shows what was matched, what was normalized,
/// and for each piece whether it was accepted as a catalog-row ID or
/// dropped (with the drop reason). Used by
/// `spec-coverage-report --explain <file>` to debug silently-dropped
/// citations without reading the normalizer source.
///
/// Writes directly to stdout. Returns `Err` only on filesystem failures.
pub fn explain_file(path: &Path) -> Result<(), CoverageError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| CoverageError::Scan(format!("failed to read {}: {e}", path.display())))?;

    println!("Explaining citations in: {}", path.display());
    let mut accepted = 0usize;
    let mut dropped = 0usize;

    for (lineno, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        if let Some(id) = extract_const_field_id(trimmed) {
            println!("line {}: const-field citation", lineno + 1);
            println!("  {line}");
            println!("  ✓ ACCEPTED as citation: {id}");
            accepted += 1;
            continue;
        }

        let Some(rest) = find_citation_prefix(trimmed) else {
            continue;
        };

        println!("line {}: comment citation", lineno + 1);
        println!("  {line}");
        println!("  collected (text after prefix): {rest:?}");

        for (piece_idx, piece) in rest.split(',').enumerate() {
            println!("  piece {}: {piece:?}", piece_idx + 1);
            let steps = trace_normalize(piece);
            for (step_name, step_val) in &steps.steps {
                println!("    after {step_name:<24}: {step_val:?}");
            }
            match &steps.outcome {
                NormalizeOutcome::Accepted(id) => {
                    println!("    ✓ ACCEPTED as citation: {id}");
                    accepted += 1;
                }
                NormalizeOutcome::DroppedEmpty => {
                    println!(
                        "    ✗ DROPPED: empty after normalization \
                         (hint: citation prefix used on non-ID text)"
                    );
                    dropped += 1;
                }
                NormalizeOutcome::DroppedInvalidChars(final_s) => {
                    println!(
                        "    ✗ DROPPED: final text {final_s:?} contains \
                         whitespace or non-identifier chars \
                         (valid chars: ASCII alphanumeric, '-', '_')"
                    );
                    println!(
                        "      hint: catalog IDs are bare identifiers — \
                         put extra prose in `(parenthetical)` form or on a \
                         separate line. Em-dashes and nested backticks in \
                         the citation tail are the common cause."
                    );
                    dropped += 1;
                }
            }
        }
    }

    println!();
    println!("Summary: {accepted} accepted, {dropped} dropped");
    if dropped > 0 {
        println!(
            "Dropped citations are invisible to the coverage gate. Fix \
             the citation format or run with --check to confirm no \
             FALSE VERIFIED rows result from this file."
        );
    }
    Ok(())
}

/// Output of tracing `normalize_row_id` against a single piece.
struct NormalizeTrace {
    /// Per-step intermediate values: `(step_name, value_after_step)`.
    steps: Vec<(&'static str, String)>,
    outcome: NormalizeOutcome,
}

enum NormalizeOutcome {
    Accepted(String),
    DroppedEmpty,
    /// Dropped because final text still contains whitespace or
    /// non-identifier chars. Carries the final text for display.
    DroppedInvalidChars(String),
}

/// Mirror `normalize_row_id`'s logic while collecting per-step
/// intermediate values. Must stay in lockstep with `normalize_row_id` —
/// if the normalizer gains a step, add a matching entry here.
fn trace_normalize(raw: &str) -> NormalizeTrace {
    let mut steps = Vec::with_capacity(5);
    let s = raw.trim();
    steps.push(("trim", s.to_string()));

    let s = s.split_once(". ").map_or(s, |(before, _)| before);
    steps.push(("sentence-boundary drop", s.to_string()));

    let s = s.split_once(" (").map_or(s, |(before, _)| before);
    steps.push(("paren qualifier drop", s.to_string()));

    let s = s.trim_end_matches('.');
    steps.push(("trailing-period trim", s.to_string()));

    let s = s.trim_matches('`').trim();
    steps.push(("backtick trim", s.to_string()));

    let outcome = if s.is_empty() {
        NormalizeOutcome::DroppedEmpty
    } else if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        NormalizeOutcome::DroppedInvalidChars(s.to_string())
    } else {
        NormalizeOutcome::Accepted(s.to_string())
    };
    NormalizeTrace { steps, outcome }
}
