//! CSI dispatch handler and SGR attribute parsing.

extern crate alloc;

use alloc::vec::Vec;
use core::convert::TryFrom;
use core::iter;

use log::debug;

use crate::ansi::colors::Rgb;
use crate::ansi::handler::Handler;
use crate::ansi::processor::Timeout;
use crate::ansi::types::{
    Attr, ClearMode, Color, CursorShape, CursorStyle, KeyboardModes,
    KeyboardModesApplyBehavior, LineClearMode, Mode, ModifyOtherKeys, NamedColor,
    NamedPrivateMode, PrivateMode, ScpCharPath, ScpUpdateMode, TabulationClearMode,
};
use crate::{Params, ParamsIter};

use super::SYNC_UPDATE_TIMEOUT;

/// Dispatch a CSI escape sequence to the handler.
#[allow(clippy::cognitive_complexity)]
pub(super) fn dispatch<H: Handler, T: Timeout>(
    handler: &mut H,
    preceding_char: &mut Option<char>,
    sync_timeout: &mut T,
    terminated: &mut bool,
    params: &Params,
    intermediates: &[u8],
    has_ignored_intermediates: bool,
    action: char,
) {
    macro_rules! unhandled {
        () => {{
            debug!(
                "[Unhandled CSI] action={:?}, params={:?}, intermediates={:?}",
                action, params, intermediates
            );
        }};
    }

    if has_ignored_intermediates || intermediates.len() > 2 {
        unhandled!();
        return;
    }

    let mut params_iter = params.iter();

    let mut next_param_or = |default: u16| match params_iter.next() {
        Some(&[param, ..]) if param != 0 => param,
        _ => default,
    };

    match (action, intermediates) {
        ('@', []) => handler.insert_blank(next_param_or(1) as usize),
        ('A', []) => handler.move_up(next_param_or(1) as usize),
        ('B', []) | ('e', []) => handler.move_down(next_param_or(1) as usize),
        ('b', []) => {
            if let Some(c) = *preceding_char {
                for _ in 0..next_param_or(1) {
                    handler.input(c);
                }
            } else {
                debug!("tried to repeat with no preceding char");
            }
        },
        ('C', []) | ('a', []) => handler.move_forward(next_param_or(1) as usize),
        ('c', intermediates) if next_param_or(0) == 0 => {
            handler.identify_terminal(intermediates.first().map(|&i| i as char))
        },
        ('D', []) => handler.move_backward(next_param_or(1) as usize),
        ('d', []) => handler.goto_line(next_param_or(1) as i32 - 1),
        ('E', []) => handler.move_down_and_cr(next_param_or(1) as usize),
        ('F', []) => handler.move_up_and_cr(next_param_or(1) as usize),
        ('G', []) | ('`', []) => handler.goto_col(next_param_or(1) as usize - 1),
        ('W', [b'?']) if next_param_or(0) == 5 => handler.set_tabs(8),
        ('g', []) => {
            let mode = match next_param_or(0) {
                0 => TabulationClearMode::Current,
                3 => TabulationClearMode::All,
                _ => {
                    unhandled!();
                    return;
                },
            };

            handler.clear_tabs(mode);
        },
        ('H', []) | ('f', []) => {
            let y = next_param_or(1) as i32;
            let x = next_param_or(1) as usize;
            handler.goto(y - 1, x - 1);
        },
        ('h', []) => {
            for param in params_iter.map(|param| param[0]) {
                handler.set_mode(Mode::new(param))
            }
        },
        ('h', [b'?']) => {
            for param in params_iter.map(|param| param[0]) {
                // Handle sync updates opaquely.
                if param == NamedPrivateMode::SyncUpdate as u16 {
                    sync_timeout.set_timeout(SYNC_UPDATE_TIMEOUT);
                    *terminated = true;
                }

                handler.set_private_mode(PrivateMode::new(param))
            }
        },
        ('I', []) => handler.move_forward_tabs(next_param_or(1)),
        ('J', []) => {
            let mode = match next_param_or(0) {
                0 => ClearMode::Below,
                1 => ClearMode::Above,
                2 => ClearMode::All,
                3 => ClearMode::Saved,
                _ => {
                    unhandled!();
                    return;
                },
            };

            handler.clear_screen(mode);
        },
        ('K', []) => {
            let mode = match next_param_or(0) {
                0 => LineClearMode::Right,
                1 => LineClearMode::Left,
                2 => LineClearMode::All,
                _ => {
                    unhandled!();
                    return;
                },
            };

            handler.clear_line(mode);
        },
        ('k', [b' ']) => {
            // SCP control.
            let char_path = match next_param_or(0) {
                0 => ScpCharPath::Default,
                1 => ScpCharPath::LTR,
                2 => ScpCharPath::RTL,
                _ => {
                    unhandled!();
                    return;
                },
            };

            let update_mode = match next_param_or(0) {
                0 => ScpUpdateMode::ImplementationDependant,
                1 => ScpUpdateMode::DataToPresentation,
                2 => ScpUpdateMode::PresentationToData,
                _ => {
                    unhandled!();
                    return;
                },
            };

            handler.set_scp(char_path, update_mode);
        },
        ('L', []) => handler.insert_blank_lines(next_param_or(1) as usize),
        ('l', []) => {
            for param in params_iter.map(|param| param[0]) {
                handler.unset_mode(Mode::new(param))
            }
        },
        ('l', [b'?']) => {
            for param in params_iter.map(|param| param[0]) {
                handler.unset_private_mode(PrivateMode::new(param))
            }
        },
        ('M', []) => handler.delete_lines(next_param_or(1) as usize),
        ('m', []) => {
            if params.is_empty() {
                handler.terminal_attribute(Attr::Reset);
            } else {
                attrs_from_sgr_parameters(handler, &mut params_iter);
            }
        },
        ('m', [b'>']) => {
            let mode = match (next_param_or(1) == 4).then(|| next_param_or(0)) {
                Some(0) => ModifyOtherKeys::Reset,
                Some(1) => ModifyOtherKeys::EnableExceptWellDefined,
                Some(2) => ModifyOtherKeys::EnableAll,
                _ => return unhandled!(),
            };
            handler.set_modify_other_keys(mode);
        },
        ('m', [b'?']) => {
            if params_iter.next() == Some(&[4]) {
                handler.report_modify_other_keys();
            } else {
                unhandled!()
            }
        },
        ('n', []) => handler.device_status(next_param_or(0) as usize),
        ('P', []) => handler.delete_chars(next_param_or(1) as usize),
        ('p', [b'$']) => {
            let mode = next_param_or(0);
            handler.report_mode(Mode::new(mode));
        },
        ('p', [b'?', b'$']) => {
            let mode = next_param_or(0);
            handler.report_private_mode(PrivateMode::new(mode));
        },
        ('q', [b' ']) => {
            // DECSCUSR (CSI Ps SP q) -- Set Cursor Style.
            let cursor_style_id = next_param_or(0);
            let shape = match cursor_style_id {
                0 => None,
                1 | 2 => Some(CursorShape::Block),
                3 | 4 => Some(CursorShape::Underline),
                5 | 6 => Some(CursorShape::Beam),
                _ => {
                    unhandled!();
                    return;
                },
            };
            let cursor_style =
                shape.map(|shape| CursorStyle { shape, blinking: cursor_style_id % 2 == 1 });

            handler.set_cursor_style(cursor_style);
        },
        ('r', []) => {
            let top = next_param_or(1) as usize;
            let bottom =
                params_iter.next().map(|param| param[0] as usize).filter(|&param| param != 0);

            handler.set_scrolling_region(top, bottom);
        },
        ('r', [b'?']) => {
            // XTRESTORE: restore saved private mode values.
            let modes: Vec<u16> = params_iter.map(|p| p[0]).collect();
            handler.restore_private_mode_values(&modes);
        },
        ('S', []) => handler.scroll_up(next_param_or(1) as usize),
        ('s', []) => handler.save_cursor_position(),
        ('s', [b'?']) => {
            // XTSAVE: save private mode values.
            let modes: Vec<u16> = params_iter.map(|p| p[0]).collect();
            handler.save_private_mode_values(&modes);
        },
        ('T', []) => handler.scroll_down(next_param_or(1) as usize),
        ('t', []) => match next_param_or(1) as usize {
            14 => handler.text_area_size_pixels(),
            18 => handler.text_area_size_chars(),
            22 => handler.push_title(),
            23 => handler.pop_title(),
            _ => unhandled!(),
        },
        ('u', [b'?']) => handler.report_keyboard_mode(),
        ('u', [b'=']) => {
            let mode = KeyboardModes::from_bits_truncate(next_param_or(0) as u8);
            let behavior = match next_param_or(1) {
                3 => KeyboardModesApplyBehavior::Difference,
                2 => KeyboardModesApplyBehavior::Union,
                // Default is replace.
                _ => KeyboardModesApplyBehavior::Replace,
            };
            handler.set_keyboard_mode(mode, behavior);
        },
        ('u', [b'>']) => {
            let mode = KeyboardModes::from_bits_truncate(next_param_or(0) as u8);
            handler.push_keyboard_mode(mode);
        },
        ('u', [b'<']) => {
            // The default is 1.
            handler.pop_keyboard_modes(next_param_or(1));
        },
        ('u', []) => handler.restore_cursor_position(),
        ('X', []) => handler.erase_chars(next_param_or(1) as usize),
        ('Z', []) => handler.move_backward_tabs(next_param_or(1)),
        _ => unhandled!(),
    }
}

