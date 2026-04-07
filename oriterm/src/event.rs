//! Application-level event types.
//!
//! [`TermEvent`] is the winit user-event type that flows from background
//! threads (PTY reader, config watcher, mux event proxy, GPU device-lost
//! callback) into the main event loop. Defined here rather than in `tab` so
//! that non-tab modules (like `config::monitor`) can reference it without
//! backwards dependencies.

use crate::gpu::recovery::GpuLossReason;

/// Events sent from background threads to the winit event loop.
///
/// The mux event proxy, config watcher, and the wgpu device-lost callback
/// produce these. The event loop dispatches them in `user_event()`.
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
    MoveTabToNewWindow(crate::session::TabId),
    /// Open the settings window (deferred from overlay dispatch to event loop).
    OpenSettings,
    /// Open a confirmation dialog as a real OS window.
    ///
    /// Deferred to `user_event()` because dialog creation needs `&ActiveEventLoop`.
    OpenConfirmation(ConfirmationRequest),
    /// GPU device lost — recovery dispatcher should run.
    ///
    /// Sent by the `wgpu::Device::set_device_lost_callback` registered after
    /// every successful `Adapter::request_device(...)`, and synthesized by
    /// the render path when `SurfaceError::Lost`/`Other`/`OutOfMemory` is
    /// observed during a frame. The handler in `event_loop.rs` records the
    /// event; the actual `App::recover_gpu()` state machine lands in 5.16.2.
    GpuDeviceLost {
        /// Why the device was lost. Drives the 5.16.10 backoff and OOM
        /// short-circuit branches.
        reason: GpuLossReason,
        /// Diagnostic message captured at the loss site.
        message: String,
    },
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
