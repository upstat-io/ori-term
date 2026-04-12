//! Cross-check ori_term's key_encoding pipeline against the pinned
//! terminfo entry. For each function/cursor/editing cap declared in
//! `extra/ori_term.info`, this test asserts that the bytes
//! `super::encode_key` would emit exactly match what the terminfo
//! says they should be.
//!
//! The point is to catch silent divergences where the terminfo claims
//! `kf1=\EOP` but the encoder emits `\E[11~` (or vice versa). Vim,
//! less, htop and every ncurses-aware tool look up key sequences in
//! terminfo — if the encoder doesn't match, F1 silently breaks for
//! the user.
//!
//! Skips when `tic` or `infocmp` are unavailable (Windows native) —
//! runtime gate, never a `cfg(unix)` block (per CLAUDE.md
//! cross-platform rule: compile everywhere, runtime skip).

mod function_keys;
mod modified_keys;
mod navigation;

use std::collections::HashMap;

use oriterm_core::TermMode;
use oriterm_test_support::{
    TerminfoEnv, decode_terminfo_string, infocmp_available, infocmp_dump, tic_available,
};
use winit::keyboard::{Key, KeyLocation, NamedKey};

use super::{KeyEventType, KeyInput, Modifiers, encode_key};

/// One terminfo cap → key mapping. Fields are plain data so the
/// table can be `static`; the test loop constructs the borrowing
/// `KeyInput` per-iteration from owning `Key::Named(...)` locals.
#[derive(Copy, Clone)]
struct CapMapping {
    /// Terminfo cap name (e.g., `kf1`, `kcub1`).
    cap: &'static str,
    /// Named key (F1-F24, ArrowLeft, Home, etc.).
    named: NamedKey,
    /// Modifier bitset.
    mods: Modifiers,
    /// Terminal mode bits to set when encoding this key. Use
    /// `TermMode::APP_CURSOR` for cursor/Home/End keys in application
    /// mode, `TermMode::APP_KEYPAD` for numpad keys, or
    /// `TermMode::empty()` for normal mode.
    term_mode: TermMode,
}

/// Assert that encode_key output matches the terminfo declaration.
///
/// Accepts a pre-parsed infocmp dump (`HashMap`) so each test
/// function invokes infocmp exactly ONCE rather than once per cap.
/// Panics on missing cap — for a pinned SSOT terminfo, a missing
/// cap is a test infrastructure bug, not a skip reason.
fn assert_encoded_matches_terminfo(caps: &HashMap<String, String>, mapping: &CapMapping) {
    let cap_str = caps.get(mapping.cap).unwrap_or_else(|| {
        panic!(
            "SSOT violation: cap '{}' not found in infocmp dump of pinned ori_term \
             terminfo. Every cap in the test table MUST be declared in \
             extra/ori_term.info. If this cap was intentionally removed, \
             remove it from the test table too.",
            mapping.cap,
        )
    });
    let expected = decode_terminfo_string(cap_str);

    // Own the Key on the stack so the &'a Key borrow in KeyInput is valid.
    let key = Key::Named(mapping.named);

    let input = KeyInput {
        key: &key,
        mods: mapping.mods,
        mode: mapping.term_mode,
        text: None,
        location: KeyLocation::Standard,
        event_type: KeyEventType::Press,
        alternate_key: None,
    };
    let actual = encode_key(&input);

    assert_eq!(
        actual, expected,
        "key_encoding::encode_key for cap {} produced {:?} but terminfo says {:?}",
        mapping.cap, actual, expected,
    );
}

/// Set up the pinned terminfo environment and dump caps.
///
/// Returns `None` when `tic` or `infocmp` are unavailable (runtime
/// skip on Windows native). Callers should early-return on `None`.
fn setup_terminfo_env() -> Option<HashMap<String, String>> {
    if !tic_available() || !infocmp_available() {
        return None;
    }
    let env = TerminfoEnv::compile();
    Some(infocmp_dump(&env, "ori_term").expect("infocmp dump"))
}

/// Run a `CapMapping` array through `assert_encoded_matches_terminfo`,
/// skipping when tic/infocmp are unavailable.
fn run_cap_mapping_test(mappings: &[CapMapping]) {
    let Some(caps) = setup_terminfo_env() else {
        return;
    };
    let mut tested = 0usize;
    for mapping in mappings {
        assert_encoded_matches_terminfo(&caps, mapping);
        tested += 1;
    }
    assert_eq!(
        tested,
        mappings.len(),
        "count pin: expected to test {} caps, only tested {}",
        mappings.len(),
        tested,
    );
}

/// Encode a named key with the given modifiers and terminal mode.
///
/// Convenience helper that constructs a `KeyInput` for a standard
/// `Press` event with no text, no alternate key, and
/// `KeyLocation::Standard`.
fn encode_named_key(named: NamedKey, mods: Modifiers, mode: TermMode) -> Vec<u8> {
    let key = Key::Named(named);
    let input = KeyInput {
        key: &key,
        mods,
        mode,
        text: None,
        location: KeyLocation::Standard,
        event_type: KeyEventType::Press,
        alternate_key: None,
    };
    encode_key(&input)
}

/// Map xterm modifier suffix to Modifiers bitset.
///
/// The base caps (kLFT, kRIT, etc.) without a digit suffix use
/// modifier param 2 = Shift.
fn suffix_to_mods(suffix: Option<u8>) -> Modifiers {
    match suffix {
        None | Some(2) => Modifiers::SHIFT,
        Some(3) => Modifiers::ALT,
        Some(4) => Modifiers::ALT.union(Modifiers::SHIFT),
        Some(5) => Modifiers::CONTROL,
        Some(6) => Modifiers::CONTROL.union(Modifiers::SHIFT),
        Some(7) => Modifiers::CONTROL.union(Modifiers::ALT),
        _ => unreachable!("invalid xterm modifier suffix"),
    }
}

/// Map a modified-key base name to its `NamedKey`.
fn modified_base_to_named(base: &str) -> NamedKey {
    match base {
        "kLFT" => NamedKey::ArrowLeft,
        "kRIT" => NamedKey::ArrowRight,
        "kUP" => NamedKey::ArrowUp,
        "kDN" => NamedKey::ArrowDown,
        "kHOM" => NamedKey::Home,
        "kEND" => NamedKey::End,
        "kIC" => NamedKey::Insert,
        "kDC" => NamedKey::Delete,
        "kNXT" => NamedKey::PageDown,
        "kPRV" => NamedKey::PageUp,
        _ => unreachable!("unknown modified-key base: {base}"),
    }
}
