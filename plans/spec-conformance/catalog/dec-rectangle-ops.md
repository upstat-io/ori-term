---
schema_version: "1.0"
stack: dec-rectangle-ops
title: "DEC Private CSI Rectangular Operations Catalog"
owner_section: "09A"
---

# DEC Private CSI Rectangular Operations Catalog

This catalog covers DEC private CSI sequences that operate on a 4-coordinate rectangular area (top, left, bottom, right). It includes DECRQCRA (the checksum-query op that surfaced the original gap via esctest), the DECxxxA family (DECCARA, DECRARA, DECCRA, DECFRA, DECERA, DECSERA), DECSACE (modifier for DECCARA/DECRARA), and the xterm extensions XTCHECKSUM and XTREPORTSGR.

- Primary spec: xterm `ctlseqs.txt` + DEC STD 070 §6 / VT420 PRM
- Cross-reference: wezterm `docs/escape-sequences.md`
- Stack ID prefix: `DECRECT`
- Related catalogs: `catalog/dec-private-modes.md` (numeric DECSET/DECRST), `catalog/dec-presentation.md` (column ops + presentation queries)

---

## DECRECT-DECSACE

| Field | Value |
|---|---|
| **ID** | `DECRECT-DECSACE` |
| **Spec source** | xterm ctlseqs.txt `CSI Ps * x` |
| **Sequence** | `CSI Ps * x` — Select attribute change extent |
| **Description** | Controls which attributes are changed by DECCARA/DECRARA; Ps=0 stream, Ps=1 rect |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | effect-mode-state |
| **Test chain** | parser:pending dispatch:pending state:pending |
| **Verification** | missing |
| **De-facto ref** | — |
| **Notes** | Param stored as `ace_mode` on Term; consumed by DECCARA/DECRARA to determine change extent |

---

## DECRECT-DECCARA

| Field | Value |
|---|---|
| **ID** | `DECRECT-DECCARA` |
| **Spec source** | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr;Pm $ r` |
| **Sequence** | `CSI Pt;Pl;Pb;Pr;Pm $ r` — Change attributes in rectangular area |
| **Description** | Applies SGR attribute change to cells in the specified rectangle |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | state-snapshot |
| **Test chain** | parser:pending dispatch:pending state:pending snapshot:pending |
| **Verification** | missing |
| **De-facto ref** | wezterm `docs/escape-sequences.md` DECCARA |
| **Notes** | DECLRMM-aware; DECSACE mode governs stream vs rect extent |

---

## DECRECT-DECRARA

| Field | Value |
|---|---|
| **ID** | `DECRECT-DECRARA` |
| **Spec source** | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr;Pm $ t` |
| **Sequence** | `CSI Pt;Pl;Pb;Pr;Pm $ t` — Reverse attributes in rectangular area |
| **Description** | Reverses (toggles) video attributes in cells within the specified rectangle |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | state-snapshot |
| **Test chain** | parser:pending dispatch:pending state:pending snapshot:pending |
| **Verification** | missing |
| **De-facto ref** | — |
| **Notes** | Reversal applies only to SGR attributes listed in Pm params |

---

## DECRECT-DECCRA

