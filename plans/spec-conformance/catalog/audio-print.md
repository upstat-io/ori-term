---
schema_version: "1.0"
stack: audio_print
title: "Audio + Print Catalog"
owner_section: "01 (bootstrap), 20 (verification)"
---

# Audio + Print Catalog

Terminal audio (BEL variants, DECPS) and the ancient print-through protocol (CSI `i` for copy-to-printer). Section 20 (Audio + Print) drives rows to `verified`.

| ID | Spec source | Sequence | Description | Implementation | Apex layer | Test chain | Verification | De-facto ref | Notes |
|---|---|---|---|---|---|---|---|---|---|
| AUDIO-BEL | ECMA-48 §8.3.3 | `` `BEL` `` (`0x07`) | Bell — cross-reference `ecma-48.md::ECMA48-C0-BEL` | See `ecma-48.md::ECMA48-C0-BEL` | effect-host-notification | — | implemented-unverified | — | Cross-reference row. Visual bell via tab-bar pulse; audible bell stub (BUG-08-1). |
| AUDIO-DECPS | DEC STD 070 §6.3 | `` `CSI Pvolume ; Pduration ; Pnote ,~` `` (DECPS) | DEC Play Sound — internal speaker beep | MISSING — to be added by Section 20 (Audio + Print) | effect-audio | parser:pending dispatch:pending effect:pending | missing | — | No dispatch arm in `csi::dispatch`. |
| PRINT-MC-ON | ECMA-48 §8.3.82 | `` `CSI i` `` / `` `CSI ? 5 i` `` (MC) | Media Copy on / Auto-print on | MISSING — to be added by Section 20 (Audio + Print) | effect-pty-write | parser:pending dispatch:pending effect:pending | missing | — | Historical print-through. Unlikely to ship beyond a stub. |
| PRINT-MC-OFF | ECMA-48 §8.3.82 | `` `CSI 4 i` `` / `` `CSI ? 4 i` `` (MC) | Media Copy off / Auto-print off | MISSING — to be added by Section 20 (Audio + Print) | effect-pty-write | parser:pending dispatch:pending effect:pending | missing | — | |
