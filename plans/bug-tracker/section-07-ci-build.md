---
section: "07"
title: "CI & Build Bugs"
status: in-progress
reviewed: false
goal: "Track and fix bugs in CI workflows, release automation, and build scripts"
depends_on: []
third_party_review:
  status: resolved
  updated: 2026-03-30
sections:
  - id: "07.1"
    title: "Active Bugs"
    status: in-progress
  - id: "07.R"
    title: "Third Party Review Findings"
    status: complete
---

# Section 07: CI & Build Bugs

**Status:** Not Started
**Goal:** Track and fix bugs in CI workflows, release automation, and build scripts.

**Note:** This section is never marked complete. New bugs are appended as discovered.

---

## 07.1 Active Bugs

- [ ] `[BUG-07-021][low]` **Graceful Skip Protocol drift across teseq + tack test suites — 18 sites use `eprintln!("<tool> not installed, skipping")` instead of the protocol's required `SKIP:` prefix** — found during /tpr-review round 1 of BUG-07-007 fix (2026-04-25).
  Repro: `grep -rln 'not installed, skipping' oriterm_core/tests/teseq/ oriterm_core/tests/tack/ crates/oriterm_test_support/src/tack_framework/` returns 18 files. `.claude/rules/tests.md` §Graceful Skip Protocol literally says: "log a `SKIP: <reason>` message and `return` early ... use `eprintln!(\"SKIP: tack binary unavailable\")`" — the SKIP: prefix is the contract, not a suggestion. Visible-but-non-conforming messages like "tack not installed, skipping" pass the visibility test but fail the prefix convention, which makes grep-for-skips noisy and breaks the convention readers rely on for skip-vs-fail diagnosis.
  Subsystem: `oriterm_core/tests/teseq/*.rs` + `oriterm_core/tests/tack/**/*.rs` + `crates/oriterm_test_support/src/tack_framework/runner/tests.rs`
  Found: 2026-04-25 | Source: /tpr-review round 1 of BUG-07-007 + BUG-07-020 close-out (codex agreement-finding raised on the in-scope vttest sites; same drift exists across teseq/tack but those files were OUT OF SCOPE for BUG-07-007 so are filed separately here).
  Severity: low — purely a hygiene/convention drift; no behavioral or correctness consequences. Tests still skip visibly; just not in the prescribed format.
  Fix: mechanical sed across all 18 files: `eprintln!("<tool> not installed, skipping");` → `eprintln!("SKIP: <tool> not installed");`. Touch only the literal `eprintln!` argument; leave skip semantics unchanged. Ungated by-construction (the wrapper helpers — `tack_available`, `vttest_available` — still gate the actual skip decision; this fix only normalizes the printed message format).
  Note: the BUG-07-007 in-scope sites (`crates/oriterm_test_support/src/vttest_walker/tests.rs` + `oriterm_core/tests/vttest/menu*.rs`) were fixed inline during /tpr-review round 1; this entry tracks the broader teseq/tack/tack_framework drift that was outside the BUG-07-007 scope.

- [x] `[BUG-07-020][medium]` **`vttest_available()` probe `vttest --help` exits 1 — entire vttest text-side suite (8 menu test files × 3 sizes = 24 tests) silently skips on every host** — found during /fix-bug BUG-07-007 Phase 3 (2026-04-25).
  Resolved: fixed on 2026-04-25 in commit `e265e5af`. Switched `vttest_available()` from `tool_available("vttest", "--help")` to `tool_available("vttest", "-V")` — verified `vttest -V` exits 0 and prints `VT100 test program, version 2.7 (20230201)` while `vttest --help` exits 1. Added `vttest_available_pinned_to_capital_v_probe_via_direct_spawn` regression test mirroring the existing tack pin so any future revert is caught by an independent ground-truth spawn (not via `tool_available`, decoupled from any future `tool_available` change). Surfaced during /fix-bug BUG-07-007 Phase 3 when the new walker's smoke tests reported "vttest not installed, skipping" on a host with `/usr/bin/vttest` present — fix-interference reorder per CLAUDE.md "Fix interference = reorder, don't skip" landed BUG-07-020 first as a separate commit before the BUG-07-007 walker work.
  Repro: `vttest --help; echo $?` prints usage banner and exits `1`. `crates/oriterm_test_support/src/session/tools/mod.rs:42-44` defines `vttest_available()` as `tool_available("vttest", "--help")`, and `tool_available` (lines 31-38) requires `status.success()`. Net effect: every `oriterm_core/tests/vttest/menu*.rs` test prints `"vttest not installed, skipping"` and returns OK on hosts where vttest IS installed at `/usr/bin/vttest`. Verified: `cargo test -p oriterm_test_support --lib vttest_walker -- --nocapture` shows all 4 new BUG-07-007 smoke tests skipping with the message even though `which vttest` returns `/usr/bin/vttest`.
  Subsystem: `crates/oriterm_test_support/src/session/tools/mod.rs` — exact same antipattern family as the prior tack `-V`/`-h` fix already documented at lines 65-73 of the same file (tack v1.08 also exits 1 from `-V`; the fix was switching the probe to `-h`).
  Found: 2026-04-25 | Source: /fix-bug BUG-07-007 Phase 3 (smoke tests for the new walker silently skipped, surfacing the regression).

- [x] `[BUG-07-019][medium]` **`spec-coverage-report --check` UNCATALOGED BACKLOG is 100% false positives — OSC tuple signatures from the runtime observer and the catalog canonicalizer disagree on which slot carries the OSC numeric id** — found during CI nightly fix sweep (2026-04-21).
  Resolved: fixed on 2026-04-25 in commits `49ccb2e0` (SSOT alignment + scope expansions for `dispatch_extract` paths and `classify_from_map` rewrite), `133b87c9` (matrix counter pinned to producer cells, not selectors — TPR round 0), `d12021e3` (per-producer params shape pinned + test-all.sh stale comment cleared — TPR round 1), and `<round-2 commit pending>` (runtime producer made observable via `pub fn perform_action_to_tuple` so the matrix asserts the producer's actual `Tuple`, not a reconstructed proxy — TPR round 2). Aligned all four producers (catalog `parse_osc`, dispatch `extract_osc_arms`, capture `osc_dispatch`, runtime `perform_action_to_tuple`) on the SSOT shape `(Osc, [], <payload>, <selector>)` with the dispatch selector in `final_byte`. Eliminated all OSC false positives from `--check` UNCATALOGED BACKLOG; residual 20 entries are non-OSC categories (charset designation Da/Esc, DCS, CSI', C0) tracked separately.
  Workaround note: `test-all.sh` still runs `spec-coverage-report --check` BEFORE `cargo test` because the residual non-OSC categories would still false-fire. Moving the gate post-test is blocked on those separate fixes.

