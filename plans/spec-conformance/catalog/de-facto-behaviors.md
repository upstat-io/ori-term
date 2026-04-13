---
schema_version: "1.0"
stack: defacto
title: "De-facto Behaviors Catalog"
owner_section: "01 (bootstrap), 08 + 15 (verification)"
---

# De-facto Behaviors Catalog

Sequences where the authoritative "spec" is an established terminal emulator's behavior rather than a formal standard, OR where ori_term's behavior diverges from ECMA-48 / DEC STD 070 to match de-facto expectations. The `De-facto ref` column names the reference terminal whose behavior is being tracked.

Section 01.8 reconciliation pass moves any rows that are purely de-facto here (rather than to `ecma-48.md` / `xterm-ctlseqs.md`).

| ID | Spec source | Sequence | Description | Implementation | Apex layer | Test chain | Verification | De-facto ref | Notes |
|---|---|---|---|---|---|---|---|---|---|
| DFCT-SGR-SUB-COLON | ITU T.416 (ODA colon sub-params) | `` `CSI 4 : 3 m` `` (curl), `` `CSI 38 : 2 : : r : g : b m` `` (RGB) | Colon-separated SGR sub-parameters | `` `attrs_from_sgr_parameters` + `handle_colon_rgb` (`crates/vte/src/ansi/dispatch/csi.rs`) `` | state-snapshot | parser:pending dispatch:pending state:pending | implemented-unverified | wezterm escape-sequences.md | ITU T.416 says colon; ECMA-48 never standardized this form. wezterm / kitty / alacritty all support it. |
| DFCT-SGR-UNDERLINE-COLOR | Kitty underline-color extension | `` `CSI 58 ; 5 ; Ps m` `` / `` `CSI 58 ; 2 ; r ; g ; b m` `` | Underline color — cross-reference `ecma-48.md::ECMA48-SGR-58` / `ECMA48-SGR-59` | See `ecma-48.md::ECMA48-SGR-58` / `ECMA48-SGR-59` | state-snapshot | — | implemented-unverified | wezterm escape-sequences.md | Cross-reference row. |
| DFCT-CURSOR-STYLE-OSC-50 | xterm ctlseqs (OSC 50) | `` `OSC 50 ; CursorShape=Ps BEL\|ST` `` | Legacy cursor shape OSC — cross-reference `osc.md::OSC-50` | See `osc.md::OSC-50` | state-snapshot | — | implemented-unverified | — | Cross-reference row. Preferred modern form is DECSCUSR (`xterm-ctlseqs.md::XT-DECSCUSR`). |
| DFCT-CELL-ALPHA | — (to be defined by Section 15) | To be defined — cell-level transparency | Cell-level alpha blending (foreground / background transparency) | MISSING — to be added by Section 15 (Cell-Level Alpha + Transparency) | gpu-instance | — | missing | — | No established spec; Section 15 defines ori_term's de-facto semantics. |
| DFCT-CSI-XTSMGRAPHICS | — (de-facto) | `` `CSI ? Ps ; Ps ; Ps S` `` (XTSMGRAPHICS) | xterm Set/Get Graphics Attributes — sixel/color register negotiation | MISSING — reference impl (reviewer decision) required before Section 12 (Sixel) picks it up | parser-only | parser:pending | missing | captures/notcurses-demo-intro.cap | Emitted by notcurses-demo (4×). xterm extension for negotiating sixel graphics capabilities. |
| DFCT-CSI-XTVERSION | — (de-facto) | `` `CSI > Ps q` `` (XTVERSION) | xterm version query — requests terminal version string | MISSING — reference impl (reviewer decision) required before Section 08 picks it up | effect-pty-write | parser:pending | missing | captures/notcurses-demo-intro.cap, captures/tmux-split-resize.cap | Emitted by notcurses-demo (1×), tmux (1×). Expected response: `DCS > \| version ST`. |
| DFCT-DCS-XTGETTCAP | — (de-facto) | `` `DCS + q Pt ST` `` (XTGETTCAP) | xterm Get Termcap/Terminfo — requests named capability value | MISSING — reference impl (reviewer decision) required before Section 08 picks it up | effect-pty-write | parser:pending | missing | captures/notcurses-demo-intro.cap | Emitted by notcurses-demo (1×). Expected response: `DCS 1 + r name=value ST` or `DCS 0 + r ST`. |
| DFCT-APC-GENERIC | — (de-facto) | `` `APC Pt ST` `` | Application Program Command — generic non-kitty APC | MISSING — parser drops; not dispatched outside kitty graphics protocol | parser-only | parser:pending | missing | captures/notcurses-demo-intro.cap | Emitted by notcurses-demo (1×). Kitty graphics uses `APC G <payload> ST` (separate KG-* rows). |
| DFCT-CSI-PERCENT-M | — (de-facto) | `` `CSI % Ps m` `` | Unknown CSI with `%` intermediate and `m` final | MISSING — parser drops; investigate whether this is a malformed emission or a reserved sequence | parser-only | parser:pending | missing | captures/vim-edit-passwd.cap | Emitted by vim (1×). May be a malformed SGR with stray `%` intermediate byte. |
| DFCT-DCS-Z | — (de-facto) | `` `DCS Pt z ST` `` | Unknown DCS with `z` final byte | MISSING — parser drops; investigate whether this is a malformed emission or a reserved sequence | parser-only | parser:pending | missing | captures/vim-edit-passwd.cap | Emitted by vim (1×). No known terminal protocol uses DCS with `z` final byte. |
