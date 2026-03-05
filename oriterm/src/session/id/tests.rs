use super::{IdAllocator, SessionId, TabId, WindowId};

#[test]
fn tab_id_from_raw_roundtrip() {
    let id = TabId::from_raw(42);
    assert_eq!(id.raw(), 42);
}

#[test]
fn window_id_from_raw_roundtrip() {
    let id = WindowId::from_raw(7);
    assert_eq!(id.raw(), 7);
}

#[test]
fn tab_id_display() {
    let id = TabId::from_raw(3);
    assert_eq!(format!("{id}"), "Tab(3)");
}

#[test]
fn window_id_display() {
    let id = WindowId::from_raw(5);
    assert_eq!(format!("{id}"), "Window(5)");
}

#[test]
fn tab_id_equality() {
    let a = TabId::from_raw(1);
    let b = TabId::from_raw(1);
    let c = TabId::from_raw(2);
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn window_id_equality() {
    let a = WindowId::from_raw(1);
    let b = WindowId::from_raw(1);
    let c = WindowId::from_raw(2);
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn allocator_starts_at_one() {
    let mut alloc = IdAllocator::<TabId>::new();
    let first = alloc.alloc();
    assert_eq!(first.raw(), 1);
}

#[test]
fn allocator_monotonically_increasing() {
    let mut alloc = IdAllocator::<WindowId>::new();
    let a = alloc.alloc();
    let b = alloc.alloc();
    let c = alloc.alloc();
    assert_eq!(a.raw(), 1);
    assert_eq!(b.raw(), 2);
    assert_eq!(c.raw(), 3);
}

#[test]
fn allocator_default_matches_new() {
    let mut from_new = IdAllocator::<TabId>::new();
    let mut from_default = IdAllocator::<TabId>::default();
    assert_eq!(from_new.alloc().raw(), from_default.alloc().raw());
}

#[test]
fn session_id_trait_works_generically() {
    fn alloc_pair<T: SessionId>(alloc: &mut IdAllocator<T>) -> (T, T) {
        (alloc.alloc(), alloc.alloc())
    }

    let mut tab_alloc = IdAllocator::<TabId>::new();
    let (a, b) = alloc_pair(&mut tab_alloc);
    assert_eq!(a.raw(), 1);
    assert_eq!(b.raw(), 2);
}

#[test]
fn tab_id_hash_consistent() {
    use std::collections::HashSet;

    let id = TabId::from_raw(10);
    let mut set = HashSet::new();
    set.insert(id);
    assert!(set.contains(&TabId::from_raw(10)));
    assert!(!set.contains(&TabId::from_raw(11)));
}
