//! Extended color schemes (Batch 1): classic editor and terminal themes.

#![allow(clippy::unreadable_literal)]

use super::{BuiltinScheme, ansi16, rgb};

/// Monokai Pro — the official Monokai Pro palette by Monokai (monokai.pro).
pub(super) const MONOKAI_PRO: BuiltinScheme = BuiltinScheme {
    name: "Monokai Pro",
    ansi: ansi16([
        0x2d2a2e, 0xff6186, 0xa9dc76, 0xffd866, 0xfc9867, 0xab9df2, 0x78dae6, 0xfcfcfa, 0x727072,
        0xff6186, 0xa9dc76, 0xffd866, 0xfc9867, 0xab9df2, 0x78dae6, 0xfcfcfa,
    ]),
    fg: rgb(0xfcfcfa),
    bg: rgb(0x2d2a2e),
    cursor: rgb(0xc1c0c0),
};

/// Monokai Soda — Monokai variant with deeper contrast and neon accents.
pub(super) const MONOKAI_SODA: BuiltinScheme = BuiltinScheme {
    name: "Monokai Soda",
    ansi: ansi16([
        0x1a1a1a, 0xf4005f, 0x98e024, 0xfa8419, 0x9d65ff, 0xf4005f, 0x58d1eb, 0xc4c5b5, 0x625e4c,
        0xf4005f, 0x98e024, 0xe0d561, 0x9d65ff, 0xf4005f, 0x58d1eb, 0xf6f6ef,
    ]),
    fg: rgb(0xc4c5b5),
    bg: rgb(0x1a1a1a),
    cursor: rgb(0xf6f7ec),
};

/// Argonaut — bold, saturated colors on a near-black background.
pub(super) const ARGONAUT: BuiltinScheme = BuiltinScheme {
    name: "Argonaut",
    ansi: ansi16([
        0x232323, 0xff000f, 0x8ce10b, 0xffb900, 0x008df8, 0x6d43a6, 0x00d8eb, 0xffffff, 0x444444,
        0xff2740, 0xabe15b, 0xffd242, 0x0092ff, 0x9a5feb, 0x67fff0, 0xffffff,
    ]),
    fg: rgb(0xfffaf4),
    bg: rgb(0x0e1019),
    cursor: rgb(0xff0018),
};

/// Espresso — warm coffee-toned dark theme from the Espresso editor.
pub(super) const ESPRESSO: BuiltinScheme = BuiltinScheme {
    name: "Espresso",
    ansi: ansi16([
        0x353535, 0xd25252, 0xa5c261, 0xffc66d, 0x6c99bb, 0xd197d9, 0xbed6ff, 0xeeeeec, 0x535353,
        0xf00c0c, 0xc2e075, 0xe1e48b, 0x8ab7d9, 0xefb5f7, 0xdcf4ff, 0xffffff,
    ]),
    fg: rgb(0xffffff),
    bg: rgb(0x323232),
    cursor: rgb(0xd6d6d6),
};

/// Nightfly — deep navy theme inspired by vim-nightfly-colors (bluz71).
pub(super) const NIGHTFLY: BuiltinScheme = BuiltinScheme {
    name: "Nightfly",
    ansi: ansi16([
        0x1d3b53, 0xfc514e, 0xa1cd5e, 0xe3d18a, 0x82aaff, 0xc792ea, 0x7fdbca, 0xa1aab8, 0x7c8f8f,
        0xff5874, 0x21c7a8, 0xecc48d, 0x82aaff, 0xae81ff, 0x7fdbca, 0xd6deeb,
    ]),
    fg: rgb(0xbdc1c6),
    bg: rgb(0x011627),
    cursor: rgb(0x9ca1aa),
};

/// Srcery — a dark color scheme with vivid, high-contrast colors.
pub(super) const SRCERY: BuiltinScheme = BuiltinScheme {
    name: "Srcery",
    ansi: ansi16([
        0x1c1b19, 0xef2f27, 0x519f50, 0xfbb829, 0x2c78bf, 0xe02c6d, 0x0aaeb3, 0xbaa67f, 0x918175,
        0xf75341, 0x98bc37, 0xfed06e, 0x68a8e4, 0xff5c8f, 0x2be4d0, 0xfce8c3,
    ]),
    fg: rgb(0xfce8c3),
    bg: rgb(0x1c1b19),
    cursor: rgb(0xfbb829),
};

/// Cobalt2 — vibrant blue-based theme by Wes Bos.
pub(super) const COBALT2: BuiltinScheme = BuiltinScheme {
    name: "Cobalt2",
    ansi: ansi16([
        0x000000, 0xff0000, 0x38de21, 0xffe50a, 0x1460d2, 0xff005d, 0x00bbbb, 0xbbbbbb, 0x555555,
        0xf40e17, 0x3bd01d, 0xedc809, 0x5555ff, 0xff55ff, 0x6ae3fa, 0xffffff,
    ]),
    fg: rgb(0xffffff),
    bg: rgb(0x132738),
    cursor: rgb(0xf0cc09),
};