#[inline]
fn attrs_from_sgr_parameters<H: Handler>(handler: &mut H, params: &mut ParamsIter<'_>) {
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
                parse_sgr_color(&mut iter).map(Attr::Foreground)
            },
            [38, params @ ..] => handle_colon_rgb(params).map(Attr::Foreground),
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
                parse_sgr_color(&mut iter).map(Attr::Background)
            },
            [48, params @ ..] => handle_colon_rgb(params).map(Attr::Background),
            [49] => Some(Attr::Background(Color::Named(NamedColor::Background))),
            [58] => {
                let mut iter = params.map(|param| param[0]);
                parse_sgr_color(&mut iter).map(|color| Attr::UnderlineColor(Some(color)))
            },
            [58, params @ ..] => {
                handle_colon_rgb(params).map(|color| Attr::UnderlineColor(Some(color)))
            },
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

/// Handle colon separated rgb color escape sequence.
#[inline]
fn handle_colon_rgb(params: &[u16]) -> Option<Color> {
    let rgb_start = if params.len() > 4 { 2 } else { 1 };
    let rgb_iter = params[rgb_start..].iter().copied();
    let mut iter = iter::once(params[0]).chain(rgb_iter);

    parse_sgr_color(&mut iter)
}

/// Parse a color specifier from list of attributes.
fn parse_sgr_color(params: &mut dyn Iterator<Item = u16>) -> Option<Color> {
    match params.next() {
        Some(2) => Some(Color::Spec(Rgb {
            r: u8::try_from(params.next()?).ok()?,
            g: u8::try_from(params.next()?).ok()?,
            b: u8::try_from(params.next()?).ok()?,
        })),
        Some(5) => Some(Color::Indexed(u8::try_from(params.next()?).ok()?)),
        _ => None,
    }
}
