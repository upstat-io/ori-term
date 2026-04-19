---
section: "17"
title: "Kitty Keyboard Protocol"
canonical_spec_sources:
  - "sw.kovidgoyal.net/kitty/keyboard-protocol/ — primary canonical source (kitty source is the authoritative protocol definition; covers mode push/pop stack, all 5 disambiguation flags, key encoding format for every key class)"
  - "kitty source keys.py (~/.local/share/ori_term/reference_repos/console_repos/kitty/kitty/keys.py) — encoding logic cross-reference"
last_walked: null
walked_by: null
---

# Top-Down Spec Audit — Section 17: Kitty Keyboard Protocol

## Canonical spec source(s)

sw.kovidgoyal.net/kitty/keyboard-protocol/ is the authoritative top-down enumerator for the kitty keyboard protocol. It defines every aspect of the protocol: the mode-stack CSI sequences (`CSI > u` push, `CSI < u` pop, `CSI = u` set, `CSI ? u` query), all 5 disambiguation flags (DISAMBIGUATE_ESC_CODES, REPORT_EVENT_TYPES, REPORT_ALTERNATE_KEYS, REPORT_ALL_KEYS_AS_ESC, REPORT_ASSOCIATED_TEXT), the key encoding format for every key class (printable, functional, modifier-only), event types (press, repeat, release), and the full modifier encoding table. kitty's `keys.py` is the secondary cross-reference for implementation of the encoding logic. Every sequence and mode variant the spec defines maps to a catalog row or carries an explicit `not-targeted` decision.

## Sequence-to-catalog mapping

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| _**TODO: implementer populates this table when picking up Section 17. Walk sw.kovidgoyal.net/kitty/keyboard-protocol/ top-down. Every CSI sequence, mode flag, key class, event type, and modifier encoding variant gets a row here.**_ | | | |

## Decisions

_**TODO: implementer documents every `not-targeted` decision here with rationale. Kitty keyboard protocol features intentionally excluded from ori_term's coverage need a written justification.**_

## Verification

- [ ] Every row with `Decision: mapped` resolves to a real catalog row that exists in `plans/spec-conformance/catalog/` with the cited row ID.
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes with this audit file in scope.
- [ ] No row in the canonical spec source is missing from this table (top-down completeness).
- [ ] `last_walked` date is set; `walked_by` is set.