- [ ] `[BUG-07-017][low]` **`.claude/skills/impl-hygiene-review/plan-annotations.py` crashes with `NameError: name 'AIMS_SECTION_RE' is not defined`** — found by §12.2 impl-hygiene-review Phase 0.
  Repro: invoke the impl-hygiene-review skill against any scope; Phase 0 runs `plan-annotations.py` and it crashes before producing output. The other three Phase-0 tools (`hygiene-lint.py`, `enum-drift.py`, `fn-rename.py`) still run successfully — this is an isolated bug in the plan-annotations linter.
  Subsystem: `.claude/skills/impl-hygiene-review/plan-annotations.py` (around line 631 — reference to undefined regex constant).
  Analysis: Pre-existing tool bug. The constant `AIMS_SECTION_RE` was presumably renamed or removed during prior tooling work without updating a reference. Fix: find the actual regex the code intends (likely `SECTION_RE` or similar) and rename the reference, or restore the deleted constant.
  Found: 2026-04-20 | Source: impl-hygiene-review Phase 0.

- [ ] `[BUG-07-018][low]` **`.claude/skills/impl-hygiene-review/hygiene-lint.py` nesting-depth rule overfires on test files despite `impl-hygiene.md §Guard Clauses` scoping to non-test code** — found by §12.2 impl-hygiene-review Phase 3 / Phase 4 cross-check.
  Repro: test files with deep nesting (e.g., `dcs_q_introducer_p1_p2_p3_cartesian_product` in `oriterm_core/tests/spec_chain/sixel/state_machine.rs`, a three-level cartesian-product loop that is test-appropriate) trigger a BLOAT/minor finding; codex + gemini both verified the rule's `non-test code` scope and confirmed the linter's overfire.
  Subsystem: `.claude/skills/impl-hygiene-review/hygiene-lint.py` (nesting-depth check).
  Analysis: Linter walks every `.rs` file without a test-file filter. Fix: skip paths matching `tests.rs` / `tests/**/*.rs` or any module reached under `#[cfg(test)]`. Test files still get other hygiene checks — just not the production-code nesting-depth cap.
  Found: 2026-04-20 | Source: impl-hygiene-review Phase 3/4 cross-check.

- [x] `[BUG-07-016][medium]` **`spec-coverage-report --check` fails with 80+ pre-existing FALSE VERIFIED rows and 6 UNCATALOGED CITATIONS (trailing-period artifacts)** — found by §09A.N gate run.
  Resolved: OBE on 2026-04-25. The two original failure modes — gate 1 (FALSE VERIFIED rows: catalog says verified but no test cites) and gate 2 (UNCATALOGED CITATIONS: trailing-period artifacts like `OSC-2.`, `OSC-112.`, `OSC-4-QUERY. Apex: ...`, `SIXEL-DCS-Q-MINIMAL`, `OSC-8 (path).`) — are now both clean. Verified by `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check 2>&1 | grep -cE 'FALSE VERIFIED|UNCATALOGED CITATIONS'` → `0`. The current `--check` exit-1 firing is from gate 4 (UNCATALOGED BACKLOG: 49 runtime tuples observed but not in catalog), which is a different failure mode tracked under `BUG-07-019` (filed 2026-04-21). The remediation paths originally proposed in this entry (scanner trim-trailing-punctuation in `scan.rs`; per-row catalog-citation hookup) appear to have been resolved by spec-conformance plan work between 2026-04-20 (filing) and 2026-04-25 — likely as part of catalog citation sweeps tracked in spec-conformance Section 09A or its successor sections. Closure verified by checking gate 1+2 output is empty, NOT by running git archaeology (banned per CLAUDE.md). The remaining BACKLOG failure is owned by BUG-07-019.
  Repro: `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check` exits 1 with `FALSE VERIFIED (catalog says verified but no test cites): DEC-DECSCNM, DEC-X10-MOUSE, …` listing 80+ rows across dec-private-modes / ecma-48 / iterm2 / osc / shell-integration / xterm-ctlseqs stacks, plus UNCATALOGED CITATIONS `OSC-2.`, `OSC-112.`, `OSC-4-QUERY. Apex: state-snapshot / effect-pty-write.`, `OSC-50.`, `SIXEL-DCS-Q-MINIMAL`, `OSC-8 (\`plans/spec-conformance/catalog/osc.md\`).`.
  Subsystem: `crates/oriterm_test_support/src/spec_chain/coverage/scan.rs` (citation scanner) + `plans/spec-conformance/catalog/*.md` (stale `verified` status vs. scanner-recognizable citations).
  Analysis: Pre-existing cross-section systemic gap from before §09A. Two distinct root causes:
  (1) **Scanner strictness vs. natural comment style**: `scan.rs` splits `//! Catalog rows: OSC-0, OSC-1, OSC-2.` on `,` and keeps the trailing `.` on the last piece, registering `OSC-2.` as a distinct row ID. Similar for `SIXEL-DCS-Q-MINIMAL` (no catalog row exists — typo or renamed row), `OSC-4-QUERY. Apex: …` (whole sentence kept as row ID), `OSC-8 (\`path\`).` (parenthetical path kept as part of ID). Fix: trim trailing punctuation + stop at first whitespace in `scan.rs`'s piece extraction, OR normalize test comments.
  (2) **Citations missing entirely for `verified`-status rows**: 80+ rows are marked `verified` in the catalog but no `.rs` file cites their ID in any recognized comment form. Examples: DEC-DECSCNM (dec-private-modes), OSC-7 (osc), SHINT-OSC-133-PROMPT (shell-integration). The test logic exists; the catalog-row-ID comment hookup is missing. Fix: walk each false-verified row, locate the test that actually covers it, and add a `// Catalog row: <ID>` or `//! Catalog rows: <ID>` comment in the test file's header.
  Found: 2026-04-20 | Source: §09A.N completion gate run.
  Note: `§09A.R` `third_party_review.status: resolved` marks this as "residual drift risk accepted" per `iter_cap_reached`; this bug promotes that residual risk to a concrete tracked artifact per CLAUDE.md §Bug Discipline. Scope: cross-section (touches dec-private-modes / ecma-48 / iterm2 / osc / shell-integration / xterm-ctlseqs), not owned by any single in-progress spec-conformance section. Ownership: spec-conformance Section 23 (CI gating) or a dedicated citation-hygiene sweep section.

