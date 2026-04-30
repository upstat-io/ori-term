//! Searchable-dropdown popup widget — the search-extension mounted in
//! response to [`crate::widgets::WidgetAction::OpenSearchableDropdown`].
//!
//! The trigger control is the regular [`crate::widgets::dropdown::DropdownWidget`]
//! constructed via `DropdownWidget::new(items).with_searchable(true)`. The
//! trigger emits the searchable-open action on click / Confirm / arrow keys;
//! the App layer mounts a [`SearchableDropdownPopupWidget`] in response —
//! that popup is the search-specific extension this module owns.
//!
//! State-ownership rule: the trigger does NOT intercept `Selected` to mutate
//! its own `selected` field directly. The popup emits
//! `WidgetAction::Selected { id: trigger_id, index: canonical_index }` and
//! the trigger's existing `accept_action` (in `DropdownWidget`) updates the
//! `selected` field in place. `form_builder` rebuilds the page on the next
//! config-change cycle for the form-level state.

mod popup;

pub use popup::{SearchableDropdownPopupWidget, SearchableDropdownStyle};
