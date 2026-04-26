//! Kitty keyboard protocol encoding (CSI u, modifiers, event types, associated
//! text, alternate keys, dispatch precedence).

use winit::keyboard::{Key, KeyLocation, NamedKey};

use oriterm_core::TermMode;

use super::{KeyEventType, KeyInput, Modifiers, enc, enc_release, enc_text, encode_key, no_mode};

// --- Kitty mode helpers ---

fn kitty_disambiguate() -> TermMode {
    TermMode::default() | TermMode::DISAMBIGUATE_ESC_CODES
}

fn kitty_report_events() -> TermMode {
    TermMode::default() | TermMode::DISAMBIGUATE_ESC_CODES | TermMode::REPORT_EVENT_TYPES
}

fn kitty_report_all() -> TermMode {
    TermMode::default() | TermMode::REPORT_ALL_KEYS_AS_ESC
}

/// Encode with custom event type.
fn enc_event(
    key: Key,
    mods: Modifiers,
    mode: TermMode,
    text: Option<&str>,
    event_type: KeyEventType,
) -> Vec<u8> {
    encode_key(&KeyInput {
        key: &key,
        mods,
        mode,
        text,
        location: KeyLocation::Standard,
        event_type,
        alternate_key: None,
    })
}

fn kitty_report_text() -> TermMode {
    TermMode::default() | TermMode::REPORT_ALL_KEYS_AS_ESC | TermMode::REPORT_ASSOCIATED_TEXT
}

fn kitty_report_text_events() -> TermMode {
    kitty_report_text() | TermMode::REPORT_EVENT_TYPES
}

fn kitty_all_flags() -> TermMode {
    TermMode::default()
        | TermMode::DISAMBIGUATE_ESC_CODES
        | TermMode::REPORT_EVENT_TYPES
        | TermMode::REPORT_ALL_KEYS_AS_ESC
        | TermMode::REPORT_ASSOCIATED_TEXT
}

// --- Kitty: basic CSI u encoding ---

#[test]
fn kitty_escape() {
    let r = enc(
        Key::Named(NamedKey::Escape),
        Modifiers::empty(),
        kitty_disambiguate(),
    );
    assert_eq!(r, b"\x1b[27u");
}

#[test]
fn kitty_enter() {
    let r = enc(
        Key::Named(NamedKey::Enter),
        Modifiers::empty(),
        kitty_disambiguate(),
    );
    assert_eq!(r, b"\x1b[13u");
}

#[test]
fn kitty_tab() {
    let r = enc(
        Key::Named(NamedKey::Tab),
        Modifiers::empty(),
        kitty_disambiguate(),
    );
    assert_eq!(r, b"\x1b[9u");
}

#[test]
fn kitty_backspace() {
    let r = enc(
        Key::Named(NamedKey::Backspace),
        Modifiers::empty(),
        kitty_disambiguate(),
    );
    assert_eq!(r, b"\x1b[127u");
}

#[test]
fn kitty_f1() {
    // F1 has an unambiguous legacy sequence (SS3 P), so DISAMBIGUATE_ESC_CODES
    // alone falls back to legacy encoding.
    let r = enc(
        Key::Named(NamedKey::F1),
        Modifiers::empty(),
        kitty_disambiguate(),
    );
    assert_eq!(r, b"\x1bOP");
}

#[test]
fn kitty_f1_report_all() {
    // With REPORT_ALL_KEYS_AS_ESC, F1 uses its legacy terminator `P`.
    let r = enc(
        Key::Named(NamedKey::F1),
        Modifiers::empty(),
        kitty_all_flags(),
    );
    assert_eq!(r, b"\x1b[1P");
}

#[test]
fn kitty_arrow_up() {
    // ArrowUp has an unambiguous legacy sequence (CSI A), so
    // DISAMBIGUATE_ESC_CODES alone falls back to legacy.
    let r = enc(
        Key::Named(NamedKey::ArrowUp),
        Modifiers::empty(),
        kitty_disambiguate(),
    );
    assert_eq!(r, b"\x1b[A");
}

