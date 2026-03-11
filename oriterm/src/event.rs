//! Application-level event types.
//!
//! [`TermEvent`] is the winit user-event type that flows from background
//! threads (PTY reader, config watcher, mux event proxy) into the main
//! event loop. Defined here rather than in `tab` so that non-tab modules
//! (like `config::monitor`) can reference it without backwards dependencies.

/// Events sent from background threads to the winit event loop.
///
/// The mux event proxy and config watcher produce these. The event loop
/// dispatches them in `user_event()`.
#[derive(Debug)]
pub(crate) enum TermEvent {
    /// The config file watcher detected a change.
    ConfigReload,
    /// The mux layer has events to process.
    ///
    /// Sent by the mux event proxy to wake the winit event loop when pane
    /// events arrive over the mpsc channel.
    MuxWakeup,
    /// Create a new window (keybinding action deferred to event loop).
    CreateWindow,
    /// Move a tab to a new window (context menu action deferred to event loop).
    MoveTabToNewWindow(usize),
    /// Open the settings window (deferred from overlay dispatch to event loop).
    OpenSettings,
    /// Open a confirmation dialog as a real OS window.
    ///
    /// Deferred to `user_event()` because dialog creation needs `&ActiveEventLoop`.
    OpenConfirmation(ConfirmationRequest),
}

/// Request to open a confirmation dialog window.
///
/// Carries everything needed to build the dialog content. The `kind` field
/// determines what happens when the user clicks OK.
#[derive(Debug)]
pub(crate) struct ConfirmationRequest {
    /// Dialog title bar text.
    pub title: String,
    /// Message body shown in the dialog.
    pub message: String,
    /// Optional content preview (e.g. clipboard text for paste confirmation).
    pub content: Option<String>,
    /// Label for the OK/confirm button.
    pub ok_label: String,
    /// Label for the Cancel button.
    pub cancel_label: String,
    /// What action to take when the user confirms.
    pub kind: ConfirmationKind,
}

/// Identifies the action to take when a confirmation dialog is accepted.
#[derive(Debug)]
pub(crate) enum ConfirmationKind {
    /// Paste multi-line text into the active terminal pane.
    Paste { text: String },
}
