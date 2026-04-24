//! Application cursor mode (DECCKM) and application keypad mode (DECKPAM/DECKPNM) encoding.

use winit::keyboard::{Key, NamedKey};

use super::{
    KeyEventType, Modifiers, app_cursor_mode, app_keypad_mode, enc, enc_numpad, enc_numpad_full,
    enc_numpad_text, enc_text, kitty_disambiguate_mode,
};

// --- Application cursor mode ---

#[test]
fn app_cursor_up_no_mods() {
    let r = enc(
        Key::Named(NamedKey::ArrowUp),
        Modifiers::empty(),
        app_cursor_mode(),
    );
    assert_eq!(r, b"\x1bOA");
}

#[test]
fn app_cursor_down_no_mods() {
    let r = enc(
        Key::Named(NamedKey::ArrowDown),
        Modifiers::empty(),
        app_cursor_mode(),
    );
    assert_eq!(r, b"\x1bOB");
}

#[test]
fn app_cursor_home_no_mods() {
    let r = enc(
        Key::Named(NamedKey::Home),
        Modifiers::empty(),
        app_cursor_mode(),
    );
    assert_eq!(r, b"\x1bOH");
}

#[test]
fn app_cursor_end_no_mods() {
    let r = enc(
        Key::Named(NamedKey::End),
        Modifiers::empty(),
        app_cursor_mode(),
    );
    assert_eq!(r, b"\x1bOF");
}

#[test]
fn app_cursor_up_with_ctrl() {
    // Modifiers override SS3 — use CSI format.
    let r = enc(
        Key::Named(NamedKey::ArrowUp),
        Modifiers::CONTROL,
        app_cursor_mode(),
    );
    assert_eq!(r, b"\x1b[1;5A");
}

// --- APP_KEYPAD numpad ---

#[test]
fn numpad_5_app_keypad() {
    let r = enc_numpad(
        Key::Character("5".into()),
        Modifiers::empty(),
        app_keypad_mode(),
    );
    assert_eq!(r, b"\x1bOu");
}

#[test]
fn numpad_0_app_keypad() {
    let r = enc_numpad(
        Key::Character("0".into()),
        Modifiers::empty(),
        app_keypad_mode(),
    );
    assert_eq!(r, b"\x1bOp");
}

/// Regression: BUG-08-13 — numpad digits with no `APP_KEYPAD` must emit the
/// digit byte even when winit does not populate `KeyEvent::text`. Before the
/// fix this returned empty bytes and the shell saw no keystrokes.
/// See: plans/bug-tracker/fix-BUG-08-013.md
#[test]
fn numpad_5_no_app_keypad_no_text_falls_back_to_logical_char() {
    let r = enc_numpad(
        Key::Character("5".into()),
        Modifiers::empty(),
        super::no_mode(),
    );
    assert_eq!(r, b"5");
}

#[test]
fn numpad_enter_app_keypad() {
    let r = enc_numpad(
        Key::Named(NamedKey::Enter),
        Modifiers::empty(),
        app_keypad_mode(),
    );
    assert_eq!(r, b"\x1bOM");
}

#[test]
fn numpad_plus_app_keypad() {
    let r = enc_numpad(
        Key::Character("+".into()),
        Modifiers::empty(),
        app_keypad_mode(),
    );
    assert_eq!(r, b"\x1bOk");
}

#[test]
fn numpad_minus_app_keypad() {
    let r = enc_numpad(
        Key::Character("-".into()),
        Modifiers::empty(),
        app_keypad_mode(),
    );
    assert_eq!(r, b"\x1bOm");
}

#[test]
fn numpad_star_app_keypad() {
    let r = enc_numpad(
        Key::Character("*".into()),
        Modifiers::empty(),
        app_keypad_mode(),
    );
    assert_eq!(r, b"\x1bOj");
}

#[test]
fn numpad_dot_app_keypad() {
    let r = enc_numpad(
        Key::Character(".".into()),
        Modifiers::empty(),
        app_keypad_mode(),
    );
    assert_eq!(r, b"\x1bOn");
}

#[test]
fn non_numpad_5_app_keypad() {
    // Standard location — `APP_KEYPAD` should not affect it.
    let r = enc_text(
        Key::Character("5".into()),
        Modifiers::empty(),
        app_keypad_mode(),
        "5",
    );
    assert_eq!(r, b"5");
}

// --- Numpad divide in APP_KEYPAD ---

