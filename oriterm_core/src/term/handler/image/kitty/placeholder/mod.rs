//! Kitty unicode-placeholder (`U=1`) cell decoding.
//!
//! Encoding per kitty graphics-protocol.rst:
//! - `U+10EEEE` base codepoint marks a placeholder cell.
//! - 1st combining diacritic encodes the 0-indexed image row.
//! - 2nd combining diacritic encodes the 0-indexed image column.
//! - 3rd combining diacritic encodes the high byte of `image_id`.
//! - Cell foreground color carries the low 24 bits of `image_id`.
//! - Cell underline color (when non-zero) carries the `placement_id`.
//!
//! Continuation rules (left-cell-to-right within the same grid row):
//! - With no diacritics: inherit row + col+1 + high from the previous
//!   placeholder cell IFF same `image_id` low bits + same `placement_id`.
//! - With only the row diacritic: inherit col+1 + high from the previous
//!   placeholder cell IFF same row + same `image_id` low + same placement.
//! - With row + col diacritics: inherit high from the previous placeholder
//!   cell IFF same image/placement and col == prev.col + 1.
//!
//! Reference: `graphics_unicode.zig` in ghostty (functions `IncompletePlacement::init`,
//! `canAppend`, and `complete`).

use vte::ansi::Color;

/// 297 diacritic codepoints used by kitty's unicode-placeholder protocol,
/// sorted ascending so binary search yields a 0-indexed value.
///
/// Source: kitty `gen/rowcolumn-diacritics.txt`. The position in this array
/// equals the row/column value the diacritic represents.
pub(super) static DIACRITICS: [char; 297] = [
    '\u{0305}',
    '\u{030D}',
    '\u{030E}',
    '\u{0310}',
    '\u{0312}',
    '\u{033D}',
    '\u{033E}',
    '\u{033F}',
    '\u{0346}',
    '\u{034A}',
    '\u{034B}',
    '\u{034C}',
    '\u{0350}',
    '\u{0351}',
    '\u{0352}',
    '\u{0357}',
    '\u{035B}',
    '\u{0363}',
    '\u{0364}',
    '\u{0365}',
    '\u{0366}',
    '\u{0367}',
    '\u{0368}',
    '\u{0369}',
    '\u{036A}',
    '\u{036B}',
    '\u{036C}',
    '\u{036D}',
    '\u{036E}',
    '\u{036F}',
    '\u{0483}',
    '\u{0484}',
    '\u{0485}',
    '\u{0486}',
    '\u{0487}',
    '\u{0592}',
    '\u{0593}',
    '\u{0594}',
    '\u{0595}',
    '\u{0597}',
    '\u{0598}',
    '\u{0599}',
    '\u{059C}',
    '\u{059D}',
    '\u{059E}',
    '\u{059F}',
    '\u{05A0}',
    '\u{05A1}',
    '\u{05A8}',
    '\u{05A9}',
    '\u{05AB}',
    '\u{05AC}',
    '\u{05AF}',
    '\u{05C4}',
    '\u{0610}',
    '\u{0611}',
    '\u{0612}',
    '\u{0613}',
    '\u{0614}',
    '\u{0615}',
    '\u{0616}',
    '\u{0617}',
    '\u{0657}',
    '\u{0658}',
    '\u{0659}',
    '\u{065A}',
    '\u{065B}',
    '\u{065D}',
    '\u{065E}',
    '\u{06D6}',
    '\u{06D7}',
    '\u{06D8}',
    '\u{06D9}',
    '\u{06DA}',
    '\u{06DB}',
    '\u{06DC}',
    '\u{06DF}',
    '\u{06E0}',
    '\u{06E1}',
    '\u{06E2}',
    '\u{06E4}',
    '\u{06E7}',
    '\u{06E8}',
    '\u{06EB}',
    '\u{06EC}',
    '\u{0730}',
    '\u{0732}',
    '\u{0733}',
    '\u{0735}',
    '\u{0736}',
    '\u{073A}',
    '\u{073D}',
    '\u{073F}',
    '\u{0740}',
    '\u{0741}',
    '\u{0743}',
    '\u{0745}',
    '\u{0747}',
    '\u{0749}',
    '\u{074A}',
    '\u{07EB}',
    '\u{07EC}',
    '\u{07ED}',
    '\u{07EE}',
    '\u{07EF}',
    '\u{07F0}',
    '\u{07F1}',
    '\u{07F3}',
    '\u{0816}',
    '\u{0817}',
    '\u{0818}',
    '\u{0819}',
    '\u{081B}',
    '\u{081C}',
    '\u{081D}',
    '\u{081E}',
    '\u{081F}',
    '\u{0820}',
    '\u{0821}',
    '\u{0822}',
    '\u{0823}',
    '\u{0825}',
    '\u{0826}',
    '\u{0827}',
    '\u{0829}',
    '\u{082A}',
    '\u{082B}',
    '\u{082C}',
    '\u{082D}',
    '\u{0951}',
    '\u{0953}',
    '\u{0954}',
    '\u{0F82}',
    '\u{0F83}',
    '\u{0F86}',
    '\u{0F87}',
    '\u{135D}',
    '\u{135E}',
    '\u{135F}',
    '\u{17DD}',
    '\u{193A}',
    '\u{1A17}',
    '\u{1A75}',
    '\u{1A76}',
    '\u{1A77}',
    '\u{1A78}',
    '\u{1A79}',
    '\u{1A7A}',
    '\u{1A7B}',
    '\u{1A7C}',
    '\u{1B6B}',
    '\u{1B6D}',
    '\u{1B6E}',
    '\u{1B6F}',
    '\u{1B70}',
    '\u{1B71}',
    '\u{1B72}',
    '\u{1B73}',
    '\u{1CD0}',
    '\u{1CD1}',
    '\u{1CD2}',
    '\u{1CDA}',
    '\u{1CDB}',
    '\u{1CE0}',
    '\u{1DC0}',
    '\u{1DC1}',
    '\u{1DC3}',
    '\u{1DC4}',
    '\u{1DC5}',
    '\u{1DC6}',
    '\u{1DC7}',
    '\u{1DC8}',
    '\u{1DC9}',
    '\u{1DCB}',
    '\u{1DCC}',
    '\u{1DD1}',
    '\u{1DD2}',
    '\u{1DD3}',
    '\u{1DD4}',
    '\u{1DD5}',
    '\u{1DD6}',
    '\u{1DD7}',
    '\u{1DD8}',
    '\u{1DD9}',
    '\u{1DDA}',
    '\u{1DDB}',
    '\u{1DDC}',
    '\u{1DDD}',
    '\u{1DDE}',
    '\u{1DDF}',
    '\u{1DE0}',
    '\u{1DE1}',
    '\u{1DE2}',
    '\u{1DE3}',
    '\u{1DE4}',
    '\u{1DE5}',
    '\u{1DE6}',
    '\u{1DFE}',
    '\u{20D0}',
    '\u{20D1}',
    '\u{20D4}',
    '\u{20D5}',
    '\u{20D6}',
    '\u{20D7}',
    '\u{20DB}',
    '\u{20DC}',
    '\u{20E1}',
    '\u{20E7}',
    '\u{20E9}',
    '\u{20F0}',
    '\u{2CEF}',
    '\u{2CF0}',
    '\u{2CF1}',
    '\u{2DE0}',
    '\u{2DE1}',
    '\u{2DE2}',
    '\u{2DE3}',
    '\u{2DE4}',
    '\u{2DE5}',
    '\u{2DE6}',
    '\u{2DE7}',
    '\u{2DE8}',
    '\u{2DE9}',
    '\u{2DEA}',
    '\u{2DEB}',
    '\u{2DEC}',
    '\u{2DED}',
    '\u{2DEE}',
    '\u{2DEF}',
    '\u{2DF0}',
    '\u{2DF1}',
    '\u{2DF2}',
    '\u{2DF3}',
    '\u{2DF4}',
    '\u{2DF5}',
    '\u{2DF6}',
    '\u{2DF7}',
    '\u{2DF8}',
    '\u{2DF9}',
    '\u{2DFA}',
    '\u{2DFB}',
    '\u{2DFC}',
    '\u{2DFD}',
    '\u{2DFE}',
    '\u{2DFF}',
    '\u{A66F}',
    '\u{A67C}',
    '\u{A67D}',
    '\u{A6F0}',
    '\u{A6F1}',
    '\u{A8E0}',
    '\u{A8E1}',
    '\u{A8E2}',
    '\u{A8E3}',
    '\u{A8E4}',
    '\u{A8E5}',
    '\u{A8E6}',
    '\u{A8E7}',
    '\u{A8E8}',
    '\u{A8E9}',
    '\u{A8EA}',
    '\u{A8EB}',
    '\u{A8EC}',
    '\u{A8ED}',
    '\u{A8EE}',
    '\u{A8EF}',
    '\u{A8F0}',
    '\u{A8F1}',
    '\u{AAB0}',
    '\u{AAB2}',
    '\u{AAB3}',
    '\u{AAB7}',
    '\u{AAB8}',
    '\u{AABE}',
    '\u{AABF}',
    '\u{AAC1}',
    '\u{FE20}',
    '\u{FE21}',
    '\u{FE22}',
    '\u{FE23}',
    '\u{FE24}',
    '\u{FE25}',
    '\u{FE26}',
    '\u{10A0F}',
    '\u{10A38}',
    '\u{1D185}',
    '\u{1D186}',
    '\u{1D187}',
    '\u{1D188}',
    '\u{1D189}',
    '\u{1D1AA}',
    '\u{1D1AB}',
    '\u{1D1AC}',
    '\u{1D1AD}',
    '\u{1D242}',
    '\u{1D243}',
    '\u{1D244}',
];

