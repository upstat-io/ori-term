//! SGR (Select Graphic Rendition) attribute dispatch.
//!
//! Maps `vte::ansi::Attr` variants to cursor template modifications.
//! Called from the VTE Handler impl via `terminal_attribute`.

use vte::ansi::{Attr, Color, NamedColor};

use crate::cell::{Cell, CellFlags};

/// Apply an SGR attribute to the cursor template cell.
///
/// Each `Attr` variant either sets/clears a flag, changes a color, or
/// resets all attributes. Underline variants are mutually exclusive —
/// setting one clears all others.
pub(super) fn apply(template: &mut Cell, attr: &Attr) {
    match attr {
        Attr::Reset => {
            template.fg = Color::Named(NamedColor::Foreground);
            template.bg = Color::Named(NamedColor::Background);
            template.flags = CellFlags::empty();
            template.set_underline_color(None);
        }
        Attr::Bold => template.flags.insert(CellFlags::BOLD),
        Attr::Dim => template.flags.insert(CellFlags::DIM),
        Attr::Italic => template.flags.insert(CellFlags::ITALIC),
        Attr::Underline => {
            template.flags.remove(CellFlags::ALL_UNDERLINES);
            template.flags.insert(CellFlags::UNDERLINE);
        }
        Attr::DoubleUnderline => {
            template.flags.remove(CellFlags::ALL_UNDERLINES);
            template.flags.insert(CellFlags::DOUBLE_UNDERLINE);
        }
        Attr::Undercurl => {
            template.flags.remove(CellFlags::ALL_UNDERLINES);
            template.flags.insert(CellFlags::CURLY_UNDERLINE);
        }
        Attr::DottedUnderline => {
            template.flags.remove(CellFlags::ALL_UNDERLINES);
            template.flags.insert(CellFlags::DOTTED_UNDERLINE);
        }
        Attr::DashedUnderline => {
            template.flags.remove(CellFlags::ALL_UNDERLINES);
            template.flags.insert(CellFlags::DASHED_UNDERLINE);
        }
        Attr::BlinkSlow | Attr::BlinkFast => template.flags.insert(CellFlags::BLINK),
        Attr::Reverse => template.flags.insert(CellFlags::INVERSE),
        Attr::Hidden => template.flags.insert(CellFlags::HIDDEN),
        Attr::Strike => template.flags.insert(CellFlags::STRIKETHROUGH),
        Attr::CancelBold => template.flags.remove(CellFlags::BOLD),
        Attr::CancelBoldDim => template.flags.remove(CellFlags::BOLD | CellFlags::DIM),
        Attr::CancelItalic => template.flags.remove(CellFlags::ITALIC),
        Attr::CancelUnderline => template.flags.remove(CellFlags::ALL_UNDERLINES),
        Attr::CancelBlink => template.flags.remove(CellFlags::BLINK),
        Attr::CancelReverse => template.flags.remove(CellFlags::INVERSE),
        Attr::CancelHidden => template.flags.remove(CellFlags::HIDDEN),
        Attr::CancelStrike => template.flags.remove(CellFlags::STRIKETHROUGH),
        Attr::Foreground(color) => template.fg = *color,
        Attr::Background(color) => template.bg = *color,
        Attr::UnderlineColor(color) => template.set_underline_color(*color),
    }
}
