---
section: "12"
title: "Sixel"
status: in-progress

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
  - "DCS-abort is pinned end-to-end through the VTE performer: CAN (0x18), SUB (0x1A), and ESC-mid-DCS all drive `Performer::unhook` (`crates/vte/src/lib.rs:341-355`) → `dispatch/mod.rs:118-131` → `Term::handle_sixel_end` (`oriterm_core/src/term/handler/image/sixel.rs:34-64`) but the test asserts **no image placement is created** when the abort happens before sixel data is finalized. (Today the abort path stores unconditionally — if the test fails, that is BUG-12-* filed via `/add-bug`.)"
  - "Sixel grid integration verified: SIXEL_SCROLLING mode 80 cursor positioning, SIXEL_CURSOR_RIGHT mode 8452 cursor positioning, image placement creation at `(cell_col, stable_row)` with `z_index: 0`, orphan cleanup via `prune_scrollback`."
  - "Sixel GPU rendering verified via golden image apex on the deterministic lane from §05 — five expanded scenarios: (a) solid rectangle, (b) multi-color palette-switch mid-image, (c) `!` repeat optimization, (d) `$` CR + `-` NL banding interaction, (e) transparency composite against the deterministic background without sub-pixel jitter, PLUS (f) SIXEL_SCROLLING OFF goldens, (g) SIXEL_CURSOR_RIGHT ON goldens so §12.3's behavioral bullets are visually pinned."
  - "Sixel + unicode/subcell occlusion verified: sixel placements use `z_index: 0` (`oriterm_core/src/term/handler/image/sixel.rs:79-97`) and non-negative images render above text (`oriterm/src/gpu/prepare/emit.rs:264-285`). Goldens pin z-order against §11 glyph families — sixel + wide-CJK, sixel + ZWJ cluster, sixel + half-blocks/quadrants/sextants."
  - "Sixel + image lifecycle handled correctly per §07 (consumes §07's handlers — §07 is `status: complete`): scrollback eviction **removes** sixel placements via `prune_scrollback`; ED / EL erase **remove** placements in the erased region via `remove_placements_in_region`; alt-screen toggle **preserves** the primary-cache placement across enter/exit (alt cache is separate); resize (column shrink) **removes** column-out-of-bounds placements via `ImageCache::on_resize`; resize with reflow **remaps** placement `StableRowIndex` values via `remap_placements`; font-size / DPI change **recomputes** `FixedPixels` cell coverage via `SetCellDimensions`."
  - "Sixel ↔ Kitty cross-stack hand-off proven at `ImageCache` + placement level: a shared-cache regression test places a sixel image, then a kitty image into the same `ImageCache` instance on a `Term`, and asserts both placements are independently addressable via the public snapshot API `Term::renderable_content()` (`oriterm_core/src/term/snapshot.rs:33`) → `RenderableContent::images` (deeper mixed-protocol rendering interference is delegated to §13.6 via an explicit DEFERRED-TO-DOWNSTREAM cross-link)."
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
    status: not-started
  - id: "12.4"
    title: "Verify sixel GPU rendering via golden image apex (expanded scenarios + cursor-mode goldens)"
    status: not-started
  - id: "12.5"
    title: "Verify sixel + image lifecycle interactions + sixel↔kitty ImageCache hand-off"
    status: not-started
  - id: "12.R"
    title: "Third Party Review Findings"
    status: complete

  - id: "12.N"
    title: "Completion Checklist"
    status: not-started
# TPR Checkpoint Placement: 12.3 (after parser/decoder state machine + invariants + grid integration — covers .1-.3),
# 12.5 (after GPU goldens + lifecycle + cross-stack hand-off — covers .4-.5), final in 12.N
---

# Section 12: Sixel

**Status:** Not Started
**Goal:** Sixel is the first full visual stack — its verification chain exercises the entire pipeline from DCS byte parsing through GPU composition. This section drives every sixel catalog row to `verified`, closes the parser/decoder state-machine seam end-to-end, and pins three invariants that the current 5-row catalog does not cover: background-mode distinction, palette reset per DCS q, and DCS abort correctness.

**Success Criteria:** see frontmatter.