/// Resolve a diacritic codepoint to its 0-indexed row/column value.
///
/// Returns `None` when the codepoint is not in the kitty diacritic table.
pub fn diacritic_to_index(cp: char) -> Option<u32> {
    DIACRITICS
        .binary_search(&cp)
        .ok()
        .map(|i| u32::try_from(i).unwrap_or(0))
}

/// Convert a `vte::ansi::Color` into the 24-bit image-ID component the
/// kitty protocol encodes in fg / underline.
///
/// `Named` colors carry no useful bits → 0; `Indexed` carries the palette
/// index (always fits in 8 bits); `Spec` packs the RGB triplet into 24 bits.
pub fn color_to_id(c: Color) -> u32 {
    match c {
        Color::Named(_) => 0,
        Color::Indexed(v) => u32::from(v),
        Color::Spec(rgb) => (u32::from(rgb.r) << 16) | (u32::from(rgb.g) << 8) | u32::from(rgb.b),
    }
}

/// Per-cell placement state decoded from a placeholder cell's diacritics
/// and colors. "Incomplete" because diacritics are optional; continuation
/// rules fill missing fields from the preceding cell in the same run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IncompletePlacement {
    /// Low 24 bits of `image_id`, sourced from the cell's fg color.
    pub image_id_low: u32,
    /// High byte of `image_id`, sourced from the 3rd diacritic when present.
    pub image_id_high: Option<u8>,
    /// `placement_id`, sourced from the cell's underline color (0 → None).
    pub placement_id: Option<u32>,
    /// Image-row index from the 1st diacritic.
    pub row: Option<u32>,
    /// Image-column index from the 2nd diacritic.
    pub col: Option<u32>,
}

