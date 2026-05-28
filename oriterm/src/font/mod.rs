//! Font management: discovery, loading, and rasterization.
//!
//! This module handles finding font files on disk across platforms, loading
//! them into memory, and rasterizing glyphs for the GPU renderer.
//!
//! # Architecture
//!
//! - [`discovery`] resolves family names and style variants to file paths.
//! - [`collection`] loads font bytes, computes cell metrics, and rasterizes
//!   glyphs into bitmaps for atlas upload.

pub(crate) mod collection;
pub(crate) mod discovery;
mod metrics;
pub(crate) mod shaper;
pub(crate) mod ui_font_sizes;

use std::fmt;

use bitflags::bitflags;

pub(crate) use collection::parse_features;
pub(crate) use collection::parse_hex_range;
pub(crate) use collection::{FontByteCache, FontCollection, FontSet, RasterizedGlyph, size_key};
pub use metrics::{
    CellMetrics, FontRasterConfig, GlyphFormat, HintingMode, StrokeMetrics, SubpixelMode,
};
pub(crate) use shaper::{
    CachedTextMeasurer, ShapeFaces, ShapeSink, ShapingRun, TextShapeCache, UiFontMeasurer,
    build_col_glyph_map, prepare_line, shape_prepared_runs,
};
pub(crate) use ui_font_sizes::UiFontSizes;

/// Font style for face selection.
///
/// Discriminant values match the primary face array indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GlyphStyle {
    /// Normal weight, upright.
    Regular = 0,
    /// Bold weight, upright.
    Bold = 1,
    /// Normal weight, italic/oblique.
    Italic = 2,
    /// Bold weight, italic/oblique.
    BoldItalic = 3,
}

impl GlyphStyle {
    /// Derive the glyph style from cell attribute flags.
    pub fn from_cell_flags(flags: oriterm_core::CellFlags) -> Self {
        let bold = flags.contains(oriterm_core::CellFlags::BOLD);
        let italic = flags.contains(oriterm_core::CellFlags::ITALIC);
        match (bold, italic) {
            (true, true) => Self::BoldItalic,
            (true, false) => Self::Bold,
            (false, true) => Self::Italic,
            (false, false) => Self::Regular,
        }
    }
}

/// Compact face index into the font collection.
///
/// Indices 0–3 map to primary style variants (Regular, Bold, Italic, `BoldItalic`).
/// Indices 4+ map to fallback fonts in priority order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FaceIdx(pub u16);

impl FaceIdx {
    /// Regular primary face.
    pub const REGULAR: Self = Self(0);

    /// Number of primary style slots (Regular, Bold, Italic, `BoldItalic`).
    ///
    /// Fallback font indices start at this offset.
    pub const PRIMARY_COUNT: u16 = 4;

    /// Sentinel for built-in geometric glyphs (box drawing, blocks, braille, powerline).
    ///
    /// These glyphs are rasterized from cell dimensions, not from any font face.
    pub const BUILTIN: Self = Self(u16::MAX);

    /// Construct a `FaceIdx` from a zero-based fallback index.
    ///
    /// Fallback 0 maps to `FaceIdx(4)`, fallback 1 to `FaceIdx(5)`, etc.
    pub fn from_fallback_index(idx: usize) -> Self {
        Self(idx as u16 + Self::PRIMARY_COUNT)
    }

    /// Whether this is the Bold (1) or `BoldItalic` (3) primary face slot.
    ///
    /// These faces are inherently bold — adding synthetic bold would double-embolden.
    pub fn is_bold_primary(self) -> bool {
        self.0 == 1 || self.0 == 3
    }

    /// Whether this index refers to a fallback font (index >= `PRIMARY_COUNT`).
    pub fn is_fallback(self) -> bool {
        self.0 >= Self::PRIMARY_COUNT && self != Self::BUILTIN
    }

    /// Convert to `usize` for array indexing.
    pub fn as_usize(self) -> usize {
        self.0 as usize
    }