#[test]
fn kitty_arrow_up_report_all() {
    // With REPORT_ALL_KEYS_AS_ESC, ArrowUp uses its legacy terminator `A`.
    let r = enc(
        Key::Named(NamedKey::ArrowUp),
        Modifiers::empty(),
        kitty_all_flags(),
    );
    assert_eq!(r, b"\x1b[1A");
}

#[test]
fn kitty_end_disambiguate_uses_legacy() {
    // End has an unambiguous legacy sequence (CSI F), so
    // DISAMBIGUATE_ESC_CODES alone should NOT produce CSI 57357 u.
    let r = enc(
        Key::Named(NamedKey::End),
        Modifiers::empty(),
        kitty_disambiguate(),
    );
    assert_eq!(r, b"\x1b[F");
}

#[test]
fn kitty_home_disambiguate_uses_legacy() {
    let r = enc(
        Key::Named(NamedKey::Home),
        Modifiers::empty(),
        kitty_disambiguate(),
    );
    assert_eq!(r, b"\x1b[H");
}

#[test]
fn kitty_delete_disambiguate_uses_legacy() {
    let r = enc(
        Key::Named(NamedKey::Delete),
        Modifiers::empty(),
        kitty_disambiguate(),
    );
    assert_eq!(r, b"\x1b[3~");
}

#[test]
fn kitty_page_up_disambiguate_uses_legacy() {
    let r = enc(
        Key::Named(NamedKey::PageUp),
        Modifiers::empty(),
        kitty_disambiguate(),
    );
    assert_eq!(r, b"\x1b[5~");
}

// --- Kitty: modifiers ---

#[test]
fn kitty_ctrl_a() {
    let r = enc(
        Key::Character("a".into()),
        Modifiers::CONTROL,
        kitty_disambiguate(),
    );
    assert_eq!(r, b"\x1b[97;5u");
}

#[test]
fn kitty_shift_tab() {
    let r = enc(
        Key::Named(NamedKey::Tab),
        Modifiers::SHIFT,
        kitty_disambiguate(),
    );
    assert_eq!(r, b"\x1b[9;2u");
}

#[test]
fn kitty_shift_a() {
    let r = enc(
        Key::Character("A".into()),
        Modifiers::SHIFT,
        kitty_disambiguate(),
    );
    // 'A' is codepoint 65, Shift modifier param = 2.
    assert_eq!(r, b"\x1b[65;2u");
}

// --- Kitty: plain text passthrough ---

#[test]
fn kitty_plain_text() {
    // Printable char with no mods — should send as plain text, not CSI u.
    let r = enc_text(
        Key::Character("a".into()),
        Modifiers::empty(),
        kitty_disambiguate(),
        "a",
    );
    assert_eq!(r, b"a");
}

/// Regression: BUG-08-013 — `Key::Character("a")` in Kitty DISAMBIGUATE with
/// no `text` must fall back to the logical-char byte rather than returning
/// empty. Prior to the fix this returned an empty vec and the shell silently
/// dropped the keystroke on backends that don't populate `text`.
/// See: bug-tracker/plans/completed/BUG-08-013/00-overview.md
#[test]
fn kitty_plain_char_no_text_field_falls_back_to_logical_char() {
    let r = enc(
        Key::Character("a".into()),
        Modifiers::empty(),
        kitty_disambiguate(),
    );
    assert_eq!(r, b"a");
}

// --- Kitty: REPORT_ALL_KEYS forces CSI u ---

#[test]
fn kitty_report_all_plain_char() {
    // REPORT_ALL_KEYS forces even plain text through CSI u.
    let r = enc_text(
        Key::Character("a".into()),
        Modifiers::empty(),
        kitty_report_all(),
        "a",
    );
    assert_eq!(r, b"\x1b[97u");
}

// --- Kitty: event types ---

#[test]
fn kitty_release_without_report_events() {
    // DISAMBIGUATE only — release should produce nothing.
    let r = enc_release(
        Key::Named(NamedKey::Escape),
        Modifiers::empty(),
        kitty_disambiguate(),
    );
    assert!(r.is_empty());
}

