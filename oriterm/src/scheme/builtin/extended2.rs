//! Extended color schemes (Batch 2): dark/light pairs, creative/modern,
//! and Base16 variants.

#![allow(clippy::unreadable_literal)]

use super::{BuiltinScheme, ansi16, rgb};

/// Modus Vivendi — Emacs built-in dark accessibility theme by Protesilaos Stavrou.
pub(super) const MODUS_VIVENDI: BuiltinScheme = BuiltinScheme {
    name: "Modus Vivendi",
    ansi: ansi16([
        0x000000, 0xff5f59, 0x44bc44, 0xd0bc00, 0x2fafff, 0xfeacd0, 0x00d3d0, 0xffffff, 0x1e1e1e,
        0xff5f5f, 0x44df44, 0xefef00, 0x338fff, 0xff66ff, 0x00eff0, 0x989898,
    ]),
    fg: rgb(0xffffff),
    bg: rgb(0x000000),
    cursor: rgb(0xffffff),
};

/// Modus Operandi — Emacs built-in light accessibility theme by Protesilaos Stavrou.
pub(super) const MODUS_OPERANDI: BuiltinScheme = BuiltinScheme {
    name: "Modus Operandi",
    ansi: ansi16([
        0xffffff, 0xa60000, 0x006800, 0x6f5500, 0x0031a9, 0x721045, 0x005e8b, 0x000000, 0xf2f2f2,
        0xd00000, 0x008900, 0x808000, 0x0000ff, 0xdd22dd, 0x008899, 0x595959,
    ]),
    fg: rgb(0x000000),
    bg: rgb(0xffffff),
    cursor: rgb(0x000000),
};

/// Pencil Dark — dark variant of the iA Writer-inspired Pencil theme.
pub(super) const PENCIL_DARK: BuiltinScheme = BuiltinScheme {
    name: "Pencil Dark",
    ansi: ansi16([
        0x212121, 0xc30771, 0x10a778, 0xa89c14, 0x008ec4, 0x523c79, 0x20a5ba, 0xd9d9d9, 0x424242,
        0xfb007a, 0x5fd7af, 0xf3e430, 0x20bbfc, 0x6855de, 0x4fb8cc, 0xf1f1f1,
    ]),
    fg: rgb(0xf1f1f1),
    bg: rgb(0x212121),
    cursor: rgb(0x20bbfc),
};

/// Pencil Light — light variant of the iA Writer-inspired Pencil theme.
pub(super) const PENCIL_LIGHT: BuiltinScheme = BuiltinScheme {
    name: "Pencil Light",
    ansi: ansi16([
        0x212121, 0xc30771, 0x10a778, 0xa89c14, 0x008ec4, 0x523c79, 0x20a5ba, 0xd9d9d9, 0x424242,
        0xfb007a, 0x5fd7af, 0xf3e430, 0x20bbfc, 0x6855de, 0x4fb8cc, 0xf1f1f1,
    ]),
    fg: rgb(0x424242),
    bg: rgb(0xf1f1f1),
    cursor: rgb(0x20bbfc),
};

/// Seoul256 Dark — muted dark theme inspired by the Vim seoul256 colorscheme.
pub(super) const SEOUL256_DARK: BuiltinScheme = BuiltinScheme {
    name: "Seoul256 Dark",
    ansi: ansi16([
        0x4e4e4e, 0xd68787, 0x5f865f, 0xd8af5f, 0x85add4, 0xd7afaf, 0x87afaf, 0xd0d0d0, 0x626262,
        0xd75f87, 0x87af87, 0xffd787, 0xadd4fb, 0xffafaf, 0x87d7d7, 0xe4e4e4,
    ]),
    fg: rgb(0xd0d0d0),
    bg: rgb(0x3a3a3a),
    cursor: rgb(0xd0d0d0),
};

/// Seoul256 Light — muted light theme inspired by the Vim seoul256 colorscheme.
pub(super) const SEOUL256_LIGHT: BuiltinScheme = BuiltinScheme {
    name: "Seoul256 Light",
    ansi: ansi16([
        0x4e4e4e, 0xaf5f5f, 0x5f885f, 0xaf8760, 0x5f87ae, 0x875f87, 0x5f8787, 0xe4e4e4, 0x3a3a3a,
        0x870100, 0x005f00, 0xd8865f, 0x0087af, 0x87025f, 0x008787, 0xeeeeee,
    ]),
    fg: rgb(0x4e4e4e),
    bg: rgb(0xdadada),
    cursor: rgb(0x4e4e4e),
};

