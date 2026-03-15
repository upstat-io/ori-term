//! Core widget types and traits for the UI framework.
//!
//! Provides the [`Widget`] trait, action/response types, and context structs
//! that widgets use during layout, drawing, and event handling. Each widget
//! is a concrete struct implementing `Widget`. Trait objects (`Box<dyn Widget>`)
//! are used for dynamic dispatch in overlay and container contexts.

pub mod text_measurer;

pub mod button;
pub mod checkbox;
pub mod container;
pub mod dialog;
pub mod dropdown;
pub mod form_layout;
pub mod form_row;
pub mod form_section;
pub mod label;
pub mod menu;
pub mod panel;
pub mod scroll;
pub mod separator;
pub mod settings_panel;
pub mod slider;
pub mod spacer;
pub mod stack;
pub mod status_badge;
pub mod tab_bar;
pub mod text_input;
pub mod toggle;
pub mod window_chrome;

use std::cell::Cell;
use std::time::Instant;

use crate::draw::{DrawList, SceneCache};
use crate::geometry::Rect;
use crate::icons::ResolvedIcons;
use crate::input::{EventResponse, HoverEvent, KeyEvent, MouseEvent, MouseEventKind};
use crate::layout::LayoutBox;
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;

pub use text_measurer::TextMeasurer;

/// Whether a widget wants to acquire or release mouse capture.
///
/// Capture is a routing directive: when a widget acquires capture, all
/// subsequent mouse events (Move, Up) are routed to that widget regardless
/// of cursor position, until capture is released.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaptureRequest {
    /// No capture change requested.
    #[default]
    None,
    /// Request mouse capture for the responding widget.
    Acquire,
    /// Release any active mouse capture.
    Release,
}

impl CaptureRequest {
    /// Whether the capture should be released based on the response and event.
    ///
    /// Returns `true` on explicit `Release`, or on `None` with a mouse-up
    /// event (implicit release when the child didn't request anything).
    pub fn should_release(self, event_kind: &MouseEventKind) -> bool {
        matches!(self, Self::Release)
            || (matches!(self, Self::None) && matches!(event_kind, MouseEventKind::Up(_)))
    }
}

/// How a widget responded to an event, including an optional semantic action.
///
/// Widgets return this from event handlers. The `response` field tells the
/// framework how to handle propagation; the `action` field carries semantic
/// meaning for the application layer.
#[derive(Debug, Clone, PartialEq)]
pub struct WidgetResponse {
    /// How the framework should handle this event.
    pub response: EventResponse,
    /// Optional semantic action for the application layer to interpret.
    pub action: Option<WidgetAction>,
    /// Whether the widget wants to acquire or release mouse capture.
    pub capture: CaptureRequest,
    /// The widget that produced this response (for invalidation tracking).
    ///
    /// Set by container-side injection: containers fill this with the child's
    /// `WidgetId` after receiving a response. Widgets themselves leave it `None`.
    pub source: Option<WidgetId>,
}

impl WidgetResponse {
    /// Event handled, no action emitted.
    pub fn handled() -> Self {
        Self {
            response: EventResponse::Handled,
            action: None,
            capture: CaptureRequest::None,
            source: None,
        }
    }

    /// Event ignored — propagate to parent.
    pub fn ignored() -> Self {
        Self {
            response: EventResponse::Ignored,
            action: None,
            capture: CaptureRequest::None,
            source: None,
        }
    }

    /// Visual-only change (hover color, focus ring). Repaint needed, no relayout.
    pub fn paint() -> Self {
        Self {
            response: EventResponse::RequestPaint,
            action: None,
            capture: CaptureRequest::None,
            source: None,
        }
    }

    /// Structural change (text content, visibility). Relayout + repaint needed.
    pub fn layout() -> Self {
        Self {
            response: EventResponse::RequestLayout,
            action: None,
            capture: CaptureRequest::None,
            source: None,
        }
    }

