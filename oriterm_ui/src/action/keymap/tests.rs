use crate::action::keymap_action::{Activate, Dismiss};
use crate::actions;
use crate::input::{Key, Modifiers};

use super::{KeyBinding, Keymap, Keystroke};

#[test]
fn lookup_exact_match() {
    let keymap = Keymap::defaults();
    let ks = Keystroke::new(Key::Tab, Modifiers::NONE);
    let action = keymap.lookup(ks, &[]);
    assert!(action.is_some());
    assert_eq!(action.unwrap().name(), "widget::FocusNext");
}

#[test]
fn lookup_with_modifiers() {
    let keymap = Keymap::defaults();
    let ks = Keystroke::new(Key::Tab, Modifiers::SHIFT_ONLY);
    let action = keymap.lookup(ks, &[]);
    assert!(action.is_some());
    assert_eq!(action.unwrap().name(), "widget::FocusPrev");
}

#[test]
fn lookup_no_match() {
    let keymap = Keymap::defaults();
    let ks = Keystroke::new(Key::Character('z'), Modifiers::NONE);
    assert!(keymap.lookup(ks, &[]).is_none());
}

#[test]
fn context_scoping_button() {
    let keymap = Keymap::defaults();
    let enter = Keystroke::new(Key::Enter, Modifiers::NONE);

    // Without Button context, Enter has no global match (Activate is scoped).
    assert!(keymap.lookup(enter, &[]).is_none());

    // With Button context, Enter matches Activate.
    let action = keymap.lookup(enter, &["Button"]);
    assert!(action.is_some());
    assert_eq!(action.unwrap().name(), "widget::Activate");
}

#[test]
fn context_scoping_dropdown() {
    let keymap = Keymap::defaults();
    let arrow_down = Keystroke::new(Key::ArrowDown, Modifiers::NONE);

    // Without context, ArrowDown has no match.
    assert!(keymap.lookup(arrow_down, &[]).is_none());

    // In Dropdown context, ArrowDown -> NavigateDown.
    let action = keymap.lookup(arrow_down, &["Dropdown"]);
    assert_eq!(action.unwrap().name(), "widget::NavigateDown");

    // In Slider context, ArrowDown -> DecrementValue.
    let action = keymap.lookup(arrow_down, &["Slider"]);
    assert_eq!(action.unwrap().name(), "widget::DecrementValue");
}

#[test]
fn context_precedence_deeper_wins() {
    // Manually construct to control order: global binding vs contextual.
    let mut keymap = Keymap::new();
    keymap.rebind(KeyBinding::new(
        Keystroke::new(Key::Escape, Modifiers::NONE),
        Dismiss,
        None, // global
    ));
    keymap.rebind(KeyBinding::new(
        Keystroke::new(Key::Escape, Modifiers::NONE),
        Dismiss,
        Some("Dialog"),
    ));

    // With Dialog in context, the scoped binding wins.
    let esc = Keystroke::new(Key::Escape, Modifiers::NONE);
    let action = keymap.lookup(esc, &["Dialog"]);
    assert_eq!(action.unwrap().name(), "widget::Dismiss");

    // Without context, the global binding matches.
    let action = keymap.lookup(esc, &[]);
    assert_eq!(action.unwrap().name(), "widget::Dismiss");
}

#[test]
fn deeper_context_stack_position_wins() {
    // Two bindings for same key, different contexts.
    actions!(test_ns, [ActionA, ActionB]);
    let mut keymap = Keymap::new();
    keymap.rebind(KeyBinding::new(
        Keystroke::new(Key::Enter, Modifiers::NONE),
        ActionA,
        Some("Outer"),
    ));
    keymap.rebind(KeyBinding::new(
        Keystroke::new(Key::Enter, Modifiers::NONE),
        ActionB,
        Some("Inner"),
    ));

    // Stack: ["Outer", "Inner"] — Inner is deeper, ActionB wins.
    let enter = Keystroke::new(Key::Enter, Modifiers::NONE);
    let action = keymap.lookup(enter, &["Outer", "Inner"]);
    assert_eq!(action.unwrap().name(), "test_ns::ActionB");

    // Stack: ["Inner", "Outer"] — Outer is deeper, ActionA wins.
    let action = keymap.lookup(enter, &["Inner", "Outer"]);
    assert_eq!(action.unwrap().name(), "test_ns::ActionA");
}

#[test]
fn rebind_replaces_existing() {
    let mut keymap = Keymap::defaults();
    let tab = Keystroke::new(Key::Tab, Modifiers::NONE);

    // Initially Tab -> FocusNext.
    assert_eq!(keymap.lookup(tab, &[]).unwrap().name(), "widget::FocusNext");

    // Rebind Tab (global) to Activate.
    keymap.rebind(KeyBinding::new(tab, Activate, None));
    assert_eq!(keymap.lookup(tab, &[]).unwrap().name(), "widget::Activate");
}

