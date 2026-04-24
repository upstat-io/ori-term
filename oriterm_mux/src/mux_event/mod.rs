//! Mux event types and the PTY-to-mux event bridge.
//!
//! [`MuxEvent`] carries pane events from PTY reader threads to the mux layer
//! via an mpsc channel. The IO thread routes effects via
//! `effect_router` → `MuxEvent` directly — no event-listener adapter
//! sits between the VTE handler and the channel.
//!
//! [`MuxNotification`] carries pane lifecycle notifications (output, closed,
//! title changes, bell). Clients drain these via the notification channel.

use std::fmt;

use oriterm_core::ClipboardType;
use oriterm_core::color::Rgb;
use oriterm_core::effect::{ClipboardSelection, NotificationSource, ResponseToken};

use crate::PaneId;

/// Events from pane PTY reader threads to the mux layer.
///
/// Sent over `mpsc::Sender<MuxEvent>`. The mux processes these on the main
/// thread during each event loop iteration.
///
/// **Adding a variant?** The event pump match in `in_process/event_pump.rs`
/// must be updated to handle it — the compiler enforces this via exhaustive
/// matching. Also update the `Debug` impl below and tests in `tests.rs`.
pub enum MuxEvent {
    /// Pane has new terminal output — grid is dirty.
    PaneOutput(PaneId),
    /// PTY process exited.
    PaneExited {
        /// Which pane's process exited.
        pane_id: PaneId,
        /// Exit code from the child process.
        exit_code: i32,
    },
    /// Pane title changed (OSC 0/2).
    PaneTitleChanged {
        /// Which pane changed title.
        pane_id: PaneId,
        /// New title text.
        title: String,
    },
    /// Pane icon name changed (OSC 0/1).
    PaneIconChanged {
        /// Which pane changed icon name.
        pane_id: PaneId,
        /// New icon name text.
        icon_name: String,
    },
    /// Pane working directory changed (OSC 7).
    PaneCwdChanged {
        /// Which pane changed CWD.
        pane_id: PaneId,
        /// New working directory path.
        cwd: String,
    },
    /// A command completed in a pane (OSC 133;D) with the given duration.
    CommandComplete {
        /// Which pane completed a command.
        pane_id: PaneId,
        /// Time elapsed between OSC 133;C and OSC 133;D.
        duration: std::time::Duration,
    },
    /// Bell fired in a pane.
    PaneBell(PaneId),
    /// Data to write to a pane's PTY (DA responses, etc.).
    ///
    /// Carries raw bytes rather than `String` so non-UTF-8 replies (binary
    /// Kitty Graphics replies, raw DA2 with `0x9C`, future binary protocols)
    /// survive the effect→MuxEvent boundary byte-exact. Today's OSC 52 /
    /// OSC 10–12 replies are ASCII so this is latent coverage, but the
    /// effect-side variant is `Vec<u8>`; silently downgrading to `String`
    /// at the mux boundary is an SSOT violation.
    PtyWrite {
        /// Target pane.
        pane_id: PaneId,
        /// Bytes to write.
        data: Vec<u8>,
    },
    /// OSC 52 clipboard store request.
    ClipboardStore {
        /// Originating pane.
        pane_id: PaneId,
        /// Which clipboard to target.
        clipboard_type: ClipboardType,
        /// Text to store.
        text: String,
    },
    /// Desktop notification (OSC 9 / 99 / 777).
    ///
    /// Added in effect-cutover 01.1: flows from the effect router to the
    /// main thread, where it is forwarded as
    /// [`MuxNotification::DesktopNotification`] for the window / desktop
    /// integration layer to display.
    DesktopNotification {
        /// Originating pane.
        pane_id: PaneId,
        /// Which OSC sequence produced the notification.
        source: NotificationSource,
        /// Notification title.
        title: String,
        /// Notification body.
        body: String,
    },
    /// Discard pending desktop notifications for a pane.
    ///
    /// Added in effect-cutover 01.1. Carries the contract of
    /// [`HostEffect::ClearPendingNotifications`] across the
    /// IO-thread→main-thread boundary so downstream staging buffers
    /// (`mux_pump`, `window_management`, daemon broadcast) can purge
    /// their queued [`MuxNotification::DesktopNotification`] entries
    /// for the originating pane. The event pump forwards each
    /// occurrence as [`MuxNotification::ClearPendingDesktopNotifications`].
    ClearPendingDesktopNotifications(PaneId),
    /// OSC 52 clipboard load request with a typed `ResponseToken`.
    ///
    /// Added in effect-cutover 01.1. The main thread reads the clipboard
    /// and calls `MuxBackend::fulfill_host_request` to route the reply
    /// back to the originating pane. Supersedes the closure-carrying
    /// the deleted closure-carrying load notification.
    HostClipboardLoad {
        /// Originating pane.
        pane_id: PaneId,
        /// Which clipboard to read.
        selection: ClipboardSelection,
        /// Raw OSC 52 clipboard character (e.g. `b'c'`, `b'p'`, `b's'`).
        clipboard_char: u8,
        /// OSC string terminator (ST or BEL) to use in the reply.
        terminator: String,
        /// Reply token — the consumer calls `.fulfill(text)` with the
        /// clipboard contents.
        reply: ResponseToken<String>,
    },
    /// OSC color query with a typed `ResponseToken`.
    ///
    /// Added in effect-cutover 01.1. The main thread looks up the color and
    /// fulfills the reply via `MuxBackend::fulfill_host_request`.
    HostColorQuery {
        /// Originating pane.
        pane_id: PaneId,
        /// OSC prefix string (`"4"`, `"10"`, `"11"`, `"12"`).
        prefix: String,
        /// Color index (for palette queries).
        index: usize,
        /// OSC string terminator (ST or BEL) to use in the reply.
        terminator: String,
        /// Reply token — the consumer calls `.fulfill(color)` with the
        /// requested `Rgb` value.
        reply: ResponseToken<Rgb>,
    },
    /// Next animation frame deadline for a pane.
    ///
    /// Emitted by the IO thread after `Term::advance_animations` runs and
    /// the next-deadline changed. The main thread forwards the instant to
    /// `RenderScheduler::request_frame_at` so the winit event loop's
    /// `ControlFlow::WaitUntil` wakes at the right instant to render the
    /// next frame. `None` means no animation is active (or none is
    /// viewport-visible); consumers can drop any previously-tracked
    /// deadline for the pane.
    AnimationDeadlineChanged {
        /// Originating pane.
        pane_id: PaneId,
        /// Next frame deadline, or `None` when no animation is active.
        deadline: Option<std::time::Instant>,
    },
}

