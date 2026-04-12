//! Module-level integration tests for the `catalog` module.
//!
//! Per-submodule unit tests live in their respective sibling
//! `tests.rs` files:
//! - `tuple/tests.rs` — canonical tuple parsing and Tuple equality
//! - `check/tests.rs` — `--check` pass finding detection
//! - `parser/tests.rs` — catalog markdown table parsing
//!
//! This file covers cross-module integration tests, positive-pin
//! tests for extractors, and the `walk_catalog_files` function
//! from `mod.rs`.

use super::tuple::Category;
use super::walk_catalog_files;
use super::{build_dispatch_map, extract_dispatch_tuples, extract_namedprivatemode_tuples};

/// Resolve the workspace root from `CARGO_MANIFEST_DIR`.
fn workspace_root() -> std::path::PathBuf {
    std::env::var("CARGO_MANIFEST_DIR")
        .ok()
        .and_then(|mpd| {
            std::path::PathBuf::from(mpd)
                .parent()?
                .parent()
                .map(std::path::Path::to_path_buf)
        })
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

// -------- walk_catalog_files -----------------------------------------------

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

// -------- Positive-pin: extract_dispatch_tuples ----------------------------

#[test]
fn extract_dispatch_tuples_includes_known_csi_tuples() {
    let root = workspace_root();
    let csi_path = root.join("crates/vte/src/ansi/dispatch/csi.rs");
    if !csi_path.exists() {
        eprintln!(
            "SKIP: VTE dispatch source not found at {}",
            csi_path.display()
        );
        return;
    }
    let tuples = extract_dispatch_tuples(&root).expect("extraction succeeds");

    // CUP: CSI Ps ; Ps H — cursor position, universally implemented.
    let has_cup = tuples
        .iter()
        .any(|t| t.category == Category::Csi && t.intermediates.is_empty() && t.final_byte == "H");
    assert!(has_cup, "CUP (CSI H) must be present in dispatch tuples");

    // ED: CSI Ps J — erase in display.
    let has_ed = tuples
        .iter()
        .any(|t| t.category == Category::Csi && t.intermediates.is_empty() && t.final_byte == "J");
    assert!(has_ed, "ED (CSI J) must be present in dispatch tuples");

    // SGR: CSI Ps m — at least one SGR parameter tuple.
    let has_sgr = tuples
        .iter()
        .any(|t| t.category == Category::Csi && t.intermediates.is_empty() && t.final_byte == "m");
    assert!(has_sgr, "SGR (CSI m) must be present in dispatch tuples");

    // APC: unconditional dispatch.
    let has_apc = tuples
        .iter()
        .any(|t| t.category == Category::Apc && t.final_byte == "ST");
    assert!(has_apc, "APC must be present in dispatch tuples");
}

// -------- Positive-pin: extract_namedprivatemode_tuples --------------------

#[test]
fn extract_namedprivatemode_tuples_includes_known_modes() {
    let root = workspace_root();
    let types_path = root.join("crates/vte/src/ansi/types.rs");
    if !types_path.exists() {
        eprintln!("SKIP: types.rs not found at {}", types_path.display());
        return;
    }
    let tuples = extract_namedprivatemode_tuples(&root).expect("extraction succeeds");

    // DECCKM (mode 1) — cursor key mode.
    let has_decckm_set = tuples.iter().any(|t| {
        t.category == Category::Csi
            && t.intermediates == [b'?']
            && t.params == "1"
            && t.final_byte == "h"
    });
    assert!(has_decckm_set, "DECCKM set (CSI ? 1 h) must be present");

    // DECTCEM (mode 25) — show/hide cursor.
    let has_dectcem_reset = tuples.iter().any(|t| {
        t.category == Category::Csi
            && t.intermediates == [b'?']
            && t.params == "25"
            && t.final_byte == "l"
    });
    assert!(
        has_dectcem_reset,
        "DECTCEM reset (CSI ? 25 l) must be present"
    );

    // Every mode number should produce exactly 2 tuples (h + l).
    assert_eq!(
        tuples.len() % 2,
        0,
        "NamedPrivateMode tuples must come in h/l pairs"
    );
}

// -------- Positive-pin: build_dispatch_map ---------------------------------

#[test]
fn build_dispatch_map_includes_known_handler_names() {
    let root = workspace_root();
    let csi_path = root.join("crates/vte/src/ansi/dispatch/csi.rs");
    if !csi_path.exists() {
        eprintln!(
            "SKIP: VTE dispatch source not found at {}",
            csi_path.display()
        );
        return;
    }
    let map = build_dispatch_map(&root).expect("dispatch map builds");

    // The map should contain at least CSI, OSC, ESC, and APC entries.
    let categories: std::collections::BTreeSet<_> = map.keys().map(|t| t.category).collect();
    assert!(
        categories.contains(&Category::Csi),
        "dispatch map must contain CSI entries"
    );
    assert!(
        categories.contains(&Category::Osc),
        "dispatch map must contain OSC entries"
    );

    // Verify a known handler: CUP (CSI H) should call `goto`.
    let cup_key = map
        .keys()
        .find(|t| t.category == Category::Csi && t.intermediates.is_empty() && t.final_byte == "H");
    assert!(cup_key.is_some(), "CUP must be in dispatch map");
    let cup_handlers = &map[cup_key.unwrap()];
    assert!(
        cup_handlers.contains("goto"),
        "CUP handler must include `goto`, got: {cup_handlers:?}"
    );

    // NamedPrivateMode entries should have set/unset handlers.
    let has_set_pm = map.values().any(|hs| hs.contains("set_private_mode"));
    assert!(
        has_set_pm,
        "dispatch map must contain set_private_mode handler"
    );
    let has_unset_pm = map.values().any(|hs| hs.contains("unset_private_mode"));
    assert!(
        has_unset_pm,
        "dispatch map must contain unset_private_mode handler"
    );

    // APC handler.
    let has_apc_handler = map.values().any(|hs| hs.contains("apc_dispatch"));
    assert!(
        has_apc_handler,
        "dispatch map must contain apc_dispatch handler"
    );
}
