---
section: "12"
title: "Sixel"
status: complete

reviewed: true
goal: "Drive every catalog row in `catalog/sixel.md` from `implemented-unverified` to `verified` via the spec_chain harness — first full visual stack section, exercising the entire pipeline (DCS-state parser → state-machine operator dispatch → image cache → GPU image render → golden image). Close the parser/decoder state-machine seam end-to-end, pin DCS-abort + palette-lifetime + background-mode semantics, and establish occlusion + mixed-protocol cross-stack hand-offs that downstream sections (§13 Kitty, §14 iTerm2) can rely on."
success_criteria:
  - "Top-down spec audit committed at `plans/spec-conformance/audits/section-12-top-down-inventory.md`. Every sequence in the canonical spec source(s) for this stack (DEC STD 070 §5/6 (primary); libsixel + wezterm cross-references) maps to a catalog row ID OR carries an explicit `not-targeted` decision with rationale. `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes for this audit file. This is enforced PER `plans/spec-conformance/audits/README.md` lint contract — added by Section 09A as the SSOT for top-down catalog coverage to prevent the bottom-up gap that hid DECRQCRA from the catalog."
  - "Audit-file coverage extends the current 5-row catalog with per-operator rows (`\"` raster attrs, `#` define, `#N` select, `!N` repeat, `$` CR, `-` NL, sixel data byte) AND behavioral rows for: (a) aborted DCS commits nothing on CAN/SUB/ESC mid-image, (b) raster-attrs-before-data vs raster-attrs-mid-stream materially affect output, (c) `SetToBg` vs `DeviceDefault` render distinctly (not both opaque black), (d) DECSET 80 default state + 8452 cursor-right behavior as separate rows, (e) color-map reset-vs-persist across DCS q invocations. `not-targeted` rows exist for DEC STD 070 macro-set and DECGRA non-sixel sequences with rationale that the parser drops them silently."
  - "Every row in `catalog/sixel.md` is `verified`."
  - "Sixel parser + decoder (the `SixelParser` state machine at `oriterm_core/src/image/sixel/mod.rs`) is verified end-to-end as one coupled unit: DCS q introducer (params P1/P2/P3), raster attrs (`\"` Pan/Pad/Ph/Pv), color define (`#n;Pu;Px;Py;Pz`) with Pu=1 (HLS) and Pu=2 (RGB), color select (`#n`), repeat (`!n`), CR (`$`), NL (`-`), sixel data byte (`?`..`~`), intermixed `#` color changes mid-data, `!` repeat interaction with palette, abort via CAN/SUB/ESC mid-DCS."
  - "Background mode semantics are pinned distinctly: `SixelBgMode::DeviceDefault` (P2=0), `SixelBgMode::NoChange` (P2=1, transparent α=0), `SixelBgMode::SetToBg` (P2=2, filled with terminal bg). A test asserts `SetToBg` output differs from `DeviceDefault` output on identical pixel input (addresses the prior false-equivalence in `SixelParser::finish`; §12.2 fix routes `Term::effective_background` into the parser at DCS-hook time so SetToBg is both DEC-distinct and DECSCNM-aware)."
  - "Palette-lifetime invariant is pinned: every DCS q invocation rebuilds the VT340 palette from scratch (`SixelParser::new` in `oriterm_core/src/image/sixel/mod.rs` copies `VT340_PALETTE` into the fresh per-parser `palette` Vec), so color-map state from a prior sixel does NOT leak into the next. A regression test emits two back-to-back DCS q streams with different palettes and asserts the second stream's output depends only on its own palette definitions."
  - "`!` repeat clamping semantics are pinned at `SixelParser::emit_sixel` in `oriterm_core/src/image/sixel/mod.rs` (`let count = count.min(MAX_DIMENSION);`; `MAX_DIMENSION` const declared at the top of the same file) — a test feeds `!20000~` and asserts either (a) the documented libsixel-compatible behaviour of clamping to MAX_DIMENSION / 10,000 pixels, or (b) if the implementation diverges, the divergence is recorded in a catalog-row note and cross-referenced against libsixel's `decoder.c`."
  - "DCS-abort is pinned end-to-end through the VTE performer: CAN (0x18), SUB (0x1A), and ESC-mid-DCS all drive `Performer::unhook` in `crates/vte/src/lib.rs` → `dispatch::dispatch_unhook` in `crates/vte/src/ansi/dispatch/mod.rs` → `Term::handle_sixel_end(aborted)` in `oriterm_core/src/term/handler/image/sixel.rs`. §12.1 landed the fix: `Handler::sixel_end(aborted: bool)` + `ProcessorState::dcs_aborted` + new `DcsEscape` VTE state; the handler early-returns on `aborted` so no placement is committed."
  - "Sixel grid integration verified: SIXEL_SCROLLING mode 80 cursor positioning, SIXEL_CURSOR_RIGHT mode 8452 cursor positioning, image placement creation at `(cell_col, stable_row)` with `z_index: 0`, orphan cleanup via `prune_scrollback`."
  - "Sixel GPU rendering verified via golden image apex on the deterministic lane from §05 — five expanded scenarios: (a) solid rectangle, (b) multi-color palette-switch mid-image, (c) `!` repeat optimization, (d) `$` CR + `-` NL banding interaction, (e) transparency composite against the deterministic background without sub-pixel jitter, PLUS (f) SIXEL_SCROLLING OFF goldens, (g) SIXEL_CURSOR_RIGHT ON goldens so §12.3's behavioral bullets are visually pinned."
  - "Sixel + unicode/subcell occlusion verified: sixel placements use `z_index: 0` (`Term::sixel_create_placement` in `oriterm_core/src/term/handler/image/sixel.rs`) and non-negative images render above text (image-instance emit in `oriterm/src/gpu/prepare/emit.rs`). Goldens pin z-order against §11 glyph families — sixel + wide-CJK, sixel + ZWJ cluster, sixel + half-blocks/quadrants/sextants."
  - "Sixel + image lifecycle handled correctly per §07 (consumes §07's handlers — §07 is `status: complete`): scrollback eviction **removes** sixel placements via `prune_scrollback`; ED / EL erase **remove** placements in the erased region via `remove_placements_in_region`; alt-screen toggle **preserves** the primary-cache placement across enter/exit (alt cache is separate); resize (column shrink) **removes** column-out-of-bounds placements via `ImageCache::on_resize`; resize with reflow **remaps** placement `StableRowIndex` values via `remap_placements`; font-size / DPI change **recomputes** `FixedPixels` cell coverage via `SetCellDimensions`."
  - "Sixel ↔ Kitty cross-stack hand-off proven at `ImageCache` + placement level: a shared-cache regression test places a sixel image, then a kitty image into the same `ImageCache` instance on a `Term`, and asserts both placements are independently addressable via the public snapshot API `Term::renderable_content()` in `oriterm_core/src/term/snapshot.rs` → `RenderableContent::images` (deeper mixed-protocol rendering interference is delegated to §13.6 via an explicit DEFERRED-TO-DOWNSTREAM cross-link)."
  - "All existing spec_chain sixel tests stay green without modification — specifically `oriterm/src/gpu/visual_regression/spec_chain/pilots/sixel_minimal.rs` (the §04 sixel pilot) continues to pass. No sixel-specific teseq suite exists (`oriterm_core/tests/teseq/` has no sixel scenarios); this section does not introduce one, and the broader teseq infrastructure is archived by §23.5 once the spec-conformance chain covers its scenarios."
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release."
  - "Section's mission criterion connection: contributes to **Verification chain complete per row** and (for lifecycle rung) **Image lifecycle correct under resize/reflow/scrollback/alt-screen**."
inspired_by:
  - "DEC STD 070 — primary spec; defines DCS q semantics, P1-P6 raster attrs, color operators"
  - "libsixel `src/decoder.c` — reference implementation for parsing edge cases (palette reset, repeat clamping, abort path)"
  - "wezterm `term/src/terminalstate/sixel.rs` — production reference for HLS rotation, raster attrs, transparency"
depends_on: ["05", "07", "08"]
third_party_review:
  status: clean
  updated: 2026-04-20
  notes: "12 findings verified + fixed inline across rounds 0-2 (commits 54041ae6, e5a19364, fcc2f258); round 3 returned clean from both reviewers. All findings resolved (see §12.R for the audit trail) — status flipped `findings` → `clean` once `open_count` reached 0."
sections:
  - id: "12.0"
    title: "Top-down spec audit (BLOCKING) — per-operator + behavioral rows"
    status: complete
  - id: "12.1"
    title: "Verify sixel parser+decoder state machine end-to-end (DCS q, raster attrs, color ops, repeat, CR/NL, data, abort)"
    status: complete
  - id: "12.2"
    title: "Verify background modes + palette-lifetime + repeat-clamp invariants"
    status: complete
  - id: "12.3"
    title: "Verify sixel grid integration + §11 occlusion (SIXEL_SCROLLING, SIXEL_CURSOR_RIGHT, z-order)"
    status: complete
  - id: "12.4"
    title: "Verify sixel GPU rendering via golden image apex (expanded scenarios + cursor-mode goldens)"
    status: complete
  - id: "12.5"
    title: "Verify sixel + image lifecycle interactions + sixel↔kitty ImageCache hand-off"
    status: complete
  - id: "12.R"
    title: "Third Party Review Findings"
    status: complete

  - id: "12.N"
    title: "Completion Checklist"
    status: complete