#[test]
fn kitty_release_with_report_events() {
    let r = enc_event(
        Key::Named(NamedKey::Escape),
        Modifiers::empty(),
        kitty_report_events(),
        None,
        KeyEventType::Release,
    );
    assert_eq!(r, b"\x1b[27;1:3u");
}

#[test]
fn kitty_repeat_with_report_events() {
    let r = enc_event(
        Key::Character("a".into()),
        Modifiers::empty(),
        kitty_report_events(),
        Some("a"),
        KeyEventType::Repeat,
    );
    assert_eq!(r, b"\x1b[97;1:2u");
}

#[test]
fn kitty_press_with_report_events() {
    // Press is the default — event type suffix omitted.
    let r = enc_event(
        Key::Named(NamedKey::Escape),
        Modifiers::empty(),
        kitty_report_events(),
        None,
        KeyEventType::Press,
    );
    assert_eq!(r, b"\x1b[27u");
}

// --- Kitty: char release with REPORT_EVENT_TYPES ---

#[test]
fn kitty_char_release_with_report_events() {
    let r = enc_event(
        Key::Character("a".into()),
        Modifiers::empty(),
        kitty_report_events(),
        Some("a"),
        KeyEventType::Release,
    );
    assert_eq!(r, b"\x1b[97;1:3u");
}

// --- Kitty: modifier + event type combined ---

#[test]
fn kitty_ctrl_a_release() {
    let r = enc_event(
        Key::Character("a".into()),
        Modifiers::CONTROL,
        kitty_report_events(),
        None,
        KeyEventType::Release,
    );
    assert_eq!(r, b"\x1b[97;5:3u");
}

// --- Legacy release still suppressed ---

#[test]
fn legacy_release_still_empty() {
    let r = enc_release(Key::Named(NamedKey::ArrowUp), Modifiers::empty(), no_mode());
    assert!(r.is_empty());
}

// --- Dispatch priority: Kitty overrides legacy ---

#[test]
fn kitty_disambiguate_uses_legacy_for_unambiguous_arrow() {
    // DISAMBIGUATE_ESC_CODES alone should use legacy for ArrowUp (unambiguous).
    let legacy = enc(Key::Named(NamedKey::ArrowUp), Modifiers::empty(), no_mode());
    let kitty = enc(
        Key::Named(NamedKey::ArrowUp),
        Modifiers::empty(),
        kitty_disambiguate(),
    );
    assert_eq!(legacy, b"\x1b[A");
    assert_eq!(kitty, legacy);
}

#[test]
fn kitty_report_all_overrides_legacy_for_arrow_up() {
    // REPORT_ALL_KEYS_AS_ESC forces CSI encoding (not legacy SS3/CSI)
    // but uses the legacy terminator `A` per the Kitty spec.
    let kitty = enc(
        Key::Named(NamedKey::ArrowUp),
        Modifiers::empty(),
        kitty_all_flags(),
    );
    assert_eq!(kitty, b"\x1b[1A");
}

#[test]
fn kitty_overrides_legacy_for_enter() {
    // Legacy would produce \r; Kitty disambiguate produces ESC[13u.
    let legacy = enc(Key::Named(NamedKey::Enter), Modifiers::empty(), no_mode());
    let kitty = enc(
        Key::Named(NamedKey::Enter),
        Modifiers::empty(),
        kitty_disambiguate(),
    );
    assert_eq!(legacy, b"\r");
    assert_eq!(kitty, b"\x1b[13u");
}

// --- Kitty: Shift+Enter, Shift+Backspace ---

#[test]
fn kitty_shift_enter() {
    let r = enc(
        Key::Named(NamedKey::Enter),
        Modifiers::SHIFT,
        kitty_disambiguate(),
    );
    assert_eq!(r, b"\x1b[13;2u");
}

#[test]
fn kitty_shift_backspace() {
    let r = enc(
        Key::Named(NamedKey::Backspace),
        Modifiers::SHIFT,
        kitty_disambiguate(),
    );
    assert_eq!(r, b"\x1b[127;2u");
}

// --- Kitty: space key ---

