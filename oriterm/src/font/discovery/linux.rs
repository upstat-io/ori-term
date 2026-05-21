//! Linux font discovery via recursive directory scanning.
//!
//! Owns only the platform-specific bit — `font_dirs()`. Enumeration, parsing,
//! grouping, the resolution-bridge logic, the platform-default fallback walk,
//! and the user-fallback resolver all live in the shared `super::unix` module
//! per `` SSOT discipline.

use std::collections::HashMap;
use std::path::PathBuf;

use super::{DiscoveryResult, FallbackDiscovery, FamilyEntry};

/// Standard font directories on Linux, in priority order.
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

/// Try platform default families — delegates to the shared
/// `try_platform_defaults_with_index` in `super::unix`.
pub(super) fn try_platform_defaults(
 _weight: u16,
 index: &HashMap<String, PathBuf>,
) -> Option<DiscoveryResult> {
 super::unix::try_platform_defaults_with_index(index)
}

/// Resolve a user-configured fallback font name — delegates to
/// `super::unix::resolve_user_fallback_with_index`.
pub(super) fn resolve_user_fallback(
 family: &str,
 index: &HashMap<String, PathBuf>,
) -> Option<FallbackDiscovery> {
 super::unix::resolve_user_fallback_with_index(family, index)
}

/// Enumerate every monospace font family found under `roots` — delegates to
/// the shared parser in `super::unix`.
pub(super) fn enumerate_mono_families_from_roots(roots: &[PathBuf]) -> Vec<FamilyEntry> {
 super::unix::enumerate_mono_families_from_roots(roots)
}
