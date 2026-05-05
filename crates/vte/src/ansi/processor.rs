//! VTE processor, performer, sync state, and timeout trait.

extern crate alloc;

use alloc::vec::Vec;
use core::time::Duration;
#[cfg(feature = "std")]
use std::time::Instant;

use super::handler::Handler;

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
 /// DECRQSS: Request Status String (DCS `$q`... ST).
    Decrqss,
 /// DECRSPS: Restore Presentation Status (DCS Ps `$t` Pt ST).
 ///
 /// Carries the `Ps` selector (1 = DECCIR cursor-info, 2 = DECTABSR
 /// tab-stops) alongside the payload accumulated via `put`.
    Decrsps { ps: u16 },
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

 // VENDORED PATCH (oriterm): spec-conformance §12 — DCS abort flag.
 /// Set by `Perform::notify_dcs_abort` when the parser aborts an
 /// in-flight DCS via CAN (0x18), SUB (0x1A), or ESC (0x1B) —
 /// consumed by `dispatch_unhook` so sixel (and other DCS kinds) can
 /// distinguish a normal ST finish from an abort. Reset to `false`
 /// after every unhook so the next DCS starts with a clean flag.
    pub(super) dcs_aborted: bool,

 /// Buffer for DECRQSS data bytes (the status type being queried).
    pub(super) decrqss_buf: Vec<u8>,

 /// Buffer for DECRSPS payload bytes (state-restore Pt body).
    pub(super) decrsps_buf: Vec<u8>,
}

/// State for synchronized terminal updates.
///
/// Mode 2026 (synchronized output) gates SNAPSHOT publication, not byte
/// processing. Bytes inside a BSU/ESU window dispatch through the
/// handler inline; the only state held here is the deadline timer used
/// by the run loop's `select!` deadline arm to bound the sync window.
#[derive(Debug, Default)]
pub(super) struct SyncState<T: Timeout> {
 /// Sync window deadline (set by BSU, cleared by ESU/timeout).
    pub(super) timeout: T,
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
 ///
 /// Mode 2026 (synchronized output) does NOT gate byte processing —
 /// handler dispatch happens inline regardless of whether a sync
 /// window is active. Sync gating lives one layer up at the snapshot-
 /// publication boundary.
    #[inline]
    pub fn advance<H>(&mut self, handler: &mut H, bytes: &[u8])
    where
        H: Handler,
    {
        let mut performer = Performer::new(&mut self.state, handler);
        self.parser.advance(&mut performer, bytes);
    }

 /// Process bytes from the PTY, recording raw `Perform` callbacks via
 /// the `PerformObserver` before dispatching to the `Handler`.
 ///
 /// This is the observation-enabled counterpart to [`advance`](Self::advance).
 /// The observer sees every raw parser action (CSI, OSC, ESC, execute,
 /// print, DCS, APC) in arrival order — Mode 2026 does NOT defer
 /// dispatch, so observed actions match the byte-stream order
 /// exactly.
    #[inline]
    pub fn advance_with_observer<H, O>(
        &mut self,
        handler: &mut H,
        observer: &mut O,
        bytes: &[u8],
    ) where
        H: Handler,
        O: super::observer::PerformObserver,
    {
        let mut observed = ObservedPerformer::new(&mut self.state, handler, observer);
        self.parser.advance(&mut observed, bytes);
    }

 /// True iff a Mode 2026 sync window is active (parser-side timer
 /// armed). Used by tests; the run loop relies on
 /// [`Self::sync_timeout`] for `select!` deadline computation.
    #[inline]
    pub fn is_sync_active(&self) -> bool {
        self.state.sync_state.timeout.pending_timeout()
    }

 /// Disarm the parser-side sync timer.
 ///
 /// Called from the IO thread's `handle_sync_timeout` after the run
 /// loop's deadline arm fires; ESU dispatch handles disarm via the
 /// handler-level dispatcher.
    #[inline]
    pub fn clear_sync_timeout(&mut self) {
        self.state.sync_state.timeout.clear_timeout();
    }
}

/// Helper type that implements `crate::Perform`.
///
/// Processor creates a Performer when running advance and passes the Performer
/// to `crate::Parser`.
pub(super) struct Performer<'a, H: Handler, T: Timeout> {
    pub(super) state: &'a mut ProcessorState<T>,
    pub(super) handler: &'a mut H,
}

impl<'a, H: Handler + 'a, T: Timeout> Performer<'a, H, T> {
 /// Create a performer.
    #[inline]
    pub fn new<'b>(state: &'b mut ProcessorState<T>, handler: &'b mut H) -> Performer<'b, H, T> {
        Performer { state, handler }
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

/// `Performer` variant that records raw `PerformAction` entries into
/// a `PerformObserver` before delegating to the real `Performer`.
///
/// Uses the same `ProcessorState` and `Handler` — no dispatch duplication.
pub(super) struct ObservedPerformer<
    'a,
    H: Handler,
    T: Timeout,
    O: super::observer::PerformObserver,
> {
    pub(super) state: &'a mut ProcessorState<T>,
    pub(super) handler: &'a mut H,
    pub(super) observer: &'a mut O,
}

impl<'a, H: Handler + 'a, T: Timeout, O: super::observer::PerformObserver>
    ObservedPerformer<'a, H, T, O>
{
    #[inline]
    pub fn new<'b>(
        state: &'b mut ProcessorState<T>,
        handler: &'b mut H,
        observer: &'b mut O,
    ) -> ObservedPerformer<'b, H, T, O> {
        ObservedPerformer { state, handler, observer }
    }
}