#[test]
fn kitty_space() {
    let r = enc(
        Key::Named(NamedKey::Space),
        Modifiers::empty(),
        kitty_disambiguate(),
    );
    assert_eq!(r, b"\x1b[32u");
}

#[test]
fn kitty_ctrl_space() {
    let r = enc(
        Key::Named(NamedKey::Space),
        Modifiers::CONTROL,
        kitty_disambiguate(),
    );
    assert_eq!(r, b"\x1b[32;5u");
}

// --- Kitty: multi-modifier named keys ---

#[test]
fn kitty_ctrl_shift_arrow_up() {
    // ArrowUp with modifiers has an unambiguous legacy sequence (CSI 1;6 A),
    // so DISAMBIGUATE_ESC_CODES alone uses legacy encoding.
    let r = enc(
        Key::Named(NamedKey::ArrowUp),
        Modifiers::CONTROL | Modifiers::SHIFT,
        kitty_disambiguate(),
    );
    assert_eq!(r, b"\x1b[1;6A");
}

#[test]
fn kitty_ctrl_shift_arrow_up_report_all() {
    // With REPORT_ALL_KEYS_AS_ESC, modified ArrowUp uses legacy terminator `A`.
    let r = enc(
        Key::Named(NamedKey::ArrowUp),
        Modifiers::CONTROL | Modifiers::SHIFT,
        kitty_all_flags(),
    );
    // Ctrl=4, Shift=1, param = 1 + 4 + 1 = 6. Base=1 for letter keys.
    assert_eq!(r, b"\x1b[1;6A");
}

#[test]
fn kitty_alt_ctrl_a() {
    let r = enc(
        Key::Character("a".into()),
        Modifiers::ALT | Modifiers::CONTROL,
        kitty_disambiguate(),
    );
    // Alt=2, Ctrl=4, param = 1 + 2 + 4 = 7.
    assert_eq!(r, b"\x1b[97;7u");
}

// --- Kitty: multi-char text passthrough ---

#[test]
fn kitty_multi_char_text_passthrough() {
    // Kitty: multi-char Character key → send as text (can't encode as single codepoint).
    let r = enc_text(
        Key::Character("ñ".into()),
        Modifiers::empty(),
        kitty_disambiguate(),
        "ñ",
    );
    // Single codepoint ñ (U+00F1) → plain text passthrough in disambiguate mode.
    assert_eq!(r, "ñ".as_bytes());
}

#[test]
fn kitty_true_multi_char_sends_text() {
    // Two-char string that can't be a single codepoint → text passthrough.
    let r = enc_text(
        Key::Character("ae".into()),
        Modifiers::empty(),
        kitty_disambiguate(),
        "ae",
    );
    assert_eq!(r, b"ae");
}

// --- Kitty: associated text (REPORT_ASSOCIATED_TEXT) ---

#[test]
fn kitty_text_plain_char() {
    // 'a' with associated text → CSI u with text codepoint.
    let r = enc_text(
        Key::Character("a".into()),
        Modifiers::empty(),
        kitty_report_text(),
        "a",
    );
    assert_eq!(r, b"\x1b[97;1;97u");
}

#[test]
fn kitty_text_shift_a() {
    // Shift+a produces 'A' (codepoint 65).
    let r = enc_text(
        Key::Character("a".into()),
        Modifiers::SHIFT,
        kitty_report_text(),
        "A",
    );
    assert_eq!(r, b"\x1b[97;2;65u");
}

#[test]
fn kitty_text_ctrl_a_no_text() {
    // Ctrl+a produces control code → text filtered out, no text suffix.
    let r = enc_text(
        Key::Character("a".into()),
        Modifiers::CONTROL,
        kitty_report_text(),
        "\x01",
    );
    assert_eq!(r, b"\x1b[97;5u");
}

#[test]
fn kitty_text_named_key_no_text() {
    // Named keys (Enter) have no associated text.
    let r = enc(
        Key::Named(NamedKey::Enter),
        Modifiers::empty(),
        kitty_report_text(),
    );
    assert_eq!(r, b"\x1b[13u");
}

