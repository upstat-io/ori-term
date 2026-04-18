//! Boundary-crossing terminal side effects.
//!
//! `Effect` is the production interface for everything that LEAVES the terminal:
//! PTY writes, host platform calls (clipboard, audio, title, notification),
//! UI hints, and presentation gates (sync output). State changes (cells,
//! cursor, palette, modes, image placements, hyperlinks) are NOT effects —
//! they are observed via `RenderableContent` snapshots.

mod families;
mod response;
pub mod sink;
mod types;

pub use families::{
    AlreadyFulfilled, AudioKind, AudioRequest, ClipboardSelection, HostEffect, HostRequest,
    NotificationSource, PresentationEffect, PrintKind, PrintRequest, PtyEffect, PtyWriteKind,
    ResponseFulfilled, ResponseToken, SyncAbortReason, UiEffect, format_clipboard_reply,
    format_color_reply,
};
pub use response::{PendingResponse, PollResult};
pub use sink::legacy::DesktopNotificationRecord;
pub use sink::{EffectSink, LegacyEventSink, QueueingEffectSink, VoidEffectSink};
pub use types::Effect;

#[cfg(test)]
mod tests;
