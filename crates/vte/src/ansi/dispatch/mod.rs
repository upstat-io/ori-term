//! `impl crate::Perform` for `Performer` and `ObservedPerformer` — VTE
//! dispatch routing.
//!
//! Inline dispatch bodies (execute, hook, put, unhook, esc_dispatch, print,
//! apc) are extracted to free functions so BOTH performer types call the
//! same canonical logic — no algorithmic duplication.
//!
//! CSI and OSC dispatch already live in their own submodules (`csi`, `osc`).

extern crate alloc;

use alloc::vec::Vec;
use core::mem;

use log::debug;

use crate::Params;

use super::handler::Handler;
use super::processor::{DcsState, Performer, ProcessorState, Timeout, MAX_APC_LEN};
use super::types::{CharsetIndex, StandardCharset, C0};
use super::SYNC_UPDATE_TIMEOUT;

mod csi;
mod observed;
mod osc;

// ── Shared dispatch free functions ──────────────────────────────────

#[inline]
fn dispatch_print<H: Handler, T: Timeout>(state: &mut ProcessorState<T>, handler: &mut H, c: char) {
    handler.input(c);
    state.preceding_char = Some(c);
}

#[inline]
fn dispatch_execute<H: Handler>(handler: &mut H, byte: u8) {
    match byte {
        C0::HT => handler.put_tab(1),
        C0::BS => handler.backspace(),
        C0::CR => handler.carriage_return(),
        C0::LF | C0::VT | C0::FF => handler.linefeed(),
        C0::BEL => handler.bell(),
        C0::SUB => handler.substitute(),
        C0::SI => handler.set_active_charset(CharsetIndex::G0),
        C0::SO => handler.set_active_charset(CharsetIndex::G1),
        _ => debug!("[unhandled] execute byte={:02x}", byte),
    }
}

#[inline]
fn dispatch_hook<H: Handler, T: Timeout>(
    state: &mut ProcessorState<T>,
    handler: &mut H,
    params: &Params,
    intermediates: &[u8],
    ignore: bool,
    action: char,
) {
    match action {
        'q' if intermediates.is_empty() => {
            // DCS with action 'q' = sixel introducer.
            let flat: Vec<u16> = params.iter().flat_map(|sub| sub.iter().copied()).collect();
            handler.sixel_start(&flat);
            state.dcs_state = DcsState::Sixel;
        },
        'q' if intermediates == [b'$'] => {
            // DCS $ q ... ST = DECRQSS (Request Status String).
            state.decrqss_buf.clear();
            state.dcs_state = DcsState::Decrqss;
        },
        _ => {
            debug!(
                "[unhandled hook] params={:?}, ints: {:?}, ignore: {:?}, action: {:?}",
                params, intermediates, ignore, action
            );
            state.dcs_state = DcsState::None;
        },
    }
}

#[inline]
fn dispatch_put<H: Handler, T: Timeout>(
    state: &mut ProcessorState<T>,
    handler: &mut H,
    byte: u8,
) {
    match state.dcs_state {
        DcsState::Sixel => handler.sixel_put(byte),
        DcsState::Decrqss => {
            // Collect the status type bytes (e.g., `"p` for DECSCL).
            if state.decrqss_buf.len() < 8 {
                state.decrqss_buf.push(byte);
            }
        },
        DcsState::None => debug!("[unhandled put] byte={:?}", byte),
    }
}

#[inline]
fn dispatch_unhook<H: Handler, T: Timeout>(state: &mut ProcessorState<T>, handler: &mut H) {
    match state.dcs_state {
        DcsState::Sixel => handler.sixel_end(),
        DcsState::Decrqss => {
            let query: Vec<u8> = state.decrqss_buf.drain(..).collect();
            handler.decrqss(&query);
        },
        DcsState::None => debug!("[unhandled unhook]"),
    }
    state.dcs_state = DcsState::None;
}

#[inline]
fn dispatch_apc_start<T: Timeout>(state: &mut ProcessorState<T>) {
    state.apc_buf.clear();
}

#[inline]
fn dispatch_apc_put<T: Timeout>(state: &mut ProcessorState<T>, byte: u8) {
    if state.apc_buf.len() < MAX_APC_LEN {
        state.apc_buf.push(byte);
    }
}

#[inline]
fn dispatch_apc_end<H: Handler, T: Timeout>(state: &mut ProcessorState<T>, handler: &mut H) {
    let payload = mem::take(&mut state.apc_buf);
    if !payload.is_empty() {
        handler.apc_dispatch(&payload);
    }
}

#[inline]
fn dispatch_esc<H: Handler>(handler: &mut H, intermediates: &[u8], byte: u8) {
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
            handler.configure_charset(index, $charset)
        }};
    }

    match (byte, intermediates) {
        (b'B', intermediates) => configure_charset!(StandardCharset::Ascii, intermediates),
        (b'D', []) => handler.linefeed(),
        (b'E', []) => {
            handler.linefeed();
            handler.carriage_return();
        },
        (b'H', []) => handler.set_horizontal_tabstop(),
        (b'M', []) => handler.reverse_index(),
        (b'Z', []) => handler.identify_terminal(None),
        (b'c', []) => handler.reset_state(),
        (b'0', intermediates) => {
            configure_charset!(StandardCharset::SpecialCharacterAndLineDrawing, intermediates)
        },
        (b'7', []) => handler.save_cursor_position(),
        (b'8', [b'#']) => handler.decaln(),
        (b'8', []) => handler.restore_cursor_position(),
        (b'=', []) => handler.set_keypad_application_mode(),
        (b'>', []) => handler.unset_keypad_application_mode(),
        (b'N', []) => handler.set_single_shift(CharsetIndex::G2),
        (b'O', []) => handler.set_single_shift(CharsetIndex::G3),
        // String terminator, do nothing (parser handles as string terminator).
        (b'\\', []) => (),
        _ => unhandled!(),
    }
}

// ── Performer Perform impl ──────────────────────────────────────────

impl<'a, H, T> crate::Perform for Performer<'a, H, T>
where
    H: Handler + 'a,
    T: Timeout,
{
    #[inline]
    fn print(&mut self, c: char) {
        dispatch_print(self.state, self.handler, c);
    }

    #[inline]
    fn execute(&mut self, byte: u8) {
        dispatch_execute(self.handler, byte);
    }

    #[inline]
    fn hook(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char) {
        dispatch_hook(self.state, self.handler, params, intermediates, ignore, action);
    }

    #[inline]
    fn put(&mut self, byte: u8) {
        dispatch_put(self.state, self.handler, byte);
    }

    #[inline]
    fn unhook(&mut self) {
        dispatch_unhook(self.state, self.handler);
    }

    #[inline]
    fn apc_start(&mut self) {
        dispatch_apc_start(self.state);
    }

    #[inline]
    fn apc_put(&mut self, byte: u8) {
        dispatch_apc_put(self.state, byte);
    }

    #[inline]
    fn apc_end(&mut self) {
        dispatch_apc_end(self.state, self.handler);
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
        dispatch_esc(self.handler, intermediates, byte);
    }

    #[inline]
    fn terminated(&self) -> bool {
        self.terminated
    }
}
