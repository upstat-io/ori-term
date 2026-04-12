---
schema_version: "0.1-provisional"
stack: kittykbd
title: "Kitty Keyboard Protocol Catalog"
owner_section: "01 (bootstrap), 17 (verification)"
---

# Kitty Keyboard Protocol Catalog

The Kitty keyboard protocol extends xterm's key reporting with modifier/event-type flags and a push/pop stack. Dispatch flows through `csi::dispatch` for the `u`-family sequences and through `Term::*_keyboard_mode` handlers in `oriterm_core/src/term/handler/dcs.rs`.

Section 17 (Kitty Keyboard Protocol) absorbs the existing `oriterm/src/key_encoding/terminfo_xcheck/` surface and drives every row to `verified`.

| ID | Spec source | Sequence | Description | Implementation | Apex layer | Test chain | Verification | De-facto ref | Notes |
|---|---|---|---|---|---|---|---|---|---|
| KKBD-QUERY | Kitty keyboard-protocol | `` `CSI ? u` `` | Query current flags → `CSI ? <flags> u` response (cross-reference `xterm-ctlseqs.md::XT-KITTY-KBD-QUERY`) | See `xterm-ctlseqs.md::XT-KITTY-KBD-QUERY` | effect-pty-write | — | implemented-unverified | — | Cross-reference row. |
| KKBD-PUSH | Kitty keyboard-protocol | `` `CSI > Ps u` `` | Push flags onto the keyboard-mode stack | `` `csi::dispatch` (`crates/vte/src/ansi/dispatch/csi.rs`) — `('u', [b'>'])` arm → `Term::push_keyboard_mode` → `Term::dcs_push_keyboard_mode` (`oriterm_core/src/term/handler/dcs.rs`) `` | effect-mode-state | parser:pending dispatch:pending state:pending | implemented-unverified | — | Stack max depth `KEYBOARD_MODE_STACK_MAX_DEPTH`. |
| KKBD-POP | Kitty keyboard-protocol | `` `CSI < Ps u` `` | Pop flags from the keyboard-mode stack | `` `csi::dispatch` (`crates/vte/src/ansi/dispatch/csi.rs`) — `('u', [b'<'])` arm → `Term::pop_keyboard_modes` → `Term::dcs_pop_keyboard_modes` (`oriterm_core/src/term/handler/dcs.rs`) `` | effect-mode-state | parser:pending dispatch:pending state:pending | implemented-unverified | — | |
| KKBD-SET | Kitty keyboard-protocol | `` `CSI = Ps ; Ps u` `` | Set flags with apply behavior (replace / union / difference) | `` `csi::dispatch` (`crates/vte/src/ansi/dispatch/csi.rs`) — `('u', [b'='])` arm → `Term::set_keyboard_mode` → `Term::dcs_set_keyboard_mode` (`oriterm_core/src/term/handler/dcs.rs`) `` | effect-mode-state | parser:pending dispatch:pending state:pending | implemented-unverified | — | `KeyboardModes` bitflags: disambiguate esc codes / report event types / report alternate keys / report all keys as esc / report associated text. |
| KKBD-KEY-REPORT | Kitty keyboard-protocol | `` `CSI key ; mods ; event u` `` | Enhanced key event report (press / release / repeat) | MISSING — to be added by Section 17 (Kitty Keyboard Protocol) | effect-pty-write | parser:pending dispatch:pending effect:pending | missing | — | Key encoding itself lives in `oriterm/src/key_encoding/` which is outside `oriterm_core`. |
