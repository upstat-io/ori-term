use super::{SearchableDropdownPopupWidget, SearchableDropdownStyle};

use crate::geometry::{Point, Rect};
use crate::input::{InputEvent, Key, Modifiers, MouseButton};
use crate::widget_id::WidgetId;
use crate::widgets::{Widget, WidgetAction};

fn popup_with(items: Vec<&str>) -> SearchableDropdownPopupWidget {
    SearchableDropdownPopupWidget::new(
        WidgetId::next(),
        items.into_iter().map(str::to_owned).collect(),
        None,
        None,
        SearchableDropdownStyle::default(),
    )
}

fn key(k: Key) -> InputEvent {
    InputEvent::KeyDown {
        key: k,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn empty_query_keeps_all_items_visible() {
    let popup = popup_with(vec!["Alpha", "Beta", "Gamma"]);
    assert_eq!(popup.filtered_indices(), &[0, 1, 2]);
    assert_eq!(popup.highlighted(), Some(0));
}

#[test]
fn character_input_filters_to_matching_items() {
    let mut popup = popup_with(vec!["Alpha", "Beta", "Gamma"]);
    let r = popup.on_input(&key(Key::Character('B')), Rect::new(0.0, 0.0, 200.0, 200.0));
    assert!(r.handled);
    assert_eq!(popup.query(), "B");
    assert_eq!(popup.filtered_indices(), &[1]);
    assert_eq!(popup.highlighted(), Some(0));
}

#[test]
fn case_insensitive_filter_matches_letter_in_other_case() {
    let mut popup = popup_with(vec!["Alpha", "Beta", "Cherry"]);
    let _ = popup.on_input(&key(Key::Character('a')), Rect::new(0.0, 0.0, 200.0, 200.0));
    // Lowercased 'a' matches Alpha ('A') and Beta ('a' at end). "Cherry" has
    // no 'a' — pinning the gap closes the lint that asserts case-insensitive
    // match without also asserting the rejection of unrelated rows.
    assert_eq!(popup.filtered_indices(), &[0, 1]);
}

#[test]
fn substring_filter_matches_internal_chars() {
    let mut popup = popup_with(vec!["Alpha", "Beta", "Gamma"]);
    let _ = popup.on_input(&key(Key::Character('e')), Rect::new(0.0, 0.0, 200.0, 200.0));
    let _ = popup.on_input(&key(Key::Character('t')), Rect::new(0.0, 0.0, 200.0, 200.0));
    assert_eq!(popup.filtered_indices(), &[1]); // only Beta has "et"
}

#[test]
fn backspace_pops_query_and_re_filters() {
    let mut popup = popup_with(vec!["Alpha", "Beta", "Gamma"]);
    let _ = popup.on_input(&key(Key::Character('B')), Rect::new(0.0, 0.0, 200.0, 200.0));
    assert_eq!(popup.filtered_indices(), &[1]);
    let _ = popup.on_input(&key(Key::Backspace), Rect::new(0.0, 0.0, 200.0, 200.0));
    assert_eq!(popup.query(), "");
    assert_eq!(popup.filtered_indices(), &[0, 1, 2]);
}

#[test]
fn no_match_query_yields_empty_filtered_list() {
    let mut popup = popup_with(vec!["Alpha", "Beta", "Gamma"]);
    let _ = popup.on_input(&key(Key::Character('z')), Rect::new(0.0, 0.0, 200.0, 200.0));
    assert!(popup.filtered_indices().is_empty());
    assert_eq!(popup.highlighted(), None);
}

#[test]
fn arrow_down_advances_highlight_within_filter() {
    let mut popup = popup_with(vec!["Alpha", "Beta", "Gamma"]);
    let _ = popup.on_input(&key(Key::ArrowDown), Rect::new(0.0, 0.0, 200.0, 200.0));
    assert_eq!(popup.highlighted(), Some(1));
    let _ = popup.on_input(&key(Key::ArrowDown), Rect::new(0.0, 0.0, 200.0, 200.0));
    assert_eq!(popup.highlighted(), Some(2));
    // Wrap from bottom to top.
    let _ = popup.on_input(&key(Key::ArrowDown), Rect::new(0.0, 0.0, 200.0, 200.0));
    assert_eq!(popup.highlighted(), Some(0));
}

#[test]
fn arrow_up_at_top_wraps_to_last_filtered_item() {
    let mut popup = popup_with(vec!["Alpha", "Beta", "Gamma"]);
    assert_eq!(popup.highlighted(), Some(0));
    let _ = popup.on_input(&key(Key::ArrowUp), Rect::new(0.0, 0.0, 200.0, 200.0));
    assert_eq!(popup.highlighted(), Some(2));
}

#[test]
fn enter_emits_selected_with_canonical_index_not_filtered() {
    let trigger_id = WidgetId::next();
    let mut popup = SearchableDropdownPopupWidget::new(
        trigger_id,
        vec![
            "Alpha".to_owned(),
            "Beta".to_owned(),
            "Charlie".to_owned(),
            "Delta".to_owned(),
        ],
        None,
        None,
        SearchableDropdownStyle::default(),
    );
    // Filter to "C" — only "Charlie" matches.
    let _ = popup.on_input(&key(Key::Character('C')), Rect::new(0.0, 0.0, 200.0, 200.0));
    assert_eq!(popup.filtered_indices(), &[2]);
    assert_eq!(popup.highlighted(), Some(0));
    // Enter must emit canonical index 2, not filtered index 0.
    let result = popup.on_input(&key(Key::Enter), Rect::new(0.0, 0.0, 200.0, 200.0));
    assert!(result.handled);
    let action = result.action.expect("Enter must emit an action");
    match action {
        WidgetAction::Selected { id, index } => {
            assert_eq!(id, trigger_id, "id must route to trigger");
            assert_eq!(
                index, 2,
                "must be canonical 'Charlie' index, not filtered 0"
            );
        }
        other => panic!("expected Selected, got {other:?}"),
    }
}

#[test]
fn enter_with_no_filtered_match_emits_no_action() {
    let mut popup = popup_with(vec!["Alpha", "Beta", "Gamma"]);
    let _ = popup.on_input(&key(Key::Character('z')), Rect::new(0.0, 0.0, 200.0, 200.0));
    let r = popup.on_input(&key(Key::Enter), Rect::new(0.0, 0.0, 200.0, 200.0));
    assert!(r.handled);
    assert!(r.action.is_none(), "no match → no Selected emit");
}

#[test]
fn space_input_appends_to_query_not_select() {
    let mut popup = popup_with(vec!["Two Words", "Single"]);
    let r = popup.on_input(&key(Key::Space), Rect::new(0.0, 0.0, 200.0, 200.0));
    assert!(r.handled);
    assert!(r.action.is_none(), "Space must not emit Selected");
    assert_eq!(popup.query(), " ");
    assert_eq!(popup.filtered_indices(), &[0]);
}

#[test]
fn escape_passes_through_to_overlay_manager() {
    let mut popup = popup_with(vec!["Alpha"]);
    let r = popup.on_input(&key(Key::Escape), Rect::new(0.0, 0.0, 200.0, 200.0));
    // Popup itself does not emit a close action — overlay manager handles dismissal.
    assert!(!r.handled, "Escape must propagate up to OverlayManager");
    assert!(r.action.is_none());
}

#[test]
fn click_on_filtered_row_emits_selected_with_canonical_index() {
    let trigger_id = WidgetId::next();
    let mut popup = SearchableDropdownPopupWidget::new(
        trigger_id,
        vec!["Alpha".to_owned(), "Beta".to_owned(), "Charlie".to_owned()],
        None,
        None,
        SearchableDropdownStyle::default(),
    );
    // Filter to "B" — only "Beta" matches at canonical index 1.
    let _ = popup.on_input(&key(Key::Character('B')), Rect::new(0.0, 0.0, 200.0, 200.0));
    assert_eq!(popup.filtered_indices(), &[1]);
    // Click on the first filtered row.
    let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
    let click_y = 6.0 + popup.style_for_test().padding.top + 24.0 + 2.0 + 4.0;
    let click_event = InputEvent::MouseDown {
        pos: Point::new(20.0, click_y),
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    let result = popup.on_input(&click_event, bounds);
    assert!(result.handled);
    if let Some(WidgetAction::Selected { id, index }) = result.action {
        assert_eq!(id, trigger_id);
        assert_eq!(
            index, 1,
            "click must emit canonical Beta index, not filtered 0"
        );
    } else {
        panic!("expected Selected from click, got {:?}", result.action);
    }
}

#[test]
fn rebuild_filter_directly_for_unit_isolation() {
    let mut popup = popup_with(vec!["Alpha", "Beta", "Gamma"]);
    popup.set_query_for_test("eta");
    popup.rebuild_filter();
    assert_eq!(popup.filtered_indices(), &[1]);
}

// Test-only helpers — popup's internals are not pub but tests living in the
// same module ARE the canonical access path. These helpers exist solely so
// click-row-y arithmetic stays close to the production layout numbers without
// hardcoding magic constants.
impl SearchableDropdownPopupWidget {
    fn style_for_test(&self) -> &SearchableDropdownStyle {
        &self.style
    }

    fn set_query_for_test(&mut self, q: &str) {
        self.query.clear();
        self.query.push_str(q);
    }
}
