---
bug: "BUG-04-004"
title: "Emoji in tab title vanishes after monitor transition"
severity: "medium"
status: complete
goal: "After a DPI change (monitor transition with different scale factor), all UI font collections retain the terminal font's emoji fallback so emoji in tab titles continue to render."
success_criteria:
  - "After `UiFontSizes::set_dpi` rebuilds collections, every collection in the registry still carries the injected emoji fallback face."
  - "After `App::handle_dpi_change`, an emoji codepoint resolved through any UI font size resolves to the fallback face (not the primary face's `notdef`)."
  - "Negative pin: a regression that removes the re-injection causes a test to fail explicitly."
subsystem: "oriterm/src/font/ui_font_sizes/mod.rs"
found: "2026-03-31"
source: "manual"
third_party_review:
  status: none
  updated: null
---

# Fix: BUG-04-004 — Emoji in tab title vanishes after monitor transition

**Status:** Complete (2026-04-25)
**Severity:** medium
**Goal:** After a DPI change, every UI font collection retains the terminal-font emoji fallback that was injected at renderer init, so emoji in tab titles continue to resolve to the color fallback face after the window moves between monitors with different scale factors.

**Success Criteria:**
- [ ] After `set_dpi` rebuilds all collections, every collection's `fallback_font_data().len()` equals the count that was injected at renderer init.
- [ ] After `ensure_size` adds a new collection at runtime, that collection also carries the injected emoji fallback.
- [ ] After `create_default_collection`, the standalone collection carries the injected emoji fallback.
- [ ] Negative pin: removing the re-injection causes a dedicated regression test to fail.

**Context:** The tab bar widget renders emoji icons by shaping the emoji string through the UI font path. The UI primary font (IBM Plex Mono) has no emoji glyphs; emoji is supplied by the terminal font's emoji fallback, injected once into UI collections via `UiFontSizes::inject_fallbacks` at `WindowRenderer::new` time. On a DPI change, `App::handle_dpi_change` calls `WindowRenderer::set_font_size`, which calls `UiFontSizes::set_dpi`, which calls `rebuild_all` — and `rebuild_all` recreates every collection from `font_set` (which has no emoji fallback) plus the `post_rebuild_hook` (which only reapplies user font config). The injected emoji fallback is lost. Subsequent emoji shaping resolves to a missing-glyph face on the primary font, so emoji disappears from the tab bar while ASCII titles still render.

---

## 1. Root Cause Analysis

- **Symptom**: Emoji in tab titles disappears after dragging the window between monitors with different DPI/scale factors. ASCII text continues to render.
- **Proximate cause**: `oriterm/src/font/ui_font_sizes/mod.rs` `rebuild_all` (lines 297–326) calls `self.collections.clear()` then constructs new `FontCollection`s from `self.font_set.clone()`. `self.font_set` is `FontSet::ui_embedded()` whose `fallbacks` is empty. The `post_rebuild_hook` (`apply_font_config`, `oriterm/src/app/config_reload/font_config.rs:19`) only reapplies user features, fallback meta, and codepoint maps — it does NOT re-inject the emoji fallback.
- **Root cause**: The emoji fallback is injected as a one-shot at `oriterm/src/gpu/window_renderer/mod.rs:150` via `sizes.inject_fallbacks(&emoji_data)` and applied directly to existing collections, but the injected data is not persisted on `UiFontSizes`. Rebuild paths therefore have no way to reapply it. **This is a SSOT violation** per `.claude/rules/impl-hygiene.md`: the "what fallbacks should every UI collection carry" knowledge has no canonical home — it lives transiently inside `inject_fallbacks` arguments and inside whichever collections existed at injection time.
- **Blast radius**:
  - All call sites that lose injected fallbacks on rebuild: `set_dpi` (DPI change), `ensure_size` (runtime size addition), `create_default_collection` (standalone default).
  - The bug only fires on rebuild paths — first-frame rendering after init is fine.
  - Cross-crate impact: none — entirely within `oriterm`.
