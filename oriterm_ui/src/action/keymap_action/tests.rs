use super::*;

#[test]
fn action_names_are_namespace_qualified() {
    assert_eq!(Activate.name(), "widget::Activate");
    assert_eq!(NavigateUp.name(), "widget::NavigateUp");
    assert_eq!(NavigateDown.name(), "widget::NavigateDown");
    assert_eq!(Confirm.name(), "widget::Confirm");
    assert_eq!(Dismiss.name(), "widget::Dismiss");
    assert_eq!(FocusNext.name(), "widget::FocusNext");
    assert_eq!(FocusPrev.name(), "widget::FocusPrev");
    assert_eq!(IncrementValue.name(), "widget::IncrementValue");
    assert_eq!(DecrementValue.name(), "widget::DecrementValue");
    assert_eq!(ValueToMin.name(), "widget::ValueToMin");
    assert_eq!(ValueToMax.name(), "widget::ValueToMax");
}

#[test]
fn boxed_clone_preserves_identity() {
    let original = Activate;
    let cloned = original.boxed_clone();
    assert_eq!(cloned.name(), "widget::Activate");
}

#[test]
fn as_any_downcast_works() {
    let action: Box<dyn KeymapAction> = Box::new(Activate);
    assert!(action.as_any().downcast_ref::<Activate>().is_some());
    assert!(action.as_any().downcast_ref::<Dismiss>().is_none());
}

#[test]
fn custom_namespace_actions() {
    actions!(settings, [ResetDefaults, Save]);
    assert_eq!(ResetDefaults.name(), "settings::ResetDefaults");
    assert_eq!(Save.name(), "settings::Save");
}
