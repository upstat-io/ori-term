//! Shared Unix (Linux + macOS) font enumeration.
//!
//! Linux and macOS share the same directory-scan strategy: walk standard font
//! dirs, memory-map each candidate file, parse the `OpenType` `name`/`OS/2`/
//! `head`/`post` tables via `skrifa`, and group faces by `display_name` to
//! emit one [`FamilyEntry`] per family.
//!
//! This module owns the protocol (parsing, grouping, bridge); per-platform
//! files own only the platform-specific bits — `font_dirs()`, the resolution-
//! bridge entry point, and the platform-default fallback list.

#![cfg(any(target_os = "linux", target_os = "macos"))]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use memmap2::Mmap;
use skrifa::MetadataProvider;
use skrifa::raw::tables::head::MacStyle;
use skrifa::raw::tables::os2::SelectionFlags;
use skrifa::raw::{FileRef, TableProvider};
use skrifa::string::StringId;

use super::families::PRIMARY_FAMILIES;
use super::walk::walk_font_dirs;
use super::{
    DiscoveryResult, FallbackDiscovery, FamilyEntry, FamilySlots, FontOrigin,
    resolve_fallback_chain, try_families_from_specs,
};

/// Per-file shape collected during enumeration before grouping into families.
struct RawFaceInfo {
    display_name: String,
    path: PathBuf,
    face_index: u32,
    is_bold: bool,
    is_italic: bool,
}

/// Build a filename → full path index from the given roots. Used by both
/// Linux and macOS `build_font_index()` per `LEAK:algorithmic-duplication`
/// SSOT discipline.
pub(super) fn build_font_index_from_roots(roots: &[PathBuf]) -> HashMap<String, PathBuf> {
    let mut index = HashMap::new();
    walk_font_dirs(roots, &mut |path| {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            index
                .entry(name.to_owned())
                .or_insert_with(|| path.to_path_buf());
        }
    });
    index
}

/// Enumerate every monospace font family found under `roots`.
///
/// Walks each root recursively, memory-maps every regular file, parses the
/// `OpenType` `name` / `OS/2` / `head` / `post` tables via `skrifa`, and groups
/// faces by `display_name`. Only families with a Regular face are emitted —
/// `Bold`/`Italic`/`BoldItalic`-only collections are dropped (no anchor face).
pub(super) fn enumerate_mono_families_from_roots(roots: &[PathBuf]) -> Vec<FamilyEntry> {
    let mut faces_by_family: HashMap<String, Vec<RawFaceInfo>> = HashMap::new();
    walk_font_dirs(roots, &mut |path| match parse_face_info(path) {
        Some(face) => faces_by_family
            .entry(face.display_name.clone())
            .or_default()
            .push(face),
        None => log::trace!(
            "font enumeration: skipping {} (parse error or non-monospace)",
            path.display()
        ),
    });

    let mut entries: Vec<FamilyEntry> = Vec::with_capacity(faces_by_family.len());
    for (display_name, faces) in faces_by_family {
        let regular = faces.iter().find(|f| !f.is_bold && !f.is_italic);
        let bold = faces.iter().find(|f| f.is_bold && !f.is_italic);
        let italic = faces.iter().find(|f| !f.is_bold && f.is_italic);
        let bold_italic = faces.iter().find(|f| f.is_bold && f.is_italic);
        let Some(reg) = regular else {
            log::trace!("font enumeration: skipping family {display_name} (no regular face found)");
            continue;
        };
        entries.push(FamilyEntry {
            display_name,
            paths: [
                Some(reg.path.clone()),
                bold.map(|f| f.path.clone()),
                italic.map(|f| f.path.clone()),
                bold_italic.map(|f| f.path.clone()),
            ],
            face_indices: [
                reg.face_index,
                bold.map_or(0, |f| f.face_index),
                italic.map_or(0, |f| f.face_index),
                bold_italic.map_or(0, |f| f.face_index),
            ],
        });
    }
    entries
}