- **Affected files**:
  - `oriterm/src/font/ui_font_sizes/mod.rs` — store injected fallbacks on `UiFontSizes`; reapply in `rebuild_all`, `ensure_size`, `create_default_collection`.
  - `oriterm/src/font/ui_font_sizes/tests.rs` — regression tests covering all three rebuild paths.

**Reference implementations**: Not applicable — this is a project-internal injection contract, not a protocol behavior.

---

## 1.5 Fix Consensus (via /tp-help)

- **Proposed approach (pre-consensus)**: Store the injected fallback `FontData` slice on `UiFontSizes` as an `injected_fallbacks: Vec<loading::FontData>` field. `inject_fallbacks` appends to this field AND continues to apply to existing collections via `append_fallback_data`. `rebuild_all`, `ensure_size`, and `create_default_collection` apply `fc.append_fallback_data(&self.injected_fallbacks)` after the post-rebuild hook so injected fallbacks survive every rebuild path.
- **tp-help run scratch dir**: `/tmp/tpr-round-ori_term-bXTifomE`

### Round 1

- **Codex summary**: SSOT pattern is architecturally right. Adjustment: the "run hook, then append injected fallbacks" sequence is repeated in three bodies → violates `impl-hygiene.md §Algorithmic DRY`; extract a private `finalize_collection` helper. TDD matrix needs: actual init order (hook installed BEFORE injection per `app/init/mod.rs:169-179`), repeated DPI transitions (count stays stable), repeated `inject_fallbacks` calls (idempotency or explicit one-shot), semantic pin for `scale_factor=1.0` path, and a config-reload cell (`rebuild_ui_font_sizes` in `config_reload/font_config.rs:181-202`). Double-inject risk: none in `rebuild_all` itself, but present at `inject_fallbacks` API boundary if called twice on the same registry.
- **Gemini summary**: SSOT pattern correct. `ensure_size` is the right side of the contract. Add "multiple injection" test case — relevant for GPU recovery path (`app/gpu_recovery.rs:90-113` preserves `UiFontSizes` across recovery and calls `WindowRenderer::new` again which would re-inject). Implementation guidance: dedupe via `Arc::ptr_eq` on `FontData.data` + `index` equality — makes `inject_fallbacks` idempotent.
- **Agreement points**:
  1. SSOT on `UiFontSizes` is architecturally correct.
  2. Alternative (A) — mutate `font_set.fallbacks` — correctly rejected (scale_factor=1.0 vs cap-height-normalized path divergence).
  3. `ensure_size` modification is the right contract.
  4. `inject_fallbacks` must be idempotent (both flagged unbounded growth / double-inject).
- **Disagreement points**: none — both reviewers agreed on the direction. Codex emphasized DRY extraction; Gemini emphasized idempotency implementation. Both are additive, not conflicting.
- **Independent code verification**:
  - Codex init-order claim: VERIFIED. `oriterm/src/app/init/mod.rs:169-179` installs `apply_font_config_to_ui_sizes` (hook) on `sizes`, then passes `sizes` to `WindowRenderer::new` which calls `inject_fallbacks` at `oriterm/src/gpu/window_renderer/mod.rs:150`. Hook is installed BEFORE injection.
  - Codex `rebuild_ui_font_sizes` claim: VERIFIED, and EXPANDS the bug scope. `oriterm/src/app/config_reload/font_config.rs:181-202` constructs a fresh `UiFontSizes::new(FontSet::ui_embedded(), ...)` on config reload and calls `renderer.replace_ui_font_sizes(ui_sizes)` — no re-injection call. Config-reload ALSO drops emoji fallback. Fix must cover this path.
  - Gemini GPU-recovery claim: VERIFIED as future path. `oriterm/src/app/gpu_recovery.rs:102-103` documents the intent to preserve `UiFontSizes` across a `Healthy → Recovering` transition and rebuild `WindowRenderer` (currently a 5.16.2 stub). When 5.16.3–5.16.8 land, `WindowRenderer::new` will be called against a surviving `UiFontSizes` → `inject_fallbacks` would run twice without dedup, doubling the Vec. Idempotency is mandatory.
