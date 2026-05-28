//! SGR (Select Graphic Rendition) attribute parsing helpers extracted from
//! the CSI dispatch table. The single public entry point is
//! [`attrs_from_sgr_parameters`], invoked from `csi/mod.rs::dispatch` when the
//! final byte is `m`.

extern crate alloc;

use core::convert::TryFrom;
use core::iter;

use crate::ansi::colors::Rgb;
use crate::ansi::handler::Handler;
use crate::ansi::types::{Attr, Color, NamedColor};
use crate::ParamsIter;

#[inline]
pub(super) fn attrs_from_sgr_parameters<H: Handler>(handler: &mut H, params: &mut ParamsIter<'_>) {
    while let Some(param) = params.next() {
        let attr = match param {
            [0] => Some(Attr::Reset),
            [1] => Some(Attr::Bold),
            [2] => Some(Attr::Dim),
            [3] => Some(Attr::Italic),
            [4, 0] => Some(Attr::CancelUnderline),
            [4, 2] => Some(Attr::DoubleUnderline),
            [4, 3] => Some(Attr::Undercurl),
            [4, 4] => Some(Attr::DottedUnderline),
            [4, 5] => Some(Attr::DashedUnderline),
            [4, ..] => Some(Attr::Underline),
            [5] => Some(Attr::BlinkSlow),
            [6] => Some(Attr::BlinkFast),
            [7] => Some(Attr::Reverse),
            [8] => Some(Attr::Hidden),
            [9] => Some(Attr::Strike),
            [21] => Some(Attr::CancelBold),
            [22] => Some(Attr::CancelBoldDim),
            [23] => Some(Attr::CancelItalic),
            [24] => Some(Attr::CancelUnderline),
            [25] => Some(Attr::CancelBlink),
            [27] => Some(Attr::CancelReverse),
            [28] => Some(Attr::CancelHidden),
            [29] => Some(Attr::CancelStrike),
            [53] => Some(Attr::Overline),
            [55] => Some(Attr::CancelOverline),
            [73] => Some(Attr::Superscript),
            [74] => Some(Attr::Subscript),
            [75] => Some(Attr::CancelSuperSubscript),
            [30] => Some(Attr::Foreground(Color::Named(NamedColor::Black))),
            [31] => Some(Attr::Foreground(Color::Named(NamedColor::Red))),
            [32] => Some(Attr::Foreground(Color::Named(NamedColor::Green))),
            [33] => Some(Attr::Foreground(Color::Named(NamedColor::Yellow))),
            [34] => Some(Attr::Foreground(Color::Named(NamedColor::Blue))),
            [35] => Some(Attr::Foreground(Color::Named(NamedColor::Magenta))),
            [36] => Some(Attr::Foreground(Color::Named(NamedColor::Cyan))),
            [37] => Some(Attr::Foreground(Color::Named(NamedColor::White))),
            [38] => {
                let mut iter = params.map(|param| param[0]);
                parse_sgr_color(&mut iter).map(fg_attr)
            },
            [38, params @ ..] => handle_colon_rgb(params).map(fg_attr),
            [39] => Some(Attr::Foreground(Color::Named(NamedColor::Foreground))),
            [40] => Some(Attr::Background(Color::Named(NamedColor::Black))),
            [41] => Some(Attr::Background(Color::Named(NamedColor::Red))),
            [42] => Some(Attr::Background(Color::Named(NamedColor::Green))),
            [43] => Some(Attr::Background(Color::Named(NamedColor::Yellow))),
            [44] => Some(Attr::Background(Color::Named(NamedColor::Blue))),
            [45] => Some(Attr::Background(Color::Named(NamedColor::Magenta))),
            [46] => Some(Attr::Background(Color::Named(NamedColor::Cyan))),
            [47] => Some(Attr::Background(Color::Named(NamedColor::White))),
            [48] => {
                let mut iter = params.map(|param| param[0]);
                parse_sgr_color(&mut iter).map(bg_attr)
            },
            [48, params @ ..] => handle_colon_rgb(params).map(bg_attr),
            [49] => Some(Attr::Background(Color::Named(NamedColor::Background))),
            [58] => {
                let mut iter = params.map(|param| param[0]);
                parse_sgr_color(&mut iter).map(underline_attr)
            },
            [58, params @ ..] => handle_colon_rgb(params).map(underline_attr),
            [59] => Some(Attr::UnderlineColor(None)),
            [90] => Some(Attr::Foreground(Color::Named(NamedColor::BrightBlack))),
            [91] => Some(Attr::Foreground(Color::Named(NamedColor::BrightRed))),
            [92] => Some(Attr::Foreground(Color::Named(NamedColor::BrightGreen))),
            [93] => Some(Attr::Foreground(Color::Named(NamedColor::BrightYellow))),
            [94] => Some(Attr::Foreground(Color::Named(NamedColor::BrightBlue))),
            [95] => Some(Attr::Foreground(Color::Named(NamedColor::BrightMagenta))),
            [96] => Some(Attr::Foreground(Color::Named(NamedColor::BrightCyan))),
            [97] => Some(Attr::Foreground(Color::Named(NamedColor::BrightWhite))),
            [100] => Some(Attr::Background(Color::Named(NamedColor::BrightBlack))),
            [101] => Some(Attr::Background(Color::Named(NamedColor::BrightRed))),
            [102] => Some(Attr::Background(Color::Named(NamedColor::BrightGreen))),
            [103] => Some(Attr::Background(Color::Named(NamedColor::BrightYellow))),
            [104] => Some(Attr::Background(Color::Named(NamedColor::BrightBlue))),
            [105] => Some(Attr::Background(Color::Named(NamedColor::BrightMagenta))),
            [106] => Some(Attr::Background(Color::Named(NamedColor::BrightCyan))),
            [107] => Some(Attr::Background(Color::Named(NamedColor::BrightWhite))),
            _ => None,
        };

        match attr {
            Some(attr) => handler.terminal_attribute(attr),
            None => continue,
        }
    }
}

