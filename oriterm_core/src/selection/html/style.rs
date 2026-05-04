//! CSS style mapping for HTML clipboard export.
//!
//! Owns [`CellStyle`] (the per-cell resolved-style sidecar used to drive
//! `<span>` coalescing) and the two style-axis enums [`UnderlineKind`] and
//! [`VerticalAlign`]. The struct and `write_css` are extracted from
//! `html/mod.rs` to keep the parent module under the 500-line limit.

use std::fmt::Write;

use vte::ansi::Rgb;

use crate::cell::CellFlags;

use super::HtmlCtx;

/// Resolved cell style for HTML output.
///
/// Bools map directly to CSS properties (bold, italic, strikethrough, dim,
/// overline). [`UnderlineKind`] and [`VerticalAlign`] each carry a single
/// CSS-axis value because their variants are mutually exclusive.
#[allow(
    clippy::struct_excessive_bools,
    reason = "1:1 mapping to CSS properties"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) struct CellStyle {
    pub(super) fg: Option<Rgb>,
    pub(super) bg: Option<Rgb>,
    pub(super) bold: bool,
    pub(super) italic: bool,
    pub(super) underline: UnderlineKind,
    pub(super) underline_color: Option<Rgb>,
    pub(super) strikethrough: bool,
    pub(super) overline: bool,
    pub(super) vertical_align: VerticalAlign,
    pub(super) dim: bool,
}

/// Underline variant for CSS mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum UnderlineKind {
    #[default]
    None,
    Single,
    Double,
    Curly,
    Dotted,
    Dashed,
}

/// Vertical-alignment variant for SGR 73 (superscript) and SGR 74 (subscript).
///
/// Mutually exclusive — `super` and `sub` cannot be set together. The SGR
/// handler in `term/handler/sgr.rs` enforces this by clearing the opposite flag
/// when one is set, so `from_cell` never sees both at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum VerticalAlign {
    #[default]
    None,
    Super,
    Sub,
}

impl CellStyle {
    /// Build a style from a terminal cell.
    pub(super) fn from_cell(cell: &crate::cell::Cell, ctx: &HtmlCtx<'_>) -> Self {
        let flags = cell.flags;
        let mut fg = ctx.palette.resolve(cell.fg);
        let mut bg = ctx.palette.resolve(cell.bg);

        if flags.contains(CellFlags::INVERSE) {
            std::mem::swap(&mut fg, &mut bg);
        }

        let vertical_align = if flags.contains(CellFlags::SUPERSCRIPT) {
            VerticalAlign::Super
        } else if flags.contains(CellFlags::SUBSCRIPT) {
            VerticalAlign::Sub
        } else {
            VerticalAlign::None
        };

        Self {
            fg: if fg == ctx.default_fg { None } else { Some(fg) },
            bg: if bg == ctx.default_bg { None } else { Some(bg) },
            bold: flags.contains(CellFlags::BOLD),
            italic: flags.contains(CellFlags::ITALIC),
            underline: if flags.contains(CellFlags::DOUBLE_UNDERLINE) {
                UnderlineKind::Double
            } else if flags.contains(CellFlags::CURLY_UNDERLINE) {
                UnderlineKind::Curly
            } else if flags.contains(CellFlags::DOTTED_UNDERLINE) {
                UnderlineKind::Dotted
            } else if flags.contains(CellFlags::DASHED_UNDERLINE) {
                UnderlineKind::Dashed
            } else if flags.contains(CellFlags::UNDERLINE) {
                UnderlineKind::Single
            } else {
                UnderlineKind::None
            },
            underline_color: cell
                .extra
                .as_ref()
                .and_then(|e| e.underline_color)
                .map(|c| ctx.palette.resolve(c)),
            strikethrough: flags.contains(CellFlags::STRIKETHROUGH),
            overline: flags.contains(CellFlags::OVERLINE),
            vertical_align,
            dim: flags.contains(CellFlags::DIM),
        }
    }

    /// Returns true if this style matches the terminal defaults (no styling needed).
    pub(super) fn is_default(&self) -> bool {
        self.fg.is_none()
            && self.bg.is_none()
            && !self.bold
            && !self.italic
            && self.underline == UnderlineKind::None
            && self.underline_color.is_none()
            && !self.strikethrough
            && !self.overline
            && self.vertical_align == VerticalAlign::None
            && !self.dim
    }

    /// Write CSS properties into `buf`.
    pub(super) fn write_css(&self, buf: &mut String) {
        if let Some(fg) = self.fg {
            let _ = write!(buf, "color:#{:02x}{:02x}{:02x};", fg.r, fg.g, fg.b);
        }
        if let Some(bg) = self.bg {
            let _ = write!(
                buf,
                "background-color:#{:02x}{:02x}{:02x};",
                bg.r, bg.g, bg.b
            );
        }
        if self.bold {
            buf.push_str("font-weight:bold;");
        }
        if self.italic {
            buf.push_str("font-style:italic;");
        }
        if self.dim {
            buf.push_str("opacity:0.5;");
        }

        // Build text-decoration value directly into `buf` — no intermediate
        // `Vec<&str>` allocation. `decoration_count` tracks whether we've
        // emitted any tokens yet so we know when to insert the leading
        // `text-decoration:` and the inter-token spaces.
        let mut decoration_count = 0usize;
        let mut emit_dec = |buf: &mut String, token: &str| {
            if decoration_count == 0 {
                buf.push_str("text-decoration:");
            } else {
                buf.push(' ');
            }
            buf.push_str(token);
            decoration_count += 1;
        };
        match self.underline {
            UnderlineKind::None => {}
            UnderlineKind::Single => emit_dec(buf, "underline"),
            UnderlineKind::Double => emit_dec(buf, "underline double"),
            UnderlineKind::Curly => emit_dec(buf, "underline wavy"),
            UnderlineKind::Dotted => emit_dec(buf, "underline dotted"),
            UnderlineKind::Dashed => emit_dec(buf, "underline dashed"),
        }
        if self.strikethrough {
            emit_dec(buf, "line-through");
        }
        if self.overline {
            emit_dec(buf, "overline");
        }
        if decoration_count > 0 {
            buf.push(';');
            // Note: when underline_color is set AND overline is also set,
            // CSS applies the same color to BOTH decorations (single
            // text-decoration-color per element). The GPU renders overline
            // in fg unconditionally; the divergence is a known minor drift
 // tracked in §3.
            if let Some(uc) = self.underline_color {
                let _ = write!(
                    buf,
                    "text-decoration-color:#{:02x}{:02x}{:02x};",
                    uc.r, uc.g, uc.b
                );
            }
        }
        match self.vertical_align {
            VerticalAlign::None => {}
            // No font-size shrink: glyph-size reduction breaks the monospace
            // grid inside <pre> AND diverges from the GPU which renders
 // vertical offset only (no shrink). See fix- §1.5.
            VerticalAlign::Super => buf.push_str("vertical-align:super;"),
            VerticalAlign::Sub => buf.push_str("vertical-align:sub;"),
        }
    }
}
