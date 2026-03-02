use oriterm_ui::widgets::menu::MenuEntry;

use super::{ContextAction, build_dropdown_menu};

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
