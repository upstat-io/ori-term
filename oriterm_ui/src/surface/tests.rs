//! Unit tests for surface abstractions.

use super::{DamageKind, DamageSet, SurfaceLifecycle};

// DamageSet

#[test]
fn damage_set_starts_empty() {
    let ds = DamageSet::default();
    assert!(ds.is_empty());
    assert!(!ds.contains(DamageKind::Layout));
    assert!(!ds.contains(DamageKind::Paint));
}

#[test]
fn damage_set_insert_and_contains() {
    let mut ds = DamageSet::default();
    ds.insert(DamageKind::Paint);
    assert!(!ds.is_empty());
    assert!(ds.contains(DamageKind::Paint));
    assert!(!ds.contains(DamageKind::Layout));
}

#[test]
fn damage_set_multiple_kinds() {
    let mut ds = DamageSet::default();
    ds.insert(DamageKind::Layout);
    ds.insert(DamageKind::Overlay);
    ds.insert(DamageKind::ScrollTransform);
    assert!(ds.contains(DamageKind::Layout));
    assert!(ds.contains(DamageKind::Overlay));
    assert!(ds.contains(DamageKind::ScrollTransform));
    assert!(!ds.contains(DamageKind::Paint));
    assert!(!ds.contains(DamageKind::Cursor));
}

#[test]
fn damage_set_insert_idempotent() {
    let mut ds = DamageSet::default();
    ds.insert(DamageKind::Cursor);
    ds.insert(DamageKind::Cursor);
    assert!(ds.contains(DamageKind::Cursor));
}

#[test]
fn damage_set_clear() {
    let mut ds = DamageSet::default();
    ds.insert(DamageKind::Layout);
    ds.insert(DamageKind::Paint);
    ds.clear();
    assert!(ds.is_empty());
    assert!(!ds.contains(DamageKind::Layout));
}

#[test]
fn damage_set_is_urgent() {
    let mut ds = DamageSet::default();
    assert!(!ds.is_urgent());

    ds.insert(DamageKind::Cursor);
    assert!(!ds.is_urgent(), "cursor-only is not urgent");

    ds.insert(DamageKind::Paint);
    assert!(ds.is_urgent(), "paint is urgent");

    ds.clear();
    ds.insert(DamageKind::Layout);
    assert!(ds.is_urgent(), "layout is urgent");
}

// SurfaceLifecycle

#[test]
fn lifecycle_valid_transitions() {
    let s = SurfaceLifecycle::CreatedHidden;
    assert!(s.can_transition_to(SurfaceLifecycle::Primed));
    assert!(s.can_transition_to(SurfaceLifecycle::Destroyed));
    assert!(!s.can_transition_to(SurfaceLifecycle::Visible));

    let s = SurfaceLifecycle::Primed;
    assert!(s.can_transition_to(SurfaceLifecycle::Visible));
    assert!(!s.can_transition_to(SurfaceLifecycle::Closing));

    let s = SurfaceLifecycle::Visible;
    assert!(s.can_transition_to(SurfaceLifecycle::Closing));
    assert!(!s.can_transition_to(SurfaceLifecycle::Destroyed));

    let s = SurfaceLifecycle::Closing;
    assert!(s.can_transition_to(SurfaceLifecycle::Destroyed));
    assert!(!s.can_transition_to(SurfaceLifecycle::Visible));
}

#[test]
fn lifecycle_transition_returns_new_state() {
    let s = SurfaceLifecycle::CreatedHidden;
    let s = s.transition(SurfaceLifecycle::Primed);
    assert_eq!(s, SurfaceLifecycle::Primed);
    let s = s.transition(SurfaceLifecycle::Visible);
    assert_eq!(s, SurfaceLifecycle::Visible);
}

#[test]
#[should_panic(expected = "invalid lifecycle transition")]
fn lifecycle_invalid_transition_panics() {
    let s = SurfaceLifecycle::CreatedHidden;
    let _ = s.transition(SurfaceLifecycle::Visible);
}

#[test]
fn lifecycle_bail_out_path() {
    let s = SurfaceLifecycle::CreatedHidden;
    let s = s.transition(SurfaceLifecycle::Destroyed);
    assert_eq!(s, SurfaceLifecycle::Destroyed);
}
