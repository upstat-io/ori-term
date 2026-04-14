---
schema_version: "1.0"
stack: sixel
title: "Sixel Graphics Catalog"
owner_section: "01 (bootstrap), 12 (verification)"
---

# Sixel Graphics Catalog

DCS q sixel introducer + payload. Section 12 (Sixel) is blocked until the `kitty.rs` BLOAT split lands (see `plans/bug-tracker/section-08-core-terminal.md`). Section 01 populates the rows so the blocker is visible against a concrete catalog target.

| ID | Spec source | Sequence | Description | Implementation | Apex layer | Test chain | Verification | De-facto ref | Notes |
|---|---|---|---|---|---|---|---|---|---|
| SIXEL-DCS-q | DEC STD 070 §6.3 + VT340 sixel | `` `DCS Ps1 ; Ps2 ; Ps3 q <data> ST` `` | Sixel raster image — enters sixel accumulation state | `` `Performer::hook` (`crates/vte/src/ansi/dispatch/mod.rs`) → `Term::sixel_start` → `Term::handle_sixel_start` (`oriterm_core/src/term/handler/image/sixel.rs`) `` | texture-render | parser:pending dispatch:pending snapshot:pending frame-input:pending texture:pending | implemented-unverified | — | Sets `DcsState::Sixel`. Raster params Ps1/Ps2/Ps3 encode aspect ratio + background option. |
| SIXEL-DCS-put | DEC STD 070 §6.3 | `` `DCS q <data>` `` (per-byte payload) | Sixel pixel-data accumulation | `` `Performer::put` (`crates/vte/src/ansi/dispatch/mod.rs`) — `DcsState::Sixel` arm → `Term::sixel_put` → `Term::handle_sixel_put` (`oriterm_core/src/term/handler/image/sixel.rs`) `` | texture-render | parser:pending dispatch:pending snapshot:pending frame-input:pending texture:pending | implemented-unverified | — | Each payload byte feeds into the parser's sixel state machine. |
| SIXEL-DCS-unhook | DEC STD 070 §6.3 | `` `DCS q <data> ST` `` | Sixel finalize — commit image to cache | `` `Performer::unhook` (`crates/vte/src/ansi/dispatch/mod.rs`) — `DcsState::Sixel` arm → `Term::sixel_end` → `Term::handle_sixel_end` (`oriterm_core/src/term/handler/image/sixel.rs`) `` | texture-render | parser:pending dispatch:pending snapshot:pending frame-input:pending texture:pending | implemented-unverified | — | Finalizes the image and hands it to the cache. |
| SIXEL-MODE-80 | xterm ctlseqs (mode 80) | `` `CSI ? 80 h` `` / `` `CSI ? 80 l` `` | Sixel scrolling mode — row cross-reference to `dec-private-modes.md::DEC-SIXEL-SCROLLING` | See `dec-private-modes.md::DEC-SIXEL-SCROLLING` | effect-mode-state | — | implemented-unverified | — | Cross-reference row. Actual row lives in `dec-private-modes.md`. |
| SIXEL-MODE-8452 | xterm ctlseqs (mode 8452) | `` `CSI ? 8452 h` `` / `` `CSI ? 8452 l` `` | Sixel cursor-right mode — row cross-reference to `dec-private-modes.md::DEC-SIXEL-CURSOR-RIGHT` | See `dec-private-modes.md::DEC-SIXEL-CURSOR-RIGHT` | effect-mode-state | — | implemented-unverified | — | Cross-reference row. |
