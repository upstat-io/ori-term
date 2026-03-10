//! Unit tests for the [`WindowManager`].

#![allow(
    unsafe_code,
    reason = "transmute for constructing distinct winit WindowIds in tests"
)]

use super::WindowManager;
use super::types::{DialogKind, ManagedWindow, WindowKind, WindowRequest};

// Compile-time guarantee that WindowId is the same size as u64 on this platform.
const _: () = assert!(
    size_of::<winit::window::WindowId>() == size_of::<u64>(),
    "WindowId size mismatch — tests require u64-sized WindowId"
);

/// Create a distinct `WindowId` from an integer for testing.
///
/// `WindowId::dummy()` always returns the same value, so we transmute from
/// `u64` to create distinct IDs. This is sound on Linux where `WindowId` is
/// a newtype chain over `u64`, guarded by the compile-time size check above.
fn wid(n: u64) -> winit::window::WindowId {
    // SAFETY: Guarded by the const assertion above. On platforms where
    // WindowId has a different layout, the assertion fails at compile time.
    unsafe { std::mem::transmute::<u64, winit::window::WindowId>(n) }
}

fn main_window(id: u64) -> ManagedWindow {
    ManagedWindow::new(wid(id), WindowKind::Main)
}

fn dialog_window(id: u64, parent: u64) -> ManagedWindow {
    ManagedWindow::with_parent(
        wid(id),
        WindowKind::Dialog(DialogKind::Settings),
        wid(parent),
    )
}

fn tear_off_window(id: u64) -> ManagedWindow {
    ManagedWindow::new(wid(id), WindowKind::TearOff)
}

// --- WindowKind predicates ---

#[test]
fn window_kind_predicates() {
    assert!(WindowKind::Main.is_main());
    assert!(!WindowKind::Main.is_dialog());
    assert!(!WindowKind::Main.is_tear_off());
    assert!(WindowKind::Main.is_primary());

    let dialog = WindowKind::Dialog(DialogKind::Settings);
    assert!(!dialog.is_main());
    assert!(dialog.is_dialog());
    assert!(!dialog.is_tear_off());
    assert!(!dialog.is_primary());

    assert!(!WindowKind::TearOff.is_main());
    assert!(!WindowKind::TearOff.is_dialog());
    assert!(WindowKind::TearOff.is_tear_off());
    assert!(WindowKind::TearOff.is_primary());
}

// --- Registry CRUD ---

#[test]
fn register_and_lookup() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));

    assert!(wm.contains(wid(1)));
    assert!(!wm.contains(wid(99)));
    assert_eq!(wm.len(), 1);
    assert_eq!(wm.get(wid(1)).unwrap().kind, WindowKind::Main);
}

#[test]
fn empty_manager() {
    let wm = WindowManager::new();
    assert!(wm.is_empty());
    assert_eq!(wm.len(), 0);
    assert_eq!(wm.primary_window_count(), 0);
    assert!(wm.get(wid(1)).is_none());
}

#[test]
fn windows_of_kind_filters_correctly() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    wm.register(main_window(2));
    wm.register(tear_off_window(3));
    wm.register(dialog_window(4, 1));

    let mains: Vec<_> = wm.main_windows().collect();
    assert_eq!(mains.len(), 2);

    let dialogs: Vec<_> = wm.windows_of_kind(WindowKind::is_dialog).collect();
    assert_eq!(dialogs.len(), 1);

    assert_eq!(wm.primary_window_count(), 3);
}

// --- Hierarchy ---

#[test]
fn register_with_parent_links_child() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    wm.register(dialog_window(10, 1));

    let parent = wm.get(wid(1)).unwrap();
    assert_eq!(parent.children, vec![wid(10)]);

    let child = wm.get(wid(10)).unwrap();
    assert_eq!(child.parent, Some(wid(1)));
}

#[test]
fn unregister_cascading_close() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    wm.register(dialog_window(10, 1));
    wm.register(dialog_window(11, 1));

    let removed = wm.unregister(wid(1));

    // Children first, then parent.
    assert_eq!(removed.len(), 3);
    assert_eq!(removed[0].winit_id, wid(10));
    assert_eq!(removed[1].winit_id, wid(11));
    assert_eq!(removed[2].winit_id, wid(1));
    assert!(wm.is_empty());
}

