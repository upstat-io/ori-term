---
bug: "BUG-08-8"
title: "kitty.rs is 480 lines — BLOAT-adjacent; must split before Sections 12 / 13 implementation"
severity: "high"
status: in-progress
goal: "Split `oriterm_core/src/term/handler/image/kitty.rs` (480 lines) into per-action submodules under `oriterm_core/src/term/handler/image/kitty/` with every resulting file ≤ 200 lines, preserving all existing behavior + test pass status."
success_criteria:
  - "`oriterm_core/src/term/handler/image/kitty.rs` (monolith) no longer exists; replaced by `kitty/mod.rs` + per-action submodules."
  - "Every file under `oriterm_core/src/term/handler/image/kitty/` is ≤ 200 lines verified by `find oriterm_core/src/term/handler/image/kitty/ -name '*.rs' -exec wc -l {} +` — every output line ≤ 200."
  - "`oriterm_core/src/term/handler/image/kitty_animation.rs` is moved into `kitty/` as `frame.rs` (a=f) + `animate.rs` (a=a) per BUG-08-8's proposed-fix diagram."
  - "`handle_kitty_graphics` dispatch entry preserves the exact same `KittyAction` match behavior; no action routing changes."
  - "All existing tests still pass: `./test-all.sh` + `./build-all.sh` + `./clippy-all.sh` green across debug + release (workspace + `x86_64-pc-windows-gnu` cross-compile)."
  - "No new `#[cfg_attr(test)]` or behavioral change lands; split is purely mechanical (rename + move + re-export)."
subsystem: "oriterm_core/src/term/handler/image/kitty.rs"
found: "2026-04-11"
source: "continue-roadmap"
third_party_review:
  status: none
  updated: null
---

# Fix: BUG-08-8 — kitty.rs BLOAT split

**Status:** In Progress
**Severity:** high
**Goal:** Partition the 480-line `kitty.rs` monolith into per-action submodules under `oriterm_core/src/term/handler/image/kitty/` so §13 verification work can land new per-action logic without overflowing the 500-line hard limit. No semantic change; pure structural refactor.

**Success Criteria:** see frontmatter.

**Context:** `oriterm_core/src/term/handler/image/kitty.rs` is 480 lines (verified by `wc -l`, 2026-04-21). `.claude/rules/code-hygiene.md` §File Size enforces a 500-line hard limit with a ~450-line proactive-split threshold. Starting §13's verification work on this file would push it over the limit as soon as any per-action code lands (e.g., §13.0.5's delete-specifier fixes, §13.3's compose-frame addition, §13.4's U=1 placeholder resolution). This bug is the BLOCKING precondition for §13.1–§13.6 per `plans/spec-conformance/section-13-kitty-graphics.md` §13.0.5.

This bug is orthogonal to BUG-08-7 (delete-specifier correctness). BUG-08-8 is pure plumbing; BUG-08-7 is semantic. §13.0.5 bundles both, but `/fix-bug BUG-08-8` handles ONLY the split; `/fix-bug BUG-08-7` follows and fixes the delete arms against the post-split structure.

---

## 1. Root Cause Analysis

- **Symptom**: `wc -l oriterm_core/src/term/handler/image/kitty.rs` returns 480. File is 20 lines below the 500-line hard limit and 30 lines above the ~450-line proactive-split threshold.
- **Proximate cause**: All seven action handlers (transmit, transmit+place, place, delete, query, frame, animate) plus their shared helpers (store_image, decode_pixels, store_from_file, accumulate_chunk, finalize_payload, create_placement, respond) live in one file.
- **Root cause**: The kitty protocol handler was implemented monolithically without establishing a per-action submodule convention. As coverage expanded (chunked transmission, animation, file/tempfile transport, response emission), the file naturally grew. No canonical split point was ever carved; the file accumulated until it hit the proactive threshold.
- **Blast radius**:
  - 1 file to split (kitty.rs → 7 new per-action files + 1 mod.rs)
  - 1 file to consolidate (kitty_animation.rs → kitty/frame.rs + kitty/animate.rs)
  - 1 parent mod.rs to update (image/mod.rs — remove `mod kitty_animation;` since it moves inside `kitty/`; `mod kitty;` stays as directory module)
  - Internal `pub(super) fn` visibility re-scoping: `kitty_finalize_payload`, `kitty_accumulate_chunk`, `kitty_decode_pixels`, `kitty_respond` (currently `pub(super)` so image/mod.rs can see them) — after split, these are called across kitty/ submodules only; `pub(super)` inside kitty/ is correct scope.
  - Zero integration-test breakage (no inline `#[test]` modules in kitty.rs; protocol exercised via VTE integration tests through `handle_apc_dispatch`).
  - Zero public-API surface change (`handle_apc_dispatch` in image/mod.rs stays `pub(in crate::term::handler)`; internal method names stay identical).