#[test]
fn numpad_divide_app_keypad() {
    let r = enc_numpad(
        Key::Character("/".into()),
        Modifiers::empty(),
        app_keypad_mode(),
    );
    assert_eq!(r, b"\x1bOo");
}

// --- BUG-08-13 regression: numpad character keys without APP_KEYPAD ---
//
// When winit does not populate `KeyEvent::text` for numpad characters — some
// backends and certain Ctrl-combos leave it `None` — the encoder must fall
// back to the logical-key character rather than returning empty bytes.
// See: plans/bug-tracker/fix-BUG-08-013.md

/// Helper: every digit with `text=None` maps to the digit byte.
#[test]
fn numpad_digits_legacy_no_text_emit_digit_byte() {
    for digit in ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'] {
        let key = Key::Character(digit.to_string().into());
        let r = enc_numpad(key, Modifiers::empty(), super::no_mode());
        assert_eq!(
            r,
            digit.to_string().as_bytes(),
            "numpad {digit} with text=None"
        );
    }
}

/// Helper: operators + decimal with `text=None` map to their byte.
#[test]
fn numpad_operators_legacy_no_text_emit_operator_byte() {
    for op in ['+', '-', '*', '/', '.'] {
        let key = Key::Character(op.to_string().into());
        let r = enc_numpad(key, Modifiers::empty(), super::no_mode());
        assert_eq!(r, op.to_string().as_bytes(), "numpad {op} with text=None");
    }
}

/// When text IS populated, it still wins over the logical-char fallback.
#[test]
fn numpad_digit_prefers_text_over_logical_char() {
    // Contrived: logical char says "5" but winit text says "x".
    // We must forward `text` because it is locale-aware and reflects IME.
    let r = enc_numpad_text(
        Key::Character("5".into()),
        Modifiers::empty(),
        super::no_mode(),
        Some("x"),
    );
    assert_eq!(r, b"x", "text must win over Key::Character(ch)");
}

/// Alt+Numpad5 with `text=None`: ESC prefix + digit byte. Without the fix,
/// the Alt branch was gated on `text.is_some()` and returned empty.
#[test]
fn numpad_alt_digit_no_text_emits_esc_prefix_plus_digit() {
    let r = enc_numpad(Key::Character("5".into()), Modifiers::ALT, super::no_mode());
    assert_eq!(r, b"\x1b5");
}

/// Alt+Numpad5 with `text=Some("5")`: same as above (text preferred).
#[test]
fn numpad_alt_digit_with_text_emits_esc_prefix_plus_digit() {
    let r = enc_numpad_text(
        Key::Character("5".into()),
        Modifiers::ALT,
        super::no_mode(),
        Some("5"),
    );
    assert_eq!(r, b"\x1b5");
}

/// Ctrl+Numpad5 routes through `ctrl_key_byte` — digit 5 maps to GS (0x1d).
/// Unchanged by the fix; regression pin.
#[test]
fn numpad_ctrl_digit_5_emits_gs() {
    let r = enc_numpad(
        Key::Character("5".into()),
        Modifiers::CONTROL,
        super::no_mode(),
    );
    assert_eq!(r, vec![0x1d]);
}

/// Ctrl+Numpad1 — digit 1 is NOT in the Ctrl+digit shortcut table, so Ctrl
/// is ignored and the encoder falls through to the plain-text path. With
/// the fix, `text=None` falls back on "1".
#[test]
fn numpad_ctrl_digit_1_falls_through_to_digit() {
    let r = enc_numpad(
        Key::Character("1".into()),
        Modifiers::CONTROL,
        super::no_mode(),
    );
    assert_eq!(r, b"1");
}

/// NumpadEnter as Named + no APP_KEYPAD: `encode_simple_named(Enter)` →
/// `b"\r"`. Text-independent; regression pin for the already-works path.
#[test]
fn numpad_enter_named_legacy_no_text() {
    let r = enc_numpad(
        Key::Named(NamedKey::Enter),
        Modifiers::empty(),
        super::no_mode(),
    );
    assert_eq!(r, b"\r");
}

/// NumpadEnter as `Key::Character("\r")` (some backends emit this shape) +
/// no APP_KEYPAD + text=None: with the fix, falls back to the logical-char
/// bytes and emits CR.
#[test]
fn numpad_enter_character_cr_legacy_no_text() {
    let r = enc_numpad(
        Key::Character("\r".into()),
        Modifiers::empty(),
        super::no_mode(),
    );
    assert_eq!(r, b"\r");
}

