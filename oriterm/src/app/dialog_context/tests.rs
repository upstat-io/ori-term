//! Regression tests for dialog context dispatch logic.
//!
//! Tests cover the extracted helpers (redraw decision) and key conversion
//! functions. `winit_key_to_input_event` takes `&winit::event::KeyEvent`
//! whose `platform_specific` field is `pub(crate)` in winit, so we test
//! the two sub-functions it delegates to (`winit_key_to_ui_key` and
//! `winit_mods_to_ui`) individually instead.

use oriterm_ui::controllers::ControllerRequests;
use oriterm_ui::input::{Key as UiKey, Modifiers as UiModifiers};
use winit::keyboard::{Key, ModifiersState, NamedKey};

use super::key_conversion::{winit_key_to_ui_key, winit_mods_to_ui};
use super::needs_content_redraw;

// ---------------------------------------------------------------------------
// needs_content_redraw — regression for TPR-10-014
// ---------------------------------------------------------------------------

#[test]
fn redraw_when_handled_only() {
    assert!(needs_content_redraw(true, false, ControllerRequests::NONE));
}

#[test]
fn redraw_when_state_changed_only() {
    assert!(needs_content_redraw(false, true, ControllerRequests::NONE));
}

#[test]
fn redraw_when_paint_requested_only() {
    assert!(needs_content_redraw(
        false,
        false,
        ControllerRequests::PAINT
    ));
}

#[test]
fn no_redraw_when_nothing_happened() {
    assert!(!needs_content_redraw(
        false,
        false,
        ControllerRequests::NONE
    ));
}

#[test]
fn redraw_when_all_signals_present() {
    assert!(needs_content_redraw(true, true, ControllerRequests::PAINT));
}

#[test]
fn redraw_ignores_non_paint_requests() {
    // ANIM_FRAME alone should not trigger redraw.
    assert!(!needs_content_redraw(
        false,
        false,
        ControllerRequests::ANIM_FRAME
    ));
}

// ---------------------------------------------------------------------------
// key_conversion — winit_key_to_ui_key
// ---------------------------------------------------------------------------

#[test]
fn tab_maps_to_ui_tab() {
    assert_eq!(
        winit_key_to_ui_key(&Key::Named(NamedKey::Tab)),
        Some(UiKey::Tab)
    );
}

#[test]
fn enter_maps_to_ui_enter() {
    assert_eq!(
        winit_key_to_ui_key(&Key::Named(NamedKey::Enter)),
        Some(UiKey::Enter)
    );
}

#[test]
fn space_maps_to_ui_space() {
    assert_eq!(
        winit_key_to_ui_key(&Key::Named(NamedKey::Space)),
        Some(UiKey::Space)
    );
}

#[test]
fn escape_maps_to_ui_escape() {
    assert_eq!(
        winit_key_to_ui_key(&Key::Named(NamedKey::Escape)),
        Some(UiKey::Escape)
    );
}

#[test]
fn backspace_maps_to_ui_backspace() {
    assert_eq!(
        winit_key_to_ui_key(&Key::Named(NamedKey::Backspace)),
        Some(UiKey::Backspace)
    );
}

#[test]
fn delete_maps_to_ui_delete() {
    assert_eq!(
        winit_key_to_ui_key(&Key::Named(NamedKey::Delete)),
        Some(UiKey::Delete)
    );
}

#[test]
fn home_end_map() {
    assert_eq!(
        winit_key_to_ui_key(&Key::Named(NamedKey::Home)),
        Some(UiKey::Home)
    );
    assert_eq!(
        winit_key_to_ui_key(&Key::Named(NamedKey::End)),
        Some(UiKey::End)
    );
}

#[test]
fn arrow_keys_all_map() {
    assert_eq!(
        winit_key_to_ui_key(&Key::Named(NamedKey::ArrowUp)),
        Some(UiKey::ArrowUp)
    );
    assert_eq!(
        winit_key_to_ui_key(&Key::Named(NamedKey::ArrowDown)),
        Some(UiKey::ArrowDown)
    );
    assert_eq!(
        winit_key_to_ui_key(&Key::Named(NamedKey::ArrowLeft)),
        Some(UiKey::ArrowLeft)
    );
    assert_eq!(
        winit_key_to_ui_key(&Key::Named(NamedKey::ArrowRight)),
        Some(UiKey::ArrowRight)
    );
}

#[test]
fn page_up_down_map() {
    assert_eq!(
        winit_key_to_ui_key(&Key::Named(NamedKey::PageUp)),
        Some(UiKey::PageUp)
    );
    assert_eq!(
        winit_key_to_ui_key(&Key::Named(NamedKey::PageDown)),
        Some(UiKey::PageDown)
    );
}

#[test]
fn character_key_maps() {
    assert_eq!(
        winit_key_to_ui_key(&Key::Character("a".into())),
        Some(UiKey::Character('a'))
    );
    assert_eq!(
        winit_key_to_ui_key(&Key::Character("Z".into())),
        Some(UiKey::Character('Z'))
    );
}

#[test]
fn function_key_returns_none() {
    assert_eq!(winit_key_to_ui_key(&Key::Named(NamedKey::F1)), None);
    assert_eq!(winit_key_to_ui_key(&Key::Named(NamedKey::F12)), None);
}

#[test]
fn media_key_returns_none() {
    assert_eq!(winit_key_to_ui_key(&Key::Named(NamedKey::MediaPlay)), None);
}

#[test]
fn dead_key_returns_none() {
    assert_eq!(winit_key_to_ui_key(&Key::Dead(None)), None);
}

// ---------------------------------------------------------------------------
// key_conversion — winit_mods_to_ui
// ---------------------------------------------------------------------------

#[test]
fn empty_mods_maps_to_none() {
    assert_eq!(winit_mods_to_ui(ModifiersState::empty()), UiModifiers::NONE);
}

#[test]
fn shift_maps() {
    let ui = winit_mods_to_ui(ModifiersState::SHIFT);
    assert!(ui.shift());
    assert!(!ui.ctrl());
    assert!(!ui.alt());
}

#[test]
fn ctrl_maps() {
    let ui = winit_mods_to_ui(ModifiersState::CONTROL);
    assert!(ui.ctrl());
    assert!(!ui.shift());
}

#[test]
fn alt_maps() {
    let ui = winit_mods_to_ui(ModifiersState::ALT);
    assert!(ui.alt());
}

#[test]
fn super_maps() {
    let ui = winit_mods_to_ui(ModifiersState::SUPER);
    assert!(ui.logo());
}

#[test]
fn combined_mods_map() {
    let mods = ModifiersState::CONTROL | ModifiersState::SHIFT;
    let ui = winit_mods_to_ui(mods);
    assert!(ui.ctrl());
    assert!(ui.shift());
    assert!(!ui.alt());
    assert!(!ui.logo());

    let mods = ModifiersState::ALT | ModifiersState::SUPER;
    let ui = winit_mods_to_ui(mods);
    assert!(ui.alt());
    assert!(ui.logo());
    assert!(!ui.ctrl());
    assert!(!ui.shift());
}
