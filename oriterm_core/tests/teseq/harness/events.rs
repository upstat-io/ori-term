//! Structured event capture for teseq test assertions.
//!
//! `RecordedEvent` mirrors `oriterm_core::event::Event` but replaces closures
//! with their identifying data, enabling equality comparison and snapshots.

use std::sync::{Arc, Mutex};

use oriterm_core::ClipboardType;
use oriterm_core::event::{Event, EventListener};

/// Structured event capture for test assertions.
///
/// Mirrors `oriterm_core::event::Event` but replaces closures with
/// their identifying data, enabling equality comparison and snapshots.
#[derive(Clone, Debug, PartialEq)]
pub enum RecordedEvent {
    /// New content available.
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
    /// OSC 52 clipboard load request (closure stripped).
    ClipboardLoad(ClipboardType),
    /// OSC 4/10/11 color query response (closure stripped).
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

// Exhaustive match ensures RecordedEvent stays in sync with Event.
// When a new Event variant is added, this match will fail to compile,
// forcing the implementer to add a corresponding RecordedEvent variant.
impl From<&Event> for RecordedEvent {
    fn from(event: &Event) -> Self {
        match event {
            Event::Wakeup => Self::Wakeup,
            Event::Bell => Self::Bell,
            Event::Title(t) => Self::Title(t.clone()),
            Event::ResetTitle => Self::ResetTitle,
            Event::IconName(n) => Self::IconName(n.clone()),
            Event::ResetIconName => Self::ResetIconName,
            Event::ClipboardStore(ty, text) => Self::ClipboardStore(*ty, text.clone()),
            Event::ClipboardLoad(ty, _) => Self::ClipboardLoad(*ty),
            Event::ColorRequest(idx, _) => Self::ColorRequest(*idx),
            Event::PtyWrite(s) => Self::PtyWrite(s.clone()),
            Event::CursorBlinkingChange => Self::CursorBlinkingChange,
            Event::Cwd(path) => Self::Cwd(path.clone()),
            Event::CommandComplete(_) => Self::CommandComplete,
            Event::MouseCursorDirty => Self::MouseCursorDirty,
            Event::ChildExit(code) => Self::ChildExit(*code),
        }
    }
}

/// Event listener that captures structured `RecordedEvent`s.
#[derive(Clone)]
pub struct RecordedListener {
    events: Arc<Mutex<Vec<RecordedEvent>>>,
}

impl RecordedListener {
    /// Create a new listener with an empty event buffer.
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// All captured events.
    pub fn events(&self) -> Vec<RecordedEvent> {
        self.events.lock().expect("lock poisoned").clone()
    }

    /// Only `PtyWrite` events (response bytes).
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

impl EventListener for RecordedListener {
    fn send_event(&self, event: Event) {
        self.events
            .lock()
            .expect("lock poisoned")
            .push(RecordedEvent::from(&event));
    }
}