/// NumpadEnter as Named in APP_KEYPAD mode: `encode_numpad_app(Enter)` →
/// `b"\x1bOM"`. Text-independent; regression pin.
#[test]
fn numpad_enter_named_app_keypad_no_text() {
    let r = enc_numpad(
        Key::Named(NamedKey::Enter),
        Modifiers::empty(),
        app_keypad_mode(),
    );
    assert_eq!(r, b"\x1bOM");
}

/// LINE_FEED_NEW_LINE + NumpadEnter (Named) + no text → `b"\r\n"`. Pins
/// interaction between LNM mode and numpad Enter.
#[test]
fn numpad_enter_named_linefeed_mode_no_text() {
    let mode = super::no_mode() | oriterm_core::TermMode::LINE_FEED_NEW_LINE;
    let r = enc_numpad(Key::Named(NamedKey::Enter), Modifiers::empty(), mode);
    assert_eq!(r, b"\r\n");
}

// --- Kitty DISAMBIGUATE path × numpad character ---

/// Numpad digit in Kitty DISAMBIGUATE mode, Press, text=None: the
/// send-as-text branch must fall back to the logical char.
#[test]
fn numpad_digit_kitty_disambiguate_press_no_text() {
    let r = enc_numpad(
        Key::Character("5".into()),
        Modifiers::empty(),
        kitty_disambiguate_mode(),
    );
    assert_eq!(r, b"5");
}

/// Negative pin (codex's refinement): numpad digit release in Kitty
/// DISAMBIGUATE WITHOUT REPORT_EVENT_TYPES must still be suppressed — the
/// fallback must not leak release bytes into the PTY.
#[test]
fn numpad_digit_kitty_disambiguate_release_no_report_events_suppressed() {
    let r = enc_numpad_full(
        Key::Character("5".into()),
        Modifiers::empty(),
        kitty_disambiguate_mode(),
        None,
        KeyEventType::Release,
    );
    assert!(
        r.is_empty(),
        "release without REPORT_EVENT_TYPES must be suppressed even with fallback"
    );
}

/// Same as above but for a non-numpad character — confirms the fix's
/// release-suppression guard fires regardless of location.
#[test]
fn standard_char_kitty_disambiguate_release_no_report_events_suppressed() {
    use winit::keyboard::KeyLocation;
    let r = super::encode_key(&super::KeyInput {
        key: &Key::Character("a".into()),
        mods: Modifiers::empty(),
        mode: kitty_disambiguate_mode(),
        text: None,
        location: KeyLocation::Standard,
        event_type: KeyEventType::Release,
        alternate_key: None,
    });
    assert!(r.is_empty());
}

/// Multi-char Character (dead-key composition, IME output) on release in
/// Kitty DISAMBIGUATE without REPORT_EVENT_TYPES must also be suppressed.
/// The `resolve_char_codepoint` None branch bypassed `should_send_as_text`
/// and would otherwise leak `"ae"` bytes — codex Phase 5 finding.
#[test]
fn multichar_character_kitty_disambiguate_release_no_report_events_suppressed() {
    use winit::keyboard::KeyLocation;
    let r = super::encode_key(&super::KeyInput {
        key: &Key::Character("ae".into()),
        mods: Modifiers::empty(),
        mode: kitty_disambiguate_mode(),
        text: Some("ae"),
        location: KeyLocation::Standard,
        event_type: KeyEventType::Release,
        alternate_key: None,
    });
    assert!(
        r.is_empty(),
        "multi-char Character release must be suppressed"
    );
}

/// Multi-char Character with text=None on release in Kitty DISAMBIGUATE
/// must also be suppressed (fallback must not leak on release).
#[test]
fn multichar_character_kitty_disambiguate_release_no_text_suppressed() {
    use winit::keyboard::KeyLocation;
    let r = super::encode_key(&super::KeyInput {
        key: &Key::Character("ae".into()),
        mods: Modifiers::empty(),
        mode: kitty_disambiguate_mode(),
        text: None,
        location: KeyLocation::Standard,
        event_type: KeyEventType::Release,
        alternate_key: None,
    });
    assert!(r.is_empty());
}

