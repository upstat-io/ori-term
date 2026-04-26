use std::fs;

use tempfile::TempDir;

use super::{
    audits_dir, captures_dir, catalog_dir, coverage_baseline_path, spec_conformance_dir, specs_dir,
    term_workspace_root, walk_up_from, wrapper_root,
};

/// Helper: create a temp dir with `plans/spec-conformance/` marker subdir.
fn temp_with_marker() -> TempDir {
    let tmp = TempDir::new().expect("tempdir");
    fs::create_dir_all(tmp.path().join("plans/spec-conformance")).expect("create marker subdir");
    tmp
}

/// Helper: create a temp dir without the marker.
fn temp_without_marker() -> TempDir {
    TempDir::new().expect("tempdir")
}

// ── walk_up_from — semantic pins ─────────────────────────────────────────

/// Pin: marker at the start dir → walk-up returns the start dir itself.
#[test]
fn walk_up_returns_self_when_marker_at_start_dir() {
    let tmp = temp_with_marker();
    let result = walk_up_from(tmp.path());
    assert_eq!(result.as_deref(), Some(tmp.path()));
}

/// Pin: marker one level up (canonical "wrapper at parent of term_repo") case.
#[test]
fn walk_up_returns_parent_when_marker_one_level_up() {
    let wrapper = temp_with_marker();
    let term_repo = wrapper.path().join("term_repo");
    fs::create_dir_all(&term_repo).expect("create term_repo");
    let result = walk_up_from(&term_repo);
    assert_eq!(result.as_deref(), Some(wrapper.path()));
}

/// Pin: marker three levels up (mirrors actual depth from
/// `term_repo/oriterm_core/tests/spec_chain/pilots/` to wrapper).
#[test]
fn walk_up_returns_ancestor_when_marker_three_levels_up() {
    let wrapper = temp_with_marker();
    let deep = wrapper
        .path()
        .join("term_repo/crates/oriterm_test_support/src/paths");
    fs::create_dir_all(&deep).expect("create deep");
    let result = walk_up_from(&deep);
    assert_eq!(result.as_deref(), Some(wrapper.path()));
}

/// Pin: no marker anywhere → walk-up returns None (standalone term_repo case).
///
/// Uses a deeply-nested temp tree with no `plans/spec-conformance/` ancestor.
/// Walk-up exhausts to filesystem root.
#[test]
fn walk_up_returns_none_when_no_marker_anywhere() {
    let no_marker = temp_without_marker();
    let inner = no_marker.path().join("crates/foo/src");
    fs::create_dir_all(&inner).expect("create inner");
    // Note: this assumes no ancestor of `inner` (above the temp dir) has a
    // `plans/spec-conformance/` subdirectory. On a clean test runner that's
    // true. If the test ever fires unexpectedly on a developer machine where
    // a parent dir has such a subdir, the temp dir is the wrong base —
    // we'd need to fork a child process with a chrooted/sandboxed root,
    // which is out of scope for an in-process unit test.
    let result = walk_up_from(&inner);
    // We assert that EITHER the result is None OR the discovered root is
    // outside our temp dir (guarding against the rare developer-machine
    // case where a parent dir up the tree has `plans/spec-conformance/`).
    if let Some(found) = result {
        assert!(
            !found.starts_with(no_marker.path()),
            "walk-up should not find a marker inside the marker-less temp dir; found {}",
            found.display(),
        );
    }
}

/// Negative pin: marker is a regular file (not a directory) → walk-up
/// distinguishes file from dir, returns None.
#[test]
fn walk_up_distinguishes_dir_marker_from_file_marker() {
    let tmp = TempDir::new().expect("tempdir");
    fs::create_dir_all(tmp.path().join("plans")).expect("create plans");
    fs::write(
        tmp.path().join("plans/spec-conformance"),
        "this is a file, not a dir",
    )
    .expect("write file");
    let result = walk_up_from(tmp.path());
    // The marker exists as a file, but is_dir() is false → walk-up keeps going.
    // Result depends on what's above the temp dir; verify it's not pointing at our tmp.
    if let Some(found) = result {
        assert_ne!(found, tmp.path(), "file-not-dir should NOT count as marker");
    }
}