- [x] `[BUG-07-013][medium]` **Nightly Windows CI failed — `new_headless_with_software_preference_uses_force_fallback` rejects "Microsoft Basic Render Driver" (a legitimate software rasterizer) because the assertion's KNOWN-name list is incomplete** — found by continue-roadmap.
  Found: 2026-04-14 | Source: continue-roadmap (nightly CI failure triaged during spec-conformance Section 07 close-out)
  Fixed: 2026-04-14 — Refactored `oriterm/src/gpu/state/tests.rs:553-579` to trust `adapter_info().device_type == wgpu::DeviceType::Cpu` as the primary authoritative signal (wgpu contract: CPU adapters ARE software rasterizers by definition), keeping the KNOWN-name string list as a defensive fallback for older wgpu versions that misreport device_type. Added `"microsoft basic render"` to the fallback list for completeness. Applied the same fix to the sibling `oriterm/src/gpu/visual_regression/deterministic_lane_tests.rs::deterministic_lane_selects_software_adapter` test which had the same brittle KNOWN-name list. Windows Dx12 reports `device_type=Cpu` for Microsoft Basic Render Driver, so the new primary path short-circuits before the string check. Verified locally via `cargo test -p oriterm --lib new_headless_with_software_preference` (filter-match, not exact — the test ran and passed). CI confirmed fix on nightly run 24374936218 (2026-04-14 00:54 UTC, 5m44s, all platforms success).
  Impact: CI was failing on Windows when the llvmpipe/lavapipe runners were unavailable and fell back to Microsoft Basic Render Driver. The test was structurally too strict — it codified a hardcoded list of driver names instead of trusting the wgpu-reported device type.

- [x] `[BUG-07-014][high]` **`merge-findings.py` silently swallows corrupt active-reviewer envelopes as zero findings** — found by tpr-review.
  Repro: With a valid codex envelope and a corrupt gemini JSON file (active reviewer, not circuit-breaker-skipped), `merge-findings.py` exits 0 and emits `reviewer_mode: "single"`, `tripped_reviewer: "gemini"`, and zero findings instead of failing.
  Subsystem: `.claude/skills/dual-tpr/scripts/merge-findings.py`
  Found: 2026-04-16 | Source: tpr-review | Reviewer: codex
  Resolved: Fixed on 2026-04-16. Removed the `except (json.JSONDecodeError, OSError)` clause from `load_envelope()` that was swallowing errors from files that exist but are corrupt. Missing files (legitimate circuit-breaker skips) still return the empty-findings sentinel. Verified: corrupt JSON now raises with exit code 1; missing files still return `reviewer_mode: "single"`.

- [x] `[BUG-07-015][medium]` **`merge-findings.py` undercounts actionable findings when mixed-severity agreements exist** — found by tpr-review.
  Repro: codex emits `high` and gemini emits `informational` for the same `(location, title)` — merger produces `max_severity: "high"` but `summary.actionable: 0` because the informational half subtracts from the agreement count.
  Subsystem: `.claude/skills/dual-tpr/scripts/merge-findings.py`
  Found: 2026-04-16 | Source: tpr-review | Reviewer: codex
  Resolved: Fixed on 2026-04-16. Replaced per-entry informational counting with a pair-level max-severity check: a `(location, title)` pair is only informational if the MAX severity across all entries for that pair is `informational`. Mixed-severity agreements (e.g. high + informational) are now correctly counted as actionable. Verified with regression tests for both mixed-severity and pure-informational cases.

