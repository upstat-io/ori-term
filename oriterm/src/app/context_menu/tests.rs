use oriterm_ui::widgets::menu::MenuEntry;

use super::{ContextAction, build_dropdown_menu, build_grid_context_menu, build_tab_context_menu};

#[test]
fn dropdown_menu_with_schemes() {
    let (entries, state) = build_dropdown_menu("Dracula", &["Catppuccin", "Dracula", "Gruvbox"]);

    // Settings, separator, then 3 scheme check items.
    assert_eq!(entries.len(), 5);
    assert!(matches!(&entries[0], MenuEntry::Item { label } if label == "Settings"));
    assert!(matches!(&entries[1], MenuEntry::Separator));
    assert!(
        matches!(&entries[2], MenuEntry::Check { label, checked } if label == "Catppuccin" && !checked)
    );
    assert!(
        matches!(&entries[3], MenuEntry::Check { label, checked } if label == "Dracula" && *checked)
    );
    assert!(
        matches!(&entries[4], MenuEntry::Check { label, checked } if label == "Gruvbox" && !checked)
    );

    assert_eq!(state.resolve(0), Some(&ContextAction::Settings));
    assert_eq!(state.resolve(1), None); // separator
    assert_eq!(
        state.resolve(3),
        Some(&ContextAction::SelectScheme("Dracula".into()))
    );
}

#[test]
fn resolve_out_of_bounds() {
    let (_, state) = build_dropdown_menu("", &["A"]);
    assert_eq!(state.resolve(99), None);
}

#[test]
fn dropdown_empty_schemes() {
    let (entries, _) = build_dropdown_menu("", &[]);
    // Settings + separator.
    assert_eq!(entries.len(), 2);
}

// -- Tab context menu --

#[test]
fn tab_context_menu_entries() {
    let (entries, _state) = build_tab_context_menu(2);

    // Close, Duplicate, separator, Move to New Window.
    assert_eq!(entries.len(), 4);
    assert!(matches!(&entries[0], MenuEntry::Item { label } if label == "Close Tab"));
    assert!(matches!(&entries[1], MenuEntry::Item { label } if label == "Duplicate Tab"));
    assert!(matches!(&entries[2], MenuEntry::Separator));
    assert!(matches!(&entries[3], MenuEntry::Item { label } if label == "Move to New Window"));
}

#[test]
fn tab_context_menu_actions() {
    let (_, state) = build_tab_context_menu(5);

    assert_eq!(state.resolve(0), Some(&ContextAction::CloseTab(5)));
    assert_eq!(state.resolve(1), Some(&ContextAction::DuplicateTab(5)));
    assert_eq!(state.resolve(2), None); // separator
    assert_eq!(state.resolve(3), Some(&ContextAction::MoveToNewWindow(5)));
}

// -- Grid context menu --

#[test]
fn grid_context_menu_with_selection() {
    let (entries, state) = build_grid_context_menu(true);

    // Copy, Paste, Select All, separator, New Tab, Close Tab, separator, Settings.
    assert_eq!(entries.len(), 8);
    assert!(matches!(&entries[0], MenuEntry::Item { label } if label == "Copy"));
    assert!(matches!(&entries[1], MenuEntry::Item { label } if label == "Paste"));
    assert!(matches!(&entries[2], MenuEntry::Item { label } if label == "Select All"));
    assert!(matches!(&entries[3], MenuEntry::Separator));
    assert!(matches!(&entries[4], MenuEntry::Item { label } if label == "New Tab"));
    assert!(matches!(&entries[5], MenuEntry::Item { label } if label == "Close Tab"));
    assert!(matches!(&entries[6], MenuEntry::Separator));
    assert!(matches!(&entries[7], MenuEntry::Item { label } if label == "Settings"));

    assert_eq!(state.resolve(0), Some(&ContextAction::Copy));
    assert_eq!(state.resolve(7), Some(&ContextAction::Settings));
}

#[test]
fn grid_context_menu_without_selection() {
    let (entries, state) = build_grid_context_menu(false);

    // No Copy entry. Paste, Select All, separator, New Tab, Close Tab, separator, Settings.
    assert_eq!(entries.len(), 7);
    assert!(matches!(&entries[0], MenuEntry::Item { label } if label == "Paste"));
    assert!(matches!(&entries[1], MenuEntry::Item { label } if label == "Select All"));

    assert_eq!(state.resolve(0), Some(&ContextAction::Paste));
    assert_eq!(state.resolve(1), Some(&ContextAction::SelectAll));
}

#[test]
fn grid_context_menu_action_coverage() {
    let (_, state) = build_grid_context_menu(true);

    // Verify all action variants resolve correctly.
    assert_eq!(state.resolve(0), Some(&ContextAction::Copy));
    assert_eq!(state.resolve(1), Some(&ContextAction::Paste));
    assert_eq!(state.resolve(2), Some(&ContextAction::SelectAll));
    assert_eq!(state.resolve(3), None); // separator
    assert_eq!(state.resolve(4), Some(&ContextAction::NewTab));
    assert_eq!(state.resolve(5), Some(&ContextAction::CloseTab(0)));
    assert_eq!(state.resolve(6), None); // separator
    assert_eq!(state.resolve(7), Some(&ContextAction::Settings));
    assert_eq!(state.resolve(8), None); // out of bounds
}
