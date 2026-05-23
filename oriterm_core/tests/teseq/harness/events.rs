//! Structured effect capture for teseq test assertions.
//!
//! `RecordedEvent` provides a closure-free, equality-comparable view of
//! the [`Effect`] stream emitted by `Term<RecordedListener>` — teseq tests
//! historically asserted on `RecordedEvent`-shaped variants, and this
//! shim preserves the same shape after the effect-cutover migration
//! (see `plans/effect-cutover/`).

use std::sync::{Arc, Mutex};

use oriterm_core::ClipboardType;
use oriterm_core::effect::sink::EffectSink;
use oriterm_core::effect::{
    ClipboardSelection, Effect, HostEffect, HostRequest, PtyEffect, UiEffect,
};

/// Structured effect capture for test assertions.
///
/// Equality-comparable mirror of the `Effect` variants the teseq harness
/// observes; closures and sources of non-determinism (durations,
/// internal sources) are stripped so snapshots stay stable.
#[derive(Clone, Debug, PartialEq)]
#[allow(
    dead_code,
    reason = "exhaustive enum surface; some variants are only emitted by tests not yet migrated"
)]
pub enum RecordedEvent {
    /// New content available — set whenever a `UiEffect::CursorBlinkChanged` fires
    /// or whenever the harness records a wakeup-like signal.
    Wakeup,
    /// BEL character received.
    Bell,
    /// Window title changed (OSC 0/2).
    Title(String),
    /// Window title reset to default.
    ResetTitle,
    /// Icon name changed (OSC 0/1).
    IconName(String),
    /// Icon name reset to default.
    ResetIconName,
    /// OSC 52 clipboard store request.
    ClipboardStore(ClipboardType, String),
    /// OSC 52 clipboard load request (token stripped).
    ClipboardLoad(ClipboardType),
    /// OSC 4/10/11/12 color query (token stripped).
    ColorRequest(usize),
    /// Response bytes to write back to PTY.
    PtyWrite(String),
    /// Cursor blink state toggled.
    CursorBlinkingChange,
    /// Current working directory changed (OSC 7 via mux RawInterceptor).
    Cwd(String),
    /// Command completed (duration stripped — non-deterministic).
    CommandComplete,
    /// Mouse cursor shape may need update.
    MouseCursorDirty,
    /// Child process exited.
    ChildExit(i32),
}

fn clipboard_from_selection(selection: ClipboardSelection) -> ClipboardType {
    match selection {
        ClipboardSelection::Clipboard => ClipboardType::Clipboard,
        ClipboardSelection::Primary | ClipboardSelection::Select => ClipboardType::Selection,
    }
}

fn record_effect(effect: Effect) -> Option<RecordedEvent> {
    Some(match effect {
        Effect::Host(HostEffect::Bell) => RecordedEvent::Bell,
        Effect::Host(HostEffect::TitleSet { value: Some(t) }) => RecordedEvent::Title(t),
        Effect::Host(HostEffect::TitleSet { value: None }) => RecordedEvent::ResetTitle,
        Effect::Host(HostEffect::IconNameSet { value: Some(n) }) => RecordedEvent::IconName(n),
        Effect::Host(HostEffect::IconNameSet { value: None }) => RecordedEvent::ResetIconName,
        Effect::Host(HostEffect::ClipboardStore { selection, data }) => {
            RecordedEvent::ClipboardStore(clipboard_from_selection(selection), data)
        }
        Effect::Host(HostEffect::CwdSet { cwd }) => RecordedEvent::Cwd(cwd),
        Effect::Host(HostEffect::CommandComplete { .. }) => RecordedEvent::CommandComplete,
        Effect::Host(HostEffect::ChildExit { code }) => RecordedEvent::ChildExit(code),
        Effect::Pty(PtyEffect::Write { bytes, .. }) => {
            RecordedEvent::PtyWrite(String::from_utf8_lossy(&bytes).into_owned())
        }
        Effect::Ui(UiEffect::CursorBlinkChanged { .. }) => RecordedEvent::CursorBlinkingChange,
        Effect::Ui(UiEffect::MouseCursorDirty) => RecordedEvent::MouseCursorDirty,
        Effect::HostRequest(HostRequest::ClipboardLoad { selection, .. }) => {
            RecordedEvent::ClipboardLoad(clipboard_from_selection(selection))
        }
        Effect::HostRequest(HostRequest::ColorQuery { index, .. }) => {
            RecordedEvent::ColorRequest(index)
        }
        Effect::Host(HostEffect::DesktopNotification { .. })
        | Effect::Host(HostEffect::ClearPendingNotifications)
        | Effect::Host(HostEffect::AudioRequest(_))
        | Effect::Host(HostEffect::PrintRequest(_))
        | Effect::Host(HostEffect::UrgencyHint)
        | Effect::Presentation(_) => return None,
    })
}

/// Effect sink that captures structured [`RecordedEvent`]s.
#[derive(Clone, Default)]
pub struct RecordedListener {
    events: Arc<Mutex<Vec<RecordedEvent>>>,
}

impl RecordedListener {
    /// Create a new sink with an empty event buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// All captured events.
    pub fn events(&self) -> Vec<RecordedEvent> {
        self.events.lock().expect("lock poisoned").clone()
    }

    /// Only `PtyWrite` payloads (response bytes).
    pub fn pty_writes(&self) -> Vec<String> {
        self.events
            .lock()
            .expect("lock poisoned")
            .iter()
            .filter_map(|e| {
                if let RecordedEvent::PtyWrite(s) = e {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Clear all captured events.
    pub fn clear(&self) {
        self.events.lock().expect("lock poisoned").clear();
    }
}

impl EffectSink for RecordedListener {
    fn push(&self, effect: Effect) {
        if let Some(rec) = record_effect(effect) {
            self.events.lock().expect("lock poisoned").push(rec);
        }
    }

    fn drain_into(&self, _out: &mut Vec<Effect>) {}
}
