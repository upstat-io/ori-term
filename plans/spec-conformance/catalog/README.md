---
schema_version: "0.1-provisional"
title: "Spec Conformance Catalog"
owner_sections: "01 (bootstrap — this stub), 04.7 (schema freeze extension)"
---

# Spec Conformance Catalog

This directory is the SSOT for every protocol sequence ori_term targets. Each markdown file is a single protocol family with one row per sequence (expanded — one row per supported SGR parameter, OSC number, DEC private mode, etc.).

**Ownership:** Section 01 (Catalog Bootstrap) owns this stub and the initial row population. Section 04.7 extends this file below the "Schema evolution" boundary with the frozen (1.0) schema reference, after the pilots in Section 04.5/04.6 + the deterministic golden lane in Section 05.6 have landed.

**Schema version:** all catalog files currently declare `schema_version: "0.1-provisional"`. Section 04.7 flips every file to `schema_version: "1.0"` in lockstep.

## Catalog files

| File | Stack | Primary owner | Verification section |
|---|---|---|---|
| `ecma-48.md` | ECMA48 | Section 01 | Section 08 (ECMA-48 Baseline) |
| `xterm-ctlseqs.md` | XT | Section 01 | Section 08 (ECMA-48 Baseline) |
| `dec-private-modes.md` | DEC | Section 01 | Section 09 (DEC Private Modes full) |
| `osc.md` | OSC | Section 01 | Section 10 (OSC Suite full) |
| `sixel.md` | SIXEL | Section 01 | Section 12 (Sixel) |
| `kitty-graphics.md` | KG | Section 01 | Section 13 (Kitty Graphics Protocol) |
| `kitty-keyboard.md` | KKBD | Section 01 | Section 17 (Kitty Keyboard Protocol) |
| `iterm2.md` | ITERM2 | Section 01 | Section 14 (iTerm2 Inline Images) |
| `mode-2026.md` | M2026 | Section 01 | Section 06 (Terminal Mode Plumbing) |
| `unicode-subcell.md` | USC | Section 01 | Section 11 (Unicode Subcell Glyphs) |
| `mouse.md` | MOUSE | Section 01 | Section 16 (Mouse Protocols) |
| `charsets.md` | CHSET | Section 01 | Section 18 (Charsets + UAX Policy) |
| `audio-print.md` | AUDIO | Section 01 | Section 20 (Audio + Print) |
| `shell-integration.md` | SHINT | Section 01 | Section 10 (OSC Suite full) |
| `historical.md` | HIST | Section 01 | Sections 19 + 26 |
| `de-facto-behaviors.md` | DFCT | Section 01 | Sections 08 + 15 |
| `_legacy-tack-mapping.md` | — (traceability map) | Section 02.4 (tack absorption) | Section 23 (cross-stack regression sweep) |

## Authority ladder (one-line recap)

Catalog rows cite `Spec source` in preference order — see `plans/spec-conformance/00-overview.md §Authority Ladder` for the full rules:

1. **Primary spec** — the protocol's authoritative document (ECMA-48, DEC STD 070, Kitty graphics protocol doc, XTerm ctlseqs, Unicode Annex, etc.)
2. **Published extension** — a terminal emulator project's publicly documented extension (Final Term OSC 8, Kitty keyboard, iTerm2 proprietary)
3. **De-facto behavior** — reference-impl agreement without a written spec (ITU T.416 colon-separated SGR, urxvt)
4. **NEVER** — peer implementation source code is NEVER a `Spec source`. wezterm / alacritty / ghostty belong in the `De-facto ref` column only.

## Verification status (schema 0.1-provisional)

| Status | Meaning |
|---|---|
| `missing` | Dispatch arm does not exist; sequence is not parsed or reaches a log-and-drop |
| `stub` | Dispatch arm exists but the handler is a no-op (empty body or log-only), OR `Term` does not override a `Handler` default impl |
| `implemented-unverified` | Handler performs the expected mutation/effect but no verification-chain test pins the behavior |
| `verified-partial` / `verified` / `verified-with-deviation` | **FORBIDDEN in schema 0.1-provisional.** Section 01 never bootstraps these statuses; they are earned by Sections 04–20 via the verification chain harness. |

## Catalog row schema (provisional — full freeze in 04.7)

See `plans/spec-conformance/00-overview.md §Catalog Row Schema` for the authoritative 10-column schema. The column order is: `ID`, `Spec source`, `Sequence`, `Description`, `Implementation`, `Apex layer`, `Test chain`, `Verification`, `De-facto ref`, `Notes`. The `Implementation` column uses stable symbols as the primary anchor; file paths are metadata; line numbers, when present, are regenerated metadata that ride on the file path.

## Schema evolution

*Section 04.7 extends this file below this boundary with the frozen schema reference (after the pilots land). Do NOT rewrite the content above this line — Section 04.7's additions go strictly below.*
