//! Fire-and-forget host platform effects.
//!
//! These effects do not require a response from the host. The terminal
//! emits them and moves on — the host processes them asynchronously.

/// Fire-and-forget host platform effects.
#[derive(Debug, Clone)]
pub enum HostEffect {
    /// Audible bell (BEL, U+0007).
    Bell,
    /// Visual bell (DECVB) — separate variant, not a flag on Bell.
    /// Audible and visual bells route to different host consumers
    /// (audio adapter vs UI flash animator); dispatching on the variant
    /// is cleaner than forcing every consumer to check a flag.
    VisualBell,
    /// Desktop notification emitted by OSC 9 / OSC 99 / OSC 777.
    DesktopNotification {
        source: NotificationSource,
        title: String,
        body: String,
    },
    /// Window title set via OSC 0 / OSC 2. `None` resets to default.
    TitleSet { value: Option<String> },
    /// Icon name set via OSC 1. `None` resets to default.
    IconNameSet { value: Option<String> },
    /// Current working directory set via OSC 7.
    CwdSet { cwd: String },
    /// Audio playback request (DECPS / OSC audio extensions).
    AudioRequest(AudioRequest),
    /// Printer passthrough request (MC — media copy).
    PrintRequest(PrintRequest),
    /// Fire-and-forget clipboard write. No reply token needed — the host
    /// stores the data and the terminal does not observe a response.
    ClipboardStore {
        selection: ClipboardSelection,
        data: String,
    },
    /// Child process exited with the given status code.
    ChildExit { code: i32 },
    /// Shell command completed (shell integration duration tracking).
    CommandComplete { duration: std::time::Duration },
    /// Instructs the consumer to discard any queued desktop notifications.
    ///
    /// Emitted unconditionally by the RIS handler (reset to initial state).
    /// Each sink type handles this in its own `push()` arm:
    /// - `LegacyEventSink`: clears its internal `pending_notifications` queue.
    /// - `QueueingEffectSink`: queues the marker; the consumer discards
    ///   preceding `DesktopNotification` effects during `drain_into()`.
    /// - `VoidEffectSink`: no-op.
    ClearPendingNotifications,
}

/// Which OSC sequence originated a desktop notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationSource {
    /// OSC 9 (ConEmu/Windows Terminal style).
    Osc9,
    /// OSC 99 (kitty notification protocol).
    Osc99,
    /// OSC 777 (rxvt-unicode style).
    Osc777,
}

/// Audio playback request parameters.
#[derive(Debug, Clone)]
pub struct AudioRequest {
    pub kind: AudioKind,
    /// Volume level 0..=7 (DECPS scale). 0 = off.
    pub volume: u8,
    /// Duration in milliseconds. 0 = implementation-defined default.
    pub duration_ms: u16,
    /// DECPS note number (1..=25). Only meaningful for `AudioKind::Tone`.
    pub note: u8,
}

/// Audio effect type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioKind {
    /// Single tone (DECPS).
    Tone,
    /// Stop any playing audio.
    Stop,
}

/// Printer passthrough request (MC — media copy).
#[derive(Debug, Clone)]
pub struct PrintRequest {
    pub kind: PrintKind,
    pub data: Vec<u8>,
}

/// Print mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintKind {
    /// Print screen contents (MC 0 / CSI ? 1 i).
    Screen,
    /// Print current line (MC 1).
    Line,
    /// Start/stop passthrough to printer (MC 5 / MC 4).
    Passthrough,
}

/// Clipboard selection target for OSC 52.
///
/// Mirrors the OSC 52 clipboard character mapping:
/// `c` = clipboard, `p` = primary, `s` = select.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClipboardSelection {
    /// System clipboard (`c`).
    Clipboard,
    /// X11 primary selection (`p`).
    Primary,
    /// X11 select buffer (`s`).
    Select,
}
