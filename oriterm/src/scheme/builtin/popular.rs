//! Popular color schemes (One Dark/Light, Solarized, Dracula, Gruvbox, Nord, Monokai).

#![allow(clippy::unreadable_literal)]

use super::{BuiltinScheme, ansi16, rgb};

pub(super) const ONE_DARK: BuiltinScheme = BuiltinScheme {
    name: "One Dark",
    ansi: ansi16([
        0x282c34, 0xe06c75, 0x98c379, 0xe5c07b, 0x61afef, 0xc678dd, 0x56b6c2, 0xabb2bf, 0x545862,
        0xe06c75, 0x98c379, 0xe5c07b, 0x61afef, 0xc678dd, 0x56b6c2, 0xbec5d4,
    ]),
    fg: rgb(0xabb2bf),
    bg: rgb(0x282c34),
    cursor: rgb(0x528bff),
};

pub(super) const ONE_LIGHT: BuiltinScheme = BuiltinScheme {
    name: "One Light",
    ansi: ansi16([
        0x383a42, 0xe45649, 0x50a14f, 0xc18401, 0x4078f2, 0xa626a4, 0x0184bc, 0xa0a1a7, 0x4f525e,
        0xe45649, 0x50a14f, 0xc18401, 0x4078f2, 0xa626a4, 0x0184bc, 0xfafafa,
    ]),
    fg: rgb(0x383a42),
    bg: rgb(0xfafafa),
    cursor: rgb(0x526fff),
};

pub(super) const SOLARIZED_DARK: BuiltinScheme = BuiltinScheme {
    name: "Solarized Dark",
    ansi: ansi16([
        0x073642, 0xdc322f, 0x859900, 0xb58900, 0x268bd2, 0xd33682, 0x2aa198, 0xeee8d5, 0x002b36,
        0xcb4b16, 0x586e75, 0x657b83, 0x839496, 0x6c71c4, 0x93a1a1, 0xfdf6e3,
    ]),
    fg: rgb(0x839496),
    bg: rgb(0x002b36),
    cursor: rgb(0x839496),
};

pub(super) const SOLARIZED_LIGHT: BuiltinScheme = BuiltinScheme {
    name: "Solarized Light",
    ansi: ansi16([
        0xeee8d5, 0xdc322f, 0x859900, 0xb58900, 0x268bd2, 0xd33682, 0x2aa198, 0x073642, 0xfdf6e3,
        0xcb4b16, 0x93a1a1, 0x839496, 0x657b83, 0x6c71c4, 0x586e75, 0x002b36,
    ]),
    fg: rgb(0x657b83),
    bg: rgb(0xfdf6e3),
    cursor: rgb(0x657b83),
};

pub(super) const DRACULA: BuiltinScheme = BuiltinScheme {
    name: "Dracula",
    ansi: ansi16([
        0x21222c, 0xff5555, 0x50fa7b, 0xf1fa8c, 0xbd93f9, 0xff79c6, 0x8be9fd, 0xf8f8f2, 0x6272a4,
        0xff6e6e, 0x69ff94, 0xffffa5, 0xd6acff, 0xff92df, 0xa4ffff, 0xffffff,
    ]),
    fg: rgb(0xf8f8f2),
    bg: rgb(0x282a36),
    cursor: rgb(0xf8f8f2),
};

pub(super) const GRUVBOX_DARK: BuiltinScheme = BuiltinScheme {
    name: "Gruvbox Dark",
    ansi: ansi16([
        0x282828, 0xcc241d, 0x98971a, 0xd79921, 0x458588, 0xb16286, 0x689d6a, 0xa89984, 0x928374,
        0xfb4934, 0xb8bb26, 0xfabd2f, 0x83a598, 0xd3869b, 0x8ec07c, 0xebdbb2,
    ]),
    fg: rgb(0xebdbb2),
    bg: rgb(0x282828),
    cursor: rgb(0xebdbb2),
};

pub(super) const GRUVBOX_LIGHT: BuiltinScheme = BuiltinScheme {
    name: "Gruvbox Light",
    ansi: ansi16([
        0xfbf1c7, 0xcc241d, 0x98971a, 0xd79921, 0x458588, 0xb16286, 0x689d6a, 0x7c6f64, 0x928374,
        0x9d0006, 0x79740e, 0xb57614, 0x076678, 0x8f3f71, 0x427b58, 0x3c3836,
    ]),
    fg: rgb(0x3c3836),
    bg: rgb(0xfbf1c7),
    cursor: rgb(0x3c3836),
};

pub(super) const NORD: BuiltinScheme = BuiltinScheme {
    name: "Nord",
    ansi: ansi16([
        0x3b4252, 0xbf616a, 0xa3be8c, 0xebcb8b, 0x81a1c1, 0xb48ead, 0x88c0d0, 0xe5e9f0, 0x4c566a,
        0xbf616a, 0xa3be8c, 0xebcb8b, 0x81a1c1, 0xb48ead, 0x8fbcbb, 0xeceff4,
    ]),
    fg: rgb(0xd8dee9),
    bg: rgb(0x2e3440),
    cursor: rgb(0xd8dee9),
};

pub(super) const MONOKAI: BuiltinScheme = BuiltinScheme {
    name: "Monokai",
    ansi: ansi16([
        0x272822, 0xf92672, 0xa6e22e, 0xf4bf75, 0x66d9ef, 0xae81ff, 0xa1efe4, 0xf8f8f2, 0x75715e,
        0xf92672, 0xa6e22e, 0xf4bf75, 0x66d9ef, 0xae81ff, 0xa1efe4, 0xf9f8f5,
    ]),
    fg: rgb(0xf8f8f2),
    bg: rgb(0x272822),
    cursor: rgb(0xf8f8f0),
};
