//! Linux font discovery via recursive directory scanning.
//!
//! Owns only the platform-specific bits — `font_dirs()`, the platform-default
//! fallback list, and `resolve_user_fallback`. Enumeration, parsing, grouping,
//! and the resolution-bridge logic live in the shared `super::unix` module
//! per `LEAK:algorithmic-duplication` SSOT discipline.

use std::collections::HashMap;
use std::path::PathBuf;

use super::families::PRIMARY_FAMILIES;
use super::{
    DiscoveryResult, FallbackDiscovery, FamilyEntry, FontOrigin, resolve_fallback_chain,
    try_families_from_specs,
};

/// Standard font directories on Linux, in priority order.
///
/// User fonts take precedence over system fonts so personal installations
/// override distribution-provided versions.
pub(super) fn font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::with_capacity(3);
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".local/share/fonts"));
    }
    dirs.push(PathBuf::from("/usr/share/fonts"));
    dirs.push(PathBuf::from("/usr/local/share/fonts"));
    dirs
}

/// Build a filename → full path index by scanning all font directories once.
///
/// First-seen wins: if the same filename exists in multiple directories, the
/// one from the highest-priority directory (user before system) is kept.
pub(crate) fn build_font_index() -> HashMap<String, PathBuf> {
    super::unix::build_font_index_from_roots(&font_dirs())
}

/// Try to find a user-specified family — delegates to the shared bridge +
/// filename heuristics in `super::unix`.
pub(super) fn try_user_family(
    name: &str,
    _weight: u16,
    index: &HashMap<String, PathBuf>,
) -> Option<DiscoveryResult> {
    super::unix::try_user_family_with_bridge(name, index)
}

/// Try platform default families in priority order.
pub(super) fn try_platform_defaults(
    _weight: u16,
    index: &HashMap<String, PathBuf>,
) -> Option<DiscoveryResult> {
    let lookup = |filename: &str| -> Option<PathBuf> { index.get(filename).cloned() };

    let primary = try_families_from_specs(PRIMARY_FAMILIES, &lookup, FontOrigin::DirectoryScan)?;
    let fallbacks = resolve_fallback_chain(&lookup, FontOrigin::DirectoryScan);
    Some(DiscoveryResult { primary, fallbacks })
}

/// Resolve a user-configured fallback font name to a path.
///
/// Accepts a pre-built font index to avoid rescanning font directories.
pub(super) fn resolve_user_fallback(
    family: &str,
    index: &HashMap<String, PathBuf>,
) -> Option<FallbackDiscovery> {
    // Try as filename in index.
    if let Some(path) = index.get(family) {
        return Some(FallbackDiscovery {
            path: path.clone(),
            face_index: 0,
            origin: FontOrigin::UserConfig,
        });
    }

    // Try as absolute path.
    let path = PathBuf::from(family);
    if path.is_absolute() && path.exists() {
        return Some(FallbackDiscovery {
            path,
            face_index: 0,
            origin: FontOrigin::UserConfig,
        });
    }

    None
}

/// Enumerate every monospace font family found under `roots` — delegates to
/// the shared parser in `super::unix`.
pub(super) fn enumerate_mono_families_from_roots(roots: &[PathBuf]) -> Vec<FamilyEntry> {
    super::unix::enumerate_mono_families_from_roots(roots)
}
