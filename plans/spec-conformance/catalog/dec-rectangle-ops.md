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
| `DECRECT-DECSACE` | xterm ctlseqs.txt `CSI Ps * x` | `` `CSI Ps * x` `` | Select attribute change extent; Ps=0 stream, Ps=1 rect | `oriterm_core/src/term/handler/rect_ops/mod.rs::decsace_impl` | effect-mode-state | parser:green dispatch:green state:green | implemented-unverified | — | Ps=2 → Rectangle, all other values → Stream; stored as `AceMode` on Term (NEVER on Grid per §09A.R finding #9 LEAK guard); consumed by DECCARA/DECRARA; RIS resets to Stream |
| `DECRECT-DECCARA` | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr;Pm $ r` | `` `CSI Pt;Pl;Pb;Pr;Pm $ r` `` | Change attributes in rectangular area | `oriterm_core/src/term/handler/rect_ops/mod.rs::deccara_impl` | state-snapshot | parser:green dispatch:green state:green snapshot:pending | implemented-unverified | wezterm docs/escape-sequences.md DECCARA | DECLRMM-aware via `clamp_rect`; DECSACE governs stream/rect extent; delegates to `Grid::apply_sgr_rect` (grid-layer ownership per §09A.R finding #8); DEC SGR subset only (0/1/4/5/7/8/22/24/25/27/28) |
| `DECRECT-DECRARA` | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr;Pm $ t` | `` `CSI Pt;Pl;Pb;Pr;Pm $ t` `` | Reverse attributes in rectangular area | `oriterm_core/src/term/handler/rect_ops/mod.rs::decrara_impl` | state-snapshot | parser:green dispatch:green state:green snapshot:pending | implemented-unverified | — | XOR-toggle of SGR bits per cell; Ps=0 toggles every reversible bit (BOLD/UNDERLINE/BLINK/INVERSE/HIDDEN); delegates to `Grid::reverse_sgr_rect`; DECLRMM-aware + DECSACE-aware |
| `DECRECT-DECCRA` | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr;Pp;Pt;Pl;Pp $ v` | `` `CSI Pt;Pl;Pb;Pr;Pp;Pt;Pl;Pp $ v` `` | Copy rectangular area from source page to destination page | `oriterm_core/src/term/handler/rect_ops/mod.rs::deccra_impl` | state-snapshot | parser:green dispatch:green state:green snapshot:pending | implemented-unverified | wezterm docs/escape-sequences.md DECCRA | Copy-before-overwrite via scratch buffer in `Grid::copy_rect` (single allocation per DECCRA, permitted by plan); page params ignored (ori_term is single-page — verified-with-deviation); dest clipped to grid bounds |
| `DECRECT-DECFRA` | xterm ctlseqs.txt `CSI Pc;Pt;Pl;Pb;Pr $ x` | `` `CSI Pc;Pt;Pl;Pb;Pr $ x` `` | Fill rectangular area with character Pc + current SGR | `oriterm_core/src/term/handler/rect_ops/mod.rs::decfra_impl` | state-snapshot | parser:green dispatch:green state:green snapshot:pending | implemented-unverified | wezterm docs/escape-sequences.md DECFRA, xterm charproc.c:5659-5677 | Pc=0 defaults to space per xterm `use_default_value`; Pc in [0x20..=0x7E] ∪ [0xA0..=0xFF] else ignored; delegates to `Grid::fill_rect`; DECLRMM-aware |
| `DECRECT-XTCHECKSUM` | xterm ctlseqs.txt `CSI Ps # y` | `` `CSI Ps # y` `` | Select checksum extension flags (xterm patch-336) | `oriterm_core/src/term/handler/rect_ops/mod.rs::xtchecksum_impl` | effect-mode-state | parser:green dispatch:green state:green | implemented-unverified | xterm patch-336 | Ps is a bitmask; stored as `checksum_flags: u16` on Term; consumed by DECRQCRA handler |
| `DECRECT-DECRQCRA` | xterm ctlseqs.txt `CSI Pi;Pg;Pt;Pl;Pb;Pr * y` | `` `CSI Pi;Pg;Pt;Pl;Pb;Pr * y` `` | Request checksum of rectangular area; emits DCS reply | `oriterm_core/src/term/handler/rect_ops/mod.rs::decrqcra_impl` | effect-pty-write | parser:green dispatch:green effect:green | implemented-unverified | xterm patch-336 (algorithm); esctest2 DECRQCRA suite (clamping) | Reply `DCS Pi !~ XXXX ST` (4-hex checksum); synchronous emission via PtyEffect::Write (NOT HostRequest); algorithm = xterm sum-then-negate |
| `DECRECT-DECERA` | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr $ z` | `` `CSI Pt;Pl;Pb;Pr $ z` `` | Erase rectangular area (space + default attrs) | `oriterm_core/src/term/handler/rect_ops/mod.rs::decera_impl` | state-snapshot | parser:green dispatch:green state:green snapshot:pending | implemented-unverified | — | DECLRMM-aware via `clamp_rect`; IGNORES `CellFlags::PROTECTED` (DECSERA honors it); delegates to `Grid::erase_rect_all` with BCE bg from cursor template |
| `DECRECT-DECSERA` | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr $ {` | `` `CSI Pt;Pl;Pb;Pr $ {` `` | Selective erase rectangular area (skip DECSCA-protected cells) | `oriterm_core/src/term/handler/rect_ops/mod.rs::decsera_impl` | state-snapshot | parser:green dispatch:green state:green snapshot:pending | implemented-unverified | — | Companion to DECERA; PROTECTED cells (set by DECSCA Ps=1) survive the erase; delegates to `Grid::erase_rect_unprotected` |
| `DECRECT-XTREPORTSGR` | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr # \|` | `` `CSI Pt;Pl;Pb;Pr # \|` `` | Report selected graphic rendition (xterm) | MISSING — to be added by Section 09A | effect-pty-write | parser:pending dispatch:pending effect:pending | missing | xterm patch-336 | DCS reply per-cell format; verified-with-deviation acceptable if only basic SGR attrs included |