impl fmt::Debug for MuxEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PaneOutput(id) => write!(f, "PaneOutput({id})"),
            Self::PaneExited { pane_id, exit_code } => {
                write!(f, "PaneExited({pane_id}, code={exit_code})")
            }
            Self::PaneTitleChanged { pane_id, title } => {
                write!(f, "PaneTitleChanged({pane_id}, {title:?})")
            }
            Self::PaneIconChanged { pane_id, icon_name } => {
                write!(f, "PaneIconChanged({pane_id}, {icon_name:?})")
            }
            Self::PaneCwdChanged { pane_id, cwd } => {
                write!(f, "PaneCwdChanged({pane_id}, {cwd:?})")
            }
            Self::CommandComplete { pane_id, duration } => {
                write!(f, "CommandComplete({pane_id}, {duration:?})")
            }
            Self::PaneBell(id) => write!(f, "PaneBell({id})"),
            Self::PtyWrite { pane_id, data } => {
                write!(f, "PtyWrite({pane_id}, {} bytes)", data.len())
            }
            Self::ClipboardStore {
                pane_id,
                clipboard_type,
                ..
            } => write!(f, "ClipboardStore({pane_id}, {clipboard_type:?})"),
            Self::DesktopNotification {
                pane_id,
                source,
                title,
                ..
            } => write!(f, "DesktopNotification({pane_id}, {source:?}, {title:?})"),
            Self::ClearPendingDesktopNotifications(id) => {
                write!(f, "ClearPendingDesktopNotifications({id})")
            }
            Self::HostClipboardLoad {
                pane_id, selection, ..
            } => write!(f, "HostClipboardLoad({pane_id}, {selection:?})"),
            Self::HostColorQuery {
                pane_id,
                prefix,
                index,
                ..
            } => write!(f, "HostColorQuery({pane_id}, {prefix:?}, index={index})"),
            Self::AnimationDeadlineChanged { pane_id, deadline } => {
                write!(
                    f,
                    "AnimationDeadlineChanged({pane_id}, deadline={:?})",
                    deadline.map(|d| d.saturating_duration_since(std::time::Instant::now())),
                )
            }
        }
    }
}