**Context:** Pass 1 confirmed sixel parser, decoder, grid integration, and GPU rendering all exist. Section 04's pilot verified one minimal scenario (opaque rectangle). This section drives every other catalog row AND adds the per-operator + behavioral rows the current catalog lacks. The HLS rotation bug suspected by the audit memory turned out to be CORRECT (`hue - 120.0` at `oriterm_core/src/image/sixel/color.rs:41` — verified by Pass 1). Section 07 (image lifecycle) is `status: complete`, so `ImageCache::on_resize`, `remap_placements`, and the 42-scenario lifecycle matrix are available runway for §12.5.

**Code seam this section owns (in-crate anchors):**
- `oriterm_core/src/image/sixel/mod.rs` — `SixelParser` coupled state machine: byte intake in `SixelParser::feed`, operator dispatch in `finish_command`, palette rebuild in `SixelParser::new` (copies `VT340_PALETTE`), repeat clamp in `emit_sixel` (`count.min(MAX_DIMENSION)`; const at the top of the file), raster attrs Pan/Pad ignored in `apply_raster_attrs` (reads only `params[2]`/`params[3]`), `SetToBg` enum as `SixelBgMode` + undrawn-pixel fill in `finish()` now routes through `terminal_bg` captured from `Term::effective_background` (DECSCNM-aware) rather than the pre-§12.2 opaque-black collapse. Test-only VT340 bypass lives in sibling `bypass.rs`.
- `oriterm_core/src/term/handler/image/sixel.rs` — handler wiring: `handle_sixel_start` (captures `Term::effective_background` at DCS-hook time) / `handle_sixel_put` / `handle_sixel_end` (early-return on `aborted`), `sixel_create_placement` with `z_index: 0`.
- `crates/vte/src/lib.rs:341-355` — VTE `Performer::unhook` drives DCS finalize; CAN/SUB/ESC-mid-DCS route through the same `unhook` callback.
- `crates/vte/src/ansi/dispatch/mod.rs:118-131` — dispatch routes `DcsState::Sixel` unhook to `Term::sixel_end` regardless of whether the DCS was finalized or aborted.
- `oriterm_core/src/term/snapshot.rs:33` — `Term::renderable_content()` public snapshot API; at `:79` `renderable_content_into()` is the hot-path no-alloc variant. Both populate `RenderableContent::images` via the private helper `extract_images` at `:243`. Consumers (tests, GPU pipeline) drive through the public API.
- `oriterm/src/gpu/window_renderer/frame_prep.rs:149-173` + `oriterm/src/gpu/image_render/mod.rs:67-151` + `oriterm/src/gpu/prepare/emit.rs:262-285` — image-render prepare path shared with §13 Kitty; `z_index >= 0` draws above text.

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

**Why merged:** The original plan split parser (§12.1) and decoder (§12.2) into separate test files. The `SixelParser` at `oriterm_core/src/image/sixel/mod.rs:33-73,115-190,256-360` is **one coupled state machine** — byte intake, command parsing, palette mutation, raster-attrs, and pixel emission all share the same `SixelParser` struct. A byte-level test that tokenizes `P1` parameters but never drives palette mutation can be green while palette dispatch is broken. Merging §12.1 + §12.2 into a single state-machine rung forces intermixed-operator coverage, which is what actually verifies the coupling. Pure-decoder invariants (background mode, palette reset, repeat clamp) move to §12.2 because they are observable properties of the decoder's **output state** rather than per-operator dispatch correctness.

**What to test (catalog rows driven to `verified` by this subsection — the per-operator + abort rows from §12.0):**

