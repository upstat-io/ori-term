//! VTE processor, performer, sync state, and timeout trait.

extern crate alloc;

use alloc::vec::Vec;
use core::mem;
use core::time::Duration;
#[cfg(feature = "std")]
use std::time::Instant;

use super::handler::Handler;
use super::types::NamedPrivateMode;
use super::{BSU_CSI, ESU_CSI, SYNC_BUFFER_SIZE, SYNC_ESCAPE_LEN, SYNC_UPDATE_TIMEOUT};

/// Maximum APC payload size (32 MiB). Prevents OOM from malicious input.
pub(super) const MAX_APC_LEN: usize = 32 * 1024 * 1024;

/// Tracks the active DCS sequence type for `put`/`unhook` dispatch.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) enum DcsState {
    /// No active DCS sequence (or an unrecognized one).
    #[default]
    None,
    /// Active sixel sequence (DCS action `q`).
    Sixel,
    /// DECRQSS: Request Status String (DCS `$q` ... ST).
    Decrqss,
}

/// Internal state for VTE processor.
#[derive(Debug, Default)]
pub(super) struct ProcessorState<T: Timeout> {
    /// Last processed character for repetition.
    pub(super) preceding_char: Option<char>,

    /// State for synchronized terminal updates.
    pub(super) sync_state: SyncState<T>,

    /// Buffer for accumulating APC payload bytes across `advance` calls.
    pub(super) apc_buf: Vec<u8>,

    /// Active DCS sequence type for routing `put`/`unhook` calls.
    pub(super) dcs_state: DcsState,

    /// Buffer for DECRQSS data bytes (the status type being queried).
    pub(super) decrqss_buf: Vec<u8>,
}

/// State for synchronized terminal updates.
#[derive(Debug)]
pub(super) struct SyncState<T: Timeout> {
    /// Handler for synchronized updates.
    pub(super) timeout: T,

    /// Bytes read during the synchronized update.
    pub(super) buffer: Vec<u8>,
}

impl<T: Timeout> Default for SyncState<T> {
    fn default() -> Self {
        Self { buffer: Vec::with_capacity(SYNC_BUFFER_SIZE), timeout: Default::default() }
    }
}

/// The processor wraps a `crate::Parser` to ultimately call methods on a
/// Handler.
#[cfg(feature = "std")]
#[derive(Default)]
pub struct Processor<T: Timeout = StdSyncHandler> {
    pub(super) state: ProcessorState<T>,
    parser: crate::Parser,
}

/// The processor wraps a `crate::Parser` to ultimately call methods on a
/// Handler.
#[cfg(not(feature = "std"))]
#[derive(Default)]
pub struct Processor<T: Timeout> {
    pub(super) state: ProcessorState<T>,
    parser: crate::Parser,
}

impl<T: Timeout> Processor<T> {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Synchronized update timeout.
    pub fn sync_timeout(&self) -> &T {
        &self.state.sync_state.timeout
    }

    /// Process a new byte from the PTY.
    #[inline]
    pub fn advance<H>(&mut self, handler: &mut H, bytes: &[u8])
    where
        H: Handler,
    {
        let mut processed = 0;
        while processed != bytes.len() {
            if self.state.sync_state.timeout.pending_timeout() {
                processed += self.advance_sync(handler, &bytes[processed..]);
            } else {
                let mut performer = Performer::new(&mut self.state, handler);
                processed +=
                    self.parser.advance_until_terminated(&mut performer, &bytes[processed..]);
            }
        }
    }

    /// End a synchronized update.
    pub fn stop_sync<H>(&mut self, handler: &mut H)
    where
        H: Handler,
    {
        self.stop_sync_internal(handler, None);
    }

    /// End a synchronized update.
    ///
    /// The `bsu_offset` parameter should be passed if the sync buffer contains
    /// a new BSU escape that is not part of the current synchronized
    /// update.
    fn stop_sync_internal<H>(&mut self, handler: &mut H, bsu_offset: Option<usize>)
    where
        H: Handler,
    {
        // Process all synchronized bytes.
        //
        // NOTE: We do not use `advance_until_terminated` here since BSU sequences are
        // processed automatically during the synchronized update.
        let buffer = mem::take(&mut self.state.sync_state.buffer);
        let offset = bsu_offset.unwrap_or(buffer.len());
        let mut performer = Performer::new(&mut self.state, handler);
        self.parser.advance(&mut performer, &buffer[..offset]);
        self.state.sync_state.buffer = buffer;

        match bsu_offset {
            // Just clear processed bytes if there is a new BSU.
            //
            // NOTE: We do not need to re-process for a new ESU since the `advance_sync`
            // function checks for BSUs in reverse.
            Some(bsu_offset) => {
                let new_len = self.state.sync_state.buffer.len() - bsu_offset;
                self.state.sync_state.buffer.copy_within(bsu_offset.., 0);
                self.state.sync_state.buffer.truncate(new_len);
            },
            // Report mode and clear state if no new BSU is present.
            None => {
                handler.unset_private_mode(NamedPrivateMode::SyncUpdate.into());
                self.state.sync_state.timeout.clear_timeout();
                self.state.sync_state.buffer.clear();
            },
        }
    }

