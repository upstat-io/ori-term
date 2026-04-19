---
section: "21"
title: "notcurses-demo Harness + Scene Matrix + qrcode smoke"
canonical_spec_sources: []  # integration section — corpus manifest is the audit input
audit_input: "notcurses scene corpus manifest (28 scenes — see notcurses-demo scene matrix at ~/projects/reference_repos/console_repos/notcurses/)"
last_walked: null
walked_by: null
---

# Top-Down Audit — Section 21: notcurses-demo Harness + Scene Matrix + qrcode smoke (INTEGRATION SECTION)

## Audit input

This is an integration section. The audit input is NOT a control-sequence spec; it is a corpus manifest:

The 28 notcurses-demo scenes — see the notcurses-demo scene matrix at `~/projects/reference_repos/console_repos/notcurses/`. Scene letters in default order: `ixetunchdmbkywjgarvlsfqzo`. Source files live at `~/projects/reference_repos/console_repos/notcurses/src/demo/<scene>.c`.

The `audits/` SSOT introduced by Section 09A (per `plans/spec-conformance/audits/README.md`) adapts to integration sections by treating the corpus manifest as the top-down enumerator. The completeness check is: every corpus entry has harness wiring + a per-entry pass criterion.

## Corpus-to-wiring mapping

| Entry (corpus identifier) | Source citation | Harness wiring (file:test) | Pass criterion |
|---|---|---|---|
| _**TODO: implementer populates this table when picking up Section 21. Every entry in the corpus manifest gets a row here, with the corresponding harness wiring and per-entry pass criterion.**_ | | | |

## Verification

- [ ] Every row has a harness-wiring reference (file path + test name) that resolves to a real test.
- [ ] Every row has a pass criterion (concrete and testable).
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes (integration-section variant).
- [ ] `last_walked` and `walked_by` set.