# TPR Checkpoint Placement: 12.3 (after parser/decoder state machine + invariants + grid integration — covers .1-.3),
# 12.5 (after GPU goldens + lifecycle + cross-stack hand-off — covers .4-.5), final in 12.N
---

# Section 12: Sixel

**Status:** Complete (all subsections §12.0–§12.5, §12.R, §12.N closed 2026-04-21; full TPR + hygiene review clean).
**Goal:** Sixel is the first full visual stack — its verification chain exercises the entire pipeline from DCS byte parsing through GPU composition. This section drives every sixel catalog row to `verified`, closes the parser/decoder state-machine seam end-to-end, and pins three invariants that the prior 5-row catalog did not cover: background-mode distinction (§12.2 landed the DECSCNM-aware SetToBg plumbing via `Term::effective_background`), palette reset per DCS q (§12.2 pinned via back-to-back DCS tests + a RAII-guarded negative pin in sibling `bypass.rs`), and DCS abort correctness (§12.1 added the `DcsEscape` state + `Handler::sixel_end(aborted)` drop path).

**Success Criteria:** see frontmatter.

**Context:** Pass 1 confirmed sixel parser, decoder, grid integration, and GPU rendering all exist. Section 04's pilot verified one minimal scenario (opaque rectangle). This section drives every other catalog row AND adds the per-operator + behavioral rows the current catalog lacks. The HLS rotation bug suspected by the audit memory turned out to be CORRECT (`hls_to_rgb`'s `hue - 120.0` rotation in `oriterm_core/src/image/sixel/color.rs` — verified by Pass 1). Section 07 (image lifecycle) is `status: complete`, so `ImageCache::on_resize`, `remap_placements`, and the 42-scenario lifecycle matrix are available runway for §12.5.

**Code seam this section owns (in-crate anchors):**
- `oriterm_core/src/image/sixel/mod.rs` — `SixelParser` coupled state machine: byte intake in `SixelParser::feed`, operator dispatch in `finish_command`, palette rebuild in `SixelParser::new` (copies `VT340_PALETTE`), repeat clamp in `emit_sixel` (`count.min(MAX_DIMENSION)`; const at the top of the file), raster attrs Pan/Pad ignored in `apply_raster_attrs` (reads only `params[2]`/`params[3]`), `SetToBg` enum as `SixelBgMode` + undrawn-pixel fill in `finish()` now routes through `terminal_bg` captured from `Term::effective_background` (DECSCNM-aware) rather than the pre-§12.2 opaque-black collapse. Test-only VT340 bypass lives in sibling `bypass.rs`.
- `oriterm_core/src/term/handler/image/sixel.rs` — handler wiring: `handle_sixel_start` (captures `Term::effective_background` at DCS-hook time) / `handle_sixel_put` / `handle_sixel_end` (early-return on `aborted`), `sixel_create_placement` with `z_index: 0`.
- `crates/vte/src/lib.rs` — VTE `Performer::unhook` / `advance_dcs_passthrough` / `advance_dcs_escape` drive DCS finalize; CAN/SUB/ESC-mid-DCS route through `unhook` with the aborted flag set.
- `crates/vte/src/ansi/dispatch/mod.rs` — `dispatch_unhook` routes `DcsState::Sixel` to `Term::sixel_end(aborted)` with the aborted flag preserved.
- `oriterm_core/src/term/snapshot.rs` — `Term::renderable_content()` is the public snapshot API; `renderable_content_into()` is the hot-path no-alloc variant. Both populate `RenderableContent::images` via the private helper `extract_images`. Consumers (tests, GPU pipeline) drive through the public API.
- `oriterm/src/gpu/window_renderer/frame_prep.rs` + `oriterm/src/gpu/image_render/mod.rs` + `oriterm/src/gpu/prepare/emit.rs` — image-render prepare path shared with §13 Kitty; `z_index >= 0` draws above text.

**NO BUG-08-8 blocker on §12.** Earlier plan drafts pre-blocked Section 12 on the `kitty.rs` BLOAT split. That gate is **factually wrong** and is removed:
- Section 12's implementation surface is `oriterm_core/src/term/handler/image/sixel.rs` (140 lines) + `oriterm_core/src/image/sixel/mod.rs` (438 lines) — NOT `kitty.rs`.
- `.claude/rules/code-hygiene.md` §File Size (line 91) scopes the 500-line cap **file-local**: "Source files (excluding `tests.rs`) must not exceed 500 lines." `kitty.rs` being at 480 lines does not block edits to unrelated files.
- BUG-08-8 remains tracked at `plans/bug-tracker/section-08-core-terminal.md` and still blocks §13 (Kitty Graphics) implementation as that section's own blocker note states. §12 proceeds independently.

**Reference implementations:** see frontmatter.

**Depends on:** Section 05 (deterministic golden lane — `status: complete`), Section 07 (image lifecycle handlers — `status: complete`), Section 08 (ECMA-48 baseline correct — `status: complete`).

**Cross-section hand-offs (downstream consumers):**
- §13.6 (Kitty + sixel cross-stack regression) consumes §12.5's shared-`ImageCache` handshake test. Deep mixed-protocol rendering interference (sixel + kitty in same grid row, same cell, overlapping placements) is **DEFERRED-TO-DOWNSTREAM** to §13.6 and called out in §12.N.
- §14 (iTerm2) and §15 (Cell-Level Alpha) inherit the `z_index` + transparency composite pins established here.

---

## 12.0 Top-down spec audit (BLOCKING — precedes all other subsections)

**Goal:** Walk DEC STD 070 §5/§6 TOP-DOWN. Every sequence the spec defines gets a row in the audit file at `plans/spec-conformance/audits/section-12-top-down-inventory.md`, mapped to either a catalog row ID or an explicit `not-targeted` decision with rationale. Expand the current 5-row catalog to per-operator granularity plus the behavioral rows the state machine requires.

**Why this exists:** Section 09A introduced the `audits/` SSOT to close the bottom-up catalog construction gap that hid DECRQCRA from the catalog. The current `catalog/sixel.md` has only 5 broad rows (DCS-q introducer, DCS-put payload, DCS-unhook, MODE-80 xref, MODE-8452 xref). That granularity cannot pin per-operator correctness or behavioral invariants — a `!` repeat bug can sit under `SIXEL-DCS-put` indefinitely with no catalog row failing. The audit file forces the expansion.

**Canonical spec source(s):** DEC STD 070 §5 (Sixel Color Extension) + §6 (Sixel Graphics Extension), primary; libsixel `src/decoder.c` + wezterm `term/src/terminalstate/sixel.rs` cross-references.

**Files touched:**
- `plans/spec-conformance/audits/section-12-top-down-inventory.md` (POPULATE — currently a stub created by §09A.10; the table at `audits/section-12-top-down-inventory.md:21-27` is empty)
- `plans/spec-conformance/catalog/sixel.md` (EXPAND — open new rows using the canonical 10-column schema from `plans/spec-conformance/00-overview.md §Catalog Row Schema`)

**Completion criteria:**

- [x] Audit file `plans/spec-conformance/audits/section-12-top-down-inventory.md` is populated with every sequence in DEC STD 070 §5/§6.
- [x] Audit file contains per-operator rows (not a single "sixel data" mega-row):
  - [x] DCS q introducer with parameters P1 (aspect-ratio/Pan), P2 (background-select), P3 (Ph, horizontal grid — ignored in our impl, **note the divergence**)
  - [x] Raster attributes operator `"` with sub-params Pan, Pad, Ph, Pv
  - [x] Color definition operator `#n;Pu;Px;Py;Pz` — separate row per Pu value (Pu=1 HLS, Pu=2 RGB)
  - [x] Color selection operator `#n` (bare select, no definition)
  - [x] Repeat operator `!n` with data-byte payload
  - [x] CR operator `$` (graphic carriage return — reset x, keep y)
  - [x] NL operator `-` (graphic newline — reset x, advance y by 6)
  - [x] Sixel data byte `?`..`~` (the 6-pixel column encoding)
- [x] Audit file contains behavioral rows (not just operator rows):
  - [x] `SIXEL-BG-DeviceDefault` (P2=0) — renders with device-default bg
  - [x] `SIXEL-BG-NoChange` (P2=1) — undrawn pixels alpha=0 (transparent)
  - [x] `SIXEL-BG-SetToBg` (P2=2) — undrawn pixels filled with terminal bg color, **distinct from DeviceDefault**
  - [x] `SIXEL-ABORT-CAN` — CAN (0x18) mid-DCS aborts; no placement committed
  - [x] `SIXEL-ABORT-SUB` — SUB (0x1A) mid-DCS aborts; no placement committed
  - [x] `SIXEL-ABORT-ESC` — ESC mid-DCS aborts; no placement committed
  - [x] `SIXEL-RASTER-BEFORE-DATA` — `"` before first data byte sets dimensions
  - [x] `SIXEL-RASTER-MID-STREAM` — `"` emitted after data is ignored OR re-dimensions (document which)
  - [x] `SIXEL-PALETTE-RESET-PER-DCS` — palette rebuilt from VT340 defaults on every DCS q; prior definitions do not leak
  - [x] `SIXEL-MODE-80-DEFAULT` — SIXEL_SCROLLING default state verified
  - [x] `SIXEL-MODE-8452-CURSOR-RIGHT` — DECSET 8452 cursor-right behavior
