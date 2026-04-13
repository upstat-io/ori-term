---
schema_version: "1.0"
title: "Spec Conformance Catalog"
owner_sections: "01 (bootstrap — this stub), 04.7 (schema freeze extension)"
---

# Spec Conformance Catalog

This directory is the SSOT for every protocol sequence ori_term targets. Each markdown file is a single protocol family with one row per sequence (expanded — one row per supported SGR parameter, OSC number, DEC private mode, etc.).

**Ownership:** Section 01 (Catalog Bootstrap) owns this stub and the initial row population. Section 04.7 extends this file below the "Schema evolution" boundary with the frozen (1.0) schema reference, after the pilots in Section 04.5/04.6 + the deterministic golden lane in Section 05.6 have landed.

**Schema version:** all catalog files declare `schema_version: "1.0"`. Frozen by Section 04.7 on 2026-04-13.

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

## Verification status

| Status | Meaning |
|---|---|
| `missing` | Dispatch arm does not exist; sequence is not parsed or reaches a log-and-drop |
| `stub` | Dispatch arm exists but the handler is a no-op (empty body or log-only), OR `Term` does not override a `Handler` default impl |
| `implemented-unverified` | Handler performs the expected mutation/effect but no verification-chain test pins the behavior |
| `verified-partial` | Some rungs pass but not all applicable rungs to the row's apex. Set by Sections 04–20 via the verification chain harness. |
| `verified` | All applicable rungs pass to the row's declared apex. Set by Sections 04–20 via the verification chain harness. |
| `verified-with-deviation` | All rungs pass but behavior intentionally deviates from spec (documented in Notes). Set by Sections 04–20. |

## Catalog row schema

See `plans/spec-conformance/00-overview.md §Catalog Row Schema` for the authoritative 10-column schema. The column order is: `ID`, `Spec source`, `Sequence`, `Description`, `Implementation`, `Apex layer`, `Test chain`, `Verification`, `De-facto ref`, `Notes`. The `Implementation` column uses stable symbols as the primary anchor; file paths are metadata; line numbers, when present, are regenerated metadata that ride on the file path. See the Frozen Schema Reference below for the full column definitions.

## Schema evolution

*Section 04.7 extends this file below this boundary with the frozen schema reference (after the pilots land). Do NOT rewrite the content above this line — Section 04.7's additions go strictly below.*

---

## Frozen Schema Reference (v1.0) — Section 04.7

*Added by Section 04.7 after both pilots (04.5 sixel visual + 04.6 DA1 non-visual) passed green and Section 05's deterministic golden lane validated end-to-end. Frozen on 2026-04-13.*

### Canonical Row Format

Each catalog file contains a single markdown table. The column order is fixed — tools (the citation scanner, the coverage report) depend on positional stability:

| # | Column | Type | Description |
|---|---|---|---|
| 1 | `ID` | `STACK-SEQ-VARIANT` | Unique row identifier. Format: `{STACK}-{SEQUENCE}[-{VARIANT}]`. The citation scanner greps test files for this ID. |
| 2 | `Spec source` | citation | Primary spec reference per the authority ladder (see above). NEVER a peer implementation. |
| 3 | `Sequence` | code | The byte sequence in backtick-escaped inline code. |
| 4 | `Description` | prose | What the sequence does. |
| 5 | `Implementation` | code | Stable symbol path: `Handler::method` or `Performer::hook` → `Term::handler`. File paths are metadata; line numbers are regenerated. |
| 6 | `Apex layer` | enum | The deepest rung the verification chain drives for this row (see ApexLayer below). |
| 7 | `Test chain` | rung:status | Per-rung status. Format: `parser:done dispatch:done ...` or `parser:pending`. |
| 8 | `Verification` | enum | Overall verification status (see below). |
| 9 | `De-facto ref` | citation | Peer implementation reference (wezterm, alacritty, ghostty, etc.). Column name is `De-facto ref` — NOT `De-facto reference` (SSOT: `00-overview.md` §Catalog Row Schema). |
| 10 | `Notes` | prose | Implementation notes, caveats, cross-references. |