#[test]
fn rebind_appends_new() {
    let mut keymap = Keymap::new();
    let f1 = Keystroke::new(Key::Character('?'), Modifiers::NONE);
    assert!(keymap.lookup(f1, &[]).is_none());

    keymap.rebind(KeyBinding::new(f1, Activate, None));
    assert_eq!(keymap.lookup(f1, &[]).unwrap().name(), "widget::Activate");
}

#[test]
fn unmatched_fallthrough() {
    let keymap = Keymap::defaults();
    // A random character key not in the keymap.
    let ks = Keystroke::new(Key::Character('x'), Modifiers::NONE);
    assert!(keymap.lookup(ks, &[]).is_none());
}

#[test]
fn defaults_cover_all_expected_bindings() {
    let keymap = Keymap::defaults();
    let m = Modifiers::NONE;

    // Focus traversal (global).
    assert_eq!(
        keymap
            .lookup(Keystroke::new(Key::Tab, m), &[])
            .unwrap()
            .name(),
        "widget::FocusNext"
    );
    assert_eq!(
        keymap
            .lookup(Keystroke::new(Key::Tab, Modifiers::SHIFT_ONLY), &[])
            .unwrap()
            .name(),
        "widget::FocusPrev"
    );

    // Button activation.
    assert_eq!(
        keymap
            .lookup(Keystroke::new(Key::Enter, m), &["Button"])
            .unwrap()
            .name(),
        "widget::Activate"
    );
    assert_eq!(
        keymap
            .lookup(Keystroke::new(Key::Space, m), &["Button"])
            .unwrap()
            .name(),
        "widget::Activate"
    );

    // Dropdown.
    assert_eq!(
        keymap
            .lookup(Keystroke::new(Key::ArrowDown, m), &["Dropdown"])
            .unwrap()
            .name(),
        "widget::NavigateDown"
    );
    assert_eq!(
        keymap
            .lookup(Keystroke::new(Key::ArrowUp, m), &["Dropdown"])
            .unwrap()
            .name(),
        "widget::NavigateUp"
    );
    assert_eq!(
        keymap
            .lookup(Keystroke::new(Key::Enter, m), &["Dropdown"])
            .unwrap()
            .name(),
        "widget::Confirm"
    );
    assert_eq!(
        keymap
            .lookup(Keystroke::new(Key::Space, m), &["Dropdown"])
            .unwrap()
            .name(),
        "widget::Confirm"
    );
    assert_eq!(
        keymap
            .lookup(Keystroke::new(Key::Escape, m), &["Dropdown"])
            .unwrap()
            .name(),
        "widget::Dismiss"
    );

    // Menu.
    assert_eq!(
        keymap
            .lookup(Keystroke::new(Key::ArrowDown, m), &["Menu"])
            .unwrap()
            .name(),
        "widget::NavigateDown"
    );
    assert_eq!(
        keymap
            .lookup(Keystroke::new(Key::Enter, m), &["Menu"])
            .unwrap()
            .name(),
        "widget::Confirm"
    );
    assert_eq!(
        keymap
            .lookup(Keystroke::new(Key::Space, m), &["Menu"])
            .unwrap()
            .name(),
        "widget::Confirm"
    );
    assert_eq!(
        keymap
            .lookup(Keystroke::new(Key::Escape, m), &["Menu"])
            .unwrap()
            .name(),
        "widget::Dismiss"
    );

    // Dialog.
    assert_eq!(
        keymap
            .lookup(Keystroke::new(Key::Escape, m), &["Dialog"])
            .unwrap()
            .name(),
        "widget::Dismiss"
    );

    // Slider.
    assert_eq!(
        keymap
            .lookup(Keystroke::new(Key::ArrowRight, m), &["Slider"])
            .unwrap()
            .name(),
        "widget::IncrementValue"
    );
    assert_eq!(
        keymap
            .lookup(Keystroke::new(Key::ArrowLeft, m), &["Slider"])
            .unwrap()
            .name(),
        "widget::DecrementValue"
    );
    assert_eq!(
        keymap
            .lookup(Keystroke::new(Key::Home, m), &["Slider"])
            .unwrap()
            .name(),
        "widget::ValueToMin"
    );
    assert_eq!(
        keymap
            .lookup(Keystroke::new(Key::End, m), &["Slider"])
            .unwrap()
            .name(),
        "widget::ValueToMax"
    );
}