/// Negative pin: only `plans/` exists, not `plans/spec-conformance/` → walk-up keeps walking.
#[test]
fn walk_up_ignores_partial_marker_subdir() {
    let tmp = TempDir::new().expect("tempdir");
    fs::create_dir_all(tmp.path().join("plans")).expect("create plans only");
    let result = walk_up_from(tmp.path());
    if let Some(found) = result {
        assert_ne!(
            found,
            tmp.path(),
            "plans/ alone (no spec-conformance/ child) is not the marker"
        );
    }
}

// ── public API smoke ─────────────────────────────────────────────────────

/// Smoke: `term_workspace_root()` returns a path that ends with `term_repo`
/// AND contains `crates/oriterm_test_support` as a child.
#[test]
fn term_workspace_root_resolves_to_term_repo() {
    let root = term_workspace_root();
    assert!(
        root.join("crates/oriterm_test_support").is_dir(),
        "term_workspace_root should contain crates/oriterm_test_support; got {}",
        root.display()
    );
}

/// Smoke: `wrapper_root()` returns the same value across two calls (OnceLock cache).
#[test]
fn wrapper_root_returns_consistent_result_within_process() {
    let first = wrapper_root();
    let second = wrapper_root();
    assert_eq!(
        first, second,
        "wrapper_root should return the same value across calls"
    );
}

/// Smoke: when wrapper IS present (current test environment), `wrapper_root()`
/// is `Some` and points at a directory containing `plans/spec-conformance/`.
///
/// This test exercises the "happy path" the live tests in the workspace
/// depend on. If it fails in CI, the wrapper layout has changed and the
/// graceful-skip arms in consumer tests would now fire silently.
#[test]
fn wrapper_root_finds_wrapper_under_actual_layout() {
    let Some(root) = wrapper_root() else {
        eprintln!(
            "SKIP wrapper_root_finds_wrapper_under_actual_layout: \
             no wrapper repo discoverable from {} \
             (running under standalone term_repo checkout)",
            env!("CARGO_MANIFEST_DIR")
        );
        return;
    };
    assert!(
        root.join("plans/spec-conformance").is_dir(),
        "wrapper_root should point at a dir containing plans/spec-conformance/; got {}",
        root.display()
    );
}

/// Smoke: convenience helpers all derive from `wrapper_root()` consistently.
/// When wrapper is present, every helper returns `Some(path)` matching its
/// documented child-dir mapping.
#[test]
fn convenience_helpers_match_wrapper_root_under_actual_layout() {
    let Some(wrapper) = wrapper_root() else {
        eprintln!("SKIP convenience_helpers_match_wrapper_root: standalone term_repo");
        return;
    };
    assert_eq!(
        spec_conformance_dir().as_deref(),
        Some(wrapper.join("plans/spec-conformance")).as_deref()
    );
    assert_eq!(
        catalog_dir().as_deref(),
        Some(wrapper.join("plans/spec-conformance/catalog")).as_deref()
    );
    assert_eq!(
        captures_dir().as_deref(),
        Some(wrapper.join("plans/spec-conformance/captures")).as_deref()
    );
    assert_eq!(
        specs_dir().as_deref(),
        Some(wrapper.join("plans/spec-conformance/specs")).as_deref()
    );
    assert_eq!(
        audits_dir().as_deref(),
        Some(wrapper.join("plans/spec-conformance/audits")).as_deref()
    );
    assert_eq!(
        coverage_baseline_path().as_deref(),
        Some(wrapper.join("plans/spec-conformance/coverage-baseline.toml")).as_deref()
    );
}
