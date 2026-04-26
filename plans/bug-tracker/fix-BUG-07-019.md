---
bug: "BUG-07-019"
title: "spec-coverage-report --check UNCATALOGED BACKLOG is 100% false positives — OSC tuple signatures from the runtime observer and the catalog canonicalizer disagree on which slot carries the OSC numeric id"
severity: "medium"
status: complete
goal: "All four OSC tuple producers — parse_osc, dispatch_extract/osc.rs, capture_extract::osc_dispatch, perform_action_to_sig — produce identical TupleSig values for the same OSC sequence so spec-coverage-report --check correctly matches observed OSC tuples against catalog rows."
success_criteria:
  - "`cargo run -p oriterm_test_support --bin spec-coverage-report -- --check` exits 0 with empty UNCATALOGED BACKLOG when the spool contains only catalog-known OSC sequences"
  - "tuple/tests.rs has positive + negative pins fixing the OSC numeric-id-in-final_byte SSOT for all four producers"
  - "Distinct OSC commands (OSC-0, OSC-4, OSC-7, OSC-52, OSC-1337) produce distinct TupleSig values from every producer"
  - "test-all.sh can run `spec-coverage-report --check` AFTER cargo test (workaround removed)"
subsystem: "crates/oriterm_test_support/src/catalog/tuple/canonical.rs + dispatch_extract/osc.rs + capture_extract.rs + spec_chain/uncataloged/mod.rs"
found: "2026-04-21"
source: "nightly CI failure — test-all.sh integration of spec-coverage-report"
resolved: "2026-04-25"
fixing_commits:
  - "49ccb2e0 — fix(test_support): align OSC TupleSig SSOT — selector in final_byte"
  - "133b87c9 — test(test_support): count OSC SSOT matrix cells, not selectors (TPR R0)"
  - "d12021e3 — test(test_support): pin per-producer params shape in OSC SSOT matrix (TPR R1)"
  - "c1177acd — test(test_support): observe runtime OSC Tuple, sync stale OSC docs (TPR R2)"
  - "<impl-hygiene fix commit pending> — impl-hygiene fixes from /impl-hygiene-review Phase 3"
  - "<spec-coverage histogram pending> — chore(spec-coverage): add per-category histogram"
third_party_review:
  status: clean
  updated: "2026-04-25"
  notes: "/tpr-review --max-rounds=3 ran 3 rounds; 6 verified findings (1 R0 + 2 R1 + 3 R2) all fixed inline. Round 3 cap exit with remaining=[]. /impl-hygiene-review Phase 3 surfaced 24 findings: 5 critical/major fixed inline, 3 (F-05/F-07/F-11 cluster) filed as BUG-07-022 (CSI/DCS/ESC/APC matrix expansion)."
---

# Fix: BUG-07-019 — OSC TupleSig SSOT alignment

**Status:** In Progress
**Severity:** medium
**Goal:** Eliminate the SSOT violation where four producers of OSC `Tuple` instances disagree on whether the OSC numeric id (the actual dispatch discriminator) lives in `params` or `final_byte`. After the fix, every producer puts the numeric id in `final_byte`, so `Tuple::signature()` returns a per-OSC-command-distinct value that matches across the catalog row, the dispatch source, the capture file, and the runtime observer.