#[test]
fn kitty_text_release_no_text() {
    // Release events never include associated text.
    let r = enc_event(
        Key::Character("a".into()),
        Modifiers::empty(),
        kitty_report_text_events(),
        Some("a"),
        KeyEventType::Release,
    );
    assert_eq!(r, b"\x1b[97;1:3u");
}

#[test]
fn kitty_text_repeat_includes_text() {
    // Repeat events include associated text.
    let r = enc_event(
        Key::Character("a".into()),
        Modifiers::empty(),
        kitty_report_text_events(),
        Some("a"),
        KeyEventType::Repeat,
    );
    assert_eq!(r, b"\x1b[97;1:2;97u");
}

#[test]
fn kitty_text_multi_codepoint() {
    // Multi-codepoint text uses colon separators.
    let r = enc_text(
        Key::Character("a".into()),
        Modifiers::empty(),
        kitty_report_text(),
        "ab",
    );
    assert_eq!(r, b"\x1b[97;1;97:98u");
}

#[test]
fn kitty_text_filters_control_chars() {
    // Control characters in text are filtered; remaining chars kept.
    let r = enc_text(
        Key::Character("a".into()),
        Modifiers::empty(),
        kitty_report_text(),
        "a\nb",
    );
    // \n (0x0A) filtered out, leaves 'a' (97) and 'b' (98).
    assert_eq!(r, b"\x1b[97;1;97:98u");
}

#[test]
fn kitty_text_all_control_no_text_suffix() {
    // If all text chars are control codes, no text suffix emitted.
    let r = enc_text(
        Key::Character("a".into()),
        Modifiers::empty(),
        kitty_report_text(),
        "\x01\x02",
    );
    assert_eq!(r, b"\x1b[97u");
}

#[test]
fn kitty_text_non_ascii() {
    // Non-ASCII codepoint (e.g. U+00E5 = 229 = 'å').
    let r = enc_text(
        Key::Character("a".into()),
        Modifiers::empty(),
        kitty_report_text(),
        "\u{00E5}",
    );
    assert_eq!(r, b"\x1b[97;1;229u");
}

#[test]
fn kitty_text_without_flag_no_text() {
    // Without REPORT_ASSOCIATED_TEXT, text is not included even if present.
    let r = enc_text(
        Key::Character("a".into()),
        Modifiers::empty(),
        kitty_report_all(),
        "a",
    );
    assert_eq!(r, b"\x1b[97u");
}

#[test]
fn kitty_text_bypasses_plain_text_passthrough() {
    // With REPORT_ASSOCIATED_TEXT, plain printable chars still get CSI u encoding.
    let r = enc_text(
        Key::Character("a".into()),
        Modifiers::empty(),
        TermMode::default() | TermMode::DISAMBIGUATE_ESC_CODES | TermMode::REPORT_ASSOCIATED_TEXT,
        "a",
    );
    assert_eq!(r, b"\x1b[97;1;97u");
}

// --- Kitty: release gating by mode flags ---

#[test]
fn kitty_named_key_release_with_report_events_only() {
    // REPORT_EVENT_TYPES without REPORT_ALL — named key release should still be sent.
    let r = enc_event(
        Key::Named(NamedKey::Enter),
        Modifiers::empty(),
        kitty_report_events(),
        None,
        KeyEventType::Release,
    );
    assert_eq!(r, b"\x1b[13;1:3u");
}

#[test]
fn kitty_named_key_release_disambiguate_only() {
    // DISAMBIGUATE alone — release should be suppressed (no REPORT_EVENT_TYPES).
    let r = enc_event(
        Key::Named(NamedKey::Enter),
        Modifiers::empty(),
        kitty_disambiguate(),
        None,
        KeyEventType::Release,
    );
    assert!(r.is_empty());
}

// --- Kitty: bare modifiers with REPORT_ALL ---

#[test]
fn kitty_bare_shift_report_all_produces_nothing() {
    // Bare modifier keys are not in our kitty_codepoint table, so they produce nothing.
    let r = enc(
        Key::Named(NamedKey::Shift),
        Modifiers::SHIFT,
        kitty_report_all(),
    );
    assert!(r.is_empty());
}

