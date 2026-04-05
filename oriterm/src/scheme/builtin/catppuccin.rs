//! Catppuccin color scheme family (Mocha, Latte, Frappe, Macchiato).

#![allow(clippy::unreadable_literal)]

use super::{BuiltinScheme, ansi16, rgb};

pub(super) const CATPPUCCIN_MOCHA: BuiltinScheme = BuiltinScheme {
    name: "Catppuccin Mocha",
    ansi: ansi16([
        0x45475a, 0xf38ba8, 0xa6e3a1, 0xf9e2af, 0x89b4fa, 0xf5c2e7, 0x94e2d5, 0xbac2de, 0x585b70,
        0xf38ba8, 0xa6e3a1, 0xf9e2af, 0x89b4fa, 0xf5c2e7, 0x94e2d5, 0xa6adc8,
    ]),
    fg: rgb(0xcdd6f4),
    bg: rgb(0x1e1e2e),
    cursor: rgb(0xf5e0dc),
};

pub(super) const CATPPUCCIN_LATTE: BuiltinScheme = BuiltinScheme {
    name: "Catppuccin Latte",
    ansi: ansi16([
        0x5c5f77, 0xd20f39, 0x40a02b, 0xdf8e1d, 0x1e66f5, 0xea76cb, 0x179c99, 0xacb0be, 0x6c6f85,
        0xd20f39, 0x40a02b, 0xdf8e1d, 0x1e66f5, 0xea76cb, 0x179c99, 0xbcc0cc,
    ]),
    fg: rgb(0x4c4f69),
    bg: rgb(0xeff1f5),
    cursor: rgb(0xdc8a78),
};

pub(super) const CATPPUCCIN_FRAPPE: BuiltinScheme = BuiltinScheme {
    name: "Catppuccin Frappe",
    ansi: ansi16([
        0x51576d, 0xe78284, 0xa6d189, 0xe5c890, 0x8caaee, 0xf4b8e4, 0x81c8be, 0xb5bfe2, 0x626880,
        0xe78284, 0xa6d189, 0xe5c890, 0x8caaee, 0xf4b8e4, 0x81c8be, 0xa5adce,
    ]),
    fg: rgb(0xc6d0f5),
    bg: rgb(0x303446),
    cursor: rgb(0xf2d5cf),
};

pub(super) const CATPPUCCIN_MACCHIATO: BuiltinScheme = BuiltinScheme {
    name: "Catppuccin Macchiato",
    ansi: ansi16([
        0x494d64, 0xed8796, 0xa6da95, 0xeed49f, 0x8aadf4, 0xf5bde6, 0x8bd5ca, 0xb8c0e0, 0x5b6078,
        0xed8796, 0xa6da95, 0xeed49f, 0x8aadf4, 0xf5bde6, 0x8bd5ca, 0xa5adcb,
    ]),
    fg: rgb(0xcad3f5),
    bg: rgb(0x24273a),
    cursor: rgb(0xf4dbd6),
};