#[test]
fn unregister_deep_hierarchy() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    wm.register(dialog_window(10, 1));

    // Grandchild: a dialog owned by another dialog.
    let mut grandchild =
        ManagedWindow::with_parent(wid(20), WindowKind::Dialog(DialogKind::About), wid(10));
    grandchild.visible = true;
    wm.register(grandchild);

    let removed = wm.unregister(wid(1));
    assert_eq!(removed.len(), 3);
    // Depth-first: grandchild (20), child (10), root (1).
    assert_eq!(removed[0].winit_id, wid(20));
    assert_eq!(removed[1].winit_id, wid(10));
    assert_eq!(removed[2].winit_id, wid(1));
}

#[test]
fn unregister_clears_focus() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    wm.register(main_window(2));
    wm.set_focused(Some(wid(1)));

    wm.unregister(wid(1));
    assert_eq!(wm.focused_id(), None);
}

#[test]
fn unregister_preserves_focus_on_other_window() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    wm.register(main_window(2));
    wm.set_focused(Some(wid(2)));

    wm.unregister(wid(1));
    assert_eq!(wm.focused_id(), Some(wid(2)));
}

#[test]
fn unregister_child_removes_from_parent_children_list() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    wm.register(dialog_window(10, 1));
    wm.register(dialog_window(11, 1));

    wm.unregister(wid(10));

    let parent = wm.get(wid(1)).unwrap();
    assert_eq!(parent.children, vec![wid(11)]);
}

#[test]
fn reparent_moves_between_parents() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    wm.register(main_window(2));
    wm.register(dialog_window(10, 1));

    wm.reparent(wid(10), Some(wid(2)));

    let old_parent = wm.get(wid(1)).unwrap();
    assert!(old_parent.children.is_empty());

    let new_parent = wm.get(wid(2)).unwrap();
    assert_eq!(new_parent.children, vec![wid(10)]);

    let child = wm.get(wid(10)).unwrap();
    assert_eq!(child.parent, Some(wid(2)));
}

#[test]
fn reparent_to_none_makes_root() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    wm.register(dialog_window(10, 1));

    wm.reparent(wid(10), None);

    let old_parent = wm.get(wid(1)).unwrap();
    assert!(old_parent.children.is_empty());

    let child = wm.get(wid(10)).unwrap();
    assert_eq!(child.parent, None);
}

#[test]
fn children_of_iterates_children() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    wm.register(dialog_window(10, 1));
    wm.register(dialog_window(11, 1));

    let children: Vec<_> = wm.children_of(wid(1)).map(|w| w.winit_id).collect();
    assert_eq!(children, vec![wid(10), wid(11)]);
}

#[test]
fn children_of_empty() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));

    let children: Vec<_> = wm.children_of(wid(1)).collect();
    assert!(children.is_empty());
}

// --- Lifecycle ---

#[test]
fn should_exit_when_last_main_closes() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));

    assert!(wm.should_exit_on_close(wid(1)));
}

#[test]
fn should_not_exit_when_tear_off_remains() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    wm.register(tear_off_window(2));

    assert!(!wm.should_exit_on_close(wid(1)));
}

#[test]
fn should_not_exit_when_main_remains() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    wm.register(main_window(2));

    assert!(!wm.should_exit_on_close(wid(1)));
}

#[test]
fn dialogs_alone_dont_keep_app_alive() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    wm.register(dialog_window(10, 1));

    // Even though dialog 10 would remain, the app should exit.
    assert!(wm.should_exit_on_close(wid(1)));
}

#[test]
fn find_dialog_parent_prefers_focused() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    wm.register(main_window(2));
    wm.set_focused(Some(wid(2)));

    assert_eq!(wm.find_dialog_parent(), Some(wid(2)));
}

#[test]
fn find_dialog_parent_falls_back_to_any_main() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    wm.register(main_window(2));
    // No focused window.

    let parent = wm.find_dialog_parent();
    assert!(parent.is_some());
}

#[test]
fn find_dialog_parent_none_when_no_primary() {
    let wm = WindowManager::new();
    assert_eq!(wm.find_dialog_parent(), None);
}

#[test]
fn find_dialog_parent_skips_focused_dialog() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    wm.register(dialog_window(10, 1));
    wm.set_focused(Some(wid(10)));

    // Should skip the focused dialog and find the main window.
    assert_eq!(wm.find_dialog_parent(), Some(wid(1)));
}

#[test]
fn prepare_create_dialog_not_resizable() {
    let request = WindowRequest {
        kind: WindowKind::Dialog(DialogKind::Settings),
        parent: Some(wid(1)),
        title: "Settings".to_string(),
        size: Some((600, 400)),
        position: None,
        visible: true,
        decorations: true,
    };

    let attrs = WindowManager::prepare_create(&request);
    assert!(!attrs.resizable);
}

