//! Modified key terminfo cross-check tests (kLFT/kRIT/kUP/kDN/kHOM/kEND/
//! kIC/kDC/kNXT/kPRV with modifier suffixes 3-7, plus kind/kri).

use oriterm_core::TermMode;
use oriterm_test_support::{
    TerminfoEnv, decode_terminfo_string, infocmp_available, infocmp_dump, infocmp_query,
    tic_available,
};
use winit::keyboard::{Key, KeyLocation, NamedKey};

use super::{
    KeyEventType, KeyInput, Modifiers, encode_key, modified_base_to_named, suffix_to_mods,
};

/// Cross-check all 62 modified-key caps against the encoder.
///
/// 10 bases x (1 base + 5 suffixes) + 2 aliases (kind/kri) = 62.
#[test]
fn modified_keys_match_terminfo() {
    if !tic_available() || !infocmp_available() {
        return;
    }
    let env = TerminfoEnv::compile();
    let caps = infocmp_dump(&env, "ori_term").expect("infocmp dump");

    let bases = [
        "kLFT", "kRIT", "kUP", "kDN", "kHOM", "kEND", "kIC", "kDC", "kNXT", "kPRV",
    ];

    let mut tested = 0usize;
    for base in bases {
        // Base cap (e.g., kLFT) = Shift variant (modifier param 2).
        let val = caps.get(base).unwrap_or_else(|| {
            panic!("SSOT violation: modified-key base cap '{base}' not found in infocmp dump")
        });
        let expected = decode_terminfo_string(val);
        let key = Key::Named(modified_base_to_named(base));
        let input = KeyInput {
            key: &key,
            mods: suffix_to_mods(None),
            mode: TermMode::empty(),
            text: None,
            location: KeyLocation::Standard,
            event_type: KeyEventType::Press,
            alternate_key: None,
        };
        let actual = encode_key(&input);
        assert_eq!(
            actual, expected,
            "encode_key for modified cap {base} produced {:?} but terminfo says {:?}",
            actual, expected,
        );
        tested += 1;

        // Suffixed caps (e.g., kLFT3, kLFT4, ..., kLFT7).
        for suffix in 3..=7u8 {
            let cap_name = format!("{base}{suffix}");
            let val = caps.get(cap_name.as_str()).unwrap_or_else(|| {
                panic!("SSOT violation: modified-key cap '{cap_name}' not found in infocmp dump")
            });
            let expected = decode_terminfo_string(val);
            let key = Key::Named(modified_base_to_named(base));
            let input = KeyInput {
                key: &key,
                mods: suffix_to_mods(Some(suffix)),
                mode: TermMode::empty(),
                text: None,
                location: KeyLocation::Standard,
                event_type: KeyEventType::Press,
                alternate_key: None,
            };
            let actual = encode_key(&input);
            assert_eq!(
                actual, expected,
                "encode_key for modified cap {cap_name} produced {:?} but terminfo says {:?}",
                actual, expected,
            );
            tested += 1;
        }
    }

    // Special ncurses aliases: kind = Shift+Down, kri = Shift+Up.
    for (cap, named) in [("kind", NamedKey::ArrowDown), ("kri", NamedKey::ArrowUp)] {
        let val = caps.get(cap).unwrap_or_else(|| {
            panic!("SSOT violation: ncurses alias cap '{cap}' not found in infocmp dump")
        });
        let expected = decode_terminfo_string(val);
        let key = Key::Named(named);
        let input = KeyInput {
            key: &key,
            mods: Modifiers::SHIFT,
            mode: TermMode::empty(),
            text: None,
            location: KeyLocation::Standard,
            event_type: KeyEventType::Press,
            alternate_key: None,
        };
        let actual = encode_key(&input);
        assert_eq!(
            actual, expected,
            "encode_key for modified cap {cap} produced {:?} but terminfo says {:?}",
            actual, expected,
        );
        tested += 1;
    }

    // Count pin: 10 bases * (1 base + 5 suffixes) + 2 aliases = 62.
    assert_eq!(
        tested, 62,
        "count pin: expected to test 62 modified-key caps, only tested {tested}",
    );
}

/// Negative pin: `infocmp_query` returns `None` for a cap not declared
/// in `extra/ori_term.info`. Validates the parsing helper itself.
#[test]
fn infocmp_query_returns_none_for_cap_not_in_ori_term() {
    if !tic_available() || !infocmp_available() {
        return;
    }
    let env = TerminfoEnv::compile();
    // kf64 is outside the xterm kfN namespace and MUST NOT be declared.
    assert!(infocmp_query(&env, "ori_term", "kf64").is_none());
}
