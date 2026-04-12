---
schema_version: "0.1-provisional"
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