/// Multi-char Character on PRESS still emits the text (regression pin —
/// release suppression must not affect press).
#[test]
fn multichar_character_kitty_disambiguate_press_emits_text() {
    use winit::keyboard::KeyLocation;
    let r = super::encode_key(&super::KeyInput {
        key: &Key::Character("ae".into()),
        mods: Modifiers::empty(),
        mode: kitty_disambiguate_mode(),
        text: Some("ae"),
        location: KeyLocation::Standard,
        event_type: KeyEventType::Press,
        alternate_key: None,
    });
    assert_eq!(r, b"ae");
}

// --- Named keys with unambiguous legacy in Kitty mode: release suppression ---
//
// Round-1 TPR finding (codex + gemini agreement): Named keys that fall
// through the kitty.rs `has_unambiguous_legacy` shortcut to `legacy::encode_legacy`
// were bypassing the release-suppression check. The top-of-`encode_kitty`
// guard added in round-1 fix closes the leak. These pins guard against
// regression.

/// ArrowUp release in Kitty DISAMBIGUATE without REPORT_EVENT_TYPES must
/// be suppressed. Pre-fix it leaked `b"\x1b[A"` via the legacy bypass.
#[test]
fn arrow_up_kitty_disambiguate_release_no_report_events_suppressed() {
    use winit::keyboard::KeyLocation;
    let r = super::encode_key(&super::KeyInput {
        key: &Key::Named(NamedKey::ArrowUp),
        mods: Modifiers::empty(),
        mode: kitty_disambiguate_mode(),
        text: None,
        location: KeyLocation::Standard,
        event_type: KeyEventType::Release,
        alternate_key: None,
    });
    assert!(
        r.is_empty(),
        "ArrowUp release must be suppressed in DISAMBIGUATE"
    );
}

/// F1 release in Kitty DISAMBIGUATE without REPORT_EVENT_TYPES must be
/// suppressed (F1 has unambiguous legacy `\x1bOP`).
#[test]
fn f1_kitty_disambiguate_release_no_report_events_suppressed() {
    use winit::keyboard::KeyLocation;
    let r = super::encode_key(&super::KeyInput {
        key: &Key::Named(NamedKey::F1),
        mods: Modifiers::empty(),
        mode: kitty_disambiguate_mode(),
        text: None,
        location: KeyLocation::Standard,
        event_type: KeyEventType::Release,
        alternate_key: None,
    });
    assert!(r.is_empty());
}

/// Delete release (tilde-terminated, also has unambiguous legacy).
#[test]
fn delete_kitty_disambiguate_release_no_report_events_suppressed() {
    use winit::keyboard::KeyLocation;
    let r = super::encode_key(&super::KeyInput {
        key: &Key::Named(NamedKey::Delete),
        mods: Modifiers::empty(),
        mode: kitty_disambiguate_mode(),
        text: None,
        location: KeyLocation::Standard,
        event_type: KeyEventType::Release,
        alternate_key: None,
    });
    assert!(r.is_empty());
}

/// Home release (letter-terminated with `needs_app_cursor`, unambiguous).
#[test]
fn home_kitty_disambiguate_release_no_report_events_suppressed() {
    use winit::keyboard::KeyLocation;
    let r = super::encode_key(&super::KeyInput {
        key: &Key::Named(NamedKey::Home),
        mods: Modifiers::empty(),
        mode: kitty_disambiguate_mode(),
        text: None,
        location: KeyLocation::Standard,
        event_type: KeyEventType::Release,
        alternate_key: None,
    });
    assert!(r.is_empty());
}

/// ArrowUp PRESS in DISAMBIGUATE still emits via legacy (regression pin —
/// the new top-of-function guard must NOT block presses).
#[test]
fn arrow_up_kitty_disambiguate_press_emits_legacy() {
    use winit::keyboard::KeyLocation;
    let r = super::encode_key(&super::KeyInput {
        key: &Key::Named(NamedKey::ArrowUp),
        mods: Modifiers::empty(),
        mode: kitty_disambiguate_mode(),
        text: None,
        location: KeyLocation::Standard,
        event_type: KeyEventType::Press,
        alternate_key: None,
    });
    assert_eq!(r, b"\x1b[A");
}

/// ArrowUp release WITH REPORT_EVENT_TYPES still emits via CSI u (the
/// suppression guard must only fire when report_events is off).
#[test]
fn arrow_up_kitty_release_with_report_events_emits_csi_u() {
    use winit::keyboard::KeyLocation;
    let mode = kitty_disambiguate_mode() | oriterm_core::TermMode::REPORT_EVENT_TYPES;
    let r = super::encode_key(&super::KeyInput {
        key: &Key::Named(NamedKey::ArrowUp),
        mods: Modifiers::empty(),
        mode,
        text: None,
        location: KeyLocation::Standard,
        event_type: KeyEventType::Release,
        alternate_key: None,
    });
    assert_eq!(r, b"\x1b[1;1:3A");
}

