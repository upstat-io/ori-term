//! Shared `MenuWidget` construction for dropdown popup overlays.
//!
//! Both [`crate::app::App::open_dialog_dropdown`] (settings dialogs) and
//! [`crate::app::App::open_dropdown_popup`] (primary windows) mount a
//! `MenuWidget` sized to the trigger anchor with identical styling +
//! initial-state semantics. This module is the canonical home for that
//! construction so the two call sites share one implementation
//! (algorithmic-duplication).

use oriterm_ui::geometry::Rect;
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widgets::menu::{MenuEntry, MenuStyle, MenuWidget};

/// Construction parameters for a dropdown popup `MenuWidget`. Bundles the
/// fields forwarded from `WidgetAction::OpenDropdown` plus the theme so
/// the build helper signature stays under the parameter-hygiene limit.
pub(crate) struct DropdownPopupConfig<'a> {
    pub(crate) options: Vec<String>,
    pub(crate) selected: usize,
    pub(crate) anchor: Rect,
    pub(crate) searchable: bool,
    pub(crate) initial_highlight: Option<usize>,
    pub(crate) theme: &'a UiTheme,
}

/// Builds a `MenuWidget` configured as a dropdown popup: theme-derived
/// chrome, anchor-matched width, scrollable, optional searchable mode,
/// optional initial-highlight override. The returned widget is pre-scrolled
/// so the highlighted entry (`hovered`, falling back to `selected`) is
/// visible — preventing `with_initial_highlight` from being scrolled
/// off-screen when the trigger's selected index is deep in a long list.
pub(crate) fn build_dropdown_menu_widget(config: DropdownPopupConfig<'_>) -> MenuWidget {
    let DropdownPopupConfig {
        options,
        selected,
        anchor,
        searchable,
        initial_highlight,
        theme,
    } = config;

    let entries: Vec<MenuEntry> = options
        .into_iter()
        .map(|label| MenuEntry::Item { label })
        .collect();

    let mut style = MenuStyle::from_theme(theme);
    style.min_width = anchor.width();
    style.extra_width = 24.0;
    style.shadow_color = theme.shadow;
    style.max_height = Some(300.0);
    style.selected_bg = theme.accent.with_alpha(0.12);

    let mut widget = MenuWidget::new(entries).with_style(style);
    if selected < widget.entries().len() {
        widget = widget.with_selected_index(selected);
    }
    if searchable {
        widget = widget.with_searchable(true);
        if let Some(idx) = initial_highlight {
            widget = widget.with_initial_highlight(idx);
        }
    }
    let scroll_target = widget.hovered().unwrap_or(selected);
    if scroll_target < widget.entries().len() {
        widget.ensure_visible(scroll_target);
    }
    widget
}
