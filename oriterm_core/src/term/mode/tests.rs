//! Tests for terminal mode flags.

use vte::ansi::KeyboardModes;

use super::TermMode;

#[test]
fn default_has_show_cursor_line_wrap_alternate_scroll_and_cursor_blinking() {
    let mode = TermMode::default();
    assert!(mode.contains(TermMode::SHOW_CURSOR));
    assert!(mode.contains(TermMode::LINE_WRAP));
    assert!(mode.contains(TermMode::ALTERNATE_SCROLL));
    assert!(mode.contains(TermMode::CURSOR_BLINKING));
}

#[test]
fn default_does_not_have_other_modes() {
    let mode = TermMode::default();
    assert!(!mode.contains(TermMode::APP_CURSOR));
    assert!(!mode.contains(TermMode::ALT_SCREEN));
    assert!(!mode.contains(TermMode::BRACKETED_PASTE));
    assert!(!mode.contains(TermMode::MOUSE_REPORT_CLICK));
}

#[test]
fn set_and_clear_individual_modes() {
    let mut mode = TermMode::default();

    mode.insert(TermMode::BRACKETED_PASTE);
    assert!(mode.contains(TermMode::BRACKETED_PASTE));

    mode.remove(TermMode::BRACKETED_PASTE);
    assert!(!mode.contains(TermMode::BRACKETED_PASTE));

    // Original defaults still intact.
    assert!(mode.contains(TermMode::SHOW_CURSOR));
    assert!(mode.contains(TermMode::LINE_WRAP));
}

#[test]
fn any_mouse_is_union_of_mouse_modes() {
    let expected = TermMode::MOUSE_REPORT_CLICK
        | TermMode::MOUSE_DRAG
        | TermMode::MOUSE_MOTION
        | TermMode::MOUSE_X10;
    assert_eq!(TermMode::ANY_MOUSE, expected);
}

#[test]
fn any_mouse_detects_any_single_mouse_mode() {
    let click_only = TermMode::MOUSE_REPORT_CLICK;
    assert!(click_only.intersects(TermMode::ANY_MOUSE));

    let drag_only = TermMode::MOUSE_DRAG;
    assert!(drag_only.intersects(TermMode::ANY_MOUSE));

    let motion_only = TermMode::MOUSE_MOTION;
    assert!(motion_only.intersects(TermMode::ANY_MOUSE));

    let x10_only = TermMode::MOUSE_X10;
    assert!(x10_only.intersects(TermMode::ANY_MOUSE));
}

#[test]
fn empty_mode_has_no_mouse() {
    let mode = TermMode::empty();
    assert!(!mode.intersects(TermMode::ANY_MOUSE));
}

#[test]
fn all_flags_are_distinct() {
    let flags = [
        TermMode::SHOW_CURSOR,
        TermMode::APP_CURSOR,
        TermMode::APP_KEYPAD,
        TermMode::MOUSE_REPORT_CLICK,
        TermMode::MOUSE_DRAG,
        TermMode::MOUSE_MOTION,
        TermMode::MOUSE_SGR,
        TermMode::MOUSE_UTF8,
        TermMode::ALT_SCREEN,
        TermMode::LINE_WRAP,
        TermMode::ORIGIN,
        TermMode::INSERT,
        TermMode::FOCUS_IN_OUT,
        TermMode::BRACKETED_PASTE,
        TermMode::SYNC_UPDATE,
        TermMode::URGENCY_HINTS,
        TermMode::CURSOR_BLINKING,
        TermMode::LINE_FEED_NEW_LINE,
        TermMode::DISAMBIGUATE_ESC_CODES,
        TermMode::REPORT_EVENT_TYPES,
        TermMode::REPORT_ALTERNATE_KEYS,
        TermMode::REPORT_ALL_KEYS_AS_ESC,
        TermMode::REPORT_ASSOCIATED_TEXT,
        TermMode::ALTERNATE_SCROLL,
        TermMode::MOUSE_URXVT,
        TermMode::MOUSE_X10,
        TermMode::SIXEL_SCROLLING,
        TermMode::SIXEL_CURSOR_RIGHT,
        TermMode::REVERSE_VIDEO,
        TermMode::ENABLE_MODE_3,
        TermMode::WIN32_INPUT,
        TermMode::LEFT_RIGHT_MARGIN,
        TermMode::COLOR_SCHEME_UPDATE,
        TermMode::DECBKM,
    ];

    // Each individual flag has exactly one bit set (excluding composite ANY_MOUSE).
    for flag in &flags {
        assert!(
            flag.bits().is_power_of_two(),
            "{flag:?} is not a single bit"
        );
    }
}

#[test]
fn kitty_keyboard_protocol_is_union_of_all_kitty_flags() {
    let expected = TermMode::DISAMBIGUATE_ESC_CODES
        | TermMode::REPORT_EVENT_TYPES
        | TermMode::REPORT_ALTERNATE_KEYS
        | TermMode::REPORT_ALL_KEYS_AS_ESC
        | TermMode::REPORT_ASSOCIATED_TEXT;
    assert_eq!(TermMode::KITTY_KEYBOARD_PROTOCOL, expected);
}

#[test]
fn keyboard_modes_to_term_mode_conversion() {
    let modes = KeyboardModes::DISAMBIGUATE_ESC_CODES | KeyboardModes::REPORT_EVENT_TYPES;
    let term_mode = TermMode::from(modes);

    assert!(term_mode.contains(TermMode::DISAMBIGUATE_ESC_CODES));
    assert!(term_mode.contains(TermMode::REPORT_EVENT_TYPES));
    assert!(!term_mode.contains(TermMode::REPORT_ALTERNATE_KEYS));
    assert!(!term_mode.contains(TermMode::REPORT_ALL_KEYS_AS_ESC));
    assert!(!term_mode.contains(TermMode::REPORT_ASSOCIATED_TEXT));
}