/// Jellybeans — a warm, colorful Vim theme by `NanoTech`.
pub(super) const JELLYBEANS: BuiltinScheme = BuiltinScheme {
    name: "Jellybeans",
    ansi: ansi16([
        0x929292, 0xe27373, 0x94b979, 0xffba7b, 0x97bedc, 0xe1c0fa, 0x00988e, 0xdedede, 0xbdbdbd,
        0xffa1a1, 0xbddeab, 0xffdca0, 0xb1d8f6, 0xfbdaff, 0x1ab2a8, 0xffffff,
    ]),
    fg: rgb(0xdedede),
    bg: rgb(0x121212),
    cursor: rgb(0xffa560),
};

/// Molokai — a dark color scheme based on the Vim Molokai theme.
pub(super) const MOLOKAI: BuiltinScheme = BuiltinScheme {
    name: "Molokai",
    ansi: ansi16([
        0x121212, 0xfa2573, 0x98e123, 0xdfd460, 0x1080d0, 0x8700ff, 0x43a8d0, 0xbbbbbb, 0x555555,
        0xf6669d, 0xb1e05f, 0xfff26d, 0x00afff, 0xaf87ff, 0x51ceff, 0xffffff,
    ]),
    fg: rgb(0xbbbbbb),
    bg: rgb(0x121212),
    cursor: rgb(0xbbbbbb),
};

/// Wombat — subdued, earthy-toned dark theme.
pub(super) const WOMBAT: BuiltinScheme = BuiltinScheme {
    name: "Wombat",
    ansi: ansi16([
        0x000000, 0xff615a, 0xb1e969, 0xebd99c, 0x5da9f6, 0xe86aff, 0x82fff7, 0xdedacf, 0x313131,
        0xf58c80, 0xddf88f, 0xeee5b2, 0xa5c7ff, 0xddaaff, 0xb7fff9, 0xffffff,
    ]),
    fg: rgb(0xdedacf),
    bg: rgb(0x171717),
    cursor: rgb(0xbbbbbb),
};

/// Afterglow — subdued warm dark theme with muted colors.
pub(super) const AFTERGLOW: BuiltinScheme = BuiltinScheme {
    name: "Afterglow",
    ansi: ansi16([
        0x151515, 0xac4142, 0x7e8e50, 0xe5b567, 0x6c99bb, 0x9f4e85, 0x7dd6cf, 0xd0d0d0, 0x505050,
        0xac4142, 0x7e8e50, 0xe5b567, 0x6c99bb, 0x9f4e85, 0x7dd6cf, 0xf5f5f5,
    ]),
    fg: rgb(0xd0d0d0),
    bg: rgb(0x212121),
    cursor: rgb(0xd0d0d0),
};

/// Spacegray — cool-toned dark theme inspired by the Spacegray Sublime Text theme.
pub(super) const SPACEGRAY: BuiltinScheme = BuiltinScheme {
    name: "Spacegray",
    ansi: ansi16([
        0x000000, 0xb04b57, 0x87b379, 0xe5c179, 0x7d8fa4, 0xa47996, 0x85a7a5, 0xb3b8c3, 0x000000,
        0xb04b57, 0x87b379, 0xe5c179, 0x7d8fa4, 0xa47996, 0x85a7a5, 0xffffff,
    ]),
    fg: rgb(0xb3b8c3),
    bg: rgb(0x20242d),
    cursor: rgb(0xb3b8c3),
};

/// Tender — soft pastel dark theme with warm highlights.
pub(super) const TENDER: BuiltinScheme = BuiltinScheme {
    name: "Tender",
    ansi: ansi16([
        0x1d1d1d, 0xc5152f, 0xc9d05c, 0xffc24b, 0xb3deef, 0xd3b987, 0x73cef4, 0xeeeeee, 0x323232,
        0xf43753, 0xd9e066, 0xfacc72, 0xc0eafb, 0xefd093, 0xa1d6ec, 0xffffff,
    ]),
    fg: rgb(0xeeeeee),
    bg: rgb(0x282828),
    cursor: rgb(0xeeeeee),
};

/// Flatland — desaturated dark theme inspired by the Flatland Sublime Text theme.
pub(super) const FLATLAND: BuiltinScheme = BuiltinScheme {
    name: "Flatland",
    ansi: ansi16([
        0x1d1d19, 0xf18339, 0x9fd364, 0xf4ef6d, 0x5096be, 0x695abc, 0xd63865, 0xffffff, 0x1d1d19,
        0xd22a24, 0xa7d42c, 0xff8949, 0x61b9d0, 0x695abc, 0xd63865, 0xffffff,
    ]),
    fg: rgb(0xb8dbef),
    bg: rgb(0x1d1f21),
    cursor: rgb(0x708284),
};

/// Twilight — warm, low-contrast dark theme inspired by the `TextMate` Twilight theme.
pub(super) const TWILIGHT: BuiltinScheme = BuiltinScheme {
    name: "Twilight",
    ansi: ansi16([
        0x141414, 0xc06d44, 0xafb97a, 0xc2a86c, 0x44474a, 0xb4be7c, 0x778385, 0xffffd4, 0x262626,
        0xde7c4c, 0xccd88c, 0xe2c47e, 0x5a5e62, 0xd0dc8e, 0x8a989b, 0xffffd4,
    ]),
    fg: rgb(0xffffd4),
    bg: rgb(0x141414),
    cursor: rgb(0xffffff),
};