- [ ] `[BUG-07-011][low]` **`plans/spec-conformance/index.md` at 535 lines — approaching plan-audit BLOAT_RISK heuristic (500-line threshold)** — found by tpr-review during /review-plan Phase 4 on `plans/spec-conformance/section-02-tack-absorption.md`.
  Repro: `wc -l plans/spec-conformance/index.md` → `535`. `python3 .claude/skills/plan-audit/plan-audit.py plans/spec-conformance --verify --json | python3 -c "import json,sys;d=json.load(sys.stdin); [print(f) for f in d['findings'] if f.get('check')=='BLOAT_RISK' and 'index.md' in f.get('location','')]"` reports `major BLOAT_RISK plans/spec-conformance/index.md — 535 lines, at or near 500-line limit`.
  Subsystem: `plans/spec-conformance/index.md` (plan-hygiene — no code crate). Filed under section 07 per `/add-bug` subsystem mapping (plan-hygiene bugs that don't map to a code crate go in CI & Build alongside BUG-07-005 / BUG-07-006 / BUG-07-010).
  Found: 2026-04-11 | Source: tpr-review | Reviewers: codex + gemini (TPR-02-002-codex + TPR-02-002-gemini convergence during `/review-plan` Phase 4 on `plans/spec-conformance/section-02-tack-absorption.md`)
  Context: `plans/spec-conformance/index.md` is the searchable keyword-cluster index for a 26-section plan. Each section carries a ~15-line keyword cluster plus a row in the Quick Reference table. The 500-line hard limit from `.claude/rules/code-hygiene.md` §File Organization is scoped to "source files excluding `tests.rs`", but `plan-audit.py` applies the same heuristic as a BLOAT_RISK advisory to plan-index markdown files.
  Why NOT fixed in Section 02 scope: `plans/spec-conformance/section-02-tack-absorption.md` is the Phase 0b mechanical absorption step — its atomic commit is scoped to the 8-file set needed for the absorption. Adding index-trimming work would double-scope Section 02 against the same file Section 01's Catalog Bootstrap will eventually edit, creating a merge conflict. Filing here keeps Section 02 scoped and makes the BLOAT a separately tracked artifact.
  Fix approach:
  - Option A: split the 26 keyword clusters into a separate `plans/spec-conformance/catalog-keyword-index.md` that `index.md` links to, leaving `index.md` as the slim Quick Reference + cross-ref table only
  - Option B: trim each keyword cluster from ~15 lines to ~8 lines by removing redundant synonyms (current clusters list both formal names AND developer aliases; trim to one canonical name + 3–4 most-likely search keywords)
  - Option C: extract the Quick Reference table to a separate file and have `index.md` be ONLY the keyword clusters
  - Consult `plans/completed/` for prior plan-index conventions to pick the approach with least divergence
  Severity: low — no correctness impact, no runtime impact, no test impact. The index is usable as-is today. This is a plan-audit advisory that becomes actionable only if the file keeps growing.

- [ ] `[BUG-07-010][medium]` **`./clippy-all.sh` does not lint test targets — 151 pre-existing clippy errors across 6 `oriterm_core` test targets (lib test + alloc_regression + rss_regression + tack + teseq + vttest)** — found by continue-roadmap (tack-conformance section 05.0).
  Superseded by: plans/clippy-gate-hardening/ on 2026-04-25. Cluster co-escalation with BUG-07-005. Cleanup lands in clippy-gate-hardening Section 03; closure in Section 10.
  Repro: `cargo clippy -p oriterm_core --tests --target x86_64-unknown-linux-gnu -- -D warnings` produces **151 errors** that block compilation across 6 distinct test targets. `./clippy-all.sh` runs `cargo clippy --workspace -- -D warnings` which only checks lib + bin targets, so all 151 violations have been silently passing CI.
  Subsystem: `clippy-all.sh` + `oriterm_core/src/term/tests.rs` + `oriterm_core/tests/{alloc_regression,rss_regression,tack/main,teseq,vttest}/`
  Found: 2026-04-08 | Source: continue-roadmap
  Affected test targets (each fails to compile under `-D warnings`):
  - `oriterm_core` (lib test) — `oriterm_core/src/term/tests.rs` and other sibling unit tests. Includes `float_cmp` on `assert_eq!(viewport_y, 48.0)` at line 1926, `len_zero` (`.len() > 0`) at line 2126, plus dozens more (`unnested_or_patterns`, `derive_partial_eq_without_eq`, `manual_let_else`, `match_same_arms`, `unnecessary_raw_string_hashes`, `derivable_impls`, `doc_markdown`, etc.).
  - `oriterm_core` (test "alloc_regression") — 1 error (likely `doc_markdown`).
  - `oriterm_core` (test "rss_regression") — 1 error (likely `doc_markdown`).
  - `oriterm_core` (test "tack") — 3 errors, all `doc_markdown` in `oriterm_core/tests/tack/main.rs:4:5`, `35:55`, `36:5`. NOT introduced by tack-conformance 05.0; my new files in `oriterm_core/tests/tack/test_menu/` are clean.
  - `oriterm_core` (test "teseq") — error count not enumerated, in `oriterm_core/tests/teseq/`.
  - `oriterm_core` (test "vttest") — 11 errors. **Overlaps with `[BUG-07-005]`** — that bug already documents the menu*.rs violations.
  Same root cause family as `[BUG-07-005]` and `[BUG-07-006]`: clippy-all.sh's scope is too narrow. BUG-07-005 covers the integration tests in `oriterm_core/tests/vttest/`; this entry expands the surface area to ALL oriterm_core test targets including the lib test (sibling `tests.rs` files like `oriterm_core/src/term/tests.rs`), `alloc_regression`, `rss_regression`, `tack`, and `teseq`. The total violation count is therefore **151 errors across 6 test targets**, of which 11 are the BUG-07-005 vttest subset and 140 are NEW surface area first quantified here.
  Severity: medium — same family as 005/006, larger surface area, no production impact (test code only) but blocks any future `--all-targets` clippy hardening of `clippy-all.sh`. Filing now (per CLAUDE.md `/add-bug` discipline) so the cleanup is tracked as a single coherent fix.
  Fix: (1) update each violation site across the 6 affected test targets — most are mechanical (`!is_empty()` over `len() > 0`, `Eq` derive alongside `PartialEq`, `let-else` over `match`, backticks in doc comments). The `float_cmp` violation needs `assert!((a - b).abs() < EPSILON)`. (2) Update `./clippy-all.sh` to add `cargo clippy --workspace --all-targets -- -D warnings` (or a separate `cargo clippy --workspace --tests` step) so test-target lints are gated by CI going forward. (3) Coordinate with BUG-07-005's vttest fix so the 11 overlapping errors are not double-counted. None of the 151 errors are caused by tack-conformance section 05.0 — verified by `cargo clippy -p oriterm_core --test tack --target x86_64-unknown-linux-gnu -- -D warnings` showing all 3 tack-test errors live in `main.rs:4`, `35`, `36` (untouched by 05.0).
  Note: Active work in tack-conformance section 05 (M1: 05.0 → 05.0.b → 05.0.c → 05.1) does not modify any of the violation lines. Discovered while running `cargo clippy -p oriterm_core --tests` to verify 05.0's new files are clean — they ARE clean; these errors come from pre-existing test code that has never been linted because `./clippy-all.sh` excludes test targets.

- [x] `[BUG-07-009][high]` **10 Windows ConPTY runtime test failures in `oriterm_test_support`, first surfaces as `STATUS_DLL_INIT_FAILED` (`0xC0000142`) on cmd.exe spawn after several silent-long-lived ping spawns** — found by nightly CI 2026-04-08 after fixing the Windows compile errors that previously masked these tests.
  **Fixed 2026-04-08.** Three-part fix.
  **(1) HPCON premature close.** `PtySession::spawn` dropped `pair.master` at function exit, calling `ClosePseudoConsole` while the child was still alive. Per Microsoft's documented contract, `ClosePseudoConsole` must be called only after the child has exited; doing it earlier corrupts console-subsystem DLL state and causes subsequent `cmd.exe` spawns to fail with `STATUS_DLL_INIT_FAILED` or hang inside `WaitForSingleObject`. Fix: store `pair.master` as `_master` field on `PtySession` (declared after `child` so Rust's declaration-order field-drop runs `child` first, then `_master` — the Microsoft-sanctioned ordering). Mirror of the in-tree production pattern at `oriterm_mux/src/pty/spawn.rs:261` (`PtyControl(pair.master)`) and the wezterm reference at `mux/src/domain.rs:619-652`.
  **(2) Grandchild orphan.** Helper test commands wrapped real subprocesses in `cmd.exe /C "ping … > NUL"`. The wrapper made cmd.exe the immediate ConPTY child and ping a grandchild; `PtySession::drop` only terminates the immediate child, leaving ping orphaned but still attached as a console client. The subsequent `ClosePseudoConsole` blocked waiting for the grandchild to release the HPCON. Fix: replace all wrapped-subprocess helpers with `cmd.exe /C "echo X & pause > NUL"` patterns where `pause` is a `cmd.exe` builtin (in-process, no grandchild). Affects `spawn_silent_long_lived` and the navigator pre-existing-anchor / alternate-anchor tests.
  **(3) Parallel ConPTY contention.** Empirical testing showed that running >4 simultaneous active `PtySession`s on Windows ballooned per-test wall-clock from <1 s to 50+ s — Windows ConPTY contends across the entire pseudoconsole lifetime, not just spawn. Fix: introduce `CONPTY_LIFETIME_LOCK` (a `static Mutex<()>` held in an `_conpty_guard` field for the entire `PtySession` lifetime) to serialize ConPTY-using tests on Windows. Non-PTY tests (parser, terminfo, helpers) still run in parallel. Linux/macOS PTYs are unaffected — the lock is `cfg(windows)`-only.
  **(4) Path-format mismatch.** `child_process_with_apply_env_reads_pinned_terminfo` asserted on the full Windows tempdir path, but MSYS infocmp on Windows reports paths in normalized form (drive letter stripped, `\` → `/`). Fix: assert on the unique tempdir basename instead. Also gated on a new `infocmp_respects_terminfo_env()` runtime probe so hosts whose infocmp lacks env-var precedence support skip cleanly.
  **Verification.** All 53 `oriterm_test_support` tests now pass in parallel in 9.81 s on a Windows native host (previously: 10 failures + multiple 60+ s hangs). Full `cargo test --workspace` is green (~7000 tests across all crates). New regression test: `pty_session_repeated_spawn_drop_cycle_succeeds_on_subsequent_cmd_exe_spawn` (semantic pin for the spawn-drop-spawn cycle). See `fix-BUG-07-009.md` for the full investigation.
  Repro:
  1. Cross-compile + run the test suite on a Windows runner: `cargo test -p oriterm_test_support --target x86_64-pc-windows-msvc` (or trigger a nightly CI run).
  2. Observe that 10 tests fail in this exact set:
     - `session::sync::tests::pty_session_wait_for_any_returns_some_zero_when_primary_matches`
     - `session::sync::tests::pty_session_wait_for_any_returns_some_alt_when_alternate_matches`
     - `session::sync::tests::pty_session_wait_for_any_prefers_primary_over_alternates_on_tie`
     - `session::teardown::tests::pty_session_wait_for_child_exit_returns_on_clean_exit`
     - `session::teardown::tests::pty_session_quit_tack_returns_status_when_child_exits`
     - `session::teardown::tests::pty_session_quit_tack_exits_early_when_child_dies_after_first_q`
     - `tack_framework::navigator::tests::navigator_panics_when_anchor_already_present_in_pre_grid`
     - `tack_framework::navigator::tests::navigator_matches_alternate_when_primary_never_appears`
     - `tack_framework::runner::tests::live_session_finish_asserts_clean_exit_via_quit_tack`
     - `terminfo::tests::child_process_with_apply_env_reads_pinned_terminfo`
  3. The first chronological failure is `pty_session_wait_for_child_exit_returns_on_clean_exit`, which spawns `cmd.exe /C exit 0` and observes `ExitStatus { code: 3221225794, signal: None }`. `0xC0000142` = `STATUS_DLL_INIT_FAILED` — cmd.exe is failing during DLL initialization, not during `exit 0`.
  4. The earlier-running tests `pty_session_drains_simple_output` (cmd.exe echo) and the wait-for-with-context tests (cmd.exe-free, silent ping child) PASS. The pattern is: every test that spawns a fresh `cmd.exe` AFTER the four `spawn_silent_long_lived` tests (`ping 127.0.0.1 -n 11 > NUL`) gets `STATUS_DLL_INIT_FAILED`. Each silent-ping test ends with `PtySession::drop`, which calls `child.kill() + child.wait()` — but the ConPTY HPCON / handle pool may still be retained until the master `Arc<Mutex<Inner>>` count drops to zero (`crates/portable-pty/src/win/conpty.rs`).
  5. The `child_process_with_apply_env_reads_pinned_terminfo` failure is a separate root cause: on the Windows CI runner, `infocmp` runs but cannot find `ori_term` via `$TERMINFO`/`$TERMINFO_DIRS` (likely an MSYS infocmp that doesn't honor ncurses env-var precedence).
  Subsystem: `crates/oriterm_test_support/src/session/{sync,teardown}/`, `crates/oriterm_test_support/src/tack_framework/{navigator,runner}/`, `crates/oriterm_test_support/src/terminfo/tests.rs`, possibly `crates/portable-pty/src/win/conpty.rs` (HPCON drop ordering)
  Found: 2026-04-08 | Source: nightly CI failure analysis after fixing the Windows compile errors that masked these tests
  Severity: **high** — these tests have NEVER run successfully on Windows (they were latent under the compile error fixed by `b6e99416`). Per CLAUDE.md cross-platform rule, every test must work on all three platforms. The tests landed across tack-conformance Sections 01-04 (commits between `f338bfc1` and `efec3818`) and the Windows path was never exercised.
  Hypothesis (needs Windows-side verification): the four `spawn_silent_long_lived` tests leak HPCON resources because the master `ConPtyMasterPty` Arc may be retained beyond `PtySession::Drop` via portable-pty internals, or the ConPTY pseudo-console is released before the spawned child has fully detached. After 4 leaked handles the Windows console subsystem can't init a new cmd.exe.
  Fix:
  1. Investigate on a real Windows host (or in a Windows VM with ConPTY) — reproduce and instrument the HPCON handle count after each `PtySession::Drop`.
  2. Audit `PtySession::spawn` (`crates/oriterm_test_support/src/session/mod.rs:95-143`) for master/slave drop ordering. The current code does `drop(pair.slave)` before `try_clone_reader`/`take_writer`, then drops `pair.master` implicitly at the end of `spawn()`. Verify that this ordering is sound under ConPTY — the reader's `try_clone()` of `readable` may not be enough to keep the underlying file descriptor alive after the master is dropped.
  3. If portable-pty's ConPTY backend has a leak, file an upstream fix (vendored at `crates/portable-pty/`).
  4. For `child_process_with_apply_env_reads_pinned_terminfo`: gate the test on a Windows-specific check that `infocmp` is the ncurses-style binary (or skip cleanly when an MSYS variant is detected). Alternatively, replace it with a Windows-equivalent integration test that uses a tool that DOES respect `$TERMINFO`.
  Note: this bug was uncovered by `b6e99416`'s Windows compile fix (`fix(test-support): nightly CI macOS hashed-db panic + Windows -D warnings`). That commit fixed the legitimate Windows -D warnings errors so the test crate would compile cross-platform; the runtime failures it surfaced are pre-existing latent bugs in the recent (Apr 7-8) tack-conformance test additions, not regressions introduced by the compile fix. Filing — not deferral — per CLAUDE.md `/add-bug` discipline.

- [ ] `[BUG-07-012][medium]` **`oriterm_test_support` has 14 clippy `--all-targets` warnings in test files — same gate gap as BUG-07-005/010**
  Superseded by: plans/clippy-gate-hardening/ on 2026-04-25. Cluster co-escalation with BUG-07-005. Cleanup lands in clippy-gate-hardening Section 04; closure in Section 10.
  Repro: `cargo clippy -p oriterm_test_support --all-targets -- -D warnings` produces 14 errors (all in test files, lib target is clean). `./clippy-all.sh` only checks lib+bin targets so these never fire in CI.
  Subsystem: `crates/oriterm_test_support/src/{session,tack_framework}/` test files
  Found: 2026-04-11 | Source: continue-roadmap
  Locations:
  - `session/sync/tests.rs:406` — `items_after_statements`
  - `session/version_gate/tests.rs:152` — `items_after_statements` or `late_init`
  - `tack_framework/cap_coverage/tests.rs:{369,384}` — `into_iter_on_single_item` (2x)
  - `tack_framework/cap_coverage/tests.rs:536` — `iter().count()` on slice
  - `tack_framework/runner/tests.rs:181` — `pass_by_value_not_consumed`
  - `tack_framework/runner/tests.rs:{603,625}` — `doc_markdown` (2x)
  - `tack_framework/scenarios/menu_inventory/tests.rs:{8,9}` — `doc_markdown` + `items_after_statements`
  - `tack_framework/scenarios/sgr_modes/tests.rs:{21,78,95}` — `doc_markdown` + `format_push_string` (3x)
  - `tack_framework/scenarios/tools_menu_inventory/tests.rs:136` — `case_sensitive_file_extension_comparisons`
  Same root cause family as BUG-07-005, BUG-07-006, BUG-07-010: `clippy-all.sh`'s scope is too narrow. The gate fix (adding `--all-targets`) should land as ONE fix covering all four bugs' crate surface areas simultaneously.
  Fix: (1) update each violation site — all are mechanical (backticks in docs, `.len()` over `.iter().count()`, `write!` over `format!(..)`, `Path::new().extension()` over `.ends_with(".rs")`, etc.), (2) coordinate with BUG-07-010's clippy-all.sh gate fix so `--all-targets` covers this crate too.
  Severity: medium — no production impact, test code only. Blocks `--all-targets` hardening of `clippy-all.sh`.

- [x] `[BUG-07-004][medium]` **Windows PTY size propagation test removed** — found by tpr-review.
  Resolved: fixed on 2026-04-25 in commits `7a1cc3e9` (test addition) + `0dc03ae7` (import hoist). See `plans/bug-tracker/fix-BUG-07-004.md` for the full design consensus, plan TPR findings, and rationale. Refactored `oriterm_core/tests/vttest/pty_size.rs` to a shared `assert_pty_reports_size(rows, cols, cmd, parse)` helper plus `pty_size_propagation_unix_stty_reports_correct_dimensions` (POSIX, `stty size`) and `pty_size_propagation_windows_mode_con_reports_correct_dimensions` (ConPTY, `cmd /d /c mode con`). Each platform branch runs two cases (33×97 + 50×40 negative pin). Phase 1.75 `/tp-help` consensus refined the approach (`cmd /d /c` over `cmd /c` for AutoRun bypass); Phase 2.5 plan TPR caught and resolved 5 actionable findings (parser locale claim, 50×20 mode-con-wrap, 24×24 matrix drift, stale exit-criteria filter, swallowed `child.wait()` error); Phase 5 code TPR closed clean.
  Found: 2026-04-02 | Source: tpr-review

- [x] `[BUG-07-007][medium]` **vttest screen-walker scaffold duplicated across 13+ functions in two crates** — found by impl-hygiene-review (tack-conformance section 01.N).
  Resolved: fixed on 2026-04-25 in commits `e265e5af` (BUG-07-020 prerequisite — vttest_available probe `--help`→`-V`), `a38fa97b` (extracted `walk_vttest_screens` helper to `crates/oriterm_test_support/src/vttest_walker/`; migrated all 17 call sites across `oriterm_core/tests/vttest/menu*.rs` and `oriterm/src/gpu/visual_regression/vttest/`), `492ef76d` (TPR round-0 fixes: clippy `is_ok_and`, SKIP eprintln, doc, tracker entry), and `3f6c6bb9` (TPR round-1 fixes: module `//!` doc, SKIP: prefix on all 32 in-scope skip sites). Closed 17 hand-rolled loops with one canonical higher-order helper per /tp-help round-1 design consensus (Codex + Gemini agreed). Net diff: ~250 lines deleted across the migration. /tpr-review --max-rounds=2 ran with 6 verified actionable findings (4 round-0 + 2 round-1, all fixed inline) + 2 gemini findings dropped at verification; user-accepted at iter_cap_reached. Out-of-scope skip-protocol drift filed as BUG-07-021. See `plans/bug-tracker/fix-BUG-07-007.md` for full TDD matrix, design consensus, TPR audit trail.
  Repro: read `oriterm/src/gpu/visual_regression/vttest/mod.rs:23-121` (run_menu1_golden + run_menu2_golden), `oriterm/src/gpu/visual_regression/vttest/menus_3_8.rs` (run_menu3/4/6/7/8_golden, 5 functions), and `oriterm_core/tests/vttest/menu1.rs` through `menu8.rs` (run_menuN_X functions, 8+ functions). Each function shares the SAME control-flow skeleton: `headless_env()` (GPU side only) → `PtySession::spawn_vttest(cols, rows)` → `wait_for("Enter choice number", 5000)` → optional snapshot of menu screen → `send(b"<digit>\r")` → loop walking screens with per-screen snapshot/golden assertion until `text.contains("Enter choice number")` → break on `screen > 20`. ~25 lines of identical scaffolding × 13+ instances. The only thing that varies is the per-screen action (insta::assert_snapshot! on the text side, assert_golden on the GPU side) and the menu digit/label.
  Subsystem: `oriterm_test_support` (canonical home for the helper) + `oriterm_core/tests/vttest/` + `oriterm/src/gpu/visual_regression/vttest/`
  Found: 2026-04-07 | Source: impl-hygiene-review (tack-conformance section 01.N)
  Severity: medium — pre-existing duplication that the section 01 deduplication faithfully preserved (zero behavioral change was the section mandate). Per impl-hygiene.md "cross-crate duplication: even 2 instances = extract to a shared crate" rule, this 13+ instance pattern is overdue for extraction.
  Fix: add a higher-order helper to `oriterm_test_support`:
  ```rust
  pub fn walk_vttest_screens(
      session: &mut PtySession,
      max_screens: usize,
      mut on_screen: impl FnMut(&mut PtySession, usize),
  ) {
      let mut screen = 1;
      loop {
          let text = session.grid_text();
          if text.contains("Enter choice number") { break; }
          on_screen(session, screen);
          session.send(b"\r");
          screen += 1;
          if screen > max_screens { break; }
      }
  }
  ```
  Each `run_menuN_*` function then collapses to ~5 lines that pass a closure for the per-screen snapshot/golden call. Eliminates ~250 lines of duplication across 13 functions in two crates.
  Note: discovered during the section 01 final hygiene pass. NOT introduced by section 01 — it's pre-deduplication code that the migration correctly preserved verbatim. Section 01 is closing out clean; this is a follow-up for `/fix-bug` (or rolled into a future section's "test infrastructure cleanup" subsection).

