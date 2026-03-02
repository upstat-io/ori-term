//! Context menu types and builder functions.
//!
//! Defines [`ContextAction`] (what happens when a menu item is selected) and
//! [`ContextMenuState`] (maps entry indices to actions). Builder functions
//! produce the `(Vec<MenuEntry>, ContextMenuState)` pairs that are pushed
//! as overlays.

use oriterm_ui::widgets::menu::MenuEntry;

/// What to do when a context menu item is selected.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum ContextAction {
    /// Open settings (no-op until Section 21.3).
    Settings,
    /// Switch to a named color scheme.
    SelectScheme(String),
}

/// Maps entry indices to context actions.
///
/// Separators have `None` — only clickable entries have actions.
#[derive(Debug)]
pub(crate) struct ContextMenuState {
    actions: Vec<Option<ContextAction>>,
}

impl ContextMenuState {
    /// Resolve an entry index to its action.
    pub(super) fn resolve(&self, index: usize) -> Option<&ContextAction> {
        self.actions.get(index).and_then(|a| a.as_ref())
    }
}

/// Build the dropdown menu for the tab bar dropdown button.
pub(super) fn build_dropdown_menu(
    active_scheme: &str,
    scheme_names: &[&str],
) -> (Vec<MenuEntry>, ContextMenuState) {
    let mut entries = Vec::new();
    let mut actions = Vec::new();

    entries.push(MenuEntry::Item {
        label: "Settings".into(),
    });
    actions.push(Some(ContextAction::Settings));

    entries.push(MenuEntry::Separator);
    actions.push(None);

    for &name in scheme_names {
        let checked = name.eq_ignore_ascii_case(active_scheme);
        entries.push(MenuEntry::Check {
            label: name.to_owned(),
            checked,
        });
        actions.push(Some(ContextAction::SelectScheme(name.to_owned())));
    }

    (entries, ContextMenuState { actions })
}

#[cfg(test)]
mod tests;