/// Tango Dark — classic GNOME/Tango Desktop Project dark palette.
pub(super) const TANGO_DARK: BuiltinScheme = BuiltinScheme {
    name: "Tango Dark",
    ansi: ansi16([
        0x000000, 0xcc0000, 0x4e9a06, 0xc4a000, 0x3465a4, 0x75507b, 0x06989a, 0xd3d7cf, 0x555753,
        0xef2929, 0x8ae234, 0xfce94f, 0x729fcf, 0xad7fa8, 0x34e2e2, 0xeeeeec,
    ]),
    fg: rgb(0xffffff),
    bg: rgb(0x000000),
    cursor: rgb(0xffffff),
};

/// Tango Light — classic GNOME/Tango Desktop Project light palette.
pub(super) const TANGO_LIGHT: BuiltinScheme = BuiltinScheme {
    name: "Tango Light",
    ansi: ansi16([
        0x000000, 0xcc0000, 0x4e9a06, 0xc4a000, 0x3465a4, 0x75507b, 0x06989a, 0xd3d7cf, 0x555753,
        0xef2929, 0x8ae234, 0xfce94f, 0x729fcf, 0xad7fa8, 0x34e2e2, 0xeeeeec,
    ]),
    fg: rgb(0x000000),
    bg: rgb(0xffffff),
    cursor: rgb(0x000000),
};

/// Zenbones Dark — warm, low-contrast dark theme from the zenbones.nvim family.
pub(super) const ZENBONES_DARK: BuiltinScheme = BuiltinScheme {
    name: "Zenbones Dark",
    ansi: ansi16([
        0x1c1917, 0xde6e7c, 0x819b69, 0xb77e64, 0x6099c0, 0xb279a7, 0x66a5ad, 0xb4bdc3, 0x403833,
        0xe8838f, 0x8bae68, 0xd68c67, 0x61abda, 0xcf86c1, 0x65b8c1, 0x888f94,
    ]),
    fg: rgb(0xb4bdc3),
    bg: rgb(0x1c1917),
    cursor: rgb(0xc4cacf),
};

/// Zenbones Light — warm, low-contrast light theme from the zenbones.nvim family.
pub(super) const ZENBONES_LIGHT: BuiltinScheme = BuiltinScheme {
    name: "Zenbones Light",
    ansi: ansi16([
        0xf0edec, 0xa8334c, 0x4f6c31, 0x944927, 0x286486, 0x88507d, 0x3b8992, 0x2c363c, 0xcfc1ba,
        0x94253e, 0x3f5a22, 0x803d1c, 0x1d5573, 0x7b3b70, 0x2b747c, 0x4f5e68,
    ]),
    fg: rgb(0x2c363c),
    bg: rgb(0xf0edec),
    cursor: rgb(0x2c363c),
};

/// Nvim Dark — Neovim's built-in dark colorscheme.
pub(super) const NVIM_DARK: BuiltinScheme = BuiltinScheme {
    name: "Nvim Dark",
    ansi: ansi16([
        0x07080d, 0xffc0b9, 0xb3f6c0, 0xfce094, 0xa6dbff, 0xffcaff, 0x8cf8f7, 0xeef1f8, 0x4f5258,
        0xffc0b9, 0xb3f6c0, 0xfce094, 0xa6dbff, 0xffcaff, 0x8cf8f7, 0xeef1f8,
    ]),
    fg: rgb(0xe0e2ea),
    bg: rgb(0x14161b),
    cursor: rgb(0x9b9ea4),
};

/// Nvim Light — Neovim's built-in light colorscheme.
pub(super) const NVIM_LIGHT: BuiltinScheme = BuiltinScheme {
    name: "Nvim Light",
    ansi: ansi16([
        0x07080d, 0x590008, 0x005523, 0x6b5300, 0x004c73, 0x470045, 0x007373, 0xeef1f8, 0x4f5258,
        0x590008, 0x005523, 0x6b5300, 0x004c73, 0x470045, 0x007373, 0xeef1f8,
    ]),
    fg: rgb(0x14161b),
    bg: rgb(0xe0e2ea),
    cursor: rgb(0x9b9ea4),
};

