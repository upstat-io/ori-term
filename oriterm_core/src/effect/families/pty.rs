//! PTY write effects — bytes sent back to the child process.

/// A write to the PTY master fd (terminal → child process).
///
/// **Allocation note**: the current `Event::PtyWrite(String)` already
/// allocates per reply. This is cold-path (device queries, not per-cell).
/// The `bytes` field uses `Vec<u8>` rather than `String` because PTY replies
/// are byte-oriented (DCS responses contain raw bytes). If profiling shows
/// this matters, a future optimization can pool reply buffers — but PTY
/// replies are O(1) per query, never per-cell, so the allocation is not
/// on the hot path.
#[derive(Debug, Clone)]
pub enum PtyEffect {
    /// Write bytes back to the PTY (DA1/DA2/DA3 reply, CPR, DSR, DECRPM,
    /// DECRQSS reply, kitty image protocol ACK/error, mouse-encoded events,
    /// keyboard-encoded events, focus events, etc.).
    Write { bytes: Vec<u8>, kind: PtyWriteKind },
}

/// Classifies the purpose of a PTY write for diagnostics and filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtyWriteKind {
    DeviceAttribute,
    CursorReport,
    DeviceStatus,
    ModeReport,
    StatusString,
    ImageProtocolReply,
    MouseEvent,
    KeyboardEvent,
    FocusEvent,
    /// DECRQCRA (CSI * y) checksum reply — DCS Pi ! ~ XXXX ST.
    ChecksumReport,
    /// XTSMGRAPHICS (CSI ? Pi ; Pa ; Pv S) reply — `CSI ? Pi ; Ps [;Pv [;Pv2]] S`.
    GraphicsAttributeReport,
    /// ENQ (`0x05`) answerback reply — outbound bytes from the configured
    /// answerback string. Empty default suppresses emission entirely.
    Answerback,
    Other,
}

impl PtyWriteKind {
    /// Returns all `PtyWriteKind` variants as a static slice.
    /// This is the canonical source for PTY write kind coverage in tests —
    /// callers MUST iterate `Self::all()` rather than maintain parallel
    /// `let kinds = [...]` arrays, so a new variant lights up every consumer
    /// without manual sync.
    pub fn all() -> &'static [Self] {
        &[
            Self::DeviceAttribute,
            Self::CursorReport,
            Self::DeviceStatus,
            Self::ModeReport,
            Self::StatusString,
            Self::ImageProtocolReply,
            Self::MouseEvent,
            Self::KeyboardEvent,
            Self::FocusEvent,
            Self::ChecksumReport,
            Self::GraphicsAttributeReport,
            Self::Answerback,
            Self::Other,
        ]
    }
}
