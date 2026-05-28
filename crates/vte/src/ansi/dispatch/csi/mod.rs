//! CSI dispatch handler.
//!
//! SGR attribute parsing lives in [`sgr`]; everything else dispatches inline
//! against `(action, intermediates)` tuples.

extern crate alloc;

use alloc::vec::Vec;

use log::debug;

use crate::ansi::handler::Handler;
use crate::ansi::processor::Timeout;
use crate::ansi::types::{
    Attr, ClearMode, CursorShape, CursorStyle, KeyboardModes,
    KeyboardModesApplyBehavior, LineClearMode, Mode, ModifyOtherKeys, NamedPrivateMode,
    PrivateMode, ScpCharPath, ScpUpdateMode, TabulationClearMode,
};
use crate::Params;

use super::SYNC_UPDATE_TIMEOUT;

mod sgr;
use sgr::attrs_from_sgr_parameters;

/// Dispatch a CSI escape sequence to the handler.
#[allow(clippy::cognitive_complexity)]
pub(super) fn dispatch<H: Handler, T: Timeout>(
    handler: &mut H,
    preceding_char: &mut Option<char>,
    sync_timeout: &mut T,
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
        ('@', [b' ']) => handler.scroll_left(next_param_or(1) as usize),
        ('A', []) => handler.move_up(next_param_or(1) as usize),
        ('A', [b' ']) => handler.scroll_right(next_param_or(1) as usize),
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
 // BSU arms the run-loop deadline so a sync window with
 // no ESU still terminates after SYNC_UPDATE_TIMEOUT.
 // Handler dispatch continues inline — Mode 2026 gates
 // snapshot publication, not byte processing.
                if param == NamedPrivateMode::SyncUpdate as u16 {
                    sync_timeout.set_timeout(SYNC_UPDATE_TIMEOUT);
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
        ('J', [b'?']) => {
            let mode = match next_param_or(0) {
                0 => ClearMode::Below,
                1 => ClearMode::Above,
                2 => ClearMode::All,
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
        ('K', [b'?']) => {
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
 // ESU disarms the run-loop deadline so it does NOT
 // fire after the sync window legitimately ends. Owns
 // the full timer-state transition that pairs with the
 // BSU arm's `set_timeout`.
                if param == NamedPrivateMode::SyncUpdate as u16 {
                    sync_timeout.clear_timeout();
                }
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
        ('p', [b'!']) => handler.decstr(),
        ('p', [b'$']) => {
            let mode = next_param_or(0);
            handler.report_mode(Mode::new(mode));
        },
        ('p', [b'?', b'$']) => {
            let mode = next_param_or(0);
            handler.report_private_mode(PrivateMode::new(mode));
        },
        ('p', [b'"']) => {
 // DECSCL — Set Conformance Level (CSI Pl;Pc " p).
            let level = next_param_or(0);
            let c1_mode = next_param_or(0);
            handler.decscl(level, c1_mode);
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
        ('q', [b'"']) => {
 // DECSCA — Select Character Protection Attribute (CSI Ps " q).
            handler.decsca(next_param_or(0));
        },
        ('q', [b'>']) => {
 // XTVERSION (CSI > Ps q) — report terminal name and version.
 //
 // Reply policy from xterm `charproc.c::CASE_REPORT_VERSION`: replies
 // only when `GetParam(0) <= 0` (default/zero Ps). Non-zero Ps falls
 // through to unhandled. We mirror that gate at the dispatch
 // layer so the Handler method only fires on the requested form.
            if next_param_or(0) == 0 {
                handler.xtversion();
            } else {
                unhandled!();
            }
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
        ('r', [b'$']) => {
 // DECCARA — Change Attributes in Rectangular Area (CSI Pt;Pl;Pb;Pr;Pm $ r).
            let top = next_param_or(1);
            let left = next_param_or(1);
            let bot = next_param_or(1);
            let right = next_param_or(1);
            let attrs: Vec<u16> = params_iter.map(|p| p[0]).collect();
            handler.deccara(top, left, bot, right, &attrs);
        },
        ('S', []) => handler.scroll_up(next_param_or(1) as usize),
        ('S', [b'?']) => {
 // XTSMGRAPHICS — `CSI ? Pi ; Pa ; Pv S`. Exactly 3 top-level
 // params required; malformed arity is silently dropped per the check
 // at xterm `charproc.c:5159` (`if nparam != 3`).
 //
 // CRITICAL: `params.len()` (per `crates/vte/src/params.rs`)
 // returns "total number of parameters and subparameters" —
 // it INCLUDES subparams. Top-level arity check MUST use
 // `params.iter().count()`, which counts parameter GROUPS
 // (each yielding `&[u16]` of subparams).
 //
 // Example: `\x1b[?1:2;1;0S` has 3 top-level params (the
 // first carries subparam `:2`):
 // - `params.len()` = 4 (values: 1, 2, 1, 0)
 // - `params.iter().count()` = 3 (groups)
 //
 // `next_param_or` consumes the FIRST sub-value of each
 // group; subsequent subs are silently ignored.
            if params.iter().count() != 3 {
                unhandled!();
                return;
            }
            let pi = next_param_or(0);
            let pa = next_param_or(0);
            let pv = next_param_or(0);
            handler.graphics_attribute(pi, pa, pv);
        },
        ('s', []) => {
 // CSI s / DECSLRM ambiguity: pass params to handler which
 // knows whether mode 69 (DECLRMM) is active.
 //
 // VTE always pushes at least one default-0 param before CSI
 // dispatch (see `action_csi_dispatch` in lib.rs), so
 // `params.is_empty()` is never true. To distinguish
 // explicit-params from no-params, use BOTH arity (a semicolon
 // was seen → `params.len() > 1`) AND non-default values (at
 // least one explicit non-zero param). This correctly treats
 // `CSI 0;0 s` as DECSLRM (has-params) while still treating
 // `CSI s` as zero-params. `CSI 0 s` is indistinguishable
 // from `CSI s` at the parser level, and both mean "use
 // defaults" per ECMA-48 §5.4.2.
            let arity = params.len();
            let left = next_param_or(0);
            let right = next_param_or(0);
            let has_params = arity > 1 || left != 0;
            handler.decslrm_or_save_cursor(has_params, left, right);
        },
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
        ('t', [b'$']) => {
 // DECRARA — Reverse Attributes in Rectangular Area (CSI Pt;Pl;Pb;Pr;Pm $ t).
            let top = next_param_or(1);
            let left = next_param_or(1);
            let bot = next_param_or(1);
            let right = next_param_or(1);
            let attrs: Vec<u16> = params_iter.map(|p| p[0]).collect();
            handler.decrara(top, left, bot, right, &attrs);
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
        ('u', [b'&']) => handler.decrqupss(),
        ('v', [b'$']) => {
 // DECCRA — Copy Rectangular Area
 // (CSI Pts;Pls;Pbs;Prs;Pps;Ptd;Pld;Ppd $ v).
            let st = next_param_or(1);
            let sl = next_param_or(1);
            let sb = next_param_or(1);
            let sr = next_param_or(1);
            let sp = next_param_or(1);
            let dt = next_param_or(1);
            let dl = next_param_or(1);
            let dp = next_param_or(1);
            handler.deccra(st, sl, sb, sr, sp, dt, dl, dp);
        },
        ('v', [b'"']) => handler.decrqde(),
        ('w', [b'$']) => handler.decrqpsr(next_param_or(0)),
        ('X', []) => handler.erase_chars(next_param_or(1) as usize),
        ('x', [b'*']) => handler.decsace(next_param_or(0)),
        ('x', [b'$']) => {
 // DECFRA — Fill Rectangular Area (CSI Pc;Pt;Pl;Pb;Pr $ x).
            let ch = next_param_or(0x20);
            let top = next_param_or(1);
            let left = next_param_or(1);
            let bot = next_param_or(1);
            let right = next_param_or(1);
            handler.decfra(ch, top, left, bot, right);
        },
        ('y', [b'#']) => handler.xtchecksum(next_param_or(0)),
        ('y', [b'*']) => {
 // DECRQCRA — Request Checksum of Rectangular Area
 // (CSI Pi;Pg;Pt;Pl;Pb;Pr * y).
            let id = next_param_or(0);
            let page = next_param_or(1);
            let top = next_param_or(1);
            let left = next_param_or(1);
            let bot = next_param_or(1);
            let right = next_param_or(1);
            handler.decrqcra(id, page, top, left, bot, right);
        },
        ('Z', []) => handler.move_backward_tabs(next_param_or(1)),
        ('z', [b'$']) => {
 // DECERA — Erase Rectangular Area (CSI Pt;Pl;Pb;Pr $ z).
            let top = next_param_or(1);
            let left = next_param_or(1);
            let bot = next_param_or(1);
            let right = next_param_or(1);
            handler.decera(top, left, bot, right);
        },
        ('{', [b'#']) => handler.push_sgr(),
        ('{', [b'$']) => {
 // DECSERA — Selective Erase Rectangular Area (CSI Pt;Pl;Pb;Pr $ {).
            let top = next_param_or(1);
            let left = next_param_or(1);
            let bot = next_param_or(1);
            let right = next_param_or(1);
            handler.decsera(top, left, bot, right);
        },
        ('|', [b'#']) => {
 // XTREPORTSGR — Report SGR attributes of Rectangular Area
 // (CSI Pt;Pl;Pb;Pr # |).
            let top = next_param_or(1);
            let left = next_param_or(1);
            let bot = next_param_or(1);
            let right = next_param_or(1);
            handler.xtreportsgr(top, left, bot, right);
        },
        ('}', [b'#']) => handler.pop_sgr(),
        ('}', [b'$']) => handler.decsasd(next_param_or(0)),
        ('}', [b'\'']) => handler.decic(next_param_or(1)),
        ('~', [b'$']) => handler.decssdt(next_param_or(0)),
        ('~', [b'\'']) => handler.decdc(next_param_or(1)),
        ('w', [b'\'']) => {
            // DECEFR — Enable Filter Rectangle (CSI Pt;Pl;Pb;Pr ' w).
            // DEC Locator subsystem (independent of DECSET 1001).
            let pt = next_param_or(0);
            let pl = next_param_or(0);
            let pb = next_param_or(0);
            let pr = next_param_or(0);
            handler.decefr(pt, pl, pb, pr);
        },
        ('z', [b'\'']) => {
            // DECELR — Enable Locator Reporting (CSI Ps;Pu ' z).
            // Ps: 0=disabled, 1=continuous, 2=one-report-then-disabled.
            // Pu: 0|2=character cells, 1=pixels.
            let ps = next_param_or(0);
            let pu = next_param_or(0);
            handler.decelr(ps, pu);
        },
        ('{', [b'\'']) => {
            // DECSLE — Select Locator Events (CSI Pm ' {).
            // Pm = bitmask of event classes to report.
            let events: Vec<u16> = std::iter::once(next_param_or(0))
                .chain(params_iter.map(|p| p[0]))
                .collect();
            handler.decsle(&events);
        },
        ('|', [b'\'']) => {
            // DECRQLP — Request Locator Position (CSI Ps ' |).
            // Ps: 0|1|omitted = transmit a single DECLRP locator report.
            handler.decrqlp(next_param_or(0));
        },
        _ => unhandled!(),
    }
}

#[cfg(test)]
mod tests;