- [ ] `[BUG-07-006][medium]` **`./clippy-all.sh` does not enable feature flags — 9 pre-existing clippy violations in `oriterm_ui/src/testing/`** — found by continue-roadmap.
  Superseded by: plans/clippy-gate-hardening/ on 2026-04-25. Cluster co-escalation with BUG-07-005. Cleanup lands in clippy-gate-hardening Section 05; closure in Section 10.
  Repro: `cargo clippy -p oriterm --features gpu-tests --tests -- -D warnings` produces 9 errors. `./clippy-all.sh` runs `cargo clippy --workspace -- -D warnings` which uses the default feature set. The `oriterm_ui::testing` module is gated behind `#[cfg(feature = "testing")]`, so it's never linted by CI. Same root cause family as `[BUG-07-005]` (clippy-all scope is too narrow), different surface area (feature-gated lib code vs unconditional test target code).
  Subsystem: `clippy-all.sh` + `oriterm_ui/src/testing/`
  Found: 2026-04-07 | Source: continue-roadmap
  Locations:
  - `oriterm_ui/src/testing/scene_snapshot/mod.rs:101:12,28,44` — `float_cmp` (3×)
  - `oriterm_ui/src/testing/scene_snapshot/mod.rs:123:5` — `if_not_else`
  - `oriterm_ui/src/testing/scene_snapshot/mod.rs:176:14` — clippy lint (TBD)
  - `oriterm_ui/src/testing/harness.rs:46:37` — clippy lint (TBD)
  - `oriterm_ui/src/testing/harness_dispatch.rs:56:13` — clippy lint (TBD)
  - `oriterm_ui/src/testing/mock_measurer/mod.rs:28:5` — clippy lint (TBD)
  - `oriterm_ui/src/testing/query.rs:19:25` — clippy lint (TBD)
  Fix: (1) update each violation site, and (2) add `--features testing` to `./clippy-all.sh` (or add `cargo clippy --workspace --all-features` as a sibling step) so feature-gated code is gated by CI going forward. None caused by tack-conformance section 01.4 GPU migration — verified by reading my diffs against violation lines (none of the modified files are oriterm_ui).
  Note: Active work in tack-conformance section 01.4 (GPU vttest migration) does not modify the lines flagged above. Discovered when running `cargo clippy --features gpu-tests --tests` to verify my changes were clean; my changes WERE clean — these errors come from the feature-gated `oriterm_ui::testing` module which my new dev-dep on `oriterm_test_support` had nothing to do with.

