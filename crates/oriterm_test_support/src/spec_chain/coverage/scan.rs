//! Citation scanner — walks test directories for catalog row ID references.
//!
//! Recognizes these citation forms in `.rs` files:
//! - `// Catalog row: <ID>` (line comment)
//! - `//! Catalog row: <ID>` (inner doc comment)
//! - `/// Catalog row: <ID>` (outer doc comment)
//! - `catalog_row_id: "<ID>"` (const field in `SpecScenario`)
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
/// extracting citations via string matching.
pub fn scan_test_citations(dirs: &[PathBuf]) -> Result<Vec<Citation>, CoverageError> {
    let mut citations = Vec::new();
    for dir in dirs {
        if dir.exists() {
            walk_dir_recursive(dir, &mut citations)?;
        }
    }
    Ok(citations)
}

fn walk_dir_recursive(dir: &Path, citations: &mut Vec<Citation>) -> Result<(), CoverageError> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| CoverageError::Scan(format!("failed to read {}: {e}", dir.display())))?;

    for entry in entries {
        let entry = entry.map_err(|e| CoverageError::Scan(format!("dir entry error: {e}")))?;
        let path = entry.path();

        if path.is_dir() {
            walk_dir_recursive(&path, citations)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            scan_file(&path, citations)?;
        } else {
            // Non-.rs files are ignored.
        }
    }
    Ok(())
}

fn scan_file(path: &Path, citations: &mut Vec<Citation>) -> Result<(), CoverageError> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| CoverageError::Scan(format!("failed to read {}: {e}", path.display())))?;

    for line in content.lines() {
        let trimmed = line.trim();

        // Pattern 1: comment citation — `// Catalog row: ID`
        // Also matches `//! Catalog row: ID` and `/// Catalog row: ID`
        if let Some(rest) = trimmed
            .strip_prefix("// Catalog row: ")
            .or_else(|| trimmed.strip_prefix("//! Catalog row: "))
            .or_else(|| trimmed.strip_prefix("/// Catalog row: "))
        {
            let id = rest.trim();
            if !id.is_empty() {
                citations.push(Citation {
                    row_id: id.to_string(),
                    file_path: path.to_path_buf(),
                });
            }
        }

        // Pattern 2: const field citation — `catalog_row_id: "ID"`
        if let Some(id) = extract_const_field_id(trimmed) {
            citations.push(Citation {
                row_id: id,
                file_path: path.to_path_buf(),
            });
        }
    }
    Ok(())
}

/// Extract a catalog row ID from a `catalog_row_id: "ID"` pattern.
///
/// Uses `split` instead of byte indexing to avoid `string_slice` clippy
/// warnings and handle multi-byte UTF-8 safely.
fn extract_const_field_id(line: &str) -> Option<String> {
    let after = line.split("catalog_row_id:").nth(1)?;
    let after_first_quote = after.split('"').nth(1)?;
    let id = after_first_quote.split('"').next()?;
    if id.is_empty() {
        return None;
    }
    Some(id.to_string())
}
