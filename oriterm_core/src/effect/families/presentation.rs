//! Presentation gate effects — Mode 2026 (synchronized output).

/// Presentation-layer effects for synchronized output (Mode 2026).
///
/// Mode 2026 gates snapshot publication on `TermMode::SYNC_UPDATE`;
/// bytes inside a BSU/ESU window dispatch through the handler inline.
/// The only effect emitted from this surface is `Abort` — fired when
/// the run loop's deadline arm closes a sync window the app left open
/// past the 150 ms timeout.
#[derive(Debug, Clone, Copy)]
pub enum PresentationEffect {
    /// Sync window closed without an ESU — fired by `handle_sync_timeout`.
    Abort { reason: SyncAbortReason },
}

/// Why a synchronized update was aborted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncAbortReason {
    /// App did not send ESU within the sync window deadline.
    Timeout,
}