| Field | Value |
|---|---|
| **ID** | `DECRECT-DECCRA` |
| **Spec source** | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr;Pp;Pt;Pl;Pp $ v` |
| **Sequence** | `CSI Pt;Pl;Pb;Pr;Pp;Pt;Pl;Pp $ v` — Copy rectangular area |
| **Description** | Copies a rectangular area of cells from source page to destination page |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | state-snapshot |
| **Test chain** | parser:pending dispatch:pending state:pending snapshot:pending |
| **Verification** | missing |
| **De-facto ref** | wezterm `docs/escape-sequences.md` DECCRA |
| **Notes** | Source and destination pages; overlapping regions defined by copy-before-overwrite semantics |

---

## DECRECT-DECFRA

| Field | Value |
|---|---|
| **ID** | `DECRECT-DECFRA` |
| **Spec source** | xterm ctlseqs.txt `CSI Pc;Pt;Pl;Pb;Pr $ x` |
| **Sequence** | `CSI Pc;Pt;Pl;Pb;Pr $ x` — Fill rectangular area |
| **Description** | Fills the specified rectangular area with character Pc and current SGR attributes |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | state-snapshot |
| **Test chain** | parser:pending dispatch:pending state:pending snapshot:pending |
| **Verification** | missing |
| **De-facto ref** | wezterm `docs/escape-sequences.md` DECFRA |
| **Notes** | DECLRMM-aware; Pc is a character code point, not a string |

---

## DECRECT-XTCHECKSUM

| Field | Value |
|---|---|
| **ID** | `DECRECT-XTCHECKSUM` |
| **Spec source** | xterm ctlseqs.txt `CSI Ps # y` |
| **Sequence** | `CSI Ps # y` — Select checksum extension flags (xterm) |
| **Description** | Sets xterm checksum algorithm flags used by subsequent DECRQCRA requests |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | effect-mode-state |
| **Test chain** | parser:pending dispatch:pending state:pending |
| **Verification** | missing |
| **De-facto ref** | xterm patch-336 |
| **Notes** | Ps is a bitmask; stored as `checksum_flags: u16` on Term; consumed by DECRQCRA handler |

---

## DECRECT-DECRQCRA

| Field | Value |
|---|---|
| **ID** | `DECRECT-DECRQCRA` |
| **Spec source** | xterm ctlseqs.txt `CSI Pi;Pg;Pt;Pl;Pb;Pr * y` |
| **Sequence** | `CSI Pi;Pg;Pt;Pl;Pb;Pr * y` — Request checksum of rectangular area |
| **Description** | Computes a checksum of the specified rectangular area and emits a DCS reply |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | effect-pty-write |
| **Test chain** | parser:pending dispatch:pending effect:pending |
| **Verification** | missing |
| **De-facto ref** | xterm patch-336 (algorithm); esctest2 `DECRQCRA` suite (coordinate clamping) |
| **Notes** | Reply format: `DCS Pi ! ~ XXXX ST` (4-hex-digit checksum); synchronous emission via PtyEffect::Write (NOT HostRequest); algorithm: xor-folded 16-bit sum of attribute-selected cell data |

---

## DECRECT-DECERA

| Field | Value |
|---|---|
| **ID** | `DECRECT-DECERA` |
| **Spec source** | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr $ z` |
| **Sequence** | `CSI Pt;Pl;Pb;Pr $ z` — Erase rectangular area |
| **Description** | Erases all characters in the specified rectangle (replaces with space, default attrs) |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | state-snapshot |
| **Test chain** | parser:pending dispatch:pending state:pending snapshot:pending |
| **Verification** | missing |
| **De-facto ref** | — |
| **Notes** | DECLRMM-aware; respects DECSCA selective-erase protection attribute |

---

## DECRECT-DECSERA

| Field | Value |
|---|---|
| **ID** | `DECRECT-DECSERA` |
| **Spec source** | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr $ {` |
| **Sequence** | `` CSI Pt;Pl;Pb;Pr $ { `` — Selective erase rectangular area |
| **Description** | Erases unprotected characters in the specified rectangle (DECSCA-protected cells are skipped) |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | state-snapshot |
| **Test chain** | parser:pending dispatch:pending state:pending snapshot:pending |
| **Verification** | missing |
| **De-facto ref** | — |
| **Notes** | Companion to DECERA; only erases cells not marked with DECSCA protection |

---

## DECRECT-XTREPORTSGR

| Field | Value |
|---|---|
| **ID** | `DECRECT-XTREPORTSGR` |
| **Spec source** | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr # \|` |
| **Sequence** | `CSI Pt;Pl;Pb;Pr # \|` — Report selected graphic rendition (xterm) |
| **Description** | Emits the SGR attributes for each cell in the rectangle as a DCS stream |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | effect-pty-write |
| **Test chain** | parser:pending dispatch:pending effect:pending |
| **Verification** | missing |
| **De-facto ref** | xterm patch-336 |
| **Notes** | DCS reply per-cell format; complex serialization; verified-with-deviation acceptable if only basic SGR attrs are included |