#[test]
fn keyboard_modes_no_mode_converts_to_empty() {
    let term_mode = TermMode::from(KeyboardModes::NO_MODE);
    assert!(!term_mode.intersects(TermMode::KITTY_KEYBOARD_PROTOCOL));
}

#[test]
fn any_mouse_encoding_is_union_of_encoding_modes() {
    let expected = TermMode::MOUSE_SGR
        | TermMode::MOUSE_UTF8
        | TermMode::MOUSE_URXVT
        | TermMode::MOUSE_SGR_PIXEL;
    assert_eq!(TermMode::ANY_MOUSE_ENCODING, expected);
}

#[test]
fn new_flags_are_distinct() {
    assert!(TermMode::REVERSE_WRAP.bits().is_power_of_two());
    assert!(TermMode::MOUSE_URXVT.bits().is_power_of_two());
    assert!(TermMode::MOUSE_X10.bits().is_power_of_two());
    assert!(TermMode::WIN32_INPUT.bits().is_power_of_two());
    assert_ne!(TermMode::REVERSE_WRAP, TermMode::MOUSE_URXVT);
    assert_ne!(TermMode::MOUSE_X10, TermMode::MOUSE_URXVT);
    assert_ne!(TermMode::MOUSE_X10, TermMode::REVERSE_WRAP);
}

#[test]
fn default_does_not_have_new_modes() {
    let mode = TermMode::default();
    assert!(!mode.contains(TermMode::REVERSE_WRAP));
    assert!(!mode.contains(TermMode::MOUSE_URXVT));
    assert!(!mode.contains(TermMode::MOUSE_X10));
    assert!(!mode.contains(TermMode::REVERSE_VIDEO));
    assert!(!mode.contains(TermMode::WIN32_INPUT));
}

#[test]
fn reverse_video_set_and_clear() {
    let mut mode = TermMode::default();
    assert!(!mode.contains(TermMode::REVERSE_VIDEO));

    mode.insert(TermMode::REVERSE_VIDEO);
    assert!(mode.contains(TermMode::REVERSE_VIDEO));

    mode.remove(TermMode::REVERSE_VIDEO);
    assert!(!mode.contains(TermMode::REVERSE_VIDEO));
}

#[test]
fn term_mode_size_is_8_bytes() {
    // Structural regression guard: TermMode is a u64 bitflags.
    // Widened from u32 in plan §08.3
    // to carry DECLRMM (mode 69 = bit 32). If this assertion breaks, a
    // flag overflowed u64 or the representation was changed — both
    // require explicit review.
    assert_eq!(size_of::<TermMode>(), 8, "TermMode should be 8 bytes (u64)");
}

/// Regression: TPR impl-hygiene F6 — the `From<KeyboardModes> for
/// TermMode` and `From<TermMode> for KeyboardModes` impls must be exact
/// inverses over the kitty-protocol subset. A divergence would silently
/// corrupt `snapshot_keyboard_mode_stack` / `restore_keyboard_mode_stack`
/// via the `KeyboardModes::from(self.mode)` capture at snapshot time.
#[test]
fn keyboard_modes_termmode_roundtrip_preserves_all_bits() {
    use vte::ansi::KeyboardModes;

    for bits in 0u8..=0b0001_1111 {
        let km = KeyboardModes::from_bits_truncate(bits);
        let tm = TermMode::from(km);
        let round_trip = KeyboardModes::from(tm);
        assert_eq!(
            km, round_trip,
            "KeyboardModes -> TermMode -> KeyboardModes must round-trip \
 for bits {bits:#b}: got {round_trip:?} from {km:?}"
        );
    }
}

/// Regression: TPR impl-hygiene F6 — non-kitty TermMode bits
/// must NOT appear in `KeyboardModes::from(TermMode)`. The inverse is
/// scoped to the kitty-protocol subset.
#[test]
fn termmode_to_keyboard_modes_masks_non_kitty_bits() {
    use vte::ansi::KeyboardModes;

    let mode = TermMode::default() | TermMode::BRACKETED_PASTE | TermMode::ALT_SCREEN;
    let km = KeyboardModes::from(mode);
    assert_eq!(
        km,
        KeyboardModes::NO_MODE,
        "non-kitty bits must be dropped by the TermMode -> KeyboardModes inverse"
    );
}

// ── encode_enter_base ──

#[test]
fn encode_enter_base_lnm_off_sends_cr() {
    let mode = TermMode::default();
    assert!(!mode.contains(TermMode::LINE_FEED_NEW_LINE));
    assert_eq!(super::encode_enter_base(mode), b"\r");
}

#[test]
fn encode_enter_base_lnm_on_sends_crlf() {
    let mut mode = TermMode::default();
    mode.insert(TermMode::LINE_FEED_NEW_LINE);
    assert_eq!(super::encode_enter_base(mode), b"\r\n");
}

/// LNM on must NOT return bare CR — the fix is to produce CR+LF.
#[test]
fn encode_enter_base_lnm_on_not_bare_cr() {
    let mut mode = TermMode::default();
    mode.insert(TermMode::LINE_FEED_NEW_LINE);
    assert_ne!(super::encode_enter_base(mode), b"\r");
}
