---
section: "05"
title: "oriterm_ui Cleanup"
status: not-started
reviewed: false
goal: "Drive `cargo clippy -p oriterm_ui --all-targets --features testing -- -D warnings` to exit 0 on host AND Windows GNU, fixing 761 violations: 145 mechanical/structural + 616 float_cmp instances distributed across 50 oriterm_ui test files (per-test-file `#![expect(clippy::float_cmp, reason=...)]` review IS the unit of work, NOT per-instance)."
success_criteria:
  - "`cargo clippy -p oriterm_ui --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` exits 0 (default features)"
  - "`cargo clippy -p oriterm_ui --all-targets --target x86_64-unknown-linux-gnu --features testing -- -D warnings` exits 0"
  - "`cargo clippy -p oriterm_ui --all-targets --target x86_64-pc-windows-gnu -- -D warnings` exits 0"
  - "`cargo clippy -p oriterm_ui --all-targets --target x86_64-pc-windows-gnu --features testing -- -D warnings` exits 0"
  - "`cargo test -p oriterm_ui` green; widget harness tests, animation tests, layout tests all unaffected"
  - "All 50 oriterm_ui test files containing float_cmp have either (a) module-level `#![expect(clippy::float_cmp, reason=...)]` annotation OR (b) per-call `#[expect]` with reason OR (c) the tests rewritten to use approx_eq macro from a workspace shared helper (option (a) recommended for files where ALL float_cmp uses are intentional bitwise comparison; option (c) defers a workspace-shared helper decision to a follow-up plan if needed)"
  - "Closes BUG-07-006 (supersede in Section 10)"
inspired_by:
  - "Section 02 oriterm_ipc cleanup pattern"
  - "Section 03 oriterm_core cleanup pattern"
  - "oriterm_ui/src/geometry/transform2d.rs:130 (production-code #[expect(clippy::float_cmp, reason=\"...\")] precedent)"
  - "oriterm_ui/src/icons/footer.rs:7 (file-level #![expect(clippy::unreadable_literal, reason=\"...\")] precedent)"
