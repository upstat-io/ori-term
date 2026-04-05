//! Tokyo Night color scheme family and `WezTerm` default.

#![allow(clippy::unreadable_literal)]

use super::{BuiltinScheme, ansi16, rgb};

pub(super) const TOKYO_NIGHT: BuiltinScheme = BuiltinScheme {
    name: "Tokyo Night",
    ansi: ansi16([
        0x15161e, 0xf7768e, 0x9ece6a, 0xe0af68, 0x7aa2f7, 0xbb9af7, 0x7dcfff, 0xa9b1d6, 0x414868,
        0xf7768e, 0x9ece6a, 0xe0af68, 0x7aa2f7, 0xbb9af7, 0x7dcfff, 0xc0caf5,
    ]),
    fg: rgb(0xa9b1d6),
    bg: rgb(0x1a1b26),
    cursor: rgb(0xc0caf5),
};

pub(super) const TOKYO_NIGHT_STORM: BuiltinScheme = BuiltinScheme {
    name: "Tokyo Night Storm",
    ansi: ansi16([
        0x1d202f, 0xf7768e, 0x9ece6a, 0xe0af68, 0x7aa2f7, 0xbb9af7, 0x7dcfff, 0xa9b1d6, 0x414868,
        0xf7768e, 0x9ece6a, 0xe0af68, 0x7aa2f7, 0xbb9af7, 0x7dcfff, 0xc0caf5,
    ]),
    fg: rgb(0xa9b1d6),
    bg: rgb(0x24283b),
    cursor: rgb(0xc0caf5),
};

pub(super) const TOKYO_NIGHT_LIGHT: BuiltinScheme = BuiltinScheme {
    name: "Tokyo Night Light",
    ansi: ansi16([
        0xe9e9ed, 0xf52a65, 0x587539, 0x8c6c3e, 0x2e7de9, 0x9854f1, 0x007197, 0x6172b0, 0xa1a6c5,
        0xf52a65, 0x587539, 0x8c6c3e, 0x2e7de9, 0x9854f1, 0x007197, 0x3760bf,
    ]),
    fg: rgb(0x3760bf),
    bg: rgb(0xd5d6db),
    cursor: rgb(0x3760bf),
};

pub(super) const WEZTERM_DEFAULT: BuiltinScheme = BuiltinScheme {
    name: "WezTerm Default",
    ansi: ansi16([
        0x000000, 0xcc5555, 0x55cc55, 0xcdcd55, 0x5455cb, 0xcc55cc, 0x7acaca, 0xcccccc, 0x555555,
        0xff5555, 0x55ff55, 0xffff55, 0x5555ff, 0xff55ff, 0x55ffff, 0xffffff,
    ]),
    fg: rgb(0xb2b2b2),
    bg: rgb(0x000000),
    cursor: rgb(0x52ad70),
};
