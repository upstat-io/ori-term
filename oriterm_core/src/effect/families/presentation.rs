//! Presentation gate effects — Mode 2026 (synchronized output).

/// Presentation-layer effects for synchronized output (Mode 2026).
///
/// These effects control the commit/abort cycle that prevents tearing
/// during rapid application output.
#[derive(Debug, Clone, Copy)]
pub enum PresentationEffect {
    /// Begin synchronized update — buffer output until commit/abort.
    Begin,
    /// Commit buffered output as a single atomic frame.
    Commit { snapshot_seqno: u64 },
    /// Abort synchronized update — flush buffered output.
    Abort { reason: SyncAbortReason },
}

/// Why a synchronized update was aborted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncAbortReason {
    /// Application did not commit within the timeout window.
    Timeout,
    /// Buffered output exceeded the maximum allowed byte count.
    MaxBufferBytesExceeded,
    /// Application disconnected before committing.
    AppDisconnected,
}
