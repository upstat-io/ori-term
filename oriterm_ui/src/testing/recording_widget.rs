//! Reusable input-recording widget for harness regression tests.
//!
//! [`RecordingWidget`] records every [`InputEvent`] it receives via
//! `on_input`. The paired [`RecordedEvents`] handle exposes count,
//! last-event, and all-events queries without consumers writing
//! `borrow()` boilerplate. Replaces the `FocusFallthroughProbe` pattern
//! that was hand-rolled per regression test pinning a fall-through /
//! pipeline-routing gate.

use std::cell::RefCell;
use std::rc::Rc;

use crate::geometry::Rect;
use crate::input::InputEvent;
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::widget_id::WidgetId;
use crate::widgets::{LayoutCtx, OnInputResult, Widget};

/// Default recording-widget layout width.
///
/// `pub(super)` so the sibling `testing/tests.rs` can read it for T15
/// (layout-dimension regression pin). Not exposed beyond `testing::`.
pub(super) const RECORDING_WIDGET_WIDTH: f32 = 120.0;
/// Default recording-widget layout height.
pub(super) const RECORDING_WIDGET_HEIGHT: f32 = 40.0;

/// Test-only widget that captures every [`InputEvent`] reaching `on_input`.
///
/// Pair with [`RecordedEvents`] returned by [`RecordingWidget::new`] to
/// assert which events were delivered to the widget's `on_input` callback.
/// Does NOT override `handle_keymap_action` — the trait default returning
/// `None` applies, mirroring `DialogWidget` and matching the contract
/// that overlay keymap fall-through gates depend on.
pub struct RecordingWidget {
    id: WidgetId,
    key_context: Option<&'static str>,
    sense: Sense,
    events: Rc<RefCell<Vec<InputEvent>>>,
}

/// Cloneable handle to a [`RecordingWidget`]'s recorded event log.
///
/// Returned alongside the widget at construction time because the widget
/// itself is typically moved into `Box<dyn Widget>` and consumed by overlay
/// or window-root APIs. The handle stays with the test for assertions.
#[derive(Clone)]
pub struct RecordedEvents(Rc<RefCell<Vec<InputEvent>>>);

impl RecordedEvents {
    /// Total number of recorded events (all variants).
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.borrow().len()
    }

    /// Returns `true` if no events have been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
    }

    /// Number of `KeyDown` events recorded.
    #[must_use]
    pub fn count_keydowns(&self) -> usize {
        self.0
            .borrow()
            .iter()
            .filter(|e| matches!(e, InputEvent::KeyDown { .. }))
            .count()
    }

    /// Most recent recorded event, or `None` if none have been recorded.
    #[must_use]
    pub fn last_event(&self) -> Option<InputEvent> {
        self.0.borrow().last().copied()
    }

    /// Snapshot of all recorded events, in observation order.
    #[must_use]
    pub fn all(&self) -> Vec<InputEvent> {
        self.0.borrow().clone()
    }
}

impl RecordingWidget {
    /// Creates a recording widget paired with its events handle.
    ///
    /// The widget moves into the harness; the cloneable [`RecordedEvents`]
    /// handle stays with the test for assertions on what reached `on_input`.
    #[must_use]
    pub fn new(key_context: Option<&'static str>, sense: Sense) -> (Self, RecordedEvents) {
        let events = Rc::new(RefCell::new(Vec::new()));
        let widget = Self {
            id: WidgetId::next(),
            key_context,
            sense,
            events: Rc::clone(&events),
        };
        (widget, RecordedEvents(events))
    }
}

impl Widget for RecordingWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
        LayoutBox::leaf(RECORDING_WIDGET_WIDTH, RECORDING_WIDGET_HEIGHT).with_widget_id(self.id)
    }

    fn key_context(&self) -> Option<&'static str> {
        self.key_context
    }

    fn sense(&self) -> Sense {
        self.sense
    }

    fn on_input(&mut self, event: &InputEvent, _bounds: Rect) -> OnInputResult {
        self.events.borrow_mut().push(*event);
        OnInputResult::handled()
    }
}