    /// Fallback index (zero-based) for indexing into the fallback array.
    ///
    /// Returns `None` if this is a primary face.
    pub fn fallback_index(self) -> Option<usize> {
        if self.is_fallback() {
            Some(self.0 as usize - Self::PRIMARY_COUNT as usize)
        } else {
            None
        }
    }
}

/// Distinguishes terminal grid fonts from UI fonts in atlas cache keys.
///
/// Terminal and UI text may use different font collections at different sizes.
/// Including the realm in [`RasterKey`] ensures glyphs from different
/// collections never collide in the atlas cache, even if they share the
/// same glyph ID and face index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(u8)]
pub enum FontRealm {
    /// Terminal grid text (monospace).
    #[default]
    Terminal = 0,
    /// UI overlay text (tab bar, labels, dialogs).
    Ui = 1,
}

/// Cache key for rasterized glyphs — glyph-ID-based, not character-based.
///
/// The `size_q6` field encodes size in 26.6 fixed-point: `(size_px * 64.0).round() as u32`.
/// This avoids floating-point hashing while preserving sub-pixel size changes.
///
/// Includes [`SyntheticFlags`] so that emboldened/skewed glyphs are cached
/// separately from their unsynthesized counterparts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RasterKey {
    /// Glyph ID within the font face (or codepoint for built-in glyphs).
    ///
    /// `u32` to support Supplementary Multilingual Plane codepoints used as
    /// built-in glyph IDs (e.g. Symbols for Legacy Computing, U+1FB00+).
    pub glyph_id: u32,
    /// Which font face this glyph belongs to.
    pub face_idx: FaceIdx,
    /// Requested font weight for UI text (CSS 100–900).
    ///
    /// Terminal grid text always uses `0` (weight is implicit in the face slot).
    /// UI text carries the requested weight so different weight requests produce
    /// distinct atlas entries.
    pub weight: u16,
    /// Size in 26.6 fixed-point: `(size_px * 64.0).round() as u32`.
    pub size_q6: u32,
    /// Synthetic transformations applied at rasterization time.
    pub synthetic: SyntheticFlags,
    /// Whether this glyph was rasterized with hinting enabled.
    pub hinted: bool,
    /// Horizontal subpixel phase (0–3). See [`subpx_bin`].
    pub subpx_x: u8,
    /// Which font realm this glyph belongs to (terminal vs UI).
    pub font_realm: FontRealm,
}

impl RasterKey {
    /// Construct a raster key from a resolved glyph, size, hinting, and subpixel phase.
    ///
    /// Defaults to [`FontRealm::Terminal`]. Use [`with_realm`](Self::with_realm)
    /// for UI text glyphs.
    pub fn from_resolved(resolved: ResolvedGlyph, size_q6: u32, hinted: bool, subpx_x: u8) -> Self {
        Self {
            glyph_id: u32::from(resolved.glyph_id),
            face_idx: resolved.face_idx,
            weight: 0,
            size_q6,
            synthetic: resolved.synthetic,
            hinted,
            subpx_x,
            font_realm: FontRealm::Terminal,
        }
    }

    /// Return a copy with the given font realm.
    #[must_use]
    pub fn with_realm(mut self, realm: FontRealm) -> Self {
        self.font_realm = realm;
        self
    }
}

/// Quantize a fractional pixel offset to one of 4 horizontal phases.
///
/// Phases: 0 → 0.00, 1 → 0.25, 2 → 0.50, 3 → 0.75.
/// Grid text at integer boundaries always returns 0.
pub fn subpx_bin(offset: f32) -> u8 {
    let fract = offset.fract().abs();
    // 4 bins centered at 0.00, 0.25, 0.50, 0.75 with boundaries at
    // 0.125, 0.375, 0.625, 0.875.
    match (fract * 4.0 + 0.5) as u8 {
        1 => 1,
        2 => 2,
        3 => 3,
        _ => 0, // 0, 4+ (0.875+ wraps to next integer) → phase 0
    }
}