- [ ] `[BUG-07-005][medium]` **`./clippy-all.sh` does not lint test targets — 11 pre-existing clippy violations in `oriterm_core/tests/vttest/`** — found by continue-roadmap.
  Escalated to plan: plans/clippy-gate-hardening/ on 2026-04-25. /fix-bug Phase 1.5 escalation: scope grew from 11 vttest violations to ~1480 violations across 6 crates plus the gate-flip work for clippy-all.sh + ci.yml + lefthook.yml. Cluster co-escalated: BUG-07-005, BUG-07-006, BUG-07-010, BUG-07-012 + 3 new entries to be filed in clippy-gate-hardening Section 01.2 (oriterm_mux, oriterm_ipc, oriterm). Closure happens in clippy-gate-hardening Section 10.
  Repro: `cargo clippy -p oriterm_core --test vttest -- -D warnings` produces 11 errors. `./clippy-all.sh` runs `cargo clippy --workspace -- -D warnings` which only checks lib + bin targets, so test-target violations have been silently passing CI.
  Subsystem: `clippy-all.sh` + `oriterm_core/tests/vttest/menu*.rs`
  Found: 2026-04-07 | Source: continue-roadmap
  Locations:
  - `oriterm_core/tests/vttest/menu1.rs:107:14`, `124:14`, `133:14` — `needless_range_loop` (3×)
  - `oriterm_core/tests/vttest/menu2.rs:49:26` — `string_slice`
  - `oriterm_core/tests/vttest/menu4.rs:4:38` — `doc_markdown`
  - `oriterm_core/tests/vttest/menu5.rs:14:5` — doc list item without indentation
  - `oriterm_core/tests/vttest/menu6.rs:11:14`, `11:28` — `doc_markdown` (2×)
  - `oriterm_core/tests/vttest/menu7.rs:3:55` — `doc_markdown`
  - `oriterm_core/tests/vttest/menu8.rs:11:1` — `too_many_lines` (124/100)
  - `oriterm_core/tests/vttest/menu8.rs:12:39` — `redundant_closure_for_method_calls`
  Fix: (1) update each violation site, and (2) add `--all-targets` to `./clippy-all.sh` so test-target lints are gated by CI going forward. None caused by tack-conformance section 01.3 PtySession migration — verified by reading the diffs against violation lines.
  Note: Active work in tack-conformance section 01 touches `oriterm_core/tests/vttest/session.rs` and the menu*.rs imports, but does not modify the lines flagged above.

