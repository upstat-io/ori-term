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
    /// Close the tab at the given index.
    CloseTab(usize),
    /// Duplicate the tab at the given index (new tab inheriting CWD).
    DuplicateTab(usize),
    /// Move the tab at the given index to a new window.
    MoveToNewWindow(usize),
    /// Copy the current selection to clipboard.
    Copy,
    /// Paste from clipboard.
    Paste,
    /// Select all text in the terminal.
    SelectAll,
    /// Create a new tab.
    NewTab,
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

/// Build the tab right-click context menu.
///
/// Entries: Close Tab, Duplicate Tab, Move to New Window.
pub(super) fn build_tab_context_menu(tab_index: usize) -> (Vec<MenuEntry>, ContextMenuState) {
    let entries = vec![
        MenuEntry::Item {
            label: "Close Tab".into(),
        },
        MenuEntry::Item {
            label: "Duplicate Tab".into(),
        },
        MenuEntry::Separator,
        MenuEntry::Item {
            label: "Move to New Window".into(),
        },
    ];
    let actions = vec![
        Some(ContextAction::CloseTab(tab_index)),
        Some(ContextAction::DuplicateTab(tab_index)),
        None, // separator
        Some(ContextAction::MoveToNewWindow(tab_index)),
    ];
    (entries, ContextMenuState { actions })
}

/// Build the grid right-click context menu.
///
/// Copy is only shown when a selection exists. Entries: Copy, Paste,
/// Select All, separator, New Tab, Close Tab, separator, Settings.
pub(super) fn build_grid_context_menu(has_selection: bool) -> (Vec<MenuEntry>, ContextMenuState) {
    let mut entries = Vec::new();
    let mut actions = Vec::new();

    if has_selection {
        entries.push(MenuEntry::Item {
            label: "Copy".into(),
        });
        actions.push(Some(ContextAction::Copy));
    }

    entries.push(MenuEntry::Item {
        label: "Paste".into(),
    });
    actions.push(Some(ContextAction::Paste));

    entries.push(MenuEntry::Item {
        label: "Select All".into(),
    });
    actions.push(Some(ContextAction::SelectAll));

    entries.push(MenuEntry::Separator);
    actions.push(None);

    entries.push(MenuEntry::Item {
        label: "New Tab".into(),
    });
    actions.push(Some(ContextAction::NewTab));

    entries.push(MenuEntry::Item {
        label: "Close Tab".into(),
    });
    actions.push(Some(ContextAction::CloseTab(0))); // index 0 is a placeholder — close_active_tab used

    entries.push(MenuEntry::Separator);
    actions.push(None);

    entries.push(MenuEntry::Item {
        label: "Settings".into(),
    });
    actions.push(Some(ContextAction::Settings));

    (entries, ContextMenuState { actions })
}

#[cfg(test)]
mod tests;
