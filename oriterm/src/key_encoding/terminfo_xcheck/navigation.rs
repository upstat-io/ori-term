//! Cursor key and editing key terminfo cross-check tests.

use oriterm_core::TermMode;
use winit::keyboard::NamedKey;

use super::{CapMapping, Modifiers, encode_named_key, run_cap_mapping_test};

// 08.3: Cursor keys

/// Cursor keys in application (smkx) mode. The terminfo caps
/// (kcub1/kcud1/kcuf1/kcuu1) declare the application-mode form
/// because that's how curses uses them.
static CURSOR_KEYS_APP: &[CapMapping] = &[
    CapMapping {
        cap: "kcub1",
        named: NamedKey::ArrowLeft,
        mods: Modifiers::empty(),
        term_mode: TermMode::APP_CURSOR,
    },
    CapMapping {
        cap: "kcud1",
        named: NamedKey::ArrowDown,
        mods: Modifiers::empty(),
        term_mode: TermMode::APP_CURSOR,
    },
    CapMapping {
        cap: "kcuf1",
        named: NamedKey::ArrowRight,
        mods: Modifiers::empty(),
        term_mode: TermMode::APP_CURSOR,
    },
    CapMapping {
        cap: "kcuu1",
        named: NamedKey::ArrowUp,
        mods: Modifiers::empty(),
        term_mode: TermMode::APP_CURSOR,
    },
];

#[test]
fn cursor_keys_app_mode_match_terminfo() {
    run_cap_mapping_test(CURSOR_KEYS_APP);
}

/// Cursor keys in normal (rmkx) mode — verify encode_key produces
/// the standard CSI sequence directly, not the application form.
/// Pure encoder test — no infocmp/tic dependency, runs on all platforms.
#[test]
fn cursor_keys_normal_mode_emit_csi() {
    let pairs: &[(NamedKey, &[u8])] = &[
        (NamedKey::ArrowUp, b"\x1b[A"),
        (NamedKey::ArrowDown, b"\x1b[B"),
        (NamedKey::ArrowRight, b"\x1b[C"),
        (NamedKey::ArrowLeft, b"\x1b[D"),
    ];
    for (named, expected) in pairs {
        let actual = encode_named_key(*named, Modifiers::empty(), TermMode::empty());
        assert_eq!(
            actual, *expected,
            "{:?} in normal mode produced {:?}, expected {:?}",
            named, actual, expected,
        );
    }
}

// 08.4: Editing/navigation keys

/// Editing keypad keys validated against terminfo. Note: khome/kend
/// use APP_CURSOR because the terminfo declares the application-mode
/// (SS3) form.
static EDITING_KEYS: &[CapMapping] = &[
    CapMapping {
        cap: "kbs",
        named: NamedKey::Backspace,
        mods: Modifiers::empty(),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "khome",
        named: NamedKey::Home,
        mods: Modifiers::empty(),
        term_mode: TermMode::APP_CURSOR,
    },
    CapMapping {
        cap: "kend",
        named: NamedKey::End,
        mods: Modifiers::empty(),
        term_mode: TermMode::APP_CURSOR,
    },
    CapMapping {
        cap: "kpp",
        named: NamedKey::PageUp,
        mods: Modifiers::empty(),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "knp",
        named: NamedKey::PageDown,
        mods: Modifiers::empty(),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kdch1",
        named: NamedKey::Delete,
        mods: Modifiers::empty(),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kich1",
        named: NamedKey::Insert,
        mods: Modifiers::empty(),
        term_mode: TermMode::empty(),
    },
];

#[test]
fn editing_keys_match_terminfo() {
    run_cap_mapping_test(EDITING_KEYS);
}

/// Home/End in normal (non-APP_CURSOR) mode — verify encode_key
/// produces the standard CSI sequence, not the application SS3 form.
/// Pure encoder test — no infocmp/tic dependency, runs on all platforms.
#[test]
fn editing_keys_normal_mode_emit_csi() {
    let pairs: &[(NamedKey, &[u8])] = &[(NamedKey::Home, b"\x1b[H"), (NamedKey::End, b"\x1b[F")];
    for (named, expected) in pairs {
        let actual = encode_named_key(*named, Modifiers::empty(), TermMode::empty());
        assert_eq!(
            actual, *expected,
            "{:?} in normal mode produced {:?}, expected {:?}",
            named, actual, expected,
        );
    }
}