/// Pane lifecycle notifications from the mux layer.
///
/// These flow from the mux to clients after the mux has processed
/// incoming [`MuxEvent`]s and updated its state.
///
/// # Event-to-notification mapping
///
/// | `MuxEvent` variant | `MuxNotification` | Notes |
/// |---|---|---|
/// | `PaneOutput` | `PaneOutput` | 1:1 forwarding |
/// | `PaneExited` | `PaneClosed` | Carries exit code |
/// | `PaneTitleChanged` | `PaneMetadataChanged` | Collapsed with icon/CWD |
/// | `PaneIconChanged` | `PaneMetadataChanged` | Collapsed with title/CWD |
/// | `PaneCwdChanged` | `PaneMetadataChanged` | Collapsed with title/icon |
/// | `CommandComplete` | `CommandComplete` | 1:1 forwarding |
/// | `PaneBell` | `PaneBell` | 1:1 forwarding |
/// | `PtyWrite` | *(not forwarded)* | Handled inline (PTY write) |
/// | `ClipboardStore` | `ClipboardStore` | 1:1 forwarding |
/// | `HostClipboardLoad` | `HostClipboardLoad` | Carries `ResponseToken` |
/// | `HostColorQuery` | `HostColorQuery` | Carries `ResponseToken` |
pub enum MuxNotification {
    /// A pane's metadata changed (title, icon name, or CWD).
    ///
    /// Downstream consumers should re-read pane metadata rather than
    /// relying on which specific field changed.
    PaneMetadataChanged(PaneId),
    /// A pane has new content to render.
    PaneOutput(PaneId),
    /// A pane was closed (PTY exited, removed from registry).
    PaneClosed {
        /// Which pane was closed.
        pane_id: PaneId,
        /// Exit code from the child process (0 = clean exit).
        exit_code: i32,
    },
    /// A bell or urgent notification fired in a pane.
    PaneBell(PaneId),
    /// A long-running command completed in a pane.
    CommandComplete {
        /// Which pane completed a command.
        pane_id: PaneId,
        /// Command execution duration.
        duration: std::time::Duration,
    },
    /// OSC 52 clipboard store request forwarded from a pane.
    ClipboardStore {
        /// Originating pane.
        pane_id: PaneId,
        /// Which clipboard to target.
        clipboard_type: ClipboardType,
        /// Text to store.
        text: String,
    },
    /// Desktop notification (OSC 9 / 99 / 777) forwarded from a pane.
    ///
    /// Added in effect-cutover 01.1. The receiving client displays the
    /// notification via the platform-native notification API.
    DesktopNotification {
        /// Originating pane.
        pane_id: PaneId,
        /// Which OSC sequence produced the notification.
        source: NotificationSource,
        /// Notification title.
        title: String,
        /// Notification body.
        body: String,
    },
    /// Purge any queued desktop notifications for a pane (OSC `RIS` reset).
    ///
    /// Added in effect-cutover 01.1. Every downstream staging buffer that
    /// holds `DesktopNotification` notifications for `pane_id` MUST discard
    /// them when this variant arrives.
    ClearPendingDesktopNotifications(PaneId),
    /// OSC 52 clipboard load forwarded with a typed `ResponseToken`.
    ///
    /// Added in effect-cutover 01.1. The main thread reads the clipboard
    /// and calls `MuxBackend::fulfill_host_request` with a
    /// `HostReply::ClipboardLoad` payload; the IO thread's pending-response
    /// poll then emits the formatted PTY reply.
    ///
    /// # Move-only across staging buffers
    ///
    /// The `reply` field carries an `Arc<Mutex<Option<String>>>`. Every
    /// downstream staging-buffer hop MUST move the notification (via
    /// `Vec::drain`, `mem::replace`, match-move) rather than `.clone()` it.
    /// Cloning defeats the `Arc::strong_count`-based cancellation detection
    /// in `PendingResponse`. See
    /// `oriterm_core::effect::ResponseToken` doc comment for the SSOT.
    HostClipboardLoad {
        /// Originating pane.
        pane_id: PaneId,
        /// Which clipboard to read.
        selection: ClipboardSelection,
        /// Raw OSC 52 clipboard character.
        clipboard_char: u8,
        /// OSC string terminator (ST or BEL).
        terminator: String,
        /// Reply token — consumer calls `.fulfill(text)` with the
        /// clipboard contents.
        reply: ResponseToken<String>,
    },
    /// OSC color query forwarded with a typed `ResponseToken`.
    ///
    /// Added in effect-cutover 01.1. Same move-only discipline as
    /// [`MuxNotification::HostClipboardLoad`].
    HostColorQuery {
        /// Originating pane.
        pane_id: PaneId,
        /// OSC prefix string.
        prefix: String,
        /// Color index (palette queries).
        index: usize,
        /// OSC string terminator.
        terminator: String,
        /// Reply token — consumer calls `.fulfill(color)` with the
        /// requested `Rgb` value.
        reply: ResponseToken<Rgb>,
    },
    /// Another process requested a new tab via the daemon.
    ///
    /// The receiving client should create a new tab in its active window
    /// using its own configuration.
    NewTab,
    /// Next animation frame deadline for a pane (kitty graphics `a=f`/`a=a`
    /// and future sixel-animation sources). The receiving client forwards
    /// `deadline` to `RenderScheduler::request_frame_at` so the winit
    /// event loop wakes at the right instant. `None` means no animation
    /// is active; consumers drop any previously-tracked deadline for the
    /// pane.
    AnimationDeadlineChanged {
        /// Originating pane.
        pane_id: PaneId,
        /// Next frame deadline, or `None` when no animation is active.
        deadline: Option<std::time::Instant>,
    },
}