#[test]
fn kitty_bare_control_report_all_produces_nothing() {
    let r = enc(
        Key::Named(NamedKey::Control),
        Modifiers::CONTROL,
        kitty_report_all(),
    );
    assert!(r.is_empty());
}

// --- Kitty: all flags combined ---

#[test]
fn kitty_enter_all_flags_press() {
    // All 5 mode bits active — Enter press with no text.
    let r = enc(
        Key::Named(NamedKey::Enter),
        Modifiers::empty(),
        kitty_all_flags(),
    );
    assert_eq!(r, b"\x1b[13u");
}

#[test]
fn kitty_enter_all_flags_release() {
    // All flags — release event includes event type suffix.
    let r = enc_event(
        Key::Named(NamedKey::Enter),
        Modifiers::empty(),
        kitty_all_flags(),
        None,
        KeyEventType::Release,
    );
    assert_eq!(r, b"\x1b[13;1:3u");
}

#[test]
fn kitty_char_all_flags_press_with_text() {
    // All flags — 'a' press includes associated text.
    let r = enc_text(
        Key::Character("a".into()),
        Modifiers::empty(),
        kitty_all_flags(),
        "a",
    );
    assert_eq!(r, b"\x1b[97;1;97u");
}

// --- Kitty: associated text edge cases ---

#[test]
fn kitty_text_del_filtered() {
    // DEL (0x7F) is filtered from associated text.
    let r = enc_text(
        Key::Character("a".into()),
        Modifiers::empty(),
        kitty_report_text(),
        "a\x7Fb",
    );
    assert_eq!(r, b"\x1b[97;1;97:98u");
}

#[test]
fn kitty_text_c1_control_filtered() {
    // C1 control characters (0x80-0x9F) are filtered from associated text.
    let r = enc_text(
        Key::Character("a".into()),
        Modifiers::empty(),
        kitty_report_text(),
        "a\u{0085}b",
    );
    // U+0085 (NEL) is in C1 range, filtered out.
    assert_eq!(r, b"\x1b[97;1;97:98u");
}

#[test]
fn kitty_text_space_key_with_text() {
    // Space (codepoint 32) with REPORT_ASSOCIATED_TEXT includes text suffix.
    let r = enc_event(
        Key::Named(NamedKey::Space),
        Modifiers::empty(),
        kitty_report_text(),
        Some(" "),
        KeyEventType::Press,
    );
    assert_eq!(r, b"\x1b[32;1;32u");
}

#[test]
fn kitty_text_ctrl_shift_letter() {
    // Ctrl+Shift+A: key codepoint 97, modifier 6 (Ctrl=4 + Shift=1 + 1), text 'A' (65).
    let r = enc_text(
        Key::Character("a".into()),
        Modifiers::CONTROL | Modifiers::SHIFT,
        kitty_report_text(),
        "A",
    );
    assert_eq!(r, b"\x1b[97;6;65u");
}

#[test]
fn kitty_text_emoji_codepoint() {
    // High codepoint emoji in text field.
    let r = enc_text(
        Key::Character("a".into()),
        Modifiers::empty(),
        kitty_report_text(),
        "\u{1F600}",
    );
    // U+1F600 = 128512
    assert_eq!(r, b"\x1b[97;1;128512u");
}

// --- Kitty: repeat event for named keys ---

#[test]
fn kitty_named_key_repeat() {
    // F1 repeat with REPORT_EVENT_TYPES — legacy terminator `P`, event suffix :2.
    let r = enc_event(
        Key::Named(NamedKey::F1),
        Modifiers::empty(),
        kitty_report_events(),
        None,
        KeyEventType::Repeat,
    );
    assert_eq!(r, b"\x1b[1;1:2P");
}

#[test]
fn kitty_arrow_repeat() {
    // Arrow key repeat — legacy terminator `A`, event suffix :2.
    let r = enc_event(
        Key::Named(NamedKey::ArrowUp),
        Modifiers::empty(),
        kitty_report_events(),
        None,
        KeyEventType::Repeat,
    );
    assert_eq!(r, b"\x1b[1;1:2A");
}

