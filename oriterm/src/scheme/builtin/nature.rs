//! Nature-inspired color schemes (Rose Pine, Everforest, Kanagawa, Ayu).

#![allow(clippy::unreadable_literal)]

use super::{BuiltinScheme, ansi16, rgb};

pub(super) const ROSE_PINE: BuiltinScheme = BuiltinScheme {
    name: "Rose Pine",
    ansi: ansi16([
        0x26233a, 0xeb6f92, 0x31748f, 0xf6c177, 0x9ccfd8, 0xc4a7e7, 0xebbcba, 0xe0def4, 0x6e6a86,
        0xeb6f92, 0x31748f, 0xf6c177, 0x9ccfd8, 0xc4a7e7, 0xebbcba, 0xe0def4,
    ]),
    fg: rgb(0xe0def4),
    bg: rgb(0x191724),
    cursor: rgb(0xe0def4),
};

pub(super) const ROSE_PINE_MOON: BuiltinScheme = BuiltinScheme {
    name: "Rose Pine Moon",
    ansi: ansi16([
        0x393552, 0xeb6f92, 0x3e8fb0, 0xf6c177, 0x9ccfd8, 0xc4a7e7, 0xea9a97, 0xe0def4, 0x6e6a86,
        0xeb6f92, 0x3e8fb0, 0xf6c177, 0x9ccfd8, 0xc4a7e7, 0xea9a97, 0xe0def4,
    ]),
    fg: rgb(0xe0def4),
    bg: rgb(0x232136),
    cursor: rgb(0xe0def4),
};

pub(super) const ROSE_PINE_DAWN: BuiltinScheme = BuiltinScheme {
    name: "Rose Pine Dawn",
    ansi: ansi16([
        0xf2e9e1, 0xb4637a, 0x286983, 0xea9d34, 0x56949f, 0x907aa9, 0xd7827e, 0x575279, 0x9893a5,
        0xb4637a, 0x286983, 0xea9d34, 0x56949f, 0x907aa9, 0xd7827e, 0x575279,
    ]),
    fg: rgb(0x575279),
    bg: rgb(0xfaf4ed),
    cursor: rgb(0x575279),
};

pub(super) const EVERFOREST_DARK: BuiltinScheme = BuiltinScheme {
    name: "Everforest Dark",
    ansi: ansi16([
        0x475258, 0xe67e80, 0xa7c080, 0xdbbc7f, 0x7fbbb3, 0xd699b6, 0x83c092, 0xd3c6aa, 0x7a8478,
        0xe67e80, 0xa7c080, 0xdbbc7f, 0x7fbbb3, 0xd699b6, 0x83c092, 0xd3c6aa,
    ]),
    fg: rgb(0xd3c6aa),
    bg: rgb(0x2d353b),
    cursor: rgb(0xd3c6aa),
};

pub(super) const EVERFOREST_LIGHT: BuiltinScheme = BuiltinScheme {
    name: "Everforest Light",
    ansi: ansi16([
        0xf3ead3, 0xf85552, 0x8da101, 0xdfa000, 0x3a94c5, 0xdf69ba, 0x35a77c, 0x5c6a72, 0x939f91,
        0xf85552, 0x8da101, 0xdfa000, 0x3a94c5, 0xdf69ba, 0x35a77c, 0x5c6a72,
    ]),
    fg: rgb(0x5c6a72),
    bg: rgb(0xfdf6e3),
    cursor: rgb(0x5c6a72),
};

pub(super) const KANAGAWA: BuiltinScheme = BuiltinScheme {
    name: "Kanagawa",
    ansi: ansi16([
        0x16161d, 0xc34043, 0x76946a, 0xc0a36e, 0x7e9cd8, 0x957fb8, 0x6a9589, 0xc8c093, 0x727169,
        0xe82424, 0x98bb6c, 0xe6c384, 0x7fb4ca, 0x938aa9, 0x7aa89f, 0xdcd7ba,
    ]),
    fg: rgb(0xdcd7ba),
    bg: rgb(0x1f1f28),
    cursor: rgb(0xc8c093),
};

pub(super) const KANAGAWA_LIGHT: BuiltinScheme = BuiltinScheme {
    name: "Kanagawa Light",
    ansi: ansi16([
        0xc7c7c7, 0xc84053, 0x6f894e, 0x77713f, 0x4d699b, 0xb35b79, 0x597b75, 0x545464, 0xa6a69c,
        0xe82424, 0x6f894e, 0x77713f, 0x4d699b, 0xb35b79, 0x597b75, 0x1f1f28,
    ]),
    fg: rgb(0x1f1f28),
    bg: rgb(0xf2ecbc),
    cursor: rgb(0x43436c),
};

pub(super) const AYU_DARK: BuiltinScheme = BuiltinScheme {
    name: "Ayu Dark",
    ansi: ansi16([
        0x01060e, 0xea6c73, 0x91b362, 0xf9af4f, 0x53bdfa, 0xfae994, 0x90e1c6, 0xc7c7c7, 0x686868,
        0xf07178, 0xc2d94c, 0xffb454, 0x59c2ff, 0xffee99, 0x95e6cb, 0xffffff,
    ]),
    fg: rgb(0xbfbdb6),
    bg: rgb(0x0d1017),
    cursor: rgb(0xe6b450),
};

pub(super) const AYU_MIRAGE: BuiltinScheme = BuiltinScheme {
    name: "Ayu Mirage",
    ansi: ansi16([
        0x191e2a, 0xed8274, 0xa6cc70, 0xfad07b, 0x6dcbfa, 0xcfbafa, 0x90e1c6, 0xc7c7c7, 0x686868,
        0xf28779, 0xbae67e, 0xffd580, 0x73d0ff, 0xd4bfff, 0x95e6cb, 0xffffff,
    ]),
    fg: rgb(0xcccac2),
    bg: rgb(0x1f2430),
    cursor: rgb(0xffcc66),
};

pub(super) const AYU_LIGHT: BuiltinScheme = BuiltinScheme {
    name: "Ayu Light",
    ansi: ansi16([
        0x000000, 0xff3333, 0x86b300, 0xf29718, 0x41a6d9, 0xf07178, 0x4dbf99, 0xc7c7c7, 0x686868,
        0xe65050, 0x99cc00, 0xe6b673, 0x55b4d4, 0xf27983, 0x5ccfab, 0xffffff,
    ]),
    fg: rgb(0x5c6166),
    bg: rgb(0xfafafa),
    cursor: rgb(0xff6a00),
};
