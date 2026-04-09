---
section: "02"
title: "Tack-Conformance Absorption"
status: not-started
reviewed: false
goal: "Mechanically supersede `plans/tack-conformance/` by adding header notices, updating reroute frontmatter, and creating an empty mapping-table file — no implementation, no file moves, no code changes."
success_criteria:
  - "`plans/tack-conformance/00-overview.md` carries a header note: 'Superseded by `plans/spec-conformance/`. New work continues under that plan. Existing section files remain in place for citation stability.'"
  - "`plans/tack-conformance/index.md` carries the same header note AND its frontmatter `status:` field is updated from `active` to `resolved`"
  - "`plans/spec-conformance/catalog/_legacy-tack-mapping.md` exists with header + empty table (filled by section 08)"
  - "Existing tack-conformance section files (01-09) are NOT moved, NOT renamed, NOT modified beyond the overview/index header notes"
  - "`plans/spec-conformance/00-overview.md` `supersedes` frontmatter already lists `plans/tack-conformance/` (verified)"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green (no code changes — should be a no-op)"
  - "Section's mission criterion connection: contributes to **`tack-conformance` plan absorbed and superseded** mission criterion"
inspired_by:
  - "Codex Round 2 + Step 6B feedback — leave files in place for citation stability and grepability; no rename churn during architecture work"
