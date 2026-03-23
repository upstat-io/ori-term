//! Keybindings page builder — keyboard shortcut display.
//!
//! Builds a page with three sections showing the default keybindings:
//! Tabs & Panes, Clipboard, and Navigation. Each row is a `KeybindRow`
//! showing the action name and key badges.

use oriterm_ui::layout::SizeSpec;
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widgets::Widget;
use oriterm_ui::widgets::container::ContainerWidget;
use oriterm_ui::widgets::keybind::KeybindRow;

use super::appearance::{ROW_GAP, build_settings_page, section_title};

/// Builds the Keybindings page content widget (display-only).
pub(super) fn build_page(theme: &UiTheme) -> Box<dyn Widget> {
    build_settings_page(
        "KEYBINDINGS",
        "Keyboard shortcuts for common actions",
        vec![
            build_tabs_section(theme),
            build_clipboard_section(theme),
            build_nav_section(theme),
        ],
        theme,
    )
}

/// Tabs & Panes section.
fn build_tabs_section(theme: &UiTheme) -> Box<dyn Widget> {
    let title = section_title("Tabs & Panes", theme);
    let rows: Vec<Box<dyn Widget>> = vec![
        keybind("New tab", &["Ctrl", "Shift", "T"], theme),
        keybind("Close tab", &["Ctrl", "Shift", "W"], theme),
        keybind("Split vertically", &["Ctrl", "Shift", "D"], theme),
        keybind("Split horizontally", &["Ctrl", "Shift", "E"], theme),
        keybind("Next tab", &["Ctrl", "Tab"], theme),
        keybind("Previous tab", &["Ctrl", "Shift", "Tab"], theme),
    ];

    let mut col = ContainerWidget::column()
        .with_width(SizeSpec::Fill)
        .with_gap(ROW_GAP)
        .with_child(title);
    for row in rows {
        col = col.with_child(row);
    }
    Box::new(col)
}

/// Clipboard section.
fn build_clipboard_section(theme: &UiTheme) -> Box<dyn Widget> {
    let title = section_title("Clipboard", theme);
    let rows: Vec<Box<dyn Widget>> = vec![
        keybind("Copy", &["Ctrl", "Shift", "C"], theme),
        keybind("Paste", &["Ctrl", "Shift", "V"], theme),
    ];

    let mut col = ContainerWidget::column()
        .with_width(SizeSpec::Fill)
        .with_gap(ROW_GAP)
        .with_child(title);
    for row in rows {
        col = col.with_child(row);
    }
    Box::new(col)
}

/// Navigation section.
fn build_nav_section(theme: &UiTheme) -> Box<dyn Widget> {
    let title = section_title("Navigation", theme);
    let rows: Vec<Box<dyn Widget>> = vec![
        keybind("Scroll up", &["Ctrl", "Shift", "Up"], theme),
        keybind("Scroll down", &["Ctrl", "Shift", "Down"], theme),
        keybind("Search", &["Ctrl", "Shift", "F"], theme),
        keybind("Settings", &["Ctrl", ","], theme),
    ];

    let mut col = ContainerWidget::column()
        .with_width(SizeSpec::Fill)
        .with_gap(ROW_GAP)
        .with_child(title);
    for row in rows {
        col = col.with_child(row);
    }
    Box::new(col)
}

/// Creates a boxed `KeybindRow` from an action name and key parts.
fn keybind(action: &str, keys: &[&str], theme: &UiTheme) -> Box<dyn Widget> {
    let keys_owned: Vec<String> = keys.iter().map(|k| (*k).to_owned()).collect();
    Box::new(KeybindRow::new(action, keys_owned, theme))
}
