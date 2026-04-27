//! Path discovery for `ori_term` test infrastructure.
//!
//! `ori_term` is split between:
//! - **`term_repo/`** — Cargo workspace containing all Rust crates,
//!   `assets/`, `target/`, vendored crates (`crates/vte`, `crates/portable-pty`).
//!   Ships standalone to end users.
//! - **Wrapper repo** — parent of `term_repo/`, owns `plans/` (incl.
//!   `plans/spec-conformance/{catalog,captures,specs,audits,coverage-baseline.toml}`),
//!   `bug-tracker/`, `diagnostics/`, `scripts/`. Internal-only; never shipped.
//!
//! Two ownership domains, two helpers:
//!
//! - [`term_workspace_root`] — `term_repo/` Cargo workspace root. Always
//!   available; this code only compiles inside the workspace. Use for
//!   source/test directory scanning, `target/`, vendored crates.
//! - [`wrapper_root`] — wrapper repo root, found by walking up from
//!   `CARGO_MANIFEST_DIR` until `plans/spec-conformance/` is encountered.
//!   Returns `None` on standalone `term_repo` checkout. Consumers MUST handle
//!   the `None` case via graceful skip per `.claude/rules/tests.md
//!   §Graceful Skip Protocol` — never panic, never `include_bytes!` against
//!   wrapper-resident files.
//!
//! Convenience helpers ([`spec_conformance_dir`], [`catalog_dir`],
//! [`captures_dir`], [`specs_dir`], [`audits_dir`],
//! [`coverage_baseline_path`]) all return `Option<PathBuf>` mirroring
//! `wrapper_root`'s nullability.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Marker directory used to identify the wrapper repo root during walk-up.
const SPEC_CONFORMANCE_MARKER: &str = "plans/spec-conformance";

/// Cargo workspace root for `term_repo/`. Always available — this function
/// only compiles inside the workspace, and the workspace root is the
/// grandparent of this crate's `CARGO_MANIFEST_DIR`.
//
// Note: `expect()` below is reachable only if oriterm_test_support is
// relocated outside `crates/<name>/` — a structural change that would
// invalidate the workspace layout. Acceptable failure mode.
#[must_use]
pub fn term_workspace_root() -> &'static Path {
    static CACHED: OnceLock<PathBuf> = OnceLock::new();
    CACHED.get_or_init(|| {
        let mpd = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        mpd.parent()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .expect("CARGO_MANIFEST_DIR must have two ancestors (the term_repo workspace root)")
    })
}

/// Wrapper repo root, found by walking up from `CARGO_MANIFEST_DIR` until
/// `plans/spec-conformance/` is encountered. Returns `None` on standalone
/// `term_repo` checkout (no wrapper).
///
/// Cached per-process via `OnceLock`. The walk-up cost is a handful of
/// `stat` calls, so caching is purely ergonomic.
#[must_use]
pub fn wrapper_root() -> Option<&'static Path> {
    static CACHED: OnceLock<Option<PathBuf>> = OnceLock::new();
    CACHED
        .get_or_init(|| walk_up_from(Path::new(env!("CARGO_MANIFEST_DIR"))))
        .as_deref()
}

/// Wrapper-relative `plans/spec-conformance/`. Returns `None` when wrapper absent.
#[must_use]
pub fn spec_conformance_dir() -> Option<PathBuf> {
    wrapper_root().map(|r| r.join(SPEC_CONFORMANCE_MARKER))
}

/// Wrapper-relative `plans/spec-conformance/catalog/`. Returns `None` when wrapper absent.
#[must_use]
pub fn catalog_dir() -> Option<PathBuf> {
    spec_conformance_dir().map(|d| d.join("catalog"))
}

/// Wrapper-relative `plans/spec-conformance/captures/`. Returns `None` when wrapper absent.
#[must_use]
pub fn captures_dir() -> Option<PathBuf> {
    spec_conformance_dir().map(|d| d.join("captures"))
}

/// Wrapper-relative `plans/spec-conformance/specs/`. Returns `None` when wrapper absent.
#[must_use]
pub fn specs_dir() -> Option<PathBuf> {
    spec_conformance_dir().map(|d| d.join("specs"))
}

/// Wrapper-relative `plans/spec-conformance/audits/`. Returns `None` when wrapper absent.
#[must_use]
pub fn audits_dir() -> Option<PathBuf> {
    spec_conformance_dir().map(|d| d.join("audits"))
}

/// Wrapper-relative `plans/spec-conformance/coverage-baseline.toml`.
/// Returns `None` when wrapper absent.
#[must_use]
pub fn coverage_baseline_path() -> Option<PathBuf> {
    spec_conformance_dir().map(|d| d.join("coverage-baseline.toml"))
}

/// Walk up from `start` until `plans/spec-conformance/` is found as a
/// directory. Returns `Some(ancestor)` on hit, `None` when the filesystem
/// root is reached without a match.
///
/// Pure — no env reads, no caching. Parameterized on `start` so unit tests
/// can drive it with temp-dir fixtures (the public [`wrapper_root`] uses
/// `env!("CARGO_MANIFEST_DIR")` which cannot be mocked from tests).
pub(crate) fn walk_up_from(start: &Path) -> Option<PathBuf> {
    let mut cur = start.to_path_buf();
    loop {
        if cur.join(SPEC_CONFORMANCE_MARKER).is_dir() {
            return Some(cur);
        }
        if !cur.pop() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests;
