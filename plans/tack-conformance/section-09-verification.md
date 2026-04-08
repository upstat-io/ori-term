---
section: "09"
title: "Verification: Test Matrix, Cross-Platform, Performance"
status: not-started
reviewed: false
goal: "Final verification gate for the entire tack-conformance plan. Cross-validate tack scenarios against vttest scenarios where they overlap (DA/DSR responses), confirm cross-platform skip discipline holds (compile + run on Linux, compile + skip on Windows native, compile + run on macOS), confirm performance invariants are not regressed (zero idle CPU, no allocation regressions in the hot render path), and run the full test suite end-to-end. Closes the plan."
success_criteria:
  - "Cross-validation matrix: every DA/DSR response captured by tack scenarios in Section 06 matches the same response asserted by vttest menu6 (oriterm_core/tests/vttest/menu6.rs). Document the diff (should be empty)."
  - "Cross-platform skip matrix: documented for Linux, macOS, Windows. Each row shows: tic available?, tack available?, infocmp available?, GPU adapter available?, expected pass/skip behavior."
  - "All Section 01-08 deliverables verified: shared PtySession, terminfo provisioning, scenario framework, 50+ catalog scenarios (18 test_menu + ~12 tools_menu active + ~23 direct-VTE cap xcheck from Section 06 Track B), 6 GPU goldens, full kf1-kf63 keyboard cross-check"
  - "Bounded-poll invariants verified for the new Section 04 primitives: `PtySession::wait_for_with_context` (delegates to the same loop body as `wait_for`, no parallel poll loop) and `PtySession::quit_tack` (state-aware loop that observes `try_wait()` after every `q\\n`, panics on max-iteration overflow). Section 09 runs the unit tests added by 04.0 (`pty_session_wait_for_with_context_uses_custom_message`, `pty_session_quit_tack_returns_status_when_child_exits`, `pty_session_quit_tack_panics_on_max_iterations`) and confirms they assert the contract end-to-end"
  - "Per-scenario `tic` compile cost decision (Mi2): after Sections 05/06/07 land, measure `./test-all.sh` wall-clock and decide whether to add the `OnceLock` `tic` cache called out in Section 04's `runner.rs` Mi2 lever. If the regression vs. pre-tack-conformance baseline exceeds 10s wall-clock, file `/add-bug` and fix in Section 09 — do NOT defer to a follow-up plan (the lever exists in Section 04's docs precisely so Section 09 can pull it without scope creep)"
  - "`./test-all.sh` green: vttest text + vttest GPU goldens + tack text + tack GPU goldens + keyboard terminfo_xcheck all pass"
  - "`./build-all.sh` green: workspace + cross-compile to `x86_64-pc-windows-gnu`"
  - "`./clippy-all.sh` green: zero new warnings, zero new `#[allow(clippy::...)]`"
  - "Performance invariants from CLAUDE.md still hold: zero idle CPU beyond cursor blink, zero allocations in hot render path, stable RSS under sustained output. None of the tack tests touch the production render loop, so any regression here is unrelated to this plan and is a separate bug."
  - "00-overview.md mission success criteria: ALL ticked"
  - "Plan archival: 00-overview.md status: complete, index.md status: resolved, plan moved to plans/completed/tack-conformance/"
  - "Bug-tracker entries from Sections 01-08 (if any /add-bug filings during plan execution) all resolved or appropriately deferred to follow-up plans"
inspired_by:
  - "ori_term Section 7 verification template (plans/completed/teseq-conformance/section-07-verification.md — final verification structure)"
  - "ori_term vttest size matrix (oriterm_core/tests/vttest/menu*.rs — cross-size validation we mirror here)"