/// Everblush — vibrant dark theme with a warm, saturated palette.
pub(super) const EVERBLUSH: BuiltinScheme = BuiltinScheme {
    name: "Everblush",
    ansi: ansi16([
        0x232a2d, 0xe57474, 0x8ccf7e, 0xe5c76b, 0x67b0e8, 0xc47fd5, 0x6cbfbf, 0xb3b9b8, 0x2d3437,
        0xef7e7e, 0x96d988, 0xf4d67a, 0x71baf2, 0xce89df, 0x67cbe7, 0xbdc3c2,
    ]),
    fg: rgb(0xdadada),
    bg: rgb(0x141b1e),
    cursor: rgb(0xdadada),
};

/// Fairy Floss — pastel neon theme by `sailorhg`.
pub(super) const FAIRY_FLOSS: BuiltinScheme = BuiltinScheme {
    name: "Fairy Floss",
    ansi: ansi16([
        0x040303, 0xf92672, 0xc2ffdf, 0xe6c000, 0xc2ffdf, 0xffb8d1, 0xc5a3ff, 0xf8f8f0, 0x6090cb,
        0xff857f, 0xc2ffdf, 0xffea00, 0xc2ffdf, 0xffb8d1, 0xc5a3ff, 0xf8f8f0,
    ]),
    fg: rgb(0xf8f8f2),
    bg: rgb(0x5a5475),
    cursor: rgb(0xf8f8f0),
};

/// Shades of Purple — vivid purple-heavy dark theme by Ahmad Awais.
pub(super) const SHADES_OF_PURPLE: BuiltinScheme = BuiltinScheme {
    name: "Shades of Purple",
    ansi: ansi16([
        0x1e1e3f, 0xd90429, 0x3ad900, 0xffe700, 0x6943ff, 0xff2c70, 0x00c5c7, 0xc7c7c7, 0x808080,
        0xd90429, 0x3ad900, 0xffe700, 0x6943ff, 0xff2c70, 0x00c5c7, 0xffffff,
    ]),
    fg: rgb(0xc7c7c7),
    bg: rgb(0x1e1e3f),
    cursor: rgb(0xc7c7c7),
};

/// Synthwave — retro synthwave aesthetic with neon on deep violet.
pub(super) const SYNTHWAVE: BuiltinScheme = BuiltinScheme {
    name: "Synthwave",
    ansi: ansi16([
        0x011627, 0xfe4450, 0x72f1b8, 0xfede5d, 0x03edf9, 0xff7edb, 0x03edf9, 0xffffff, 0x575656,
        0xfe4450, 0x72f1b8, 0xfede5d, 0x03edf9, 0xff7edb, 0x03edf9, 0xffffff,
    ]),
    fg: rgb(0xffffff),
    bg: rgb(0x262335),
    cursor: rgb(0x03edf9),
};

/// Sakura — soft pink/lavender dark theme with cherry blossom tones.
pub(super) const SAKURA: BuiltinScheme = BuiltinScheme {
    name: "Sakura",
    ansi: ansi16([
        0x000000, 0xd52370, 0x41af1a, 0xbc7053, 0x6964ab, 0xc71fbf, 0x939393, 0x998eac, 0x786d69,
        0xf41d99, 0x22e529, 0xf59574, 0x9892f1, 0xe90cdd, 0xeeeeee, 0xcbb6ff,
    ]),
    fg: rgb(0xdd7bdc),
    bg: rgb(0x18131e),
    cursor: rgb(0xff65fd),
};

/// Spaceduck — intergalactic purple-tinted dark theme.
pub(super) const SPACEDUCK: BuiltinScheme = BuiltinScheme {
    name: "Spaceduck",
    ansi: ansi16([
        0x16172d, 0xe33400, 0x5ccc96, 0xb3a1e6, 0x00a3cc, 0xf2ce00, 0x7a5ccc, 0x686f9a, 0x16172d,
        0xe33400, 0x5ccc96, 0xb3a1e6, 0x00a3cc, 0xf2ce00, 0x7a5ccc, 0xf0f1ce,
    ]),
    fg: rgb(0xecf0c1),
    bg: rgb(0x0f111b),
    cursor: rgb(0xecf0c1),
};