/// Parse a single font file into a [`RawFaceInfo`], or return `None` on any
/// failure (unreadable file, non-font magic, missing tables, non-monospace).
#[expect(
    unsafe_code,
    reason = "memmap2::Mmap::map is unsafe by API; font files are read-only system resources, immutable Mmap matches loading.rs convention"
)]
fn parse_face_info(path: &Path) -> Option<RawFaceInfo> {
    let file = std::fs::File::open(path).ok()?;
    // SAFETY: Font files are read-only system resources that are not modified
    // or truncated while the terminal is running. The mapping is immutable.
    let mmap: Mmap = unsafe { Mmap::map(&file).ok()? };

    let file_ref = FileRef::new(&mmap).ok()?;
    let (font, face_index) = match file_ref {
        FileRef::Font(f) => (f, 0u32),
        // v1: only face index 0 from collections — full TTC enumeration is
        // tracked separately and does not block this catalog.
        FileRef::Collection(c) => (c.get(0).ok()?, 0u32),
    };

    // Monospace gate at enumeration time: anything else gets dropped.
    let post = font.post().ok()?;
    if post.is_fixed_pitch() == 0 {
        return None;
    }

    // Family name: prefer the typographic family (id 16); fall back to the
    // legacy family name (id 1). Reject empty/whitespace-only results.
    let raw_name = font
        .localized_strings(StringId::TYPOGRAPHIC_FAMILY_NAME)
        .english_or_first()
        .or_else(|| {
            font.localized_strings(StringId::FAMILY_NAME)
                .english_or_first()
        })?;
    let trimmed = raw_name.to_string().trim().to_owned();
    if trimmed.is_empty() {
        return None;
    }

    // Style detection: try `OS/2` `fsSelection` first; fall back to the legacy
    // `head.macStyle` for Apple-shipped fonts that omit `OS/2`. Without the
    // fallback, those fonts are silently dropped.
    let (is_bold, is_italic) = if let Ok(os2) = font.os2() {
        let fs = os2.fs_selection();
        (
            fs.contains(SelectionFlags::BOLD),
            fs.contains(SelectionFlags::ITALIC),
        )
    } else if let Ok(head) = font.head() {
        let mac = head.mac_style();
        (mac.contains(MacStyle::BOLD), mac.contains(MacStyle::ITALIC))
    } else {
        return None;
    };

    Some(RawFaceInfo {
        display_name: trimmed,
        path: path.to_path_buf(),
        face_index,
        is_bold,
        is_italic,
    })
}

/// Resolve a user-specified family name via the catalog bridge plus filename
/// heuristics. Used by both Linux and macOS `try_user_family()`.
///
/// Lookup order:
/// 1. `family_paths()` catalog — handles `OpenType`-`name`-table family names
///    (e.g. `"JetBrains Mono"`) which the filename heuristics below cannot
///    match. Sibling Bold/Italic/BoldItalic faces discovered during
///    enumeration win; missing slots get a second chance via filename probing
///    against the existing index (zero disk I/O).
/// 2. Direct filename match (`name == filename`).
/// 3. `{name}-Regular.{ttf,otf}` patterns plus matching Bold/Italic/BoldItalic.
/// 4. Absolute path.
pub(super) fn try_user_family_with_bridge(
    name: &str,
    index: &HashMap<String, PathBuf>,
) -> Option<DiscoveryResult> {
    try_user_family_with_bridge_using(name, index, super::family_paths)
}

