//! HTML text extraction from grid selection.
//!
//! Converts a `Selection` range into an HTML fragment with inline CSS styles
//! for foreground/background colors, bold, italic, underline, and strikethrough.
//! Used when `copy_formatting` is enabled so pasting into rich text editors
//! preserves terminal formatting.

use std::fmt::Write;

use crate::cell::CellFlags;
use crate::color::Palette;
use crate::grid::Grid;
use crate::image::KITTY_PLACEHOLDER;
use crate::index::Column;

use vte::ansi::{Color, NamedColor, Rgb};

use super::{Selection, SelectionMode};

/// Extract selected text as an HTML fragment with inline styles.
///
/// Returns a standalone HTML string with `<pre>` and `<span>` elements styled
/// via inline CSS. The font family and size are embedded in the `<pre>` style.
pub fn extract_html(
    grid: &Grid,
    selection: &Selection,
    palette: &Palette,
    font_family: &str,
    font_size_pt: f32,
) -> String {
    let (start, end) = selection.ordered();
    let Some(start_abs) = start.row.to_absolute(grid) else {
        return String::new();
    };
    let Some(end_abs) = end.row.to_absolute(grid) else {
        return String::new();
    };

    let ctx = HtmlCtx {
        palette,
        default_fg: palette.resolve(Color::Named(NamedColor::Foreground)),
        default_bg: palette.resolve(Color::Named(NamedColor::Background)),
    };

    let mut body = String::with_capacity(1024);

    if selection.mode == SelectionMode::Block {
        let min_col = start.col.min(end.col);
        let max_col = start.col.max(end.col);
        for abs_row in start_abs..=end_abs {
            if let Some(row) = grid.absolute_row(abs_row) {
                append_html_cells(&mut body, row, min_col, max_col, &ctx);
            }
            if abs_row < end_abs {
                body.push('\n');
            }
        }
    } else {
        for abs_row in start_abs..=end_abs {
            if let Some(row) = grid.absolute_row(abs_row) {
                let row_start = if abs_row == start_abs {
                    start.effective_start_col()
                } else {
                    0
                };
                let row_end = if abs_row == end_abs {
                    end.effective_end_col()
                } else {
                    row.cols().saturating_sub(1)
                };

                let last_col = row.cols().saturating_sub(1);
                let is_wrapped =
                    row.cols() > 0 && row[Column(last_col)].flags.contains(CellFlags::WRAP);

                append_html_cells(&mut body, row, row_start, row_end, &ctx);

                if !is_wrapped && abs_row < end_abs {
                    body.push('\n');
                }
            }
        }
    }

    // Trim trailing whitespace lines.
    let trimmed = body.trim_end();

    format!(
        "<pre style=\"font-family:'{font_family}',monospace;font-size:{font_size_pt:.1}pt\">\
         {trimmed}</pre>"
    )
}