- [x] `[BUG-07-008][medium]` **`oriterm_test_support` PtySession test uses `#[cfg(unix)]` instead of a runtime gate** — found by /tp-help pre-check (Codex) during /review-plan on tack-conformance section 02.
  **Fixed 2026-04-07.** Resolved by tack-conformance section 02.3. `crates/oriterm_test_support/src/session/tests.rs::pty_session_drains_simple_output` no longer carries `#[cfg(unix)]` — replaced with a portable two-arm test (`/bin/sh -c "printf hello"` on Unix, `cmd.exe /C "echo hello"` on Windows) wrapped in `#[cfg(unix)] / #[cfg(windows)]` blocks INSIDE the `#[test] fn`, restoring Windows ConPTY drain coverage. Verified by `cargo build --target x86_64-pc-windows-gnu -p oriterm_test_support --tests` and the host `cargo test -p oriterm_test_support` (12 tests pass).
  Repro: open `crates/oriterm_test_support/src/session/tests.rs:16` — `pty_session_drains_simple_output` is wrapped in `#[cfg(unix)]` so the test source does not even compile on Windows. The test spawns `/bin/sh -c "printf hello"` and asserts the PTY drain contains `hello`.
  Subsystem: `crates/oriterm_test_support/src/session/tests.rs`
  Found: 2026-04-07 | Source: /tp-help pre-check (Codex)
  Severity: medium — same antipattern family as `[BUG-07-004]` (Windows PTY size test removed by `#[cfg(unix)]`). CLAUDE.md cross-platform rule: "All code must compile and run correctly on all three platforms… Every `#[cfg(target_os = "...")]` block must have counterparts for all supported targets — no platform left behind." Section 02 of `tack-conformance` explicitly bans this exact pattern in its skip-discipline subsection — Section 01 contradicts the very rule Section 02 articulates.
  Fix: **Owned by tack-conformance Section 02.3.** The fix is a portable two-arm test (`/bin/sh -c "printf hello"` on Unix, `cmd.exe /C "echo hello"` on Windows) so Windows gets real ConPTY drain coverage. Implementation steps and full code listing live in `plans/tack-conformance/section-02-terminfo-provisioning.md` under 02.3. When that section lands, check this box, add a "Fixed YYYY-MM-DD" line, and the bug closes automatically. Do NOT run a separate `/fix-bug BUG-07-008` — Section 02's skip-discipline subsection IS the fix.