- **Affected files**:
  - `oriterm_core/src/term/handler/image/kitty.rs` — DELETE (monolith gone after split).
  - `oriterm_core/src/term/handler/image/kitty_animation.rs` — DELETE (consolidated into `kitty/frame.rs` + `kitty/animate.rs`).
  - `oriterm_core/src/term/handler/image/mod.rs` — remove `mod kitty_animation;` line; `mod kitty;` stays and now points at directory module.
  - NEW: `oriterm_core/src/term/handler/image/kitty/mod.rs` — dispatch entry (`handle_kitty_graphics` + `KittyAction` match) + shared type (`KittyStoreParams`).
  - NEW: `oriterm_core/src/term/handler/image/kitty/transmit.rs` — `kitty_transmit`, `kitty_transmit_and_place`, `kitty_finalize_payload`, `kitty_accumulate_chunk`.
  - NEW: `oriterm_core/src/term/handler/image/kitty/place.rs` — `kitty_place`, `kitty_create_placement`.
  - NEW: `oriterm_core/src/term/handler/image/kitty/delete.rs` — `kitty_delete` (pre-BUG-08-7 logic — semantic fix in separate /fix-bug).
  - NEW: `oriterm_core/src/term/handler/image/kitty/store.rs` — `kitty_store_image`, `kitty_decode_pixels`, `kitty_store_from_file`.
  - NEW: `oriterm_core/src/term/handler/image/kitty/query.rs` — `kitty_query`.
  - NEW: `oriterm_core/src/term/handler/image/kitty/response.rs` — `kitty_respond`.
  - NEW: `oriterm_core/src/term/handler/image/kitty/frame.rs` — `kitty_frame` (moved from kitty_animation.rs).
  - NEW: `oriterm_core/src/term/handler/image/kitty/animate.rs` — `kitty_animate` (moved from kitty_animation.rs).

**Reference implementations**:
- **wezterm** `term/src/terminalstate/kitty.rs` + `term/src/terminalstate/kitty/` — similar per-action submodule structure; wezterm's split proves this layout scales for the full kitty protocol.

---

## 1.5 Fix Consensus (via /tp-help)

- **Proposed approach (pre-consensus)**: Partition kitty.rs strictly by action name — one submodule per `KittyAction` enum variant — plus three helper submodules (`store.rs` for decode/store/file-read helpers, `response.rs` for reply emission, `mod.rs` for dispatch + `KittyStoreParams` shared type). `transmit.rs` owned `kitty_finalize_payload` + `kitty_accumulate_chunk`. Move kitty_animation.rs into `frame.rs` + `animate.rs`. `delete.rs` as file module (BUG-08-7 converts to directory later). Preserve `pub(super)` visibility scope. No semantic change.
- **tp-help run scratch dir**: `/tmp/tpr-round-ori_term-Dcv1NnbI`

### Round 1

