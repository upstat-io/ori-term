---
schema_version: "0.1-provisional"
stack: charsets
title: "Charsets + UAX Policy Catalog"
owner_section: "01 (bootstrap), 18 (verification)"
---

# Charsets + UAX Policy Catalog

Charset designation (G0–G3), shift in/out, single shift, and the ori_term UAX (Unicode Annex) policy for normalization / width / grapheme clustering. Section 18 (Charsets + UAX Policy) drives rows to `verified`.

Standard ECMA-48 charset designation sequences live in `ecma-48.md::ECMA48-ESC-B` and `ECMA48-ESC-0`. This file extends them with the UAX policy rows.

| ID | Spec source | Sequence | Description | Implementation | Apex layer | Test chain | Verification | De-facto ref | Notes |
|---|---|---|---|---|---|---|---|---|---|
| CHSET-G0-G3-DESIGNATE | ECMA-48 §5.3 | `` `ESC ( B/0` ``, `` `ESC ) B/0` ``, `` `ESC * B/0` ``, `` `ESC + B/0` `` | Designate ASCII / DEC Special to G0/G1/G2/G3 — cross-reference `ecma-48.md::ECMA48-ESC-B` and `ECMA48-ESC-0` | See `ecma-48.md::ECMA48-ESC-B` / `ECMA48-ESC-0` | state-snapshot | — | implemented-unverified | — | Cross-reference row. |
| CHSET-SO-SI | ECMA-48 §8.3.114 / §8.3.119 | `` `SO` `` / `` `SI` `` | Activate G1 / G0 as GL — cross-reference `ecma-48.md::ECMA48-C0-SO` / `ECMA48-C0-SI` | See `ecma-48.md::ECMA48-C0-SO` / `ECMA48-C0-SI` | state-snapshot | — | implemented-unverified | — | Cross-reference row. |
| CHSET-SS2-SS3 | ECMA-48 §8.3.112 / §8.3.113 | `` `ESC N` `` / `` `ESC O` `` | Single Shift G2 / G3 — cross-reference `ecma-48.md::ECMA48-ESC-N` / `ECMA48-ESC-O` | See `ecma-48.md::ECMA48-ESC-N` / `ECMA48-ESC-O` | state-snapshot | — | implemented-unverified | — | Cross-reference row. |
| UAX-WIDTH | UAX #11 (East Asian Width) | — (not a sequence; it's a width policy) | Cell width policy — `unicode-width` crate (CJK = 2, combining = 0, ZWJ = merged) | `` `Term::input` (`oriterm_core/src/term/handler/mod.rs`) — uses `unicode_width::UnicodeWidthChar::width` `` | state-snapshot | parser:pending state:pending | implemented-unverified | — | Section 18 verifies width against UAX #11 reference. |
| UAX-GRAPHEME-CLUSTERING | UAX #29 (Grapheme Cluster Boundaries) | — | Grapheme cluster policy (affects selection, search, reflow) | `` `unicode-segmentation` usage in selection / search paths (`oriterm_core/src/selection`, `oriterm_core/src/search`) `` | state-snapshot | state:pending | implemented-unverified | — | |
| UAX-BIDI | UAX #9 (Unicode Bidirectional Algorithm) | — | BiDi policy for RTL scripts (Hebrew, Arabic) | MISSING — to be added by Section 18 (Charsets + UAX Policy) | state-snapshot | state:pending | missing | — | No BiDi handling today. XT-SCP dispatch arm parses BiDi direction but hits the `Handler` default impl. |
| CHSET-NRCS | DEC STD 070 §5.3 | `` `ESC ( A/C/E/H/K/Q/R/Y/Z/=` `` etc. | DEC NRCS (National Replacement Character Sets) | MISSING — to be added by Section 18 (Charsets + UAX Policy) | state-snapshot | state:pending | missing | — | Only ASCII + DEC Special are designated today. |
| CHSET-LS2 | ECMA-48 §8.3.78 | `` `ESC n` `` (LS2) | Locking Shift 2 — invoke G2 into GL | MISSING — to be added by Section 18 (Charsets + UAX Policy) | state-snapshot | parser:pending dispatch:pending state:pending | missing | — | Not dispatched in `esc_dispatch`. Locking shifts change the GL mapping until the next locking shift. Distinct from SS2 which is single-character. |
| CHSET-LS3 | ECMA-48 §8.3.79 | `` `ESC o` `` (LS3) | Locking Shift 3 — invoke G3 into GL | MISSING — to be added by Section 18 (Charsets + UAX Policy) | state-snapshot | parser:pending dispatch:pending state:pending | missing | — | Not dispatched in `esc_dispatch`. |
| CHSET-LS1R | ECMA-48 §8.3.76 | `` `ESC ~` `` (LS1R) | Locking Shift 1 Right — invoke G1 into GR | MISSING — to be added by Section 18 (Charsets + UAX Policy) | state-snapshot | parser:pending dispatch:pending state:pending | missing | — | Not dispatched in `esc_dispatch`. GR-side locking shifts; rarely used in modern terminals. |
| CHSET-LS2R | ECMA-48 §8.3.77 | `` `ESC }` `` (LS2R) | Locking Shift 2 Right — invoke G2 into GR | MISSING — to be added by Section 18 (Charsets + UAX Policy) | state-snapshot | parser:pending dispatch:pending state:pending | missing | — | Not dispatched. |
| CHSET-LS3R | ECMA-48 §8.3.75 | `` `ESC \|` `` (LS3R) | Locking Shift 3 Right — invoke G3 into GR | MISSING — to be added by Section 18 (Charsets + UAX Policy) | state-snapshot | parser:pending dispatch:pending state:pending | missing | — | Not dispatched. |