/// Convert a subpixel bin (0–3) back to a fractional offset for rasterization.
pub fn subpx_offset(bin: u8) -> f32 {
    match bin {
        1 => 0.25,
        2 => 0.50,
        3 => 0.75,
        _ => 0.0, // 0 and out-of-range
    }
}

bitflags! {
 /// Flags indicating synthetic style transformations needed at render time.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct SyntheticFlags: u8 {
 /// No synthetic transformations.
        const NONE   = 0;
 /// Synthetic emboldening needed (no real bold variant).
        const BOLD   = 0b01;
 /// Synthetic slant needed (no real italic variant).
        const ITALIC = 0b10;
    }
}

/// Result of resolving a character to a font face and glyph ID.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedGlyph {
    /// Glyph ID within the font face.
    pub glyph_id: u16,
    /// Which font face resolved this character.
    pub face_idx: FaceIdx,
    /// Whether synthetic style transformations are needed.
    pub synthetic: SyntheticFlags,
}

/// Font loading and validation errors.
#[derive(Debug)]
pub enum FontError {
    /// Font data is invalid or could not be parsed.
    InvalidFont(String),
    /// I/O error reading a font file.
    Io(std::io::Error),
}

impl fmt::Display for FontError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFont(msg) => write!(f, "invalid font: {msg}"),
            Self::Io(err) => write!(f, "font I/O error: {err}"),
        }
    }
}

impl std::error::Error for FontError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::InvalidFont(_) => None,
        }
    }
}

impl From<std::io::Error> for FontError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

/// Whether a character should be rendered as a built-in geometric glyph.
///
/// O(1) range match covering box drawing, block elements, braille patterns,
/// powerline symbols, and Symbols for Legacy Computing. Lives here (not in
/// `gpu::builtin_glyphs`) because the font shaper needs it to skip built-in
/// chars during run segmentation, and the font module must not depend on the
/// GPU module.
pub(crate) fn is_builtin(ch: char) -> bool {
    matches!(
        ch,
        '\u{2500}'..='\u{257F}'     // Box Drawing
        | '\u{2580}'..='\u{259F}'   // Block Elements
        | '\u{2800}'..='\u{28FF}'   // Braille Patterns
        | '\u{E0B0}'..='\u{E0B4}'   // Powerline separators (solid + outline triangles)
        | '\u{E0B6}'                // Powerline left rounded separator
        | '\u{F5D0}'..='\u{F60D}'   // Branch drawing (Kitty/Ghostty PUA)
        | '\u{1FB00}'..='\u{1FB9F}' // Symbols for Legacy Computing
        | '\u{1CD00}'..='\u{1CDE5}' // Symbols for Legacy Computing Supplement (octants)
    ) || is_builtin_geometric(ch)
}

/// Subset of Geometric Shapes (U+25A0–U+25FF) rendered as built-in glyphs.
///
/// Only codepoints with actual built-in rendering are listed here; the rest
/// fall through to font-based rendering.
fn is_builtin_geometric(ch: char) -> bool {
    matches!(
        ch,
        '\u{25A0}'..='\u{25A3}' // Squares (filled, outlined, nested)
        | '\u{25AA}'..='\u{25AB}' // Small squares
        | '\u{25B2}'..='\u{25C5}' // Triangles (up, right, down, left — filled + outlined)
        | '\u{25C6}'..='\u{25CB}' // Diamonds, fisheye, lozenge, white circle
        | '\u{25CE}'..='\u{25CF}' // Bullseye, black circle
        | '\u{25D0}'..='\u{25D3}' // Half circles
        | '\u{25E2}'..='\u{25E5}' // Corner triangles (filled)
        | '\u{25EF}'             // Large circle
        | '\u{25F8}'..='\u{25FB}' // Corner triangle outlines + medium white square
        | '\u{25FC}'..='\u{25FF}' // Medium/small squares + corner outline
    )
}

#[cfg(test)]
mod tests;
