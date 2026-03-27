//! Terminal page builder — cursor, scrollback, and shell settings.
//!
//! Builds a page with three sections: Cursor (`CursorPicker` + blink toggle),
//! Scrollback (max lines input), and Shell (default shell path + paste warning).

use oriterm_ui::layout::SizeSpec;
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widgets::Widget;
use oriterm_ui::widgets::container::ContainerWidget;
use oriterm_ui::widgets::cursor_picker::CursorPickerWidget;
use oriterm_ui::widgets::dropdown::DropdownWidget;
use oriterm_ui::widgets::number_input::NumberInputWidget;
use oriterm_ui::widgets::setting_row::SettingRowWidget;
use oriterm_ui::widgets::text_input::{TextInputStyle, TextInputWidget};
use oriterm_ui::widgets::toggle::ToggleWidget;

use crate::config::{Config, CursorStyle, PasteWarning};

use super::SettingsIds;
use super::shared::{build_section_header, build_settings_page};

/// Builds the Terminal page content widget.
///
/// Writes cursor, scrollback, shell, and paste warning IDs into `SettingsIds`.
pub(super) fn build_page(
    config: &Config,
    ids: &mut SettingsIds,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    build_settings_page(
        "Terminal",
        "Cursor style, scrollback, and shell settings",
        vec![
            build_cursor_section(config, ids, theme),
            build_scrollback_section(config, ids, theme),
            build_shell_section(config, ids, theme),
        ],
        theme,
    )
}

/// Cursor section: style picker + blink toggle.
fn build_cursor_section(
    config: &Config,
    ids: &mut SettingsIds,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    let selected = match config.terminal.cursor_style {
        CursorStyle::Block => 0,
        CursorStyle::Bar => 1,
        CursorStyle::Underline => 2,
    };
    let picker = CursorPickerWidget::new(selected, theme);
    ids.cursor_picker = picker.id();

    let picker_row = SettingRowWidget::new(
        "Cursor style",
        "Block, bar, or underline cursor shape",
        Box::new(picker),
        theme,
    );

    let blink_toggle = ToggleWidget::new().with_on(config.terminal.cursor_blink);
    ids.cursor_blink_toggle = blink_toggle.id();

    let blink_row = SettingRowWidget::new(
        "Cursor blink",
        "Blinking cursor animation",
        Box::new(blink_toggle),
        theme,
    );

    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_child(build_section_header("Cursor", theme))
            .with_child(Box::new(picker_row))
            .with_child(Box::new(blink_row)),
    )
}

/// Scrollback section: max lines input.
fn build_scrollback_section(
    config: &Config,
    ids: &mut SettingsIds,
    theme: &UiTheme,
) -> Box<dyn Widget> {
    let input = NumberInputWidget::new(
        config.terminal.scrollback as f32,
        0.0,
        100_000.0,
        1000.0,
        theme,
    );
    ids.scrollback_input = input.id();

    let row = SettingRowWidget::new(
        "Maximum lines",
        "Scrollback buffer size (0 = unlimited)",
        Box::new(input),
        theme,
    );

    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_child(build_section_header("Scrollback", theme))
            .with_child(Box::new(row)),
    )
}

/// Shell section: default shell + paste warning.
fn build_shell_section(config: &Config, ids: &mut SettingsIds, theme: &UiTheme) -> Box<dyn Widget> {
    let mut shell_input = TextInputWidget::new()
        .with_placeholder("System default")
        .with_style(TextInputStyle::settings(theme));
    if let Some(ref shell) = config.terminal.shell {
        shell_input.set_text(shell.clone());
    }
    ids.shell_input = shell_input.id();

    let shell_row = SettingRowWidget::new(
        "Default shell",
        "Takes effect for new terminal tabs",
        Box::new(shell_input),
        theme,
    );

    let paste_items = vec!["Always".to_owned(), "Never".to_owned()];
    let paste_idx = match config.behavior.warn_on_paste {
        PasteWarning::Always | PasteWarning::Threshold(_) => 0,
        PasteWarning::Never => 1,
    };
    let paste_dropdown = DropdownWidget::new(paste_items).with_selected(paste_idx);
    ids.paste_warning_dropdown = paste_dropdown.id();

    let paste_row = SettingRowWidget::new(
        "Paste warning",
        "Confirm before pasting large text blocks",
        Box::new(paste_dropdown),
        theme,
    );

    Box::new(
        ContainerWidget::column()
            .with_width(SizeSpec::Fill)
            .with_child(build_section_header("Shell", theme))
            .with_child(Box::new(shell_row))
            .with_child(Box::new(paste_row)),
    )
}