// --- Round-2 TPR (gemini): multi-char and Unidentified arms must suppress
// --- non-Press events even when REPORT_EVENT_TYPES is active. There is no
// --- codepoint to encode, so leaking raw text on release/repeat would be
// --- a protocol violation.

/// Multi-char Character on Release with REPORT_EVENT_TYPES active: must
/// suppress (no codepoint for CSI u, so raw text would be a leak).
#[test]
fn multichar_character_kitty_release_with_report_events_suppressed() {
    use winit::keyboard::KeyLocation;
    let mode = kitty_disambiguate_mode() | oriterm_core::TermMode::REPORT_EVENT_TYPES;
    let r = super::encode_key(&super::KeyInput {
        key: &Key::Character("ae".into()),
        mods: Modifiers::empty(),
        mode,
        text: Some("ae"),
        location: KeyLocation::Standard,
        event_type: KeyEventType::Release,
        alternate_key: None,
    });
    assert!(
        r.is_empty(),
        "multi-char release with REPORT_EVENT_TYPES must not leak raw text"
    );
}

/// Multi-char Character on Repeat with REPORT_EVENT_TYPES active: same.
#[test]
fn multichar_character_kitty_repeat_with_report_events_suppressed() {
    use winit::keyboard::KeyLocation;
    let mode = kitty_disambiguate_mode() | oriterm_core::TermMode::REPORT_EVENT_TYPES;
    let r = super::encode_key(&super::KeyInput {
        key: &Key::Character("ae".into()),
        mods: Modifiers::empty(),
        mode,
        text: Some("ae"),
        location: KeyLocation::Standard,
        event_type: KeyEventType::Repeat,
        alternate_key: None,
    });
    assert!(r.is_empty());
}

/// Multi-char Character on Press with REPORT_EVENT_TYPES still emits text.
/// (Regression pin — guard must only fire on non-Press.)
#[test]
fn multichar_character_kitty_press_with_report_events_emits_text() {
    use winit::keyboard::KeyLocation;
    let mode = kitty_disambiguate_mode() | oriterm_core::TermMode::REPORT_EVENT_TYPES;
    let r = super::encode_key(&super::KeyInput {
        key: &Key::Character("ae".into()),
        mods: Modifiers::empty(),
        mode,
        text: Some("ae"),
        location: KeyLocation::Standard,
        event_type: KeyEventType::Press,
        alternate_key: None,
    });
    assert_eq!(r, b"ae");
}

/// Unidentified key on Release with REPORT_EVENT_TYPES active: must suppress.
#[test]
fn unidentified_kitty_release_with_report_events_suppressed() {
    use winit::keyboard::{KeyLocation, NativeKey};
    let mode = kitty_disambiguate_mode() | oriterm_core::TermMode::REPORT_EVENT_TYPES;
    let r = super::encode_key(&super::KeyInput {
        key: &Key::Unidentified(NativeKey::Unidentified),
        mods: Modifiers::empty(),
        mode,
        text: Some("x"),
        location: KeyLocation::Standard,
        event_type: KeyEventType::Release,
        alternate_key: None,
    });
    assert!(r.is_empty());
}

/// Unidentified key on Press with REPORT_EVENT_TYPES still emits text.
#[test]
fn unidentified_kitty_press_with_report_events_emits_text() {
    use winit::keyboard::{KeyLocation, NativeKey};
    let mode = kitty_disambiguate_mode() | oriterm_core::TermMode::REPORT_EVENT_TYPES;
    let r = super::encode_key(&super::KeyInput {
        key: &Key::Unidentified(NativeKey::Unidentified),
        mods: Modifiers::empty(),
        mode,
        text: Some("x"),
        location: KeyLocation::Standard,
        event_type: KeyEventType::Press,
        alternate_key: None,
    });
    assert_eq!(r, b"x");
}

// --- Round-3 TPR (codex): Repeat without REPORT_EVENT_TYPES is press-
// --- equivalent per `resolve_event_suffix`. Multi-char and Unidentified
// --- arms must NOT suppress Repeat unless REPORT_EVENT_TYPES is active.