depends_on: ["01"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "02.1"
    title: "Add supersede notice to plans/tack-conformance/00-overview.md"
    status: not-started
  - id: "02.2"
    title: "Add supersede notice + update reroute frontmatter to plans/tack-conformance/index.md"
    status: not-started
  - id: "02.3"
    title: "Create plans/spec-conformance/catalog/_legacy-tack-mapping.md (empty)"
    status: not-started
  - id: "02.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "02.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 02: Tack-Conformance Absorption

**Status:** Not Started
**Goal:** Mechanically mark `plans/tack-conformance/` as superseded by `plans/spec-conformance/` so that new sessions reading the project state see one canonical plan rather than two parallel tracks. Per Codex's Round 2 + Step 6B guidance, this is **plan hygiene only** — no file moves, no implementation, no code changes. Existing tack-conformance section files (01-09) stay where they are for citation stability and grepability.

**Success Criteria:**
- [ ] `plans/tack-conformance/00-overview.md` has supersede header note
- [ ] `plans/tack-conformance/index.md` has supersede header note AND `status: resolved` frontmatter
- [ ] `plans/spec-conformance/catalog/_legacy-tack-mapping.md` exists (empty mapping table created in this section, populated by section 08)
- [ ] Existing tack-conformance section files (01-09) untouched beyond what's listed above
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green (zero code changes)
- [ ] Connects to mission criterion: **`tack-conformance` plan absorbed and superseded**

**Context:** Tack-conformance is in flight: sections 01-05 complete, section 06 (TOOLS_MENU_INVENTORY) just landed, sections 06.0.b/c-06.N pending, sections 07-09 not started. Per Codex's Step 6B feedback, absorbing this work via mechanical plan hygiene (rather than burying it inside section 08) keeps section 08 focused on protocol implementation. Doing the absorption early — Phase 0b — removes dual-track ambiguity while tack section 06.x is still moving. The mapping table file is created empty here and populated by section 08 as it converts tack scenarios into spec verification chains.

**Reference implementations:**
- **plans/roadmap/index.md** — example of `reroute` frontmatter shape with `status: resolved`
- **plans/completed/** — example of how completed plans carry "superseded" or "resolved" notes

**Depends on:** Section 01 (the catalog/ directory exists, so the empty mapping file can be placed inside it).

---

## 02.1 Add supersede notice to plans/tack-conformance/00-overview.md

**File(s):** `plans/tack-conformance/00-overview.md`

Add a header note immediately after the H1 title that points future readers to spec-conformance. Do NOT modify the rest of the file — its content remains valid for in-flight sections 06.* through 09.

- [ ] Read `plans/tack-conformance/00-overview.md` to understand its current header structure
- [ ] Add a blockquote header note immediately after the H1 (or after frontmatter if there's no H1):
  ```markdown
  > **⚠ Superseded by [plans/spec-conformance/](../spec-conformance/00-overview.md).**
  >
  > Tack-conformance is now part of the broader spec-conformance plan. New work
  > continues under spec-conformance. Existing section files (01-09) remain in
  > place for citation stability — commit messages and PRs that reference these
  > files (e.g., "tack-conformance/section-06") still resolve correctly.
  >
  > Section 02 of spec-conformance documents the absorption strategy. Section 08
  > of spec-conformance populates `plans/spec-conformance/catalog/_legacy-tack-mapping.md`
  > with the row → tack-section mapping as ECMA-48 baseline catalog rows are verified.
  ```
- [ ] Do NOT touch any other part of the file. The plan body, mission, sections list, and any in-flight notes stay as-is.
- [ ] **Validation**: `git diff plans/tack-conformance/00-overview.md` shows ONLY the added blockquote.

---

## 02.2 Add supersede notice + update reroute frontmatter to plans/tack-conformance/index.md

**File(s):** `plans/tack-conformance/index.md`

The index is the file the `/continue-roadmap` skill reads to find the active reroute. Updating its frontmatter `status: active` → `status: resolved` removes tack-conformance from the active reroute queue and lets `/continue-roadmap` discover spec-conformance instead.

- [ ] Read `plans/tack-conformance/index.md` to confirm the current frontmatter shape
- [ ] Update the frontmatter `status` field from `active` to `resolved`:
  ```yaml
  ---
  reroute: true
  name: "Tack Conformance"
  full_name: "Tack Conformance: Automated Terminfo Capability Validation Suite"
  status: resolved   # was: active
  order: 1
  ---
  ```
- [ ] Add a header blockquote (same shape as 02.1) immediately after the H1, pointing to spec-conformance.
- [ ] Update `plans/spec-conformance/index.md` frontmatter `status: queued` → `status: active` and `order: 2` → `order: 1` (it now becomes the active reroute since tack-conformance is resolved). Note: this is a 2-line edit to spec-conformance's own index.
- [ ] **Validation**: `grep -A1 'reroute:' plans/tack-conformance/index.md plans/spec-conformance/index.md` shows the expected status values

---

## 02.3 Create plans/spec-conformance/catalog/_legacy-tack-mapping.md (empty)

**File(s):** `plans/spec-conformance/catalog/_legacy-tack-mapping.md` (created)

The mapping table is an empty placeholder file with header + blank table. Section 08 (ECMA-48 baseline) populates it as it converts tack scenarios into spec verification chains. Creating it empty here means section 08 has a known target file to write into.

- [ ] Create `plans/spec-conformance/catalog/_legacy-tack-mapping.md`:
  ```markdown
  # Legacy Tack-Conformance Mapping

  > **Purpose**: This file maps spec-conformance catalog row IDs to legacy
  > `plans/tack-conformance/` section IDs. As section 08 (ECMA-48 baseline)
  > of spec-conformance converts existing tack scenarios into spec verification
  > chains, the mapping is recorded here. This preserves traceability from
  > the new catalog to the original tack work without renaming any files.
  >
  > **Maintenance**: Add rows as section 08 progresses. Each row is added when
  > a catalog row reaches `verified` status with credit owed to a tack scenario.

  | Catalog row    | Legacy tack section                                | Conversion status |
  |---             |---                                                 |---                |
  | _(empty — section 08 populates as it goes)_      |                                                    |                   |
  ```
- [ ] **Validation**: file exists and is readable by markdown parser

---

## 02.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 02.N Completion Checklist

- [ ] `plans/tack-conformance/00-overview.md` has supersede blockquote header (verified by `git diff`)
- [ ] `plans/tack-conformance/index.md` has supersede blockquote AND frontmatter `status: resolved`
- [ ] `plans/spec-conformance/index.md` frontmatter updated to `status: active`, `order: 1`
- [ ] `plans/spec-conformance/catalog/_legacy-tack-mapping.md` exists with header + empty table
- [ ] No other files in `plans/tack-conformance/` modified
- [ ] No code files modified anywhere (zero `.rs` changes)
- [ ] `./build-all.sh` green (no-op)
- [ ] `./test-all.sh` green (no-op)
- [ ] `./clippy-all.sh` green (no-op)
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` Quick Reference table status updated for section 02
- [ ] `00-overview.md` mission success criteria updated (`tack-conformance plan absorbed and superseded` checked off)
- [ ] `index.md` section 02 status updated
- [ ] `/tpr-review` passed — independent Codex review (this section is mechanical so review should be brief)
- [ ] `/impl-hygiene-review last commit` passed (hygiene review of zero code changes — should pass trivially)

**Exit Criteria:** `plans/tack-conformance/` carries "superseded" notices in both overview and index, frontmatter is `status: resolved`, spec-conformance is now the active reroute (`status: active`, `order: 1`), the empty `_legacy-tack-mapping.md` exists for section 08 to populate. Zero code changes. Existing in-flight tack section 06.* work continues under its existing path with no rename churn.
