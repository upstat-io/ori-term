---
schema_version: "1.0"
stack: dec-rectangle-ops
title: "DEC Private CSI Rectangular Operations Catalog"
owner_section: "09A"
---

# DEC Private CSI Rectangular Operations Catalog

DEC private CSI sequences that operate on a 4-coordinate rectangular area (top, left, bottom, right). Covers:

- **DECRQCRA** — checksum query that surfaced the original top-down gap via esctest.
- **DECxxxA family** — DECCARA, DECRARA, DECCRA, DECFRA, DECERA, DECSERA.
- **DECSACE** — attribute-change-extent modifier for DECCARA/DECRARA.
- **xterm extensions** — XTCHECKSUM (patch-336 checksum flags) and XTREPORTSGR (per-cell SGR report).

Primary spec: xterm `ctlseqs.txt` + DEC STD 070 §6 / VT420 Programming Reference Manual.

Cross-reference: wezterm `docs/escape-sequences.md` (behavior tiebreaker for ambiguous parameter handling).

Stack ID prefix: `DECRECT`.

Related catalogs: `catalog/dec-private-modes.md` (numeric DECSET/DECRST modes), `catalog/dec-presentation.md` (column ops + presentation queries).

| ID | Spec source | Sequence | Description | Implementation | Apex layer | Test chain | Verification | De-facto ref | Notes |
|---|---|---|---|---|---|---|---|---|---|
| `DECRECT-DECSACE` | xterm ctlseqs.txt `CSI Ps * x` | `` `CSI Ps * x` `` | Select attribute change extent; Ps=0 stream, Ps=1 rect | MISSING — to be added by Section 09A | effect-mode-state | parser:pending dispatch:pending state:pending | missing | — | Param stored as `ace_mode` on Term; consumed by DECCARA/DECRARA |
| `DECRECT-DECCARA` | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr;Pm $ r` | `` `CSI Pt;Pl;Pb;Pr;Pm $ r` `` | Change attributes in rectangular area | MISSING — to be added by Section 09A | state-snapshot | parser:pending dispatch:pending state:pending snapshot:pending | missing | wezterm docs/escape-sequences.md DECCARA | DECLRMM-aware; DECSACE mode governs stream vs rect extent |
| `DECRECT-DECRARA` | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr;Pm $ t` | `` `CSI Pt;Pl;Pb;Pr;Pm $ t` `` | Reverse attributes in rectangular area | MISSING — to be added by Section 09A | state-snapshot | parser:pending dispatch:pending state:pending snapshot:pending | missing | — | Reversal applies only to SGR attributes listed in Pm params |
| `DECRECT-DECCRA` | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr;Pp;Pt;Pl;Pp $ v` | `` `CSI Pt;Pl;Pb;Pr;Pp;Pt;Pl;Pp $ v` `` | Copy rectangular area from source page to destination page | MISSING — to be added by Section 09A | state-snapshot | parser:pending dispatch:pending state:pending snapshot:pending | missing | wezterm docs/escape-sequences.md DECCRA | Copy-before-overwrite semantics for overlapping regions |
| `DECRECT-DECFRA` | xterm ctlseqs.txt `CSI Pc;Pt;Pl;Pb;Pr $ x` | `` `CSI Pc;Pt;Pl;Pb;Pr $ x` `` | Fill rectangular area with character Pc + current SGR | MISSING — to be added by Section 09A | state-snapshot | parser:pending dispatch:pending state:pending snapshot:pending | missing | wezterm docs/escape-sequences.md DECFRA | DECLRMM-aware; Pc is a character code point, not a string |
| `DECRECT-XTCHECKSUM` | xterm ctlseqs.txt `CSI Ps # y` | `` `CSI Ps # y` `` | Select checksum extension flags (xterm patch-336) | `oriterm_core/src/term/handler/rect_ops/mod.rs::xtchecksum_impl` | effect-mode-state | parser:green dispatch:green state:green | implemented-unverified | xterm patch-336 | Ps is a bitmask; stored as `checksum_flags: u16` on Term; consumed by DECRQCRA handler |
| `DECRECT-DECRQCRA` | xterm ctlseqs.txt `CSI Pi;Pg;Pt;Pl;Pb;Pr * y` | `` `CSI Pi;Pg;Pt;Pl;Pb;Pr * y` `` | Request checksum of rectangular area; emits DCS reply | `oriterm_core/src/term/handler/rect_ops/mod.rs::decrqcra_impl` | effect-pty-write | parser:green dispatch:green effect:green | implemented-unverified | xterm patch-336 (algorithm); esctest2 DECRQCRA suite (clamping) | Reply `DCS Pi !~ XXXX ST` (4-hex checksum); synchronous emission via PtyEffect::Write (NOT HostRequest); algorithm = xterm sum-then-negate |
| `DECRECT-DECERA` | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr $ z` | `` `CSI Pt;Pl;Pb;Pr $ z` `` | Erase rectangular area (space + default attrs) | MISSING — to be added by Section 09A | state-snapshot | parser:pending dispatch:pending state:pending snapshot:pending | missing | — | DECLRMM-aware; respects DECSCA selective-erase protection attribute |
| `DECRECT-DECSERA` | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr $ {` | `` `CSI Pt;Pl;Pb;Pr $ {` `` | Selective erase rectangular area (skip DECSCA-protected cells) | MISSING — to be added by Section 09A | state-snapshot | parser:pending dispatch:pending state:pending snapshot:pending | missing | — | Companion to DECERA; only erases unprotected cells |
| `DECRECT-XTREPORTSGR` | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr # \|` | `` `CSI Pt;Pl;Pb;Pr # \|` `` | Report selected graphic rendition (xterm) | MISSING — to be added by Section 09A | effect-pty-write | parser:pending dispatch:pending effect:pending | missing | xterm patch-336 | DCS reply per-cell format; verified-with-deviation acceptable if only basic SGR attrs included |
