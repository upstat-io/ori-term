//! HTML text extraction from grid selection.
//!
//! Converts a `Selection` range into an HTML fragment with inline CSS styles
//! for foreground/background colors, bold, italic, underline, strikethrough,
//! overline, and superscript/subscript. Used when `copy_formatting` is enabled
//! so pasting into rich text editors preserves terminal formatting.
//!
//! - Top-level extraction (`extract_html`, `extract_html_with_text`) and the
//!   `<span>`-coalescing append helpers live in this file.
//! - The per-cell CSS style sidecar ([`CellStyle`]) and its CSS-emission
//!   logic live in `style.rs` to keep this file under the 500-line limit.

mod style;

use crate::cell::CellFlags;
use crate::color::Palette;
use crate::grid::Grid;
use crate::image::KITTY_PLACEHOLDER;
use crate::index::Column;

use vte::ansi::{Color, NamedColor, Rgb};

use self::style::CellStyle;
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
pub(super) struct HtmlCtx<'a> {
    pub(super) palette: &'a Palette,
    pub(super) default_fg: Rgb,
    pub(super) default_bg: Rgb,
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
