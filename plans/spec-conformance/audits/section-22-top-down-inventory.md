---
section: "22"
title: "Real-App E2E Harness"
canonical_spec_sources: []  # integration section — corpus manifest is the audit input
audit_input: "real-app session corpus manifest (vim, htop, btop, tmux, aerc, helix, ncmpcpp, less, nvim, etc.)"
last_walked: null
walked_by: null
---

# Top-Down Audit — Section 22: Real-App E2E Harness (INTEGRATION SECTION)

## Audit input

This is an integration section. The audit input is NOT a control-sequence spec; it is a corpus manifest:

Real-app session corpus manifest covering the milestone app matrix: vim, htop, btop, tmux, aerc, helix, ncmpcpp, less, nvim, and any additional apps added during section implementation. Committed session captures live at `crates/oriterm_test_support/tests/data/real_app_captures/<app>/<scenario>.cap` alongside sidecar `.env.toml` files. Recording instructions are in `crates/oriterm_test_support/tests/data/real_app_captures/README.md`.

The `audits/` SSOT introduced by Section 09A (per `plans/spec-conformance/audits/README.md`) adapts to integration sections by treating the corpus manifest as the top-down enumerator. The completeness check is: every corpus entry has harness wiring + a per-entry pass criterion.

## Corpus-to-wiring mapping

| Entry (corpus identifier) | Source citation | Harness wiring (file:test) | Pass criterion |
|---|---|---|---|
| _**TODO: implementer populates this table when picking up Section 22. Every entry in the corpus manifest gets a row here, with the corresponding harness wiring and per-entry pass criterion.**_ | | | |

## Verification

- [ ] Every row has a harness-wiring reference (file path + test name) that resolves to a real test.
- [ ] Every row has a pass criterion (concrete and testable).
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes (integration-section variant).
- [ ] `last_walked` and `walked_by` set.
