---
section: "25"
title: "Real-App FULL-PASS Milestone"
canonical_spec_sources: []  # integration section — corpus manifest is the audit input
audit_input: "Section 22's harness corpus (vim, htop, btop, tmux, aerc, helix, ncmpcpp, less, nvim, etc.) + per-app session goldens"
last_walked: null
walked_by: null
---

# Top-Down Audit — Section 25: Real-App FULL-PASS Milestone (INTEGRATION SECTION)

## Audit input

This is an integration section. The audit input is NOT a control-sequence spec; it is a corpus manifest:

Section 22's harness corpus: every app in the milestone app matrix (vim, neovim, helix, htop, btop, tmux, aerc, ncmpcpp, less) with their committed session capture files and per-app golden snapshots. Session captures live at `crates/oriterm_test_support/tests/data/real_app_captures/<app>/<scenario>.cap`; snapshots live at `crates/oriterm_test_support/tests/references/real_app/<app>/<scenario>.snap`. Per-app scenarios are documented in `crates/oriterm_test_support/tests/data/real_app_captures/README.md`.

The `audits/` SSOT introduced by Section 09A (per `plans/spec-conformance/audits/README.md`) adapts to integration sections by treating the corpus manifest as the top-down enumerator. The completeness check is: every corpus entry has harness wiring + a per-entry pass criterion.

## Corpus-to-wiring mapping

| Entry (corpus identifier) | Source citation | Harness wiring (file:test) | Pass criterion |
|---|---|---|---|
| _**TODO: implementer populates this table when picking up Section 25. Every app × scenario pair in the milestone matrix gets a row here, with the corresponding harness wiring (test file + test name) and per-entry pass criterion (concrete correctness criterion — e.g., "final grid snapshot matches committed golden with 0-byte diff").**_ | | | |

## Verification

- [ ] Every row has a harness-wiring reference (file path + test name) that resolves to a real test.
- [ ] Every row has a pass criterion (concrete and testable).
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes (integration-section variant).
- [ ] `last_walked` and `walked_by` set.
