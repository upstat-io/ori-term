//! `impl crate::Perform for Performer` — VTE dispatch routing.
//!
//! Delegates osc_dispatch and csi_dispatch to submodule free functions
//! to keep each file under 500 lines.

extern crate alloc;

use alloc::vec::Vec;
use core::mem;

use log::debug;

use crate::Params;

use super::handler::Handler;
use super::processor::{DcsState, Performer, Timeout, MAX_APC_LEN};
use super::types::{CharsetIndex, StandardCharset, C0};
use super::SYNC_UPDATE_TIMEOUT;

mod csi;
mod osc;

impl<'a, H, T> crate::Perform for Performer<'a, H, T>
where
    H: Handler + 'a,
    T: Timeout,
{
    #[inline]
    fn print(&mut self, c: char) {
        self.handler.input(c);
        self.state.preceding_char = Some(c);
    }

    #[inline]
    fn execute(&mut self, byte: u8) {
        match byte {
            C0::HT => self.handler.put_tab(1),
            C0::BS => self.handler.backspace(),
            C0::CR => self.handler.carriage_return(),
            C0::LF | C0::VT | C0::FF => self.handler.linefeed(),
            C0::BEL => self.handler.bell(),
            C0::SUB => self.handler.substitute(),
            C0::SI => self.handler.set_active_charset(CharsetIndex::G0),
            C0::SO => self.handler.set_active_charset(CharsetIndex::G1),
            _ => debug!("[unhandled] execute byte={:02x}", byte),
        }
    }

    #[inline]
    fn hook(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char) {
        match action {
            'q' if intermediates.is_empty() => {
                // DCS with action 'q' = sixel introducer.
                let flat: Vec<u16> = params.iter().flat_map(|sub| sub.iter().copied()).collect();
                self.handler.sixel_start(&flat);
                self.state.dcs_state = DcsState::Sixel;
            },
            _ => {
                debug!(
                    "[unhandled hook] params={:?}, ints: {:?}, ignore: {:?}, action: {:?}",
                    params, intermediates, ignore, action
                );
                self.state.dcs_state = DcsState::None;
            },
        }
    }

    #[inline]
    fn put(&mut self, byte: u8) {
        match self.state.dcs_state {
            DcsState::Sixel => self.handler.sixel_put(byte),
            DcsState::None => debug!("[unhandled put] byte={:?}", byte),
        }
    }

    #[inline]
    fn unhook(&mut self) {
        match self.state.dcs_state {
            DcsState::Sixel => self.handler.sixel_end(),
            DcsState::None => debug!("[unhandled unhook]"),
        }
        self.state.dcs_state = DcsState::None;
    }

    #[inline]
    fn apc_start(&mut self) {
        self.state.apc_buf.clear();
    }

    #[inline]
    fn apc_put(&mut self, byte: u8) {
        if self.state.apc_buf.len() < MAX_APC_LEN {
            self.state.apc_buf.push(byte);
        }
    }

    #[inline]
    fn apc_end(&mut self) {
        let payload = mem::take(&mut self.state.apc_buf);
        if !payload.is_empty() {
            self.handler.apc_dispatch(&payload);
        }
    }

    #[inline]
    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        osc::dispatch(&mut *self.handler, params, bell_terminated);
    }

    #[allow(clippy::cognitive_complexity)]
    #[inline]
    fn csi_dispatch(
        &mut self,
        params: &Params,
        intermediates: &[u8],
        has_ignored_intermediates: bool,
        action: char,
    ) {
        csi::dispatch(
            &mut *self.handler,
            &mut self.state.preceding_char,
            &mut self.state.sync_state.timeout,
            &mut self.terminated,
            params,
            intermediates,
            has_ignored_intermediates,
            action,
        );
    }

    #[inline]
    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        macro_rules! unhandled {
            () => {{
                debug!(
                    "[unhandled] esc_dispatch ints={:?}, byte={:?} ({:02x})",
                    intermediates, byte as char, byte
                );
            }};
        }

        macro_rules! configure_charset {
            ($charset:path, $intermediates:expr) => {{
                let index: CharsetIndex = match $intermediates {
                    [b'('] => CharsetIndex::G0,
                    [b')'] => CharsetIndex::G1,
                    [b'*'] => CharsetIndex::G2,
                    [b'+'] => CharsetIndex::G3,
                    _ => {
                        unhandled!();
                        return;
                    },
                };
                self.handler.configure_charset(index, $charset)
            }};
        }

        match (byte, intermediates) {
            (b'B', intermediates) => configure_charset!(StandardCharset::Ascii, intermediates),
            (b'D', []) => self.handler.linefeed(),
            (b'E', []) => {
                self.handler.linefeed();
                self.handler.carriage_return();
            },
            (b'H', []) => self.handler.set_horizontal_tabstop(),
            (b'M', []) => self.handler.reverse_index(),
            (b'Z', []) => self.handler.identify_terminal(None),
            (b'c', []) => self.handler.reset_state(),
            (b'0', intermediates) => {
                configure_charset!(StandardCharset::SpecialCharacterAndLineDrawing, intermediates)
            },
            (b'7', []) => self.handler.save_cursor_position(),
            (b'8', [b'#']) => self.handler.decaln(),
            (b'8', []) => self.handler.restore_cursor_position(),
            (b'=', []) => self.handler.set_keypad_application_mode(),
            (b'>', []) => self.handler.unset_keypad_application_mode(),
            (b'N', []) => self.handler.set_single_shift(CharsetIndex::G2),
            (b'O', []) => self.handler.set_single_shift(CharsetIndex::G3),
            // String terminator, do nothing (parser handles as string terminator).
            (b'\\', []) => (),
            _ => unhandled!(),
        }
    }

    #[inline]
    fn terminated(&self) -> bool {
        self.terminated
    }
}
