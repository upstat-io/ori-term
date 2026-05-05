//! Shared font-directory walker — single source of truth for "walk font dirs".
//!
//! Used by both Linux/macOS filename-index builders AND the family enumeration
//! pipeline. Closes the algorithmic-duplication LEAK between
//! `linux.rs::index_font_dir` and `macos.rs::index_font_dir` (per
//! §Algorithmic DRY).

#![cfg(any(target_os = "linux", target_os = "macos"))]

use std::path::{Path, PathBuf};

/// Recursively walk a list of font directories, calling `visitor` for each
/// regular file encountered. Skips non-readable directories silently — a
/// missing or permission-denied font dir is not an error, just an empty walk.
pub(crate) fn walk_font_dirs(roots: &[PathBuf], visitor: &mut dyn FnMut(&Path)) {
    for root in roots {
        walk_dir(root, visitor);
    }
}

fn walk_dir(dir: &Path, visitor: &mut dyn FnMut(&Path)) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_dir(&path, visitor);
        } else {
            visitor(&path);
        }
    }
}