### ApexLayer Values

Matches `oriterm_test_support::spec_chain::ApexLayer`:

| Value | Rung chain | Use for |
|---|---|---|
| `parser-only` | Parser | Tokenization-only sequences |
| `dispatch` | Parser → Dispatch | Routing verification |
| `state` | Parser → Dispatch → State | Terminal mutation |
| `renderable` | ... → Renderable | Content extraction |
| `frame-input` | ... → FrameInput | Grid assembly |
| `gpu-instance` | ... → GpuInstance | Instance buffer |
| `texture-render` | ... → TextureRender | Pixel output |
| `golden-image` | ... → GoldenImage | Golden comparison (visual apex) |
| `effect-pty-write` | Parser → Dispatch → Effect | PTY reply (DA1, DSR, mouse) |
| `effect-clipboard` | Parser → Dispatch → Effect | OSC 52 clipboard |
| `effect-host-title` | Parser → Dispatch → Effect | OSC 0/2 title change |
| `effect-mode-state` | Parser → Dispatch → Effect | DECSET/DECRST mode |
| `effect-presentation-commit` | Parser → Dispatch → Effect | Mode 2026 |
| `effect-audio` | Parser → Dispatch → Effect | DECPS / OSC audio |
| `effect-host-notification` | Parser → Dispatch → Effect | OSC 9/99/777 |

### RungName Values

Matches `oriterm_test_support::spec_chain::RungName`:

| Rung | # | Observer location |
|---|---|---|
| `Parser` | 1 | `oriterm_test_support::spec_chain::observers::parser` |
| `Dispatch` | 2 | `oriterm_test_support::spec_chain::observers::dispatch` |
| `State` | 3a | `oriterm_test_support::spec_chain::observers::state` |
| `Effect` | 3b | `oriterm_test_support::spec_chain::observers::effect` |
| `Renderable` | 4 | `oriterm_test_support::spec_chain::observers::renderable` |
| `FrameInput` | 5 | `oriterm::gpu::visual_regression::spec_chain::observers::frame_input` |
| `GpuInstance` | 6 | `oriterm::gpu::visual_regression::spec_chain::observers::gpu_instance` |
| `TextureRender` | 7 | `oriterm::gpu::visual_regression::spec_chain::observers::texture` |
| `GoldenImage` | 8 | `oriterm::gpu::visual_regression::spec_chain::observers::golden` |

### Verification Status Values

| Status | Meaning | Who sets it |
|---|---|---|
| `missing` | No dispatch arm; sequence is not parsed or reaches log-and-drop | Section 01 bootstrap |
| `stub` | Dispatch arm exists but handler is a no-op (empty or log-only) | Section 01 bootstrap |
| `implemented-unverified` | Handler performs the expected mutation/effect but no verification-chain test pins behavior | Section 01 bootstrap |
| `verified-partial` | Some rungs pass but not all applicable rungs to the row's apex | Sections 04–20 |
| `verified` | All applicable rungs pass to the row's declared apex | Sections 04–20 |
| `verified-with-deviation` | All rungs pass but behavior intentionally deviates from spec (documented in Notes) | Sections 04–20 |

### Adding a New Row

1. Identify the protocol family → pick the catalog file
2. Choose an `ID` following the `STACK-SEQ-VARIANT` pattern
3. Fill all 10 columns (use `—` for empty optional columns)
4. Set `Verification` to `missing`, `stub`, or `implemented-unverified`
5. Set `Apex layer` to the deepest rung the implementation supports
6. Set `Test chain` to `parser:pending dispatch:pending ...` for each applicable rung

### Migrating a Row to `verified`

1. Write a `SpecScenario` test that drives through the row's declared apex
2. Include `catalog_row_id: "THE-ROW-ID"` in the scenario (citation scanner requirement)
3. Run the test through the harness — all rungs must pass
4. Update `Test chain` to `parser:done dispatch:done ...`
5. Update `Verification` to `verified` (or `verified-with-deviation` with a Notes entry)
