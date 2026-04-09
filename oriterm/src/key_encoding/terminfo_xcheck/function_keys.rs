//! Function key terminfo cross-check tests (kf1-kf63).

use oriterm_core::TermMode;
use winit::keyboard::NamedKey;

use super::{CapMapping, Modifiers, run_cap_mapping_test};

/// F1-F12, no modifiers, normal mode.
static F_KEYS_BASE: &[CapMapping] = &[
    CapMapping {
        cap: "kf1",
        named: NamedKey::F1,
        mods: Modifiers::empty(),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf2",
        named: NamedKey::F2,
        mods: Modifiers::empty(),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf3",
        named: NamedKey::F3,
        mods: Modifiers::empty(),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf4",
        named: NamedKey::F4,
        mods: Modifiers::empty(),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf5",
        named: NamedKey::F5,
        mods: Modifiers::empty(),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf6",
        named: NamedKey::F6,
        mods: Modifiers::empty(),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf7",
        named: NamedKey::F7,
        mods: Modifiers::empty(),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf8",
        named: NamedKey::F8,
        mods: Modifiers::empty(),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf9",
        named: NamedKey::F9,
        mods: Modifiers::empty(),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf10",
        named: NamedKey::F10,
        mods: Modifiers::empty(),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf11",
        named: NamedKey::F11,
        mods: Modifiers::empty(),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf12",
        named: NamedKey::F12,
        mods: Modifiers::empty(),
        term_mode: TermMode::empty(),
    },
];

/// kf13-kf24 = Shift+F1-F12 (xterm convention).
static F_KEYS_SHIFTED: &[CapMapping] = &[
    CapMapping {
        cap: "kf13",
        named: NamedKey::F1,
        mods: Modifiers::SHIFT,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf14",
        named: NamedKey::F2,
        mods: Modifiers::SHIFT,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf15",
        named: NamedKey::F3,
        mods: Modifiers::SHIFT,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf16",
        named: NamedKey::F4,
        mods: Modifiers::SHIFT,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf17",
        named: NamedKey::F5,
        mods: Modifiers::SHIFT,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf18",
        named: NamedKey::F6,
        mods: Modifiers::SHIFT,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf19",
        named: NamedKey::F7,
        mods: Modifiers::SHIFT,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf20",
        named: NamedKey::F8,
        mods: Modifiers::SHIFT,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf21",
        named: NamedKey::F9,
        mods: Modifiers::SHIFT,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf22",
        named: NamedKey::F10,
        mods: Modifiers::SHIFT,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf23",
        named: NamedKey::F11,
        mods: Modifiers::SHIFT,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf24",
        named: NamedKey::F12,
        mods: Modifiers::SHIFT,
        term_mode: TermMode::empty(),
    },
];

/// kf25-kf36 = Ctrl+F1-F12 (xterm convention).
static F_KEYS_CTRL: &[CapMapping] = &[
    CapMapping {
        cap: "kf25",
        named: NamedKey::F1,
        mods: Modifiers::CONTROL,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf26",
        named: NamedKey::F2,
        mods: Modifiers::CONTROL,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf27",
        named: NamedKey::F3,
        mods: Modifiers::CONTROL,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf28",
        named: NamedKey::F4,
        mods: Modifiers::CONTROL,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf29",
        named: NamedKey::F5,
        mods: Modifiers::CONTROL,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf30",
        named: NamedKey::F6,
        mods: Modifiers::CONTROL,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf31",
        named: NamedKey::F7,
        mods: Modifiers::CONTROL,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf32",
        named: NamedKey::F8,
        mods: Modifiers::CONTROL,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf33",
        named: NamedKey::F9,
        mods: Modifiers::CONTROL,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf34",
        named: NamedKey::F10,
        mods: Modifiers::CONTROL,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf35",
        named: NamedKey::F11,
        mods: Modifiers::CONTROL,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf36",
        named: NamedKey::F12,
        mods: Modifiers::CONTROL,
        term_mode: TermMode::empty(),
    },
];

