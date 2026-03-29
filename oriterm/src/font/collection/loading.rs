//! Font loading: discovery → raw bytes → `FontSet`.
//!
//! Bridges platform font discovery with the `FontCollection` validation pipeline.
//! [`FontByteCache`] deduplicates file reads across multiple `FontSet::load` calls.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::super::FontError;
use super::super::discovery::{self, FontOrigin};

/// Cache for font file bytes, keyed by file path.
///
/// Deduplicates `std::fs::read` calls when the same font file appears in
/// multiple discovery results (e.g., terminal and UI fallback chains both
/// include `NotoColorEmoji`). Short-lived: constructed during startup, used
/// for font loading, then dropped.
pub struct FontByteCache {
    entries: HashMap<PathBuf, Arc<Vec<u8>>>,
}

impl FontByteCache {
    /// Create an empty cache.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Load font bytes from `path`, returning a cached `Arc` on repeat reads.
    pub fn load(&mut self, path: &Path) -> std::io::Result<Arc<Vec<u8>>> {
        if let Some(data) = self.entries.get(path) {
            return Ok(Arc::clone(data));
        }
        let bytes = std::fs::read(path)?;
        let arc = Arc::new(bytes);
        self.entries.insert(path.to_owned(), Arc::clone(&arc));
        Ok(arc)
    }
}

/// Raw font bytes and collection index (pre-validation).
///
/// Cheap to clone: font bytes are `Arc`-shared, index is `Copy`.
#[derive(Clone)]
pub(super) struct FontData {
    /// Font file bytes shared via `Arc` for rustybuzz face creation.
    pub(super) data: Arc<Vec<u8>>,
    /// Face index within a `.ttc` collection (0 for standalone `.ttf`).
    pub(super) index: u32,
}

/// Four style variants, optional medium weight, and ordered fallback chain.
///
/// Constructed by [`FontSet::load`] from discovery results. Passed to
/// [`FontCollection::new`] for validation and metrics computation.
///
/// Cheap to clone: each [`FontData`] shares font bytes via `Arc`.
#[derive(Clone)]
pub struct FontSet {
    /// Human-readable family name.
    pub(super) family_name: String,
    /// Regular face data (always present).
    pub(super) regular: FontData,
    /// Bold face data (if a real bold variant was found).
    pub(super) bold: Option<FontData>,
    /// Italic face data (if a real italic variant was found).
    pub(super) italic: Option<FontData>,
    /// Bold-italic face data (if a real bold-italic variant was found).
    pub(super) bold_italic: Option<FontData>,
    /// Medium (500) face data for UI text weight fidelity.
    ///
    /// Separate from the 4-slot primary array because Medium is not a CSS
    /// font-face style slot — it's an intermediate weight used exclusively
    /// by the UI text pipeline. Substituted into the Regular shaping/raster
    /// position when `requested_weight` is in 500..700.
    pub(super) medium: Option<FontData>,
    /// Which style slots have real font files.
    #[allow(dead_code, reason = "font fields consumed in later sections")]
    pub(super) has_variant: [bool; 4],
    /// Ordered fallback fonts for missing-glyph coverage.
    pub(super) fallbacks: Vec<FontData>,
}

impl FontSet {
    /// Build a `FontSet` from the embedded `JetBrains` Mono Regular only.
    ///
    /// No system font discovery, no Bold/Italic/BoldItalic variants, no
    /// fallbacks. Produces deterministic output regardless of system fonts —
    /// ideal for visual regression tests.
    #[cfg(test)]
    pub fn embedded() -> Self {
        Self {
            family_name: "JetBrains Mono (embedded)".to_owned(),
            regular: FontData {
                data: Arc::new(discovery::EMBEDDED_FONT_DATA.to_vec()),
                index: 0,
            },
            bold: None,
            italic: None,
            bold_italic: None,
            medium: None,
            has_variant: [true, false, false, false],
            fallbacks: vec![FontData {
                data: Arc::new(discovery::TEST_EMOJI_DATA.to_vec()),
                index: 0,
            }],
        }
    }

    /// Build a `FontSet` from embedded IBM Plex Mono (Regular + Medium + Bold).
    ///
    /// Fixed UI font for settings dialogs and chrome — independent of the
    /// user's terminal font configuration. Matches the mockup's
    /// `font-family: 'IBM Plex Mono'` exactly on all platforms.
    pub fn ui_embedded() -> Self {
        Self {
            family_name: "IBM Plex Mono (embedded)".to_owned(),
            regular: FontData {
                data: Arc::new(discovery::UI_FONT_REGULAR.to_vec()),
                index: 0,
            },
            bold: Some(FontData {
                data: Arc::new(discovery::UI_FONT_BOLD.to_vec()),
                index: 0,
            }),
            italic: None,
            bold_italic: None,
            medium: Some(FontData {
                data: Arc::new(discovery::UI_FONT_MEDIUM.to_vec()),
                index: 0,
            }),
            has_variant: [true, true, false, false],
            fallbacks: Vec::new(),
        }
    }

