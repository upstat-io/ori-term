---
section: "06"
title: "oriterm_mux Cleanup"
status: not-started
reviewed: false
goal: "Drive `cargo clippy -p oriterm_mux --all-targets -- -D warnings` to exit 0 on host AND Windows GNU, fixing 192 violations: 121 mechanical, 18 structural, 14 judgment (decimal_bitwise_operands 12, unchecked_time_subtraction 8, string_slice 3, map_err_ignore 1, needless_pass_by_value 1)."
success_criteria:
  - "`cargo clippy -p oriterm_mux --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` exits 0"
  - "`cargo clippy -p oriterm_mux --all-targets --target x86_64-pc-windows-gnu -- -D warnings` exits 0"
  - "`cargo test -p oriterm_mux` green; pane lifecycle, snapshot double-buffer, IO thread tests unaffected"
  - "All 14 judgment-class violations have committed `#[expect(reason=...)]` or refactored fixes"
  - "Closes BUG-07-NNN (oriterm_mux entry from Section 01.2; supersede in Section 10)"
inspired_by:
  - "Section 02-05 cleanup pattern"
depends_on: ["05"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "06.1"
    title: "Auto-fix sweep with diff review"
    status: not-started
  - id: "06.2"
    title: "Manual cleanup of 18 structural violations"
    status: not-started
  - id: "06.3"
    title: "Per-site judgment review (decimal_bitwise_operands × 12, unchecked_time_subtraction × 8, etc.)"
    status: not-started
  - id: "06.4"
    title: "Cross-target verification"
    status: not-started
  - id: "06.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "06.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 06: oriterm_mux Cleanup

**Status:** Not Started
**Goal:** Second-largest crate by violation count (192). 121 mechanical lints clear via auto-fix; the bulk of the manual work is in the 18 structural and 14 judgment cases — particularly `unchecked_time_subtraction` which can panic on mock-time tests, and `decimal_bitwise_operands` which often signals a real protocol-bit-flag confusion.

**Success Criteria:** see frontmatter.

**Context:** oriterm_mux is the pane server (PTY I/O, pane lifecycle, snapshot double-buffer, mux backend, daemon-server IPC protocol). It owns the IO thread and snapshot transfer to the main thread. Per Section 01 baseline:
- 121 mechanical lints: doc_markdown 85, used_underscore_binding 37, default_trait_access 11, items_after_statements 8, manual_assert 5, needless_continue 4, no_effect_underscore_binding 3, etc.
- 18 structural lints: redundant_clone 4, plus a long tail.
- 14 judgment lints: **decimal_bitwise_operands 12** (often a real bug; likely PDU codec or protocol bit-flag operations), **unchecked_time_subtraction 8** (can panic; relevant for IO thread that does time-based throttling), **string_slice 3** (UTF-8 boundary), **map_err_ignore 1**, **needless_pass_by_value 1**.

The judgment category for oriterm_mux is more correctness-sensitive than the smaller crates — `unchecked_time_subtraction` panics in production are real risks; `decimal_bitwise_operands` may indicate a missed cast (decimal vs hex literal mismatch).

**Reference implementations:**
- Section 02-05 cleanup pattern
- `oriterm_ui/src/draw/border/mod.rs:111` — `#[expect(clippy::float_cmp, reason="...")]` precedent for production-code judgment

**Depends on:** Section 05 (oriterm_ui clean — used as dev-dep in some oriterm_mux tests; also `oriterm_core` clean per transitive Section 03 — oriterm_mux's `Term` is `oriterm_core::Term`).

---

## Intelligence Reconnaissance

Queries run 2026-04-25:

- `scripts/intel-query.sh` — not present in this project; queries below used Grep / Glob / Read / cargo clippy --message-format=json instead.
- `cargo clippy -p oriterm_mux --all-targets --message-format=json | bucket-by-code` — 192 violations confirmed; top distribution per Section 01 baseline.
- `Grep -rn 'Instant::now\(\) -' oriterm_mux/src/` — locate the 8 `unchecked_time_subtraction` sites; expected in IO thread throttling, snapshot timestamps, and PTY read-deadline logic.
- `Grep -rn '0x[0-9A-Fa-f]+ \| 0x[0-9A-Fa-f]+\|0x[0-9A-Fa-f]+ \& 0x[0-9A-Fa-f]+' oriterm_mux/src/` — locate the 12 `decimal_bitwise_operands` sites; expected in PDU codec.

Results summary (≤500 chars) [ori]: 192 violations dominate-mechanical. 8 `unchecked_time_subtraction` (IO thread throttling — verify each `now() - earlier` cannot underflow given surrounding code; rewrite to `checked_duration_since` if it can). 12 `decimal_bitwise_operands` (PDU codec — verify each is a real bit-flag operation or a missed hex-cast). 121 mechanical clear via auto-fix.

See _(intel graph not available in this project; use Grep/Glob)_ for the full query protocol.

---

## 06.1 Auto-fix sweep with diff review

**File(s):** `oriterm_mux/src/**/*.rs`, `oriterm_mux/tests/**/*.rs`

- [ ] `cargo clippy --fix --all-targets -p oriterm_mux --target x86_64-unknown-linux-gnu --allow-dirty`
- [ ] Capture diff; manual review per Section 02 template.
- [ ] Specifically watch:
  - `used_underscore_binding` (37 sites): rename `_foo` → `foo`. Verify no shadowing.
  - `default_trait_access` (11 sites): `T::default()` → `<T as Default>::default()` or struct-literal default. Verify field-init order if any has side effects.
  - `manual_assert` (5 sites): `panic!` → `assert!`. Verify panic message format preserved.
- [ ] `cargo test -p oriterm_mux` green; IO thread + snapshot double-buffer tests pass.
- [ ] Commit: `chore(oriterm_mux): apply cargo clippy --fix for 121 mechanical lints`.

- [ ] **Subsection close-out (06.1)**: standard template.

---

## 06.2 Manual cleanup of 18 structural violations

- [ ] Enumerate remaining (~18): `cargo clippy -p oriterm_mux --all-targets --target x86_64-unknown-linux-gnu -- -D warnings`.
- [ ] Per-site fix: redundant_clone (4 — verify reuse), needless_pass_by_value (1 — verify ownership requirements), unnecessary_cast (varies — verify cast IS unnecessary, not a deliberate type narrowing).
- [ ] `cargo test -p oriterm_mux` green.
- [ ] Commit: `style(oriterm_mux): cleanup 18 structural lints`.

- [ ] **Subsection close-out (06.2)**: standard template.

---

## 06.3 Per-site judgment review (14 instances)

**File(s):** TBD per JSON enumeration

### unchecked_time_subtraction (8 sites)

For each site:
- Read ±10 lines around the cite.
- Determine whether `Instant::now() - earlier_instant` can underflow given surrounding code:
  - If `earlier` is captured from an earlier `Instant::now()` call WITHOUT clock manipulation in between → cannot underflow → `#[expect(clippy::unchecked_time_subtraction, reason="<source of earlier_instant; underflow impossible>")]`.
  - If `earlier` comes from a snapshot or external source → CAN underflow → rewrite to `Instant::now().checked_duration_since(earlier).unwrap_or(Duration::ZERO)` per the lint suggestion.
- Apply per-site verdict.

### decimal_bitwise_operands (12 sites)

For each site:
- Read the operation (`a | b` or `a & b` where one operand is a decimal literal).
- Determine whether the literal is a hex-flag mistake (e.g., `0x40 | 0x80` typed as `64 | 128`) or genuinely decimal arithmetic (e.g., `pdu_size | LENGTH_PREFIX_BYTES` where the prefix bytes count is decimal):
  - If hex-flag mistake → rewrite the literal to hex (`0x40` instead of `64`); commit as `fix(oriterm_mux): rewrite N decimal-typed bit-flag literals to hex (clippy::decimal_bitwise_operands)`.
  - If genuine decimal arithmetic → `#[expect(clippy::decimal_bitwise_operands, reason="N is a count, not a flag")]`.
- Apply per-site.

### string_slice (3 sites), map_err_ignore (1 site), needless_pass_by_value (1 site)

- Per-site judgment per Section 03.4 protocol for string_slice; per-site `#[expect]` or rewrite for the others.

- [ ] Apply all judgment-class fixes; verify `cargo clippy -p oriterm_mux --all-targets -- -D warnings` exits 0.
- [ ] Commit: `style(oriterm_mux): per-site judgments for unchecked_time_subtraction × 8, decimal_bitwise_operands × 12, etc.`.

- [ ] **Subsection close-out (06.3)**: standard template.

---

## 06.4 Cross-target verification

- [ ] `cargo clippy -p oriterm_mux --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` exits 0
- [ ] `cargo clippy -p oriterm_mux --all-targets --target x86_64-pc-windows-gnu -- -D warnings` exits 0 — Windows-specific code paths (named pipe IPC, ConPTY) covered.

- [ ] **Subsection close-out (06.4)**: standard template.

---

## 06.R Third Party Review Findings

- None.

---

## 06.N Completion Checklist

- [ ] Both target cells exit 0 (`-D warnings`)
- [ ] `cargo test -p oriterm_mux` green; IO thread + double-buffer tests pass
- [ ] `cargo test --all` green (regression canary)
- [ ] All 14 judgment sites have committed verdicts (rewrite or `#[expect(reason=...)]`)
- [ ] BUG-07-NNN (oriterm_mux entry) remains `[ ]` (closure in Section 10)
- [ ] **Plan sync**: section 06 status → complete in section file + 00-overview.md + index.md
- [ ] `/tpr-review` passed (judgment cases warrant TPR especially for unchecked_time_subtraction in IO thread)
- [ ] `/impl-hygiene-review` passed (after TPR clean)
- [ ] `/improve-tooling` section-close sweep
- [ ] `/sync-claude` section-close doc sync
- [ ] **Repo hygiene check**

**Exit Criteria:** Both oriterm_mux target cells exit 0; IO thread + snapshot tests green; all 14 judgment sites committed; section frontmatter and overview/index reflect complete.
