---
section: "03"
title: "oriterm_core Cleanup"
status: not-started
reviewed: false
goal: "Drive `cargo clippy -p oriterm_core --all-targets -- -D warnings` to exit 0 on host AND Windows GNU AND `--no-default-features` (image-protocol disabled), fixing 485 violations dominated by doc_markdown (301), field_reassign_with_default (42), needless_raw_strings (29), float_cmp (21 — JUDGMENT, mixed test+production), redundant_closure_for_method_calls (14), string_slice (14 — JUDGMENT)."
success_criteria:
  - "`cargo clippy -p oriterm_core --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` exits 0 (default features = image-protocol on)"
  - "`cargo clippy -p oriterm_core --all-targets --target x86_64-unknown-linux-gnu --no-default-features -- -D warnings` exits 0"
  - "`cargo clippy -p oriterm_core --all-targets --target x86_64-pc-windows-gnu -- -D warnings` exits 0"
  - "`cargo test -p oriterm_core` green; teseq/tack/vttest suites unaffected"
  - "All 21 `float_cmp` instances and 14 `string_slice` instances reviewed per-site: each marked with `#[expect(reason=...)]` (citing exact-representable or ASCII-safety) OR rewritten to epsilon comparison / char-boundary check"
  - "Closes BUG-07-005, BUG-07-010 (both supersede markers added in Section 10)"
  - "Connects upward to mission criteria: workspace clippy clean across all targets"
inspired_by:
  - "Section 02 oriterm_ipc cleanup pattern (auto-fix → diff review → manual cleanup → verification)"
  - "oriterm_ui/src/geometry/transform2d.rs:130 — `#[expect(clippy::float_cmp, reason=\"...\")]` precedent"
  - "oriterm_ui/src/icons/svg_import/mod.rs:100 — `#[expect(clippy::string_slice, reason=\"...\")]` precedent"