/// Inner shape of [`try_user_family_with_bridge`] with the catalog lookup
/// injected. Production calls `super::family_paths`; tests inject a fixture
/// closure so the bridge integration can be exercised without the global
/// `OnceLock`-cached catalog (closes the codex F2 host-font skip per
/// `bug-tracker/plans/BUG-02-012/section-06-tpr-findings.md` Round 0).
pub(super) fn try_user_family_with_bridge_using(
    name: &str,
    index: &HashMap<String, PathBuf>,
    family_paths_lookup: impl FnOnce(&str) -> Option<FamilySlots>,
) -> Option<DiscoveryResult> {
    let lookup = |filename: &str| -> Option<PathBuf> { index.get(filename).cloned() };

    // 1. Bridge: enumerated catalog (typographic family names).
    if let Some((mut full_paths, face_indices)) = family_paths_lookup(name) {
        if let Some(regular_path) = full_paths[0].clone() {
            if let Some(stem) = regular_path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(str::to_owned)
            {
                let base_stem = strip_regular_suffix(&stem);
                if full_paths[1].is_none() {
                    full_paths[1] = variant_from_index(&base_stem, "Bold", index);
                }
                if full_paths[2].is_none() {
                    full_paths[2] = variant_from_index(&base_stem, "Italic", index);
                }
                if full_paths[3].is_none() {
                    full_paths[3] = variant_from_index(&base_stem, "BoldItalic", index);
                }
            }
            let mut entry = super::family_from_paths(name, full_paths, FontOrigin::UserConfig);
            entry.face_indices = face_indices;
            let fallbacks = resolve_fallback_chain(&lookup, FontOrigin::DirectoryScan);
            return Some(DiscoveryResult {
                primary: entry,
                fallbacks,
            });
        }
    }

    // 2. Direct filename match.
    if let Some(path) = index.get(name) {
        let primary = super::family_from_paths(
            name,
            [Some(path.clone()), None, None, None],
            FontOrigin::UserConfig,
        );
        let fallbacks = resolve_fallback_chain(&lookup, FontOrigin::DirectoryScan);
        return Some(DiscoveryResult { primary, fallbacks });
    }

    // 3. `{name}-Regular.{ttf,otf}` heuristic.
    for ext in &["ttf", "otf"] {
        let candidate = format!("{name}-Regular.{ext}");
        if let Some(path) = index.get(&candidate) {
            let bold = index.get(&format!("{name}-Bold.{ext}")).cloned();
            let italic = index.get(&format!("{name}-Italic.{ext}")).cloned();
            let bold_italic = index.get(&format!("{name}-BoldItalic.{ext}")).cloned();

            let primary = super::family_from_paths(
                name,
                [Some(path.clone()), bold, italic, bold_italic],
                FontOrigin::UserConfig,
            );
            let fallbacks = resolve_fallback_chain(&lookup, FontOrigin::DirectoryScan);
            return Some(DiscoveryResult { primary, fallbacks });
        }
    }

    // 4. Absolute path.
    let path = PathBuf::from(name);
    if path.is_absolute() && path.exists() {
        let primary =
            super::family_from_paths(name, [Some(path), None, None, None], FontOrigin::UserConfig);
        let fallbacks = resolve_fallback_chain(&lookup, FontOrigin::DirectoryScan);
        return Some(DiscoveryResult { primary, fallbacks });
    }

    None
}

/// Strip a trailing Regular/Roman suffix from a font filename stem so we can
/// probe sibling variant filenames. Falls back to the unchanged stem when no
/// recognized suffix is found.
pub(super) fn strip_regular_suffix(stem: &str) -> String {
    const SUFFIXES: &[&str] = &[
        "-Regular", "_Regular", " Regular", "-Roman", "_Roman", " Roman", "Regular", "Roman",
    ];
    for suffix in SUFFIXES {
        if stem.len() <= suffix.len() {
            continue;
        }
        let cut = stem.len() - suffix.len();
        // Boundary-safe slicing: skip if `cut` lands inside a multi-byte char.
        if !stem.is_char_boundary(cut) {
            continue;
        }
        let (head, tail) = stem.split_at(cut);
        if tail.eq_ignore_ascii_case(suffix) {
            return head.trim_end_matches(['-', '_', ' ']).to_owned();
        }
    }
    stem.to_owned()
}

/// Look up a `{base_stem}-{variant}.{ttf,otf}` filename in the existing index.
pub(super) fn variant_from_index(
    base_stem: &str,
    variant: &str,
    index: &HashMap<String, PathBuf>,
) -> Option<PathBuf> {
    for ext in &["ttf", "otf"] {
        let key = format!("{base_stem}-{variant}.{ext}");
        if let Some(path) = index.get(&key) {
            return Some(path.clone());
        }
    }
    None
}

/// Try platform default families in priority order. Identical for Linux and
/// macOS — both walk `PRIMARY_FAMILIES` against the filename index. Closes
/// the gemini F1 LEAK:algorithmic-duplication finding from §06 Round 1.
pub(super) fn try_platform_defaults_with_index(
    index: &HashMap<String, PathBuf>,
) -> Option<DiscoveryResult> {
    let lookup = |filename: &str| -> Option<PathBuf> { index.get(filename).cloned() };

    let primary = try_families_from_specs(PRIMARY_FAMILIES, &lookup, FontOrigin::DirectoryScan)?;
    let fallbacks = resolve_fallback_chain(&lookup, FontOrigin::DirectoryScan);
    Some(DiscoveryResult { primary, fallbacks })
}

/// Resolve a user-configured fallback font name to a path. Identical for
/// Linux and macOS — try filename in index, then absolute path. Closes the
/// gemini F1 LEAK:algorithmic-duplication finding from §06 Round 1.
pub(super) fn resolve_user_fallback_with_index(
    family: &str,
    index: &HashMap<String, PathBuf>,
) -> Option<FallbackDiscovery> {
    if let Some(path) = index.get(family) {
        return Some(FallbackDiscovery {
            path: path.clone(),
            face_index: 0,
            origin: FontOrigin::UserConfig,
        });
    }

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
