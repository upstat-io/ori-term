---
section: "23"
title: "Cross-Stack Regression Sweep + Coverage Report CI"
canonical_spec_sources: []  # integration section — lint contract is the audit input
audit_input: "the lint contract itself — plans/spec-conformance/audits/README.md + coverage-baseline.toml + every per-section audit file"
last_walked: null
walked_by: null
---

# Top-Down Audit — Section 23: Cross-Stack Regression Sweep + Coverage Report CI (INTEGRATION SECTION)

## Audit input

This is an integration section. The audit input is NOT a control-sequence spec; it is the lint contract itself:

- `plans/spec-conformance/audits/README.md` — the audit-file schema + lint contract definition
- `plans/spec-conformance/coverage-baseline.toml` — the monotonicity baseline for verified row counts
- Every `plans/spec-conformance/audits/section-NN-top-down-inventory.md` file in scope (sections 09A + 11-26)

The `audits/` SSOT introduced by Section 09A (per `plans/spec-conformance/audits/README.md`) adapts to integration sections by treating the lint contract as the top-down enumerator. The completeness check is: the `--check audit-files` lint runs in CI and passes for every audit file in scope.

The lint enforces four properties per `audits/README.md §Lint contract`:
1. Every not-started section listed in `00-overview.md` Quick Reference has a corresponding audit file.
2. Every `mapped` row in any audit file resolves to a real catalog row in some `catalog/*.md` file.
3. Every audit file frontmatter parses; every row has all required columns; every `not-targeted` decision has a non-empty one-line rationale.
4. `last_walked` is present and parses as YYYY-MM-DD.

## Corpus-to-wiring mapping

| Entry (corpus identifier) | Source citation | Harness wiring (file:test) | Pass criterion |
|---|---|---|---|
| _**TODO: implementer populates this table when picking up Section 23. Every audit file in scope gets a row here. The harness wiring is the `--check audit-files` CI step. The pass criterion is: the lint passes for this file with no failures.**_ | | | |

## Verification

- [ ] `--check audit-files` mode is implemented in `crates/oriterm_test_support/src/bin/spec_coverage_report.rs` (per §09A.0).
- [ ] The lint is wired into the same CI lane as the existing `--check` modes in `.github/workflows/spec-conformance.yml`.
- [ ] CI fails on any audit-file lint failure (existence, mapping resolution, schema conformance, freshness parse).
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes locally with all committed audit files in scope.
- [ ] `last_walked` and `walked_by` set.