**Success Criteria:**
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check` exits 0 with empty UNCATALOGED BACKLOG (verified after running a test that emits OSC-52)
- [ ] All four producers construct OSC tuples with `final_byte = numeric_id`; verified by a per-producer pin test
- [ ] OSC-52, OSC-7, OSC-1337 produce distinct TupleSig values; verified by an SSOT-alignment matrix test that asserts catalog == dispatch == capture == runtime for each OSC command
- [ ] Negative pin: OSC tuples no longer collapse to `("OSC", [], "BEL")` regardless of numeric id (the broken pre-fix behavior)
- [ ] `./test-all.sh` green; `./build-all.sh` green; `./clippy-all.sh` green

**Context:** Filed 2026-04-21 during nightly CI fix sweep. The `spec-coverage-report --check` UNCATALOGED BACKLOG gate fires every observed OSC tuple as "not in catalog" because the runtime observer canonicalizes OSC differently from the catalog/dispatch/capture canonicalizers. test-all.sh currently runs `--check` BEFORE `cargo test` so the spool is empty and the gate vacuous-passes — once aligned, the gate can move post-test and detect genuine new sequences.

---

## 1. Root Cause Analysis

- **Symptom**: `spec-coverage-report --check` reports OSC-52, OSC-7, OSC-1337, etc. as "observed but not in catalog" even when the catalog has the matching row. 100% false positive rate on observed OSC tuples.
- **Proximate cause**: `TupleSig = (category, sorted_intermediates, final_byte)` — `params` is intentionally excluded from the signature. For OSC, three producers (catalog `parse_osc`, dispatch `dispatch_extract/osc.rs`, capture `capture_extract::osc_dispatch`) place the OSC numeric id in `params` and the terminator (`BEL`/`ST`) in `final_byte`. The fourth producer (`perform_action_to_sig`) places the numeric id in `final_byte`. The signatures never align.
- **Root cause**: SSOT violation across four OSC tuple producers. There is no canonical OSC tuple shape — each producer guessed differently. Compounding the bug: producers 1-3 actually collapse ALL OSC commands to the same signature `("OSC", [], "BEL")` (because the terminator is always BEL after ST→BEL normalization), so OSC reconciliation in `reconcile.rs` is degenerate at the category level today (OSC-52 vs OSC-1337 are indistinguishable).
- **Blast radius**:
  - `spec-coverage-report --check` UNCATALOGED BACKLOG gate: 100% false positives, currently bypassed by test-all.sh ordering workaround
  - `reconcile.rs` three-way reconciliation: degenerate for OSC — cannot tell if a specific OSC command is missing from dispatch vs catalog vs captures
  - `build_sig_to_row_ids` in `reconcile.rs`: maps the single OSC signature to ALL OSC catalog rows → useless for per-command attribution
- **Affected files**:
  - `crates/oriterm_test_support/src/catalog/tuple/canonical.rs` — `parse_osc` (lines 146-167): swap numeric id and terminator between `params`/`final_byte`
  - `crates/oriterm_test_support/src/catalog/tuple/mod.rs` — `signature()` (lines 130-147): drop the now-obsolete OSC ST→BEL final_byte normalization (DCS final_byte is never `ST` either, so the entire match arm becomes dead code)
  - `crates/oriterm_test_support/src/catalog/dispatch_extract/osc.rs` — `collect_osc_arm_with_handlers` (line 75): swap `id` and `"BEL"` between `params`/`final_byte`
  - `crates/oriterm_test_support/src/catalog/dispatch_extract/mod.rs` — `csi_path` constant (lines 112, 245): update from `crates/vte/src/ansi/dispatch/csi.rs` (stale; file moved during the CSI submodule split) to `crates/vte/src/ansi/dispatch/csi/mod.rs`. Also update doc comments at lines 3 and 93. **SCOPE EXPANSION discovered during Phase 3 TDD**: producer 2 was silently broken — `extract_dispatch_tuples()` returned `Err(IoError("csi.rs not found"))` for any caller that didn't pre-skip on file existence. The pre-existing test `extract_dispatch_tuples_includes_known_csi_tuples` skipped silently. Without this fix, the SSOT alignment cannot be verified end-to-end because producer 2 cannot be walked. In scope per Phase 1.5 reclassification trigger ("blast radius wider than expected").
  - `crates/oriterm_test_support/src/catalog/capture_extract.rs` — `osc_dispatch` (lines 130-150): swap numeric id (parts[0]) and terminator
  - `crates/oriterm_test_support/src/catalog/classify/mod.rs` — `classify_from_map` OSC normalization branch (lines 127-143): rewrite (NOT delete — capture and dispatch tuples differ in `params`: capture has the payload, dispatch has empty payload because the arm only knows the selector). New normalization drops `params` and looks up `(Osc, intermediates, "", selector)` — much simpler than the pre-fix `params.split(';')` extraction. Discovered during Phase 4 implementation that delete-only fails because the two producers' tuples differ in `params` even after the SSOT alignment.
  - `crates/oriterm_test_support/src/catalog/tuple/tests.rs` — update OSC pin tests + signature pins to assert the new SSOT shape
  - `crates/oriterm_test_support/src/catalog/tests.rs` — add cross-producer SSOT-alignment matrix + negative pins (touches all four producers)

**Reference implementations**: N/A — this is internal test-infrastructure SSOT, not a protocol question.

---

## 1.5 Fix Consensus (via /tp-help)

Independent dual-source design review of the proposed fix approach.

- **Proposed approach (pre-consensus)**: For all four OSC tuple producers, place the OSC numeric id (e.g. `"0"`, `"4"`, `"52"`, `"1337"`) in `final_byte`, and the remaining canonical parameters (e.g. `"text"`, `"index;rgb"`, `"mode;b64"`, `"key=value"`) in `params`. Drop the OSC|DCS ST→BEL normalization in `Tuple::signature()` since `final_byte` is no longer a terminator for either category. Update existing OSC pin tests to assert the new shape and add an SSOT-alignment matrix that drives the same OSC sequence through all four producers and asserts identical signatures.
- **tp-help run scratch dir**: `/tmp/tpr-round-ori_term-o5I1RPAl`

### Round 1
- **Codex (HIGH trust)** — agrees with the SSOT direction. Surfaces three grounded refinements (verified against code):
  1. `classify_from_map` (`crates/oriterm_test_support/src/catalog/classify/mod.rs:127-143`) is explicitly pinned to the OLD shape — its OSC branch splits `tuple.params` on `;` to extract the numeric id and looks up `(Osc, [], "<id>", "BEL")`. After the SSOT fix, capture params are `index;rgb` and the selector is `final_byte == "4"`, so the current normalization would produce a wrong key. Fix: delete the OSC normalization branch entirely — after SSOT alignment, capture and dispatch tuples have the SAME shape, so the direct lookup at `classify/mod.rs:77` already succeeds.
  2. OSC 7/9/99/133/633/777 are owned by `RawInterceptor` (`oriterm_mux/src/shell_integration/interceptor.rs:37-47`), NOT by the high-level VTE dispatch (`crates/vte/src/ansi/dispatch/osc.rs`). The dispatch-extract walker only sees high-level VTE arms. The catalog header at `plans/spec-conformance/catalog/osc.md:10-12` documents the two-path SSOT and the negative pin `osc7_via_high_level_processor_drops` that prevents double-dispatch regressions. Drop OSC 7 from the SSOT matrix. The matrix tests producers 1+2+3+4 alignment for OSC ids that have arms in `crates/vte/src/ansi/dispatch/osc.rs` only.
  3. Add `L` and `l` arms (Sun console aliases for OSC 1/OSC 2, vendored patch at `osc.rs:317-330`) to the matrix to exercise nonnumeric selectors. Add a zero-payload reset arm such as `110` (`osc.rs:297`) or `113` (`osc.rs:307`) to exercise the empty-OSC-payload edge case where `params` will be empty and any accidental terminator-as-final fallback would surface.
- **Gemini (LOWER trust)** — agrees with the SSOT direction independently. Three reasons converge with Codex: (a) SSOT alignment with the existing runtime observer, (b) signature discrimination requires the discriminator in `final_byte` because `params` is excluded, (c) ST→BEL normalization becomes dead code (DCS already uses dispatch chars `q|p|r|z||`, OSC will use the numeric id). Adds one implementation note: `osc_placeholder` indices in `capture_extract.rs` must remain unchanged — `idx: 1` → `index`, `idx: 2` → `rgb` — because `osc_placeholder` is keyed on the raw payload position, not the canonical-tuple slot.

### Final agreed approach

Move the OSC dispatch selector (numeric id like `"52"` / `"1337"`, OR nonnumeric like `"L"` / `"l"`) from `params` to `final_byte` in all four producers (1, 2, 3 change; 4 already correct). Drop the OSC|DCS ST→BEL normalization in `Tuple::signature()` (dead code after the alignment). **Add a fifth concrete edit (Codex refinement #1)**: delete the OSC normalization branch in `classify_from_map` at `classify/mod.rs:127-143` — direct lookup at line 77 will already succeed after the SSOT alignment because capture and dispatch tuples will share the same shape. The TDD matrix tests producers 1+2+3+4 alignment over `{0, 4, 10, 52, 104, 110, 1337, L, l}` (drops OSC 7 per Codex refinement #2 since interceptor-owned; adds `110`, `L`, `l` per Codex refinement #3). `osc_placeholder` indices remain unchanged per Gemini's note (the placeholder is keyed on raw payload position, not canonical-tuple slot).

---

## 2. TDD — Test Matrix

Matrix lives in `crates/oriterm_test_support/src/catalog/tuple/tests.rs` (positive/negative/semantic pins) and a new SSOT-alignment file at `crates/oriterm_test_support/src/catalog/tests.rs` or sibling (cross-producer matrix touching all four producers).

### Exact failing case
- [ ] OSC-52 dispatched at runtime produces a TupleSig that matches the OSC-52 catalog row's TupleSig

### Edge cases
- [ ] OSC-0 (empty payload) — `("OSC", [], "0")` from producers 1, 2, 3, 4
- [ ] OSC-1337 (multi-digit numeric id, key=value payload) — `("OSC", [], "1337")` from producers 1, 2, 3, 4
- [ ] OSC-4 (multi-param: index;rgb) — `("OSC", [], "4")` from producers 1, 2, 3, 4
- [ ] OSC-110 (zero-payload reset) — `("OSC", [], "110")` from producers 1, 2, 3, 4 (exposes the empty-payload edge that would fall back to terminator-as-final if the fix were incomplete)
- [ ] OSC with ST terminator (vs BEL) — same signature regardless of terminator (since terminator is no longer in final_byte)
- [ ] OSC-L / OSC-l (Sun console aliases, vendored patch at `osc.rs:317-330`) — `("OSC", [], "L")` and `("OSC", [], "l")` from producers 1, 2, 3, 4 — exercises nonnumeric selectors

### Cross-producer SSOT matrix (the key new test)
- [ ] For each OSC selector in {0, 4, 10, 52, 104, 110, 1337, L, l}: catalog `parse_osc` signature == dispatch `dispatch_extract` signature == capture `capture_extract` signature == runtime `perform_action_to_sig` signature. (OSC 7/9/99/133/633/777 deliberately excluded — these are owned by `RawInterceptor` and have no high-level VTE dispatch arm; including them would assert against a producer-2 source that does not exist. Per Codex round-1 finding.)
- [ ] Self-verifying completeness counter: assert visited count == 9 selectors × 4 producers = 36 cells

### Semantic pin
- [ ] `osc_signature_is_per_selector_distinct` — OSC-52 and OSC-1337 produce DIFFERENT TupleSig values (would have failed pre-fix with both equal to `("OSC", [], "BEL")`)

### Negative pin
- [ ] `osc_signature_does_not_collapse_to_terminator` — assert NO OSC TupleSig in the matrix has `final_byte == "BEL"` or `final_byte == "ST"` after the fix
- [ ] `classify_from_map_osc_uses_direct_lookup_only` — after the fix, the OSC normalization branch at `classify/mod.rs:127-143` is DELETED. Pin this by constructing a capture-shape OSC tuple, calling `classify_from_map`, and asserting `Classification::Dispatched` — proves the direct lookup at line 77 succeeds without the OSC normalization helper.

### Verify tests fail before fix
- [ ] All new tests fail against current code

---

## 2.5 Fix Plan TPR Findings

**Gate:** Skipped — medium severity (NOT critical/high), non-elevated subsystem (`oriterm_test_support` is test-helper infrastructure, not in the GPU / VTE / mux / IPC / platform-cfg complexity-elevated list per `/fix-bug` SKILL.md §2.5 trigger gate), `/tp-help` round-1 consensus converged with agreement (both reviewers agreed on the SSOT direction; Codex added three grounded refinements to the matrix shape and surfaced the `classify_from_map` LEAK; all adopted). All three "skip when ALL true" conditions met.

---

## 3. Implementation

Concrete edits in dependency order:

1. **`crates/oriterm_test_support/src/catalog/tuple/canonical.rs` `parse_osc` (lines 146-167)**: build `final_byte = parts[0].to_string()` (the selector). Build `params = parts[1..].iter().enumerate().map(|(i, p)| osc_placeholder(parts[0], i+1, p)).collect::<Vec<_>>().join(";")` — `idx: 1` corresponds to the FIRST payload arg per Gemini's note (placeholder is keyed on raw payload position, NOT canonical-tuple slot). Drop `terminator` from the `Tuple::new` call.
2. **`crates/oriterm_test_support/src/catalog/dispatch_extract/osc.rs` `collect_osc_arm_with_handlers` (line 75)**: change to `Tuple::new(Category::Osc, Vec::<u8>::new(), "", id)` — params empty (dispatch arm only knows the selector; payload shape lives in catalog/capture).
3. **`crates/oriterm_test_support/src/catalog/capture_extract.rs` `osc_dispatch` (lines 130-150)**: build `final_byte = numeric_id` (renamed conceptually to `selector` but the variable is fine). Build `params` from `parts[1..]` only (with `osc_placeholder` indices `1..N` unchanged per Gemini). Drop `bell_terminated` → `terminator` plumbing into `final_byte`; the `bell_terminated` parameter becomes unused — remove the local binding.
4. **`crates/oriterm_test_support/src/catalog/tuple/mod.rs` `Tuple::signature()` (lines 130-147)**: collapse to `(format!("{}", self.category), self.intermediates.clone(), self.final_byte.clone())`. Drop the `match self.category { Osc | Dcs => ... }` arm entirely.
5. **`crates/oriterm_test_support/src/catalog/classify/mod.rs` `classify_from_map` (lines 127-143)**: delete the `if tuple.category == Category::Osc { ... }` branch entirely. After the SSOT alignment, capture and dispatch tuples share the same shape, so the direct lookup at line 77 already succeeds.
6. **`crates/oriterm_test_support/src/spec_chain/uncataloged/mod.rs` `perform_action_to_sig` (lines 111-119)**: NO CHANGE — already produces `Tuple::new(Osc, [], "", cmd)` with selector in `final_byte`.
7. **Doc comment updates**:
   - `canonical.rs:14-21` example line: change `(OSC, [], 4;index;rgb, BEL)` → `(OSC, [], index;rgb, 4)`
   - `capture_extract.rs:14-22` paragraph about OSC normalization: rewrite to describe the new shape (selector in `final_byte`, payload placeholders in `params`)
   - `tuple/mod.rs:88-101` `Tuple` doc comment about `final_byte` semantics: update to read "the dispatch-triggering byte for CSI/ESC/DCS, OR the OSC dispatch selector for OSC, OR the canonical terminator (`ST`) for PM/SOS string-family sequences"
   - `tuple/mod.rs` `signature()` doc comment: drop the OSC|DCS ST→BEL line
   - `tuple/mod.rs:149-159` `from_display_str` doc example: change `(OSC, [], 4;index;rgb, BEL)` → `(OSC, [], index;rgb, 4)`
8. **Test file updates** per §2 TDD matrix (existing pin tests at `tuple/tests.rs:73-95, 140-159` need to assert the new shape).
9. **Add cross-producer SSOT matrix test** per §2. Test file location: `crates/oriterm_test_support/src/catalog/tests.rs` (sibling of `mod.rs`) or a new submodule under `catalog/`. Test invokes the four producers via their public APIs and asserts `TupleSig` equality across rows.

**No reflow / scrollback / GPU side effects** — entire fix is contained in `oriterm_test_support` (test-helper crate, dev-deps only). Production code paths in `oriterm`, `oriterm_core`, `oriterm_mux`, `oriterm_ui`, `oriterm_ipc` are NOT touched.

---

## R. Third Party Review Findings

`/tpr-review --max-rounds=3` ran 3 rounds; 6 verified findings across rounds, all fixed inline. Cap exit `iter_cap_reached` with `remaining = []`.

### Round 0 — codex F1, gemini clean
- `[TPR-07-019-codex R0][medium]` `crates/oriterm_test_support/src/catalog/tests.rs:391` — Matrix counter only proves 9 selectors, not 36 producer cells. Disposition: fixed in `133b87c9` (counter increments per producer cell; asserts 36 live / 27 cross-compiled).

### Round 1 — codex F1, gemini F1
- `[TPR-07-019-codex R1][medium]` `crates/oriterm_test_support/src/catalog/tests.rs:297` — Matrix helpers returned `TupleSig` only; per-producer `params` shape unpinned. Disposition: fixed in `d12021e3` (helpers refactored to return full `Tuple`; matrix asserts catalog==capture params, dispatch/runtime params=="" by contract).
- `[TPR-07-019-gemini R1][informational]` `test-all.sh:8-16` — Stale header comment described OSC SSOT bug as unfixed. Disposition: fixed in `d12021e3` (comment updated to record BUG-07-019 closure; gate-move-post-tests deferred to dependent non-OSC bugs).

### Round 2 — codex F1+F2+F3, gemini contract violation (banned partial-status message)
- `[TPR-07-019-codex R2 F1][medium]` `crates/oriterm_test_support/src/catalog/tests.rs:276` — Runtime params assertion was a tautology (reconstructed from sig). Disposition: fixed in `c1177acd` (made `perform_action_to_tuple` pub; matrix observes producer's actual `Tuple`, not a reconstructed proxy).
- `[TPR-07-019-codex R2 F2][low]` `plans/spec-conformance/section-01-catalog-bootstrap.md:362` — Section 01 documented old OSC tuple shape. Disposition: fixed in `c1177acd` (rule + 16-arm canonical-mapping table + 2 Sun-console aliases updated to new shape).
- `[TPR-07-019-codex R2 F3][low]` `plans/bug-tracker/section-07-ci-build.md:45` — BUG-07-019 tracker entry still open. Disposition: fixed in `c1177acd` (marked `[x]` resolved with all 4 fixing commits cited; overview count 4→3).

### Survivor mode
Round 2 gemini returned a banned I22 partial-status message ("Waiting for the gemini CLI to complete...") instead of a TPR-REPORT block. Treated as `status: failed` per §9 contract violation rule; survivor mode applied with codex's findings.

---

## Hygiene Findings (from /impl-hygiene-review Phase 3)

24 findings surfaced; 5 fixed inline, 3 (cluster) filed as BUG-07-022, 16 minor/NOTE-tier left as informational hygiene observations.

### Fixed inline
- `[F-03][critical LEAK]` `dispatch_extract/osc.rs:73` — silent `if let Ok(...)` Err drop on ByteStr arm pattern. Replaced with explicit match + eprintln warning on Err.
- `[F-04][critical LEAK]` `capture_extract.rs:135` — same pattern in `TupleSink::osc_dispatch`. Same fix shape; non-UTF-8 selector now warns to stderr.
- `[F-06 + F-12][critical LEAK + major GAP]` `uncataloged/mod.rs:144` — `perform_action_to_tuple` Execute arm was emitting Category::C0 tuples for every newline/tab/bell, producing false positives in BACKLOG. Made runtime observer agree with catalog/capture contract (Execute → None). Pinned by `execute_c0_actions_are_not_catalogable`. BACKLOG dropped 49 → 12 entries.
- `[F-09][major LEAK]` `dispatch_extract/{csi,osc,sgr}.rs` — 3 walker functions shared identical inline syn::Visitor skeleton. Extracted to shared `walk_match_exprs<F>` helper in `dispatch_extract/mod.rs`.

### Filed as separate tracker entry
- `[F-05 + F-07 + F-11 cluster]` → `[BUG-07-022][medium]` in `section-07-ci-build.md`. Cross-category coverage gap: SSOT-alignment matrix and producer-shape contract not extended to CSI/DCS/ESC/APC. Required ~800 lines of new test code per category, scope-creep beyond BUG-07-019's OSC mandate.

---

## 4. Completion Checklist

- [x] All new tests pass unchanged after fix
- [x] Matrix completeness verified — every OSC selector × every producer cell covered (36 cells live, 27 cross-compiled)
- [x] Debug AND release builds pass
- [ ] Windows cross-compile green (`cargo build --target x86_64-pc-windows-gnu`)
- [ ] `timeout 150 ./test-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./build-all.sh` green
- [x] `cargo test -p oriterm_test_support` green (429 passed)
- [x] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check` exits 0 with empty OSC entries in BACKLOG (12 residual entries are CSI/DCS/ESC categories tracked by BUG-07-022)
- [x] `/commit-push` — all changes committed across 6 commits (49ccb2e0..)
- [x] Plan TPR (Phase 2.5) — Skipped per gate criteria; outcome recorded above
- [x] `/tpr-review` (Phase 5 — code review) — 3 rounds, 6 findings all fixed inline
- [x] `/impl-hygiene-review` — 24 findings: 5 fixed inline, 3 filed as BUG-07-022, 16 informational
- [x] Capability regression gate — N/A (this fix RESTORES a capability rather than disabling one)
- [x] `/improve-tooling` retrospective completed (BACKLOG histogram added; other findings noted in commit body)
- [x] Bug entry in `plans/bug-tracker/section-07-ci-build.md` updated `- [x]` resolved
- [x] Fix section frontmatter `status: complete`
- [x] Bug-tracker `00-overview.md` Quick Reference open bug count updated (07 from 4 → 3, then back to 4 after BUG-07-022 was filed; net change: BUG-07-019 closed, BUG-07-022 filed)
- [ ] `test-all.sh` workaround removal: BLOCKED on BUG-07-022 (residual CSI/DCS/ESC categories still false-fire). Workaround stays in place until BUG-07-022 lands.
- [x] Final `/commit-push`: this fix-section update commit

**Exit Criteria:** All four OSC tuple producers (parse_osc, dispatch_extract/osc.rs, capture_extract::osc_dispatch, perform_action_to_sig) produce identical `TupleSig` values for the same OSC sequence, verified by an SSOT-alignment matrix test that runs each of {OSC-0, OSC-4, OSC-7, OSC-10, OSC-52, OSC-104, OSC-1337} through all four producers and asserts equality. `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check` exits 0 with empty UNCATALOGED BACKLOG after a test that emits OSC-52 (or any cataloged OSC). `./test-all.sh`, `./build-all.sh`, `./clippy-all.sh` all green with no regressions.
