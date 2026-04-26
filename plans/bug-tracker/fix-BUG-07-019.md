---
bug: "BUG-07-019"
title: "spec-coverage-report --check UNCATALOGED BACKLOG is 100% false positives — OSC tuple signatures from the runtime observer and the catalog canonicalizer disagree on which slot carries the OSC numeric id"
severity: "medium"
status: in-progress
goal: "All four OSC tuple producers — parse_osc, dispatch_extract/osc.rs, capture_extract::osc_dispatch, perform_action_to_sig — produce identical TupleSig values for the same OSC sequence so spec-coverage-report --check correctly matches observed OSC tuples against catalog rows."
success_criteria:
  - "`cargo run -p oriterm_test_support --bin spec-coverage-report -- --check` exits 0 with empty UNCATALOGED BACKLOG when the spool contains only catalog-known OSC sequences"
  - "tuple/tests.rs has positive + negative pins fixing the OSC numeric-id-in-final_byte SSOT for all four producers"
  - "Distinct OSC commands (OSC-0, OSC-4, OSC-7, OSC-52, OSC-1337) produce distinct TupleSig values from every producer"
  - "test-all.sh can run `spec-coverage-report --check` AFTER cargo test (workaround removed)"
subsystem: "crates/oriterm_test_support/src/catalog/tuple/canonical.rs + dispatch_extract/osc.rs + capture_extract.rs + spec_chain/uncataloged/mod.rs"
found: "2026-04-21"
source: "nightly CI failure — test-all.sh integration of spec-coverage-report"
third_party_review:
  status: none
  updated: null
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
  - `crates/oriterm_test_support/src/catalog/capture_extract.rs` — `osc_dispatch` (lines 130-150): swap numeric id (parts[0]) and terminator
  - `crates/oriterm_test_support/src/catalog/tuple/tests.rs` — update `osc_title_canonicalizes_with_numeric_id_preserved`, `osc_4_palette_canonicalizes_with_index_placeholder`, `osc_numeric_id_must_not_collapse_to_ps`, `signature_normalizes_osc_st_to_bel`, `signature_preserves_bel_for_osc` to pin the new SSOT shape
  - `crates/oriterm_test_support/src/catalog/dispatch_extract/osc_tests.rs` (or wherever dispatch_extract OSC tests live — TBD during implementation)

**Reference implementations**: N/A — this is internal test-infrastructure SSOT, not a protocol question.

---

## 1.5 Fix Consensus (via /tp-help)

Independent dual-source design review of the proposed fix approach.

- **Proposed approach (pre-consensus)**: For all four OSC tuple producers, place the OSC numeric id (e.g. `"0"`, `"4"`, `"52"`, `"1337"`) in `final_byte`, and the remaining canonical parameters (e.g. `"text"`, `"index;rgb"`, `"mode;b64"`, `"key=value"`) in `params`. Drop the OSC|DCS ST→BEL normalization in `Tuple::signature()` since `final_byte` is no longer a terminator for either category. Update existing OSC pin tests to assert the new shape and add an SSOT-alignment matrix that drives the same OSC sequence through all four producers and asserts identical signatures.
- **tp-help run scratch dir**: pending

### Round 1
- pending

### Final agreed approach
pending

---

## 2. TDD — Test Matrix

pending

### Exact failing case
- [ ] OSC-52 dispatched at runtime produces a TupleSig that matches the OSC-52 catalog row's TupleSig

### Edge cases
- [ ] OSC-0 (empty payload) — `("OSC", [], "0")` from all four producers
- [ ] OSC-1337 (multi-digit numeric id, key=value payload) — `("OSC", [], "1337")` from all four producers
- [ ] OSC-7 (single param, no `;`) — `("OSC", [], "7")` from all four producers
- [ ] OSC-4 (multi-param: index;rgb) — `("OSC", [], "4")` from all four producers
- [ ] OSC with ST terminator (vs BEL) — same signature regardless of terminator (since terminator is no longer in final_byte)

### Cross-producer SSOT matrix (the key new test)
- [ ] For each OSC numeric id in {0, 4, 7, 10, 52, 104, 1337}: catalog `parse_osc` signature == dispatch `dispatch_extract` signature == capture `capture_extract` signature == runtime `perform_action_to_sig` signature

### Semantic pin
- [ ] `osc_signature_is_per_numeric_id_distinct` — OSC-52 and OSC-7 produce DIFFERENT TupleSig values (would have failed pre-fix with both equal to `("OSC", [], "BEL")`)

### Negative pin
- [ ] `osc_signature_does_not_collapse_to_terminator` — assert NO OSC TupleSig has `final_byte == "BEL"` or `final_byte == "ST"` after the fix

### Verify tests fail before fix
- [ ] All new tests fail against current code

---

## 2.5 Fix Plan TPR Findings

**Gate:** Skipped — medium severity, non-elevated subsystem (oriterm_test_support is test-helper infrastructure, not in the GPU / VTE / mux / IPC / platform list). Pending /tp-help round-1 outcome — if round 1 doesn't converge, escalate.

---

## 3. Implementation

pending — finalized after Phase 1.75 consensus

---

## R. Third Party Review Findings

(populated during Phase 5)

---

## 4. Completion Checklist

- [ ] All new tests pass unchanged after fix
- [ ] Matrix completeness verified — every OSC numeric id × every producer cell covered
- [ ] Debug AND release builds pass
- [ ] Windows cross-compile green (`cargo build --target x86_64-pc-windows-gnu`)
- [ ] `timeout 150 ./test-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./build-all.sh` green
- [ ] `cargo test -p oriterm_test_support` green
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check` exits 0 (verified post-test)
- [ ] `/commit-push` — commit all changes before review
- [ ] Plan TPR (Phase 2.5) — outcome recorded above
- [ ] `/tpr-review` (Phase 5 — code review) passed
- [ ] `/impl-hygiene-review` passed
- [ ] Capability regression gate — N/A (this fix RESTORES a capability rather than disabling one)
- [ ] `/improve-tooling` retrospective completed
- [ ] Bug entry in `plans/bug-tracker/section-07-ci-build.md` updated `- [x]`
- [ ] Fix section frontmatter `status: complete`
- [ ] Bug-tracker `00-overview.md` Quick Reference open bug count updated (07 from 4 → 3)
- [ ] If `test-all.sh` workaround can be removed (post-test ordering re-enabled), do so in this fix or file follow-up
- [ ] Final `/commit-push`

**Exit Criteria:** All four OSC tuple producers (parse_osc, dispatch_extract/osc.rs, capture_extract::osc_dispatch, perform_action_to_sig) produce identical `TupleSig` values for the same OSC sequence, verified by an SSOT-alignment matrix test that runs each of {OSC-0, OSC-4, OSC-7, OSC-10, OSC-52, OSC-104, OSC-1337} through all four producers and asserts equality. `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check` exits 0 with empty UNCATALOGED BACKLOG after a test that emits OSC-52 (or any cataloged OSC). `./test-all.sh`, `./build-all.sh`, `./clippy-all.sh` all green with no regressions.