- **Outcome**: **Agreement + persuaded divergence** — adopt the SSOT pattern as originally proposed, AND add three consensus refinements: (i) private `finalize_collection` helper on `UiFontSizes` (DRY), (ii) idempotent `inject_fallbacks` via `Arc::ptr_eq` + index dedup, (iii) fix the config-reload sister path at `WindowRenderer::replace_ui_font_sizes` via a small private helper on `WindowRenderer` that re-injects from `font_collection` into the new `ui_font_sizes`.

### Final agreed approach

1. **`UiFontSizes` layer** — preserve injected fallbacks across its own rebuild paths:
   - Add `injected_fallbacks: Vec<loading::FontData>` field.
   - `inject_fallbacks` is idempotent: for each input `FontData`, skip if any existing entry has `Arc::ptr_eq(&existing.data, &input.data) && existing.index == input.index`. Still apply to existing collections via `append_fallback_data` for the entries that were newly recorded (already-recorded entries are no-op across the whole op).
   - Private helper `finalize_collection(&self, fc: &mut FontCollection)`: runs `post_rebuild_hook` if set, then calls `fc.append_fallback_data(&self.injected_fallbacks)` when non-empty.
   - `rebuild_all`, `ensure_size`, `create_default_collection` all delegate to `finalize_collection` — collapses three copies of the "hook → append" scaffold into one (impl-hygiene §Algorithmic DRY).

2. **`WindowRenderer` layer** — re-inject emoji fallback whenever `ui_font_sizes` is (re)assigned:
   - Private helper `fn reinject_emoji_fallback(&mut self)`: extracts `self.font_collection.fallback_font_data()`; if non-empty AND `self.ui_font_sizes` is `Some`, calls `sizes.inject_fallbacks(&data)` (idempotent).
   - `WindowRenderer::new` calls `self.reinject_emoji_fallback()` immediately after constructing `Self` (replacing the current inline logic at `oriterm/src/gpu/window_renderer/mod.rs:145-152`).
   - `WindowRenderer::replace_ui_font_sizes` calls `self.reinject_emoji_fallback()` after `self.ui_font_sizes = Some(sizes)` so config-reload preserves emoji too.

This is a two-layer SSOT: `UiFontSizes` owns "what fallbacks every collection in this registry should have"; `WindowRenderer` owns "emoji fallback belongs on every ui_font_sizes paired with a terminal font_collection". Neither layer duplicates knowledge of the other.

---

## 2. TDD — Test Matrix

Write ALL tests BEFORE the fix. Verify they fail against current code. The matrix expanded from the pre-consensus plan to incorporate Codex + Gemini additions (init-order cell, repeated DPI transitions, repeated-injection idempotency, scale_factor=1.0 semantic pin, config-reload `replace_ui_font_sizes` cell).

### Exact failing case
- [ ] `set_dpi_preserves_injected_fallbacks`: after `set_dpi(192.0)` on a registry with an injected fallback, the default collection's `fallback_font_data().len()` equals the injected count.

### Edge cases
- [ ] `set_dpi_preserves_injected_fallbacks_all_collections`: EVERY collection in the registry (not just default) carries the injected fallback after rebuild.
- [ ] `set_dpi_noop_does_not_change_fallbacks`: `set_dpi` with the same DPI leaves collections' fallback lists identical.
- [ ] `inject_fallbacks_empty_data_is_noop`: empty slice → no change to `injected_fallbacks` slot or existing collections.

### Cross-pattern coverage (all rebuild paths)
- [ ] `rebuild_all_applies_injected_fallbacks`: `set_dpi` triggered rebuild reapplies injected fallback to all collections.
- [ ] `ensure_size_applies_injected_fallbacks`: runtime size addition after injection gets the fallback.
- [ ] `create_default_collection_applies_injected_fallbacks`: standalone default after injection carries the fallback.

