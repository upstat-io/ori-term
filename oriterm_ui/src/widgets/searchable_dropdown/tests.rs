use super::SearchableDropdownWidget;

use crate::action::keymap_action::Confirm;
use crate::action::keymap_action::NavigateDown;
use crate::geometry::Rect;
use crate::widgets::{Widget, WidgetAction};

fn trigger_with(items: Vec<&str>) -> SearchableDropdownWidget {
    SearchableDropdownWidget::new(items.into_iter().map(str::to_owned).collect())
}

#[test]
fn click_emits_open_searchable_dropdown_with_full_items() {
    let mut t = trigger_with(vec!["Alpha", "Beta", "Gamma"]);
    let bounds = Rect::new(10.0, 20.0, 200.0, 30.0);
    let action = t
        .on_action(WidgetAction::Clicked(t.id()), bounds)
        .expect("click must emit an action");
    match action {
        WidgetAction::OpenSearchableDropdown {
            id,
            items,
            selected,
            anchor,
            initial_highlight,
        } => {
            assert_eq!(id, t.id());
            assert_eq!(items, vec!["Alpha", "Beta", "Gamma"]);
            assert_eq!(selected, None);
            assert_eq!(anchor, bounds);
            assert_eq!(initial_highlight, None);
        }
        other => panic!("expected OpenSearchableDropdown, got {other:?}"),
    }
}

#[test]
fn confirm_keymap_emits_open_with_no_initial_highlight() {
    let mut t = trigger_with(vec!["Alpha"]).with_selected(0);
    let bounds = Rect::new(0.0, 0.0, 100.0, 30.0);
    let action = t
        .handle_keymap_action(&Confirm, bounds)
        .expect("Confirm must emit an action");
    match action {
        WidgetAction::OpenSearchableDropdown {
            initial_highlight,
            selected,
            ..
        } => {
            assert_eq!(initial_highlight, None);
            assert_eq!(selected, Some(0));
        }
        other => panic!("expected OpenSearchableDropdown, got {other:?}"),
    }
}

#[test]
fn navigate_down_keymap_emits_open_with_initial_highlight_zero() {
    let mut t = trigger_with(vec!["Alpha", "Beta"]);
    let action = t
        .handle_keymap_action(&NavigateDown, Rect::new(0.0, 0.0, 100.0, 30.0))
        .expect("NavigateDown must emit an action");
    match action {
        WidgetAction::OpenSearchableDropdown {
            initial_highlight, ..
        } => {
            assert_eq!(initial_highlight, Some(0));
        }
        other => panic!("expected OpenSearchableDropdown, got {other:?}"),
    }
}

#[test]
fn key_context_matches_dropdown_so_default_keymap_fires() {
    let t = trigger_with(vec!["A"]);
    assert_eq!(t.key_context(), Some("Dropdown"));
}

#[test]
fn trigger_with_selected_clamps_out_of_range_to_none() {
    let t = trigger_with(vec!["A", "B"]).with_selected(99);
    assert_eq!(t.selected(), None);
}

#[test]
fn trigger_with_selected_in_range_records_index() {
    let t = trigger_with(vec!["A", "B", "C"]).with_selected(2);
    assert_eq!(t.selected(), Some(2));
}
