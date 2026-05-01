//! Semantic actions emitted by widgets for the application layer.
//!
//! `WidgetAction` is the sole communication channel between widgets and the
//! application layer. Widgets emit actions; the app matches on variants and
//! interprets them. No closures — keeps widgets stateless w.r.t. app logic.
//!
//! Lives in its own module (not `widgets/`) so that `controllers/` can import
//! it without creating a circular dependency (`controllers -> widgets`).
//!
//! Submodules:
//! - `keymap_action`: Typed actions bound to keystrokes via the keymap system.
//! - `keymap`: Keymap data structure mapping keystrokes to actions.
//! - `context`: Key context for scope-gated binding dispatch.

pub mod context;
pub mod keymap;
pub mod keymap_action;

pub use context::{build_context_stack, collect_key_contexts};
pub use keymap::{KeyBinding, Keymap, Keystroke};
pub use keymap_action::KeymapAction;

use crate::geometry::{Point, Rect};
use crate::widget_id::WidgetId;
use crate::widgets::sidebar_nav::FooterTarget;

/// A semantic action emitted by a widget for the application layer.
///
/// No closures — the app layer matches on variants and interprets them.
/// This keeps widgets stateless with respect to application logic.
#[derive(Debug, Clone, PartialEq)]
pub enum WidgetAction {
    /// A button or clickable widget was activated.
    Clicked(WidgetId),
    /// A double-click was detected on a clickable widget.
    DoubleClicked(WidgetId),
    /// A triple-click was detected on a clickable widget.
    TripleClicked(WidgetId),
    /// A boolean value was toggled (checkbox, toggle switch).
    Toggled { id: WidgetId, value: bool },
    /// A numeric value changed (slider).
    ValueChanged { id: WidgetId, value: f32 },
    /// Text content changed (text input).
    TextChanged { id: WidgetId, text: String },
    /// An item was selected by index (dropdown, menu).
    Selected { id: WidgetId, index: usize },
    /// A dropdown trigger requests opening its popup list.
    ///
    /// `searchable = true` opens the popup with `MenuWidget::with_searchable(true)` —
    /// the same `MenuWidget` extended with a type-to-filter input row at the
    /// top. `initial_highlight` overrides the popup's first highlighted entry
    /// (used when the trigger was opened via `ArrowDown` to land on the first
    /// item rather than the currently-selected one); `None` defers to
    /// `selected`. Plain dropdowns ignore the override.
    OpenDropdown {
        /// The dropdown widget's ID (for routing selection back).
        id: WidgetId,
        /// Option labels.
        options: Vec<String>,
        /// Currently selected index.
        selected: usize,
        /// Screen-space anchor rect for popup placement.
        anchor: Rect,
        /// When `true`, mount the popup in searchable mode (filterable list).
        searchable: bool,
        /// Override for the initial highlighted entry (canonical index).
        /// Forwarded only when `searchable` is `true`; `None` defers to
        /// `selected`.
        initial_highlight: Option<usize>,
    },
    /// An overlay content widget requests its own dismissal.
    DismissOverlay(WidgetId),
    /// An overlay widget requests repositioning (e.g. header drag).
    MoveOverlay { delta_x: f32, delta_y: f32 },
    /// A drag gesture started (threshold exceeded).
    DragStart { id: WidgetId, pos: Point },
    /// A drag gesture moved.
    DragUpdate {
        id: WidgetId,
        /// Movement since last `MouseMove`.
        delta: Point,
        /// Cumulative movement since `DragStart`.
        total_delta: Point,
    },
    /// A drag gesture ended (mouse released while dragging).
    DragEnd { id: WidgetId, pos: Point },
    /// A scroll event with converted pixel deltas.
    ScrollBy {
        id: WidgetId,
        delta_x: f32,
        delta_y: f32,
    },
    /// The settings panel Save button was clicked — persist and dismiss.
    SaveSettings,
    /// The settings panel Cancel button was clicked — revert and dismiss.
    CancelSettings,
    /// The settings panel Reset to Defaults button was clicked.
    ResetDefaults,
    /// Notify the settings panel that unsaved changes exist.
    SettingsUnsaved(bool),
    /// Notify the sidebar that a specific page has dirty (unsaved) state.
    PageDirty { page: usize, dirty: bool },
    /// A sidebar footer target was clicked.
    FooterAction(FooterTarget),
    /// Minimize the window.
    WindowMinimize,
    /// Maximize or restore the window.
    WindowMaximize,
    /// Close the window.
    WindowClose,
    /// A tab title was changed via inline editing.
    TabTitleChanged { index: usize, title: String },
}
