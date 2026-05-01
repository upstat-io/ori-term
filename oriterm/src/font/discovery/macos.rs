//! macOS font discovery via recursive directory scanning.
//!
//! Owns only the platform-specific bit — `font_dirs()`. Enumeration, parsing,
//! grouping, the resolution-bridge logic, the platform-default fallback walk,
//! and the user-fallback resolver all live in the shared `super::unix` module
//! per `LEAK:algorithmic-duplication` SSOT discipline. Future enhancement:
//! `CoreText` `CTFontCreateWithName` API for better matching.

use std::collections::HashMap;
use std::path::PathBuf;

use super::{DiscoveryResult, FallbackDiscovery, FamilyEntry};

/// Standard font directories on macOS, in priority order.
///
/// User fonts take precedence, then system-wide, then Apple system fonts.
pub(super) fn font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::with_capacity(4);
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join("Library/Fonts"));
    }
    dirs.push(PathBuf::from("/Library/Fonts"));
    dirs.push(PathBuf::from("/System/Library/Fonts"));
    dirs.push(PathBuf::from("/System/Library/Fonts/Supplemental"));
    dirs
}

/// Build a filename → full path index by scanning all font directories once.
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
