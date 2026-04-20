---
section: "09A"
title: "DEC Private CSI Extensions (rect ops + presentation + audits/ SSOT)"
canonical_spec_sources:
  - "xterm ctlseqs.txt §DEC Private — CSI sequences with intermediates `$`, `*`, `#`, `'`"
  - "xterm ctlseqs.txt §ESC 6 / §ESC 9 — DECBI / DECFI back/forward index"
  - "DEC STD 070 §6 / VT420 Programming Reference Manual — original DEC spec for rectangular area ops and presentation state queries"
last_walked: 2026-04-19
walked_by: "elucidsoft"
---

# Top-Down Spec Audit — Section 09A: DEC Private CSI Extensions

## Canonical spec source(s)

xterm `ctlseqs.txt` is the authoritative enumerator for every DEC private CSI sequence in the `$`, `*`, `#`, and `'` intermediate space plus the ESC-6 / ESC-9 back/forward index sequences. DEC STD 070 / VT420 PRM backs the original rectangular-area family (DECCARA / DECRARA / DECCRA / DECFRA / DECERA / DECSERA / DECSACE) and the presentation-state queries (DECRQPSR / DECRQUPSS / DECRQDE / DECSCL / DECSCA / DECSASD / DECSSDT). xterm patch-336 backs the `#`-intermediate extensions (XTCHECKSUM, XTREPORTSGR) and the DECRQCRA checksum semantics.

Every sequence the canonical sources define inside the scoped intermediates maps to a row in `catalog/dec-rectangle-ops.md`, `catalog/dec-presentation.md`, `catalog/mouse.md` (for locator extensions routed to Section 16), or `catalog/ecma-48.md` (DECRQM, which pre-existed Section 09A). Sequences not targeted by ori_term carry an explicit `not-targeted` decision with rationale.

