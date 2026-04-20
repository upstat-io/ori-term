---
section: "24"
title: "notcurses-demo FULL-PASS Milestone"
canonical_spec_sources: []  # integration section — corpus manifest is the audit input
audit_input: "Section 21's harness corpus (all 28 notcurses-demo scenes) + per-scene golden images"
last_walked: null
walked_by: null
---

# Top-Down Audit — Section 24: notcurses-demo FULL-PASS Milestone (INTEGRATION SECTION)

## Audit input

This is an integration section. The audit input is NOT a control-sequence spec; it is a corpus manifest:

Section 21's harness corpus: all 28 notcurses-demo scenes with their committed capture files and per-scene golden images. Scene letters in default order: `ixetunchdmbkywjgarvlsfqzo`. Scene captures live at `crates/oriterm_test_support/tests/data/notcurses_captures/<scene>.cap`; goldens live at `crates/oriterm_test_support/tests/references/notcurses_demo/<scene>.png`. Per-scene correctness criteria are in `plans/spec-conformance/notcurses-scene-criteria.md` (created by §24.1).

The `audits/` SSOT introduced by Section 09A (per `plans/spec-conformance/audits/README.md`) adapts to integration sections by treating the corpus manifest as the top-down enumerator. The completeness check is: every corpus entry has harness wiring + a per-entry pass criterion.

## Corpus-to-wiring mapping

| Entry (corpus identifier) | Source citation | Harness wiring (file:test) | Pass criterion |
|---|---|---|---|
| _**TODO: implementer populates this table when picking up Section 24. Every scene in the 28-scene corpus gets a row here, with the corresponding harness wiring (test file + test name) and per-entry pass criterion (concrete correctness criterion from notcurses-scene-criteria.md).**_ | | | |

## Verification

- [ ] Every row has a harness-wiring reference (file path + test name) that resolves to a real test.
- [ ] Every row has a pass criterion (concrete and testable — sourced from `notcurses-scene-criteria.md`).
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes (integration-section variant).
- [ ] `last_walked` and `walked_by` set.
