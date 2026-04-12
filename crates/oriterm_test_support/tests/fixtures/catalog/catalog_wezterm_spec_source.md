---
schema_version: "0.1-provisional"
---

# Catalog Phase-2-J-Violation Fixture

Row that MUST be rejected by the Phase 2 Finding J anti-LEAK gate.
`Spec source` cites wezterm directly — this makes a peer implementation
a shadow authority, violating the authority ladder in
`plans/spec-conformance/00-overview.md §Authority Ladder`.

Expected: `SpecSourceCitesPeer` finding.

| ID | Spec source | Sequence | Description | Implementation | Apex layer | Test chain | Verification | De-facto ref | Notes |
|---|---|---|---|---|---|---|---|---|---|
| ECMA48-WEZTERM-LEAK | wezterm escape-sequences.md | `` `CSI ? 1 h` `` | Cursor Keys (bad cite) | `` `TermHandler::apply_decset` (`oriterm_core/src/term/handler/modes.rs`) `` | state-snapshot | parser:pending | stub | — | Phase 2 Finding J anti-LEAK violation. |