depends_on: ["01", "02", "03", "04", "05", "06", "07", "08"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "09.1"
    title: "Test matrix: every scenario, every size, every platform"
    status: not-started
  - id: "09.2"
    title: "Cross-validation: tack DA/DSR vs vttest menu6"
    status: not-started
  - id: "09.3"
    title: "Performance regression check"
    status: not-started
  - id: "09.4"
    title: "Cross-platform build + skip verification"
    status: not-started
  - id: "09.5"
    title: "Plan archival and frontmatter gate"
    status: not-started
  - id: "09.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "09.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 09: Verification

**Status:** Not Started
**Goal:** Final verification gate. Confirm every Section 01-08 deliverable lands cleanly, cross-validate the tack scenarios against the existing vttest scenarios where they overlap, confirm the cross-platform skip matrix holds, confirm performance invariants are not regressed, archive the plan to `plans/completed/`.

**Success Criteria:**

- [ ] Test matrix table populated: every scenario from Sections 05-08 listed with its sizes and current pass/fail/skip status
- [ ] DA/DSR cross-validation diff is empty (or differences documented as bugs)
- [ ] Cross-platform skip matrix table documents tic/tack/infocmp/GPU availability per platform
- [ ] `./test-all.sh`, `./build-all.sh`, `./clippy-all.sh` all green on Linux
- [ ] Cross-compile for `x86_64-pc-windows-gnu` succeeds
- [ ] No regressions in performance tests (alloc_regression, event_loop_helpers tests)
- [ ] All mission criteria in `00-overview.md` checked
- [ ] Plan archived to `plans/completed/tack-conformance/`

**Context:** Sections 01-08 build the conformance suite from foundation up. Section 09 is the gate that proves everything works together — no half-finished cross-section work, no stale assumptions, no skipped platforms. The point is to leave the plan in a state where any developer 6 months from now can run `./test-all.sh` and trust the result.

**Reference implementations:**
- **ori_term teseq verification** `plans/completed/teseq-conformance/section-07-verification.md`: structural template followed here.
- **CLAUDE.md Performance Invariants section**: zero idle CPU, zero alloc in hot path, stable RSS. The tack tests don't touch the production render loop, so any regression here is unrelated — but verify regardless.

**Depends on:** Sections 01-08 ALL complete.

---

## 09.1 Test matrix: every scenario, every size, every platform

**File(s):** None (verification + documentation in this section file)

Run every scenario added by the plan and tabulate the results. Build the matrix in this file as the work proceeds.

- [ ] Run vttest text tests (post-Section-01 dedup): `timeout 150 cargo test -p oriterm_core --test vttest`. Expected: 198 scenarios all pass, zero `.snap.new` files, zero pixel diffs.
- [ ] Run vttest GPU goldens: `timeout 150 cargo test -p oriterm --features gpu-tests -- vttest_golden`. Expected: 98 PNG goldens all match.
- [ ] Run tack text scenarios: `timeout 150 cargo test -p oriterm_core --test tack`. Expected: smoke (1) + test_menu 18 scenarios (modes x7, acs, gr, color x3, cursor x3, pad_timing, send_strings, labels) + tools_menu ~12 active `#[test] fn`s (tools_menu_inventory discovery + status_reports_inventory discovery + ~8 status_reports per-sub-test scenarios covering DA1/DA2/DA3/DSR status/DSR CPR/DECRQM + sgr_modes + character_sets G0 DEC graphics + enq_ack) + 6 doc-only exclusion stubs (no #[test] fn bodies) = **~31 active scenarios + 6 stubs** all pass. Plus direct-VTE cap xcheck: `timeout 150 cargo test -p oriterm_core -- term::handler::tack_cap_xcheck` runs **~23 cap tests + meta-test** deterministically (Section 06 direct-VTE Track B).
<!-- reviewed: cohesion fix (Agent 2 Section 06 review) — updated Section 09 scenario counts to match Section 06's final M2 catalog (old "7 tools_menu scenarios" was based on the pre-rewrite draft). -->

- [ ] Run tack GPU goldens: `timeout 150 cargo test -p oriterm --features gpu-tests -- tack_golden`. Expected: **6 PNG goldens** all match (color x3, graphic_rendition, character_sets, modes).
- [ ] Run keyboard cross-check: `timeout 150 cargo test -p oriterm key_encoding::terminfo_xcheck` (preferred in-crate path; fallback `--test keyboard_terminfo` only if Section 08's integration-test fallback was taken). Expected test functions: `function_keys_match_terminfo` (kf1-kf12), `function_keys_shift_match_terminfo` (kf13-kf24), `function_keys_ctrl_match_terminfo` (kf25-kf36), `function_keys_ctrl_shift_match_terminfo` (kf37-kf48), `function_keys_alt_match_terminfo` (kf49-kf60), `function_keys_alt_shift_match_terminfo` (kf61-kf63), `cursor_keys_app_mode_match_terminfo`, `cursor_keys_normal_mode_emit_csi`, `editing_keys_match_terminfo`, `infocmp_query_returns_none_for_cap_not_in_ori_term` = **10 test functions, ~80+ individual assertions**.
- [ ] Run tack smoke test: `timeout 150 cargo test -p oriterm_core --test tack -- tack_smoke_main_menu_at_80x24`. Expected: passes deterministically.
- [ ] Run terminfo unit tests: `cargo test -p oriterm_test_support`. Expected: all unit tests pass — including the new Section 04 primitives (`pty_session_wait_for_with_context_uses_custom_message`, `pty_session_quit_tack_returns_status_when_child_exits`, `pty_session_quit_tack_panics_on_max_iterations`) plus the `parser/tests.rs` token-helper tests (`grid_has_token_rejects_substring_collision`, etc.) and the `navigator/tests.rs` pre-existing-anchor-guard tests.

- [ ] **Bounded-poll invariant pin for new Section 04 primitives.** `PtySession::wait_for_with_context` is the canonical wait loop body (the original `wait_for` delegates to it — no parallel implementation), and `PtySession::quit_tack` polls `try_wait()` after every `q\n` send. Both must NOT hot-spin on the `Ok(None)` branch. Verify the unit tests added in 04.0 cover this:
  1. `pty_session_wait_for_with_context_uses_custom_message` — confirms the closure-based panic message contract holds.
  2. `pty_session_quit_tack_returns_status_when_child_exits` — confirms the state-aware loop exits the moment the child terminates.
  3. `pty_session_quit_tack_panics_on_max_iterations` — confirms the runaway-child path panics with a diagnostic instead of looping forever.
  If any of these tests are missing or skip on the current platform without a sibling that runs there, file `/add-bug` and treat as Section 09 blocker.

- [ ] **Per-scenario `tic` compile cost (Mi2 from Section 04).** Section 04's `runner.rs` documents that each `ScenarioRunner::run_at` call invokes `TerminfoEnv::compile()` which shells out to `tic`. With ~30 scenarios × 3 sizes that's ~90 `tic` invocations per `./test-all.sh` run. Measure:
  ```
  /usr/bin/time -v timeout 600 ./test-all.sh 2>&1 | tee /tmp/test-all-after-tack.log
  ```
  Compare wall-clock against the pre-Section-01 baseline (committed in `plans/tack-conformance/baselines/test-all-pre-tack.log` if present, otherwise estimate from CI history). If the regression exceeds 10s, pull the Mi2 lever NOW (do not defer to a follow-up plan):
  - Add a `OnceLock<TerminfoEnv>` cache to `crates/oriterm_test_support/src/terminfo/mod.rs` keyed on `TerminfoVariant`.
  - Update `ScenarioRunner::run_at` to fetch from the cache instead of calling `TerminfoEnv::compile()` directly.
  - Re-run the timing comparison and confirm the regression is below 10s.
  - File a `/fix-bug` section if the cache work is non-trivial; otherwise inline.

- [ ] Compile the test matrix table into this section file:

  | Suite | Scenarios | Sizes | Linux | macOS | Windows |
  |-------|-----------|-------|-------|-------|---------|
  | vttest text (post-dedup) | ~17 menu tests | 80x24, 97x33, 120x40 | ✓ | ? | ✓ |
  | vttest GPU goldens | ~98 frames | 80x24 (mostly) | ✓ | ? | gpu-skip |
  | tack smoke | 1 | 80x24 | ✓ | ? | tools-skip |
  | tack test_menu | 18 | 80x24 + 97x33 + 120x40 (color/cursor) | ✓ | ? | tools-skip |
  | tack tools_menu | ~12 active + 6 stubs | 80x24 | ✓ | ? | tools-skip |
  | tack direct-VTE cap xcheck (Section 06 Track B) | 23 caps + meta-test | n/a | ✓ | ✓ | ✓ |
  | tack GPU goldens | 6 | 80x24 + 97x33 + 120x40 (color) | ✓ | ? | tools-skip OR gpu-skip |
  | keyboard terminfo_xcheck | kf1-kf63 + cursor (app+normal) + editing | n/a | ✓ | ? | tools-skip |


  Replace `?` with actual results once verified. macOS and Windows columns may need to be filled in by running on those platforms — if local cross-compilation doesn't include a real Windows test run, document that the Windows column is "compile-verified, runtime not exercised in this plan; CI matrix in follow-up infrastructure work will confirm."

---

## 09.2 Cross-validation: tack DA/DSR vs vttest menu6

**File(s):** None (verification only)

vttest menu6 already asserts DA/DSR/DECRQM responses (`oriterm_core/tests/vttest/menu6.rs`). Section 06 added tack scenarios that capture the SAME responses via the tools menu. If the two paths see different responses, ori_term is non-deterministic in its responses (which is a bug) OR one of the test paths is misreading the response.

- [ ] Read both test paths' captured responses:
  - vttest menu6 asserts via `walk_menu6_subscreens` — what response strings does it assert against?
  - tack tools_menu/status_reports captures the responses in the insta snapshot. Inspect the snapshot files for the DA1/DA2/DSR snapshots from Section 06.
- [ ] Diff the responses. They should be IDENTICAL (modulo formatting differences in how each test path reports them).
- [ ] If they differ:
  - Identify which one is correct (compare against ori_term's `term_handler.rs` source — what does the handler ACTUALLY emit when it processes DA1?)
  - Fix the wrong one. If both are wrong, file via `/add-bug` against ori_term.
- [ ] Document the comparison in this section file as a "Cross-validation result: ✓" or "Cross-validation result: divergence found, fixed by [commit]"

---

## 09.3 Performance regression check

**File(s):** None (run existing perf tests)

The tack tests do not touch ori_term's production render loop — they spawn external tack processes and snapshot the resulting Term state, then drop the session. No render-loop overhead, no alloc in hot path. Verify this assumption.

- [ ] Run alloc regression tests: `timeout 150 cargo test -p oriterm_core --test alloc_regression`. Expected: pass — the tack tests do not introduce new allocations in the hot render path.
- [ ] Run event_loop_helpers tests: `timeout 150 cargo test -p oriterm event_loop_helpers`. Expected: pass — the `compute_control_flow()` invariants are unchanged.
- [ ] If any perf test FAILS, this is a regression introduced by the plan. Investigate immediately:
  - Did Section 01's `PtySession` extraction add allocations that show up via `oriterm_core` test linkage?
  - Did the new `crates/oriterm_test_support` dependency introduce a transitive crate that affected something?
  - Whatever the cause, file `/add-bug` immediately and treat as blocker for closing this section.

- [ ] Run the project's benchmark suite if available: `cargo bench -p oriterm_core` (criterion). Compare to a baseline if one exists.

---

## 09.4 Cross-platform build + skip verification

**File(s):** None (verification only)

CLAUDE.md is explicit: every test must compile and run correctly on macOS, Windows, and Linux. Tools (tack, tic, infocmp) are not available on Windows native, so the test source must compile and the test bodies must runtime-skip cleanly.

- [ ] Cross-compile to `x86_64-pc-windows-gnu`: `cargo build --target x86_64-pc-windows-gnu`. All workspace members compile.
- [ ] Cross-compile tests: `cargo build --target x86_64-pc-windows-gnu --tests` for each crate. All test targets compile.
- [ ] Document the skip matrix in this section file:

  | Tool / Resource | Linux | macOS | Windows |
  |-----------------|-------|-------|---------|
  | `tic` | ✓ (apt install ncurses-bin) | ✓ (preinstalled) | ✗ (use WSL) |
  | `tack` | ✓ (apt install ncurses-bin) | ✓ (brew install ncurses) | ✗ (use WSL) |
  | `infocmp` | ✓ | ✓ | ✗ |
  | `vttest` | ✓ (apt install vttest) | ✓ (brew install vttest) | ✗ |
  | wgpu adapter | ✓ (in WSL with WSLg) | ✓ (Metal) | ✓ (DirectX) |

- [ ] On a Windows native machine (or via Windows CI runner if available), confirm the test suite runs and all tack/tic/infocmp tests skip with their `eprintln!` messages — none panic. If the implementer doesn't have Windows access, this verification falls to CI; document the gap.

---

## 09.5 Plan archival and frontmatter gate

**File(s):** All section files (frontmatter), `00-overview.md`, `index.md`, plan directory move

The plan completion frontmatter gate from `/continue-roadmap` workflow. Run all the checks before moving the plan to `completed/`.

- [ ] **00-overview.md frontmatter gate:**
  - `status: complete`
  - All Mission Success Criteria checkboxes ticked
  - Quick Reference table: every section row shows "Complete"
  - No stale "Not Started" or "In Progress" anywhere in the overview

- [ ] **All section file frontmatter gate:**
  - Every section's `status` field set to `complete`
  - Every subsection in every section's `sections` array shows `status: complete`
  - All `third_party_review` blocks resolved (`status: resolved` or `none`)
  - All `reviewed` flags appropriate (Section 01 was `true` from the start; later sections become `true` once their content was validated against actual implementation)

- [ ] **index.md frontmatter gate:**
  - If the plan declares `reroute: true` (it doesn't currently — tack-conformance is a side plan, not a roadmap reroute), set `status: resolved`
  - Section 01-09 status entries all marked Complete

- [ ] **Bug tracker integration:**
  - List any `BUG-*` filings made during Sections 01-08 (e.g., from `/add-bug` invocations triggered by TPR findings or test flakes)
  - For each, confirm: either the bug is now fixed (referenced in a fix-section file under `plans/bug-tracker/`), or the bug is appropriately deferred to a follow-up plan with rationale

- [ ] **Plan move:** `git mv plans/tack-conformance plans/completed/tack-conformance`. Confirm with `ls plans/completed/tack-conformance/` — all 9 section files + 00-overview.md + index.md present.

- [ ] **Final commit:** use `/commit-push` to commit the plan archival with a message like `chore(plans): archive completed tack-conformance plan`. Per the workflow, NEVER `git commit` directly.

- [ ] **TPR checkpoint** — `/tpr-review` covering 09.1–09.5 (the verification section as a whole). Catches: incomplete test matrix, undocumented divergences, missed performance regressions, plan-archival inconsistencies (frontmatter not updated, etc.).

---

## 09.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 09.N Completion Checklist

- [ ] Test matrix populated for all suites (vttest text, vttest GPU, tack text, tack GPU, keyboard terminfo_xcheck, terminfo unit tests)
- [ ] All test suites pass on Linux
- [ ] Cross-validation between tack DA/DSR scenarios and vttest menu6 — no divergence (or documented divergence + fix)
- [ ] Performance regression tests (alloc_regression, event_loop_helpers) all green
- [ ] Bounded-poll invariant unit tests for Section 04 primitives all pass: `pty_session_wait_for_with_context_uses_custom_message`, `pty_session_quit_tack_returns_status_when_child_exits`, `pty_session_quit_tack_panics_on_max_iterations`
- [ ] Per-scenario `tic` compile cost measured (`/usr/bin/time -v` against pre-tack baseline). If wall-clock regression exceeds 10s, the `OnceLock<TerminfoEnv>` cache (Mi2 lever from Section 04 `runner.rs`) was pulled and the regression is now under 10s — NOT deferred to a follow-up plan
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green — zero new warnings
- [ ] `timeout 150 ./test-all.sh` green
- [ ] Cross-compile for `x86_64-pc-windows-gnu` succeeds (all workspace members + all test targets)
- [ ] Cross-platform skip matrix documented in this file
- [ ] All `00-overview.md` Mission Success Criteria checkboxes ticked
- [ ] `00-overview.md` frontmatter `status: complete`
- [ ] All section files frontmatter `status: complete`, all subsections `complete`
- [ ] All `third_party_review` blocks resolved
- [ ] Any `/add-bug` filings made during Sections 01-08 are tracked (fixed or appropriately deferred)
- [ ] Plan archival: `git mv plans/tack-conformance plans/completed/tack-conformance`
- [ ] Final commit via `/commit-push`
- [ ] All TPR checkpoint findings resolved (see `09.R`)
- [ ] `/tpr-review` final pass clean
- [ ] `/impl-hygiene-review last commit` final pass clean (after TPR)

**Exit Criteria:** Every test added by Sections 01-08 passes deterministically on Linux. The cross-platform skip matrix is documented and matches reality (compile everywhere, runtime skip on Windows native). Performance invariants from CLAUDE.md are unchanged. The cross-validation between tack DA/DSR scenarios and vttest menu6 produces a clean diff (or documented + fixed divergence). The plan is archived to `plans/completed/tack-conformance/` with all frontmatter consistent. The conformance suite is complete and the project has machine-verified terminfo capability validation alongside its existing VT protocol validation.