- [ ] `[BUG-07-022][medium]` **SSOT-alignment matrix and producer-shape contract not extended to CSI/DCS/ESC/APC categories** — found by /impl-hygiene-review during /fix-bug BUG-07-019 Phase 5 (run_id `/tmp/impl-hygiene-ori_term-RtDFbAFA`, Phase 3 findings F-05 + F-07 + F-11 cluster).
  Repro: read `crates/oriterm_test_support/src/catalog/tests.rs:333` — the `osc_tuple_sig_aligns_across_all_four_producers` matrix only iterates OSC selectors. CSI/DCS/ESC/APC also flow through `TupleSig` comparison in `spec-coverage-report --check` but lack a cross-producer regression matrix. Verifiable by `grep -nE 'CSI|DCS|ESC|APC' crates/oriterm_test_support/src/catalog/tests.rs | grep -v 'walk_catalog\|extract_dispatch_tuples_includes_known_csi\|build_dispatch_map_includes' | head -20` — no per-category matrix tests beyond OSC.
  Subsystem: `crates/oriterm_test_support/src/catalog/` + `spec_chain/uncataloged/`.
  Cluster components:
    (a) **F-05** — extend the matrix infrastructure to cover at least one CSI selector (`H` cursor position), one DCS selector (`q` sixel), one ESC selector (`D` IND), and the APC `(_, [], Pt, ST)` arm. Mirror the `osc_ssot_matrix` shape per category.
    (b) **F-07** — `perform_action_to_tuple` CSI arm (`uncataloged/mod.rs:106`) emits `params=""` while `dispatch_extract::csi` emits `params="Ps"` and `capture_extract::csi_dispatch` emits `params="Ps"` or `params="Ps;Ps"` per arity. Pre-fix divergence means CSI tuples have shape mismatch the new params-shape assertions (only run for OSC) would catch. Either reconcile the SSOT (`csi_params_placeholder` extracted from `capture_extract.rs` to `tuple/canonical.rs` and called from runtime + dispatch + capture) OR document the per-producer contract and add per-category matrix tests pinning the divergence.
    (c) **F-11** — adding sibling category matrices closes the same coverage gap that BUG-07-019 had to retrofit-fix for OSC. The OSC matrix would have caught BUG-07-019 if it had existed pre-fix; equivalent matrices for other categories are the structural prevention.
  Found: 2026-04-25 | Source: impl-hygiene-review | Reviewer: codex (Phase 3 deep analysis)

---

## 07.R Third Party Review Findings

- [x] `[BUG-07-001][high]` Auto-release one per UTC day limit.
  **Fixed 2026-03-30.** `bump-build.sh` now appends a sequence number (`.2`, `.3`, ...) when the current BUILD_NUMBER already has today's date. Format: `0.2.0-alpha.YYYYMMDD[.N]`.

- [x] `[BUG-07-002][high]` `release.yml` no longer validates tag matches workspace version.
  **Fixed 2026-03-30.** Added "Verify tag matches workspace version" step in `release.yml` that extracts the version from Cargo.toml and compares it to `github.ref_name`. Fails with a clear error if they don't match.

- [x] `[BUG-07-003][medium]` Auto-release `Cargo.lock` stale after version bump.
  **Fixed 2026-03-30.** Added `cargo generate-lockfile` step in `auto-release.yml` after `sync-version.sh` and before `git add`. Also added Rust toolchain installation step since `cargo` is needed.

---