- **Codex summary**: Agrees with the directory split and consolidating `kitty_animation.rs` under `kitty/`, but flags 3 refinements:
  1. **Move `kitty_accumulate_chunk` + `kitty_finalize_payload` + `KittyStoreParams` into `mod.rs`, NOT `transmit.rs`.** Rationale: `kitty_frame` in `kitty_animation.rs:28,32` ALSO calls these helpers; putting them in `transmit.rs` makes `frame.rs` depend on a sibling action module for non-action-specific state staging — that's a LEAK per `.claude/rules/impl-hygiene.md:257` (SSOT / No Side Logic).
  2. **Rename `frame_compose.rs` → `frame.rs`.** Rationale: `a=c` Compose is a distinct future action (`KG-ACTION-COMPOSE` is `missing` in the §13.0 catalog); `frame_compose` invites conflation with the separate Compose semantics.
  3. **Make `delete/` a directory module immediately, not `delete.rs` file module.** Rationale: BUG-08-7 follows in the same §13.0.5 gate and will need `delete/tests.rs` per `.claude/rules/test-organization.md:16-24` ("any module with tests must be `foo/mod.rs` plus `foo/tests.rs`"). Establish the directory structure now to avoid rename churn.
  Also confirmed `pub(super)` is correct, file-size target `≤ 200` is right, and warned against duplicated dispatch (keep parse-error handling + info log + `KittyAction` match in `mod.rs`, don't let leaf files grow their own routing).
- **Gemini summary**: FAILED — persistent `429 RESOURCE_EXHAUSTED` on `gemini-3.1-pro-preview` across 3 retry attempts. Survivor-mode per `/tpr-review` §9. Known provider pattern per memory `feedback_no_codex_timeout.md`.
- **Agreement points**: Directory split is correct; consolidate kitty_animation.rs into kitty/; `pub(super)` visibility; file-size target ≤ 200; split by `KittyAction` enum variant matches the protocol routing shape.
- **Disagreement points**: Codex's 3 refinements diverge from the pre-consensus proposal. No cross-reviewer disagreement (gemini survivor-mode).
- **Independent code verification**:
  - `oriterm_core/src/term/handler/image/kitty_animation.rs:28` — confirmed `kitty_accumulate_chunk(cmd)` call in `kitty_frame`.
  - `oriterm_core/src/term/handler/image/kitty_animation.rs:32` — confirmed `kitty_finalize_payload(&cmd)` call in `kitty_frame`.
  - `oriterm_core/src/term/handler/image/kitty.rs:53` — confirmed `KittyAction` match is the canonical dispatch point.
  - `oriterm_core/src/term/handler/image/kitty.rs:113-146` — confirmed `kitty_finalize_payload` constructs `KittyStoreParams` from `self.loading_image` + `cmd`; state-staging, not storage.
  - `.claude/rules/impl-hygiene.md §Side Logic, §Module Roles` — confirmed: "`mod.rs` dispatches and holds shared private items; leaf files implement".
  - `.claude/rules/test-organization.md §Sibling tests.rs Pattern` — confirmed: "When a module has tests, it must be a directory module (`foo/mod.rs`), not a file module (`foo.rs`). Never have `foo.rs` alongside a `foo/` directory."
  - `plans/spec-conformance/section-13-kitty-graphics.md:149-150` — confirmed §13.0.5 specifies `kitty/delete/mod.rs` + `kitty/delete/tests.rs`.
  - `plans/spec-conformance/catalog/kitty-graphics.md` — confirmed `KG-ACTION-COMPOSE` status `missing` (from §13.0 audit).
- **Outcome**: Persuaded divergence — codex's 3 refinements verified against code and rules. Adopting revised approach.

### Final agreed approach

Revised per codex consensus (1 round, survivor mode):

**`kitty/mod.rs`** — dispatch entry + shared state-staging:
- `handle_kitty_graphics` (APC entry; `KittyAction` match)
- `KittyStoreParams` type
- `kitty_finalize_payload` (shared by transmit + frame — reads `self.loading_image`, allocates image id)
- `kitty_accumulate_chunk` (shared by transmit + frame — chunked-upload state)
- `mod animate; mod delete; mod frame; mod place; mod query; mod response; mod store; mod transmit;`

**`kitty/transmit.rs`** — transmit-specific dispatch arms only:
- `kitty_transmit` (a=t)
- `kitty_transmit_and_place` (a=T fallback)

**`kitty/place.rs`** — place dispatch + placement construction:
- `kitty_place` (a=p)
- `kitty_create_placement`

**`kitty/delete/mod.rs`** — DIRECTORY MODULE from the start (no sibling `delete.rs`):
- `kitty_delete` (pre-BUG-08-7 logic — BUG-08-7 fixes semantics + adds `delete/tests.rs` in a separate /fix-bug)

**`kitty/store.rs`** — storage + decode + file-read:
- `kitty_store_image`
- `kitty_decode_pixels` (called from transmit AND frame via `Self::`)
- `kitty_store_from_file`

**`kitty/query.rs`** — query dispatch:
- `kitty_query` (a=q)

**`kitty/response.rs`** — reply emission:
- `kitty_respond` (all reply codes)

**`kitty/frame.rs`** (renamed from the initial proposal's `frame_compose.rs` — `a=c` Compose reserves `compose.rs` for future §13.3/§13.0 work):
- `kitty_frame` (a=f — animation frame transmit)

**`kitty/animate.rs`** — animation playback:
- `kitty_animate` (a=a — stop/run-wait/run, loop count, frame index, gap)

**No public API surface change.** `handle_kitty_graphics` keeps `pub(super)` scope visible to `image/mod.rs`. All inter-submodule helpers use `pub(super)` — correct scope (visible within the `kitty/` parent).

---

## 2. TDD — Test Matrix

Pure structural refactor; TDD is about invariant preservation, not new unit tests. The "matrix" here is:

### File-size invariants (all ≤ 200 lines)
- [ ] `wc -l oriterm_core/src/term/handler/image/kitty/mod.rs` ≤ 200 (holds dispatch + `KittyStoreParams` + shared `kitty_finalize_payload`/`kitty_accumulate_chunk`)
- [ ] `wc -l oriterm_core/src/term/handler/image/kitty/transmit.rs` ≤ 200
- [ ] `wc -l oriterm_core/src/term/handler/image/kitty/place.rs` ≤ 200
- [ ] `wc -l oriterm_core/src/term/handler/image/kitty/delete/mod.rs` ≤ 200 (directory module from the start; BUG-08-7 adds `delete/tests.rs`)
- [ ] `wc -l oriterm_core/src/term/handler/image/kitty/store.rs` ≤ 200
- [ ] `wc -l oriterm_core/src/term/handler/image/kitty/query.rs` ≤ 200
- [ ] `wc -l oriterm_core/src/term/handler/image/kitty/response.rs` ≤ 200
- [ ] `wc -l oriterm_core/src/term/handler/image/kitty/frame.rs` ≤ 200 (renamed from initial `frame_compose.rs` — `a=c` Compose reserves future `compose.rs`)
- [ ] `wc -l oriterm_core/src/term/handler/image/kitty/animate.rs` ≤ 200
- [ ] The monolith file is gone: `wc -l oriterm_core/src/term/handler/image/kitty.rs` returns error (file not found)
- [ ] The sibling animation file is gone: `wc -l oriterm_core/src/term/handler/image/kitty_animation.rs` returns error (file not found)
- [ ] Flat `delete.rs` file module is NOT present: `ls oriterm_core/src/term/handler/image/kitty/delete.rs 2>&1 | grep -q 'No such'` (directory module only)

### Behavior preservation (existing test suite)
- [ ] `cargo test -p oriterm_core` green (all pre-split tests still pass without modification)
- [ ] `cargo test --all` green
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green

### Semantic pin
- [ ] Parser-level tests in `oriterm_core/src/image/kitty/tests.rs` still pass unchanged — confirms `parse_kitty_command` is untouched.
- [ ] Integration test pattern: any existing spec_chain or teseq test that exercises kitty protocol via `handle_apc_dispatch` produces identical observable behavior pre/post split (tracked via the green full-suite run above — there are currently no direct kitty spec_chain tests; they land in §13.1+).

### Negative pin
- [ ] Grep for `mod kitty_animation` in `oriterm_core/src/term/handler/image/mod.rs` returns 0 matches — proves the consolidation happened (the old sibling module declaration is gone).
- [ ] Grep for `pub(super) fn kitty_finalize_payload`, `pub(super) fn kitty_accumulate_chunk`, `pub(super) fn kitty_decode_pixels`, `pub(super) fn kitty_respond` in the split files shows each defined in exactly ONE submodule (not duplicated across the split) — proves the mechanical move, not copy.

### Verify tests fail before fix
- [ ] The file-size invariants FAIL before the split (monolith is 480 lines, over the 200-line target — actually under the 500 hard limit but over the per-submodule target); after split, every submodule file is ≤ 200 lines.

---

## 2.5 Fix Plan TPR Findings

**Gate:** MANDATORY — Complexity-elevated subsystem (`oriterm_core/src/term/handler/` is VTE handler code per Phase 2.5 gate table).

- **TPR run**: 2026-04-21 / scratch dir `/tmp/tpr-round-ori_term-BqQGqHRH` / 1 round (max-rounds=1) / survivor mode (gemini 429 persistent).
- **Key findings**: 4 findings from codex, all verified against actual code and actionable:
  1. `[TPR-08-8-001-codex][critical]` — Step ordering creates fatal `kitty.rs`/`kitty/mod.rs` module resolution collision (both would exist simultaneously during Step 1 as originally written). **Fix landed**: Step 1 rewritten to use atomic `git mv kitty.rs kitty/mod.rs` — the monolith moves into the directory module in one operation; there is never a moment where both paths exist.
  2. `[TPR-08-8-002-codex][high]` — Per-action handlers are currently private (`fn kitty_transmit` etc. at `kitty.rs:65-479` — same impl block as dispatcher). After extraction into siblings, every cross-module call would fail without `pub(super)` promotion. Original plan did not address this. **Fix landed**: Step 2 now includes an explicit visibility-promotion matrix (13 methods, current→post-split visibility, callers) + a mandatory substep "Promote visibility in the monolith FIRST" BEFORE any extraction.
  3. `[TPR-08-8-003-codex][medium]` — File-size glob `wc -l kitty/*.rs` misses `kitty/delete/mod.rs` (one directory level deeper). **Fix landed**: replaced globally with `find oriterm_core/src/term/handler/image/kitty/ -name '*.rs' -exec wc -l {} +` which descends into all subdirectories.
  4. `[TPR-08-8-004-codex][medium]` — Plan retained `frame_compose.rs` references in several success criteria, affected-file entries, and implementation steps after the §1.5 rename decision. DRIFT per `.claude/rules/impl-hygiene.md` §SSOT. **Fix landed**: normalized all references via `replace_all` to `frame.rs`, then restored the historical decision wording in §1.5 R1 summary + the Final Agreed Approach block + §2 matrix (the 3 places where the OLD name is the correct citation).
- **Plan revisions**: Step 1 rewritten (atomic rename), Step 2 expanded with visibility-promotion matrix + per-file `cargo check` cadence, Step 3 reworded (no `kitty.rs` delete — renamed in Step 1), Step 5 renumbered to Step 4 after eliminating the redundant "delete monolith" step, file-size checks use `find` consistently.
- **Outcome**: findings resolved — proceed to Phase 3 (TDD).

---

## 3. Implementation

Six-step mechanical refactor, with `cargo check` run after each step to catch incremental breakage:

### Step 1: Atomic rename to directory module (AVOIDS kitty.rs/kitty/mod.rs collision)

**Critical step ordering**: Rust forbids `foo.rs` and `foo/mod.rs` both existing — the module resolver errors out. This step moves the monolith into the directory module in a SINGLE operation so there is never a moment where both exist:

- [ ] `git mv oriterm_core/src/term/handler/image/kitty.rs oriterm_core/src/term/handler/image/kitty/mod.rs` — atomic rename. Rust's `mod kitty;` declaration in `image/mod.rs:7` now resolves to the directory module without change.
- [ ] `cargo check -p oriterm_core` — should still compile green (same code, different file path).
- [ ] Add the submodule declarations to `kitty/mod.rs` (below the existing `use` imports, above the `impl` block): `mod animate; mod delete; mod frame; mod place; mod query; mod response; mod store; mod transmit;` (alphabetical per code-hygiene.md §Import Organization). These are empty declarations pointing at files that do not exist yet — DO NOT commit this intermediate state; it will not compile. Add them just before Step 2 creates each submodule file.

### Step 2: Extract per-action submodules (visibility promotion mandatory)

**Visibility promotion — MANDATORY BEFORE extraction**: All current per-action handlers in the monolith are PRIVATE (`fn kitty_transmit`, `fn kitty_query`, etc., `kitty.rs:65-479`) because they live in the same `impl<S: EffectSink> Term<S>` block as the dispatcher. After extraction into sibling submodules under `kitty/`, each method called from `kitty/mod.rs`'s dispatch OR from a sibling submodule MUST be promoted to `pub(super)`. Omitting this fails compilation.

**Visibility-promotion matrix** (every method called across submodule boundaries):

| Method | Current | Post-split | Callers (post-split) |
|---|---|---|---|
| `kitty_query` | `fn` (private) | `pub(super) fn` | `kitty/mod.rs::handle_kitty_graphics` |
| `kitty_transmit` | `fn` | `pub(super) fn` | `kitty/mod.rs::handle_kitty_graphics` |
| `kitty_transmit_and_place` | `fn` | `pub(super) fn` | `kitty/mod.rs::handle_kitty_graphics` |
| `kitty_place` | `fn` | `pub(super) fn` | `kitty/mod.rs::handle_kitty_graphics` |
| `kitty_delete` | `fn` | `pub(super) fn` | `kitty/mod.rs::handle_kitty_graphics` |
| `kitty_frame` | `pub(super) fn` (already) | stays `pub(super) fn` | `kitty/mod.rs::handle_kitty_graphics` |
| `kitty_animate` | `pub(super) fn` (already) | stays `pub(super) fn` | `kitty/mod.rs::handle_kitty_graphics` |
| `kitty_finalize_payload` | `pub(super) fn` (already) | stays `pub(super) fn` | lives in mod.rs; `transmit.rs`, `frame.rs` call `self.kitty_finalize_payload(...)` |
| `kitty_accumulate_chunk` | `pub(super) fn` (already) | stays `pub(super) fn` | lives in mod.rs; `transmit.rs`, `frame.rs` call `self.kitty_accumulate_chunk(...)` |
| `kitty_store_image` | `fn` | `pub(super) fn` | `transmit.rs::kitty_transmit`, `transmit.rs::kitty_transmit_and_place` |
| `kitty_decode_pixels` | `pub(super) fn` (already) | stays `pub(super) fn` | `store.rs::kitty_store_image`, `store.rs::kitty_store_from_file`, `frame.rs::kitty_frame` (via `Self::`) |
| `kitty_store_from_file` | `fn` | `pub(super) fn` | `store.rs::kitty_store_image` (same submodule — could stay private, but `pub(super)` is uniform and safe) |
| `kitty_create_placement` | `fn` | `pub(super) fn` | `place.rs::kitty_place`, `place.rs::kitty_transmit_and_place` (same submodule as `kitty_place`; uniform `pub(super)`) |
| `kitty_respond` | `pub(super) fn` (already) | stays `pub(super) fn` | every submodule (transmit, place, delete, store, query, frame, animate) |

`handle_kitty_graphics` at `kitty/mod.rs` keeps its current `pub(super) fn` (visible to `image/mod.rs::handle_apc_dispatch` at `image/mod.rs:26`). No public API surface change.

**Extraction substeps** (after visibility promotion in the monolith):

- [ ] **Promote visibility in the monolith FIRST** (still at `kitty/mod.rs` after the Step 1 rename). Edit each `fn kitty_foo` to `pub(super) fn kitty_foo` per the matrix above. `cargo check -p oriterm_core` — still compiles green (promoting to a wider visibility is always safe).
- [ ] Create `kitty/transmit.rs` with `kitty_transmit` (mod.rs:71-86), `kitty_transmit_and_place` (89-110). Uses `self.kitty_finalize_payload(...)` + `self.kitty_accumulate_chunk(...)` which stay in `mod.rs`. Keep the `impl<S: EffectSink> Term<S>` wrapper. Add `mod transmit;` to `kitty/mod.rs`. Remove the extracted method bodies from mod.rs. `cargo check -p oriterm_core`.
- [ ] Create `kitty/place.rs` with `kitty_place` (149-165), `kitty_create_placement` (403-463). Add `mod place;` to mod.rs; remove method bodies. `cargo check`.
- [ ] Create `kitty/delete/` DIRECTORY. Create `kitty/delete/mod.rs` with `kitty_delete` (168-253). **No semantic change** — verbatim copy. BUG-08-7 in a separate /fix-bug adds `delete/tests.rs` + the `#[cfg(test)] mod tests;` declaration + the per-specifier semantic fixes. DO NOT create a flat `delete.rs` file module; the directory module is required now per `.claude/rules/test-organization.md:16-24` since BUG-08-7 will add tests. Add `mod delete;` to kitty/mod.rs; remove method body. `cargo check`.
- [ ] Create `kitty/store.rs` with `kitty_store_image` (283-311), `kitty_decode_pixels` (314-345), `kitty_store_from_file` (348-400). `kitty_decode_pixels` is `pub(super)` and called from both `transmit.rs` (indirectly via `kitty_store_image`) and `frame.rs` (directly via `Self::kitty_decode_pixels`). Add `mod store;`; remove method bodies. `cargo check`.
- [ ] Create `kitty/query.rs` with `kitty_query` (65-68). Add `mod query;`; remove method body. `cargo check`.
- [ ] Create `kitty/response.rs` with `kitty_respond` (466-479). Add `mod response;`; remove method body. `cargo check`.
- [ ] Create `kitty/frame.rs` (NOT `frame_compose.rs` — renamed per codex consensus to reserve the `compose` name for future `a=c` Compose work) with `kitty_frame` from the sibling file at `oriterm_core/src/term/handler/image/kitty_animation.rs:26-78`. Uses `self.kitty_finalize_payload(...)` + `self.kitty_accumulate_chunk(...)` + `Self::kitty_decode_pixels(...)` — all from mod.rs + store.rs respectively. NO transmit.rs dependency. Add `mod frame;` to kitty/mod.rs. `cargo check`.
- [ ] Create `kitty/animate.rs` with `kitty_animate` (kitty_animation.rs:88-134). Uses `self.kitty_respond(...)` from response.rs. Add `mod animate;`. `cargo check`.

### Step 3: Consolidate kitty_animation.rs and update parent mod.rs
- [ ] After extracting `kitty_frame` and `kitty_animate` into `kitty/frame.rs` + `kitty/animate.rs` in Step 2, delete the now-empty `oriterm_core/src/term/handler/image/kitty_animation.rs`. (Its content is fully migrated.) NOTE: there is no `kitty.rs` to delete — Step 1's atomic `git mv` renamed it into `kitty/mod.rs`; `kitty.rs` has not existed since Step 1.
- [ ] Edit `oriterm_core/src/term/handler/image/mod.rs` — remove `mod kitty_animation;` (line 8). `mod kitty;` stays (line 7); Rust resolves to the new directory module automatically. `cargo check -p oriterm_core`.

### Step 4: Verify (full suite)
- [ ] `cargo check -p oriterm_core` — should compile with zero errors (no semantic change).
- [ ] `cargo test -p oriterm_core` — all pre-split tests pass unchanged.
- [ ] `cargo test --all` — workspace green.
- [ ] `./build-all.sh` — debug + release + Windows cross-compile green.
- [ ] `./clippy-all.sh` — green.
- [ ] `./test-all.sh` — green.
- [ ] `find oriterm_core/src/term/handler/image/kitty/ -name '*.rs' -exec wc -l {} +` — every output ≤ 200.

### Step 5: Commit
- [ ] Invoke `/commit-push` to land the refactor.

### Code sketch — kitty/mod.rs (dispatch + shared state-staging)

```rust
//! Kitty graphics protocol handler — action dispatch + shared state-staging.
//!
//! Per-action handlers live in submodules (transmit, place, delete, frame,
//! animate, query, response, store). This file owns the APC entry point,
//! the KittyAction match, and the chunked-upload + payload-finalization
//! helpers shared across transmit + frame.

use std::sync::Arc;

use log::warn;

use crate::effect::sink::EffectSink;
use crate::image::kitty::{KittyAction, KittyTransmission, LoadingImage, parse_kitty_command};
use crate::term::Term;

mod animate;
mod delete;
mod frame;
mod place;
mod query;
mod response;
mod store;
mod transmit;

/// Parameters for storing an image via Kitty protocol.
pub(super) struct KittyStoreParams {
    pub(super) image_id: u32,
    pub(super) payload: Vec<u8>,
    pub(super) format: u32,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) transmission: KittyTransmission,
}

impl<S: EffectSink> Term<S> {
    pub(super) fn handle_kitty_graphics(&mut self, data: &[u8]) { /* routes to per-action submodules */ }
    pub(super) fn kitty_finalize_payload(&mut self, cmd: &KittyCommand) -> KittyStoreParams { /* shared by transmit + frame */ }
    pub(super) fn kitty_accumulate_chunk(&mut self, cmd: KittyCommand) { /* shared by transmit + frame */ }
}
```

---

## R. Third Party Review Findings

(Initially empty — populated during Phase 5 completion checklist.)

---

## 4. Completion Checklist

- [ ] All file-size invariants met (§2 matrix)
- [ ] Behavior preservation verified (§2 matrix)
- [ ] Semantic pin + negative pin green (§2)
- [ ] Debug AND release builds pass (`cargo b && cargo b --release`)
- [ ] Windows cross-compile green (`cargo build --target x86_64-pc-windows-gnu`)
- [ ] `timeout 150 ./test-all.sh` green — no regressions
- [ ] `./clippy-all.sh` green
- [ ] `./build-all.sh` green
- [ ] `cargo test -p oriterm_core` green
- [ ] `/commit-push` — commit the refactor before review
- [ ] Plan TPR (Phase 2.5) — completed, findings resolved
- [ ] `/tpr-review` (Phase 5 — code review) passed — independent dual-source review of the refactor found no actionable findings
- [ ] `/impl-hygiene-review` passed — AFTER code `/tpr-review` is clean
- [ ] **Capability regression gate** — N/A. This refactor does not disable, remove, or weaken any capability; it redistributes existing code.
- [ ] `/improve-tooling` retrospective completed — capture any tooling gaps surfaced during the refactor
- [ ] Bug entry in `plans/bug-tracker/section-08-core-terminal.md:92` updated: `- [x]` with resolution details + commit SHA
- [ ] Fix section frontmatter `status` updated to `complete`
- [ ] Bug-tracker `00-overview.md` Quick Reference open bug count decremented for section 08
- [ ] Final `/commit-push` — commit closure artifacts

**Exit Criteria:** `find oriterm_core/src/term/handler/image/kitty/ -name '*.rs' -exec wc -l {} +` returns every file ≤ 200 lines; `wc -l oriterm_core/src/term/handler/image/kitty.rs` returns "no such file" (monolith deleted); `wc -l oriterm_core/src/term/handler/image/kitty_animation.rs` returns "no such file" (consolidated); `cargo test --all` green; `./build-all.sh` + `./clippy-all.sh` + `./test-all.sh` green; bug entry `- [x]` with fix commit SHA.
