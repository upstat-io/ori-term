# Top-Down Spec Audit Artifacts

This directory is the **SSOT** for top-down enforcement of catalog completeness. Each not-started spec-conformance section commits a per-section audit file that walks its canonical spec source(s) row-by-row and maps every sequence to either a catalog row ID or an explicit `not-targeted` decision with rationale.

## Why this exists

The original `plans/spec-conformance/00-overview.md` mission criterion "Catalog complete" was supposed to enforce coverage. It didn't — the DECRQCRA gap (and the entire DEC private rectangular-ops family) shipped to production undetected because Section 01's catalog bootstrap was bottom-up (audit existing dispatch + add tack/teseq-discovered items) rather than top-down (walk the canonical spec source row-by-row). `Section 04.9 UncatalogedDetector` only catches sequences observed at harness time; sequences absent from both the catalog AND the test corpus are invisible.

Per-section audit files close the loop. Section 09A introduces this directory and the audit-file lint. Every section's verbiage rewrite (sections 11-26) requires a committed audit file before the section can close.

## Artifact format

Each audit file is a single markdown document at `audits/section-NN-top-down-inventory.md`. Required structure:

```markdown
---
section: "NN"
title: "<Section title — copy verbatim from section file frontmatter>"
canonical_spec_sources:
  - "<source 1 — full citation including version/edition>"
  - "<source 2 — if applicable>"
last_walked: YYYY-MM-DD
walked_by: "<implementer handle>"
---

# Top-Down Spec Audit — Section NN: <title>

## Canonical spec source(s)

<short paragraph naming the authoritative spec source(s) for this stack and explaining why these are the right enumerators (e.g., "ISO 2022 is the row-by-row enumerator for charset designations because every ISO-2022 G0/G1/G2/G3 designation maps to a catalog row").>

## Sequence-to-catalog mapping

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| `<sequence>` | `<spec-source §section>` | `<STACK-MNEMONIC>` | mapped |
| `<sequence>` | `<spec-source §section>` | — | not-targeted: `<one-line rationale>` |
| ... | ... | ... | ... |

## Decisions

For every row with `not-targeted` in the Decision column, this section explains:
- Why the sequence is intentionally excluded from ori_term's coverage
- What downstream rationale supports the exclusion (e.g., "deprecated in VT5xx; no real-world consumer detected in our capture corpus")
- Whether the exclusion is permanent or revisitable (and if revisitable, what would change the decision)

## Verification

- [ ] Every row with `Decision: mapped` resolves to a real catalog row that exists in `plans/spec-conformance/catalog/` with the cited row ID.
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes with this audit file in scope.
- [ ] No row in the canonical spec source is missing from this table (top-down completeness).
- [ ] `last_walked` date is within the section's implementation window (an audit file walked >6 months before section close MUST be re-walked).
```

## Lint contract

`spec-coverage-report --check audit-files` enforces:

1. **Existence**: every section listed in `plans/spec-conformance/00-overview.md` Quick Reference whose status is currently `in-progress` (not `not-started` and not `complete`) has a corresponding `audits/section-NN-top-down-inventory.md` file (sections 11-26 + 09A — when each is picked up for implementation). The existence check exempts sections in `not-started` status because their audit file is created at §NN.0 execution time; the lint exempts sections in `complete` status because their audit file is permanently committed. Integration sections (21, 22, 24, 25) are exempted from the spec-source completeness check via the per-section `canonical_spec_sources: []` empty list with a `# Integration section — corpus manifest is the audit input` body comment, but they still get an existence check (their audit input is the corpus manifest).
2. **Mapping resolution**: every row with `Decision: mapped` cites a catalog row ID that exists in some `catalog/*.md` file. A mapping to a non-existent row ID fails the lint.
3. **Schema conformance**: every audit file frontmatter parses; every row has all 4 columns; every `not-targeted` decision has a one-line rationale (the cell after the colon must be non-empty).
4. **Freshness**: `last_walked` date is present and parses as YYYY-MM-DD. CI does not gate on freshness staleness — that is a `/review-bugs` triage check.

`--check audit-files` is wired into the same CI lane as the other coverage gates (Section 23). The lint runs as a Rust binary inside `oriterm_test_support`.

## Workflow when adding a new section

1. Create `audits/section-NN-top-down-inventory.md` with the frontmatter populated.
2. Identify the canonical spec source(s) for the section's stack (cross-reference the section's `inspired_by:` array).
3. Walk the spec source row-by-row. For every sequence the spec defines, add a row to the mapping table.
4. For each row, decide: `mapped` (citing the catalog row ID) or `not-targeted` (with rationale).
5. Open new catalog rows in `plans/spec-conformance/catalog/<stack>.md` for any sequence that should be `mapped` but isn't in the catalog yet. Use the canonical row schema in `plans/spec-conformance/00-overview.md §Catalog Row Schema`.
6. Run `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` until clean.
7. Commit the audit file alongside the section's verification work.

## Workflow when re-walking an existing audit

1. Update `last_walked` and `walked_by` in the frontmatter.
2. Re-walk the canonical spec source. Add rows for any sequences that have appeared since the last walk (spec revisions, new RFCs, vendor extensions).
3. Re-evaluate every `not-targeted` decision — has the rationale aged out? Has a real-world consumer surfaced for a previously-excluded sequence?
4. Run the lint until clean.

## Files in this directory

- `README.md` — this file (artifact format + lint contract).
- `section-09a-top-down-inventory.md` — Section 09A's own audit (DEC Private CSI extensions). _Not yet created — will be committed when Section 09A's §09A.0 executes; the lint's existence-check exempts not-started sections so the lint does not fail on this absence before §09A.0 runs._
- `section-NN-top-down-inventory.md` — one per not-started section (11-26). Created by each section's verbiage rewrite as a stub; populated by the section's implementer when the section is picked up.