// --- Kitty: legacy tilde terminators with report_all ---

#[test]
fn kitty_insert_report_all_uses_tilde() {
    let r = enc(
        Key::Named(NamedKey::Insert),
        Modifiers::empty(),
        kitty_all_flags(),
    );
    // Insert: base=2, terminator=~.
    assert_eq!(r, b"\x1b[2~");
}

#[test]
fn kitty_f5_report_all_uses_tilde() {
    let r = enc(
        Key::Named(NamedKey::F5),
        Modifiers::empty(),
        kitty_all_flags(),
    );
    // F5: base=15, terminator=~.
    assert_eq!(r, b"\x1b[15~");
}

#[test]
fn kitty_delete_with_mods_report_all() {
    let r = enc(
        Key::Named(NamedKey::Delete),
        Modifiers::CONTROL,
        kitty_all_flags(),
    );
    // Delete: base=3, Ctrl mod=5, terminator=~.
    assert_eq!(r, b"\x1b[3;5~");
}

// --- Kitty: REPORT_ALTERNATE_KEYS ---

#[test]
fn kitty_alternate_key_included_when_different() {
    // When REPORT_ALTERNATE_KEYS + REPORT_ALL are active and the alternate
    // key differs from the logical key, it appears as `base::alternate`.
    let mode = kitty_report_all() | TermMode::REPORT_ALTERNATE_KEYS;
    let r = encode_key(&KeyInput {
        key: &Key::Character("z".into()),
        mods: Modifiers::empty(),
        mode,
        text: Some("z"),
        location: KeyLocation::Standard,
        event_type: KeyEventType::Press,
        alternate_key: Some(b'y' as u32), // e.g., German QWERTZ layout.
    });
    // base=122 (z), alternate=121 (y) → ESC[122::121;1u.
    assert_eq!(r, b"\x1b[122::121;1u");
}

#[test]
fn kitty_alternate_key_omitted_when_same() {
    // When alternate matches logical, no ::alternate suffix.
    let mode = kitty_disambiguate() | TermMode::REPORT_ALTERNATE_KEYS;
    let r = encode_key(&KeyInput {
        key: &Key::Character("a".into()),
        mods: Modifiers::empty(),
        mode,
        text: Some("a"),
        location: KeyLocation::Standard,
        event_type: KeyEventType::Press,
        alternate_key: None, // Same as logical (filtered at call site).
    });
    // Plain text — printable, no mods, no alternate.
    assert_eq!(r, b"a");
}

#[test]
fn kitty_alternate_key_not_reported_without_flag() {
    // Without REPORT_ALTERNATE_KEYS flag, alternate_key is ignored.
    let r = encode_key(&KeyInput {
        key: &Key::Character("z".into()),
        mods: Modifiers::CONTROL,
        mode: kitty_disambiguate(),
        text: None,
        location: KeyLocation::Standard,
        event_type: KeyEventType::Press,
        alternate_key: Some(b'y' as u32),
    });
    // Ctrl+z = codepoint 122, mod 5. No alternate in output.
    assert_eq!(r, b"\x1b[122;5u");
}

// --- BUG-08-012 semantic pin: post-crash restore produces plain ASCII ---

