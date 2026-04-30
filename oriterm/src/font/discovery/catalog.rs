//! `OnceLock`-cached monospace font family catalog.
//!
//! Owns [`enumerate_mono_families()`], the sibling [`family_paths()`] lookup
//! used by the resolution-bridge, and the test/bench seam
//! [`enumerate_mono_families_from_roots()`]. Extracted from `mod.rs` to keep
//! that file under the 500-line size limit (closes Round 1 codex F4 +
//! opencode F1 BLOAT findings — `bug-tracker/plans/BUG-02-012/section-06-tpr-findings.md`).

use std::collections::HashMap;
use std::sync::OnceLock;

#[cfg(target_os = "linux")]
use super::linux;
#[cfg(target_os = "macos")]
use super::macos;
#[cfg(target_os = "windows")]
use super::windows;
use super::{FamilyEntry, FamilySlots};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::path::PathBuf;

/// All installed monospace font families on the host system.
///
/// Sorted case-insensitively, deduplicated by `display_name` (first-seen wins).
/// Lazily initialized on first call; subsequent calls are O(1) lookups against
/// the cached `OnceLock`. Powers the searchable font-family dropdown in the
/// settings overlay and bridges enumerated family names back to file paths
/// via [`family_paths`].
///
/// On Linux/macOS the catalog is built by walking standard font directories,
/// memory-mapping each candidate file, and parsing the `OpenType` `name` /
/// `OS/2` / `head` / `post` tables via `skrifa`. On Windows the catalog is
/// built by iterating the `DirectWrite` system font collection and filtering
/// to monospace families.
pub fn enumerate_mono_families() -> &'static [FamilyEntry] {
    FAMILY_CATALOG.get_or_init(|| {
        let mut entries = platform_enumerate();
        // Cache lowercased keys via `sort_by_cached_key` — avoids O(N log N)
        // re-allocations of `to_lowercase()` strings during the comparator.
        entries.sort_by_cached_key(|fe| fe.display_name.to_lowercase());
        entries.dedup_by(|a, b| a.display_name.eq_ignore_ascii_case(&b.display_name));
        let map: HashMap<String, FamilySlots> = entries
            .iter()
            .map(|fe| (fe.display_name.clone(), (fe.paths.clone(), fe.face_indices)))
            .collect();
        // First-write wins; subsequent attempts are no-ops because `enumerate_mono_families`
        // is the sole writer and `OnceLock::set` is idempotent under contention.
        let _ = FAMILY_NAME_TO_PATHS.set(map);
        entries
    })
}

/// Resolve an enumerated family name to its `(paths, face_indices)` slot pair.
///
/// Used by `try_user_family` on each platform as the bridge between an
/// `OpenType`-name-table family name (e.g. `"JetBrains Mono"`) and the actual
/// file paths discovered during enumeration. Returns `None` if the family was
/// never enumerated. Initializes the catalog on first call (idempotent).
pub(in crate::font::discovery) fn family_paths(name: &str) -> Option<FamilySlots> {
    // Force catalog population so the side-table is ready.
    let _ = enumerate_mono_families();
    FAMILY_NAME_TO_PATHS
        .get()
        .and_then(|m| m.get(name).cloned())
}

static FAMILY_CATALOG: OnceLock<Vec<FamilyEntry>> = OnceLock::new();
static FAMILY_NAME_TO_PATHS: OnceLock<HashMap<String, FamilySlots>> = OnceLock::new();

/// Per-platform enumeration entry point. Linux/macOS walk standard font dirs;
/// Windows iterates `DirectWrite`'s system font collection.
#[cfg(target_os = "linux")]
fn platform_enumerate() -> Vec<FamilyEntry> {
    linux::enumerate_mono_families_from_roots(&linux::font_dirs())
}
#[cfg(target_os = "macos")]
fn platform_enumerate() -> Vec<FamilyEntry> {
    macos::enumerate_mono_families_from_roots(&macos::font_dirs())
}
#[cfg(target_os = "windows")]
fn platform_enumerate() -> Vec<FamilyEntry> {
    windows::enumerate_mono_families_inner()
}

/// Bench/test seam: enumerate mono families from explicit roots, bypassing
/// the `OnceLock`-cached system catalog. Returns an unsorted, undeduplicated
/// list — callers responsible for any post-processing they need. Linux/macOS
/// only — Windows `DirectWrite` has no path-roots concept.
#[cfg(any(target_os = "linux", target_os = "macos"))]
#[doc(hidden)]
pub fn enumerate_mono_families_from_roots(roots: &[PathBuf]) -> Vec<FamilyEntry> {
    #[cfg(target_os = "linux")]
    {
        linux::enumerate_mono_families_from_roots(roots)
    }
    #[cfg(target_os = "macos")]
    {
        macos::enumerate_mono_families_from_roots(roots)
    }
}
