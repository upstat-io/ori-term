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
| *(section 08 populates as it goes)*         |                                                                          |                     |

### Status vocabulary

- `converted` — tack scenario's PTY fixture bytes now drive a spec_chain test at the catalog row listed in the first column; both tests co-exist for regression coverage.
- `absorbed-by-spec-23` — the tack-side work is closed in place and its remaining scope is owned by spec-conformance Section 23 (no new spec_chain test is written; the absorption is structural, not behavioral).
- `absorbed-by-spec-17` — the tack-side work is closed in place and its remaining scope is owned by spec-conformance Section 17 (kitty keyboard + modifyOtherKeys + Win32 Input encoders extending the tack-08 infrastructure).
- `pending` — tack scenario exists but has not yet been converted to a spec_chain test.