depends_on: ["02"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "03.1"
    title: "Auto-fix sweep with diff review (default features)"
    status: not-started
  - id: "03.2"
    title: "Manual cleanup of structural lints"
    status: not-started
  - id: "03.3"
    title: "Per-site float_cmp judgment (21 instances; mixed test + production)"
    status: not-started
  - id: "03.4"
    title: "Per-site string_slice judgment (14 instances; UTF-8 boundary safety)"
    status: not-started
  - id: "03.5"
    title: "Cross-target + --no-default-features verification"
    status: not-started
  - id: "03.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "03.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 03: oriterm_core Cleanup

**Status:** Not Started
**Goal:** Largest mechanical-dominant crate. 485 violations are ~93% mechanical (doc_markdown 301 + field_reassign 42 + needless_raw 29 + redundant_closure 14 + smaller mech = ~430), with 35 judgment calls (21 float_cmp + 14 string_slice). The crate also has the `image-protocol` default feature, so `--no-default-features` adds a verification cell.

**Success Criteria:** see frontmatter.

**Context:** oriterm_core is the terminal emulation library — Grid, Term, VTE handler, palette, selection, search. It is the backbone every other crate consumes. Violations are concentrated in test code (`oriterm_core/src/term/tests.rs` is huge with `len_zero`, `unnested_or_patterns`, `derive_partial_eq_without_eq`, `manual_let_else`, etc. per `BUG-07-010`'s repro) and in integration tests (`oriterm_core/tests/{vttest,teseq,tack,alloc_regression,rss_regression}/`). Production code is mostly clean — the violations cluster on the test surface that the broken gate never exposed.

**Reference implementations:**
- Section 02 oriterm_ipc cleanup (per-crate template)
- `oriterm_ui/src/geometry/transform2d.rs:130` (`#[expect(clippy::float_cmp, reason=...)]` precedent for production code)
- `oriterm_ui/src/icons/svg_import/mod.rs:100` (`#[expect(clippy::string_slice, reason="SVG markup is ASCII")]` precedent)

**Depends on:** Section 02 (oriterm_ipc clean — protocol/IPC types pass through here).

---

## Intelligence Reconnaissance

Queries run 2026-04-25:

- `scripts/intel-query.sh` — not present in this project; queries below used Grep / Glob / Read / cargo clippy --message-format=json instead.
- `cargo clippy -p oriterm_core --all-targets --message-format=json` — 485 violations, top distribution per `00-overview.md` Metrics. Scan-bucket parsing reveals ~430 M (mechanical), ~30 S (structural — `match_same_arms`, `unnested_or_patterns`, `manual_let_else`), ~25 J (21 `float_cmp` + 14 `string_slice` — wait, 35 not 25; correcting in baseline.md per Section 01).
- `Glob oriterm_core/src/term/tests.rs` — single file, large; per `BUG-07-010` carries `float_cmp` on `assert_eq!(viewport_y, 48.0)` (line 1926) and `len_zero` (`.len() > 0`) at line 2126. Both are in test assertions where the comparison IS exact-representable.
- `Glob oriterm_core/tests/` — vttest (8 menu*.rs files, BUG-07-005 cluster), teseq, tack, alloc_regression, rss_regression integration test directories. Most violations are `doc_markdown` in module-level `///` comments and integration-test scaffolding.
- `Grep -rn '#\[expect(clippy::float_cmp' oriterm_core/src/` — 0 existing precedent in oriterm_core (oriterm_ui has the precedent at `transform2d.rs:130`).

Results summary (≤500 chars) [ori]: 485 violations. ~430 mechanical (auto-fixable). 35 judgment calls split: 21 `float_cmp` (test code per `tests.rs:1926` analogous), 14 `string_slice` (integration test parsing — verify ASCII safety). Auto-fix sweep should drop count to ~50 manual + judgment items. `--no-default-features` adds a verification cell; image-protocol-disabled paths may surface additional doc_markdown for `#[cfg]`-gated docs.

See _(intel graph not available in this project; use Grep/Glob)_ for the full query protocol.

---

## 03.1 Auto-fix sweep with diff review (default features)

**File(s):** mostly `oriterm_core/src/term/tests.rs`, `oriterm_core/tests/{vttest,teseq,tack,alloc_regression,rss_regression}/**/*.rs`

- [ ] `cargo clippy --fix --all-targets -p oriterm_core --target x86_64-unknown-linux-gnu --allow-dirty`
- [ ] Capture diff: `git diff -- oriterm_core/ > /tmp/oriterm_core-autofix.diff`
- [ ] **Manual diff review** (this is a LARGE diff — auto-fix touched ~430 sites):
  - Sweep for any `manual_let_else` rewrite (Gemini round-1 concern). For each occurrence: read 5 lines before/after to verify drop-timing of the diverging arm matches original `match`/`if let` semantics. The `let-else` rewrite drops the matched value at the end of the function rather than the end of the block — semantics-changing only when the matched value owns a Drop type (file handles, locks, mutex guards). If found in oriterm_core hot paths: REVERT that hunk, leave as `match`/`if let`.
  - For `field_reassign_with_default` (42 sites): verify the rewrite to struct-literal preserves field-init order and any side effects (none expected — these are `Default::default()` reassignments).
  - For `doc_markdown` (301 sites): all are backtick-wrapping. Spot-check 10-20 to verify backticks don't break doc-link or cross-reference syntax.
  - For `needless_raw_strings` (29 sites): verify dropping `r"..."` to `"..."` doesn't break a string with embedded `\n`/`\t` semantics (raw strings preserve those literally; non-raw strings interpret).
- [ ] Run `cargo test -p oriterm_core` — full crate test suite green. Teseq/tack/vttest run as integration tests; if any insta snapshot drifts, that's a real test failure (lint fix changed semantics) — STOP and revert the offending hunk.
- [ ] Commit: `chore(oriterm_core): apply cargo clippy --fix for ~430 mechanical lint cleanups`.

- [ ] **Subsection close-out (03.1)**: status → complete; `/improve-tooling` (clippy-fix.sh wrapper from Section 02 paid off?); `/sync-claude` (no API changes); repo hygiene.

---

## 03.2 Manual cleanup of structural lints

**File(s):** `oriterm_core/src/term/tests.rs` (most), various test files

- [ ] Run `cargo clippy -p oriterm_core --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` and capture remaining violations (~50 expected after auto-fix).
- [ ] For each `match_same_arms` (6 sites): consolidate via `_ =>` or merge with `|`. Verify match exhaustiveness preserved.
- [ ] For `unnested_or_patterns` (5 sites): rewrite `Foo::A(x) | Foo::B(x)` to `Foo::A(x) | Foo::B(x)` with proper grouping per the lint suggestion.
- [ ] For `manual_let_else` if any survived auto-fix: each requires drop-timing analysis. If diverging arm doesn't own a Drop type, accept `let-else` rewrite manually. If it does, leave as `match` and add `#[expect(clippy::manual_let_else, reason="diverging arm owns Drop type X — rewrite changes drop timing")]`.
- [ ] For `derive_partial_eq_without_eq` (1 site flagged in BUG-07-010): add `Eq` derive if the type is genuinely `Eq` (no float fields, no `f32`/`f64`), else `#[expect(reason="contains f32 fields, semantic equality differs from PartialEq")]`.
- [ ] For each remaining lint: verify suggestion doesn't change behavior; apply.
- [ ] `cargo test -p oriterm_core` green; teseq/tack/vttest snapshots unchanged.
- [ ] Commit: `style(oriterm_core): cleanup ~50 structural clippy violations`.

- [ ] **Subsection close-out (03.2)**: status → complete; retrospectives; repo hygiene.

---

## 03.3 Per-site float_cmp judgment (21 instances)

**File(s):** TBD — `cargo clippy -p oriterm_core --all-targets --message-format=json` enumeration filtered by `code == clippy::float_cmp`

For each of the 21 sites, classify per the `transform2d.rs:130` precedent:

- **Exact-representable** (literals like `0.0`, `1.0`, integer-cast floats, identity values): apply `#[expect(clippy::float_cmp, reason="comparing exact-representable literal {value}")]` at the call site.
- **Computed-value comparison** (arithmetic-derived floats compared for equality): rewrite to `(a - b).abs() < EPSILON` with `const EPSILON: f64 = 1e-9` or appropriate for context. Document the EPSILON in a module-level constant if reused.
- **Test-only assert_eq! invariants** (e.g., `assert_eq!(viewport_y, 48.0)` per BUG-07-010 — geometry test where 48.0 is constructed by exact arithmetic): apply `#[expect(...)]` at the call OR if many in one test file, use module-level `#![expect(clippy::float_cmp, reason="...")]` at the top of the test file.

- [ ] Enumerate the 21 sites: run the message-format=json walk-expansion script from baseline capture.
- [ ] For each site: read context, classify, apply fix, verify.
- [ ] If any site reveals a real precision bug (computed vs computed without epsilon), fix the underlying code AND mark the change in §3 implementation notes — semantics changed, requires a regression test.
- [ ] Commit: `style(oriterm_core): add #[expect(float_cmp, reason=...)] for 21 exact-representable comparisons`.

- [ ] **Subsection close-out (03.3)**: status → complete; retrospectives (was the per-site judgment tedious enough to merit `diagnostics/clippy-float-cmp-walker.py`? Section 05 will hit 50 test files in oriterm_ui — if so, file the tool now); repo hygiene.

---

## 03.4 Per-site string_slice judgment (14 instances)

**File(s):** TBD — enumerate via JSON

For each of the 14 sites, the lint flags `&s[a..b]` operations that may panic on UTF-8 boundary mismatch. Per the `oriterm_ui/src/icons/svg_import/mod.rs:100` precedent, the fix is per-site:

- **ASCII-safe** (input is provably ASCII per surrounding code — e.g., parsed CSI sequence bytes, terminfo entry parsing): apply `#[expect(clippy::string_slice, reason="ASCII-only input — escape sequence parameter")]`.
- **Char-boundary safe** (caller has verified `is_char_boundary(a) && is_char_boundary(b)`): same `#[expect]` with reason.
- **Unsafe** (input may be arbitrary UTF-8): rewrite using `s.get(a..b)` (returns `Option<&str>`) or `s.chars().skip(a).take(b - a).collect::<String>()`.

- [ ] Enumerate the 14 sites, classify per above.
- [ ] Apply fixes; verify `cargo test -p oriterm_core` green.
- [ ] Commit: `style(oriterm_core): add #[expect(string_slice, reason=...)] / Result-safe rewrites for 14 sites`.

- [ ] **Subsection close-out (03.4)**: status → complete; retrospectives; repo hygiene.

---

## 03.5 Cross-target + --no-default-features verification

**File(s):** none (verification only)

- [ ] `cargo clippy -p oriterm_core --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` exits 0 (default features = image-protocol enabled)
- [ ] `cargo clippy -p oriterm_core --all-targets --target x86_64-unknown-linux-gnu --no-default-features -- -D warnings` exits 0 — disabling image-protocol must NOT introduce new violations (e.g., `dead_code` if a use becomes unreachable, or doc-link warnings on `#[cfg(feature)]`-gated items)
- [ ] `cargo clippy -p oriterm_core --all-targets --target x86_64-pc-windows-gnu -- -D warnings` exits 0
- [ ] `cargo clippy -p oriterm_core --all-targets --target x86_64-pc-windows-gnu --no-default-features -- -D warnings` exits 0
- [ ] If `--no-default-features` surfaces NEW violations, fix inline and update bug entries' counts in baseline.md.

- [ ] **Subsection close-out (03.5)**: status → complete; retrospectives; repo hygiene.

---

## 03.R Third Party Review Findings

<!-- Reserved for /tpr-review (Codex + Gemini). -->

- None.

---

## 03.N Completion Checklist

- [ ] All four target × feature cells exit 0 (`-D warnings`)
- [ ] `cargo test -p oriterm_core` green; teseq/tack/vttest insta snapshots unchanged
- [ ] `cargo test --all` green (regression canary)
- [ ] All 21 float_cmp + 14 string_slice judgment sites have committed `#[expect(reason=...)]` or rewrites
- [ ] `BUG-07-005` and `BUG-07-010` remain `[ ]` (closure in Section 10)
- [ ] **Plan sync**: section 03 status → complete in section file + 00-overview.md + index.md
- [ ] `/tpr-review` passed (large diff; TPR is mandatory)
- [ ] `/impl-hygiene-review` passed (after TPR clean)
- [ ] `/improve-tooling` section-close sweep — by now `clippy-fix.sh` should be a committed wrapper. If 03.3/03.4 surfaced a per-site enumeration tool need, file as cross-cutting.
- [ ] `/sync-claude` section-close doc sync
- [ ] **Repo hygiene check**

**Exit Criteria:** All four oriterm_core target × feature combinations exit 0; teseq/tack/vttest green; 21 float_cmp + 14 string_slice judgment sites have explicit per-site verdicts committed; section frontmatter and overview/index reflect complete.
