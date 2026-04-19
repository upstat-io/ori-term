---
section: "16"
title: "Mouse Protocols"
canonical_spec_sources:
  - "xterm ctlseqs.txt §Mouse Tracking — numbered protocols X10/9, Normal/1000, Locator/1001, Button-event/1002, Any-event/1003, Focus/1004, UTF-8/1005, SGR/1006, URXVT/1015, SGR pixels/1016"
  - "URXVT docs — URXVT mouse protocol (1015) extension"
  - "xterm ctlseqs.txt §DEC Locator — DECEFR, DECELR, DECSLE, DECRQLP locator extension sequences"
last_walked: null
walked_by: null
---

# Top-Down Spec Audit — Section 16: Mouse Protocols

## Canonical spec source(s)

xterm `ctlseqs.txt` is the authoritative enumerator for the numbered mouse protocols (X10/9, Normal/1000, Locator/1001, Button-event/1002, Any-event/1003, Focus/1004, UTF-8/1005, SGR/1006, URXVT/1015, SGR pixels/1016). It also defines the DEC locator extension sequences (DECEFR, DECELR, DECSLE, DECRQLP) that comprise the full locator protocol surface. URXVT docs are the secondary authority for the URXVT 1015 extension. Every mouse mode, encoding format, and locator sequence in these sources maps to a catalog row or carries an explicit `not-targeted` decision.

Note on DEC locator extensions: DECEFR, DECELR, DECSLE, and DECRQLP were discovered during Section 09A's top-down audit of the DEC private CSI extension space. They were explicitly routed to Section 16's ownership (per §09A.12) because they are mouse/locator protocol extensions, not rectangle or presentation ops. The 4 rows below are pre-populated by Section 09A's §09A.10 verbiage pass so the implementer does not have to re-discover them.

## Sequence-to-catalog mapping

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| `CSI Pt;Pl;Pb;Pr ' w` | xterm ctlseqs.txt §DECEFR | MOUSE-DECEFR | mapped (NEW row to be added to `catalog/mouse.md` by Section 16's verification work — this row was discovered during Section 09A's research and explicitly assigned to Section 16's ownership per §09A.12) |
| `CSI Ps;Pu ' z` | xterm ctlseqs.txt §DECELR | MOUSE-DECELR | mapped (NEW row — same provenance) |
| `CSI Pm ' {` | xterm ctlseqs.txt §DECSLE | MOUSE-DECSLE | mapped (NEW row — same provenance) |
| `CSI Ps ' \|` | xterm ctlseqs.txt §DECRQLP | MOUSE-DECRQLP | mapped (NEW row — same provenance) |
| _**TODO: implementer populates remaining rows when picking up Section 16. Walk xterm ctlseqs.txt §Mouse Tracking and §DEC Locator row-by-row. Every numbered mouse protocol, encoding format, and mode sequence gets a row here.**_ | | | |

## Decisions

_**TODO: implementer documents every `not-targeted` decision here with rationale. Mouse sequences intentionally excluded from ori_term's coverage need a written justification.**_

## Verification

- [ ] Every row with `Decision: mapped` resolves to a real catalog row that exists in `plans/spec-conformance/catalog/` with the cited row ID.
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes with this audit file in scope.
- [ ] No row in the canonical spec source is missing from this table (top-down completeness).
- [ ] `catalog/mouse.md` has been updated with rows MOUSE-DECEFR, MOUSE-DECELR, MOUSE-DECSLE, MOUSE-DECRQLP before these rows are marked `mapped`.
- [ ] `last_walked` date is set; `walked_by` is set.
