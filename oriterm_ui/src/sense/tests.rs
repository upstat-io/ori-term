use super::Sense;

#[test]
fn none_is_empty() {
    let s = Sense::none();
    assert!(s.is_none());
    assert!(!s.has_hover());
    assert!(!s.has_click());
    assert!(!s.has_drag());
    assert!(!s.has_focus());
}

#[test]
fn default_is_none() {
    assert_eq!(Sense::default(), Sense::none());
    assert!(Sense::default().is_none());
}

#[test]
fn click_implies_hover() {
    let s = Sense::click();
    assert!(s.has_hover());
    assert!(s.has_click());
    assert!(!s.has_drag());
    assert!(!s.has_focus());
}

#[test]
fn drag_implies_hover() {
    let s = Sense::drag();
    assert!(s.has_hover());
    assert!(!s.has_click());
    assert!(s.has_drag());
    assert!(!s.has_focus());
}

#[test]
fn click_and_drag_implies_hover() {
    let s = Sense::click_and_drag();
    assert!(s.has_hover());
    assert!(s.has_click());
    assert!(s.has_drag());
    assert!(!s.has_focus());
}

#[test]
fn hover_only() {
    let s = Sense::hover();
    assert!(s.has_hover());
    assert!(!s.has_click());
    assert!(!s.has_drag());
    assert!(!s.has_focus());
}

#[test]
fn focusable_only() {
    let s = Sense::focusable();
    assert!(!s.has_hover());
    assert!(!s.has_click());
    assert!(!s.has_drag());
    assert!(s.has_focus());
    assert!(!s.is_none());
}

#[test]
fn all_has_everything() {
    let s = Sense::all();
    assert!(s.has_hover());
    assert!(s.has_click());
    assert!(s.has_drag());
    assert!(s.has_focus());
    assert!(!s.is_none());
}

#[test]
fn union_combines_flags() {
    let s = Sense::click().union(Sense::focusable());
    assert!(s.has_hover());
    assert!(s.has_click());
    assert!(s.has_focus());
    assert!(!s.has_drag());
}

#[test]
fn union_is_idempotent() {
    let a = Sense::click();
    assert_eq!(a.union(a), a);
}

#[test]
fn union_none_is_identity() {
    let s = Sense::drag();
    assert_eq!(s.union(Sense::none()), s);
    assert_eq!(Sense::none().union(s), s);
}

#[test]
fn equality() {
    assert_eq!(Sense::click(), Sense::click());
    assert_ne!(Sense::click(), Sense::drag());
    assert_ne!(Sense::none(), Sense::all());
}

#[test]
fn debug_none() {
    assert_eq!(format!("{:?}", Sense::none()), "Sense(none)");
}

#[test]
fn debug_click() {
    assert_eq!(format!("{:?}", Sense::click()), "Sense(HOVER | CLICK)");
}

#[test]
fn debug_all() {
    assert_eq!(
        format!("{:?}", Sense::all()),
        "Sense(HOVER | CLICK | DRAG | FOCUS)"
    );
}

#[test]
fn debug_focusable_only() {
    assert_eq!(format!("{:?}", Sense::focusable()), "Sense(FOCUS)");
}