/// Regression: BUG-08-012 — headline user-visible symptom pin.
///
/// After a kitty-aware child program crashes mid-command and the shell
/// emits its next OSC 133 ; A prompt, the `keyboard_mode_stack` must be
/// restored to its pre-command snapshot. The `TermMode::KITTY_KEYBOARD_PROTOCOL`
/// bits clear; `encode_key` takes the legacy branch; typing 'a' at the
/// shell prompt produces plain ASCII `b"a"`, NOT the raw CSI u fragments
/// (`0;1;100u7;1;97u`) that the shell doesn't understand.
///
/// See: bug-tracker/plans/completed/BUG-08-012/00-overview.md §2 (Semantic pin).
#[test]
fn legacy_key_encoding_after_child_crash_produces_raw_ascii_not_csi_u() {
    use oriterm_core::effect::VoidEffectSink;
    use oriterm_core::{Term, Theme};

    let mut term = Term::new(24, 80, 100, Theme::default(), VoidEffectSink);

    // Shell emits OSC 133 ; C — command-boundary snapshot.
    term.snapshot_keyboard_mode_stack();

    // Child pushes a kitty keyboard mode via `CSI > 1 u` (high-level processor).
    let mut processor = vte::ansi::Processor::<vte::ansi::StdSyncHandler>::new();
    processor.advance(&mut term, b"\x1b[>1u");
    assert!(
        term.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES),
        "setup: kitty mode is active during the command"
    );

    // Child crashes. Shell emits OSC 133 ; A — restore.
    term.restore_keyboard_mode_stack();

    assert!(
        !term.mode().intersects(TermMode::KITTY_KEYBOARD_PROTOCOL),
        "KITTY bits must be cleared after OSC 133 ; A restore"
    );

    // Typing 'a' at the shell prompt goes through the legacy branch.
    let r = enc_text(
        Key::Character("a".into()),
        Modifiers::empty(),
        term.mode(),
        "a",
    );
    assert_eq!(
        r, b"a",
        "post-crash restore: typing 'a' must produce plain ASCII, not a CSI u kitty fragment"
    );
}

// BUG-08-026: Kitty numpad disambiguation — codepoints 57399-57426.

/// Encode helper with `KeyLocation::Numpad`.
fn enc_numpad(key: Key, mode: TermMode) -> Vec<u8> {
    encode_key(&KeyInput {
        key: &key,
        mods: Modifiers::empty(),
        mode,
        text: None,
        location: KeyLocation::Numpad,
        event_type: KeyEventType::Press,
        alternate_key: None,
    })
}

/// BUG-08-026: numpad 1 with kitty active emits CSI 57400 u, NOT `b"1"`.
#[test]
fn kitty_numpad_one_emits_codepoint_57400() {
    let r = enc_numpad(Key::Character("1".into()), kitty_disambiguate());
    assert_eq!(
        r, b"\x1b[57400u",
        "numpad 1 MUST emit CSI 57400 u — got {r:?}"
    );
}

/// BUG-08-026: numpad 0 → 57399 (low end of the codepoint range).
#[test]
fn kitty_numpad_zero_emits_codepoint_57399() {
    let r = enc_numpad(Key::Character("0".into()), kitty_disambiguate());
    assert_eq!(r, b"\x1b[57399u");
}

/// BUG-08-026: numpad Enter → 57414 (transition between digits and arrows).
#[test]
fn kitty_numpad_enter_emits_codepoint_57414() {
    let r = enc_numpad(Key::Named(NamedKey::Enter), kitty_disambiguate());
    assert_eq!(r, b"\x1b[57414u");
}

/// BUG-08-026: numpad ArrowLeft → 57417 (start of named-key range).
#[test]
fn kitty_numpad_arrow_left_emits_codepoint_57417() {
    let r = enc_numpad(Key::Named(NamedKey::ArrowLeft), kitty_disambiguate());
    assert_eq!(r, b"\x1b[57417u");
}

/// BUG-08-026: numpad Delete → 57426 (high end of the codepoint range).
#[test]
fn kitty_numpad_delete_emits_codepoint_57426() {
    let r = enc_numpad(Key::Named(NamedKey::Delete), kitty_disambiguate());
    assert_eq!(r, b"\x1b[57426u");
}

/// BUG-08-026 negative pin: same key on Standard location MUST NOT use the
/// numpad codepoint range. Distinguishes the BUG-08-026 fix from a blanket
/// disambiguation that would also affect main-row digits.
#[test]
fn kitty_main_row_one_does_not_use_numpad_codepoint() {
    let r = encode_key(&KeyInput {
        key: &Key::Character("1".into()),
        mods: Modifiers::empty(),
        mode: kitty_disambiguate(),
        text: Some("1"),
        location: KeyLocation::Standard,
        event_type: KeyEventType::Press,
        alternate_key: None,
    });
    assert_eq!(
        r, b"1",
        "main-row 1 MUST emit `b\"1\"` (legacy text), NOT 57400 — got {r:?}",
    );
}