    /// Event handled, focus requested, no action.
    pub fn focus() -> Self {
        Self {
            response: EventResponse::RequestFocus,
            action: None,
            capture: CaptureRequest::None,
            source: None,
        }
    }

    /// Attaches an action to this response.
    #[must_use]
    pub fn with_action(mut self, action: WidgetAction) -> Self {
        self.action = Some(action);
        self
    }

    /// Requests mouse capture for the responding widget.
    #[must_use]
    pub fn with_capture(mut self) -> Self {
        self.capture = CaptureRequest::Acquire;
        self
    }

    /// Requests release of any active mouse capture.
    #[must_use]
    pub fn with_release_capture(mut self) -> Self {
        self.capture = CaptureRequest::Release;
        self
    }

    /// Sets the source widget ID (for invalidation tracking).
    ///
    /// Normally set by container-side injection rather than individual widgets.
    #[must_use]
    pub fn with_source(mut self, id: WidgetId) -> Self {
        self.source = Some(id);
        self
    }

    /// Sets the source widget ID if not already set.
    ///
    /// Used by containers to inject the child's ID without overwriting
    /// a source that a nested container already set.
    pub fn inject_source(&mut self, id: WidgetId) {
        if self.source.is_none() {
            self.source = Some(id);
        }
    }

    /// Marks the tracker if this response carries a dirty source.
    ///
    /// Convenience for the application layer: extracts `source` and
    /// `DirtyKind` from the response and calls `tracker.mark()`.
    pub fn mark_tracker(&self, tracker: &mut crate::invalidation::InvalidationTracker) {
        if let Some(source) = self.source {
            let kind = crate::invalidation::DirtyKind::from(self.response);
            tracker.mark(source, kind);
        }
    }
}

/// A semantic action emitted by a widget for the application layer.
///
/// No closures — the app layer matches on variants and interprets them.
/// This keeps widgets stateless with respect to application logic.
#[derive(Debug, Clone, PartialEq)]
pub enum WidgetAction {
    /// A button or clickable widget was activated.
    Clicked(WidgetId),
    /// A boolean value was toggled (checkbox, toggle switch).
    Toggled { id: WidgetId, value: bool },
    /// A numeric value changed (slider).
    ValueChanged { id: WidgetId, value: f32 },
    /// Text content changed (text input).
    TextChanged { id: WidgetId, text: String },
    /// An item was selected by index (dropdown, menu).
    Selected { id: WidgetId, index: usize },
    /// A dropdown trigger requests opening its popup list.
    OpenDropdown {
        /// The dropdown widget's ID (for routing selection back).
        id: WidgetId,
        /// Option labels.
        options: Vec<String>,
        /// Currently selected index.
        selected: usize,
        /// Screen-space anchor rect for popup placement.
        anchor: Rect,
    },
    /// An overlay content widget requests its own dismissal.
    DismissOverlay(WidgetId),
    /// An overlay widget requests repositioning (e.g. header drag).
    MoveOverlay { delta_x: f32, delta_y: f32 },
    /// The settings panel Save button was clicked — persist and dismiss.
    SaveSettings,
    /// The settings panel Cancel button was clicked — revert and dismiss.
    CancelSettings,
    /// Minimize the window.
    WindowMinimize,
    /// Maximize or restore the window.
    WindowMaximize,
    /// Close the window.
    WindowClose,
}

/// Context passed to [`Widget::layout`].
pub struct LayoutCtx<'a> {
    /// Text measurement provider.
    pub measurer: &'a dyn TextMeasurer,
    /// Active UI theme.
    pub theme: &'a UiTheme,
}

