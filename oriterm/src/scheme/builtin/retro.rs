//! Retro and classic color schemes (Snazzy, Tomorrow, Zenburn, Iceberg, `PaperColor`).

#![allow(clippy::unreadable_literal)]

use super::{BuiltinScheme, ansi16, rgb};

pub(super) const SNAZZY: BuiltinScheme = BuiltinScheme {
    name: "Snazzy",
    ansi: ansi16([
        0x282a36, 0xff5c57, 0x5af78e, 0xf3f99d, 0x57c7ff, 0xff6ac1, 0x9aedfe, 0xf1f1f0, 0x686868,
        0xff5c57, 0x5af78e, 0xf3f99d, 0x57c7ff, 0xff6ac1, 0x9aedfe, 0xf1f1f0,
    ]),
    fg: rgb(0xeff0eb),
    bg: rgb(0x282a36),
    cursor: rgb(0x97979b),
};

pub(super) const TOMORROW_NIGHT: BuiltinScheme = BuiltinScheme {
    name: "Tomorrow Night",
    ansi: ansi16([
        0x1d1f21, 0xcc6666, 0xb5bd68, 0xf0c674, 0x81a2be, 0xb294bb, 0x8abeb7, 0xc5c8c6, 0x969896,
        0xcc6666, 0xb5bd68, 0xf0c674, 0x81a2be, 0xb294bb, 0x8abeb7, 0xffffff,
    ]),
    fg: rgb(0xc5c8c6),
    bg: rgb(0x1d1f21),
    cursor: rgb(0xc5c8c6),
};

pub(super) const TOMORROW_LIGHT: BuiltinScheme = BuiltinScheme {
    name: "Tomorrow Light",
    ansi: ansi16([
        0x000000, 0xc82829, 0x718c00, 0xeab700, 0x4271ae, 0x8959a8, 0x3e999f, 0xffffff, 0x8e908c,
        0xc82829, 0x718c00, 0xeab700, 0x4271ae, 0x8959a8, 0x3e999f, 0xffffff,
    ]),
    fg: rgb(0x4d4d4c),
    bg: rgb(0xffffff),
    cursor: rgb(0x4d4d4c),
};

pub(super) const ZENBURN: BuiltinScheme = BuiltinScheme {
    name: "Zenburn",
    ansi: ansi16([
        0x4d4d4d, 0x705050, 0x60b48a, 0xdfaf8f, 0x506070, 0xdc8cc3, 0x8cd0d3, 0xdcdccc, 0x709080,
        0xdca3a3, 0xc3bf9f, 0xf0dfaf, 0x94bff3, 0xec93d3, 0x93e0e3, 0xffffff,
    ]),
    fg: rgb(0xdcdccc),
    bg: rgb(0x3f3f3f),
    cursor: rgb(0x73635a),
};

pub(super) const ICEBERG_DARK: BuiltinScheme = BuiltinScheme {
    name: "Iceberg Dark",
    ansi: ansi16([
        0x1e2132, 0xe27878, 0xb4be82, 0xe2a478, 0x84a0c6, 0xa093c7, 0x89b8c2, 0xc6c8d1, 0x6b7089,
        0xe98989, 0xc0ca8e, 0xe9b189, 0x91acd1, 0xada0d3, 0x95c4ce, 0xd2d4de,
    ]),
    fg: rgb(0xc6c8d1),
    bg: rgb(0x161821),
    cursor: rgb(0xc6c8d1),
};

pub(super) const ICEBERG_LIGHT: BuiltinScheme = BuiltinScheme {
    name: "Iceberg Light",
    ansi: ansi16([
        0xdcdfe7, 0xcc517a, 0x668e3d, 0xc57339, 0x2d539e, 0x7759b4, 0x3f83a6, 0x33374c, 0x8389a3,
        0xcc3768, 0x598030, 0xb6662d, 0x22478e, 0x6845ad, 0x327698, 0x262a3f,
    ]),
    fg: rgb(0x33374c),
    bg: rgb(0xe8e9ec),
    cursor: rgb(0x33374c),
};

pub(super) const PAPERCOLOR_DARK: BuiltinScheme = BuiltinScheme {
    name: "PaperColor Dark",
    ansi: ansi16([
        0x1c1c1c, 0xaf005f, 0x5faf00, 0xd7af5f, 0x5fafd7, 0x808080, 0xd7875f, 0xd0d0d0, 0x585858,
        0x5faf5f, 0xafd700, 0xaf87d7, 0xffaf00, 0xff5faf, 0x00afaf, 0x5f8787,
    ]),
    fg: rgb(0xd0d0d0),
    bg: rgb(0x1c1c1c),
    cursor: rgb(0xd0d0d0),
};

pub(super) const PAPERCOLOR_LIGHT: BuiltinScheme = BuiltinScheme {
    name: "PaperColor Light",
    ansi: ansi16([
        0xeeeeee, 0xaf0000, 0x008700, 0x5f8700, 0x0087af, 0x878787, 0x005f87, 0x444444, 0xbcbcbc,
        0xd70000, 0xd70087, 0x8700af, 0xd75f00, 0xd75f00, 0x005faf, 0x005f87,
    ]),
    fg: rgb(0x444444),
    bg: rgb(0xeeeeee),
    cursor: rgb(0x444444),
};
