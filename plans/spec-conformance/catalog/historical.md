---
schema_version: "1.0"
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
| HIST-DEC-LK201 | DEC LK201 technical manual | Various function-key and editing-key sequences | DEC LK201 keyboard — function keys F1–F20 and editing keypad | See `xterm-ctlseqs.md` (key encoding rows) — ori_term's key encoder covers DEC function-key sequences | effect-pty-write | — | implemented-unverified | — | LK201 function-key escapes are the ancestor of xterm's function-key encoding. ori_term inherits them via its VT220/xterm-compatible key encoder. Section 19 verifies the full legacy function-key matrix. |
| HIST-WYSE-50-60 | Wyse 50/60 user manual | Proprietary single-byte control codes + CSI subsets | Wyse 50/60 proprietary sequences | MISSING — to be added by Section 19 (Historical Legacy Control Stacks) | state-snapshot | parser:pending dispatch:pending state:pending | missing | — | Wyse terminals use incompatible CSI addressing and attribute codes. ori_term does not emulate Wyse mode; Section 19 documents the delta and justifies the refusal. |
| HIST-ADM-3A | ADM-3A operator's manual | `` `ESC =` `` (home), `` `^Z` `` (clear screen), etc. | ADM-3A dumb terminal sequences | MISSING — to be added by Section 19 (Historical Legacy Control Stacks) | state-snapshot | parser:pending dispatch:pending state:pending | missing | — | ADM-3A is the ancestor of vi cursor keys (h/j/k/l). Its escape sequences mostly conflict with ANSI. ori_term does not emulate ADM-3A mode. |
| HIST-MS-CONSOLE-VT | Microsoft Console VT spec | CSI subset + ConPTY win32-input mode (mode 9001) | Microsoft Console Virtual Terminal — Windows ConPTY sequences | See `dec-private-modes.md::DEC-WIN32-INPUT` for mode 9001. Additional ConPTY-specific CSI sub-ops in Section 19 | effect-mode-state | parser:pending dispatch:pending state:pending | stub | — | ConPTY win32-input mode (9001) is tracked. Full Microsoft Console VT compatibility (including resize, focus events via ConPTY API) is Section 19 scope. |