/// Context passed to [`Widget::draw`].
pub struct DrawCtx<'a> {
    /// Text shaping provider.
    pub measurer: &'a dyn TextMeasurer,
    /// The draw command list to append to.
    pub draw_list: &'a mut DrawList,
    /// The widget's computed bounds (from layout).
    pub bounds: Rect,
    /// The currently focused widget, if any.
    pub focused_widget: Option<WidgetId>,
    /// Current frame timestamp for animation interpolation.
    pub now: Instant,
    /// Set to `true` by widgets with running animations to request redraw.
    pub animations_running: &'a Cell<bool>,
    /// Active UI theme.
    pub theme: &'a UiTheme,
    /// Pre-resolved icon atlas entries for this frame.
    ///
    /// `None` in tests or when the GPU renderer is not available.
    /// Widgets fall back to `push_line()` when this is `None`.
    pub icons: Option<&'a ResolvedIcons>,
    /// Per-widget scene cache for retained rendering.
    ///
    /// `None` during uncached draws (tests, first frame). When present,
    /// container widgets check the cache before calling `child.draw()`.
    pub scene_cache: Option<&'a mut SceneCache>,
}

/// Context passed to mouse and keyboard event handlers.
pub struct EventCtx<'a> {
    /// Text measurement provider.
    pub measurer: &'a dyn TextMeasurer,
    /// The widget's computed bounds (from layout).
    pub bounds: Rect,
    /// Whether this widget currently has keyboard focus.
    pub is_focused: bool,
    /// The currently focused widget, if any.
    ///
    /// Containers use this to set per-child `is_focused` correctly,
    /// so only the focused child responds to key events.
    pub focused_widget: Option<WidgetId>,
    /// Active UI theme.
    pub theme: &'a UiTheme,
}

impl EventCtx<'_> {
    /// Build a child context with child-specific bounds and focus state.
    ///
    /// `child_id` determines whether the child is focused (compared against
    /// `self.focused_widget`). Pass `None` for non-focusable children.
    #[must_use]
    pub fn for_child(&self, child_bounds: Rect, child_id: Option<WidgetId>) -> Self {
        Self {
            measurer: self.measurer,
            bounds: child_bounds,
            is_focused: child_id.is_some_and(|id| self.focused_widget == Some(id)),
            focused_widget: self.focused_widget,
            theme: self.theme,
        }
    }
}

/// The core widget trait.
///
/// Each widget is a concrete struct that implements this trait. Widgets
/// own their visual state (hovered, pressed) and app state (checked, value),
/// plus a style struct with `Default` dark-theme defaults.
pub trait Widget {
    /// Returns this widget's unique identifier.
    fn id(&self) -> WidgetId;

    /// Whether this widget can receive keyboard focus.
    fn is_focusable(&self) -> bool;

    /// Builds a layout descriptor for the layout solver.
    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox;

    /// Draws the widget into the draw list.
    fn draw(&self, ctx: &mut DrawCtx<'_>);

    /// Handles a mouse event. Returns a response with optional action.
    fn handle_mouse(&mut self, event: &MouseEvent, ctx: &EventCtx<'_>) -> WidgetResponse;

    /// Handles a synthetic hover event (enter/leave).
    fn handle_hover(&mut self, event: HoverEvent, ctx: &EventCtx<'_>) -> WidgetResponse;

    /// Handles a keyboard event. Returns a response with optional action.
    fn handle_key(&mut self, event: KeyEvent, ctx: &EventCtx<'_>) -> WidgetResponse;

    /// Propagates an externally-originated action to a descendant widget.
    ///
    /// Used when an action from a popup overlay (e.g. dropdown menu selection)
    /// needs to update a widget buried in the tree. Containers propagate to
    /// children; leaf widgets check if the action targets them.
    /// Returns `true` if a descendant handled the action.
    fn accept_action(&mut self, _action: &WidgetAction) -> bool {
        false
    }

    /// Collects focusable widget IDs reachable from this widget.
    ///
    /// Leaf widgets return their own ID if focusable; containers override
    /// to recurse into children. Used by the overlay manager to build
    /// modal focus order.
    fn focusable_children(&self) -> Vec<WidgetId> {
        if self.is_focusable() {
            vec![self.id()]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
pub(crate) mod tests;
