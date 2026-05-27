//! SGR (Select Graphic Rendition) attribute dispatch.
//!
//! Maps `vte::ansi::Attr` variants to cursor template modifications.
//! Called from the VTE Handler impl via `terminal_attribute`.

use vte::ansi::{Attr, Color, NamedColor};

use crate::cell::{Cell, CellFlags, OPAQUE_ALPHA};
use crate::effect::sink::EffectSink;
use crate::term::Term;

impl<S: EffectSink> Term<S> {
    /// `SGR` — apply a graphic-rendition attribute to the cursor's template cell.
    #[inline]
    pub(super) fn terminal_attribute_impl(&mut self, attr: &Attr) {
        let template = &mut self.grid_mut().cursor_mut().template;
        apply(template, attr);
    }
}

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
            // PROTECTED is a DECSCA attribute, NOT an SGR attribute —
            // `SGR 0` does NOT clear it. Preserve the DECSCA bit and
            // clear only the SGR-owned flags (HAS_ALPHA included).
            template.flags &= CellFlags::PROTECTED;
            template.set_underline_color(None);
            // Mode-6 alpha is an SGR attribute — SGR 0 returns all
            // channels to opaque (re-syncs the sidecar + HAS_ALPHA).
            template.set_fg_alpha(OPAQUE_ALPHA);
            template.set_bg_alpha(OPAQUE_ALPHA);
            template.set_underline_alpha(OPAQUE_ALPHA);
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
        Attr::Overline => template.flags.insert(CellFlags::OVERLINE),
        Attr::CancelOverline => template.flags.remove(CellFlags::OVERLINE),
        Attr::Superscript => {
            template.flags.remove(CellFlags::SUBSCRIPT);
            template.flags.insert(CellFlags::SUPERSCRIPT);
        }
        Attr::Subscript => {
            template.flags.remove(CellFlags::SUPERSCRIPT);
            template.flags.insert(CellFlags::SUBSCRIPT);
        }
        Attr::CancelSuperSubscript => {
            template
                .flags
                .remove(CellFlags::SUPERSCRIPT | CellFlags::SUBSCRIPT);
        }
        // Plain (non-mode-6) color sets carry no alpha, so reset the channel
        // to opaque — else overwriting a prior `38:6`/`48:6`/`58:6` inherits a
        // stale translucent alpha on the sticky cursor template (covers 39/49/59).
        Attr::Foreground(color) => {
            template.fg = *color;
            template.set_fg_alpha(OPAQUE_ALPHA);
        }
        Attr::Background(color) => {
            template.bg = *color;
            template.set_bg_alpha(OPAQUE_ALPHA);
        }
        Attr::UnderlineColor(color) => {
            template.set_underline_color(*color);
            template.set_underline_alpha(OPAQUE_ALPHA);
        }
        // SGR mode-6 RGBA (`38:6`/`48:6`/`58:6`): the RGB rides the existing
        // `Color::Spec` storage; the concrete per-channel alpha goes to the
        // `CellExtra` sidecar per Decision 08 (Option C).
        Attr::ForegroundRgba(rgb, alpha) => {
            template.fg = Color::Spec(*rgb);
            template.set_fg_alpha(*alpha);
        }
        Attr::BackgroundRgba(rgb, alpha) => {
            template.bg = Color::Spec(*rgb);
            template.set_bg_alpha(*alpha);
        }
        Attr::UnderlineColorRgba(rgb, alpha) => {
            template.set_underline_color(Some(Color::Spec(*rgb)));
            template.set_underline_alpha(*alpha);
        }
    }
}