### Cross-feature interactions (hook × injection × rebuild)
- [ ] `hook_and_injection_both_apply_after_rebuild`: post-rebuild hook (features) AND injected fallback both present after rebuild — neither is lost.
- [ ] `init_order_hook_first_then_injection_survives_dpi`: mirrors production order from `app/init/mod.rs:169-179` (hook installed on `sizes`, then `inject_fallbacks` called, then `set_dpi` rebuild). Both features and injected fallback present after DPI change.
- [ ] `replace_ui_font_sizes_re_injects_emoji`: on `WindowRenderer::replace_ui_font_sizes`, the new registry receives the emoji fallback from the renderer's `font_collection`. Covers the config-reload path (`rebuild_ui_font_sizes` in `config_reload/font_config.rs:181-202`).

### Idempotency coverage
- [ ] `inject_fallbacks_is_idempotent_same_arc`: calling `inject_fallbacks(&[fb])` twice with the same `Arc`-backed `FontData` leaves `injected_fallbacks.len() == 1` AND each collection's `fallback_font_data().len() == 1`.
- [ ] `repeated_dpi_transitions_do_not_grow_fallbacks`: 10× alternation `set_dpi(96.0) / set_dpi(192.0)` after one injection leaves the fallback count on every collection at exactly 1. Pins no-unbounded-growth.

### Semantic pin (scale_factor = 1.0)
- [ ] `injected_fallback_keeps_scale_factor_one_after_rebuild`: after DPI rebuild, the default collection's `fallback_meta[0].scale_factor == 1.0` (not the cap-height-normalized ratio that `FontCollection::new` would produce). Pins the rejection of alternative (A) — count-only tests would allow (A) to pass.

### Negative pin
- [ ] `rebuild_all_without_replay_loses_emoji` *(via an internal assertion pinned by the production code)*: `debug_assert!` inside `finalize_collection` that either (a) `injected_fallbacks` is empty, or (b) after finalization the collection's `fallback_font_data().len() >= self.injected_fallbacks.len()`. Detects a regression that forgets to replay.
- [ ] Concrete test: `rebuild_all_preserves_exact_fallback_bytes`: after rebuild, compare the `Arc`-backed bytes in the collection's fallback against the originally-injected bytes — proves same data, not just "some fallback". Rejects "replay replaced it with a different face" regressions.

### Verify tests fail before fix
- [ ] All new tests fail against current code (before the fix is applied), confirming they test the right semantics.

---

## 2.5 Fix Plan TPR Findings

**Gate:** Skipped — medium severity, non-elevated subsystem (font registry, not GPU pipeline / VTE / mux / IPC), expected round-1 consensus on a localized SSOT fix.

Plan TPR: Skipped per gate — medium severity, non-elevated subsystem, deterministic localized fix.

---

## 3. Implementation

### 3a. `UiFontSizes` — preserve and replay injected fallbacks

- [ ] Add `injected_fallbacks: Vec<super::collection::loading::FontData>` field (default empty) and initialize in `new`.
- [ ] Rewrite `inject_fallbacks` to be idempotent via `Arc::ptr_eq` + index dedupe:
  - For each input `FontData`, check `self.injected_fallbacks` for an existing entry with `Arc::ptr_eq(&existing.data, &input.data) && existing.index == input.index`.
  - If present → skip (both stored slot and existing-collection apply are no-ops for that entry).
  - If absent → record in `self.injected_fallbacks` via clone AND apply to every current collection via `fc.append_fallback_data(slice_of_one)`.
  - Net: calling `inject_fallbacks(&[X])` twice in a row leaves both `self.injected_fallbacks` and every `collection.fallback_font_data()` with exactly ONE entry for X.
- [ ] Extract private helper `finalize_collection(&self, fc: &mut FontCollection)`:
  1. If `self.post_rebuild_hook` is present, run it against `fc`.
  2. If `self.injected_fallbacks` is non-empty, call `fc.append_fallback_data(&self.injected_fallbacks)`.