/// Extract selected text as both HTML and plain text in a single pass.
///
/// Returns `(html, text)`. Combines the logic of [`extract_html`] and
/// [`super::text::extract_text`] to avoid iterating selected cells twice.
pub fn extract_html_with_text(
    grid: &Grid,
    selection: &Selection,
    palette: &Palette,
    font_family: &str,
    font_size_pt: f32,
) -> (String, String) {
    let (start, end) = selection.ordered();
    let Some(start_abs) = start.row.to_absolute(grid) else {
        return (String::new(), String::new());
    };
    let Some(end_abs) = end.row.to_absolute(grid) else {
        return (String::new(), String::new());
    };

    let ctx = HtmlCtx {
        palette,
        default_fg: palette.resolve(Color::Named(NamedColor::Foreground)),
        default_bg: palette.resolve(Color::Named(NamedColor::Background)),
    };

    let mut html_body = String::with_capacity(1024);
    let mut text = String::with_capacity(256);

    if selection.mode == SelectionMode::Block {
        let min_col = start.col.min(end.col);
        let max_col = start.col.max(end.col);
        for abs_row in start_abs..=end_abs {
            if let Some(row) = grid.absolute_row(abs_row) {
                let mark = text.len();
                append_cells_dual((&mut text, &mut html_body), row, min_col, max_col, &ctx);
                super::text::trim_trailing_whitespace(&mut text, mark);
            }
            if abs_row < end_abs {
                html_body.push('\n');
                text.push('\n');
            }
        }
    } else {
        for abs_row in start_abs..=end_abs {
            if let Some(row) = grid.absolute_row(abs_row) {
                let row_start = if abs_row == start_abs {
                    start.effective_start_col()
                } else {
                    0
                };
                let row_end = if abs_row == end_abs {
                    end.effective_end_col()
                } else {
                    row.cols().saturating_sub(1)
                };

                let last_col = row.cols().saturating_sub(1);
                let is_wrapped =
                    row.cols() > 0 && row[Column(last_col)].flags.contains(CellFlags::WRAP);

                let mark = text.len();
                append_cells_dual((&mut text, &mut html_body), row, row_start, row_end, &ctx);

                // Text: trim trailing whitespace unless this is a wrapped non-last row.
                if !(is_wrapped && abs_row < end_abs) {
                    super::text::trim_trailing_whitespace(&mut text, mark);
                }

                if !is_wrapped && abs_row < end_abs {
                    html_body.push('\n');
                    text.push('\n');
                }
            }
        }
    }

    let trimmed = html_body.trim_end();
    let html = format!(
        "<pre style=\"font-family:'{font_family}',monospace;font-size:{font_size_pt:.1}pt\">\
         {trimmed}</pre>"
    );
    (html, text)
}

/// Resolved palette and default colors for HTML generation.
struct HtmlCtx<'a> {
    palette: &'a Palette,
    default_fg: Rgb,
    default_bg: Rgb,
}

/// Append HTML-styled cell content from `col_start..=col_end`.
fn append_html_cells(
    buf: &mut String,
    row: &crate::grid::Row,
    col_start: usize,
    col_end: usize,
    ctx: &HtmlCtx<'_>,
) {
    let last = col_end.min(row.cols().saturating_sub(1));

    // Track current style to coalesce adjacent cells with identical formatting.
    let mut span_open = false;
    let mut cur_style = CellStyle::default();

    for col in col_start..=last {
        let cell = &row[Column(col)];

        if cell
            .flags
            .intersects(CellFlags::WIDE_CHAR_SPACER | CellFlags::LEADING_WIDE_CHAR_SPACER)
        {
            continue;
        }
        if cell.flags.contains(CellFlags::HIDDEN) {
            continue;
        }
        if cell.ch == KITTY_PLACEHOLDER {
            continue;
        }

        let style = CellStyle::from_cell(cell, ctx);
        let ch = if cell.ch == '\0' { ' ' } else { cell.ch };

        if style != cur_style {
            if span_open {
                buf.push_str("</span>");
            }
            if style.is_default() {
                span_open = false;
            } else {
                buf.push_str("<span style=\"");
                style.write_css(buf);
                buf.push_str("\">");
                span_open = true;
            }
            cur_style = style;
        }

        push_html_escaped(buf, ch);
        if let Some(extra) = &cell.extra {
            for &zw in &extra.zerowidth {
                buf.push(zw);
            }
        }
    }

    if span_open {
        buf.push_str("</span>");
    }
}

