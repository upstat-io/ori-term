//! Tests for mouse cursor hiding decision logic.

use winit::keyboard::{Key, NamedKey};

use super::{HideContext, should_hide_cursor};

/// Helper to build a `HideContext` with common defaults.
fn ctx(key: &Key) -> HideContext<'_> {
    HideContext {
        config_enabled: true,
        already_hidden: false,
        key,
        mouse_reporting: false,
        ime_active: false,
    }
}

#[test]
fn keypress_hides_when_enabled() {
    let key = Key::Character("a".into());
    assert!(should_hide_cursor(&ctx(&key)));
}

#[test]
fn already_hidden_skips() {
    let key = Key::Character("a".into());
    let c = HideContext {
        already_hidden: true,
        ..ctx(&key)
    };
    assert!(!should_hide_cursor(&c));
}

#[test]
fn config_disabled_skips() {
    let key = Key::Character("a".into());
    let c = HideContext {
        config_enabled: false,
        ..ctx(&key)
    };
    assert!(!should_hide_cursor(&c));
}

#[test]
fn modifier_only_does_not_hide() {
    for named in [
        NamedKey::Shift,
        NamedKey::Control,
        NamedKey::Alt,
        NamedKey::Super,
    ] {
        let key = Key::Named(named);
        assert!(
            !should_hide_cursor(&ctx(&key)),
            "modifier {named:?} should not hide cursor"
        );
    }
}

#[test]
fn mouse_reporting_prevents_hiding() {
    let key = Key::Character("a".into());
    let c = HideContext {
        mouse_reporting: true,
        ..ctx(&key)
    };
    assert!(!should_hide_cursor(&c));
}

#[test]
fn ime_active_prevents_hiding() {
    let key = Key::Character("a".into());
    let c = HideContext {
        ime_active: true,
        ..ctx(&key)
    };
    assert!(!should_hide_cursor(&c));
}

#[test]
fn named_action_keys_hide() {
    for named in [NamedKey::Enter, NamedKey::Space, NamedKey::Backspace] {
        let key = Key::Named(named);
        assert!(
            should_hide_cursor(&ctx(&key)),
            "action key {named:?} should hide cursor"
        );
    }
}
