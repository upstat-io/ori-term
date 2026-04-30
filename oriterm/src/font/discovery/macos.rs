//! macOS font discovery via recursive directory scanning.
//!
//! Same scanning approach as Linux but with macOS-specific font directories.
//! Future enhancement: CoreText `CTFontCreateWithName` API for better matching.

use std::collections::HashMap;
use std::path::PathBuf;

use memmap2::Mmap;
use skrifa::MetadataProvider;
use skrifa::raw::tables::head::MacStyle;
use skrifa::raw::tables::os2::SelectionFlags;
use skrifa::raw::{FileRef, TableProvider};
use skrifa::string::StringId;

use super::families::PRIMARY_FAMILIES;
use super::walk::walk_font_dirs;
use super::{
    DiscoveryResult, FallbackDiscovery, FamilyEntry, FontOrigin, resolve_fallback_chain,
    try_families_from_specs,
};

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
    let mut index = HashMap::new();
    walk_font_dirs(&font_dirs(), &mut |path| {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            index
                .entry(name.to_owned())
                .or_insert_with(|| path.to_path_buf());
        }
    });
    index
}

/// Try to find a user-specified family by scanning for filenames.
pub(super) fn try_user_family(
    name: &str,
    _weight: u16,
    index: &HashMap<String, PathBuf>,
) -> Option<DiscoveryResult> {
    let lookup = |filename: &str| -> Option<PathBuf> { index.get(filename).cloned() };

    // Bridge: resolve enumerated family-name → real file paths. The catalog is
    // populated with OpenType `name`-table family names; the filename heuristics
    // below cannot match these. Sibling Bold/Italic/BoldItalic faces discovered
    // during enumeration win; missing slots get a second chance via filename
    // probing against the existing index (zero disk I/O).
    if let Some((mut full_paths, face_indices)) = super::family_paths(name) {
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

    // Try the name as a filename directly.
    if let Some(path) = index.get(name) {
        let primary = super::family_from_paths(
            name,
            [Some(path.clone()), None, None, None],
            FontOrigin::UserConfig,
        );
        let fallbacks = resolve_fallback_chain(&lookup, FontOrigin::DirectoryScan);
        return Some(DiscoveryResult { primary, fallbacks });
    }

    // Try common naming patterns.
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

    // Try as absolute path.
    let path = PathBuf::from(name);
    if path.is_absolute() && path.exists() {
        let primary =
            super::family_from_paths(name, [Some(path), None, None, None], FontOrigin::UserConfig);
        let fallbacks = resolve_fallback_chain(&lookup, FontOrigin::DirectoryScan);
        return Some(DiscoveryResult { primary, fallbacks });
    }

    None
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

/// Enumerate every monospace font family found under `roots`.
///
/// Mirror of the Linux enumeration path — same memory-map + skrifa parse +
/// per-family grouping. macOS shares the directory-scan strategy with Linux,
/// so no platform-specific divergence here.
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

struct RawFaceInfo {
    display_name: String,
    path: PathBuf,
    face_index: u32,
    is_bold: bool,
    is_italic: bool,
}

#[expect(
    unsafe_code,
    reason = "memmap2::Mmap::map is unsafe by API; font files are read-only system resources, immutable Mmap matches loading.rs convention"
)]
fn parse_face_info(path: &std::path::Path) -> Option<RawFaceInfo> {
    let file = std::fs::File::open(path).ok()?;
    // SAFETY: Font files are read-only system resources that are not modified
    // or truncated while the terminal is running. The mapping is immutable.
    let mmap: Mmap = unsafe { Mmap::map(&file).ok()? };

    let file_ref = FileRef::new(&mmap).ok()?;
    let (font, face_index) = match file_ref {
        FileRef::Font(f) => (f, 0u32),
        FileRef::Collection(c) => (c.get(0).ok()?, 0u32),
    };

    let post = font.post().ok()?;
    if post.is_fixed_pitch() == 0 {
        return None;
    }

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

fn strip_regular_suffix(stem: &str) -> String {
    const SUFFIXES: &[&str] = &[
        "-Regular", "_Regular", " Regular", "-Roman", "_Roman", " Roman", "Regular", "Roman",
    ];
    for suffix in SUFFIXES {
        if stem.len() <= suffix.len() {
            continue;
        }
        let cut = stem.len() - suffix.len();
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

fn variant_from_index(
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
