---
schema_version: "0.1-provisional"
stack: historical
title: "Historical Legacy Control Stacks Catalog"
owner_section: "01 (bootstrap), 19 + 26 (verification)"
---

# Historical Legacy Control Stacks Catalog

VT52, DEC LK201, Wyse 50/60, ADM-3A, IBM PC ANSI.SYS, Microsoft Console VT — all the legacy stacks ori_term either emulates or refuses to emulate. Sections 19 (Historical Legacy Control Stacks) and 26 (Historical Vector Stacks — ReGIS + Tek 4010/4014) drive rows to `verified`.

| ID | Spec source | Sequence | Description | Implementation | Apex layer | Test chain | Verification | De-facto ref | Notes |
|---|---|---|---|---|---|---|---|---|---|
| HIST-VT52-CURSOR | DEC VT52 user manual | `` `ESC A` `` (cursor up), `` `ESC B` `` (down), `` `ESC C` `` (right), `` `ESC D` `` (left) | VT52 cursor motion (pre-ANSI) | MISSING — to be added by Section 19 (Historical Legacy Control Stacks) | state-snapshot | parser:pending dispatch:pending state:pending | missing | — | VT52 mode is not currently entered — `DECANM` (`CSI ? 2 h/l`) is not dispatched. |
| HIST-VT52-HOME | DEC VT52 user manual | `` `ESC H` `` | VT52 home (conflicts with ECMA-48 HTS — ambiguous without DECANM) | See `ecma-48.md::ECMA48-ESC-H` | state-snapshot | — | implemented-unverified | — | In ANSI mode `ESC H` is HTS (set tab stop); in VT52 mode it's home. ori_term is always in ANSI mode. |
| HIST-PC-ANSI-SYS | IBM ANSI.SYS (de-facto) | Various `CSI` sequences | IBM PC ANSI.SYS sequences (mostly overlap with xterm) | — | — | — | implemented-unverified | wezterm escape-sequences.md | Most ANSI.SYS sequences are strict subsets of xterm; ori_term inherits them via the `ecma-48.md` / `xterm-ctlseqs.md` rows. |
| HIST-REGIS | DEC STD 070 §6.4 (ReGIS) | `` `DCS p ... ST` `` (ReGIS introducer) | ReGIS vector graphics | MISSING — to be added by Section 26 (Historical Vector Stacks) | texture-render | parser:pending dispatch:pending snapshot:pending texture:pending | missing | — | Shared vector raster helper planned in Section 26. |
| HIST-TEK-4010 | Tek 4010 programmer ref | `` `ESC FS` ``, `` `ESC GS` ``, etc. | Tek 4010/4014 vector plotter | MISSING — to be added by Section 26 (Historical Vector Stacks) | texture-render | parser:pending dispatch:pending texture:pending | missing | — | |
| HIST-TMUX-CONTROL | tmux control mode | `` `DCS 1000 q ... ST` `` | tmux multiplexer control mode | MISSING — to be added by Section 19 (Historical Legacy Control Stacks) | effect-pty-write | parser:pending dispatch:pending effect:pending | missing | wezterm escape-sequences.md | Allows a host terminal to bridge tmux into its own multiplexer. wezterm has partial support (see wezterm #336). |