    /// Load font data from discovery results (convenience wrapper).
    ///
    /// If `family` is `None`, uses platform defaults (with embedded fallback).
    /// The `weight` parameter is CSS-style (100–900) for the Regular slot.
    ///
    /// Production callers should prefer [`load_cached`](Self::load_cached) to
    /// share a [`FontByteCache`] across multiple loads.
    #[cfg(test)]
    pub fn load(family: Option<&str>, weight: u16) -> Result<Self, FontError> {
        Self::load_cached(family, weight, &mut FontByteCache::new())
    }

    /// Load font data with a shared byte cache for cross-call deduplication.
    ///
    /// Reusing the same `cache` across multiple `load_cached` /
    /// [`from_discovery`](Self::from_discovery) calls deduplicates file reads
    /// for fonts that appear in multiple discovery results (e.g., shared
    /// fallback chains between terminal and UI fonts).
    pub fn load_cached(
        family: Option<&str>,
        weight: u16,
        cache: &mut FontByteCache,
    ) -> Result<Self, FontError> {
        let result = discovery::discover_fonts(family, weight);
        Self::from_discovery(&result, cache)
    }

    /// Prepend user-configured fallback fonts before system-discovered fallbacks.
    ///
    /// Each family name is resolved via platform font discovery. Unresolvable
    /// families are logged and skipped. Returns a mapping from loaded fallback
    /// index to original config index, so callers can apply per-fallback
    /// metadata to the correct loaded font even when some config entries fail.
    /// The `.len()` of the returned vec is the loaded user fallback count.
    pub fn prepend_user_fallbacks(
        &mut self,
        families: &[&str],
        cache: &mut FontByteCache,
    ) -> Vec<usize> {
        let mut user_fonts = Vec::new();
        let mut config_index_map = Vec::new();
        for (config_idx, family) in families.iter().enumerate() {
            match discovery::resolve_user_fallback(family) {
                Some(fb) => match cache.load(&fb.path) {
                    Ok(data) => {
                        log::info!("font: loaded user fallback {family:?}");
                        user_fonts.push(FontData {
                            data,
                            index: fb.face_index,
                        });
                        config_index_map.push(config_idx);
                    }
                    Err(e) => {
                        log::warn!("font: failed to load user fallback {family:?}: {e}");
                    }
                },
                None => {
                    log::warn!("font: user fallback {family:?} not found, skipping");
                }
            }
        }
        // Prepend user fallbacks: they take priority over system fallbacks.
        user_fonts.append(&mut self.fallbacks);
        self.fallbacks = user_fonts;
        config_index_map
    }

    /// Build a `FontSet` from a discovery result, using `cache` for file reads.
    pub(crate) fn from_discovery(
        result: &discovery::DiscoveryResult,
        cache: &mut FontByteCache,
    ) -> Result<Self, FontError> {
        let primary = &result.primary;

        let regular = load_font_data(primary, 0, cache)?;

        let bold = try_load_variant(primary, 1, "Bold", cache);
        let italic = try_load_variant(primary, 2, "Italic", cache);
        let bold_italic = try_load_variant(primary, 3, "BoldItalic", cache);

        let fallbacks = result
            .fallbacks
            .iter()
            .filter_map(|fb| {
                let data = match cache.load(&fb.path) {
                    Ok(d) => d,
                    Err(e) => {
                        log::warn!("font: failed to load fallback {}: {e}", fb.path.display());
                        return None;
                    }
                };
                Some(FontData {
                    data,
                    index: fb.face_index,
                })
            })
            .collect();

        Ok(Self {
            family_name: primary.family_name.clone(),
            regular,
            bold,
            italic,
            bold_italic,
            medium: None,
            has_variant: primary.has_variant,
            fallbacks,
        })
    }
}

/// Try to load a primary variant, logging on failure.
///
/// Returns `None` if the variant has no file or if loading fails (with a warning).
fn try_load_variant(
    primary: &discovery::FamilyDiscovery,
    slot: usize,
    name: &str,
    cache: &mut FontByteCache,
) -> Option<FontData> {
    if !primary.has_variant[slot] {
        return None;
    }
    match load_font_data(primary, slot, cache) {
        Ok(fd) => Some(fd),
        Err(e) => {
            log::warn!("font: failed to load {name} variant: {e}");
            None
        }
    }
}

/// Load font data for a style slot from a discovery result.
fn load_font_data(
    primary: &discovery::FamilyDiscovery,
    slot: usize,
    cache: &mut FontByteCache,
) -> Result<FontData, FontError> {
    let data = if let Some(ref path) = primary.paths[slot] {
        cache.load(path)?
    } else if primary.origin == FontOrigin::Embedded && slot == 0 {
        Arc::new(discovery::EMBEDDED_FONT_DATA.to_vec())
    } else {
        return Err(FontError::InvalidFont(format!(
            "no font data for slot {slot}"
        )));
    };
    Ok(FontData {
        data,
        index: primary.face_indices[slot],
    })
}
