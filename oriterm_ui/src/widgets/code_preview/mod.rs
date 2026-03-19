//! Syntax-highlighted code preview widget.
//!
//! Displays multi-line code with hardcoded syntax coloring on a card
//! background. Used in font settings to preview the current terminal font.

use crate::color::Color;
use crate::draw::RectStyle;
use crate::geometry::Point;
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::text::TextStyle;
use crate::widget_id::WidgetId;

use super::{DrawCtx, LayoutCtx, Widget};

/// Corner radius for the preview card.
const CORNER_RADIUS: f32 = 8.0;

/// Padding inside the preview card.
const PADDING: f32 = 12.0;

/// Font size for code text.
const CODE_FONT_SIZE: f32 = 12.0;

/// Font size for the "Preview" label.
const LABEL_FONT_SIZE: f32 = 9.0;

/// Height of the "Preview" label area.
const LABEL_HEIGHT: f32 = 18.0;

/// Syntax color: keyword (purple).
const COLOR_KEYWORD: Color = Color::hex(0xBD_93_F9);

/// Syntax color: function (blue).
const COLOR_FUNCTION: Color = Color::hex(0x61_AF_EF);

/// Syntax color: string (green).
const COLOR_STRING: Color = Color::hex(0x98_C3_79);

/// Syntax color: comment (gray).
const COLOR_COMMENT: Color = Color::hex(0x5C_63_70);

/// Syntax color: number (orange).
const COLOR_NUMBER: Color = Color::hex(0xD1_9A_66);

/// A display-only code preview with syntax highlighting.
///
/// Renders hardcoded Rust code lines on a `bg_card` background.
/// No interaction — pure display.
pub struct CodePreviewWidget {
    id: WidgetId,
}

/// A single colored span in a code line.
struct CodeSpan {
    text: &'static str,
    color: Color,
}

impl CodePreviewWidget {
    /// Creates a new code preview widget.
    pub fn new() -> Self {
        Self {
            id: WidgetId::next(),
        }
    }

    /// Returns the preview code lines as colored spans.
    fn code_lines(fg: Color) -> Vec<Vec<CodeSpan>> {
        vec![
            // fn main() {
            vec![
                CodeSpan {
                    text: "fn ",
                    color: COLOR_KEYWORD,
                },
                CodeSpan {
                    text: "main",
                    color: COLOR_FUNCTION,
                },
                CodeSpan {
                    text: "() {",
                    color: fg,
                },
            ],
            // let name = "Ori";
            vec![
                CodeSpan {
                    text: "    ",
                    color: fg,
                },
                CodeSpan {
                    text: "let ",
                    color: COLOR_KEYWORD,
                },
                CodeSpan {
                    text: "name = ",
                    color: fg,
                },
                CodeSpan {
                    text: "\"Ori\"",
                    color: COLOR_STRING,
                },
                CodeSpan {
                    text: ";",
                    color: fg,
                },
            ],
            // let version = 42;
            vec![
                CodeSpan {
                    text: "    ",
                    color: fg,
                },
                CodeSpan {
                    text: "let ",
                    color: COLOR_KEYWORD,
                },
                CodeSpan {
                    text: "version = ",
                    color: fg,
                },
                CodeSpan {
                    text: "42",
                    color: COLOR_NUMBER,
                },
                CodeSpan {
                    text: ";",
                    color: fg,
                },
            ],
            // // Fast & beautiful
            vec![
                CodeSpan {
                    text: "    ",
                    color: COLOR_COMMENT,
                },
                CodeSpan {
                    text: "// Fast & beautiful",
                    color: COLOR_COMMENT,
                },
            ],
            // }
            vec![CodeSpan {
                text: "}",
                color: fg,
            }],
        ]
    }
}

impl Default for CodePreviewWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for CodePreviewWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn sense(&self) -> Sense {
        Sense::none()
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
        let line_count = 5;
        let line_h = CODE_FONT_SIZE + 4.0;
        let h = PADDING * 2.0 + LABEL_HEIGHT + line_count as f32 * line_h;
        LayoutBox::leaf(280.0, h).with_widget_id(self.id)
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        let bounds = ctx.bounds;

        // Card background.
        let bg_style = RectStyle::filled(ctx.theme.bg_card).with_radius(CORNER_RADIUS);
        ctx.scene.push_quad(bounds, bg_style);

        let x = bounds.x() + PADDING;
        let w = bounds.width() - PADDING * 2.0;
        let mut y = bounds.y() + PADDING;

        // "Preview" label.
        let label_style = TextStyle::new(LABEL_FONT_SIZE, ctx.theme.fg_faint);
        let label = "PREVIEW";
        let shaped = ctx.measurer.shape(label, &label_style, w);
        ctx.scene
            .push_text(Point::new(x, y), shaped, ctx.theme.fg_faint);
        y += LABEL_HEIGHT;

        // Code lines.
        let line_h = CODE_FONT_SIZE + 4.0;
        let fg = ctx.theme.fg_primary;
        for line in Self::code_lines(fg) {
            let mut lx = x;
            for span in &line {
                let style = TextStyle::new(CODE_FONT_SIZE, span.color);
                let shaped = ctx.measurer.shape(span.text, &style, w);
                let advance = shaped.width;
                ctx.scene.push_text(Point::new(lx, y), shaped, span.color);
                lx += advance;
            }
            y += line_h;
        }
    }
}

#[cfg(test)]
mod tests;
