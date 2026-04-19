---
schema_version: "1.0"
stack: dec-presentation
title: "DEC Private CSI Presentation Operations Catalog"
owner_section: "09A"
---

# DEC Private CSI Presentation Operations Catalog

DEC private sequences for column operations, ESC-path back/forward index, and presentation-state queries. Covers:

- **Column ops** (`CSI ' }` / `CSI ' ~`): DECIC, DECDC — insert/delete columns; DECLRMM (mode 69) must be active.
- **Back/forward index** (`ESC 6` / `ESC 9`): DECBI, DECFI — these are ESC sequences, NOT CSI; dispatch lives in `crates/vte/src/ansi/dispatch/mod.rs`, not `csi.rs`.
- **CSI presentation queries**: DECRQPSR, DECRQUPSS, DECRQDE, DECSCL, DECSCA, DECSASD, DECSSDT — all dispatch through `csi.rs`.
- **DCS-path presentation queries**: DECRQSS (`DCS $ q Pt ST`), DECRSPS (`DCS Ps $ t Pt ST`) — dispatch lives in the DCS handler, NOT `csi.rs`.

Primary spec: `xterm ctlseqs.txt` + DEC STD 070 / VT420 Programming Reference Manual.

Cross-reference: `wezterm docs/escape-sequences.md` (behavior tiebreaker for ambiguous parameter handling).

Stack ID prefix: `DECPRES`.

Related catalogs: `catalog/dec-private-modes.md` (numeric DECSET/DECRST), `catalog/dec-rectangle-ops.md` (rectangular-area ops).

| ID | Spec source | Sequence | Description | Implementation | Apex layer | Test chain | Verification | De-facto ref | Notes |
|---|---|---|---|---|---|---|---|---|---|
| `DECPRES-DECIC` | xterm ctlseqs.txt §DECIC | `` `CSI Ps ' }` `` | Insert column(s) at cursor column, shifting existing columns right | MISSING — to be added by Section 09A | state-snapshot | parser:pending dispatch:pending state:pending snapshot:pending | missing | wezterm docs/escape-sequences.md | Requires DECLRMM (mode 69) active; no-op otherwise |
| `DECPRES-DECDC` | xterm ctlseqs.txt §DECDC | `` `CSI Ps ' ~` `` | Delete column(s) at cursor column, shifting remaining columns left | MISSING — to be added by Section 09A | state-snapshot | parser:pending dispatch:pending state:pending snapshot:pending | missing | wezterm docs/escape-sequences.md | Companion to DECIC; DECLRMM-aware |
| `DECPRES-DECBI` | xterm ctlseqs.txt §DECBI | `` `ESC 6` `` | Back index — insert blank column at left margin or move cursor left | MISSING — to be added by Section 09A | state-snapshot | parser:pending dispatch:pending state:pending snapshot:pending | missing | wezterm docs/escape-sequences.md | ESC sequence (NOT CSI); dispatch in `crates/vte/src/ansi/dispatch/mod.rs`; at column 0 inserts blank column and scrolls right |
| `DECPRES-DECFI` | xterm ctlseqs.txt §DECFI | `` `ESC 9` `` | Forward index — insert blank column at right margin or move cursor right | MISSING — to be added by Section 09A | state-snapshot | parser:pending dispatch:pending state:pending snapshot:pending | missing | wezterm docs/escape-sequences.md | ESC sequence (NOT CSI); companion to DECBI; at rightmost column inserts blank column and scrolls left |
| `DECPRES-DECRQPSR` | xterm ctlseqs.txt §DECRQPSR | `` `CSI Ps $ w` `` | Request presentation state report (cursor info or tab stops) | MISSING — to be added by Section 09A | effect-pty-write | parser:pending dispatch:pending effect:pending | missing | wezterm docs/escape-sequences.md | Ps=1 cursor info report, Ps=2 tab-stop report; reply is a DCS stream; stub reply acceptable for initial verification |
| `DECPRES-DECRQUPSS` | xterm ctlseqs.txt §DECRQUPSS | `` `CSI & u` `` | Request user-preferred supplemental character set identifier | MISSING — to be added by Section 09A | effect-pty-write | parser:pending dispatch:pending effect:pending | missing | wezterm docs/escape-sequences.md | Reply format per DEC STD 070; constant reply (ISO Latin-1) acceptable for initial verification |
| `DECPRES-DECRQDE` | xterm ctlseqs.txt §DECRQDE | `` `CSI " v` `` | Request displayed extent (current grid rows and columns) | MISSING — to be added by Section 09A | effect-pty-write | parser:pending dispatch:pending effect:pending | missing | wezterm docs/escape-sequences.md | Reply: `CSI Pn;Pn " w` with current grid dimensions |
| `DECPRES-DECSCL` | xterm ctlseqs.txt §DECSCL | `` `CSI Pl;Pc " p` `` | Set terminal conformance level (VT100/VT200/VT300) | MISSING — to be added by Section 09A | effect-mode-state | parser:pending dispatch:pending state:pending | missing | wezterm docs/escape-sequences.md | Pl=1 VT100, Pl=2 VT200, Pl=3 VT300; Pc selects 7-bit or 8-bit C1 mode; triggers a soft reset |
| `DECPRES-DECSCA` | xterm ctlseqs.txt §DECSCA | `` `CSI Ps " q` `` | Select character protection attribute for subsequent writes | MISSING — to be added by Section 09A | effect-mode-state | parser:pending dispatch:pending state:pending | missing | wezterm docs/escape-sequences.md | Ps=0 or 2 unprotected, Ps=1 protected; flag stored per-cell in CellFlags; consumed by DECSERA/DECERA |
| `DECPRES-DECSASD` | xterm ctlseqs.txt §DECSASD | `` `CSI Ps $ }` `` | Select active status display (main or status line) | MISSING — to be added by Section 09A | effect-mode-state | parser:pending dispatch:pending state:pending | missing | wezterm docs/escape-sequences.md | Ps=0 main display (default), Ps=1 status line; status line not implemented — stub acceptable |
| `DECPRES-DECSSDT` | xterm ctlseqs.txt §DECSSDT | `` `CSI Ps $ ~` `` | Select status line type (off/indicator/host-writable) | MISSING — to be added by Section 09A | effect-mode-state | parser:pending dispatch:pending state:pending | missing | wezterm docs/escape-sequences.md | Ps=0 off (default), Ps=1 indicator, Ps=2 host-writable; stub acceptable |
| `DECPRES-DECRQSS` | xterm ctlseqs.txt §DECRQSS | `` `DCS $ q Pt ST` `` | Request status string for the CSI/DCS function named by Pt | MISSING — to be added by Section 09A | effect-pty-write | parser:pending dispatch:pending effect:pending | missing | wezterm docs/escape-sequences.md | DCS path — dispatch in `crates/vte/src/ansi/dispatch/dcs.rs` (NOT csi.rs); reply: `DCS 1 $ r Pt ST` for recognized Pt, `DCS 0 $ r ST` for unrecognized |
| `DECPRES-DECRSPS` | xterm ctlseqs.txt §DECRSPS | `` `DCS Ps $ t Pt ST` `` | Restore presentation state previously reported by DECRQPSR | MISSING — to be added by Section 09A | state-snapshot | parser:pending dispatch:pending state:pending | missing | wezterm docs/escape-sequences.md | DCS path — dispatch in `crates/vte/src/ansi/dispatch/dcs.rs` (NOT csi.rs); complex serialization; stub acceptable |
