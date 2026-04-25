---
section: "04"
title: "oriterm_test_support Cleanup"
status: not-started
reviewed: false
goal: "Drive `cargo clippy -p oriterm_test_support --all-targets -- -D warnings` to exit 0 on host AND Windows GNU, fixing 27 violations (predominantly mechanical: doc_markdown 7, format_push_string 4, duration_suboptimal_units 3, plus structural needless_collect/iter_on_single_items)."
success_criteria:
  - "`cargo clippy -p oriterm_test_support --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` exits 0"
  - "`cargo clippy -p oriterm_test_support --all-targets --target x86_64-pc-windows-gnu -- -D warnings` exits 0"
  - "`cargo test -p oriterm_test_support` green; PtySession + tack_framework helpers unaffected"
  - "Closes BUG-07-012 (supersede in Section 10)"
  - "Connects upward to mission criteria: 'workspace clippy clean on host AND Windows GNU' and 'BUG-07-012 closed bidirectionally (closure in Section 10)'"
inspired_by:
  - "Section 02 oriterm_ipc cleanup pattern"
  - "Section 03 oriterm_core cleanup pattern"
depends_on: ["03"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "04.1"
    title: "Auto-fix sweep with diff review"
    status: not-started
  - id: "04.2"
    title: "Manual cleanup of structural violations"
    status: not-started
  - id: "04.3"
    title: "Cross-target verification"
    status: not-started
  - id: "04.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "04.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 04: oriterm_test_support Cleanup

**Status:** Not Started
**Goal:** Mid-small crate, ~70% mechanical. Test-support helpers (`PtySession`, `tack_framework`, `cap_coverage`) consumed by every other crate's tests as a dev-dep. Cleaning here unblocks downstream crates' integration test compilation (already implicit, but explicitly verified at section close).

**Success Criteria:** see frontmatter.

**Context:** `crates/oriterm_test_support/` is the workspace test helper crate. Per BUG-07-012, 27 violations across `session/sync/tests.rs`, `version_gate/tests.rs`, `tack_framework/{cap_coverage,runner,scenarios}/tests.rs`. All in test files; lib target is clean. The crate is consumed by every other workspace crate as a dev-dep, so its test cleanup is a prerequisite for downstream sections' integration-test clippy runs.

**Reference implementations:**
- Section 02 oriterm_ipc cleanup template (auto-fix → diff review → manual → verification)
- BUG-07-012 in `plans/bug-tracker/section-07-ci-build.md` — the canonical violation list

**Depends on:** Section 03 (oriterm_core clean — its types are consumed by test_support helpers).

---

## Intelligence Reconnaissance

Queries run 2026-04-25:

- `scripts/intel-query.sh` — not present in this project; queries below used Grep / Glob / Read / cargo clippy --message-format=json instead.
- Per Section 01 baseline: 27 violations = doc_markdown 7, format_push_string 4, duration_suboptimal_units 3, needless_collect 2, map_unwrap_or 2, iter_on_single_items 2, plus 7 single-instance lints. M=19, S=5, J=0.
- All in `crates/oriterm_test_support/src/{session,tack_framework,scenarios}/**/tests.rs` per BUG-07-012's enumeration.

Results summary (≤500 chars) [ori]: 27 violations, ~70% mechanical, no judgment cases. Auto-fix sweep should drop count to ~5 manual sites. No special concerns; standard per-crate template applies.

See _(intel graph not available in this project; use Grep/Glob)_ for the full query protocol.

---

## 04.1 Auto-fix sweep with diff review

**File(s):** `crates/oriterm_test_support/src/{session,tack_framework,scenarios}/**/tests.rs`

- [ ] `cargo clippy --fix --all-targets -p oriterm_test_support --target x86_64-unknown-linux-gnu --allow-dirty`
- [ ] `git diff -- crates/oriterm_test_support/ > /tmp/test_support-autofix.diff`
- [ ] Manual diff review per Section 02 template — focus on `format_push_string` rewrites (verify `write!(s, ...)` preserves exact formatting), `duration_suboptimal_units` rewrites (verify unit conversion is correct), `iter_on_single_items` (verify behavior on single-item iteration paths).
- [ ] `cargo test -p oriterm_test_support` green.
- [ ] Commit: `chore(oriterm_test_support): apply cargo clippy --fix for 19 mechanical lints`.

- [ ] **Subsection close-out (04.1)**: standard template (status → complete; `/improve-tooling`; `/sync-claude`; repo hygiene).

---

## 04.2 Manual cleanup of structural violations

**File(s):** various tests.rs in oriterm_test_support

- [ ] Enumerate remaining ~5-8 violations: `cargo clippy -p oriterm_test_support --all-targets --target x86_64-unknown-linux-gnu -- -D warnings`.
- [ ] For each `needless_collect` (2 sites): verify the iterator can be consumed in-place; rewrite removing `.collect::<Vec<_>>()` intermediate.
- [ ] For each `iter_on_single_items` (2 sites): rewrite per suggestion (`std::iter::once` or direct value use).
- [ ] For `case_sensitive_file_extension_comparisons` (1 site): rewrite to `Path::new(s).extension().map_or(false, |e| e.eq_ignore_ascii_case("rs"))` per the lint suggestion.
- [ ] For `pass_by_value_not_consumed` (1 site at `tack_framework/runner/tests.rs:181`): verify whether `&T` over `T` changes ownership semantics; if not, accept; if yes, `#[expect(...)]`.
- [ ] `cargo test -p oriterm_test_support` green.
- [ ] Commit: `style(oriterm_test_support): cleanup remaining structural violations`.

- [ ] **Subsection close-out (04.2)**: standard template.

---

## 04.3 Cross-target verification

- [ ] `cargo clippy -p oriterm_test_support --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` exits 0
- [ ] `cargo clippy -p oriterm_test_support --all-targets --target x86_64-pc-windows-gnu -- -D warnings` exits 0
- [ ] Spot-check: downstream crates' (oriterm_core, oriterm_ui, oriterm) integration tests that use oriterm_test_support as dev-dep still compile clean once their own sections start.

- [ ] **Subsection close-out (04.3)**: standard template.

---

## 04.R Third Party Review Findings

- None.

---

## 04.N Completion Checklist

- [ ] Both target cells exit 0 (`-D warnings`)
- [ ] `cargo test -p oriterm_test_support` green
- [ ] `cargo test --all` green (regression canary)
- [ ] BUG-07-012 remains `[ ]` (closure in Section 10)
- [ ] **Plan sync**: section 04 status → complete in section file + 00-overview.md + index.md
- [ ] `/tpr-review` passed
- [ ] `/impl-hygiene-review` passed (after TPR clean)
- [ ] `/improve-tooling` section-close sweep
- [ ] `/sync-claude` section-close doc sync
- [ ] **Repo hygiene check**

**Exit Criteria:** Both oriterm_test_support target cells exit 0; downstream consumers' dev-dep compilation unaffected; section frontmatter and overview/index reflect complete.