/// kf37-kf48 = Ctrl+Shift+F1-F12 (xterm convention).
static F_KEYS_CTRL_SHIFT: &[CapMapping] = &[
    CapMapping {
        cap: "kf37",
        named: NamedKey::F1,
        mods: Modifiers::CONTROL.union(Modifiers::SHIFT),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf38",
        named: NamedKey::F2,
        mods: Modifiers::CONTROL.union(Modifiers::SHIFT),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf39",
        named: NamedKey::F3,
        mods: Modifiers::CONTROL.union(Modifiers::SHIFT),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf40",
        named: NamedKey::F4,
        mods: Modifiers::CONTROL.union(Modifiers::SHIFT),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf41",
        named: NamedKey::F5,
        mods: Modifiers::CONTROL.union(Modifiers::SHIFT),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf42",
        named: NamedKey::F6,
        mods: Modifiers::CONTROL.union(Modifiers::SHIFT),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf43",
        named: NamedKey::F7,
        mods: Modifiers::CONTROL.union(Modifiers::SHIFT),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf44",
        named: NamedKey::F8,
        mods: Modifiers::CONTROL.union(Modifiers::SHIFT),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf45",
        named: NamedKey::F9,
        mods: Modifiers::CONTROL.union(Modifiers::SHIFT),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf46",
        named: NamedKey::F10,
        mods: Modifiers::CONTROL.union(Modifiers::SHIFT),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf47",
        named: NamedKey::F11,
        mods: Modifiers::CONTROL.union(Modifiers::SHIFT),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf48",
        named: NamedKey::F12,
        mods: Modifiers::CONTROL.union(Modifiers::SHIFT),
        term_mode: TermMode::empty(),
    },
];

/// kf49-kf60 = Alt+F1-F12 (xterm convention).
static F_KEYS_ALT: &[CapMapping] = &[
    CapMapping {
        cap: "kf49",
        named: NamedKey::F1,
        mods: Modifiers::ALT,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf50",
        named: NamedKey::F2,
        mods: Modifiers::ALT,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf51",
        named: NamedKey::F3,
        mods: Modifiers::ALT,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf52",
        named: NamedKey::F4,
        mods: Modifiers::ALT,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf53",
        named: NamedKey::F5,
        mods: Modifiers::ALT,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf54",
        named: NamedKey::F6,
        mods: Modifiers::ALT,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf55",
        named: NamedKey::F7,
        mods: Modifiers::ALT,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf56",
        named: NamedKey::F8,
        mods: Modifiers::ALT,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf57",
        named: NamedKey::F9,
        mods: Modifiers::ALT,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf58",
        named: NamedKey::F10,
        mods: Modifiers::ALT,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf59",
        named: NamedKey::F11,
        mods: Modifiers::ALT,
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf60",
        named: NamedKey::F12,
        mods: Modifiers::ALT,
        term_mode: TermMode::empty(),
    },
];

/// kf61-kf63 = Alt+Shift+F1-F3 (xterm convention; ncurses kfN namespace
/// truncates at kf63, so Alt+Shift coverage stops at F3).
static F_KEYS_ALT_SHIFT: &[CapMapping] = &[
    CapMapping {
        cap: "kf61",
        named: NamedKey::F1,
        mods: Modifiers::ALT.union(Modifiers::SHIFT),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf62",
        named: NamedKey::F2,
        mods: Modifiers::ALT.union(Modifiers::SHIFT),
        term_mode: TermMode::empty(),
    },
    CapMapping {
        cap: "kf63",
        named: NamedKey::F3,
        mods: Modifiers::ALT.union(Modifiers::SHIFT),
        term_mode: TermMode::empty(),
    },
];

#[test]
fn function_keys_match_terminfo() {
    run_cap_mapping_test(F_KEYS_BASE);
}

#[test]
fn function_keys_shift_match_terminfo() {
    run_cap_mapping_test(F_KEYS_SHIFTED);
}

#[test]
fn function_keys_ctrl_match_terminfo() {
    run_cap_mapping_test(F_KEYS_CTRL);
}

#[test]
fn function_keys_ctrl_shift_match_terminfo() {
    run_cap_mapping_test(F_KEYS_CTRL_SHIFT);
}

#[test]
fn function_keys_alt_match_terminfo() {
    run_cap_mapping_test(F_KEYS_ALT);
}

#[test]
fn function_keys_alt_shift_match_terminfo() {
    run_cap_mapping_test(F_KEYS_ALT_SHIFT);
}
