---
reroute: true
name: "Clippy Gate"
full_name: "Clippy Gate Hardening"
status: active
order: 1
---

# Clippy Gate Hardening Index

> **Maintenance Notice:** Update this index when adding/modifying sections.
> **Supersedes:** `BUG-07-005`, `BUG-07-006`, `BUG-07-010`, `BUG-07-012` (cluster) plus 3 new tracker entries filed in Section 01.

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

## Keyword Clusters by Section

### Section 01: Baseline + Bug Filing
**File:** `section-01-baseline-and-bug-filing.md` | **Status:** Not Started

```
baseline, inventory, bug filing, bug-tracker, BUG-07
oriterm_mux, oriterm_ipc, oriterm gaps, three new entries
section-07-ci-build.md, plans/bug-tracker
clippy lint count per crate, message-format=json
M/S/J classification, mechanical structural judgment
```

---

### Section 02: oriterm_ipc Cleanup
**File:** `section-02-oriterm-ipc-cleanup.md` | **Status:** Not Started

```
oriterm_ipc, ipc transport, redundant_clone, manual_assert, doc_markdown
8 violations, smallest crate first, dependency-order
oriterm_ipc/Cargo.toml [lints] divergence, unsafe_code allow
workspace = true vs explicit lints, SSOT
```

---

### Section 03: oriterm_core Cleanup
**File:** `section-03-oriterm-core-cleanup.md` | **Status:** Not Started

```
oriterm_core, terminal emulation library, 485 violations
doc_markdown 301, field_reassign_with_default 42, needless_raw_strings 29
float_cmp 21, redundant_closure_for_method_calls 14, string_slice 14
cargo clippy --fix --all-targets, manual_let_else, drop-timing
oriterm_core/src/term/tests.rs, oriterm_core/tests/{vttest,teseq,tack}/
```

---

### Section 04: oriterm_test_support Cleanup
**File:** `section-04-oriterm-test-support-cleanup.md` | **Status:** Not Started

```
oriterm_test_support, dev-dependency, workspace test helpers
27 violations, doc_markdown 7, format_push_string 4
crates/oriterm_test_support/src/{session,tack_framework}/
into_iter_on_single_item, items_after_statements, case_sensitive_file_extension
BUG-07-012 superseded
```

---

### Section 05: oriterm_ui Cleanup
**File:** `section-05-oriterm-ui-cleanup.md` | **Status:** Not Started

```
oriterm_ui, UI framework, 761 violations, 145 structural + 616 float_cmp
float_cmp 616 in TEST CODE only, 50 oriterm_ui test files
module-level #![expect(clippy::float_cmp, reason="...")] per test file
production float_cmp uses #[expect] per-site (transform2d.rs:130 precedent)
oriterm_ui/src/animation/tests.rs (56), color/tests.rs (56), geometry/tests.rs (66)
oriterm_ui/src/testing/ feature-gated, --features testing required for clippy
unreadable_literal 27, doc_markdown 45, items_after_statements 8
BUG-07-006 superseded
```

---

### Section 06: oriterm_mux Cleanup
**File:** `section-06-oriterm-mux-cleanup.md` | **Status:** Not Started

```
oriterm_mux, pane server, 192 violations
doc_markdown 85, used_underscore_binding 37, decimal_bitwise_operands 12
default_trait_access 11, unchecked_time_subtraction 8 (judgment)
manual_assert 5, redundant_clone 4, string_slice 3
unfiled bug → BUG-07-NNN filed in Section 01, closed here
```

---

### Section 07: oriterm Cleanup
**File:** `section-07-oriterm-cleanup.md` | **Status:** Not Started

```
oriterm, application shell, 6-9 violations
doc_markdown, manual_assert, many_single_char_names
3 float_cmp surfaced via oriterm_ui/testing dev-dep (not new production code)
oriterm/src/scheme/builtin/*.rs #![allow(unreadable_literal)] → upgrade to #![expect(reason=...)]
unfiled bug → BUG-07-NNN filed in Section 01, closed here
```

---

### Section 08: Feature Matrix Verification
**File:** `section-08-feature-matrix-verification.md` | **Status:** Not Started

```
feature matrix, --no-default-features, --features testing, --features gpu-tests, --features profile
oriterm_core image-protocol default feature, oriterm_ui testing feature
oriterm gpu-tests + profile features
host x86_64-unknown-linux-gnu vs x86_64-pc-windows-gnu
per-crate combos × cross-target verification
```

---

### Section 09: Gate Flip + Meta-Test
**File:** `section-09-gate-flip-and-meta-test.md` | **Status:** Not Started

```
gate flip, ./clippy-all.sh, .github/workflows/ci.yml, lefthook.yml
--all-targets, source-text meta-test, regression test
oriterm/tests/architecture.rs:238-294 source-text pin precedent
oriterm/tests/architecture.rs:330-342 removal pin precedent
CI 15-min timeout extension to 25 min if needed
ci.yml clippy job line 40-63, clippy-windows-cross job 65-83
lefthook.yml clippy: pre-commit hook
SSOT for gate flags across three files
```

---

### Section 10: Cluster + New Bug Closure
**File:** `section-10-bug-closure.md` | **Status:** Not Started

```
bug closure, bidirectional supersede, BUG-07-005, BUG-07-006, BUG-07-010, BUG-07-012
plus 3 new BUG-07-NNN filed in Section 01
plans/bug-tracker/section-07-ci-build.md, plans/bug-tracker/00-overview.md
section-07 open count decrement by 7
plan status: in-progress → complete; index.md status: active → resolved
```

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | Baseline + Bug Filing | `section-01-baseline-and-bug-filing.md` |
| 02 | oriterm_ipc Cleanup | `section-02-oriterm-ipc-cleanup.md` |
| 03 | oriterm_core Cleanup | `section-03-oriterm-core-cleanup.md` |
| 04 | oriterm_test_support Cleanup | `section-04-oriterm-test-support-cleanup.md` |
| 05 | oriterm_ui Cleanup | `section-05-oriterm-ui-cleanup.md` |
| 06 | oriterm_mux Cleanup | `section-06-oriterm-mux-cleanup.md` |
| 07 | oriterm Cleanup | `section-07-oriterm-cleanup.md` |
| 08 | Feature Matrix Verification | `section-08-feature-matrix-verification.md` |
| 09 | Gate Flip + Meta-Test | `section-09-gate-flip-and-meta-test.md` |
| 10 | Cluster + New Bug Closure | `section-10-bug-closure.md` |