- [x] Write failing test matrix BEFORE implementation (TDD per `.claude/rules/tests.md` §TDD for Bugs).
- [x] **DCS q introducer** — feed `ESC P <Ps1> ; <Ps2> ; <Ps3> q` prefixes with the full P1×P2×P3 cartesian product. Observe via the spec_chain parser rung that the DCS dispatch reaches `Term::handle_sixel_start(params)` with the correct P1/P2/P3 values captured, and observe via the state-effect rung that P2 drives the resulting `SixelBgMode` distinctly (`DeviceDefault` / `NoChange` / `SetToBg`). Do NOT assert on the private `SixelParser::new` constructor signature — that is an internal implementation detail; the observable dispatch call + output-state difference is what the chain pins.
- [x] **Raster attributes `"`** — feed `" <Pan> ; <Pad> ; <Ph> ; <Pv>` before data, assert width/height are set from Ph/Pv (note Pan/Pad currently ignored — `oriterm_core/src/image/sixel/mod.rs:311-330` reads only `params[2]` / `params[3]` inside `apply_raster_attrs` — this is a documented divergence the audit row captures).
- [x] **Color define Pu=2 (RGB)** — feed `#n;2;Px;Py;Pz`, assert `palette[n] = [Px, Py, Pz]` scaled from 0-100 to 0-255.
- [x] **Color define Pu=1 (HLS)** — feed `#n;1;Ph;Pl;Ps`, assert `palette[n]` matches libsixel's `hls_to_rgb` at `oriterm_core/src/image/sixel/color.rs:41` (the `hue - 120.0` rotation verified correct in Pass 1).
- [x] **Color select `#n`** — feed `#n` without follow-on params, assert `current_color = n`.
- [x] **Repeat `!n`** — feed `!5?`, assert 5 consecutive columns of the same sixel data byte are emitted.
- [x] **CR `$`** — feed data, `$`, more data, assert second data band starts at `x = 0` with `y` unchanged.
- [x] **NL `-`** — feed data, `-`, more data, assert second data band starts at `x = 0` with `y += 6`.
- [x] **Sixel data byte `?`..`~`** — for every byte in the 63-code range, assert the 6-bit pixel column is decoded per `byte - 0x3F`.
- [x] **Intermixed `#` mid-data** — feed `<data>#5<data>`, assert the second data band uses `current_color = 5` on live placement (forces dispatch on live `SixelParser` state, not just setup).
- [x] **`!` repeat interacts with palette** — feed `#3!5?`, assert all 5 columns use palette index 3 (forces repeat to read `current_color` at emission time, not at `!` time).
- [x] **Raster-before-data** — feed `"1;1;10;20<data>`, assert dimensions set before first pixel.
- [x] **Raster-mid-stream** — feed `<data>"5;5;100;100<data>`, assert the documented behavior (ignored vs re-dimensions). Audit file row captures the choice.
- [x] **Negative pin — abort path** — feed `ESC P q <data> <CAN>`, drive through `crates/vte/src/lib.rs:341-355` `Performer::unhook` → `dispatch/mod.rs:118-131` → `Term::handle_sixel_end` at `oriterm_core/src/term/handler/image/sixel.rs:34-64`, assert:
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
- [x] **Negative pin — palette leak** — pinned by `palette_reset_per_dcs_negative_pin_bypass_breaks_vt340_fingerprint` in the unit tests (`oriterm_core/src/image/sixel/tests.rs`). Implementation adds a `#[cfg(test)]`-only `BYPASS_VT340_RESET` thread-local flag in `oriterm_core/src/image/sixel/mod.rs`; when set, `SixelParser::new` skips the VT340 rebuild loop and leaves `palette[5] = [0,0,0]`. The test flips the flag, confirms `#5@` yields opaque black (not cyan), then restores — proving the VT340 rebuild is load-bearing. Lives as a unit test because `#[cfg(test)]` is not visible to integration tests.
- [x] **Repeat clamp at `MAX_DIMENSION`** — pinned by `repeat_clamps_at_max_dimension_without_allocation_spike` in `invariants.rs`. Feeds `!20000~` (count 20 000 well above the 10 000 clamp); asserts placement commits with `w ≤ 10 000`, no panic, no OOM. libsixel also applies a protective clamp — tracked as `verified-with-deviation` in the catalog row.
- [x] **Pixel-buffer cap** — pinned by `raster_attrs_exceeding_max_pixel_bytes_aborts_cleanly` in `invariants.rs`. Feeds `"1;1;15000;15000` (900 MB > 100 MB `MAX_PIXEL_BYTES`); asserts no placement is committed (`placement_count == 0`) because `apply_raster_attrs` sets `aborted = true` and `finish()` returns `Err(ImageError::OversizedImage)`, which `handle_sixel_end` warns+drops.
- [x] Matrix count assertion — `invariant_category_matrix_completeness` in `invariants.rs` plus the `set_to_bg_*` / `device_default_and_set_to_bg_diverge_*` / `palette_reset_per_dcs_negative_pin_*` unit tests; count totals 7 invariant categories (6 integration + 1 unit).
- [x] Update `SIXEL-BG-*`, `SIXEL-PALETTE-RESET-PER-DCS`, and repeat-clamp catalog rows (from §12.0 expansion) to `verified`. `SIXEL-REPEAT-CLAMP` + `SIXEL-PIXEL-BUFFER-CAP` → `verified-with-deviation`; `SIXEL-BG-*` and `SIXEL-PALETTE-RESET-PER-DCS` → `verified`.
- [x] Verify all tests pass in both debug AND release builds — `cargo test -p oriterm_core --lib image::sixel` + `cargo test -p oriterm_core --test spec_chain sixel::` green in debug AND `--release`; `./test-all.sh`, `./clippy-all.sh`, `./build-all.sh` all green (workspace + `x86_64-pc-windows-gnu` cross-compile).
- [x] **Validation:** invariant rung green; decoder output state proven correct across DCS boundaries + buffer bounds.