impl fmt::Debug for MuxNotification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PaneMetadataChanged(id) => write!(f, "PaneMetadataChanged({id})"),
            Self::PaneOutput(id) => write!(f, "PaneOutput({id})"),
            Self::PaneClosed { pane_id, exit_code } => {
                write!(f, "PaneClosed({pane_id}, code={exit_code})")
            }
            Self::PaneBell(id) => write!(f, "PaneBell({id})"),
            Self::CommandComplete { pane_id, duration } => {
                write!(f, "CommandComplete({pane_id}, {duration:?})")
            }
            Self::ClipboardStore {
                pane_id,
                clipboard_type,
                ..
            } => write!(f, "ClipboardStore({pane_id}, {clipboard_type:?})"),
            Self::DesktopNotification {
                pane_id,
                source,
                title,
                ..
            } => write!(f, "DesktopNotification({pane_id}, {source:?}, {title:?})"),
            Self::ClearPendingDesktopNotifications(id) => {
                write!(f, "ClearPendingDesktopNotifications({id})")
            }
            Self::HostClipboardLoad {
                pane_id, selection, ..
            } => write!(f, "HostClipboardLoad({pane_id}, {selection:?})"),
            Self::HostColorQuery {
                pane_id,
                prefix,
                index,
                ..
            } => write!(f, "HostColorQuery({pane_id}, {prefix:?}, index={index})"),
            Self::NewTab => write!(f, "NewTab"),
            Self::AnimationDeadlineChanged { pane_id, deadline } => {
                write!(
                    f,
                    "AnimationDeadlineChanged({pane_id}, deadline={:?})",
                    deadline.map(|d| d.saturating_duration_since(std::time::Instant::now())),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests;
