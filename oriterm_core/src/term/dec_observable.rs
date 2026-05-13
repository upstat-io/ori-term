//! DEC private observable-state accessors.
//!
//! Holds the read-only accessor methods for state that is observable via
//! DEC private control sequences but does not gate parser dispatch:
//!
//! - DECSCA (`CSI Ps " q`) — per-character protection flag.
//! - DECSCL (`CSI Ps ; Ps " p`) — conformance level + C1 7-bit mode.
//! - DECSACE (`CSI Ps * x`) — attribute-change extent (stream vs rectangle).
//! - DECSASD (`CSI Ps $ }`) — active status display target.
//! - DECSSDT (`CSI Ps $ ~`) — status line type.
//!
//! The backing fields live on `Term<S>` in `mod.rs`; this module groups the
//! accessor surface and the `AceMode` enum so `mod.rs` stays under the
//! `code-hygiene.md §File Size` cap.

use crate::effect::sink::EffectSink;

use super::Term;

/// DECSACE attribute-change extent mode.
///
/// Selects how DECCARA / DECRARA distribute their effect across the
/// rectangle:
///
/// - `Stream` (Ps=0 or 1, default): wrap across row boundaries so only
///   the first row's left column and the last row's right column clamp
///   the stream; intermediate rows span the full width.
/// - `Rectangle` (Ps=2): strict rectangular block — every row is clipped
///   to `[left, right]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AceMode {
    /// Stream mode (DECSACE Ps=0 or 1).
    #[default]
    Stream,
    /// Rectangle mode (DECSACE Ps=2).
    Rectangle,
}

impl<S: EffectSink> Term<S> {
    /// DECSCA per-character protection flag (true = subsequent writes
    /// carry `CellFlags::PROTECTED`).
    pub fn char_protection(&self) -> bool {
        self.char_protection
    }

    /// DECSCL-reported conformance level (e.g. 64 = VT420).
    pub fn conformance_level(&self) -> u16 {
        self.conformance_level
    }

    /// DECSCL C1 mode: `true` when 7-bit C1 control encoding was
    /// selected (observable state; does not gate parser dispatch).
    pub fn c1_7bit(&self) -> bool {
        self.c1_7bit
    }

    /// DECSASD active status display target (0 = main, 1 = status).
    pub fn active_status_display(&self) -> u16 {
        self.active_status_display
    }

    /// DECSSDT status line type (0 = off, 1 = indicator, 2 = host).
    pub fn status_line_type(&self) -> u16 {
        self.status_line_type
    }

    /// DECSACE attribute-change extent mode (stream vs rectangle).
    ///
    /// Consumed by DECCARA / DECRARA when distributing SGR mutations
    /// across the rectangle. Default `Stream` matches xterm's Ps=0/1
    /// baseline; Ps=2 switches to strict `Rectangle` mode.
    pub fn ace_mode(&self) -> AceMode {
        self.ace_mode
    }
}