- [ ] Replace the three inline "hook → append" sequences in `rebuild_all`, `ensure_size`, `create_default_collection` with a single `self.finalize_collection(&mut fc)` call. Collapses three algorithmic copies into one (impl-hygiene §Algorithmic DRY).

  ```rust
  // oriterm/src/font/ui_font_sizes/mod.rs
  pub(crate) struct UiFontSizes {
      // … existing fields …
      injected_fallbacks: Vec<super::collection::loading::FontData>,
  }

  impl UiFontSizes {
      pub(crate) fn inject_fallbacks(&mut self, data: &[super::collection::loading::FontData]) {
          for fd in data {
              let already = self.injected_fallbacks.iter().any(|existing| {
                  Arc::ptr_eq(&existing.data, &fd.data) && existing.index == fd.index
              });
              if already {
                  continue;
              }
              self.injected_fallbacks.push(fd.clone());
              let slice = std::slice::from_ref(fd);
              for fc in self.collections.values_mut() {
                  fc.append_fallback_data(slice);
              }
          }
      }

      fn finalize_collection(&self, fc: &mut FontCollection) {
          if let Some(ref hook) = self.post_rebuild_hook {
              hook(fc);
          }
          if !self.injected_fallbacks.is_empty() {
              fc.append_fallback_data(&self.injected_fallbacks);
          }
      }

      // rebuild_all, ensure_size, create_default_collection replace their
      // post_rebuild_hook blocks with `self.finalize_collection(&mut fc)`.
  }
  ```

### 3b. `WindowRenderer` — re-inject emoji on `ui_font_sizes` replacement

- [ ] Extract private helper `fn reinject_emoji_fallback(&mut self)` on `WindowRenderer`:
  - `let data = self.font_collection.fallback_font_data();`
  - `if !data.is_empty() { if let Some(ref mut sizes) = self.ui_font_sizes { sizes.inject_fallbacks(&data); } }`
- [ ] `WindowRenderer::new` (`oriterm/src/gpu/window_renderer/mod.rs:145-152`): replace the current inline injection with `Self { … }.tap(|s| s.reinject_emoji_fallback())`-equivalent pattern — i.e., build `Self` first, then call `self.reinject_emoji_fallback()` before the `log::info!` + return.
- [ ] `WindowRenderer::replace_ui_font_sizes` (`oriterm/src/gpu/window_renderer/font_config.rs:28-30`): after `self.ui_font_sizes = Some(sizes);`, call `self.reinject_emoji_fallback();`. This closes the config-reload path: `rebuild_ui_font_sizes` calls `renderer.replace_ui_font_sizes(ui_sizes)` which now re-injects emoji.

  ```rust
  // oriterm/src/gpu/window_renderer/font_config.rs
  pub fn replace_ui_font_sizes(&mut self, sizes: UiFontSizes) {
      self.ui_font_sizes = Some(sizes);
      self.reinject_emoji_fallback();
  }

  // private helper (module-local impl block)
  impl WindowRenderer {
      fn reinject_emoji_fallback(&mut self) {
          let data = self.font_collection.fallback_font_data();
          if data.is_empty() {
              return;
          }
          if let Some(ref mut sizes) = self.ui_font_sizes {
              sizes.inject_fallbacks(&data);
          }
      }
  }
  ```

### 3c. Sequencing notes

- The `UiFontSizes` changes (3a) are self-contained and pass tests independently.
- The `WindowRenderer` changes (3b) depend on `inject_fallbacks` being idempotent (3a) so that `WindowRenderer::new`'s existing post-construction call does not double-inject when layered on the new `replace_ui_font_sizes` path.
- Land 3a first so 3b can land against an idempotent `inject_fallbacks`. Within a single commit is fine; ordering matters only for the partial-landing case.

---

## R. Third Party Review Findings

Code TPR (Phase 5) — 4 rounds, exit reason: **clean**.

### Round 0 — 2026-04-24 (initial review of `3c510928`)

- **Dispatch**: codex 1 finding / gemini clean
- **Verification**: verified 1 / dropped 0
- **Classification**: actionable 1 / meta 0
- **Fix commit**: `3528b508`