/// Quiet Light — understated dark theme with soft, muted tones.
pub(super) const QUIET_LIGHT: BuiltinScheme = BuiltinScheme {
    name: "Quiet Light",
    ansi: ansi16([
        0x141414, 0xc16262, 0x49b685, 0xc5b76d, 0x4992b6, 0x815bbe, 0x41a4a4, 0xc5c5c5, 0x505050,
        0xed5e7a, 0x7ece7e, 0xdbdb70, 0x4dbfff, 0xc067e4, 0x70dbd8, 0xf0f0f0,
    ]),
    fg: rgb(0xb9b9b9),
    bg: rgb(0x141414),
    cursor: rgb(0xa0a0a0),
};

/// Base16 Default Dark — the reference Base16 dark palette by Chris Kempson.
pub(super) const BASE16_DEFAULT_DARK: BuiltinScheme = BuiltinScheme {
    name: "Base16 Default Dark",
    ansi: ansi16([
        0x181818, 0xab4642, 0xa1b56c, 0xf7ca88, 0x7cafc2, 0xba8baf, 0x86c1b9, 0xd8d8d8, 0x585858,
        0xab4642, 0xa1b56c, 0xf7ca88, 0x7cafc2, 0xba8baf, 0x86c1b9, 0xf8f8f8,
    ]),
    fg: rgb(0xd8d8d8),
    bg: rgb(0x181818),
    cursor: rgb(0xd8d8d8),
};

/// Base16 Default Light — the reference Base16 light palette by Chris Kempson.
pub(super) const BASE16_DEFAULT_LIGHT: BuiltinScheme = BuiltinScheme {
    name: "Base16 Default Light",
    ansi: ansi16([
        0xf8f8f8, 0xab4642, 0xa1b56c, 0xf7ca88, 0x7cafc2, 0xba8baf, 0x86c1b9, 0x383838, 0xb8b8b8,
        0xab4642, 0xa1b56c, 0xf7ca88, 0x7cafc2, 0xba8baf, 0x86c1b9, 0x181818,
    ]),
    fg: rgb(0x383838),
    bg: rgb(0xf8f8f8),
    cursor: rgb(0x383838),
};

/// Base16 Monokai — Base16 interpretation of the classic Monokai palette.
pub(super) const BASE16_MONOKAI: BuiltinScheme = BuiltinScheme {
    name: "Base16 Monokai",
    ansi: ansi16([
        0x272822, 0xf92672, 0xa6e22e, 0xf4bf75, 0x66d9ef, 0xae81ff, 0xa1efe4, 0xf8f8f2, 0x75715e,
        0xf92672, 0xa6e22e, 0xf4bf75, 0x66d9ef, 0xae81ff, 0xa1efe4, 0xf9f8f5,
    ]),
    fg: rgb(0xf8f8f2),
    bg: rgb(0x272822),
    cursor: rgb(0xf8f8f2),
};

/// Base16 Ocean Dark — cool blue-gray Base16 palette inspired by
/// ocean colors.
pub(super) const BASE16_OCEAN_DARK: BuiltinScheme = BuiltinScheme {
    name: "Base16 Ocean Dark",
    ansi: ansi16([
        0x2b303b, 0xbf616a, 0xa3be8c, 0xebcb8b, 0x8fa1b3, 0xb48ead, 0x96b5b4, 0xc0c5ce, 0x65737e,
        0xbf616a, 0xa3be8c, 0xebcb8b, 0x8fa1b3, 0xb48ead, 0x96b5b4, 0xeff1f5,
    ]),
    fg: rgb(0xc0c5ce),
    bg: rgb(0x2b303b),
    cursor: rgb(0xc0c5ce),
};

/// Base16 Eighties — warm retro Base16 palette by Chris Kempson.
pub(super) const BASE16_EIGHTIES: BuiltinScheme = BuiltinScheme {
    name: "Base16 Eighties",
    ansi: ansi16([
        0x2d2d2d, 0xf2777a, 0x99cc99, 0xffcc66, 0x6699cc, 0xcc99cc, 0x66cccc, 0xd3d0c8, 0x747369,
        0xf2777a, 0x99cc99, 0xffcc66, 0x6699cc, 0xcc99cc, 0x66cccc, 0xf2f0ec,
    ]),
    fg: rgb(0xd3d0c8),
    bg: rgb(0x2d2d2d),
    cursor: rgb(0xd3d0c8),
};

