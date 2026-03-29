pub use crate::testing::{MockMeasurer, TEST_THEME};

#[test]
fn widget_ids_are_unique() {
    use super::Widget;
    use super::button::ButtonWidget;
    use super::checkbox::CheckboxWidget;
    use super::label::LabelWidget;

    let a = ButtonWidget::new("A");
    let b = ButtonWidget::new("B");
    let c = LabelWidget::new("C");
    let d = CheckboxWidget::new("D");

    // All IDs must be distinct.
    let ids = [a.id(), b.id(), c.id(), d.id()];
    for i in 0..ids.len() {
        for j in (i + 1)..ids.len() {
            assert_ne!(ids[i], ids[j], "widget IDs must be unique");
        }
    }
}

// -- OnInputResult --

#[test]
fn on_input_result_handled() {
    let r = super::OnInputResult::handled();
    assert!(r.handled);
    assert!(r.action.is_none());
}

#[test]
fn on_input_result_ignored() {
    let r = super::OnInputResult::ignored();
    assert!(!r.handled);
    assert!(r.action.is_none());
}

#[test]
fn on_input_result_with_action() {
    use crate::widget_id::WidgetId;

    let id = WidgetId::next();
    let r = super::OnInputResult::handled().with_action(super::WidgetAction::Clicked(id));
    assert!(r.handled);
    assert_eq!(r.action, Some(super::WidgetAction::Clicked(id)));
}