/// A color parsed from an SGR color sub-parameter list: either a plain
/// (mode 2 direct RGB / mode 5 indexed) color, or the alpha-bearing mode-6
/// RGBA form (`38:6::r:g:b:a`, wezterm-invented).
enum SgrColor {
    Plain(Color),
    Rgba(Rgb, u8),
}

/// Map a parsed foreground color to the matching `Attr`.
fn fg_attr(color: SgrColor) -> Attr {
    match color {
        SgrColor::Plain(c) => Attr::Foreground(c),
        SgrColor::Rgba(rgb, a) => Attr::ForegroundRgba(rgb, a),
    }
}

/// Map a parsed background color to the matching `Attr`.
fn bg_attr(color: SgrColor) -> Attr {
    match color {
        SgrColor::Plain(c) => Attr::Background(c),
        SgrColor::Rgba(rgb, a) => Attr::BackgroundRgba(rgb, a),
    }
}

/// Map a parsed underline color to the matching `Attr`.
fn underline_attr(color: SgrColor) -> Attr {
    match color {
        SgrColor::Plain(c) => Attr::UnderlineColor(Some(c)),
        SgrColor::Rgba(rgb, a) => Attr::UnderlineColorRgba(rgb, a),
    }
}

/// Handle colon separated rgb color escape sequence.
///
/// The color-space-id slot between the mode and the color components is
/// optional (`38:2::r:g:b` and `38:2:r:g:b` are both valid); its presence is
/// derived from the mode's component count (mode 2 = 3 RGB, mode 5 = 1 index,
/// mode 6 = 4 RGBA). The read window is anchored at the FRONT — skip the mode
/// plus the color-space slot when present — so any trailing subparameters
/// (e.g. ISO 8613-6 tolerance params after RGB) are ignored by
/// `parse_sgr_color` rather than shifting the window rightward.
#[inline]
fn handle_colon_rgb(params: &[u16]) -> Option<SgrColor> {
    let components = match *params.first()? {
        2 => 3,
        5 => 1,
        6 => 4,
        _ => return None,
    };
    // Mode + color-space slot present when length covers both plus components.
    let rgb_start = if params.len() >= components + 2 { 2 } else { 1 };
    let rgb_iter = params[rgb_start..].iter().copied();
    let mut iter = iter::once(params[0]).chain(rgb_iter);

    parse_sgr_color(&mut iter)
}

/// Parse a color specifier from list of attributes.
///
/// Modes: `2` direct RGB, `5` indexed, `6` direct RGBA (per-channel alpha;
/// the wezterm `38:6::r:g:b:a` form — alpha rides `SgrColor::Rgba`, kept out
/// of [`Color`] so the cell's RGB stays in `Color::Spec`).
fn parse_sgr_color(params: &mut dyn Iterator<Item = u16>) -> Option<SgrColor> {
    match params.next() {
        Some(2) => Some(SgrColor::Plain(Color::Spec(Rgb {
            r: u8::try_from(params.next()?).ok()?,
            g: u8::try_from(params.next()?).ok()?,
            b: u8::try_from(params.next()?).ok()?,
        }))),
        Some(5) => Some(SgrColor::Plain(Color::Indexed(u8::try_from(params.next()?).ok()?))),
        Some(6) => Some(SgrColor::Rgba(
            Rgb {
                r: u8::try_from(params.next()?).ok()?,
                g: u8::try_from(params.next()?).ok()?,
                b: u8::try_from(params.next()?).ok()?,
            },
            u8::try_from(params.next()?).ok()?,
        )),
        _ => None,
    }
}