- [x] `[TPR-04-004-codex][medium]` `oriterm/src/app/config_reload/mod.rs:187-197` — Config reload injects UI fallbacks from the OLD terminal font.
  Evidence: `rebuild_ui_font_sizes(renderer, ...)` (line 187) runs BEFORE `renderer.replace_font_collection(fc, gpu)` (line 197). Because `rebuild_ui_font_sizes` internally calls `renderer.replace_ui_font_sizes(new_ui_sizes)` which in the initial fix also called `self.reinject_emoji_fallback()`, the fresh UI registry picked up the OLD `self.font_collection`'s emoji fallback (the new `FontCollection` hadn't been installed yet). After config reload the UI would render emoji from the previous terminal font.
  Impact: config-reload path delivered stale emoji fallback when the terminal font family changed — a secondary path of the same SSOT violation BUG-04-004 targets.
  Resolution: moved `reinject_emoji_fallback()` from `replace_ui_font_sizes` (consumer-side trigger, fires before the source is ready) to `replace_font_collection` (source-side trigger, fires after the new terminal font is in place). `replace_ui_font_sizes` now only stores the new registry; `replace_font_collection` re-establishes the emoji wiring against the CURRENT `font_collection`. The invariant is order-independent: regardless of which of the two replace_* methods runs first during config reload, the reinject at the end of `replace_font_collection` pulls from the correct (newly installed) source.

### Round 1 — 2026-04-24 (review of `3528b508`)

- **Dispatch**: codex 1 finding / gemini clean
- **Verification**: verified 1 / dropped 0
- **Classification**: actionable 1 / meta 0
- **Fix commit**: `73ac90c8`

- [x] `[TPR-04-004-codex-r1][medium]` `oriterm/src/gpu/window_renderer/font_config.rs:27` — Source-side emoji reinjection lacks a regression test.
  Evidence: the round-0 fix moved `reinject_emoji_fallback()` to `replace_font_collection` but no test pinned the new ordering semantic. A future refactor that reverted the move could silently re-introduce the config-reload stale-emoji bug.
  Impact: without a regression pin, the source-side ordering invariant (`replace_ui_font_sizes` storage-only, `replace_font_collection` reinject trigger) depends on reviewer vigilance.
  Resolution: added gpu-tests-gated test `replace_font_collection_reinjects_emoji_into_current_ui_registry` in `oriterm/src/gpu/window_renderer/tests.rs`. The test mirrors the production config-reload call sequence and pins both halves of the ordering invariant — `replace_ui_font_sizes` must NOT inject, `replace_font_collection` must inject the NEW terminal font's emoji.

### Round 2 — 2026-04-24 (review of `73ac90c8`)

- **Dispatch**: codex 1 finding / gemini clean
- **Verification**: verified 1 / dropped 0
- **Classification**: actionable 1 / meta 0
- **Fix commit**: `e6fc75c8`

- [x] `[TPR-04-004-codex-r2][medium]` `oriterm/src/gpu/window_renderer/tests.rs:562` — Round-1 test missed new-source fallback identity.
  Evidence: the test asserted `after_replace_fc == 1` but did not verify the fallback came from the newly-installed `FontCollection`. A hypothetical broken reinject that left some unrelated fallback in the UI registry (carried from a previous source, loaded from a default chain) would still pass the count-only assertion.
  Impact: count-only pin allowed alternative-A-style implementations (mutating `font_set.fallbacks`) and stale-source regressions to slip past the regression gate.
  Resolution: captured the new terminal collection's fallback `Arc` BEFORE moving it into `replace_font_collection`, then assert `Arc::ptr_eq` between the captured Arc and the UI registry's post-reinject fallback. Turns count-smoke into source-identity semantic pin.

### Round 3 — 2026-04-24 (review of `e6fc75c8`)

- **Dispatch**: codex clean / gemini clean (1 informational — positive confirmation of the Arc::ptr_eq pin)
- **Verification**: verified 0 actionable findings
- **Classification**: 1 informational (no fix required)
- **Fix commit**: none — loop exiting clean

Both reviewers confirm the full chain `3c510928..e6fc75c8` is correct and comprehensive.

**Exit reason**: clean after 3 fix rounds + 1 confirmation round. All findings fixed inline; zero findings remain outstanding.

---

## H. Hygiene Review Findings