---

## 12.3 Verify sixel grid integration + §11 occlusion

**File(s):** `oriterm_core/tests/spec_chain/sixel/grid_integration.rs` (new) for the non-GPU bullets (cursor positioning, placement creation, orphan cleanup, z_index negative-pin at the snapshot level); plus the §11 OCCLUSION golden-image bullets belong in the GPU crate as new pilots alongside `oriterm/src/gpu/visual_regression/spec_chain/pilots/sixel_minimal.rs` (e.g. `sixel_occlusion_wide_cjk.rs`, `sixel_occlusion_zwj.rs`, `sixel_occlusion_subcell.rs`) with goldens at `oriterm/tests/references/sixel_occlusion_<family>.png` per `.claude/rules/crate-boundaries.md` — grid-state assertions stay in `oriterm_core`, pixel-golden assertions stay in `oriterm`.

**Depends on code paths:** `oriterm_core/src/term/handler/image/sixel.rs:68-139` (placement creation + cursor advance); `oriterm/src/gpu/prepare/emit.rs:262-285` (z-order emission); §11 unicode/subcell payloads.

- [ ] Write failing test matrix BEFORE implementation.
- [ ] **SIXEL_SCROLLING ON (default)** — place sixel, assert cursor moves to next line below the image per `oriterm_core/src/term/handler/image/sixel.rs:129-135`.
- [ ] **SIXEL_SCROLLING OFF (DECRST 80)** — place sixel, assert cursor stays at the pre-placement position per `sixel.rs:136-138`.
- [ ] **SIXEL_CURSOR_RIGHT ON (DECSET 8452)** — place sixel, assert cursor moves to the right of the image rather than below per `sixel.rs:124-128`.
- [ ] **Image placement creation** — after sixel data, `ImageCache` contains a placement with `z_index: 0`, `cell_col` and `cell_row` at cursor position, `PlacementSizing::FixedPixels { width, height }` from the decoded image (`sixel.rs:79-97`).
- [ ] **Orphan cleanup** — place sixel, scroll beyond eviction threshold, assert `prune_scrollback` removes the placement (consumes §07's handler).
- [ ] **§11 OCCLUSION — sixel + wide-CJK** — write a wide CJK character at `(row, col)`, then place a sixel at an overlapping cell, assert golden-image z-order: sixel above CJK glyph (non-negative `z_index` draws above text per `oriterm/src/gpu/prepare/emit.rs:264-285`). Catalog row: new cross-reference to `catalog/unicode-subcell.md` occlusion rows.
- [ ] **§11 OCCLUSION — sixel + ZWJ cluster** — emit a ZWJ emoji cluster (e.g., `U+1F468 U+200D U+1F4BB`), place sixel overlapping, assert z-order.
- [ ] **§11 OCCLUSION — sixel + subcell glyphs** — emit half-blocks / quadrants / sextants at an occupied row, place sixel overlapping, assert z-order against the deterministic golden.
- [ ] **Negative pin — negative z_index** — programmatically create a placement with `z_index: -1` (below text), assert the text draws **above** the image per the same emit-path logic. Proves z-order is live, not coincidentally correct.
- [ ] Matrix count assertion.
- [ ] Update grid-integration + MODE-80 + MODE-8452 + §11-occlusion catalog rows to `verified`.
- [ ] Verify all tests pass in both debug AND release builds.
- [ ] **TPR checkpoint** — `/tpr-review` covering §12.1–§12.3 (state machine + invariants + grid integration + occlusion). Catches multi-rung integration issues before GPU + lifecycle subsections.
- [ ] **Validation:** grid-integration rung green; z-order pinned against §11 subcell glyph families.

---

## 12.4 Verify sixel GPU rendering via golden image apex

**File(s):** new per-scenario pilots under `oriterm/src/gpu/visual_regression/spec_chain/pilots/` alongside the existing `sixel_minimal.rs` (e.g. `sixel_palette_switch.rs`, `sixel_repeat.rs`, `sixel_cr_nl_banding.rs`, `sixel_transparency.rs`, `sixel_scrolling_off.rs`, `sixel_cursor_right.rs`), with goldens committed as flat PNGs at `oriterm/tests/references/sixel_<scenario>.png` (matching the existing `sixel_minimal.png` naming convention). GPU-apex work MUST live in the `oriterm` crate per `.claude/rules/crate-boundaries.md` — `oriterm_core/tests/` is non-GPU; the existing `oriterm/src/gpu/visual_regression/spec_chain/pilots/sixel_minimal.rs` is the canonical pattern.

**Depends on code paths:** `oriterm_core/src/term/snapshot.rs:33` (`Term::renderable_content()` public API; populates `RenderableContent::images`); `oriterm/src/gpu/window_renderer/frame_prep.rs:149-173`; `oriterm/src/gpu/image_render/mod.rs:67-151`; `oriterm/src/gpu/prepare/emit.rs:262-285`. Deterministic lane from §05 (`status: complete`).

- [ ] Write failing test matrix BEFORE implementation.
- [ ] Capture goldens via `ORITERM_UPDATE_GOLDEN=1` using the deterministic lane (llvmpipe, grayscale hinting, pinned cell metrics) from §05.
- [ ] **Scenario A — Solid rectangle, one color** (baseline parity with §04 pilot, explicit row in this section's directory).
- [ ] **Scenario B — Palette-switch mid-image** — emit `#0;2;100;0;0<data>#1;2;0;100;0<data>`, assert the golden shows both color bands. Forces `#` dispatch on live placement (catches a parser-correct, decoder-wrong seam).
- [ ] **Scenario C — Repeat optimization** — emit `!100?`, assert golden shows 100 equal-valued columns (no per-column decode drift).
- [ ] **Scenario D — CR + NL banding interaction** — emit two bands separated by `$` and two bands separated by `-`, assert golden shows correct band stacking. Forces `$` + `-` operator semantics into the raster output, not just the state machine.
- [ ] **Scenario E — Transparency composite** — P2=1 with partial coverage over the deterministic background from §05. **Explicit sub-pixel / AA gate:** assert 0-pixel diff against golden with zero AA jitter. Any non-zero diff on the deterministic lane IS a bug (either in transparency compositing in `oriterm/src/gpu/image_render/mod.rs` or in §05's cell-metrics pinning).
- [ ] **Scenario F — SIXEL_SCROLLING OFF golden** — DECRST 80 + sixel, assert cursor-stays-at-home is visually pinned (image placed where cursor was, subsequent text overwrites image cells per no-scroll behavior). Pins §12.3's behavioral bullet visually, not just programmatically.
- [ ] **Scenario G — SIXEL_CURSOR_RIGHT golden** — DECSET 8452 + sixel, assert cursor-to-right-of-image is visually pinned (subsequent text lands to the right of the image band).
- [ ] Spec_chain tests assert goldens match on subsequent runs — 0-pixel diff tolerance on the deterministic lane.
- [ ] Negative pin — run without deterministic-lane pinning (e.g., `ORITERM_FORCE_NONDETERMINISTIC=1` if available, or a different adapter); test should skip with `eprintln!("SKIP: ...")` per `.claude/rules/tests.md` §Graceful Skip Protocol, never falsely pass or crash.
- [ ] Matrix count assertion.
- [ ] Update GPU-rendering catalog rows to `verified`.
- [ ] Verify all tests pass in both debug AND release builds.
- [ ] **Validation:** golden tests pass; back-to-back runs produce 0-pixel diff; cursor-mode goldens visually pin §12.3's behavioral bullets.

---

## 12.5 Verify sixel + image lifecycle interactions + sixel↔kitty ImageCache hand-off

**File(s):** `oriterm_core/tests/spec_chain/sixel/lifecycle.rs` (new), `oriterm_core/tests/spec_chain/sixel/cross_stack_handoff.rs` (new)

**Depends on code paths:** `oriterm_core/src/image/cache/lifecycle.rs` (§07 complete — `on_resize`, `remap_placements`, `prune_scrollback`, `remove_placements_in_region`); `oriterm_core/src/term/handler/image/sixel.rs` (sixel side); `oriterm_core/src/term/handler/image/kitty.rs` (kitty side — read-only reference, no edits).

**§07 runway note:** §07 is `status: complete` — `ImageCache::on_resize`, `ReflowMapping` / `remap_placements`, and the 42-scenario lifecycle matrix are available. §12.5 **consumes** these handlers; it does not re-implement them. Specifically relied-upon §07 deliverables:
- `ImageCache::on_resize(new_cols, new_rows)` — removes column-out-of-bounds placements.
- `ImageCache::remap_placements(mapping)` — translates `StableRowIndex` through `ReflowMapping::first_output_row`.
- `Term::resize` — invokes `remap_placements` → `prune_scrollback` → `on_resize` on primary cache; alt cache gets `on_resize` only.
- `PaneIoCommand::SetCellDimensions` — the cell-metric runtime-config wire that feeds `FixedPixels` re-coverage.

- [ ] Write failing test matrix BEFORE implementation.
- [ ] **Scrollback eviction** — place sixel near top of scrollback, fill scrollback past eviction threshold, assert placement removed from cache via `prune_scrollback`.
- [ ] **ED (erase display)** — place sixel, emit `CSI 2 J`, assert placement removed from the erased region via `remove_placements_in_region`.
- [ ] **EL (erase line)** — place sixel, emit `CSI 2 K`, assert placement at that row removed.
- [ ] **Alt-screen toggle** — place sixel in primary, enter alt screen (DECSET 1049), assert primary cache preserved AND alt cache is separate; exit alt screen, assert primary placement still intact.
- [ ] **Resize shrink columns** — place sixel at col=90, resize grid to 80 cols, assert placement removed (consumes `on_resize`).
- [ ] **Resize with reflow** — place sixel, resize with reflow enabled, assert placement's `StableRowIndex` updated via `remap_placements` (consumes §07's `ReflowMapping::first_output_row`).
- [ ] **Font-size / DPI change** — place `FixedPixels` sixel, dispatch `PaneIoCommand::SetCellDimensions`, assert placement's `cols`/`rows` coverage recomputed.
- [ ] **CROSS-STACK ImageCache hand-off** — in `cross_stack_handoff.rs`:
  - Place a sixel image at `(row=5, col=0)`.
  - Place a kitty image at `(row=10, col=0)` by driving the kitty transmit+place action through `oriterm_core/src/term/handler/image/kitty.rs` (read-only — no `kitty.rs` edits, which keeps BUG-08-8's kitty-scope gate intact).
  - Call the public snapshot API `Term::renderable_content()` (`oriterm_core/src/term/snapshot.rs:33`) — or `renderable_content_into()` at `:79` for the hot-path no-alloc variant — and assert that `RenderableContent::images` contains BOTH placements, each independently addressable. (The internal helper `Term::extract_images` at `:243` is private; the chain's state-effect rung observes the public snapshot, not the private helper.)
  - Assert neither image corrupts the other's pixel data (sixel RGBA buffer unchanged, kitty PNG-decoded RGBA buffer unchanged).
  - **This is a handshake test, not a full cross-stack rendering regression.** Deep mixed-protocol rendering interference (overlapping placements, z-order interleaving, shared-eviction races) is explicitly DEFERRED-TO-DOWNSTREAM to §13.6 — recorded as a cross-link here and in §12.N.
- [ ] Negative pin — `ImageCache::on_resize` on a cache with no sixel placements must be a no-op (proves the handler fires only on relevant placements).
- [ ] Alloc regression unchanged — per `.claude/rules/tests.md` §Performance Invariants, lifecycle handlers must not allocate per placement beyond the `remove` cost already accounted for in §07.
- [ ] Matrix count assertion.
- [ ] Update lifecycle + cross-stack-handshake catalog rows to `verified`.
- [ ] Verify all tests pass in both debug AND release builds.
- [ ] **TPR checkpoint** — `/tpr-review` covering §12.4–§12.5 (GPU goldens + lifecycle + cross-stack hand-off).
- [ ] **Validation:** lifecycle rung green; cross-stack hand-off proven at `ImageCache` level; downstream deferral to §13.6 recorded.

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

---

## 12.N Completion Checklist

- [ ] Every row in `catalog/sixel.md` is `verified` (including the per-operator + behavioral + §11-occlusion rows opened in §12.0).
- [ ] `plans/spec-conformance/audits/section-12-top-down-inventory.md` is populated, `last_walked` set to today, `walked_by` set.
- [ ] Failing test matrix written FIRST (§12.1–§12.5 each have their own TDD checkbox).
- [ ] **Matrix dimensions**: operator × DCS-count × background-mode × cursor-mode × lifecycle-event × protocol-neighbor (§11 subcell, §13 kitty hand-off).
- [ ] **Semantic pins (≥3 — one per invariant)**: (a) `SetToBg` differs from `DeviceDefault` on identical input, (b) palette resets between DCS q invocations, (c) DCS abort commits no placement.
- [ ] **Negative pins (≥3)**: (a) palette-leak guard fails if reset code is bypassed, (b) `z_index: -1` draws below text, (c) `on_resize` no-op when no sixel placements present.
- [ ] HLS rotation explicitly tested (cross-checked against libsixel `color.rs:41` `hue - 120.0`).
- [ ] `!` repeat clamp cross-checked against libsixel `src/decoder.c`; any divergence documented as a catalog-row note.
- [ ] DCS abort (CAN / SUB / ESC mid-DCS) commits no placement — if today's `handle_sixel_end` at `oriterm_core/src/term/handler/image/sixel.rs:34-64` stores unconditionally, that bug IS filed and fixed in-scope here (not deferred).
- [ ] `SixelBgMode::SetToBg` renders distinctly from `DeviceDefault` — if today's code at `oriterm_core/src/image/sixel/mod.rs:22-30,222-227` renders both as opaque black, that bug IS filed and fixed in-scope here (not deferred).
- [ ] Sixel + image lifecycle survives every grid mutation (§07 complete; §12.5 consumes handlers).
- [ ] Sixel + §11 occlusion goldens green (wide-CJK, ZWJ cluster, subcell glyphs).
- [ ] Sixel ↔ Kitty `ImageCache` hand-off green; deep mixed-protocol rendering interference DEFERRED-TO-DOWNSTREAM to §13.6 — cross-link recorded in §13.6's depends_on / success criteria, NOT silently deferred.
- [ ] `oriterm/src/gpu/visual_regression/spec_chain/pilots/sixel_minimal.rs` (§04 sixel pilot) stays green — no regression from §12's new pilots. (No sixel-specific teseq suite exists; `oriterm_core/tests/teseq/` has no sixel scenarios and §23.5 owns teseq archival.)
- [ ] Alloc regression unchanged (`oriterm_core/tests/alloc_regression.rs`).
- [ ] RSS regression unchanged (`oriterm_core/tests/rss_regression.rs`).
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release.
- [ ] Plan annotation cleanup.
- [ ] Section frontmatter `status` → `complete`.
- [ ] `00-overview.md` Quick Reference + mission success criteria checkboxes updated (contributes to **Verification chain complete per row** and **Image lifecycle correct under resize/reflow/scrollback/alt-screen**).
- [ ] `index.md` section 12 status updated.
- [ ] Next section `depends_on` verification — §13 (Kitty) currently lists `depends_on: ["12"]`; verify §13.6's cross-stack-regression depends on §12.5's hand-off test.
- [ ] `/tpr-review` passed (final, full-section).
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean).

**Exit Criteria:** Every sixel catalog row is `verified` — including per-operator, behavioral (bg-mode + palette-reset + abort), and §11-occlusion rows. Sixel is the first conformance-complete visual stack and establishes the z-order + transparency composite + mixed-protocol hand-off pins that §13 Kitty, §14 iTerm2, and §15 Cell-Level Alpha inherit.