impl IncompletePlacement {
    /// Decode the diacritic + color encoding from a single placeholder cell.
    ///
    /// `zerowidth` is the cell's combining marks (up to 3 are consulted —
    /// row, column, high-byte). `fg` is the cell foreground color, carrying
    /// the low 24 bits of `image_id`. `ul` is the optional underline color;
    /// when present and non-zero it carries the `placement_id`.
    ///
    /// Invalid diacritics at any position decode as `None` for that field
    /// (matching ghostty's behavior — undocumented in the kitty spec but
    /// the canonical reference treats invalid diacritics as missing).
    pub fn decode(zerowidth: &[char], fg: Color, ul: Option<Color>) -> Self {
        let image_id_low = color_to_id(fg);

        let placement_id = ul.and_then(|c| {
            let id = color_to_id(c);
            if id == 0 { None } else { Some(id) }
        });

        let row = zerowidth.first().and_then(|c| diacritic_to_index(*c));
        let col = zerowidth.get(1).and_then(|c| diacritic_to_index(*c));
        let image_id_high = zerowidth
            .get(2)
            .and_then(|c| diacritic_to_index(*c))
            .and_then(|v| u8::try_from(v).ok());

        Self {
            image_id_low,
            image_id_high,
            placement_id,
            row,
            col,
        }
    }

    /// Resolve missing fields using the preceding placeholder cell in the
    /// same grid row, applying kitty's run-continuation rules.
    ///
    /// `prev` is the resolved state of the immediately-preceding placeholder
    /// cell (`None` if the previous grid cell was not a placeholder or had
    /// no inheritable state). Inheritance only fires when `image_id_low` and
    /// `placement_id` match the previous cell — distinct colors break the
    /// run.
    pub fn resolve_with_continuation(
        &self,
        prev: Option<&ResolvedPlaceholder>,
    ) -> ResolvedPlaceholder {
        let inherit = prev.filter(|r| {
            r.image_id_low == self.image_id_low && r.placement_id == self.placement_id.unwrap_or(0)
        });

        let row = self.row.or_else(|| inherit.map(|r| r.image_row));

        // Column continuation: explicit col wins; otherwise inherit
        // `prev.col + 1` when the run is intact. When there is no
        // preceding placeholder cell, default to 0 — a bare placeholder
        // is the first cell of a fresh run.
        let col = if let Some(c) = self.col {
            c
        } else if let Some(r) = inherit {
            r.image_col + 1
        } else {
            0
        };

        let image_id_high = self
            .image_id_high
            .or_else(|| inherit.and_then(|r| r.image_id_high));

        let image_id = self.image_id_low | (u32::from(image_id_high.unwrap_or(0)) << 24);

        let placement_id = self.placement_id.unwrap_or(0);

        ResolvedPlaceholder {
            image_id,
            placement_id,
            image_row: row.unwrap_or(0),
            image_col: col,
            image_id_low: self.image_id_low,
            image_id_high,
        }
    }
}

/// Resolved placeholder cell — every field filled after continuation.
///
/// `image_id_low` and `image_id_high` are carried so the next cell in the
/// same grid row can inherit them via `resolve_with_continuation`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedPlaceholder {
    /// Full image ID = `image_id_low | (image_id_high.unwrap_or(0) << 24)`.
    pub image_id: u32,
    /// Placement ID (0 when the cell does not carry one).
    pub placement_id: u32,
    /// Image-row index for this cell.
    pub image_row: u32,
    /// Image-column index for this cell.
    pub image_col: u32,
    /// Low 24 bits of `image_id` (for continuation comparison).
    pub image_id_low: u32,
    /// High byte of `image_id` (for continuation inheritance).
    pub image_id_high: Option<u8>,
}

#[cfg(test)]
mod tests;