Post-TPR `/impl-hygiene-review` (Phase 5 step 4), focused scope on touched files.

**In-scope hygiene findings** (resolved inline):
- [x] BLOAT:banners × 2 — `oriterm/src/font/ui_font_sizes/tests.rs:275,281` used decorative `// ───` banners violating `code-hygiene.md §Comments`. Replaced with plain comment prefix.
- [x] BLOAT:fn-length — `oriterm/src/gpu/window_renderer/tests.rs:462` test function was 123 lines (limit 100). Extracted `build_terminal_fc`, `fresh_empty_ui_sizes`, and `ui_fallback_count` helpers; test body now well under the cap.

**Out-of-scope findings** (pre-existing, filed separately — not introduced by this fix):
- `BUG-06-015` — `oriterm/src/gpu/window_renderer/helpers.rs` 549 lines (already tracked).
- `BUG-06-021` — `oriterm/src/gpu/window_renderer/render.rs` `render_frame_cached` (line 77) and `render_cached` (line 321) nesting depth 5 (limit 4) — filed during this review.

No in-scope findings remain.

---

## I. Improve-Tooling Retrospective

Reflection on the diagnostic journey during this fix (per `/fix-bug` Phase 5 step 5):

- **Finding the root cause was fast** — reading `UiFontSizes::rebuild_all` + `inject_fallbacks` + the one-shot call site in `WindowRenderer::new` gave the full picture in ~15 minutes. The `inject_fallbacks` comment clearly said "inject into all collections in the registry" without mentioning rebuild-path persistence — the SSOT gap was visible from the doc comment alone once the question "what happens on rebuild?" was asked.

- **/tp-help consensus was load-bearing** — Codex identified the algorithmic DRY collapse (three copies of "hook → append") that I would likely have landed as-is without consensus. The 15-minute round surfaced a real code-quality improvement that would have been hard to catch in self-review.

- **/tpr-review round 0 caught the ordering bug (TPR-04-004-codex)** — the fix in `replace_ui_font_sizes` was local-optimum correct (it fixed the direct symptom — DPI change losing emoji) but globally wrong because the config-reload path had a different ordering constraint. This class of bug — "my fix for X subsystem broke Y subsystem" — is exactly what the mandatory code-review gate catches.

- **Hygiene lint was valuable and fast** — `bash .claude/skills/impl-hygiene-review/hygiene-lint.sh --scope <paths> --summary` ran in under 2 seconds and caught real cleanup work. Running it earlier in the fix flow (pre-commit, not post-TPR) would have cut review churn.

No tooling improvements actioned — the existing pipeline worked well for this fix. The hygiene lint's auto-fix mode (`--fix --apply`) is a resource I would reach for earlier next time.

---

## 4. Completion Checklist

- [ ] All new tests pass unchanged after fix (no test modifications needed)
- [ ] Matrix completeness verified — every rebuild path × interaction-with-hook combination has a test
- [ ] Debug AND release builds pass
- [ ] Windows cross-compile green
- [ ] `timeout 150 ./test-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./build-all.sh` green
- [ ] `cargo test -p oriterm` green
- [ ] `/commit-push` before review
- [ ] Plan TPR (Phase 2.5) — skipped per gate
- [ ] `/tpr-review` (Phase 5 — code review) passed
- [ ] `/impl-hygiene-review` passed
- [ ] `/improve-tooling` retrospective completed
- [ ] Bug entry in `plans/bug-tracker/section-04-fonts.md` updated to `- [x]` with resolution details
- [ ] Fix section frontmatter `status` updated to `complete`
- [ ] Overview count updated in `plans/bug-tracker/00-overview.md`
- [ ] Final `/commit-push`

**Exit Criteria:** `cargo test -p oriterm font::ui_font_sizes` runs the new regression tests (including `set_dpi_preserves_injected_fallbacks`, `ensure_size_applies_injected_fallbacks`, `create_default_collection_applies_injected_fallbacks`) and they pass. `./test-all.sh` is green. `./clippy-all.sh` is green. The bug entry in `section-04-fonts.md` is marked `- [x]` with resolution metadata.