#[test]
fn prepare_create_main_is_resizable() {
    let request = WindowRequest {
        kind: WindowKind::Main,
        parent: None,
        title: "Terminal".to_string(),
        size: None,
        position: None,
        visible: true,
        decorations: false,
    };

    let attrs = WindowManager::prepare_create(&request);
    assert!(attrs.resizable);
}

// --- Focus ---

#[test]
fn focus_tracking() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    wm.register(main_window(2));

    assert_eq!(wm.focused_id(), None);

    wm.set_focused(Some(wid(1)));
    assert_eq!(wm.focused_id(), Some(wid(1)));

    wm.set_focused(Some(wid(2)));
    assert_eq!(wm.focused_id(), Some(wid(2)));

    wm.set_focused(None);
    assert_eq!(wm.focused_id(), None);
}

#[test]
fn active_main_window_returns_focused_main() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    wm.register(main_window(2));
    wm.set_focused(Some(wid(1)));

    assert_eq!(wm.active_main_window(), Some(wid(1)));
}

#[test]
fn active_main_window_returns_parent_when_dialog_focused() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    wm.register(dialog_window(10, 1));
    wm.set_focused(Some(wid(10)));

    assert_eq!(wm.active_main_window(), Some(wid(1)));
}

#[test]
fn active_main_window_none_when_no_focus() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));

    assert_eq!(wm.active_main_window(), None);
}

#[test]
fn active_main_window_returns_tear_off() {
    let mut wm = WindowManager::new();
    wm.register(tear_off_window(1));
    wm.set_focused(Some(wid(1)));

    assert_eq!(wm.active_main_window(), Some(wid(1)));
}

#[test]
fn focused_is_child_of_returns_true_for_owned_dialog() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    wm.register(dialog_window(10, 1));
    wm.set_focused(Some(wid(10)));

    assert!(wm.focused_is_child_of(wid(1)));
}

#[test]
fn focused_is_child_of_returns_false_for_main_window() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    wm.register(main_window(2));
    wm.set_focused(Some(wid(2)));

    assert!(!wm.focused_is_child_of(wid(1)));
}

#[test]
fn focused_is_child_of_returns_false_for_different_parent() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    wm.register(main_window(2));
    wm.register(dialog_window(10, 1));
    wm.set_focused(Some(wid(10)));

    assert!(!wm.focused_is_child_of(wid(2)));
}

#[test]
fn focused_is_child_of_returns_false_when_no_focus() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));

    assert!(!wm.focused_is_child_of(wid(1)));
}

#[test]
fn is_modal_blocked_by_confirmation_dialog() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    let confirm = ManagedWindow::with_parent(
        wid(10),
        WindowKind::Dialog(DialogKind::Confirmation),
        wid(1),
    );
    wm.register(confirm);

    assert!(wm.is_modal_blocked(wid(1)));
}

#[test]
fn is_not_modal_blocked_by_settings_dialog() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    wm.register(dialog_window(10, 1)); // Settings dialog

    assert!(!wm.is_modal_blocked(wid(1)));
}

#[test]
fn is_not_modal_blocked_with_no_children() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));

    assert!(!wm.is_modal_blocked(wid(1)));
}

#[test]
fn dialog_kind_is_modal() {
    assert!(DialogKind::Confirmation.is_modal());
    assert!(!DialogKind::Settings.is_modal());
    assert!(!DialogKind::About.is_modal());
}

#[test]
fn find_modal_child_returns_confirmation() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    let confirm = ManagedWindow::with_parent(
        wid(10),
        WindowKind::Dialog(DialogKind::Confirmation),
        wid(1),
    );
    wm.register(confirm);

    assert_eq!(wm.find_modal_child(wid(1)), Some(wid(10)));
}

#[test]
fn find_modal_child_skips_settings() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));
    wm.register(dialog_window(10, 1)); // Settings, non-modal

    assert_eq!(wm.find_modal_child(wid(1)), None);
}

#[test]
fn find_modal_child_none_for_no_children() {
    let mut wm = WindowManager::new();
    wm.register(main_window(1));

    assert_eq!(wm.find_modal_child(wid(1)), None);
}

#[test]
fn find_modal_child_none_for_unknown_window() {
    let wm = WindowManager::new();

    assert_eq!(wm.find_modal_child(wid(99)), None);
}