/// Xcode Dusk — muted dark theme inspired by the Xcode Dusk appearance.
pub(super) const XCODE_DUSK: BuiltinScheme = BuiltinScheme {
    name: "Xcode Dusk",
    ansi: ansi16([
        0x282b35, 0xb21889, 0xdf0002, 0x438288, 0x790ead, 0xb21889, 0x00a0be, 0x939599, 0x686a71,
        0xb21889, 0xdf0002, 0x438288, 0x790ead, 0xb21889, 0x00a0be, 0xbebfc2,
    ]),
    fg: rgb(0x939599),
    bg: rgb(0x282b35),
    cursor: rgb(0x939599),
};

/// Apprentice — dark, low-contrast Vim theme by Romain Lafourcade.
pub(super) const APPRENTICE: BuiltinScheme = BuiltinScheme {
    name: "Apprentice",
    ansi: ansi16([
        0x1c1c1c, 0xaf5f5f, 0x5f875f, 0x87875f, 0x5f87af, 0x5f5f87, 0x5f8787, 0x6c6c6c, 0x444444,
        0xff8700, 0x87af87, 0xffffaf, 0x8fafd7, 0x8787af, 0x5fafaf, 0xffffff,
    ]),
    fg: rgb(0xbcbcbc),
    bg: rgb(0x262626),
    cursor: rgb(0xbcbcbc),
};

/// Dark+ — VS Code's default dark color theme.
pub(super) const DARK_PLUS: BuiltinScheme = BuiltinScheme {
    name: "Dark+",
    ansi: ansi16([
        0x000000, 0xcd3131, 0x0dbc79, 0xe5e510, 0x2472c8, 0xbc3fbc, 0x11a8cd, 0xe5e5e5, 0x666666,
        0xf14c4c, 0x23d18b, 0xf5f543, 0x3b8eea, 0xd670d6, 0x29b8db, 0xe5e5e5,
    ]),
    fg: rgb(0xcccccc),
    bg: rgb(0x1e1e1e),
    cursor: rgb(0xffffff),
};

/// Ubuntu — the default Ubuntu terminal color palette.
pub(super) const UBUNTU: BuiltinScheme = BuiltinScheme {
    name: "Ubuntu",
    ansi: ansi16([
        0x2e3436, 0xcc0000, 0x4e9a06, 0xc4a000, 0x3465a4, 0x75507b, 0x06989a, 0xd3d7cf, 0x555753,
        0xef2929, 0x8ae234, 0xfce94f, 0x729fcf, 0xad7fa8, 0x34e2e2, 0xeeeeec,
    ]),
    fg: rgb(0xeeeeec),
    bg: rgb(0x300a24),
    cursor: rgb(0xbbbbbb),
};

/// Homebrew — retro green-on-black terminal theme.
pub(super) const HOMEBREW: BuiltinScheme = BuiltinScheme {
    name: "Homebrew",
    ansi: ansi16([
        0x000000, 0x990000, 0x00a600, 0x999900, 0x0000b2, 0xb200b2, 0x00a6b2, 0xbfbfbf, 0x666666,
        0xe50000, 0x00d900, 0xe5e500, 0x0000ff, 0xe500e5, 0x00e5e5, 0xe5e5e5,
    ]),
    fg: rgb(0x00ff00),
    bg: rgb(0x000000),
    cursor: rgb(0x23ff18),
};

/// Bluloco Dark — vivid, colorful dark theme by Umut Topuzoglu.
pub(super) const BLULOCO_DARK: BuiltinScheme = BuiltinScheme {
    name: "Bluloco Dark",
    ansi: ansi16([
        0x41444d, 0xfc2f52, 0x25a45c, 0xff936a, 0x3476ff, 0x7a82da, 0x4483aa, 0xcdd4e0, 0x8f9aae,
        0xff6480, 0x3fc56b, 0xf9c859, 0x10b1fe, 0xff78f8, 0x5fb9bc, 0xffffff,
    ]),
    fg: rgb(0xb9c0cb),
    bg: rgb(0x282c34),
    cursor: rgb(0xffcc00),
};