## Sequence-to-catalog mapping

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| `CSI Ps $ p` | xterm ctlseqs.txt §DECRQM ANSI | `ECMA48-CSI-DECRQM-ANSI` | mapped |
| `CSI ? Ps $ p` | xterm ctlseqs.txt §DECRQM private | `ECMA48-CSI-DECRQM-PRIV` | mapped |
| `CSI Ps $ w` | xterm ctlseqs.txt §DECRQPSR | `DECPRES-DECRQPSR` | mapped |
| `CSI Pt;Pl;Pb;Pr;Pm $ r` | xterm ctlseqs.txt §DECCARA | `DECRECT-DECCARA` | mapped |
| `CSI Pt;Pl;Pb;Pr;Pm $ t` | xterm ctlseqs.txt §DECRARA | `DECRECT-DECRARA` | mapped |
| `CSI Pt;Pl;Pb;Pr;Pp;Pt;Pl;Pp $ v` | xterm ctlseqs.txt §DECCRA | `DECRECT-DECCRA` | mapped |
| `CSI Pc;Pt;Pl;Pb;Pr $ x` | xterm ctlseqs.txt §DECFRA | `DECRECT-DECFRA` | mapped |
| `CSI Pt;Pl;Pb;Pr $ z` | xterm ctlseqs.txt §DECERA | `DECRECT-DECERA` | mapped |
| `CSI Pt;Pl;Pb;Pr $ {` | xterm ctlseqs.txt §DECSERA | `DECRECT-DECSERA` | mapped |
| `CSI Ps $ \|` | xterm ctlseqs.txt §DECSCPP | — | not-targeted: DECSCPP (Select Columns Per Page) is superseded by DECLRMM (mode 69) which ori_term targets directly; no real-world consumer in our capture corpus requests the deprecated DEC-VT420 column-count op. Revisitable if a vintage-terminal compatibility scenario surfaces. |
| `CSI Ps $ }` | xterm ctlseqs.txt §DECSASD | `DECPRES-DECSASD` | mapped |
| `CSI Ps $ ~` | xterm ctlseqs.txt §DECSSDT | `DECPRES-DECSSDT` | mapped |
| `DCS $ q Pt ST` | xterm ctlseqs.txt §DECRQSS | `DECPRES-DECRQSS` | mapped |
| `DCS Ps $ t Pt ST` | xterm ctlseqs.txt §DECRSPS | `DECPRES-DECRSPS` | mapped |
| `CSI Ps * x` | xterm ctlseqs.txt §DECSACE | `DECRECT-DECSACE` | mapped |
| `CSI Pi;Pg;Pt;Pl;Pb;Pr * y` | xterm ctlseqs.txt §DECRQCRA | `DECRECT-DECRQCRA` | mapped |
| `CSI Ps * \|` | xterm ctlseqs.txt §DECSNLS | — | not-targeted: DECSNLS (Select Number of Lines per Page) is a DEC VT420 paged-display control; modern PTY-based terminals do not maintain a paged scrollback model where line-count negotiation applies. No real-world consumer detected. Permanently excluded. |
| `CSI Ps;Ps * r` | xterm ctlseqs.txt §DECSCS | — | not-targeted: DECSCS (Select Communication Speed) negotiates host serial baud rate; modern PTY-based terminals run at pipe-bandwidth with no baud concept. Permanently excluded. |
| `CSI Ps # y` | xterm ctlseqs.txt §XTCHECKSUM | `DECRECT-XTCHECKSUM` | mapped |
| `CSI Pt;Pl;Pb;Pr # \|` | xterm ctlseqs.txt §XTREPORTSGR | `DECRECT-XTREPORTSGR` | mapped |
| `CSI Ps # {` | xterm ctlseqs.txt §XTPUSHSGR | — | not-targeted: xterm SGR stack push is rarely exercised outside xterm itself; deferred until a concrete consumer surfaces (revisitable — condition: a real-app E2E capture cites XTPUSHSGR/XTPOPSGR, or Section 22 surfaces it). |
| `CSI Ps # }` | xterm ctlseqs.txt §XTPOPSGR | — | not-targeted: companion to XTPUSHSGR; same rationale. Revisitable with XTPUSHSGR. |
| `CSI Ps # p` | xterm ctlseqs.txt §XTPUSHCOLORS | — | not-targeted: xterm color-stack push; defer with XTPUSHSGR family. Revisitable if a color-stack consumer surfaces. |
| `CSI Ps # q` | xterm ctlseqs.txt §XTPOPCOLORS | — | not-targeted: companion to XTPUSHCOLORS; same rationale. |
| `CSI Ps # R` | xterm ctlseqs.txt §XTREPORTCOLORS | — | not-targeted: xterm color-stack report; defer with XTPUSHCOLORS family. |
| `CSI Pt;Pl;Pb;Pr ' w` | xterm ctlseqs.txt §DECEFR | `MOUSE-DECEFR` | mapped |
| `CSI Ps;Pu ' z` | xterm ctlseqs.txt §DECELR | `MOUSE-DECELR` | mapped |
| `CSI Pm ' {` | xterm ctlseqs.txt §DECSLE | `MOUSE-DECSLE` | mapped |
| `CSI Ps ' \|` | xterm ctlseqs.txt §DECRQLP | `MOUSE-DECRQLP` | mapped |
| `CSI Ps ' }` | xterm ctlseqs.txt §DECIC | `DECPRES-DECIC` | mapped |
| `CSI Ps ' ~` | xterm ctlseqs.txt §DECDC | `DECPRES-DECDC` | mapped |
| `ESC 6` | xterm ctlseqs.txt §DECBI | `DECPRES-DECBI` | mapped |
| `ESC 9` | xterm ctlseqs.txt §DECFI | `DECPRES-DECFI` | mapped |
| `DCS Pt ST` (DECCIR subreport) | DEC STD 070 §6.3.2 | — | not-targeted: DECCIR is the REPLY payload that DECRQPSR (Ps=1) emits — it is not itself a request sequence the terminal dispatches. The request side is DECRQPSR (mapped above). Permanently excluded as a dispatch target. |

## Decisions

Every `not-targeted` row above documents its rationale inline. Summary of exclusion categories:

- **Deprecated DEC VT420 page controls** (DECSCPP, DECSNLS, DECSCS) — the underlying page-count / baud-rate models do not apply to modern PTY-backed terminals. Permanent exclusions.
- **xterm SGR/color stack extensions** (XTPUSHSGR, XTPOPSGR, XTPUSHCOLORS, XTPOPCOLORS, XTREPORTCOLORS) — rarely exercised outside xterm itself. Revisitable if a real-app or notcurses-demo scene surfaces a consumer.
- **Reply-only payloads** (DECCIR) — these are responses to request sequences, not dispatch targets. The request sides are mapped; the reply shape is a serialization concern documented alongside each request handler.

## Verification

- [x] Every row with `Decision: mapped` resolves to a real catalog row that exists in `plans/spec-conformance/catalog/` with the cited row ID.
- [x] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes with this audit file in scope.
- [x] No row in the canonical spec source is missing from this table (top-down completeness — every DEC private `$/*/#/'` CSI intermediate plus ESC 6/9 is enumerated; no sequence is silently skipped).
- [x] `last_walked` date is within the section's implementation window.
