//! Wire types for daemon-mode `HostRequest` IPC round-trip ().
//!
//! The originating `ResponseToken<T>` is process-local
//! (`Arc<Mutex<...>>`); the daemon allocates a server-monotonic
//! `request_id`, packages the OSC parameters into a `Notify…` PDU, and
//! waits for a matching `ReplyHostRequest` from the responding client.
//! These wire types encode the cross-process payload — clipboard
//! selection, notification source, and reply contents — without leaking
//! the in-process token.

use serde::{Deserialize, Serialize};

use oriterm_core::effect::{ClipboardSelection, NotificationSource};

/// Wire encoding of an OSC 52 clipboard selection.
///
/// Stable `#[repr(u8)]` discriminants decoupled from
/// `oriterm_core::effect::ClipboardSelection`. Decoding accepts unknown
/// values by defaulting to `Clipboard` (forward-compat — matches the
/// `WireCursorShape::from_u8` pattern in `snapshot.rs`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum WireClipboardSelection {
    /// Default OSC 52 selection — `c`.
    #[default]
    Clipboard = 0,
    /// `p` (primary X selection).
    Primary = 1,
    /// `s` (cut buffer / select).
    Select = 2,
}

impl WireClipboardSelection {
    /// Construct from raw `u8`, defaulting to `Clipboard` for unknown values.
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Primary,
            2 => Self::Select,
            _ => Self::Clipboard,
        }
    }
}

impl From<ClipboardSelection> for WireClipboardSelection {
    fn from(s: ClipboardSelection) -> Self {
        match s {
            ClipboardSelection::Clipboard => Self::Clipboard,
            ClipboardSelection::Primary => Self::Primary,
            ClipboardSelection::Select => Self::Select,
        }
    }
}

impl From<WireClipboardSelection> for ClipboardSelection {
    fn from(w: WireClipboardSelection) -> Self {
        match w {
            WireClipboardSelection::Clipboard => Self::Clipboard,
            WireClipboardSelection::Primary => Self::Primary,
            WireClipboardSelection::Select => Self::Select,
        }
    }
}

/// Wire encoding of the OSC sequence that produced a desktop notification.
///
/// Stable `#[repr(u8)]` discriminants decoupled from
/// `oriterm_core::effect::NotificationSource`. Decoding defaults to `Osc9`
/// for unknown values.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum WireNotificationSource {
    /// OSC 9 — `iTerm2`-style.
    #[default]
    Osc9 = 0,
    /// OSC 99 — kitty notification protocol.
    Osc99 = 1,
    /// OSC 777 — urxvt notification protocol.
    Osc777 = 2,
}

impl WireNotificationSource {
    /// Construct from raw `u8`, defaulting to `Osc9` for unknown values.
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Osc99,
            2 => Self::Osc777,
            _ => Self::Osc9,
        }
    }
}

impl From<NotificationSource> for WireNotificationSource {
    fn from(s: NotificationSource) -> Self {
        match s {
            NotificationSource::Osc9 => Self::Osc9,
            NotificationSource::Osc99 => Self::Osc99,
            NotificationSource::Osc777 => Self::Osc777,
        }
    }
}

impl From<WireNotificationSource> for NotificationSource {
    fn from(w: WireNotificationSource) -> Self {
        match w {
            WireNotificationSource::Osc9 => Self::Osc9,
            WireNotificationSource::Osc99 => Self::Osc99,
            WireNotificationSource::Osc777 => Self::Osc777,
        }
    }
}

/// Payload for `MuxPdu::ReplyHostRequest`.
///
/// The two variants mirror the two `HostReply` shapes the App fulfills: an
/// OSC 52 clipboard read returns the clipboard text; an OSC 4 / 10 / 11 / 12
/// color query returns the resolved RGB triple. Mismatch between the
/// pending request kind and the reply payload is logged + dropped on the
/// daemon side (routing bug, not a wire-format violation).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostReplyPayload {
    /// Clipboard text fulfilled by the responding client.
    ClipboardLoad {
        /// Decoded clipboard text — routed back to the originating pane.
        text: String,
    },
    /// Resolved RGB color fulfilled by the responding client.
    ColorQuery {
        /// `[r, g, b]` triple.
        rgb: [u8; 3],
    },
}

#[cfg(test)]
mod tests;
