//! Material Design and fox/GitHub color scheme families.

#![allow(clippy::unreadable_literal)]

use super::{BuiltinScheme, ansi16, rgb};

pub(super) const MATERIAL_DARK: BuiltinScheme = BuiltinScheme {
    name: "Material Dark",
    ansi: ansi16([
        0x546e7a, 0xff5370, 0xc3e88d, 0xffcb6b, 0x82aaff, 0xc792ea, 0x89ddff, 0xeeffff, 0x546e7a,
        0xff5370, 0xc3e88d, 0xffcb6b, 0x82aaff, 0xc792ea, 0x89ddff, 0xeeffff,
    ]),
    fg: rgb(0xeeffff),
    bg: rgb(0x263238),
    cursor: rgb(0xffcc00),
};

pub(super) const MATERIAL_LIGHT: BuiltinScheme = BuiltinScheme {
    name: "Material Light",
    ansi: ansi16([
        0x546e7a, 0xff5370, 0x91b859, 0xffb62c, 0x6182b8, 0x7c4dff, 0x39adb5, 0x80cbc4, 0x546e7a,
        0xff5370, 0x91b859, 0xffb62c, 0x6182b8, 0x7c4dff, 0x39adb5, 0x80cbc4,
    ]),
    fg: rgb(0x80cbc4),
    bg: rgb(0xfafafa),
    cursor: rgb(0x272727),
};

pub(super) const NIGHTFOX: BuiltinScheme = BuiltinScheme {
    name: "Nightfox",
    ansi: ansi16([
        0x393b44, 0xc94f6d, 0x81b29a, 0xdbc074, 0x719cd6, 0x9d79d6, 0x63cdcf, 0xdfdfe0, 0x575860,
        0xd16983, 0x8ebaa4, 0xe0c989, 0x86abdc, 0xbaa1e2, 0x7ad5d6, 0xe4e4e5,
    ]),
    fg: rgb(0xcdcecf),
    bg: rgb(0x192330),
    cursor: rgb(0xcdcecf),
};

pub(super) const DAWNFOX: BuiltinScheme = BuiltinScheme {
    name: "Dawnfox",
    ansi: ansi16([
        0x575279, 0xb4637a, 0x618774, 0xea9d34, 0x286983, 0x907aa9, 0x56949f, 0xe5e9f0, 0x5b5078,
        0xc26d85, 0x629f81, 0xeea846, 0x2d81a3, 0x9b84b2, 0x5fa7b1, 0xeef0f3,
    ]),
    fg: rgb(0x575279),
    bg: rgb(0xfaf4ed),
    cursor: rgb(0x575279),
};

pub(super) const CARBONFOX: BuiltinScheme = BuiltinScheme {
    name: "Carbonfox",
    ansi: ansi16([
        0x282828, 0xee5396, 0x25be6a, 0x08bdba, 0x78a9ff, 0xbe95ff, 0x33b1ff, 0xdfdfe0, 0x484848,
        0xf16da6, 0x46c880, 0x2dc7c4, 0x8cb6ff, 0xc8a5ff, 0x52bdff, 0xe4e4e5,
    ]),
    fg: rgb(0xf2f4f8),
    bg: rgb(0x161616),
    cursor: rgb(0xf2f4f8),
};

pub(super) const GITHUB_DARK: BuiltinScheme = BuiltinScheme {
    name: "GitHub Dark",
    ansi: ansi16([
        0x484f58, 0xff7b72, 0x7ee787, 0xd29922, 0x79c0ff, 0xd2a8ff, 0xa5d6ff, 0xb1bac4, 0x6e7681,
        0xffa198, 0x56d364, 0xe3b341, 0xa5d6ff, 0xd2a8ff, 0xb6e3ff, 0xf0f6fc,
    ]),
    fg: rgb(0xc9d1d9),
    bg: rgb(0x0d1117),
    cursor: rgb(0xc9d1d9),
};

pub(super) const GITHUB_LIGHT: BuiltinScheme = BuiltinScheme {
    name: "GitHub Light",
    ansi: ansi16([
        0x24292e, 0xd73a49, 0x22863a, 0xb08800, 0x0366d6, 0x6f42c1, 0x1b7c83, 0x6a737d, 0x959da5,
        0xcb2431, 0x28a745, 0xdbab09, 0x2188ff, 0x8a63d2, 0x3192aa, 0x24292e,
    ]),
    fg: rgb(0x24292e),
    bg: rgb(0xffffff),
    cursor: rgb(0x24292e),
};

pub(super) const GITHUB_DARK_DIMMED: BuiltinScheme = BuiltinScheme {
    name: "GitHub Dark Dimmed",
    ansi: ansi16([
        0x545d68, 0xf47067, 0x57ab5a, 0xc69026, 0x539bf5, 0xb083f0, 0x76e3ea, 0xadbac7, 0x636e7b,
        0xff938a, 0x6bc46d, 0xdaaa3f, 0x6cb6ff, 0xdcbdfb, 0xb3f0ff, 0xf0f6fc,
    ]),
    fg: rgb(0xadbac7),
    bg: rgb(0x22272e),
    cursor: rgb(0xadbac7),
};
