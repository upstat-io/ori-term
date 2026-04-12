---
schema_version: "0.1-provisional"
---

# Catalog Verified-in-Bootstrap Fixture

Row that MUST be rejected by `catalog_coverage_check --check --bootstrap-mode`
(Phase 2 Finding L negative criterion). Used for the deliberate-injection
walkthrough in `plans/spec-conformance/section-01-catalog-bootstrap.md §01.3.d`.

Expected: running `check --bootstrap-mode` against a catalog directory
containing only this row produces exit code 1 with a
`VerifiedInBootstrap` finding.

| ID | Spec source | Sequence | Description | Implementation | Apex layer | Test chain | Verification | De-facto ref | Notes |
|---|---|---|---|---|---|---|---|---|---|
| ECMA48-VERIFIED | ECMA-48 §8.3.21 | `` `CSI Ps;Ps H` `` | Cursor Position | `` `TermHandler::goto` (`oriterm_core/src/term/handler/mod.rs`) `` | state-snapshot | parser:pass dispatch:pass state:pass | verified | — | Deliberately verified for --bootstrap-mode rejection walkthrough. |