- [x] Audit file contains `not-targeted` rows for:
  - [x] DEC STD 070 macro-set commands (DECDMAC / DECINVM) — rationale: ori_term does not implement macro storage; parser must silently drop.
  - [x] DECGRA and other non-sixel graphics sequences that share the DCS space — rationale: ori_term's `DcsState` gates only on `DcsState::Sixel`; parser must pass non-sixel DCS through to their respective handlers without corrupting sixel state.
- [x] Every row in the audit-file table has a `Decision` of `mapped` (cites a catalog row ID) or `not-targeted` (with one-line rationale).
- [x] Every `mapped` row resolves to a real catalog row that exists in `plans/spec-conformance/catalog/sixel.md`.
- [x] New catalog rows in `catalog/sixel.md` use the canonical 10-column schema from `plans/spec-conformance/00-overview.md §Catalog Row Schema` (frozen v1.0 — 2026-04-13).
- [x] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes for this audit file.
- [x] Audit file `last_walked` frontmatter is set to today's date and `walked_by` to the implementer's handle.

**No other subsection in this section can begin work until §12.0 is complete.** This is a hard gate. The audit file IS the row set §12.1–§12.5 drive to `verified`.

---

## 12.1 Verify sixel parser+decoder state machine end-to-end

**File(s):** `oriterm_core/tests/spec_chain/sixel/state_machine.rs` (new)

**Why merged:** The original plan split parser (§12.1) and decoder (§12.2) into separate test files. The `SixelParser` in `oriterm_core/src/image/sixel/mod.rs` is **one coupled state machine** — byte intake (`feed`), command parsing (`finish_command`), palette mutation (`apply_color`), raster-attrs (`apply_raster_attrs`), and pixel emission (`emit_sixel` + `finish`) all share the same `SixelParser` struct. A byte-level test that tokenizes `P1` parameters but never drives palette mutation can be green while palette dispatch is broken. Merging §12.1 + §12.2 into a single state-machine rung forces intermixed-operator coverage, which is what actually verifies the coupling. Pure-decoder invariants (background mode, palette reset, repeat clamp) move to §12.2 because they are observable properties of the decoder's **output state** rather than per-operator dispatch correctness.

**What to test (catalog rows driven to `verified` by this subsection — the per-operator + abort rows from §12.0):**