/// Multi-char Character Repeat WITHOUT REPORT_EVENT_TYPES is press-
/// equivalent — must emit text. `resolve_event_suffix(false, Repeat)`
/// returns `Some("")`, so the encoder treats Repeat as ordinary press
/// emission outside Kitty event-type reporting.
#[test]
fn multichar_character_kitty_repeat_no_report_events_emits_text() {
    use winit::keyboard::KeyLocation;
    let r = super::encode_key(&super::KeyInput {
        key: &Key::Character("ae".into()),
        mods: Modifiers::empty(),
        mode: kitty_disambiguate_mode(),
        text: Some("ae"),
        location: KeyLocation::Standard,
        event_type: KeyEventType::Repeat,
        alternate_key: None,
    });
    assert_eq!(
        r, b"ae",
        "repeat without REPORT_EVENT_TYPES = press-equivalent"
    );
}

/// Unidentified Repeat WITHOUT REPORT_EVENT_TYPES is press-equivalent.
#[test]
fn unidentified_kitty_repeat_no_report_events_emits_text() {
    use winit::keyboard::{KeyLocation, NativeKey};
    let r = super::encode_key(&super::KeyInput {
        key: &Key::Unidentified(NativeKey::Unidentified),
        mods: Modifiers::empty(),
        mode: kitty_disambiguate_mode(),
        text: Some("x"),
        location: KeyLocation::Standard,
        event_type: KeyEventType::Repeat,
        alternate_key: None,
    });
    assert_eq!(r, b"x");
}

/// Multi-char Character Repeat WITH REPORT_EVENT_TYPES is suppressed
/// (no codepoint to encode in CSI u). Pin to keep the report-events-on
/// branch suppressing.
#[test]
fn multichar_character_kitty_repeat_with_report_events_suppressed_pin() {
    use winit::keyboard::KeyLocation;
    let mode = kitty_disambiguate_mode() | oriterm_core::TermMode::REPORT_EVENT_TYPES;
    let r = super::encode_key(&super::KeyInput {
        key: &Key::Character("ae".into()),
        mods: Modifiers::empty(),
        mode,
        text: Some("ae"),
        location: KeyLocation::Standard,
        event_type: KeyEventType::Repeat,
        alternate_key: None,
    });
    assert!(r.is_empty());
}

// --- Round-4 TPR (codex): Unidentified matrix completeness — explicit
// --- pins for the three cells codex identified as untested.

/// Unidentified Press WITHOUT REPORT_EVENT_TYPES emits text (baseline).
#[test]
fn unidentified_kitty_press_no_report_events_emits_text() {
    use winit::keyboard::{KeyLocation, NativeKey};
    let r = super::encode_key(&super::KeyInput {
        key: &Key::Unidentified(NativeKey::Unidentified),
        mods: Modifiers::empty(),
        mode: kitty_disambiguate_mode(),
        text: Some("x"),
        location: KeyLocation::Standard,
        event_type: KeyEventType::Press,
        alternate_key: None,
    });
    assert_eq!(r, b"x");
}

/// Unidentified Release WITHOUT REPORT_EVENT_TYPES is suppressed by the
/// top-of-function guard.
#[test]
fn unidentified_kitty_release_no_report_events_suppressed() {
    use winit::keyboard::{KeyLocation, NativeKey};
    let r = super::encode_key(&super::KeyInput {
        key: &Key::Unidentified(NativeKey::Unidentified),
        mods: Modifiers::empty(),
        mode: kitty_disambiguate_mode(),
        text: Some("x"),
        location: KeyLocation::Standard,
        event_type: KeyEventType::Release,
        alternate_key: None,
    });
    assert!(r.is_empty());
}

/// Unidentified Repeat WITH REPORT_EVENT_TYPES is suppressed by the
/// per-arm guard (no codepoint to encode).
#[test]
fn unidentified_kitty_repeat_with_report_events_suppressed() {
    use winit::keyboard::{KeyLocation, NativeKey};
    let mode = kitty_disambiguate_mode() | oriterm_core::TermMode::REPORT_EVENT_TYPES;
    let r = super::encode_key(&super::KeyInput {
        key: &Key::Unidentified(NativeKey::Unidentified),
        mods: Modifiers::empty(),
        mode,
        text: Some("x"),
        location: KeyLocation::Standard,
        event_type: KeyEventType::Repeat,
        alternate_key: None,
    });
    assert!(r.is_empty());
}