depends_on: ["04"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "05.1"
    title: "Auto-fix sweep with diff review (default + testing features)"
    status: not-started
  - id: "05.2"
    title: "Manual cleanup of 145 mechanical/structural lints (non-float_cmp)"
    status: not-started
  - id: "05.3"
    title: "Per-test-file float_cmp review (50 files, 616 instances)"
    status: not-started
  - id: "05.4"
    title: "Cross-target + feature-matrix verification"
    status: not-started
  - id: "05.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "05.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 05: oriterm_ui Cleanup

**Status:** Not Started
**Goal:** Largest crate by violation count (761) and the most architecturally sensitive cleanup. The 616 float_cmp instances are the dominant feature; Phase 1 research verified they are 100% in test code (zero in production), distributed across 50 test files. Per-test-file review collapses 616 per-instance judgments into 50 per-file judgments — a 12× reduction.

**Success Criteria:** see frontmatter.

**Context:** oriterm_ui is the UI framework crate (widgets, WindowRoot, interaction, layout, animation, testing). It has the workspace's dominant lint surface. Per Section 01 baseline:
- 145 non-float_cmp violations: doc_markdown 45, unreadable_literal 27, assertions_on_constants 13, items_after_statements 8, field_reassign_with_default 6, used_underscore_binding 6, too_many_lines 5, unchecked_time_subtraction 5, needless_collect 5, redundant_clone 4, while_float 3, plus a long tail.
- 616 float_cmp violations across 50 test files: top file is `geometry/tests.rs` (66), then `animation/tests.rs` (56), `color/tests.rs` (56), `widgets/container/tests.rs` (33), `widgets/scrollbar/tests.rs` (33), and 45 more files with 1-27 instances each.

The float_cmp distribution reflects the project's testing style: widget layout, geometry, color, animation, and interaction tests use `assert_eq!(computed_f32, expected_f32_literal)` patterns where the expected value is constructed from exact-representable arithmetic (e.g., `1.0 + 2.0`, `screen_w * 0.5`, identity transforms). Bitwise equality IS the intended invariant — these are test pins, not approximate comparisons.

**Reference implementations:**
- Section 02-04 cleanup pattern (auto-fix → diff → manual → verify)
- `oriterm_ui/src/geometry/transform2d.rs:130` — production-code `#[expect(clippy::float_cmp, reason="identity is constructed with exact literals")]` precedent
- `oriterm_ui/src/icons/footer.rs:7` — file-level `#![expect(clippy::unreadable_literal, reason="generated icon coordinates")]` precedent (the structural model for option (a))

**Depends on:** Section 04 (oriterm_test_support clean — used as dev-dep in oriterm_ui's own test compilation).

---

## Intelligence Reconnaissance

Queries run 2026-04-25:

- `scripts/intel-query.sh` — not present in this project; queries below used Grep / Glob / Read / cargo clippy --message-format=json instead.
- `cargo clippy -p oriterm_ui --all-targets --message-format=json | walk-expansion-chain` — 616 float_cmp distinct call sites in 50 test files; 0 in production. Top file: `oriterm_ui/src/geometry/tests.rs` (66), confirming geometry test density.
- `Read oriterm_ui/src/icons/footer.rs:7` — file-level `#![expect(clippy::unreadable_literal, reason="generated icon coordinates")]` precedent; the structural model for option (a) per-test-file annotation.
- `Glob oriterm_ui/Cargo.toml` — `[features] testing = []` declared at line 30-31; pure cfg-gate, no transitive deps.

Results summary (≤500 chars) [ori]: 761 violations: 145 mech/structural + 616 float_cmp in 50 test files. Per-test-file `#![expect(clippy::float_cmp, reason=...)]` annotation IS the canonical fix for files where ALL float_cmps are exact-representable. Production code is clean (transform2d.rs:130 is the existing precedent). Widget/layout/animation/color tests dominate the surface.

See _(intel graph not available in this project; use Grep/Glob)_ for the full query protocol.

---

## 05.1 Auto-fix sweep with diff review (default + testing features)

**File(s):** `oriterm_ui/src/**/*.rs` — wide blast radius given 145 mechanical violations

- [ ] `cargo clippy --fix --all-targets -p oriterm_ui --target x86_64-unknown-linux-gnu --features testing --allow-dirty`
- [ ] `git diff -- oriterm_ui/ > /tmp/oriterm_ui-autofix.diff`
- [ ] **LARGE diff review** (likely 100+ hunks). Manual sweep order:
  - `doc_markdown` (45 sites): backtick-wrapping; spot-check 10 to verify no doc-link breakage.
  - `unreadable_literal` (27 sites): underscore-separator insertion (e.g., `1234567` → `1_234_567`); semantics-preserving.
  - `assertions_on_constants` (13 sites): drop the assertion or rewrite to a non-constant condition; this can change test coverage if a constant-false assertion was a test scaffold. Verify each site.
  - `items_after_statements` (8 sites): hoist item declarations above statement blocks; semantics-preserving.
  - `field_reassign_with_default` (6 sites): rewrite to struct literal; verify field order if any has side effects.
  - `used_underscore_binding` (6 sites): rename `_foo` → `foo`; verify no shadowing collision.
  - `too_many_lines` (5 sites): may apply only `#[expect]` rather than refactor; auto-fix likely doesn't apply this lint at all (manual in 05.2).
  - **`while_float` (3 sites)**: this lint flags `while f32_var < limit` patterns where floating-point loop counters can drift. Auto-fix may rewrite to integer iteration. CRITICAL: verify rewrite preserves test semantics; if integer rewrite changes iteration count, that's a real semantic change, NOT a lint fix. REVERT the hunk; use `#[expect(reason=...)]` per site.
- [ ] `cargo test -p oriterm_ui` green; widget harness tests + animation tests pass.
- [ ] Commit: `chore(oriterm_ui): apply cargo clippy --fix for ~120 mechanical lints`.

- [ ] **Subsection close-out (05.1)**: standard template.

---

## 05.2 Manual cleanup of 145 mechanical/structural lints (non-float_cmp)

**File(s):** various oriterm_ui src + test files

- [ ] Enumerate remaining (~25-50): `cargo clippy -p oriterm_ui --all-targets --target x86_64-unknown-linux-gnu --features testing -- -D warnings | grep -v float_cmp`.
- [ ] For each: classify, fix per the canonical pattern.
- [ ] Specifically watch:
  - `unchecked_time_subtraction` (5 sites): `Instant::now() - earlier` patterns can panic on mock-time tests. Per-site verdict: `#[expect(reason="test uses Duration::from_secs literals; underflow impossible")]` for test code; `checked_duration_since` rewrite for production.
  - `too_many_lines` (5 sites): if function body genuinely exceeds 100 lines and refactor would change architecture (cross-cutting), apply `#[expect(reason="dispatch table" | "icon path-data table" | ...)]`. Refactor only when the function is a clear single-responsibility violation.
  - `redundant_clone` (4 sites): per Section 02.2 protocol — verify value isn't reused after.
- [ ] `cargo test -p oriterm_ui` green; widget harness tests pass.
- [ ] Commit: `style(oriterm_ui): cleanup ~25 structural lints (non-float_cmp)`.

- [ ] **Subsection close-out (05.2)**: standard template.

---

## 05.3 Per-test-file float_cmp review (50 files, 616 instances)

**File(s):** 50 oriterm_ui test files (full list below; enumerate via JSON walk-expansion).

This is the section's most distinctive subsection. Per Phase 1 verification, all 616 float_cmp instances are in test code, distributed across 50 files. The fix is per-FILE, not per-INSTANCE.

For each test file:
1. Read the file in full (or at least every `assert_eq!`/`assert!`/`==` involving f32/f64).
2. Determine if EVERY float_cmp in the file is exact-representable (literal comparison, integer-cast, identity value).
3. If YES → add module-level `#![expect(clippy::float_cmp, reason="<reason>")]` at the top of the file. Reason should describe WHY the comparisons are exact-representable: "test pins exact-representable layout values constructed from integer arithmetic" / "test pins identity transforms with literal coordinates" / etc.
4. If MIXED (most exact, some computed) → use per-call `#[expect(...)]` for the exact ones, rewrite the computed ones to epsilon comparison with a documented `EPSILON: f32 = 1e-6` (or appropriate).
5. If ALL computed (rare in test code, possible in animation easing tests) → rewrite all to epsilon comparison; do NOT use `#![expect]` (the lint is correctly flagging real concerns).

**Top files by float_cmp count** (from Section 01 baseline):
- `oriterm_ui/src/geometry/tests.rs` (66) — Rect / Size / Point arithmetic; almost certainly all exact-representable
- `oriterm_ui/src/animation/tests.rs` (56) — animation interpolation; MIXED expected (some exact-progress like `0.0`, `1.0`; some interpolated values like `easeInOut(0.5)` that should use epsilon)
- `oriterm_ui/src/color/tests.rs` (56) — color channel arithmetic; depends on style — sRGB vs linear, alpha compositing
- `oriterm_ui/src/widgets/container/tests.rs` (33) — layout pin tests; almost certainly exact-representable
- `oriterm_ui/src/widgets/scrollbar/tests.rs` (33) — scroll position pin tests; likely exact-representable
- `oriterm_ui/src/overlay/tests.rs` (27) — overlay placement; likely exact-representable
- `oriterm_ui/src/widgets/number_input/tests.rs` (24) — numeric widget; could be MIXED (parsed values vs literal expectations)
- ... 43 more files

- [ ] Enumerate the 50 files: run `cargo clippy ... --message-format=json` walk-expansion script; produce `/tmp/oriterm_ui-float_cmp-files.txt` with file path + count.
- [ ] **For each file** (50 iterations):
  - Read the file.
  - Classify per (1)/(2)/(3) above.
  - Apply the chosen fix.
  - Run `cargo test -p oriterm_ui --test {test_target}` for that file's test target — verify the file's tests still pass.
  - Commit per-file or per-cluster (e.g., one commit for "all geometry tests" — `style(oriterm_ui/geometry): file-level #![expect(float_cmp)] for exact-representable layout pins`).
- [ ] After all 50 files processed, run `cargo clippy -p oriterm_ui --all-targets --features testing -- -D warnings`; verify 0 float_cmp errors remain.
- [ ] **Decision document**: write `plans/clippy-gate-hardening/oriterm-ui-float-cmp-decisions.md` listing each of the 50 files with classification (exact / mixed / computed) + reason — this is the audit trail for future reviewers.
- [ ] Commit decision document: `docs(plans/clippy-gate-hardening): document per-test-file float_cmp classifications`.

- [ ] **Subsection close-out (05.3)**: standard template; `/improve-tooling` retrospective should specifically reflect on whether the 50-file walk could be templated (e.g., a `diagnostics/clippy-float-cmp-classify.py` that reads each file, finds all `assert_eq!(.*f3?2, ...)` patterns, suggests classification).

---

## 05.4 Cross-target + feature-matrix verification

- [ ] `cargo clippy -p oriterm_ui --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` exits 0 (default features)
- [ ] `cargo clippy -p oriterm_ui --all-targets --target x86_64-unknown-linux-gnu --features testing -- -D warnings` exits 0
- [ ] `cargo clippy -p oriterm_ui --all-targets --target x86_64-pc-windows-gnu -- -D warnings` exits 0 (default)
- [ ] `cargo clippy -p oriterm_ui --all-targets --target x86_64-pc-windows-gnu --features testing -- -D warnings` exits 0
- [ ] If `--features testing` surfaces lints not seen in default (the `oriterm_ui::testing` module is gated), fix inline and update count in baseline.md.

- [ ] **Subsection close-out (05.4)**: standard template.

---

## 05.R Third Party Review Findings

- None.

---

## 05.N Completion Checklist

- [ ] All 4 target × feature cells exit 0 (`-D warnings`)
- [ ] `cargo test -p oriterm_ui` green; widget harness + animation + color tests pass
- [ ] `cargo test --all` green (regression canary)
- [ ] All 50 oriterm_ui test files with float_cmp have committed annotations or rewrites; decision document committed
- [ ] BUG-07-006 remains `[ ]` (closure in Section 10)
- [ ] **Plan sync**: section 05 status → complete in section file + 00-overview.md + index.md
- [ ] `/tpr-review` passed (large diff with judgment; TPR is critical)
- [ ] `/impl-hygiene-review` passed (after TPR clean)
- [ ] `/improve-tooling` section-close sweep — strong candidate: `diagnostics/clippy-float-cmp-classify.py` that scans test files for assert_eq!(f32, f32) patterns and suggests classification
- [ ] `/sync-claude` section-close doc sync
- [ ] **Repo hygiene check**

**Exit Criteria:** All four oriterm_ui target × feature combinations exit 0; widget harness + animation tests green; per-test-file float_cmp decisions committed in `oriterm-ui-float-cmp-decisions.md`; section frontmatter and overview/index reflect complete.