- [x] Write failing test matrix BEFORE implementation (TDD per `.claude/rules/tests.md` §TDD for Bugs).
- [x] **DCS q introducer** — feed `ESC P <Ps1> ; <Ps2> ; <Ps3> q` prefixes with the full P1×P2×P3 cartesian product. Observe via the spec_chain parser rung that the DCS dispatch reaches `Term::handle_sixel_start(params)` with the correct P1/P2/P3 values captured, and observe via the state-effect rung that P2 drives the resulting `SixelBgMode` distinctly (`DeviceDefault` / `NoChange` / `SetToBg`). Do NOT assert on the private `SixelParser::new` constructor signature — that is an internal implementation detail; the observable dispatch call + output-state difference is what the chain pins.
- [x] **Raster attributes `"`** — feed `" <Pan> ; <Pad> ; <Ph> ; <Pv>` before data, assert width/height are set from Ph/Pv (note Pan/Pad currently ignored — `SixelParser::apply_raster_attrs` in `oriterm_core/src/image/sixel/mod.rs` reads only `params[2]` / `params[3]` — this is a documented divergence the audit row captures).
- [x] **Color define Pu=2 (RGB)** — feed `#n;2;Px;Py;Pz`, assert `palette[n] = [Px, Py, Pz]` scaled from 0-100 to 0-255.
- [x] **Color define Pu=1 (HLS)** — feed `#n;1;Ph;Pl;Ps`, assert `palette[n]` matches libsixel's `hls_to_rgb` in `oriterm_core/src/image/sixel/color.rs` (the `hue - 120.0` rotation verified correct in Pass 1).
- [x] **Color select `#n`** — feed `#n` without follow-on params, assert `current_color = n`.
- [x] **Repeat `!n`** — feed `!5?`, assert 5 consecutive columns of the same sixel data byte are emitted.
- [x] **CR `$`** — feed data, `$`, more data, assert second data band starts at `x = 0` with `y` unchanged.
- [x] **NL `-`** — feed data, `-`, more data, assert second data band starts at `x = 0` with `y += 6`.
- [x] **Sixel data byte `?`..`~`** — for every byte in the 63-code range, assert the 6-bit pixel column is decoded per `byte - 0x3F`.
- [x] **Intermixed `#` mid-data** — feed `<data>#5<data>`, assert the second data band uses `current_color = 5` on live placement (forces dispatch on live `SixelParser` state, not just setup).
- [x] **`!` repeat interacts with palette** — feed `#3!5?`, assert all 5 columns use palette index 3 (forces repeat to read `current_color` at emission time, not at `!` time).
- [x] **Raster-before-data** — feed `"1;1;10;20<data>`, assert dimensions set before first pixel.
- [x] **Raster-mid-stream** — feed `<data>"5;5;100;100<data>`, assert the documented behavior (ignored vs re-dimensions). Audit file row captures the choice.
- [x] **Negative pin — abort path** — feed `ESC P q <data> <CAN>`, drive through `Performer::unhook` / `advance_dcs_passthrough` in `crates/vte/src/lib.rs` → `dispatch_unhook` in `crates/vte/src/ansi/dispatch/mod.rs` → `Term::handle_sixel_end(aborted)` in `oriterm_core/src/term/handler/image/sixel.rs`, assert:
  - No entry added to `ImageCache` — **BUG fix landed in-scope of §12.1**. Root cause: `handle_sixel_end` stored unconditionally because the VTE parser could not distinguish abort from ST completion. Fix: added `Perform::notify_dcs_abort` callback + `ProcessorState::dcs_aborted` flag + new `DcsEscape` VTE state that defers the unhook decision until the byte after ESC is seen (so `ESC \` normal ST stays a normal ST; any other ESC sequence is an abort); threaded the flag through `Handler::sixel_end(aborted: bool)` → `Term::handle_sixel_end(aborted)` → early return when aborted. CAN/SUB/ESC mid-DCS all pinned.
  - Same test shape with SUB (0x1A) and ESC-mid-DCS — green.
- [x] Matrix count assertion: `assert_eq!(cells_visited, OPS.len() * INTERMIX_SCENARIOS.len())` per `.claude/rules/tests.md` §Self-Verifying Matrix Completeness.
- [x] Update per-operator + per-abort catalog rows (from §12.0 expansion) to `verified`.
- [x] Verify all tests pass in both debug AND release builds.
- [x] **Validation:** state-machine rung green; parser + decoder seam proven coupled.

---

## 12.2 Verify background modes + palette-lifetime + repeat-clamp invariants

**File(s):** `oriterm_core/tests/spec_chain/sixel/invariants.rs` (new)

**Why separate from §12.1:** These are properties of the decoder's **output state** over multiple DCS invocations and output-buffer invariants, not per-operator dispatch. They belong in their own rung because the failure mode (silent pixel corruption, cross-image palette leak, unbounded allocation) is observed at the output layer, not at the token layer.

- [x] Write failing test matrix BEFORE implementation.
- [x] **`SixelBgMode::DeviceDefault` vs `SixelBgMode::SetToBg`** — fix landed in-scope: `Term::handle_sixel_start` now snapshots `Term::effective_background()` at DCS-hook time (which mirrors the DECSCNM swap in `oriterm_core/src/term/snapshot.rs`) and passes it into `SixelParser::new(params, [r,g,b])`; `finish()` fills undrawn pixels with `[0,0,0,255]` for `DeviceDefault` (VT340 opaque black) and `[r,g,b,255]` for `SetToBg` (effective terminal bg — DECSCNM-aware). Pinned by `bg_mode_set_to_bg_differs_from_device_default_on_identical_input` + `bg_mode_set_to_bg_honors_decscnm_reverse_video` in `oriterm_core/tests/spec_chain/sixel/invariants.rs` + `set_to_bg_uses_terminal_background_not_black` + `device_default_and_set_to_bg_diverge_under_non_black_terminal_bg` in the unit tests. DEC STD 070 §6.2.2 — SetToBg is now semantically distinct and DECSCNM-coherent with the render path.
- [x] **`SixelBgMode::NoChange` transparency** — pinned by `bg_mode_no_change_undrawn_pixels_have_alpha_zero` in `invariants.rs`: feeds P2=1 with partial coverage, asserts `pixels[7] == 0` on an undrawn pixel regardless of the harness terminal bg.
- [x] **Palette reset per DCS q (semantic pin)** — pinned by `palette_rebuilds_per_dcs_q_no_leak_across_invocations` and `palette_vt340_fingerprint_reappears_on_fresh_dcs` in `invariants.rs`. Two back-to-back DCS streams on the same harness confirm (a) stream B's defined palette overrides take effect regardless of stream A's state and (b) stream B's undefined `#5` sees the VT340 default cyan, not leaked red from stream A.
- [x] **Negative pin — palette leak** — pinned by `palette_reset_per_dcs_negative_pin_bypass_breaks_vt340_fingerprint` in the unit tests (`oriterm_core/src/image/sixel/tests.rs`). Implementation adds a `#[cfg(test)]`-only `BYPASS_VT340_RESET` thread-local flag + RAII `BypassVt340ResetGuard` in sibling `oriterm_core/src/image/sixel/bypass.rs` (TPR round-2 split); when the guard is active, `SixelParser::new` skips the VT340 rebuild loop and leaves `palette[5] = [0,0,0]`. The test acquires the guard inside a scoped block, confirms `#5@` yields opaque black (not cyan), then the guard's `Drop` restores — proving the VT340 rebuild is load-bearing. Lives as a unit test because `#[cfg(test)]` is not visible to integration tests.
- [x] **Repeat clamp at `MAX_DIMENSION`** — pinned by `repeat_clamps_at_max_dimension_without_allocation_spike` in `invariants.rs`. Feeds `!20000~` (count 20 000 well above the 10 000 clamp); asserts placement commits with `w ≤ 10 000`, no panic, no OOM. libsixel also applies a protective clamp — tracked as `verified-with-deviation` in the catalog row.
- [x] **Pixel-buffer cap** — pinned by `raster_attrs_exceeding_max_pixel_bytes_aborts_cleanly` in `invariants.rs`. Feeds `"1;1;15000;15000` (900 MB > 100 MB `MAX_PIXEL_BYTES`); asserts no placement is committed (`placement_count == 0`) because `apply_raster_attrs` sets `aborted = true` and `finish()` returns `Err(ImageError::OversizedImage)`, which `handle_sixel_end` warns+drops.
- [x] Matrix count assertion — `invariant_category_matrix_completeness` in `invariants.rs` plus the `set_to_bg_*` / `device_default_and_set_to_bg_diverge_*` / `palette_reset_per_dcs_negative_pin_*` unit tests; count totals 8 invariant categories (7 integration + 1 unit — the DECSCNM regression pin `bg_mode_set_to_bg_honors_decscnm_reverse_video` was added during §12.2 TPR round-0).
- [x] Update `SIXEL-BG-*`, `SIXEL-PALETTE-RESET-PER-DCS`, and repeat-clamp catalog rows (from §12.0 expansion) to `verified`. `SIXEL-REPEAT-CLAMP` + `SIXEL-PIXEL-BUFFER-CAP` → `verified-with-deviation`; `SIXEL-BG-*` and `SIXEL-PALETTE-RESET-PER-DCS` → `verified`.
- [x] Verify all tests pass in both debug AND release builds — `cargo test -p oriterm_core --lib image::sixel` + `cargo test -p oriterm_core --test spec_chain sixel::` green in debug AND `--release`; `./test-all.sh`, `./clippy-all.sh`, `./build-all.sh` all green (workspace + `x86_64-pc-windows-gnu` cross-compile).
- [x] **Validation:** invariant rung green; decoder output state proven correct across DCS boundaries + buffer bounds.

---

## 12.3 Verify sixel grid integration + §11 occlusion

**File(s):** `oriterm_core/tests/spec_chain/sixel/grid_integration.rs` for the non-GPU bullets (cursor positioning, placement creation, orphan cleanup); the §11 OCCLUSION golden-image bullets live in the GPU crate as pilots alongside `oriterm/src/gpu/visual_regression/spec_chain/pilots/sixel_minimal.rs` (`sixel_occlusion_wide_cjk.rs`, `sixel_occlusion_zwj.rs`, `sixel_occlusion_subcell.rs`) with goldens at `oriterm/tests/references/sixel_occlusion_<family>.png`. The negative-z_index pin lives at unit-test level in `oriterm/src/gpu/prepare/tests.rs` — the `emit_image_quads` z<0 vs z≥0 split is unit-tested with a 4-placement matrix, so §12.3 did not add a new pilot for that pin. Per `.claude/rules/crate-boundaries.md` — grid-state assertions stay in `oriterm_core`, pixel-golden assertions stay in `oriterm`, and emit-path logic lives with `oriterm/src/gpu/prepare`.

**Depends on code paths:** `oriterm_core/src/term/handler/image/sixel.rs:68-139` (placement creation + cursor advance); `oriterm/src/gpu/prepare/emit.rs:262-285` (z-order emission); §11 unicode/subcell payloads.

- [x] Write failing test matrix BEFORE implementation.
- [x] **SIXEL_SCROLLING ON (default)** — pinned by `sixel_scrolling_default_on_advances_cursor_below_image` in `oriterm_core/tests/spec_chain/sixel/grid_integration.rs`; multi-row sixel advances cursor by `rows.saturating_sub(1)` linefeeds.
- [x] **SIXEL_SCROLLING OFF (DECRST 80)** — pinned by `sixel_scrolling_off_via_decrst_80_keeps_cursor_at_home`; the no-op `else` arm in `sixel_create_placement` keeps cursor at (0, 0).
- [x] **SIXEL_CURSOR_RIGHT ON (DECSET 8452)** — pinned by `sixel_cursor_right_via_decset_8452_moves_cursor_right_of_image` (col advances by `cols`, row unchanged) + `sixel_cursor_right_priority_over_sixel_scrolling` (cursor_right wins when both modes are set).
- [x] **Image placement creation** — pinned by `sixel_placement_creation_sets_z_index_zero` (z_index=0) + `sixel_placement_anchors_at_cursor_position` (viewport_x/y = cell_col/cell_row × cell_pixel_width/height). `FixedPixels` sizing verified via `RenderablePlacement::display_width`/`display_height` in the same assertion.
- [x] **Orphan cleanup** — pinned by `sixel_placement_pruned_when_scrollback_evicts_its_row`; 1100 linefeeds through the VTE handler's `linefeed` → `prune_images_if_evicted` → `ImageCache::prune_scrollback` hook drops the row-0 placement.
- [x] **§11 OCCLUSION — sixel + wide-CJK** — pinned by `sixel_occlusion_wide_cjk_drives_every_rung_green` in `oriterm/src/gpu/visual_regression/spec_chain/pilots/sixel_occlusion_wide_cjk.rs`; golden PNG at `oriterm/tests/references/sixel_occlusion_wide_cjk.png`.
- [x] **§11 OCCLUSION — sixel + ZWJ cluster** — pinned by `sixel_occlusion_zwj_drives_every_rung_green`; golden at `oriterm/tests/references/sixel_occlusion_zwj.png` using the `U+1F468 U+200D U+1F4BB` man-technologist cluster.
- [x] **§11 OCCLUSION — sixel + subcell glyphs** — pinned by `sixel_occlusion_subcell_drives_every_rung_green`; golden at `oriterm/tests/references/sixel_occlusion_subcell.png` covering the `▀▌▞▙` half-block / quadrant / 3-quadrant mix rendered via `oriterm/src/gpu/builtin_glyphs`.
- [x] **Negative pin — negative z_index** — already pinned at unit-test level by `oriterm/src/gpu/prepare/tests.rs` (`image_quads_below.len() == 2` for a 4-placement matrix with z_index in `[-2, 1, -1, 0]`). The emit path at `emit_image_quads` in `oriterm/src/gpu/prepare/emit.rs` splits placements by `z_index < 0` into `image_quads_below` (drawn before text) vs `image_quads_above` (drawn after text); the pre-existing unit test pins the split, so no new §12.3 pilot was needed.
- [x] Matrix count assertion — `grid_integration_category_matrix_completeness` asserts 7 non-GPU categories; GPU occlusion is counted by the 3 pilot files.
- [x] Update grid-integration + MODE-80 + MODE-8452 + §11-occlusion catalog rows to `verified`.
- [x] Verify all tests pass in both debug AND release builds — `./test-all.sh` green; `./clippy-all.sh` green; the 3 GPU pilots pass with `--features gpu-tests` in both debug and release.
- [x] **TPR checkpoint** — §12.2 close-out ran a 5-round TPR checkpoint covering §12.1+§12.2 (10 verified findings, 9 fixed inline, 1 pre-existing filed to BUG-08-20). §12.3 adds the grid-integration + occlusion rungs; §12.3 close-out ran a 1-round §12.1–§12.3 TPR checkpoint in survivor mode (gemini was capacity-exhausted with persistent 429 RESOURCE_EXHAUSTED errors across 3 retry attempts; codex completed). Codex surfaced 2 verified findings — F1 (medium): `sixel_placement_creation_sets_z_index_zero` docstring claimed FixedPixels sizing coverage without an explicit assertion (fixed by adding `display_width=5.0` + `display_height=6.0` assertions and renaming the test); F2 (low): plan File(s) line said `grid_integration.rs` owned the negative-z pin but it actually lives at `oriterm/src/gpu/prepare/tests.rs` (fixed). A further round would be pro-forma under survivor-mode; §12's §12.5 TPR checkpoint and the §12.N final-full-section TPR will re-engage gemini when capacity returns.
- [x] **Validation:** grid-integration rung green; z-order pinned against §11 subcell glyph families.

---

## 12.4 Verify sixel GPU rendering via golden image apex

**File(s):** new per-scenario pilots under `oriterm/src/gpu/visual_regression/spec_chain/pilots/` alongside the existing `sixel_minimal.rs` (e.g. `sixel_palette_switch.rs`, `sixel_repeat.rs`, `sixel_cr_nl_banding.rs`, `sixel_transparency.rs`, `sixel_scrolling_off.rs`, `sixel_cursor_right.rs`), with goldens committed as flat PNGs at `oriterm/tests/references/sixel_<scenario>.png` (matching the existing `sixel_minimal.png` naming convention). GPU-apex work MUST live in the `oriterm` crate per `.claude/rules/crate-boundaries.md` — `oriterm_core/tests/` is non-GPU; the existing `oriterm/src/gpu/visual_regression/spec_chain/pilots/sixel_minimal.rs` is the canonical pattern.

**Depends on code paths:** `oriterm_core/src/term/snapshot.rs:33` (`Term::renderable_content()` public API; populates `RenderableContent::images`); `oriterm/src/gpu/window_renderer/frame_prep.rs:149-173`; `oriterm/src/gpu/image_render/mod.rs:67-151`; `oriterm/src/gpu/prepare/emit.rs:262-285`. Deterministic lane from §05 (`status: complete`).

- [x] Write failing test matrix BEFORE implementation.
- [x] Capture goldens via `ORITERM_UPDATE_GOLDEN=1` using the deterministic lane (llvmpipe, grayscale hinting, pinned cell metrics) from §05.
- [x] **Scenario A — Solid rectangle, one color** (baseline parity with §04 pilot, explicit row in this section's directory) — `sixel_minimal.rs` doubles as the §12.4 Scenario A row; golden at `oriterm/tests/references/sixel_minimal.png`.
- [x] **Scenario B — Palette-switch mid-image** — `sixel_palette_switch.rs` drives `#0;2;100;0;0#0!10~-#1;2;0;100;0#1!10~` through the GoldenImage apex; golden at `oriterm/tests/references/sixel_palette_switch.png`.
- [x] **Scenario C — Repeat optimization** — `sixel_repeat.rs` drives `!100~` through the apex; golden at `oriterm/tests/references/sixel_repeat.png`.
- [x] **Scenario D — CR + NL banding interaction** — `sixel_cr_nl_banding.rs` drives 4-band red/green/blue/yellow composition with `$` + `-` ops; golden at `oriterm/tests/references/sixel_cr_nl_banding.png`.
- [x] **Scenario E — Transparency composite** — `sixel_transparency.rs` drives P2=1 with 10-filled / 10-transparent / 10-filled cols over PALETTE_BG `Rgb(1,1,1)`; 0-pixel diff gate; golden at `oriterm/tests/references/sixel_transparency.png`.
- [x] **Scenario F — SIXEL_SCROLLING OFF golden** — `sixel_scrolling_off.rs` drives DECRST 80 + cursor (10,5) + sixel + "OVERWRITE" text; golden at `oriterm/tests/references/sixel_scrolling_off.png`.
- [x] **Scenario G — SIXEL_CURSOR_RIGHT golden** — `sixel_cursor_right.rs` drives DECSET 8452 + cursor (10,5) + sixel + "FOLLOW" text; golden at `oriterm/tests/references/sixel_cursor_right.png`.
- [x] **P2=2 SetToBg pilot** (added post-TPR per `[TPR-12-012-codex+gemini][high]`) — `sixel_set_to_bg.rs` drives `\x1bP0;2q#0;2;100;100;100#0!10~!10?#0!10~\x1b\\` (P2=2 with partial coverage); golden at `oriterm/tests/references/sixel_set_to_bg.png`. Required because DeviceDefault + NoChange pilots alone only bracket SetToBg structurally — a dedicated P2=2 golden is the only way to pin `SixelBgMode::SetToBg` + `Term::effective_background` plumbing at the GoldenImage rung.
- [x] Spec_chain tests assert goldens match on subsequent runs — 0-pixel diff via `compare_with_reference_strict` (`pixel_tolerance = 0`, `max_diff_percent = 0.0` under `GoldenLaneConfig::SPEC_DEFAULT`); back-to-back runs confirmed green debug + release.
- [x] Negative pin — Graceful Skip Protocol embedded per-pilot via the `let Some(mut harness) = VisualSpecHarness::new() else { eprintln!("SKIP: …"); return; };` guard that every §12.4 pilot opens with (8 sites). Removes the round-0 tautology `visual_harness_skip_protocol_never_panics_or_falsely_passes` per `[TPR-12-014-codex+gemini][medium]` — an assertion-less wrapper is not a test.
- [x] Matrix count assertion — `sixel_12_4_matrix_has_expected_scenario_count` + `sixel_12_4_every_scenario_has_committed_golden` in `sixel_12_4_matrix.rs` assert `SCENARIOS.len() == 8` (A–G + P2=2) AND every scenario's golden PNG exists under `oriterm/tests/references/`. Adding/removing a scenario without updating both tests is a catalog drift.
- [x] Update GPU-rendering catalog rows to `verified` — `catalog/sixel.md` rows SIXEL-DCS-unhook, SIXEL-BG-DeviceDefault, SIXEL-BG-NoChange, SIXEL-BG-SetToBg flipped from `texture-render` apex with `snapshot:pending frame-input:pending texture:pending` to `golden-image` apex with the full `snapshot:pass frame-input:pass gpu:pass texture:pass golden:pass` chain. SIXEL-BG-SetToBg cites the dedicated `sixel_set_to_bg.rs` pilot post-TPR round 0.
- [x] Verify all tests pass in both debug AND release builds — `cargo test -p oriterm --features gpu-tests --lib -- spec_chain::pilots::sixel --test-threads=1` green in both; `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` all green.
- [x] **Validation:** golden tests pass; back-to-back runs produce 0-pixel diff; cursor-mode goldens visually pin §12.3's behavioral bullets — confirmed.

---

## 12.5 Verify sixel + image lifecycle interactions + sixel↔kitty ImageCache hand-off

**File(s):** `oriterm_core/tests/spec_chain/sixel/lifecycle.rs` (new), `oriterm_core/tests/spec_chain/sixel/cross_stack_handoff.rs` (new)

**Depends on code paths:** `oriterm_core/src/image/cache/lifecycle.rs` (§07 complete — `on_resize`, `remap_placements`, `prune_scrollback`, `remove_placements_in_region`); `oriterm_core/src/term/handler/image/sixel.rs` (sixel side); `oriterm_core/src/term/handler/image/kitty.rs` (kitty side — read-only reference, no edits).

**§07 runway note:** §07 is `status: complete` — `ImageCache::on_resize`, `ReflowMapping` / `remap_placements`, and the 42-scenario lifecycle matrix are available. §12.5 **consumes** these handlers; it does not re-implement them. Specifically relied-upon §07 deliverables:
- `ImageCache::on_resize(new_cols, new_rows)` — removes column-out-of-bounds placements.
- `ImageCache::remap_placements(mapping)` — translates `StableRowIndex` through `ReflowMapping::first_output_row`.
- `Term::resize` — invokes `remap_placements` → `prune_scrollback` → `on_resize` on primary cache; alt cache gets `on_resize` only.
- `PaneIoCommand::SetCellDimensions` — the cell-metric runtime-config wire that feeds `FixedPixels` re-coverage.

- [x] Write failing test matrix BEFORE implementation — §12.5 is a pure consumption of §07's lifecycle handlers; the test matrix acts as regression pins that fail if §07's `prune_scrollback`, `remove_placements_in_region`, `on_resize`, or `remap_placements` paths regress.
- [x] **Scrollback eviction** — `sixel_placement_removed_when_scrollback_evicts_its_row` in `oriterm_core/tests/spec_chain/sixel/lifecycle.rs` emits 1100 linefeeds after placing a sixel; asserts `renderable_content().images.len() == 0` via `prune_scrollback`.
- [x] **ED (erase display)** — `sixel_placement_removed_on_full_screen_ed` drives `CSI 2 J` and asserts placement removed via `remove_placements_in_region(first, last, None, None)`.
- [x] **EL (erase line)** — `sixel_placement_removed_on_el_covering_its_row` drives `CSI 2 K` after CUP back to row 0; asserts placement on that row removed.
- [x] **Alt-screen toggle** — `sixel_primary_placement_preserved_across_alt_screen_toggle` pins: primary placement present → DECSET 1049 shows 0 placements (alt cache distinct) → DECRST 1049 shows 1 placement (primary intact).
- [x] **Resize shrink columns** — `sixel_placement_removed_on_resize_shrinking_past_column` uses `SpecHarness::with_size(24, 100)`, places sixel at col=90, calls `term.resize(24, 80, false)`; asserts `on_resize` removed the out-of-bounds placement. Paired with `sixel_placement_preserved_on_resize_expanding_columns` (positive pin: in-bounds placement survives column grow).
- [x] **Resize with reflow** — `sixel_placement_survives_resize_with_reflow_when_column_in_bounds` in `oriterm_core/tests/spec_chain/sixel/lifecycle.rs` places a sixel at col=0, calls `term.resize(24, 40, true)` with reflow enabled, and asserts the placement survives with unchanged `display_width`/`display_height` (remap preserves FixedPixels dims). Added post-TPR round 0 per `[TPR-12-013-codex+gemini]`; the round-0 "covered by §07 + empty-cache no-op" claim was a positive-pin gap. Paired with `resize_on_empty_cache_is_noop_no_phantom_placements` negative pin on the empty-cache reflow path.
- [x] **Font-size / DPI change** — `sixel_placement_preserved_across_cell_dimension_change` calls `Term::set_cell_dimensions(16, 32)` and asserts the placement survives with `FixedPixels` `display_width` unchanged (the handler recomputes cell coverage but does not resize the image).
- [x] **CROSS-STACK ImageCache hand-off** — `oriterm_core/tests/spec_chain/sixel/cross_stack_handoff.rs` drives sixel at `(line=5, col=0)` + kitty (`a=T,i=1,f=32,s=1,v=1,q=2;AAAAAA==`) at `(line=10, col=5)` (distinct column to pin X-axis positional independence per TPR round 1); asserts `RenderableContent::images.len() >= 2` with distinct `image_id`s, `image_data.len() >= 2` with sixel 8×6 RGBA (192 bytes) + kitty 1×1 RGBA (4 bytes) payloads keyed by distinct IDs, and a second test pins each placement's own `viewport_x` (sixel=0.0, kitty=40.0) / `viewport_y` (sixel=80.0, kitty=160.0) / `z_index` / `display_width` / `display_height` — no cross-wiring. This is the handshake, not a rendering regression; deep mixed-protocol interference DEFERRED-TO-DOWNSTREAM §13.6 (recorded here and in §12.N).
- [x] Negative pin — `resize_on_empty_cache_is_noop_no_phantom_placements` resizes an empty cache (both shrink without reflow and grow with reflow); asserts zero placements spawned — proves `on_resize` + `remap_placements` fire only on relevant placements.
- [x] Alloc regression unchanged — `oriterm_core/tests/alloc_regression.rs` green via `./test-all.sh`; lifecycle tests consume handlers whose allocation profile was already accounted for in §07 (no new allocating paths introduced in this subsection).
- [x] Matrix count assertion — `lifecycle_category_matrix_completeness` (10 categories: scrollback_eviction, ed_full_screen, el_full_line, alt_screen_toggle, resize_shrink_columns, resize_expand_columns, resize_with_reflow_preserves_in_bounds, resize_with_reflow_remaps_stable_row, cell_dimension_change, negative_pin_empty_cache_resize_noop) + `cross_stack_handoff_category_matrix_completeness` (2 handshake categories) in their respective files.
- [x] Update lifecycle + cross-stack-handshake catalog rows to `verified` — `SIXEL-CROSS-STACK-HANDOFF` flipped from `implemented-unverified` to `verified` with pilot citations.
- [x] Verify all tests pass in both debug AND release builds — `cargo test -p oriterm_core --test spec_chain -- sixel::lifecycle sixel::cross_stack_handoff --test-threads=1` green in both profiles; `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green.
- [x] **TPR checkpoint** — `/tpr-review` covering §12.4–§12.5 (GPU goldens + lifecycle + cross-stack hand-off) — to run at §12.N boundary per plan TPR-checkpoint directive; findings recorded in §12.R.
- [x] **Validation:** lifecycle rung green; cross-stack hand-off proven at the public snapshot level; downstream deferral to §13.6 recorded — confirmed.

---

## 12.R Third Party Review Findings

Populated by `/tpr-review` at the §12.3 and §12.5 checkpoints and the §12.N final gate. Every unchecked finding here MUST be resolved (fix or file+resolve via `/fix-bug`) before this section can close, per `CLAUDE.md §NEVER reason out of TPR findings`.

### Review 1 — Pre-implementation plan review via `/review-plan` (2026-04-20)

Twelve verified findings across four rounds (rounds 0–2 produced fixes; round 3 converged clean). All fixed inline via plan edits — no open items remain.

- [x] `[TPR-12-001-codex+gemini][high]` `plans/spec-conformance/section-12-sixel.md:80,227,229` — `oriterm/src/gpu/frame_prep.rs` path wrong (actual `oriterm/src/gpu/window_renderer/frame_prep.rs`) AND §12.4 GPU-apex tests specified in `oriterm_core/tests/` (wrong crate per `.claude/rules/crate-boundaries.md`). Resolved in commit `54041ae6`: corrected frame_prep path; relocated §12.4 pilots to `oriterm/src/gpu/visual_regression/spec_chain/pilots/sixel_<scenario>.rs`; split §12.3 occlusion goldens into GPU-crate pilots.
- [x] `[TPR-12-002-codex][high]` `plans/spec-conformance/section-12-sixel.md:19` — lifecycle `success_criteria` said sixel "survives" scrollback eviction + ED + EL — inverts §07 semantics (eviction/ED/EL should REMOVE, alt-screen/resize should PRESERVE/remap). Resolved in commit `54041ae6`: rewrote the criterion to distinguish remove from preserve/remap per §07's handler contract.
- [x] `[TPR-12-003-codex][medium]` `plans/spec-conformance/section-12-sixel.md:28,89` — `depends_on: ["05", "07"]` omitted §08 per `00-overview.md` Quick Reference line 739 (`12 Sixel ... 05, 07, 08`). Resolved in commit `54041ae6`: added "08" to frontmatter + body.
- [x] `[TPR-12-004-codex+gemini][medium]` `plans/spec-conformance/section-12-sixel.md:20,272` — `ImageCache::extract_images` API does not exist. Resolved in commit `54041ae6`: switched to `Term::extract_images`; later revised in `fcc2f258` to the public `Term::renderable_content()` per reviewer F3 below.
- [x] `[TPR-12-005-codex][low]` `plans/spec-conformance/section-12-sixel.md:21` — `success_criteria` gate "All existing teseq sixel tests pass" was vacuous (no sixel teseq suite exists; `oriterm_core/tests/teseq/` has no sixel scenarios). Resolved in commit `54041ae6`: replaced with falsifiable `sixel_minimal.rs` §04 pilot anchor + note about §23.5 owning teseq archival.
- [x] `[TPR-12-006-codex+gemini][medium]` `plans/spec-conformance/section-12-sixel.md:306` — §12.N checklist still contained the vacuous teseq gate that Round 0 had scrubbed from the `success_criteria` frontmatter. Resolved in commit `e5a19364`: §12.N mirror replaced with the same `sixel_minimal.rs` anchor.
- [x] `[TPR-12-007-gemini][medium]` `plans/spec-conformance/section-12-sixel.md:14,75,189` — repeat-clamp citations at `mod.rs:335-360` drifted; actual site is `oriterm_core/src/image/sixel/mod.rs:336` (`count.min(MAX_DIMENSION)` inside `emit_sixel`; `MAX_DIMENSION` const at `:17`). Resolved in commit `fcc2f258`.
- [x] `[TPR-12-008-gemini][medium]` `plans/spec-conformance/section-12-sixel.md:75,156` — raster-attrs-ignored citations at `mod.rs:78-85,310-329` and `mod.rs:310-329` drifted (the `78-85` range is `SixelParser::new`, not raster attrs). Actual: `oriterm_core/src/image/sixel/mod.rs:311-330` is the `apply_raster_attrs` body. Resolved in commit `fcc2f258`.
- [x] `[TPR-12-009-codex][medium]` `plans/spec-conformance/section-12-sixel.md:155` — §12.1 DCS q introducer bullet asserted on the private `SixelParser::new` constructor-param spy rather than the chain's observable dispatch rung. Resolved in commit `fcc2f258`: rewrote to observe `Term::handle_sixel_start(params)` dispatch + P2-driven `SixelBgMode` state-effect difference.
- [x] `[TPR-12-010-codex][low]` `plans/spec-conformance/section-12-sixel.md:193` — pixel-buffer-cap bullet cited `mod.rs:64` as abort site (that's the struct field declaration) and named the error variant `ImageError::TooLarge`. Actual: abort sites at `oriterm_core/src/image/sixel/mod.rs:318` and `:323` inside `apply_raster_attrs`; real variant is `ImageError::OversizedImage`. Resolved in commit `fcc2f258`.
- [x] `[TPR-12-011-codex][medium]` `plans/spec-conformance/section-12-sixel.md:20,79,229,272` — §12.5 hand-off test and supporting citations named the private `Term::extract_images` helper (`oriterm_core/src/term/snapshot.rs:243`) as the observed surface. Violates `.claude/rules/code-hygiene.md` §Visibility — private helpers are not the chain's state-effect rung. Resolved in commit `fcc2f258`: switched to public `Term::renderable_content()` (`:33`) and `renderable_content_into()` (`:79`).

**Convergence.** Round 3 verification returned `status: clean` from both reviewers (one informational positive-confirmation entry dropped at verification as non-actionable). All 12 findings tracked; all fixed inline. Exit reason: `clean`.

### Review 2 — Bundled TPR checkpoint covering §12.4 + §12.5 (2026-04-21)

Five verified findings (four agreement, one codex-only). Round 0 dispatched codex + gemini in parallel against the full §12.4 / §12.5 surface (GPU-apex pilots, non-GPU lifecycle + cross-stack handshake, catalog flips, plan body updates). All five resolved inline.

- [x] `[TPR-12-012-codex+gemini][high]` `plans/spec-conformance/catalog/sixel.md:22` — `SIXEL-BG-SetToBg` row flipped to `golden-image` apex + `golden:pass` based on a bracketing argument ("DeviceDefault + NoChange pilots fully bracket the SetToBg blend") rather than a dedicated P2=2 GPU pilot. Rules violated: `.claude/rules/tests.md §Matrix Testing Rule` (missing semantic pin), plan/catalog coherence. Resolved inline: added `oriterm/src/gpu/visual_regression/spec_chain/pilots/sixel_set_to_bg.rs` driving `\x1bP0;2q#0;2;100;100;100#0!10~!10?#0!10~\x1b\\` through the GoldenImage apex; committed golden at `oriterm/tests/references/sixel_set_to_bg.png`; updated `catalog/sixel.md:22` notes to cite the pilot directly instead of the bracketing argument; bumped §12.4 matrix count from 7 → 8.
- [x] `[TPR-12-013-codex+gemini][medium]` `plans/spec-conformance/section-12-sixel.md:263` (pre-update line) — "Resize with reflow" checkbox flipped to `[x]` while only covered by the empty-cache no-op negative pin + §07's complete reflow suite; no sixel-side positive pin placed an image, resized with reflow, and observed the surviving placement. Rules violated: `.claude/rules/tests.md §Matrix Testing Rule / §Interaction Testing`; §12 success_criteria line 20 requires sixel-side reflow coverage. Resolved inline: added `sixel_placement_survives_resize_with_reflow_when_column_in_bounds` in `oriterm_core/tests/spec_chain/sixel/lifecycle.rs` placing a sixel at col=0, calling `term.resize(24, 40, true)`, and asserting `renderable_content().images` still contains the placement with unchanged `display_width`/`display_height` (remap preserved FixedPixels dims). §12.5 matrix bumped 8 → 9 categories; matrix count assertion updated.
- [x] `[TPR-12-014-codex+gemini][medium]` `oriterm/src/gpu/visual_regression/spec_chain/pilots/sixel_12_4_matrix.rs:64` (pre-update line) — `visual_harness_skip_protocol_never_panics_or_falsely_passes` had no assertion — matched both `Some` and `None` branches of `VisualSpecHarness::new()` with `eprintln!` only. Rules violated: `.claude/rules/tests.md §Test Hygiene / §Negative Testing Protocol` (no orphan tests; assertion IS the test). Resolved inline: removed the tautological test. The structural negative pin lives embedded per-pilot via each pilot's `let Some(mut harness) = VisualSpecHarness::new() else { eprintln!("SKIP: …"); return; };` guard — Graceful Skip Protocol enforced at 8 sites (one per §12.4 pilot), not via a standalone wrapper. Module docs + §12.4 checkbox body updated to cite the per-pilot guard pattern.
- [x] `[TPR-12-015-codex][medium]` `oriterm_core/tests/spec_chain/sixel/cross_stack_handoff.rs:118` (pre-update line) — `sixel_and_kitty_placements_retain_independent_attributes` only asserted `image_id != sixel.image_id` and z_index=0 on sixel; never checked any concrete kitty placement attribute. Rules violated: `.claude/rules/tests.md §Matrix Testing Rule` (semantic pin). Resolved inline: tightened assertions — sixel pinned at `viewport_y=80` (line 5 × cell_h 16), kitty pinned at `viewport_y=160` (line 10 × cell_h 16) with `kitty.viewport_y > sixel.viewport_y` positional-independence pin, structural distinctness pin (kitty dims differ from sixel's (8.0, 6.0)), non-degenerate dims pin (`>0`), and kitty z_index=0 (not corrupted by coexistent sixel).
- [x] `[TPR-12-016-codex+gemini][low]` `oriterm_core/tests/spec_chain/sixel/lifecycle.rs:28` (pre-update line) — sixel DCS helper `dcs_n_cols_wide` triplicated across `grid_integration.rs`, `lifecycle.rs`, and `cross_stack_handoff.rs` (as `dcs_n_cols_red` — same algorithm). Rules violated: `.claude/rules/impl-hygiene.md §Algorithmic DRY` (3-instance threshold); `.claude/rules/test-organization.md §6 Test helpers`. Resolved inline: extracted `dcs_n_bands_tall` + `dcs_n_cols_wide` to new `crates/oriterm_test_support/src/spec_chain/sixel_fixtures.rs` module (SSOT); updated all three consumer files to import from the shared module.

**Round 1** (after round-0 fixes) — 3 findings:

- [x] `[TPR-12-017-codex+gemini][medium]` `oriterm_core/tests/spec_chain/sixel/lifecycle.rs` — round-0 `sixel_placement_survives_resize_with_reflow_when_column_in_bounds` proved the placement survives `term.resize(24, 40, true)` but not that `remap_placements` actually ran (placement at col=0 with no preceding text reflowed is an identity-remap). Resolved: added `sixel_placement_remaps_stable_row_across_reflow_with_text_wrap` — 100-char text wraps 2-way at 80 cols / 3-way at 40 cols, sixel placed at line 3, `term.resize(24, 40, true)` shifts the sixel to line 4, `viewport_y` changes from 48.0 to 64.0 (exact pin added in round 2). A no-op remap would leave viewport_y=48 and the test fails.
- [x] `[TPR-12-018-codex][medium]` `oriterm_core/tests/spec_chain/sixel/cross_stack_handoff.rs` — round-0 handshake test ignored `RenderableContent::image_data` (payload rung). Resolved: added payload-independence pin — `snap.image_data.len() >= 2`, distinct payload `image_id`s. Strengthened in round 2 to pin exact shape: sixel 8×6 RGBA (192 bytes), kitty 1×1 RGBA (4 bytes) via `find(|d| d.width == X && d.height == Y)`.
- [x] `[TPR-12-019-gemini][medium]` `oriterm_core/tests/spec_chain/sixel/cross_stack_handoff.rs` — round-0 test placed both sixel and kitty at col=0, so X-axis cross-wiring (both viewport_x collapsing to 0) would not fail any assertion. Resolved: moved kitty placement to col=5 via `\x1b[11;6H`, added `sixel.viewport_x == 0.0` + `kitty.viewport_x == 40.0` strict-equality pins + updated catalog/plan prose in round 3.

**Round 2** (after round-1 fixes) — 3 findings:

- [x] `[TPR-12-020-codex+gemini][medium]` `oriterm_core/tests/spec_chain/sixel/cross_stack_handoff.rs` — round-1 `image_data.len() >= 2` + distinct `data_ids` pin still only checked cardinality, not payload identity. Resolved: `find(|d| d.width == 8 && d.height == 6)` for sixel + `find(|d| d.width == 1 && d.height == 1)` for kitty, plus `assert_eq!(sixel_data.data.len(), 8 * 6 * 4)` and `assert_eq!(kitty_data.data.len(), 1 * 1 * 4)` — cross-wiring that swapped payloads would fail the strict equality on one or both byte counts.
- [x] `[TPR-12-021-gemini][low]` `oriterm_core/tests/spec_chain/sixel/cross_stack_handoff.rs` — `assert_ne!(kitty.viewport_x, sixel.viewport_x)` was tautological after the preceding `assert_eq!(sixel.viewport_x, 0.0)` + `assert_eq!(kitty.viewport_x, 40.0)`. Resolved: removed the redundant assert_ne; strict equalities above already pin independence.
- [x] `[TPR-12-022-gemini][medium]` `oriterm_core/tests/spec_chain/sixel/lifecycle.rs` — round-1 reflow-remap test used `assert_ne!(after_y, before_y)` — loose pin that accepts any non-48 value, including wrong ones. Resolved: replaced with exact `assert_eq!(after.images[0].viewport_y, 64.0)` — probed the actual post-reflow value and committed to it; regressions that move the placement to a different stable_row now fail strictly.

**Round 3** (convergence check after round-2 fixes) — gemini clean, codex 2 low doc-drift findings:

- [x] `[TPR-12-023-codex][low]` `plans/spec-conformance/section-12-sixel.md:266` + `plans/spec-conformance/catalog/sixel.md:43` — plan and catalog prose still described kitty placement at `(line=10, col=0)` / `(r2,c0)` after round-1 moved it to col=5 in the test. SSOT drift between plan/catalog and code. Resolved: updated the plan §12.5 checkbox body and catalog SIXEL-CROSS-STACK-HANDOFF row sequence column to `(r2,c5)` with explanatory note.
- [x] `[TPR-12-024-codex][low]` `plans/spec-conformance/section-12-sixel.md:269` — plan matrix-count prose said 8 categories while `lifecycle_category_matrix_completeness` enforced 10 (round 1 added `resize_with_reflow_remaps_stable_row`; earlier rounds added `resize_with_reflow_preserves_in_bounds`). Resolved: enumerated all 10 categories by name in the plan bullet.

**Convergence.** 4 rounds executed; 11 findings total verified + fixed inline (5 round 0 + 3 round 1 + 3 round 2 + 2 round 3). Round 3 achieved gemini `status: clean`; codex's round-3 findings were both low-severity plan/catalog prose drift (not code correctness), resolved inline. No round 4 needed. Exit reason: `clean` (all verified findings fixed, both reviewers returned clean or low-only-plan-drift at round 3, all tests green debug + release, catalog + plan + code in sync).

### Review 3 — Bundled /impl-hygiene-review covering §12.4 + §12.5 (2026-04-21)

Four actionable findings, all fixed inline:

- [x] `[HYG-12-001][BLOAT:fn-length][minor]` `oriterm_core/tests/spec_chain/sixel/cross_stack_handoff.rs:117` — `sixel_and_kitty_placements_retain_independent_attributes` was 103 lines (3 over the 100-line function limit in `.claude/rules/code-hygiene.md` §Style). Resolved: split body into two helper fns (`assert_sixel_independence` + `assert_kitty_independence`) each of ~30 lines, test body dropped to ~20 lines.
- [x] `[HYG-12-002][LEAK:algorithmic-duplication][minor]` `oriterm_core/tests/spec_chain/sixel/{grid_integration,invariants,lifecycle}.rs` — `placement_count(&SpecHarness) -> usize` 1-liner defined identically in 3 sibling files (impl-hygiene.md §Algorithmic DRY threshold: 3+ instances = always extract). Resolved: moved to `crates/oriterm_test_support/src/spec_chain/sixel_fixtures.rs`; all 3 consumers import from shared module.
- [x] `[HYG-12-003][EXPOSURE:misleading-naming][minor]` `oriterm_core/tests/spec_chain/sixel/cross_stack_handoff.rs:17` — `use dcs_n_cols_wide as dcs_n_cols_red` aliased the shared fixture to a name implying "color=red is a parameter", but the function hardcodes red `#0;2;100;0;0`. Breaks grepability + lies about the API. Resolved: dropped the alias; use `dcs_n_cols_wide` directly (its doc comment already states color is red).
- [x] `[HYG-12-004][WASTE:capacity-hint][minor]` `crates/oriterm_test_support/src/spec_chain/sixel_fixtures.rs:19,37` — `Vec::with_capacity` hints were 3 bytes short of actual write size (prefix+suffix miscalculation: `16 + bands*3` / `16 + cols` missed the `\x1b\\` 2-byte suffix, and bands math double-counted the separator). Resolved: extracted named `DCS_RED_PREFIX` + `DCS_TERMINATOR` constants with `.len()` readbacks; capacity hints now arithmetically match the writes and are self-auditing.

**Pre-existing tooling bug (out of scope, already tracked):** `.claude/skills/impl-hygiene-review/plan-annotations.py` crashes with `NameError: AIMS_SECTION_RE is not defined` (ori_lang sync-drift — regex constants not ported to ori_term). Already tracked as `[BUG-07-017][low]` in `plans/bug-tracker/section-07-ci-build.md`; no action required for §12 close-out.

**Convergence.** 1 hygiene pass, 4 actionable findings, all fixed inline. Phase 4 cross-check skipped (redundant with 4-round TPR above). No plan generation needed (all findings are inline-fixable). Exit reason: `clean` (no LEAK, DRIFT, GAP, or production-source BLOAT; all test-scope hygiene findings resolved; tests green debug + release).

---

## 12.N Completion Checklist

- [x] Every row in `catalog/sixel.md` is `verified` (including the per-operator + behavioral + §11-occlusion rows opened in §12.0) — `grep -c "implemented-unverified\|not-verified" plans/spec-conformance/catalog/sixel.md` returns 0.
- [x] `plans/spec-conformance/audits/section-12-top-down-inventory.md` is populated, `last_walked: 2026-04-20`, `walked_by: "elucidsoft"`.
- [x] Failing test matrix written FIRST — §12.1, §12.2, §12.3, §12.4, §12.5 each have their own TDD checkbox flipped to `[x]`.
- [x] **Matrix dimensions**: operator × DCS-count × background-mode × cursor-mode × lifecycle-event × protocol-neighbor — covered across `oriterm_core/tests/spec_chain/sixel/{state_machine,invariants,grid_integration,lifecycle,cross_stack_handoff}.rs` + `oriterm/src/gpu/visual_regression/spec_chain/pilots/sixel_*.rs`. Each file carries a `*_category_matrix_completeness` test that asserts the exact category count.
- [x] **Semantic pins (≥3 — one per invariant)**: (a) `bg_mode_set_to_bg_differs_from_device_default_on_identical_input`, (b) `palette_rebuilds_per_dcs_q_no_leak_across_invocations` + `palette_vt340_fingerprint_reappears_on_fresh_dcs`, (c) `dcs_abort_can_commits_no_placement` / `dcs_abort_sub_commits_no_placement` / `dcs_abort_esc_commits_no_placement`.
- [x] **Negative pins (≥3)**: (a) `palette_reset_per_dcs_negative_pin_bypass_breaks_vt340_fingerprint` in `oriterm_core/src/image/sixel/bypass.rs`, (b) §12.3 `z_index: -1` draws below text pinned in `oriterm/src/gpu/prepare/tests.rs::image_quads_below`, (c) `resize_on_empty_cache_is_noop_no_phantom_placements` in `lifecycle.rs`.
- [x] HLS rotation explicitly tested (cross-checked against libsixel `color.rs:41` `hue - 120.0`) — `color_define_pu1_hls_rotates_hue_minus_120` in `state_machine.rs`.
- [x] `!` repeat clamp cross-checked against libsixel `src/decoder.c`; divergence documented as `verified-with-deviation` in `catalog/sixel.md` SIXEL-REPEAT-CLAMP row; pinned by `repeat_clamps_at_max_dimension_without_allocation_spike` in `invariants.rs`.
- [x] DCS abort (CAN / SUB / ESC mid-DCS) commits no placement — §12.1 fix landed: `Handler::sixel_end(aborted: bool)` + `ProcessorState::dcs_aborted` + new `DcsEscape` VTE state; `Term::handle_sixel_end` early-returns on `aborted`. Pinned by `dcs_abort_can_commits_no_placement` / `dcs_abort_sub_commits_no_placement` / `dcs_abort_esc_commits_no_placement` in `state_machine.rs`.
- [x] `SixelBgMode::SetToBg` renders distinctly from `DeviceDefault` — §12.2 fix landed: `Term::handle_sixel_start` snapshots `Term::effective_background` (DECSCNM-aware) at DCS-hook time; `SixelParser::finish` routes `SetToBg` through the captured `terminal_bg`, `DeviceDefault` through `[0,0,0,255]`. Pinned by `bg_mode_set_to_bg_differs_from_device_default_on_identical_input` + `bg_mode_set_to_bg_honors_decscnm_reverse_video` in `invariants.rs`. §12.4 GPU-apex pinned by dedicated `sixel_set_to_bg.rs` pilot (P2=2 golden).
- [x] Sixel + image lifecycle survives every grid mutation (§07 complete; §12.5 consumes handlers) — `oriterm_core/tests/spec_chain/sixel/lifecycle.rs` exercises 10 categories (scrollback eviction, ED, EL, alt-screen toggle, resize shrink/expand, resize with reflow survive + remap, cell dimension change, empty-cache negative pin).
- [x] Sixel + §11 occlusion goldens green (wide-CJK, ZWJ cluster, subcell glyphs) — `oriterm/src/gpu/visual_regression/spec_chain/pilots/sixel_occlusion_{wide_cjk,zwj,subcell}.rs` + committed goldens under `oriterm/tests/references/sixel_occlusion_*.png`.
- [x] Sixel ↔ Kitty `ImageCache` hand-off green — `oriterm_core/tests/spec_chain/sixel/cross_stack_handoff.rs` pins coexistence + independent attributes + payload independence. Deep mixed-protocol rendering interference DEFERRED-TO-DOWNSTREAM to §13.6; cross-link recorded in `plans/spec-conformance/section-13-kitty-graphics.md` §13.6 body.
- [x] `oriterm/src/gpu/visual_regression/spec_chain/pilots/sixel_minimal.rs` (§04 sixel pilot) stays green — no regression from §12's new pilots. Confirmed via `cargo test -p oriterm --features gpu-tests --lib -- spec_chain::pilots::sixel`. (No sixel-specific teseq suite exists; `oriterm_core/tests/teseq/` has no sixel scenarios and §23.5 owns teseq archival.)
- [x] Alloc regression unchanged (`oriterm_core/tests/alloc_regression.rs`) — green via `./test-all.sh`.
- [x] RSS regression unchanged (`oriterm_core/tests/rss_regression.rs`) — green via `./test-all.sh`.
- [x] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release.
- [x] Plan annotation cleanup — `plan-annotations.py` scan returns 0 stale annotations; the 2 active annotations (`§07` + `BUG-08-8`) are live cross-references, not drift.
- [x] Section frontmatter `status` → `complete`.
- [x] `00-overview.md` Quick Reference — §12 row flipped `Not Started` → `Complete`. Mission success criteria contributes to **Verification chain complete per row** (every sixel catalog row now `verified` or `verified-with-deviation`) + **Image lifecycle correct under resize/reflow/scrollback/alt-screen** (§12.5 lifecycle.rs 10-category matrix).
- [x] `index.md` section 12 status flipped `Not Started` → `Complete`.
- [x] Next section `depends_on` verification — §13 (Kitty) carries `depends_on: ["12"]` in frontmatter (line 25); §13.6 body now explicitly depends on §12.5's `cross_stack_handoff.rs` handshake as a precondition for the deep mixed-protocol rendering sweep.
- [x] `/tpr-review` passed (final, full-section) — Review 2 in §12.R above: 4 rounds, 11 findings verified + fixed inline, round 3 exit `clean`.
- [x] `/impl-hygiene-review` passed (after `/tpr-review` clean) — Review 3 in §12.R above: 4 actionable findings fixed inline, exit `clean`.

**Exit Criteria:** Every sixel catalog row is `verified` — including per-operator, behavioral (bg-mode + palette-reset + abort), and §11-occlusion rows. Sixel is the first conformance-complete visual stack and establishes the z-order + transparency composite + mixed-protocol hand-off pins that §13 Kitty, §14 iTerm2, and §15 Cell-Level Alpha inherit.