/// Bluloco Light — vivid, colorful light theme by Umut Topuzoglu.
pub(super) const BLULOCO_LIGHT: BuiltinScheme = BuiltinScheme {
    name: "Bluloco Light",
    ansi: ansi16([
        0xd5d6dd, 0xd52753, 0x23974a, 0xdf631c, 0x275fe4, 0x823ff1, 0x27618d, 0x000000, 0xe4e5ed,
        0xff6480, 0x3cbc66, 0xc5a332, 0x0099e1, 0xce33c0, 0x6d93bb, 0x26272d,
    ]),
    fg: rgb(0x383a42),
    bg: rgb(0xf9f9f9),
    cursor: rgb(0x383a42),
};

/// Duskfox — dusky rose-tinted variant of the Nightfox theme family.
pub(super) const DUSKFOX: BuiltinScheme = BuiltinScheme {
    name: "Duskfox",
    ansi: ansi16([
        0x393552, 0xeb6f92, 0xa3be8c, 0xf6c177, 0x569fba, 0xc4a7e7, 0x9ccfd8, 0xe0def4, 0x47407d,
        0xf083a2, 0xb1d196, 0xf9cb8c, 0x65b1cd, 0xccb1ed, 0xa6dae3, 0xe2e0f7,
    ]),
    fg: rgb(0xe0def4),
    bg: rgb(0x232136),
    cursor: rgb(0xe0def4),
};

/// Nordfox — warmer, more saturated variant of the Nord-inspired Nightfox family.
pub(super) const NORDFOX: BuiltinScheme = BuiltinScheme {
    name: "Nordfox",
    ansi: ansi16([
        0x3b4252, 0xbf616a, 0xa3be8c, 0xebcb8b, 0x81a1c1, 0xb48ead, 0x88c0d0, 0xe5e9f0, 0x465780,
        0xd06f79, 0xb1d196, 0xf0d399, 0x8cafd2, 0xc895bf, 0x93ccdc, 0xe7ecf4,
    ]),
    fg: rgb(0xcdcecf),
    bg: rgb(0x2e3440),
    cursor: rgb(0xcdcecf),
};

/// Breeze — KDE Plasma Breeze terminal color palette.
pub(super) const BREEZE: BuiltinScheme = BuiltinScheme {
    name: "Breeze",
    ansi: ansi16([
        0x31363b, 0xed1515, 0x11d116, 0xf67400, 0x1d99f3, 0x9b59b6, 0x1abc9c, 0xeff0f1, 0x7f8c8d,
        0xc0392b, 0x1cdc9a, 0xfdbc4b, 0x3daee9, 0x8e44ad, 0x16a085, 0xfcfcfc,
    ]),
    fg: rgb(0xeff0f1),
    bg: rgb(0x31363b),
    cursor: rgb(0xeff0f1),
};

/// Campbell — the Windows Terminal default color scheme.
pub(super) const CAMPBELL: BuiltinScheme = BuiltinScheme {
    name: "Campbell",
    ansi: ansi16([
        0x0c0c0c, 0xc50f1f, 0x13a10e, 0xc19c00, 0x0037da, 0x881798, 0x3a96dd, 0xcccccc, 0x767676,
        0xe74856, 0x16c60c, 0xf9f1a5, 0x3b78ff, 0xb4009e, 0x61d6d6, 0xf2f2f2,
    ]),
    fg: rgb(0xcccccc),
    bg: rgb(0x0c0c0c),
    cursor: rgb(0xffffff),
};

/// Nova — cool blue-green Base16 theme by George Essig and Trevor D. Miller.
pub(super) const NOVA: BuiltinScheme = BuiltinScheme {
    name: "Nova",
    ansi: ansi16([
        0x3c4c55, 0x83afe5, 0x7fc1ca, 0xa8ce93, 0x83afe5, 0x9a93e1, 0xf2c38f, 0xc5d4dd, 0x899ba6,
        0x83afe5, 0x7fc1ca, 0xa8ce93, 0x83afe5, 0x9a93e1, 0xf2c38f, 0x556873,
    ]),
    fg: rgb(0xc5d4dd),
    bg: rgb(0x3c4c55),
    cursor: rgb(0xc5d4dd),
};