    /// Number of bytes in the synchronization buffer.
    #[inline]
    pub fn sync_bytes_count(&self) -> usize {
        self.state.sync_state.buffer.len()
    }

    /// Process a new byte during a synchronized update.
    ///
    /// Returns the number of bytes processed.
    #[cold]
    fn advance_sync<H>(&mut self, handler: &mut H, bytes: &[u8]) -> usize
    where
        H: Handler,
    {
        // Advance sync parser or stop sync if we'd exceed the maximum buffer size.
        if self.state.sync_state.buffer.len() + bytes.len() >= SYNC_BUFFER_SIZE - 1 {
            // Terminate the synchronized update.
            self.stop_sync_internal(handler, None);

            // Just parse the bytes normally.
            let mut performer = Performer::new(&mut self.state, handler);
            self.parser.advance_until_terminated(&mut performer, bytes)
        } else {
            self.state.sync_state.buffer.extend(bytes);
            self.advance_sync_csi(handler, bytes.len());
            bytes.len()
        }
    }

    /// Handle BSU/ESU CSI sequences during synchronized update.
    fn advance_sync_csi<H>(&mut self, handler: &mut H, new_bytes: usize)
    where
        H: Handler,
    {
        // Get constraints within which a new escape character might be relevant.
        let buffer_len = self.state.sync_state.buffer.len();
        let start_offset = (buffer_len - new_bytes).saturating_sub(SYNC_ESCAPE_LEN - 1);
        let end_offset = buffer_len.saturating_sub(SYNC_ESCAPE_LEN - 1);
        let search_buffer = &self.state.sync_state.buffer[start_offset..end_offset];

        // Search for termination/extension escapes in the added bytes.
        //
        // NOTE: It is technically legal to specify multiple private modes in the same
        // escape, but we only allow EXACTLY `\e[?2026h`/`\e[?2026l` to keep the parser
        // more simple.
        let mut bsu_offset = None;
        for index in memchr::memchr_iter(0x1B, search_buffer).rev() {
            let offset = start_offset + index;
            let escape = &self.state.sync_state.buffer[offset..offset + SYNC_ESCAPE_LEN];

            if escape == BSU_CSI {
                self.state.sync_state.timeout.set_timeout(SYNC_UPDATE_TIMEOUT);
                bsu_offset = Some(offset);
            } else if escape == ESU_CSI {
                self.stop_sync_internal(handler, bsu_offset);
                break;
            }
        }
    }
}

/// Helper type that implements `crate::Perform`.
///
/// Processor creates a Performer when running advance and passes the Performer
/// to `crate::Parser`.
pub(super) struct Performer<'a, H: Handler, T: Timeout> {
    pub(super) state: &'a mut ProcessorState<T>,
    pub(super) handler: &'a mut H,

    /// Whether the parser should be prematurely terminated.
    pub(super) terminated: bool,
}

impl<'a, H: Handler + 'a, T: Timeout> Performer<'a, H, T> {
    /// Create a performer.
    #[inline]
    pub fn new<'b>(state: &'b mut ProcessorState<T>, handler: &'b mut H) -> Performer<'b, H, T> {
        Performer { state, handler, terminated: Default::default() }
    }
}

/// Standard synchronized update handler using `std::time::Instant`.
#[cfg(feature = "std")]
#[derive(Default)]
pub struct StdSyncHandler {
    timeout: Option<Instant>,
}

#[cfg(feature = "std")]
impl StdSyncHandler {
    /// Synchronized update expiration time.
    #[inline]
    pub fn sync_timeout(&self) -> Option<Instant> {
        self.timeout
    }
}

#[cfg(feature = "std")]
impl Timeout for StdSyncHandler {
    #[inline]
    fn set_timeout(&mut self, duration: Duration) {
        self.timeout = Some(Instant::now() + duration);
    }

    #[inline]
    fn clear_timeout(&mut self) {
        self.timeout = None;
    }

    #[inline]
    fn pending_timeout(&self) -> bool {
        self.timeout.is_some()
    }
}

/// Interface for creating timeouts and checking their expiry.
///
/// This is internally used by the [`Processor`] to handle synchronized
/// updates.
pub trait Timeout: Default {
    /// Sets the timeout for the next synchronized update.
    ///
    /// The `duration` parameter specifies the duration of the timeout. Once the
    /// specified duration has elapsed, the synchronized update rotuine can be
    /// performed.
    fn set_timeout(&mut self, duration: Duration);
    /// Clear the current timeout.
    fn clear_timeout(&mut self);
    /// Returns whether a timeout is currently active and has not yet expired.
    fn pending_timeout(&self) -> bool;
}
