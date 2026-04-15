# Legacy Tack-Conformance Mapping

> **Purpose.** This file is the traceability map linking spec-conformance
> catalog row IDs to legacy `plans/tack-conformance/` section IDs. As
> `plans/spec-conformance/section-08-ecma-48-baseline.md` converts existing
> tack scenarios into spec-conformance verification chains, each converted
> catalog row lands here with a reference to its originating tack section.
>
> **Canonical policy.** The absorption strategy this file implements lives at
> [plans/spec-conformance/00-overview.md §Tack Absorption Strategy](../00-overview.md#tack-absorption-strategy-delivered-by-section-02).
> This file is an *artifact* of that policy, not a restatement of it.
>
> **Maintenance.**
> - `plans/spec-conformance/section-02-tack-absorption.md` (this section) creates
>   the file and seeds the tack-09 → spec-23 absorption row.
> - `plans/spec-conformance/section-08-ecma-48-baseline.md` appends rows as it
>   converts tack test-menu + tools-menu scenarios into spec_chain tests.
> - `plans/spec-conformance/section-17-kitty-keyboard.md` appends rows as it
>   verifies the kf1–kf63 / cursor / editing / modified-key cross-check
>   infrastructure already landed under tack-08.
> - `plans/spec-conformance/section-23-cross-stack-regression-sweep.md`
>   consumes this file to verify absorption completeness.
>
> **Scope note.** The filename (`_legacy-tack-mapping.md`) is scoped broadly
> to the whole absorption, not just section 08. Sections 08, 17, and 23 all
> append rows here as absorption work lands.

## Catalog row → tack section mapping

| Catalog row ID / handoff                    | Legacy tack section                                                      | Conversion status   |
|---                                          |---                                                                       |---                  |
| *(section 02 handoff — not a catalog row)*  | `plans/tack-conformance/section-09-verification.md`                      | `absorbed-by-spec-23` |
| `ECMA48-C0-BEL`                             | `plans/tack-conformance/section-05-test-menu-scenarios.md` (`acs`/`graphic_rendition` — combined `(bel)` probe on tack v1.08) | `converted` |
| `ECMA48-CSI-CUP`                            | `plans/tack-conformance/section-05-test-menu-scenarios.md` (`cursor_movement` — `clear` cap = `\E[H\E[2J`)                    | `converted` |
| `ECMA48-CSI-ED`                             | `plans/tack-conformance/section-05-test-menu-scenarios.md` (`cursor_movement` — `clear` cap = `\E[H\E[2J`)                    | `converted` |
| `DEC-DECAWM`                                | `plans/tack-conformance/section-05-test-menu-scenarios.md` (`modes` — `am` cap, internally exercised by tack)                 | `converted` |
| `DEC-DECREVWRAP`                            | `plans/tack-conformance/section-05-test-menu-scenarios.md` (`modes` — `bw` cap, internally exercised by tack)                 | `converted` |
| `ECMA48-C0-SO`                              | `plans/tack-conformance/section-06-tools-menu-scenarios.md` (`character_sets` — SO/SI bank switch used by tack's DEC-graphics preview)                           | `converted` |
| `ECMA48-C0-SI`                              | `plans/tack-conformance/section-06-tools-menu-scenarios.md` (`character_sets` — SO/SI matrix pair)                                                                | `converted` |
| `ECMA48-ESC-B`                              | `plans/tack-conformance/section-06-tools-menu-scenarios.md` (`character_sets` — ASCII round-trip after DEC-graphics designation)                                  | `converted` |
| `ECMA48-ESC-0`                              | `plans/tack-conformance/section-06-tools-menu-scenarios.md` (`character_sets` — DEC Special Graphics designation to G0/G1 + preview render of `q`→`─`)            | `converted` |
| `ECMA48-CSI-DA2`                            | `plans/tack-conformance/section-06-tools-menu-scenarios.md` (`status_reports_inventory` — `da2` sub-test)                                                         | `converted` |
| `ECMA48-CSI-DA3`                            | `plans/tack-conformance/section-06-tools-menu-scenarios.md` (`status_reports_inventory` — `da3` sub-test)                                                         | `converted` |
| `ECMA48-CSI-DSR-5`                          | `plans/tack-conformance/section-06-tools-menu-scenarios.md` (`status_reports_inventory` — `dsr_status` sub-test)                                                  | `converted` |
| `ECMA48-CSI-DSR-6`                          | `plans/tack-conformance/section-06-tools-menu-scenarios.md` (`status_reports_inventory` — `dsr_cpr` sub-test)                                                     | `converted` |
| `ECMA48-SGR-0`                              | `plans/tack-conformance/section-06-tools-menu-scenarios.md` (`sgr_modes` — Mode 0 reset label on tack's 80-mode grid)                                             | `converted` |
| `ECMA48-SGR-1`                              | `plans/tack-conformance/section-06-tools-menu-scenarios.md` (`sgr_modes` — Mode 1 bold label on tack's 80-mode grid)                                              | `converted` |
| `ECMA48-SGR-4`                              | `plans/tack-conformance/section-06-tools-menu-scenarios.md` (`sgr_modes` — Mode 4 underline label on tack's 80-mode grid)                                         | `converted` |
| `ECMA48-SGR-7`                              | `plans/tack-conformance/section-06-tools-menu-scenarios.md` (`sgr_modes` — Mode 7 reverse/inverse label on tack's 80-mode grid)                                   | `converted` |
| `ECMA48-C0-ENQ`                             | `plans/tack-conformance/section-06-tools-menu-scenarios.md` (`enq_ack` — blocked on BUG-08-6; ENQ dispatch missing in `Performer::execute`)                       | `pending`   |

### Status vocabulary

- `converted` — tack scenario's PTY fixture bytes now drive a spec_chain test at the catalog row listed in the first column; both tests co-exist for regression coverage.
- `absorbed-by-spec-23` — the tack-side work is closed in place and its remaining scope is owned by spec-conformance Section 23 (no new spec_chain test is written; the absorption is structural, not behavioral).
- `absorbed-by-spec-17` — the tack-side work is closed in place and its remaining scope is owned by spec-conformance Section 17 (kitty keyboard + modifyOtherKeys + Win32 Input encoders extending the tack-08 infrastructure).
- `pending` — tack scenario exists but has not yet been converted to a spec_chain test.