/// Append cells to both text and HTML buffers in a single pass.
///
/// `bufs` is `(text, html)`. Text output matches [`super::text::append_cells`]:
/// pushes all visible characters (including HIDDEN). HTML output matches
/// [`append_html_cells`]: skips HIDDEN cells and wraps styled runs in `<span>`.
fn append_cells_dual(
    bufs: (&mut String, &mut String),
    row: &crate::grid::Row,
    col_start: usize,
    col_end: usize,
    ctx: &HtmlCtx<'_>,
) {
    let (text_buf, html_buf) = bufs;
    let last = col_end.min(row.cols().saturating_sub(1));
    let mut span_open = false;
    let mut cur_style = CellStyle::default();

    for col in col_start..=last {
        let cell = &row[Column(col)];
        if cell
            .flags
            .intersects(CellFlags::WIDE_CHAR_SPACER | CellFlags::LEADING_WIDE_CHAR_SPACER)
        {
            continue;
        }
        if cell.ch == KITTY_PLACEHOLDER {
            continue;
        }

        let ch = if cell.ch == '\0' { ' ' } else { cell.ch };

        // Text: always include (matches extract_text behavior).
        text_buf.push(ch);
        if let Some(extra) = &cell.extra {
            for &zw in &extra.zerowidth {
                text_buf.push(zw);
            }
        }

        // HTML: skip HIDDEN cells.
        if cell.flags.contains(CellFlags::HIDDEN) {
            continue;
        }

        let style = CellStyle::from_cell(cell, ctx);
        if style != cur_style {
            if span_open {
                html_buf.push_str("</span>");
            }
            if style.is_default() {
                span_open = false;
            } else {
                html_buf.push_str("<span style=\"");
                style.write_css(html_buf);
                html_buf.push_str("\">");
                span_open = true;
            }
            cur_style = style;
        }

        push_html_escaped(html_buf, ch);
        if let Some(extra) = &cell.extra {
            for &zw in &extra.zerowidth {
                html_buf.push(zw);
            }
        }
    }

    if span_open {
        html_buf.push_str("</span>");
    }
}

/// Resolved cell style for HTML output.
///
/// Bools map directly to CSS properties (bold, italic, strikethrough, dim, overline).
#[allow(
    clippy::struct_excessive_bools,
    reason = "1:1 mapping to CSS properties"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct CellStyle {
    fg: Option<Rgb>,
    bg: Option<Rgb>,
    bold: bool,
    italic: bool,
    underline: UnderlineKind,
    underline_color: Option<Rgb>,
    strikethrough: bool,
    overline: bool,
    vertical_align: VerticalAlign,
    dim: bool,
}

/// Underline variant for CSS mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum UnderlineKind {
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
enum VerticalAlign {
    #[default]
    None,
    Super,
    Sub,
}

impl CellStyle {
    /// Build a style from a terminal cell.
    fn from_cell(cell: &crate::cell::Cell, ctx: &HtmlCtx<'_>) -> Self {
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
    fn is_default(&self) -> bool {
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
    fn write_css(&self, buf: &mut String) {
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

        // Build text-decoration via Vec join — scales linearly across
        // underline / strikethrough / overline rather than 12+ match arms.
        let mut decs: Vec<&str> = Vec::new();
        match self.underline {
            UnderlineKind::None => {}
            UnderlineKind::Single => decs.push("underline"),
            UnderlineKind::Double => decs.push("underline double"),
            UnderlineKind::Curly => decs.push("underline wavy"),
            UnderlineKind::Dotted => decs.push("underline dotted"),
            UnderlineKind::Dashed => decs.push("underline dashed"),
        }
        if self.strikethrough {
            decs.push("line-through");
        }
        if self.overline {
            decs.push("overline");
        }
        if !decs.is_empty() {
            buf.push_str("text-decoration:");
            buf.push_str(&decs.join(" "));
            buf.push(';');
            // Note: when underline_color is set AND overline is also set,
            // CSS applies the same color to BOTH decorations (single
            // text-decoration-color per element). The GPU renders overline
            // in fg unconditionally; the divergence is a known minor drift
            // tracked in BUG-06-014 §3.
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
            // vertical offset only (no shrink). See fix-BUG-06-014 §1.5.
            VerticalAlign::Super => buf.push_str("vertical-align:super;"),
            VerticalAlign::Sub => buf.push_str("vertical-align:sub;"),
        }
    }
}

/// Push an HTML-escaped character to the buffer.
fn push_html_escaped(buf: &mut String, ch: char) {
    match ch {
        '&' => buf.push_str("&amp;"),
        '<' => buf.push_str("&lt;"),
        '>' => buf.push_str("&gt;"),
        '"' => buf.push_str("&quot;"),
        '\'' => buf.push_str("&#39;"),
        _ => buf.push(ch),
    }
}

#[cfg(test)]
mod tests;
