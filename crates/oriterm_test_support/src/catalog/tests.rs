//! Module-level integration tests for the `catalog` module.
//!
//! Per-submodule unit tests live in their respective sibling
//! `tests.rs` files:
//! - `tuple/tests.rs` — canonical tuple parsing and Tuple equality
//! - `check/tests.rs` — `--check` pass finding detection
//! - `parser/tests.rs` — catalog markdown table parsing
//!
//! This file covers cross-module integration tests and the
//! `walk_catalog_files` function from `mod.rs`.

use super::walk_catalog_files;

#[test]
fn walk_catalog_files_returns_sorted_md_paths() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().join("catalog");
    std::fs::create_dir_all(&dir).expect("create dir");
    // Create files in reverse alphabetical order.
    for name in ["z.md", "a.md", "m.md", "README.md", "_mapping.md"] {
        std::fs::write(dir.join(name), "# stub").expect("write");
    }
    let paths = walk_catalog_files(&dir).expect("walk succeeds");
    let names: Vec<&str> = paths
        .iter()
        .filter_map(|p| p.file_name()?.to_str())
        .collect();
    // README.md and _mapping.md are excluded; remaining sorted.
    assert_eq!(names, vec!["a.md", "m.md", "z.md"]);
}

#[test]
fn walk_catalog_files_skips_non_md_files() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().join("catalog");
    std::fs::create_dir_all(&dir).expect("create dir");
    std::fs::write(dir.join("data.csv"), "csv content").expect("write");
    std::fs::write(dir.join("ecma-48.md"), "# catalog").expect("write");
    let paths = walk_catalog_files(&dir).expect("walk succeeds");
    assert_eq!(paths.len(), 1);
}
