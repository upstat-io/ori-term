---
section: "02"
title: "Tack-Conformance Absorption"
status: in-progress
reviewed: true
goal: "Mechanically supersede `plans/tack-conformance/` by adding supersede blockquotes to its overview and index, flipping frontmatter to terminal states across BOTH plans' index/overview/section-09 files atomically, creating the empty `_legacy-tack-mapping.md` mapping file (seeded with the tack-09→spec-23 handoff row), and removing stale narrative from `spec-conformance/00-overview.md` and spec-conformance sections 17 and 23 so one coherent story emerges — zero code changes."
success_criteria:
  - "`plans/tack-conformance/00-overview.md` carries the canonical supersede blockquote (see 02.1 template) immediately after its H1 AND its top-level frontmatter `status:` is updated from `in-progress` to `complete` (legal pairing with `index.md: resolved` per `.claude/skills/create-plan/plan-schema.md:675,684-687`)"
  - "`plans/tack-conformance/index.md` carries the canonical supersede blockquote immediately after its H1 AND its reroute frontmatter `status:` is updated from `active` to `resolved`"
  - "`plans/tack-conformance/section-09-verification.md` has its frontmatter `status:` flipped from `not-started` to `complete` AND a prominent body-level supersede notice is added under its H1 explaining that its scope is now owned by `plans/spec-conformance/section-23-cross-stack-regression-sweep.md`. Schema-legal rationale: `.claude/skills/create-plan/plan-schema.md:609-613` restricts sections to `not-started | in-progress | complete`; since the plan is `resolved` and the remaining work is owned elsewhere, `complete` (this section's tack-local work is closed) is the only schema-legal terminal state. The body note is load-bearing — it is where the honesty lives."
  - "`plans/tack-conformance/section-09-verification.md` subsection entries (09.1–09.N in the frontmatter `sections:` array) are ALL flipped from `not-started` to `complete`, matching the parent section. Plan-audit rejects parent=complete + child=not-started mismatches."
  - "`plans/spec-conformance/index.md` reroute frontmatter is updated: `status: queued` → `status: active`. `order: 2` is LEFT UNCHANGED per `.claude/skills/create-plan/plan-schema.md:647-658` promotion algorithm (`order is UNCHANGED — it was already numbered ahead of future queued plans`). The transition is the queue-promotion operation described in `.claude/skills/create-plan/plan-schema.md:647-658` — spec-conformance becomes the active reroute the same moment tack-conformance becomes `resolved`, but no renumbering happens. Only the `status:` line flips."
  - "`plans/spec-conformance/catalog/_legacy-tack-mapping.md` exists with the canonical header + an initially-empty table shell AND one seed row recording the tack-09 → spec-23 handoff (so Section 23's implementers discover the absorption through the mapping file, not by accident). The catalog/ directory is created if absent — this is the ONLY legal dependency on Section 01."
  - "`plans/spec-conformance/00-overview.md` stale narrative is corrected: line 351 (`sections 01-05 complete, section 06 just landed, sections 06.0.b/c–06.N pending, sections 07-09 not yet started`) is replaced with the current reality (sections 01-08 complete, section 09 absorbed by spec-23, nothing pending in tack-conformance). Line 367 (`New work that would have been tack-conformance sections 07-09 …`) is rewritten because sections 07-08 already landed — the only surviving handoff is 09 → spec-23."
  - "`plans/spec-conformance/00-overview.md` designates its **Tack Absorption Strategy** section as the ONE canonical home for absorption policy. Section 02's blockquote, Section 23's inline note, and Section 17's inline note all defer to it via file reference (no policy restating). This fixes the LEAK:scattered-knowledge finding surfaced in Phase 2 reviews."
  - "`plans/spec-conformance/section-17-kitty-keyboard.md` body is annotated with an inline pointer explaining that the kf1-kf63 / cursor / editing / modified-key infrastructure ALREADY LANDED under `oriterm/src/key_encoding/terminfo_xcheck/` via tack-conformance section 08, so Section 17 absorbs and EXTENDS that surface rather than rewriting it. Pointer references the canonical Tack Absorption Strategy block in 00-overview.md (no policy restatement)."
  - "`plans/spec-conformance/section-23-cross-stack-regression-sweep.md` body is annotated with an inline pointer explaining that tack-conformance sections 01-08 already exist as completed artifacts on main, and that tack-09's verification/archival scope is fully inherited here. Pointer references the canonical Tack Absorption Strategy block (no policy restatement). The Phase 2 finding about scope inheritance from tack-09 is explicitly satisfied."
  - "`plans/spec-conformance/00-overview.md` `supersedes:` frontmatter already lists `plans/tack-conformance/` (verified at `00-overview.md:5-6`, no edit needed — but 02.N checks this so drift is caught)"
  - "Physical files under `plans/tack-conformance/section-01-*.md` through `section-08-*.md` are NOT moved, NOT renamed, NOT edited. Citation stability is preserved. The only tack-conformance files edited by Section 02 are `index.md`, `00-overview.md`, and `section-09-verification.md`. No `git mv` is run. This is the explicit 'no file moves' covenant from the Tack Absorption Strategy."
  - "All Section 02 edits land in **one atomic commit** — `plans/tack-conformance/index.md`, `plans/tack-conformance/00-overview.md`, `plans/tack-conformance/section-09-verification.md`, `plans/spec-conformance/index.md`, `plans/spec-conformance/00-overview.md`, `plans/spec-conformance/section-17-kitty-keyboard.md`, `plans/spec-conformance/section-23-cross-stack-regression-sweep.md`, and `plans/spec-conformance/catalog/_legacy-tack-mapping.md` are all staged and committed together. A partial commit leaves `/continue-roadmap` in a split-brain state (two reroutes both `active`, or one `resolved` with a child still `not-started`) that the scanner cannot reason about."
  - "`python3 .claude/skills/plan-audit/plan-audit.py plans/spec-conformance --verify --json` and `python3 .claude/skills/plan-audit/plan-audit.py plans/tack-conformance --verify --json` are run BOTH before AND after the atomic commit, and the **finding-set diff** (`post_set - baseline_set` where each fingerprint is `(location, check, full_message)`) is EMPTY on both plans. Count comparison alone is INSUFFICIENT — a plan that resolves 3 old MAJORs while introducing 2 new ones has `count_delta = -1` but hides 2 real regressions. Plan-audit has ~183 pre-existing MAJOR findings on spec-conformance and ~42 on tack-conformance (DEAD_PATH, DUPLICATE_ITEM, SIZE_VIOLATION, BLOAT_RISK — baseline noise across the whole plan corpus) — Section 02 is NOT expected to fix any of them, just not to introduce new ones. Expected `post_set - baseline_set`: spec=`{}`, tack=`{}`. Expected `baseline_set - post_set` (resolved): spec=2 (`DEAD_PATH` on `_legacy-tack-mapping.md` in section-02 + section-08 both resolve when Section 02.4 creates the file), tack=0. Any non-empty `post_set - baseline_set` is a Broken Window Policy violation and blocks completion. Plan-audit is the ONLY meaningful validator for a zero-code plan-hygiene section — build/clippy/test scripts are trivial no-ops."
  - "`./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` green (trivial no-op since zero code changes — treated as a sanity gate, not the primary validator)"
  - "Section's mission criterion connection: delivers the `tack-conformance plan absorbed and superseded` mission criterion in `plans/spec-conformance/00-overview.md` at approximate line 41 — that checkbox flips to `[x]` as part of this section's completion, not as a follow-up."
inspired_by:
  - "Codex Round 2 + Step 6B feedback — leave files in place for citation stability and grepability; no rename churn during architecture work"
  - "Phase 2 /tp-help (Codex + Gemini) dual-reviewer consensus — fix stale narrative, collapse split-brain frontmatter, designate single canonical absorption-policy home"
  - "`.claude/skills/create-plan/plan-schema.md:609-613,619-623,675,684-687` — the schema's legal status vocabulary constrains which fields can be flipped to which terminal value"
depends_on: []
third_party_review:
  status: resolved
  updated: 2026-04-11
sections:
  - id: "02.1"
    title: "Flip tack-conformance overview + index frontmatter and add supersede blockquotes"
    status: not-started
  - id: "02.2"
    title: "Freeze tack-conformance/section-09-verification.md (frontmatter + body notice)"
    status: not-started
  - id: "02.3"
    title: "Promote spec-conformance reroute (index.md) + correct stale absorption narrative in 00-overview.md"
    status: not-started
  - id: "02.4"
    title: "Create plans/spec-conformance/catalog/_legacy-tack-mapping.md (seeded with tack-09 → spec-23 row)"
    status: not-started
  - id: "02.5"
    title: "Annotate spec-conformance sections 17 and 23 with absorption pointers (no policy restatement)"
    status: not-started
  - id: "02.6"
    title: "Atomic commit + plan-audit --verify on BOTH plans"
    status: not-started
  - id: "02.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "02.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 02: Tack-Conformance Absorption

**Status:** In Progress (Phase 4 TPR cycle complete — ready for execution)
**Goal:** Mechanically mark `plans/tack-conformance/` as superseded by `plans/spec-conformance/` so that new sessions reading the project state see one canonical plan rather than two parallel tracks, AND resolve the cross-section drift (stale narrative, split-brain frontmatter, zombie section 09, scattered absorption policy) that the Phase 2 dual-reviewer pass surfaced. Per Codex's Round 2 + Step 6B guidance this is **plan hygiene only** — no file moves, no code changes — but it IS load-bearing plan-state surgery that must land atomically: `/continue-roadmap`, the plan-audit verifier, and future session readers all rely on a consistent state across seven files. Existing tack-conformance section files 01-08 stay untouched for citation stability and grepability.

**Current reality (anchor for the narrative corrections below):** tack-conformance sections 01 through 08 are ALL `complete` on main (see `plans/tack-conformance/00-overview.md:22-35,210-218` — every mission criterion except two is checked off and every section row in the Quick Reference says Complete). Only section 09 (final verification/archival) remains `not-started`, and its scope is already declared absorbed by spec-23 — specifically, `plans/spec-conformance/section-23-cross-stack-regression-sweep.md:19` (the `inspired_by` frontmatter field, pre-existing) reads `"tack-conformance section 09 (verification) — original plan for cross-platform verification, now subsumed by this section"`. That frontmatter claim is declarative intent; the matching BODY-level inline pointer is added by 02.5.b of this section. The tack plan is not in-flight; it is 8/9 done with the last mile relocated to spec-conformance. Section 02 formalizes that reality across every file that still says otherwise.

**Success Criteria (body form — bidirectional with frontmatter):**
- [ ] tack-conformance/00-overview.md: supersede blockquote + `status: in-progress` → `status: complete`
- [ ] tack-conformance/index.md: supersede blockquote + `status: active` → `status: resolved`
- [ ] tack-conformance/section-09-verification.md: parent `status: not-started` → `complete`, all subsection statuses (09.1–09.N) → `complete`, body-level supersede notice added pointing at spec-23
- [ ] spec-conformance/index.md: `status: queued` → `active` (queue promotion per `.claude/skills/create-plan/plan-schema.md:647-658`; `order: 2` is LEFT UNCHANGED per that same algorithm — `order is UNCHANGED` is a direct quote from line 658)
- [ ] spec-conformance/catalog/_legacy-tack-mapping.md: created with canonical header, empty table shell, and the seed row `spec-23 ↔ tack-09 (absorption handoff — verification/archival)`
- [ ] spec-conformance/00-overview.md: Tack Absorption Strategy prose at lines 349-371 corrected — no more "sections 01-05 complete, 07-09 not yet started" language; the block becomes the canonical single-source policy home that other files reference
- [ ] spec-conformance/section-17-kitty-keyboard.md: inline pointer added noting the kf1-kf63 / cursor / editing / modified-key infrastructure already landed via tack-08 and that Section 17 extends it (policy pointer, no restatement)
- [ ] spec-conformance/section-23-cross-stack-regression-sweep.md: inline pointer added noting tack-01-08 are completed artifacts on main and that tack-09's scope is subsumed here (policy pointer, no restatement)
- [ ] Single atomic commit containing all eight file edits
- [ ] `plan-audit.py --verify --json` on BOTH `plans/spec-conformance` and `plans/tack-conformance`: the post-commit finding-set diff `post_set - baseline_set == {}` (where each fingerprint is `(location, check, full_message)`) on both plans. Count comparison is insufficient; the authoritative gate is the set difference. Expected shape: spec `NEW=0 resolved=2` (02.4 resolves `DEAD_PATH` on `_legacy-tack-mapping.md` in section-02 + section-08), tack `NEW=0 resolved=0`. See 02.6 for the full command and rationale.
- [ ] `00-overview.md` mission criterion `tack-conformance plan absorbed and superseded` at line 41 is checked `[x]`
- [ ] Connects to mission criterion: **`tack-conformance` plan absorbed and superseded**

**Context:** Phase 2 dual-reviewer analysis (Codex + Gemini consensus, 10 findings) surfaced that the previous shape of this section would leave the repository in an **illegal plan state**: a `resolved` tack plan whose overview still said `in-progress`, whose section-09 still said `not-started`, whose Quick Reference still said Not Started, whose parent spec-conformance overview still described tack as in-flight, whose spec-conformance sections 17 and 23 pretended tack-8 and tack-9 infrastructure didn't exist, and whose absorption policy was scattered across four locations (LEAK:scattered-knowledge). The `/continue-roadmap` scanner, the plan-audit verifier, and human readers each pay the cost of that drift. Section 02 collapses all of it into one atomic edit.

**Why this is zero-code but NOT trivial:** The only meaningful validator for a plan-hygiene section is `python3 .claude/skills/plan-audit/plan-audit.py --verify --json`. Running `./build-all.sh` / `./test-all.sh` / `./clippy-all.sh` produces a no-op pass because nothing in `crates/*/src/**` changes — those scripts are kept in the checklist as a trivial sanity gate, not as the primary verifier. The primary verifier is plan-audit, and 02.6 makes its invocation mandatory.

**Reference implementations:**
- **`.claude/skills/create-plan/plan-schema.md`:609-613** — sections use `not-started | in-progress | complete` (no `superseded`)
- **`.claude/skills/create-plan/plan-schema.md`:619-623** — index.md reroutes use `active | queued | resolved`
- **`.claude/skills/create-plan/plan-schema.md`:675,684-687** — pairing: `index.md status: resolved` ↔ `00-overview.md status: complete`
- **`.claude/skills/create-plan/plan-schema.md`:647-658** — queue promotion algorithm (the `active`/`queued` flip in 02.3)
- **plans/completed/teseq-conformance/** — canonical example of a resolved plan: `index.md: resolved`, `00-overview.md: complete`, every section `complete`
- **plans/spec-conformance/00-overview.md:349-371** — the Tack Absorption Strategy block — this section repairs its prose and designates it the canonical policy home

**Depends on:** **None.** The previous draft declared `depends_on: ["01"]` because Section 02.3 creates a file inside `plans/spec-conformance/catalog/`, and Section 01 (Catalog Bootstrap) also creates files there. But Section 01's own frontmatter (`section-01-catalog-bootstrap.md:8-9`) explicitly says Section 02 is the creator of `_legacy-tack-mapping.md`, and Section 02.4 can trivially create the `catalog/` directory with `mkdir -p`. Gating a 5-minute plan-hygiene task on the massive Section 01 bootstrap defeats the "Phase 0b — do it early" goal. Dependency removed.

---

## 02.1 Flip tack-conformance overview + index frontmatter and add supersede blockquotes

**File(s):** `plans/tack-conformance/00-overview.md`, `plans/tack-conformance/index.md`

This subsection performs the PLAN-LEVEL frontmatter flip (the outermost layer of state — "this plan is closed, its work is absorbed") and adds the canonical supersede blockquote to both the overview and the index. Both edits land in the same subsection because they form a single semantic unit: the plan's top-level state is SSOT-ified across the two files that readers land on first.

**Canonical supersede blockquote template (used verbatim in 02.1.a + 02.1.b — inserted into `tack-conformance/00-overview.md` and `tack-conformance/index.md`). 02.2 inserts a DIFFERENT body-level absorption notice into `tack-conformance/section-09-verification.md` — see 02.2 for that template. 02.3 does NOT insert the blockquote — 02.3 edits `spec-conformance/00-overview.md` + `spec-conformance/index.md` for queue promotion + stale-narrative corrections, not supersede notices.**

```markdown
> **⚠ Superseded by [plans/spec-conformance/](../spec-conformance/00-overview.md).**
>
> Sections 01–08 of tack-conformance are **completed artifacts** committed to `main`
> (each section file's frontmatter carries `status: complete`; `plans/tack-conformance/00-overview.md`'s
> Quick Reference table and the per-section `**Status:**` labels in
> `plans/tack-conformance/index.md`'s keyword clusters both reflect this). Section 09's
> row and label literally still say `Not Started` / `not-started` in those surfaces
> (02.1's scope fence forbids editing them), but its scope is absorbed per the next paragraph.
> Section 09 (final verification + archival) is superseded by
> [plans/spec-conformance/section-23-cross-stack-regression-sweep.md](../spec-conformance/section-23-cross-stack-regression-sweep.md),
> which explicitly inherits the cross-platform sweep, flake gate, performance
> invariants, and archival gate.
>
> Existing tack-conformance section files (`section-01-*.md` through
> `section-09-*.md`) remain in place for citation stability — commit messages
> and PRs that reference these paths (for example `tack-conformance/section-06`)
> continue to resolve correctly. No `git mv` is run. No files are deleted.
>
> The canonical absorption policy lives in
> [plans/spec-conformance/00-overview.md §Tack Absorption Strategy](../spec-conformance/00-overview.md#tack-absorption-strategy-delivered-by-section-02).
> This notice is intentionally a pointer, not a restatement.
```

### 02.1.a Edit `plans/tack-conformance/00-overview.md`

- [ ] Read `plans/tack-conformance/00-overview.md:1-15` and confirm the current frontmatter shape matches:
  ```yaml
  ---
  plan: "tack-conformance"
  title: "Tack Conformance: Automated Terminfo Capability Validation Suite"
  status: in-progress
  references:
    - "plans/completed/vttest-conformance/"
    - "plans/completed/golden-image-audit/"
  ---
  ```
  If `status:` is not `in-progress`, STOP and invoke `AskUserQuestion` — the precondition has drifted and blind editing would destroy state.
- [ ] Change `status: in-progress` → `status: complete` on that one frontmatter line. Per `.claude/skills/create-plan/plan-schema.md:675,684-687` this is the legal pairing with `index.md: resolved`.
- [ ] Immediately after the `# Tack Conformance: Automated Terminfo Capability Validation Suite` H1 (line 10) and BEFORE the `## Mission` heading, insert a blank line followed by the canonical supersede blockquote above. Leave a trailing blank line before `## Mission`.
- [ ] Do NOT touch `## Mission`, `## Mission Success Criteria` (the unchecked criteria are explicitly inherited by spec-23 — the body-level note explains why they stay unchecked here without being a lie), `## Architecture`, `## Quick Reference`, or anything else below the blockquote. The plan's self-documentation is preserved verbatim for citation stability.
- [ ] **Validation**: `git diff plans/tack-conformance/00-overview.md` shows exactly two changed regions — the frontmatter `status:` line and the inserted blockquote. No other lines touched.

### 02.1.b Edit `plans/tack-conformance/index.md`

- [ ] Read `plans/tack-conformance/index.md:1-8` and confirm the current frontmatter matches:
  ```yaml
  ---
  reroute: true
  name: "Tack Conformance"
  full_name: "Tack Conformance: Automated Terminfo Capability Validation Suite"
  status: active
  order: 1
  ---
  ```
  If `status:` is not `active` or `order:` is not `1`, STOP and invoke `AskUserQuestion`.
- [ ] Change `status: active` → `status: resolved` on that one frontmatter line. Leave `order: 1` as-is — `order` does not change on resolution (per `.claude/skills/create-plan/plan-schema.md:647-658`, resolution removes the plan from the active/queued sets entirely; the numeric order is historical after that).
- [ ] Immediately after the `# Tack Conformance Index` H1 (line 9) and BEFORE the `> **Maintenance Notice:** …` line, insert a blank line, the canonical supersede blockquote, and another blank line.
- [ ] Do NOT modify `## How to Use`, the Keyword Clusters, or the Quick Reference table — all of that remains valid historical reference material for anyone following a citation back to a tack section.
- [ ] **Validation**: `git diff plans/tack-conformance/index.md` shows exactly two changed regions — the frontmatter `status:` line and the inserted blockquote.

---

## 02.2 Freeze tack-conformance/section-09-verification.md (frontmatter + body notice)

**File(s):** `plans/tack-conformance/section-09-verification.md`

The Phase 2 reviewers flagged this as the **zombie section 09** issue: after Section 02 runs, the tack plan becomes `resolved` but its section-09 frontmatter still says `status: not-started`. `/continue-roadmap` treats that as "unfinished section in a resolved plan" — an illegal plan state the scanner cannot reason about. Plan-audit's `--verify` rejects parent/child status mismatches. The fix is to flip section-09 to a terminal state AND stamp a prominent body notice so future readers understand the reason is absorption, not abandonment.

Schema constraint: sections are restricted to `not-started | in-progress | complete` (`.claude/skills/create-plan/plan-schema.md:609-613`). No `superseded` value exists. Since sections 01-08 of tack are already `complete` and section 09's scope is now owned by spec-23 (its local work IS closed — it has been relocated, not abandoned), `complete` is the schema-legal terminal state. The body-level notice is where the honesty about "complete by being absorbed" lives.

- [ ] Read `plans/tack-conformance/section-09-verification.md:1-53` and confirm frontmatter `status: not-started` AND every subsection in the `sections:` array (`09.1`, `09.2`, `09.3`, `09.4`, `09.5`, `09.R`, `09.N`) is `status: not-started`.
- [ ] Change the top-level `status: not-started` → `status: complete` on line 4.
- [ ] Change every subsection entry in the `sections:` array (approximate lines 32-52) from `status: not-started` → `status: complete`. All 7 entries. Plan-audit rejects parent=complete with child≠complete.
- [ ] Immediately after the `# Section 09: Verification` H1 (approximate line 55) insert a prominent body-level supersede notice:
  ```markdown
  > **⚠ This section's scope is absorbed by
  > [plans/spec-conformance/section-23-cross-stack-regression-sweep.md](../spec-conformance/section-23-cross-stack-regression-sweep.md).**
  >
  > Tack-conformance sections 01–08 are complete on `main`. Section 09's
  > remaining scope — cross-platform skip matrix verification, `./test-all.sh`/
  > `./build-all.sh`/`./clippy-all.sh` green gate, flake-proofing matrix,
  > alloc/RSS regression checks, DA/DSR cross-validation, and plan archival —
  > is now owned by spec-conformance Section 23. The archival clause in 09.5
  > is **NOT** executed: per the Tack Absorption Strategy
  > ([plans/spec-conformance/00-overview.md §Tack Absorption Strategy](../spec-conformance/00-overview.md#tack-absorption-strategy-delivered-by-section-02))
  > no `git mv` is run and the files stay in place for citation stability.
  > This explicitly **overrides** the archival instructions in 09.5 and 09.N
  > that say `git mv plans/tack-conformance plans/completed/tack-conformance`
  > — those instructions are now void.
  >
  > The two currently-unchecked mission criteria in
  > `plans/tack-conformance/00-overview.md` (`All tests skip cleanly when
  > tack/tic unavailable` and `./test-all.sh/build-all.sh/clippy-all.sh
  > green — no regressions`) are owned by Section 23's cross-stack sweep.
  > They remain unchecked in tack-00-overview only as a historical artifact;
  > the live contract lives in spec-conformance.
  >
  > Status `complete` here means "this section's local work is closed" — the
  > plan schema (`.claude/skills/create-plan/plan-schema.md:609-613`) has no `superseded` value for
  > sections, so `complete` is the schema-legal terminal state. This
  > blockquote is where the honesty lives.
  ```
- [ ] Do NOT modify any subsection body content below the blockquote — 09.1 through 09.N remain as historical reference for what Section 23 is inheriting.
- [ ] **Validation**: `git diff plans/tack-conformance/section-09-verification.md` shows frontmatter status flips (parent + 7 children) plus the one inserted blockquote — no other text changes.

---

## 02.3 Promote spec-conformance reroute (index.md) + correct stale absorption narrative in 00-overview.md

**File(s):** `plans/spec-conformance/index.md`, `plans/spec-conformance/00-overview.md`

This subsection flips the spec-conformance side of the atomic edit (queue promotion) AND corrects the **stale absorption narrative** in `plans/spec-conformance/00-overview.md` at approximate lines 351 and 367 that the Phase 2 reviewers flagged. Both edits land in the same subsection because they form a single semantic unit: the spec-conformance plan becomes the authoritative voice, so its own self-description has to match current reality.

### 02.3.a Edit `plans/spec-conformance/index.md` — queue promotion

- [ ] Read `plans/spec-conformance/index.md:1-8` and confirm the current frontmatter matches:
  ```yaml
  ---
  reroute: true
  name: "Spec Conformance"
  full_name: "Spec Conformance: Universal Terminal Protocol Verification"
  status: queued
  order: 2
  ---
  ```
  If `status:` is not `queued` or `order:` is not `2`, STOP and invoke `AskUserQuestion`.
- [ ] Change `status: queued` → `status: active` (promotion). This is the ONLY frontmatter line that changes.
- [ ] Leave `order: 2` **UNCHANGED**. Per `.claude/skills/create-plan/plan-schema.md:647-658` promotion algorithm — line 658 literally reads `order is UNCHANGED — it was already numbered ahead of future queued plans`. The `order` field is the queue position a plan had when first added; it is NOT updated on promotion. Changing `order: 2` → `order: 1` would violate the schema invariant and was a bug in the prior draft of this section. There is no second active reroute to conflict with — `status: active` alone determines reroute priority, not `order` — so keeping `order: 2` has zero runtime effect on `/continue-roadmap` scanner selection.
- [ ] Update the Section 02 per-section status label — in `plans/spec-conformance/index.md` each section cluster carries a `**File:** ... | **Status:** ...` header line (for Section 02, approximate line 47 — search for `**File:** \`section-02-tack-absorption.md\` | \*\*Status:\*\*`). Flip this label from `**Status:** Not Started` to `**Status:** Complete` at Section 02 closeout. **This is NOT the Quick Reference table** — `plans/spec-conformance/index.md`'s Quick Reference table at the bottom of the file has columns `| ID | Title | File |` (3 columns, NO Status column), so it has no row to flip. The per-section `**Status:**` label IS the canonical section-status surface in `index.md`. If the label doesn't exist or uses a different shape, STOP and invoke `AskUserQuestion`.
- [ ] Do NOT touch any OTHER part of the file. The 535-line BLOAT concern is already filed as `BUG-07-011` (see `plans/bug-tracker/section-07-ci-build.md`) and is out of scope for this atomic edit. The 26 keyword-cluster blocks (except for the one-line Section 02 `**Status:**` flip), the `## How to Use` header, the `## Catalog Files` cross-ref table, and the `## Quick Reference` 3-column `| ID | Title | File |` table all stay exactly as-is. Adding index-trimming work here would double-scope Section 02 and risk an overlapping edit with Section 01's catalog-bootstrap work on the same file.
- [ ] **Validation**:
  - `head -8 plans/spec-conformance/index.md` shows `reroute: true`, `name:`, `full_name:`, `status: active`, `order: 2` (status flipped, order UNCHANGED). The previous `grep -A1 'reroute:'` form only printed the line containing `reroute:` plus one following line (the `name:` line), not the `status:` or `order:` lines — `head -8` or an equivalent multi-line inspection is required to verify the whole frontmatter block.
  - `grep -B1 '\*\*Status:\*\* Complete' plans/spec-conformance/index.md | grep -A1 'section-02-tack-absorption.md'` confirms the Section 02 per-section status label is now `Complete`.
  - `git diff plans/spec-conformance/index.md` shows exactly TWO changed regions — the reroute `status:` frontmatter line and the one `**Status:**` label for Section 02. No other lines touched.

### 02.3.b Edit `plans/spec-conformance/00-overview.md` — correct stale narrative + designate canonical policy home

The **Tack Absorption Strategy** block at `00-overview.md:349-371` is currently the most detailed absorption-policy prose in the repository, but it carries stale narrative that contradicts reality. Section 02 repairs the prose AND elevates this block to the **canonical single-source-of-truth** for absorption policy, so the Phase 2 LEAK:scattered-knowledge finding is resolved (02.1/02.2/02.5 blockquotes all *point* at this block rather than restating it).

- [ ] Read `plans/spec-conformance/00-overview.md:349-371` to get the current state of the Tack Absorption Strategy block.
- [ ] Replace line 351 (`plans/tack-conformance/ is in flight: sections 01-05 complete, section 06 (TOOLS_MENU_INVENTORY) just landed, sections 06.0.b/c–06.N pending, sections 07-09 not yet started.`) with:
  ```markdown
  `plans/tack-conformance/` is **effectively 8/9 done**: sections 01–08 are all `complete`
  on `main` (see `plans/tack-conformance/00-overview.md` Quick Reference) and deliver
  the shared `PtySession` infrastructure, the pinned `ori_term.info` terminfo entry,
  the scenario-catalog framework, the full test-menu and tools-menu scenario families,
  GPU goldens for the visual subset, and the full kf1–kf63 + cursor + editing + modified-key
  terminfo cross-check at `oriterm/src/key_encoding/terminfo_xcheck/`. Only section 09
  (final verification + archival) remains `not-started`, and its scope is absorbed by
  spec-conformance Section 23 (Cross-Stack Regression Sweep + Coverage CI).
  ```
- [ ] Replace item 5 at line 367 (`New work that would have been tack-conformance sections 07-09 (GPU goldens, keyboard tests, final verification) is created directly under spec-conformance …`) with:
  ```markdown
  5. Tack-conformance section 09 (final verification + archival) is absorbed by
     `plans/spec-conformance/section-23-cross-stack-regression-sweep.md`, which
     already declares it subsumes tack-09 (`section-23-*.md:19`). The two currently
     unchecked mission criteria in `plans/tack-conformance/00-overview.md` (cross-platform
     skip cleanliness and the `./test-all.sh`/`./build-all.sh`/`./clippy-all.sh` green gate)
     become Section 23's responsibility. No tack sections are created or renamed.
  ```
- [ ] Immediately above the `## Tack Absorption Strategy (delivered by Section 02)` heading (approximate line 349), insert a short marker paragraph designating this block as canonical:
  ```markdown
  > **Canonical absorption-policy home.** This block is the single source of truth
  > for how the tack-conformance plan is superseded. Other files in the repository
  > (`plans/tack-conformance/index.md`, `plans/tack-conformance/00-overview.md`,
  > `plans/tack-conformance/section-09-verification.md`, `plans/spec-conformance/section-02-tack-absorption.md`,
  > `plans/spec-conformance/section-17-kitty-keyboard.md`, `plans/spec-conformance/section-23-cross-stack-regression-sweep.md`,
  > and `plans/spec-conformance/catalog/_legacy-tack-mapping.md`) reference this
  > block by path rather than restating its content. This prevents the
  > `LEAK:scattered-knowledge` finding the Phase 2 reviewers surfaced.
  ```
- [ ] Update the mission success criterion at approximate line 41 (`**`tack-conformance` plan absorbed and superseded** — `plans/tack-conformance/` is marked superseded with a mapping table from spec catalog row IDs to legacy tack section IDs. Existing in-flight section 06.* stays in place; new tack sections 07-09 are created directly under spec-conformance. Delivered by section 02.`). The prose is stale because tack sections 06, 07, and 08 have ALL shipped since the criterion was drafted. Replace its body with the current reality:
  ```markdown
  - [ ] **`tack-conformance` plan absorbed and superseded** — `plans/tack-conformance/` carries supersede blockquotes in overview + index; index reroute frontmatter is `status: resolved`; overview frontmatter is `status: complete`; `section-09-verification.md` is flipped to `status: complete` with an explicit body-level absorption notice pointing at spec-conformance Section 23. Tack sections 01–08 are completed artifacts committed to `main` (shared `PtySession` infrastructure, pinned `ori_term.info`, scenario-catalog framework, test-menu + tools-menu scenario families, GPU goldens, full kf1–kf63 + cursor + editing + modified-key terminfo cross-check). Tack section 09 (final verification + archival) is absorbed by spec-conformance Section 23 (Cross-Stack Regression Sweep + Coverage CI). `plans/spec-conformance/catalog/_legacy-tack-mapping.md` exists with the tack-09 → spec-23 seed row. No `git mv` is run (citation-stability covenant). Delivered by section 02.
  ```
  During Section 02's own completion closeout the `- [ ]` checkbox flips to `- [x]` — see 02.N.
- [ ] Update the spec-conformance Quick Reference table row for Section 02 (approximate search term `| 02 | Tack-Conformance Absorption`) from `Status: Not Started` to `Status: Complete` as part of Section 02's own closeout. This is the ONLY Quick Reference edit Section 02 makes. If the table doesn't exist yet or is in a different shape, STOP and invoke `AskUserQuestion` — the pre-condition has drifted.
- [ ] Do NOT touch the other `## Mission Success Criteria` entries (lines 27–45 EXCEPT line 41 as updated above), the `supersedes:` frontmatter at lines 5-6, the architecture diagram (approximate lines 49–150), the verification taxonomy (approximate lines 373+), or the authority ladder (approximate lines 388+). The scope fence around Section 02's closeout edits: (a) Tack Absorption Strategy block at 349-371, (b) mission criterion at line 41, (c) Quick Reference table row for Section 02. Everything else stays exactly as-is.
- [ ] **Validation**: `git diff plans/spec-conformance/00-overview.md` shows at most FIVE changed regions — the inserted canonical-home marker, the replaced line 351, the replaced item 5, the updated mission criterion at line 41 (text replacement during execution; `[x]` flip at closeout), and the Quick Reference Section 02 status flip to Complete. No other lines touched.

---

## 02.4 Create plans/spec-conformance/catalog/_legacy-tack-mapping.md (seeded with tack-09 → spec-23 row)

**File(s):** `plans/spec-conformance/catalog/_legacy-tack-mapping.md` (created), `plans/spec-conformance/catalog/` (directory, created if absent)

The mapping file is the empirical traceability artifact linking spec-conformance catalog row IDs to legacy tack section IDs. Section 08 and Section 23 both populate it (08 appends rows as it converts tack scenarios into spec_chain tests; 23 uses it to verify absorption completeness). Section 02.4 creates the file with the canonical header, the empty conversion-table shell, AND one seed row documenting the tack-09 → spec-23 handoff so the absorption traceability is discoverable from inside the mapping file itself, not buried in prose.

- [ ] Create the directory if needed: `mkdir -p plans/spec-conformance/catalog/`. (Section 01's `catalog/` bootstrap may or may not have landed — Section 02 no longer depends on Section 01, so `mkdir -p` is the contract. The flag is idempotent.)
- [ ] Confirm `plans/spec-conformance/catalog/_legacy-tack-mapping.md` does NOT already exist. If it does, STOP and invoke `AskUserQuestion` — something has pre-created it and Section 02's creation would clobber.
- [ ] Write `plans/spec-conformance/catalog/_legacy-tack-mapping.md` with the following content verbatim:
  ````markdown
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

  | Catalog row ID / handoff               | Legacy tack section                                                      | Conversion status   |
  |---                                     |---                                                                       |---                  |
  | *(section 02 handoff — not a catalog row)*  | `plans/tack-conformance/section-09-verification.md`                 | `absorbed-by-spec-23` |
  | *(section 08 populates as it goes)*    |                                                                          |                     |

  ### Status vocabulary

  - `converted` — tack scenario's PTY fixture bytes now drive a spec_chain test at the catalog row listed in the first column; both tests co-exist for regression coverage.
  - `absorbed-by-spec-23` — the tack-side work is closed in place and its remaining scope is owned by spec-conformance Section 23 (no new spec_chain test is written; the absorption is structural, not behavioral).
  - `absorbed-by-spec-17` — the tack-side work is closed in place and its remaining scope is owned by spec-conformance Section 17 (kitty keyboard + modifyOtherKeys + Win32 Input encoders extending the tack-08 infrastructure).
  - `pending` — tack scenario exists but has not yet been converted to a spec_chain test.
  ````
- [ ] **Validation**: the file exists, is readable by a markdown parser, contains the header + the table with the tack-09 seed row + the status vocabulary. `wc -l plans/spec-conformance/catalog/_legacy-tack-mapping.md` should show a nontrivial count (≥ 30).

---

## 02.5 Annotate spec-conformance sections 17 and 23 with absorption pointers (no policy restatement)

**File(s):** `plans/spec-conformance/section-17-kitty-keyboard.md`, `plans/spec-conformance/section-23-cross-stack-regression-sweep.md`

Phase 2 dual-reviewer finding #8 surfaced that sections 17 and 23 currently read as if tack-08 and tack-09 infrastructure did not exist — Section 17 describes building key_encoding work from scratch, Section 23 only references tack-09 by one sentence at line 19. After Section 02 runs, both sections must point their implementers at the completed tack artifacts they inherit. These are SHORT, FOCUSED pointers — they do NOT restate absorption policy; they reference the canonical home in `spec-conformance/00-overview.md` (designated by 02.3.b).

### 02.5.a Edit `plans/spec-conformance/section-17-kitty-keyboard.md`

- [ ] Read `plans/spec-conformance/section-17-kitty-keyboard.md:55-70` for the current shape of the `## Context` / `## 17.1` region.
- [ ] Insert the following pointer paragraph immediately after the existing `**Reference implementations:** see frontmatter.` line (approximate line 64) and before `**Depends on:**`:
  ```markdown
  **Absorbed infrastructure (from tack-conformance section 08, now `complete`):**
  The terminfo cross-check surface at `oriterm/src/key_encoding/terminfo_xcheck/`
  already verifies the FULL kf1–kf63 namespace + cursor keys (normal + application
  mode) + editing keys + the modified-key family (kLFT/kRIT/kUP/kDN/kHOM/kEND/kIC/kDC/kNXT/kPRV
  with modifier suffixes 3–7, plus kind/kri) against the pinned `ori_term.info`
  terminfo entry. Section 17 **extends** that surface with the kitty keyboard encoder,
  modifyOtherKeys encoder, and Win32 Input encoder — it does NOT rewrite it. Before
  writing new tests, read the existing cross-check tests and add the new encoder
  variants as sibling tests in the same directory. Canonical absorption policy:
  see [plans/spec-conformance/00-overview.md §Tack Absorption Strategy](./00-overview.md#tack-absorption-strategy-delivered-by-section-02).
  ```
- [ ] Do NOT modify 17.1 through 17.N checklist items — the implementer is expected to read the absorbed-infrastructure paragraph first and then execute 17.1+ against the existing tack-08 directory, not create a parallel one.
- [ ] **Validation**: `git diff plans/spec-conformance/section-17-kitty-keyboard.md` shows exactly one inserted paragraph — no other edits.

### 02.5.b Edit `plans/spec-conformance/section-23-cross-stack-regression-sweep.md`

- [ ] Read `plans/spec-conformance/section-23-cross-stack-regression-sweep.md:45-60` for the current shape of the `## Context` / `**Depends on:**` region.
- [ ] Insert the following pointer paragraph immediately after the `**Reference implementations:** see frontmatter.` line (approximate line 54) and before `**Depends on:**`:
  ```markdown
  **Absorbed from tack-conformance section 09:** This section explicitly inherits
  the cross-platform skip matrix, the flake-proofing gate (5 runs × 2 thread-modes
  × debug/release), the DA/DSR cross-validation contract against vttest menu6, the
  alloc/RSS regression checks, and the archival-in-place covenant (no `git mv`).
  Tack-conformance sections 01–08 are already `complete` on `main` — the shared
  `PtySession`, pinned `ori_term.info`, scenario framework, test/tools menu scenario
  families, GPU goldens for the visual subset, and the kf1–kf63 terminfo cross-check
  all exist. Section 23 **consumes** those artifacts as existing regression fixtures
  and wires them into the new coverage-report `--check` CI gate. The two currently
  unchecked mission criteria in `plans/tack-conformance/00-overview.md` (cross-platform
  skip cleanliness and the `./test-all.sh`/`./build-all.sh`/`./clippy-all.sh` green
  gate) are this section's responsibility. Canonical absorption policy:
  see [plans/spec-conformance/00-overview.md §Tack Absorption Strategy](./00-overview.md#tack-absorption-strategy-delivered-by-section-02).
  ```
- [ ] Do NOT modify 23.1 through 23.N — the implementer is expected to read the absorbed-from paragraph and then execute 23.1+ with the existing tack-08 suites wired into the per-stack test matrix (they're already `oriterm_core/tests/tack/` and `oriterm/src/gpu/visual_regression/tack/`).
- [ ] **Validation**: `git diff plans/spec-conformance/section-23-cross-stack-regression-sweep.md` shows exactly one inserted paragraph — no other edits.

---

## 02.6 Atomic commit + plan-audit --verify on BOTH plans

**File(s):** all eight edited files from 02.1–02.5

Non-atomic commits leave `/continue-roadmap` in a split-brain state: two plans both flagged `active` (if the tack flip lands first without the spec flip), or a `resolved` plan with a `not-started` child (if the section-09 flip lands in a separate commit), or a spec plan whose sections 17/23 reference a `_legacy-tack-mapping.md` that doesn't exist yet. The scanner cannot reason about any of these intermediate states. Section 02 MUST stage and commit all eight files in one operation.

The plan-audit verifier is the primary gate — for a zero-code section, `./test-all.sh` / `./build-all.sh` / `./clippy-all.sh` produce trivial no-op passes. The real verification is `python3 .claude/skills/plan-audit/plan-audit.py --verify --json` on BOTH plans.

- [ ] Confirm the full edit set is present in the working tree: run `git status --porcelain` and verify exactly these eight files are modified or added:
  ```
   M plans/tack-conformance/00-overview.md
   M plans/tack-conformance/index.md
   M plans/tack-conformance/section-09-verification.md
   M plans/spec-conformance/index.md
   M plans/spec-conformance/00-overview.md
   M plans/spec-conformance/section-17-kitty-keyboard.md
   M plans/spec-conformance/section-23-cross-stack-regression-sweep.md
  ?? plans/spec-conformance/catalog/_legacy-tack-mapping.md
  ```
  If any additional file shows up (for example a `catalog/` README stray or an extra blank file), STOP and investigate — the scope fence has been broken.
- [ ] **Capture pre-edit plan-audit baselines from HEAD** (BEFORE any Section 02 edits — NOT from the already-edited worktree, whether staged OR unstaged). The baseline must reflect the `HEAD` tree state the implementer started from. The detector below must catch BOTH unstaged AND staged modifications to Section 02's target files — `git diff --quiet` without `HEAD` only compares working tree vs index and misses already-staged edits. If ANY Section 02 edits exist in the working tree (staged, unstaged, or untracked), `git stash push --include-untracked` them first, run the baselines, then `git stash pop` to restore.
  ```bash
  # Detect Section 02 target-file modifications relative to HEAD (not the index).
  # `git diff HEAD --quiet -- <paths>` catches BOTH staged and unstaged changes;
  # the untracked-file check catches the new catalog/ file before it's added.
  STASHED=
  # The catalog/ directory may not exist yet (Section 01 hasn't landed AND Section 02.4 hasn't run).
  # Create it idempotently BEFORE referencing it in the pathspec, so `git stash push -- plans/spec-conformance/catalog`
  # doesn't fail with `pathspec did not match any files` (gemini empirically verified this via mock repo).
  mkdir -p plans/spec-conformance/catalog
  # Build the stash pathspec — only include catalog if it already had pre-existing tracked files or untracked content.
  # If catalog/ is truly empty (just mkdir-created by this script), omit it from the pathspec so git doesn't error.
  CATALOG_PATHSPEC=()
  if [ -n "$(git ls-files -- plans/spec-conformance/catalog)" ] \
     || [ -n "$(git ls-files --others --exclude-standard -- plans/spec-conformance/catalog)" ]; then
    CATALOG_PATHSPEC=(plans/spec-conformance/catalog)
  fi
  if ! git diff HEAD --quiet -- \
         plans/tack-conformance \
         plans/spec-conformance/index.md \
         plans/spec-conformance/00-overview.md \
         plans/spec-conformance/section-17-kitty-keyboard.md \
         plans/spec-conformance/section-23-cross-stack-regression-sweep.md \
         "${CATALOG_PATHSPEC[@]}" 2>/dev/null \
     || [ -n "$(git ls-files --others --exclude-standard -- plans/spec-conformance/catalog)" ]; then
    echo "WARNING: Section 02 target files already modified in working tree (staged or unstaged). Stash before baseline capture."
    git stash push --include-untracked -m "section-02-baseline-capture" -- \
      plans/tack-conformance \
      plans/spec-conformance/index.md \
      plans/spec-conformance/00-overview.md \
      plans/spec-conformance/section-17-kitty-keyboard.md \
      plans/spec-conformance/section-23-cross-stack-regression-sweep.md \
      "${CATALOG_PATHSPEC[@]}" || exit 1
    STASHED=1
  fi
  python3 .claude/skills/plan-audit/plan-audit.py plans/spec-conformance --verify --json > /tmp/plan-audit-spec-baseline.json 2>&1
  python3 .claude/skills/plan-audit/plan-audit.py plans/tack-conformance --verify --json > /tmp/plan-audit-tack-baseline.json 2>&1
  if [ -n "$STASHED" ]; then
    git stash pop || { echo "stash pop failed — manual recovery needed"; exit 1; }
  fi
  python3 -c "import json; d=json.load(open('/tmp/plan-audit-spec-baseline.json')); print('spec baseline MAJOR count:', d['summary']['by_severity'].get('major', 0))"
  python3 -c "import json; d=json.load(open('/tmp/plan-audit-tack-baseline.json')); print('tack baseline MAJOR count:', d['summary']['by_severity'].get('major', 0))"
  ```
  The baseline capture is MANDATORY — without it there is no way to distinguish new findings from pre-existing ones, and the gate becomes unachievable. The `HEAD` anchor in `git diff HEAD` is load-bearing: it makes the detector catch BOTH unstaged AND staged modifications, which is required because the implementer may run `git add` before baseline capture.
- [ ] Stage all eight file edits but DO NOT commit yet. Re-run both plan-audit invocations against the staged state:
  ```bash
  python3 .claude/skills/plan-audit/plan-audit.py plans/spec-conformance --verify --json > /tmp/plan-audit-spec-post.json 2>&1
  python3 .claude/skills/plan-audit/plan-audit.py plans/tack-conformance --verify --json > /tmp/plan-audit-tack-post.json 2>&1
  ```
- [ ] **Diff-based MAJOR gate (finding-set comparison, not just count)**: COUNT-ONLY comparison is insufficient — a plan that resolves 3 old MAJORs while introducing 2 new ones has `delta = -1` (count gate passes) but 2 silent regressions. Instead, build the SET of findings (keyed by `(location, check, full_message)` tuple — the full message, NOT truncated, to prevent collisions between findings whose messages diverge only after some arbitrary character limit) from baseline and post, then verify `post_set \ baseline_set` is empty (no NEW findings) regardless of how many old findings were resolved:
  ```bash
  python3 -c "
  import json, sys
  def fingerprint(f):
      # Full message — never truncated. Two distinct findings on the same location
      # with the same check but different messages (e.g. two BLOAT_RISKs citing
      # different files) MUST compare as different, otherwise the gate silently
      # collapses them into one entry and misses regressions.
      return (f.get('location',''), f.get('check',''), f.get('message',''))
  failed = False
  for plan in ('spec', 'tack'):
      base = json.load(open(f'/tmp/plan-audit-{plan}-baseline.json'))['findings']
      post = json.load(open(f'/tmp/plan-audit-{plan}-post.json'))['findings']
      base_set = {fingerprint(f) for f in base if f.get('severity') == 'major'}
      post_set = {fingerprint(f) for f in post if f.get('severity') == 'major'}
      new_findings = post_set - base_set
      resolved_findings = base_set - post_set
      print(f'{plan}: baseline_major={len(base_set)}, post_major={len(post_set)}, resolved={len(resolved_findings)}, NEW={len(new_findings)}')
      if new_findings:
          failed = True
          print(f'  NEW MAJOR findings introduced by Section 02:')
          for loc, check, msg in sorted(new_findings):
              print(f'    [{check}] {loc} — {msg[:200]}{\"...\" if len(msg) > 200 else \"\"}')
  sys.exit(1 if failed else 0)
  "
  ```
  Expected: both plans report `NEW=0`. Section 02's own known resolved findings (spec-conformance: `DEAD_PATH _legacy-tack-mapping.md` on section-02 and section-08 — 2 resolved; tack-conformance: 0 resolved) will show up in the `resolved=N` count. Any entry under `NEW MAJOR findings introduced` is a Broken Window Policy violation — STOP and fix it in this section before committing. Pure count comparison is INSUFFICIENT — the set-difference is the authoritative gate. The fingerprint uses the FULL message (not truncated) to prevent collision between distinct findings whose messages share a common prefix.
- [ ] **Section 02's known-delta resolutions** (documented so the finding-set diff is predictable): Section 02.4 creates `plans/spec-conformance/catalog/_legacy-tack-mapping.md`, which resolves TWO existing `DEAD_PATH` findings in spec-conformance (one on `section-02-tack-absorption.md`, one on `section-08-ecma-48-baseline.md`). Expected `resolved`: spec=2, tack=0. Expected `NEW`: spec=0, tack=0. Any other shape is a regression worth investigating.
- [ ] Stage all eight files explicitly by name (NOT `git add -A` — per CLAUDE.md Git Safety Protocol, stage specific files to avoid capturing stray edits).
- [ ] Use `/commit-push` (or manual `git commit` with a HEREDOC message) to create ONE commit with a message like:
  ```
  chore(plans): absorb tack-conformance into spec-conformance (Section 02)

  Mechanical plan hygiene. Zero code changes.

  - tack-conformance: flip index.md reroute status active→resolved,
    flip 00-overview.md status in-progress→complete, flip
    section-09-verification.md status not-started→complete (parent +
    all 7 subsections); add canonical supersede blockquotes to overview,
    index, and section-09 pointing at spec-conformance.
  - spec-conformance: flip index.md reroute status queued→active
    (order: 2 UNCHANGED per plan-schema.md:658 promotion algorithm);
    correct stale Tack Absorption Strategy prose in 00-overview.md
    (sections 01-08 are complete, only 09 remains and is absorbed by
    Section 23); designate the Tack Absorption Strategy block as canonical
    single-source-of-truth for absorption policy.
  - Create plans/spec-conformance/catalog/_legacy-tack-mapping.md with
    canonical header + empty table shell + seed tack-09 → spec-23 row.
  - Annotate spec-conformance sections 17 and 23 with pointers to the
    completed tack-08 / tack-09 infrastructure (no policy restatement).

  Verified with plan-audit --verify on both plans.
  ```
- [ ] Run both `plan-audit --verify --json` invocations AGAIN after the commit (to confirm the staged state matches what was committed).
- [ ] **Validation**: `git log -1 --stat` shows exactly 8 files changed. `git show --stat HEAD` confirms it. Both `plan-audit --verify` runs exit 0.

---

## 02.R Third Party Review Findings

<!-- Populated by /review-plan Phase 4 dual-source TPR (Codex + Gemini). -->
<!-- Full finding context (evidence / impact / required_plan_update / basis / confidence) lives in /tmp/ori-tpr-{8WWshCLi,2OciEdIL,NKEWo6RB,moX7wzYC}/merged.json artifacts; kept terse here per BLOAT_RISK budget. -->

### Iteration 1 findings (all resolved)

- [x] `[TPR-02-001-codex][high]` `plans/spec-conformance/section-02-tack-absorption.md:268` — **Split-brain closeout in 02.3.b.** 02.3.b forbade editing any other part of `00-overview.md`, but 02.N required updating the Quick Reference row + line-41 mission criterion at closeout. Irreconcilable.
  Resolved 2026-04-11: Lifted 02.3.b scope fence to permit mission-criterion + Quick Reference table edits. Added explicit steps to update line-41 prose (tack 01-08 complete, tack-09 absorbed by spec-23). Architecture diagram / verification taxonomy / authority ladder stay out of scope.

- [x] `[TPR-02-002-codex][high]` `plans/spec-conformance/section-02-tack-absorption.md:385` — **Atomic eight-file commit vs mandatory bug filing.** 02.6 demanded exactly 8 files changed, but 02.N mandated `/add-bug` creating a 9th (bug-tracker) file. Internal contradiction.
  Resolved 2026-04-11: Filed `BUG-07-011` during `/review-plan` Phase 4 (pre-cursor to Section 02 execution). 02.N item rewritten to verify-only (grep for `BUG-07-011`). Atomic commit stays at 8 files.

- [x] `[TPR-02-003-codex][high]` `plans/spec-conformance/section-02-tack-absorption.md:397` — **Plan-audit gate demanded zero MAJORs but baseline is 183/42.** 02.6/02.N asked for `plan-audit --verify` exit 0 with no MAJORs beyond documented false positives. Pre-existing plan-wide baseline: ~183 MAJORs spec, ~42 MAJORs tack. Gate unachievable.
  Resolved 2026-04-11: Rewrote 02.6 as diff-based: capture pre-commit MAJOR counts, re-run post-commit, assert `delta ≤ 0`. Expected: spec `-2` (Section 02.4 resolves DEAD_PATH on `_legacy-tack-mapping.md`), tack `0`. Updated 02.N + frontmatter success criterion.

- [x] `[TPR-02-001-gemini][high]` `plans/spec-conformance/section-02-tack-absorption.md:228` — **`order: 2 → 1` violates plan-schema promotion algorithm.** Section 02.3.a instructed changing `order`, but `.claude/skills/create-plan/plan-schema.md:658` literally says `order is UNCHANGED` during promotion.
  Resolved 2026-04-11: Removed the order change at 4 locations (success criteria, atomic-commit description, 02.3.a step, 02.N checklist). All now leave `order: 2` unchanged with an inline schema citation.

- [x] `[TPR-02-002-gemini][medium]` `plans/spec-conformance/section-02-tack-absorption.md:467` — **`_______` placeholder for BLOAT bug ID is deferral.** 02.N had `Filed bug ID recorded here: _______`, which deferred filing to implementation phase, violating CLAUDE.md "filing is NOT deferral — artifact must exist now."
  Resolved 2026-04-11: Same fix as `[TPR-02-002-codex]` — filed `BUG-07-011` immediately, rewrote the 02.N line to verification-only.

### Iteration 2 findings (all resolved)

- [x] `[TPR-02-004-codex][high]` `plans/spec-conformance/section-02-tack-absorption.md:403` — **Baseline captured from already-edited worktree instead of HEAD.** 02.6's pre-commit baseline step captured plan-audit output from `plan-audit.py` on the current working tree, but if the implementer has already made ANY of the 02.1-02.5 edits in the worktree, those edits would pollute the baseline — and any new finding introduced by the edits would silently become part of the "baseline" that the post-edit comparison is supposed to catch.
  Resolved 2026-04-11: Rewrote the step to detect existing modifications in Section 02's target files and `git stash push --include-untracked` them before baseline capture, then `git stash pop` after. The baseline now reflects HEAD state, not worktree state.

- [x] `[TPR-02-005-codex][high]` `plans/spec-conformance/section-02-tack-absorption.md:416` — **Count-only diff hides resolve-while-introduce regressions.** The original gate compared `post_major_count ≤ baseline_major_count`, which means a plan that resolves 3 old DEAD_PATH findings while introducing 2 new SIZE_VIOLATION findings has `delta = -1` and passes the gate — but the 2 new regressions are completely hidden. Count comparison is insufficient for a Broken Window Policy gate.
  Resolved 2026-04-11: Rewrote the gate to build `(location, check, message)` fingerprint SETS from baseline and post findings, then assert `post_set - baseline_set == empty` (no NEW findings). Resolved findings are reported as `resolved=N` but do not substitute for "no new findings." The gate now correctly catches resolve-while-introduce regressions. Expected shape: spec `NEW=0 resolved=2`, tack `NEW=0 resolved=0`.

- [x] `[TPR-02-006-codex][medium]` `plans/spec-conformance/section-02-tack-absorption.md:444` — **Stale `order 2→1` text in 02.6 commit message template.** After iteration 1 removed the `order: 2 → order: 1` instruction from the frontmatter and step text, the 02.6 example commit message still literally said "flip index.md reroute status queued→active and order 2→1 (queue promotion)". If the implementer copy-pasted the template, they would commit with a message claiming an order change that didn't happen, creating a drift between the commit history and the actual file state.
  Resolved 2026-04-11: Updated the example commit message to say "flip index.md reroute status queued→active (order: 2 UNCHANGED per plan-schema.md:658 promotion algorithm)".

### Iteration 3 findings (all resolved)

- [x] `[TPR-02-007-codex+gemini][high]` `plans/spec-conformance/section-02-tack-absorption.md:406` — **`git diff --quiet --` misses staged edits (agreement finding).** Iteration 2 used `git diff --quiet -- <paths>` as the detector for "Section 02 target files already modified in working tree". Both reviewers independently verified in a mock git repo that this form compares working tree vs **index only**; if the implementer has already staged their Section 02 edits (`git add`), the comparison returns clean and the detector falsely reports "no modifications, no stash needed" — allowing the staged edits to pollute the baseline capture.
  Resolved 2026-04-11: Changed to `git diff HEAD --quiet --` which compares working tree + index against HEAD, catching BOTH staged and unstaged modifications. Added an explicit load-bearing note explaining why the `HEAD` anchor matters. Also made the stash path list explicit (same 6 path args as the detector) so the stash covers the same surface as the detection.

- [x] `[TPR-02-008-codex][high]` `plans/spec-conformance/section-02-tack-absorption.md:21` — **Top-level success criterion still used count-based gate language after iteration 2.** Iteration 2 rewrote the executable 02.6 gate to use set-difference, but the high-level frontmatter success criterion at line 21 still said "the DIFFERENCE in MAJOR finding counts is ≤ 0 on both plans" — which is count comparison, not set comparison. A reviewer reading ONLY the success criteria would see a rule that contradicted the actual gate 400 lines below.
  Resolved 2026-04-11: Rewrote the top-level success criterion to cite `post_set - baseline_set` set-difference, explicitly note that count comparison is insufficient, and describe the expected `resolved` / `NEW` counts. Now consistent with 02.6 step wording.

- [x] `[TPR-02-009-codex][medium]` `plans/spec-conformance/section-02-tack-absorption.md:428` — **Fingerprint truncation at `message[:200]` could collide.** Iteration 2's finding-set diff helper used `(location, check, message[:200])` as the fingerprint. Two distinct findings with the same `location` and `check` but messages that diverge only after character 200 would collapse to the same tuple; both reviewers confirmed this via local repro. The collision would silently cause one of the two findings to be treated as "already in baseline" and miss its regression signal.
  Resolved 2026-04-11: Removed the `[:200]` truncation from the fingerprint computation — the full message is now part of the tuple key. Added a code comment explaining the rationale. The display-only truncation in the failure reporter was preserved (output-side, `msg[:200] + "..."`) so the printed output stays readable, but the set-comparison itself uses the full message.

### Iteration 4 findings (all resolved)

- [x] `[TPR-02-010-codex][high]` `plans/spec-conformance/section-02-tack-absorption.md:563` — **02.N checklist still used count-based `delta ≤ 0 in MAJOR count` language.** Iteration 2 and iteration 3 propagated the set-difference gate to frontmatter line 21 and to the 02.6 executable step, but 02.N's checklist item at line 563 still said "both plans report `delta ≤ 0` in MAJOR count" — a third location carrying the original count-based language that contradicts the authoritative set-diff gate. A checklist reader following only the 02.N closeout list would see a rule contradicting 02.6.
  Resolved 2026-04-11: Rewrote the 02.N checklist item to cite `post_set - baseline_set == {}` with the expected `resolved` count shape. Now consistent with frontmatter line 21 and 02.6. Comprehensive grep confirms no other `delta ≤ 0` or count-based gate language remains in Section 02 outside of iteration-2's resolution note (which references the original count-based phrasing as historical context for the finding it resolved).

- [x] `[TPR-02-011-codex+gemini][low]` `plans/spec-conformance/section-02-tack-absorption.md:469` — **Missing `### Iteration 1 findings` subheading in 02.R (agreement finding).** Both reviewers independently noticed that 02.R listed iteration-2 and iteration-3 findings under explicit `### Iteration 2 findings (all resolved)` / `### Iteration 3 findings (all resolved)` subheadings, but the first 5 iteration-1 findings started directly under the H2 with no subheading. The inconsistency broke the "organized into iteration buckets" story the review packet described.
  Resolved 2026-04-11: Inserted `### Iteration 1 findings (all resolved)` subheading immediately before the first iteration-1 finding. 02.R is now consistently organized into `### Iteration N findings (all resolved)` buckets for all 4 iterations.

### Iteration 5 findings (all resolved)

- [x] `[TPR-02-012-codex][medium]` `plans/spec-conformance/section-02-tack-absorption.md:61` — **Body header `**Status:** Not Started` contradicted frontmatter `status: in-progress`.** After iteration 1's initial fix flipped the frontmatter `status: not-started` → `in-progress` (to resolve the plan-audit STATUS_DRIFT finding from 02.R being `complete` while the section-level status was `not-started`), the body-level `**Status:** Not Started` label right under the H1 was never updated. A reader looking at only the H1 region would see contradictory state — frontmatter says in-progress, body says not started.
  Resolved 2026-04-11: Updated the body `**Status:**` label to `In Progress (Phase 4 TPR cycle complete — ready for execution)` to match frontmatter AND document the specific lifecycle phase the section is in.

- [x] `[TPR-02-013-codex][medium]` `plans/spec-conformance/section-02-tack-absorption.md:76` — **Body success criterion still used count-based `exit 0 with only known-false-positive MAJORs surviving` language.** Iterations 2/3/4 propagated the set-difference gate to frontmatter line 21 and 02.6 and 02.N, but the body-level success criteria block at line 76 still said "`plan-audit.py --verify --json` ... returns exit 0 (with only the documented known-false-positive MAJORs surviving)". A reader checking body criteria against frontmatter criteria would see a THIRD inconsistent description of the same gate.
  Resolved 2026-04-11: Rewrote the body success criterion to cite `post_set - baseline_set == {}` with the expected `resolved` count shape. Now consistent with frontmatter line 21 + 02.6 executable + 02.N closeout checklist. Four descriptions of the gate, all aligned.

- [x] `[TPR-02-014-codex][low]` `plans/spec-conformance/section-02-tack-absorption.md:102` — **Stale DRIFT claim that the blockquote template is "used in 02.1 AND 02.2 AND 02.3".** The label said the canonical supersede blockquote is used verbatim in 02.1 + 02.2 + 02.3, but 02.2 actually uses a different body-level absorption notice (the section-09-specific `⚠ This section's scope is absorbed by ...` block) and 02.3 doesn't insert the blockquote at all (02.3 handles queue promotion + stale-narrative corrections in `spec-conformance/00-overview.md`, which are structurally different edits). A reader following the template label would expect three identical insertions and get confused when 02.2/02.3 don't match.
  Resolved 2026-04-11: Updated the label to clarify: "used verbatim in 02.1.a + 02.1.b (inserted into `tack-conformance/00-overview.md` + `tack-conformance/index.md`). 02.2 inserts a DIFFERENT body-level absorption notice into `section-09-verification.md`. 02.3 does NOT insert the blockquote — 02.3 edits queue-promotion frontmatter + stale-narrative prose."

### Iteration 6 findings (all resolved)

- [x] `[TPR-02-015-codex][medium]` `plans/spec-conformance/section-02-tack-absorption.md:18` — **Missing `supersedes:` drift check in 02.N.** Frontmatter success criterion at line 18 promises that 02.N will catch `supersedes:` drift in `plans/spec-conformance/00-overview.md`, but the 02.N completion checklist had no item that actually performed that check. An implementer following 02.N would not verify the `supersedes:` frontmatter, so the drift promise was unenforced.
  Resolved 2026-04-11: Added a new 02.N bullet that runs `grep -A2 '^supersedes:' plans/spec-conformance/00-overview.md` and confirms the `plans/tack-conformance/` entry is present. This is a verification-only step (the `supersedes:` frontmatter is NOT edited by Section 02), and the file is NOT part of the 8-file atomic commit — the check catches drift introduced by a prior commit between plan-draft and Section-02 execution.

- [x] `[TPR-02-016-codex][medium]` `plans/spec-conformance/section-02-tack-absorption.md:606` — **Stale BLOAT note claimed the 02.N checklist item "mandates filing an /add-bug".** After iteration 1 filed `BUG-07-011` during the `/review-plan` Phase 4 review AND rewrote the 02.N item to be verification-only, the `known-false-positive` note at line 606 still said the 02.N item "mandates filing an `/add-bug` for this." A reader would see the known-false-positive note contradicting the (now verification-only) 02.N checklist item.
  Resolved 2026-04-11: Updated the BLOAT note to correctly describe the current state: `BUG-07-011` was filed during `/review-plan` Phase 4 (pre-cursor to Section 02 execution), and Section 02's 02.N item is now verification-only (`grep -n BUG-07-011 plans/bug-tracker/section-07-ci-build.md`). Section 02 does NOT re-file the bug and does NOT include the bug-tracker file in its 8-file atomic commit. Bug-tracker file stays separate from Section 02's commit manifest.

### Iteration 7 findings (all resolved)

- [x] `[TPR-02-017-codex][medium]` `plans/spec-conformance/section-02-tack-absorption.md:229` — **02.3.a scope fence on `spec-conformance/index.md` conflicted with the 02.N closeout Quick Reference flip.** Same class of finding as iteration-1's `[TPR-02-001-codex]` (the 02.3.b split-brain on `00-overview.md`) but for the OTHER spec file: 02.3.a said "Do NOT touch the rest of the file" for `spec-conformance/index.md`, but 02.N later required the Quick Reference row for Section 02 to flip from `Status: Not Started` to `Status: Complete` at closeout. The implementer would either violate 02.3.a's scope fence or skip the 02.N closeout requirement.
  Resolved 2026-04-11: Lifted the 02.3.a scope fence to permit the Quick Reference Section 02 row flip (same pattern as iteration 1's fix for 02.3.b on `00-overview.md`). Keyword clusters + `## How to Use` header + `## Catalog Files` cross-ref table still stay untouched. Added an explicit validation `grep '| 02 | Tack-Conformance Absorption'` that confirms the row now shows `Complete`. **Note: iteration 8 discovered the 02.3.a fix targeted the wrong surface** — see TPR-02-018-codex below for the retarget fix.

### Iteration 8 findings (all resolved)

- [x] `[TPR-02-018-codex][high]` `plans/spec-conformance/section-02-tack-absorption.md:229` — **Iteration 7's 02.3.a Quick Reference retarget pointed at a column that does NOT exist in `plans/spec-conformance/index.md`.** Iteration 7 told the implementer to "flip the Quick Reference row for Section 02 from `Not Started` to `Complete`" — but codex empirically checked the actual file and found that `plans/spec-conformance/index.md`'s Quick Reference table has columns `| ID | Title | File |` (3 columns, NO Status column). The canonical section-status surface in `index.md` is actually the per-section `**File:** ... | **Status:** Not Started` label (approximate line 47 for Section 02), NOT the Quick Reference table. The iteration 7 fix would have told the implementer to edit a column that doesn't exist.
  Resolved 2026-04-11: Rewrote 02.3.a to target the correct surface — the `**Status:** Not Started` label at approximate line 47 of `plans/spec-conformance/index.md`. Explicitly noted "this is NOT the Quick Reference table" with the 3-column structure documented inline. Updated 02.N closeout item to check the `**Status:**` label, not the non-existent Quick Reference column. Updated validation to include a `grep -B1 '**Status:** Complete' ... | grep section-02` that pins the correct surface.

- [x] `[TPR-02-019-codex][medium]` `plans/spec-conformance/section-02-tack-absorption.md:107` — **Canonical supersede blockquote claimed "every row Complete" in tack Quick Reference tables that 02.1's scope fence forbids editing.** The blockquote at lines 107-108 says "Sections 01–08 of tack-conformance are completed artifacts committed to `main` (see the Quick Reference table below — every row Complete). Section 09 (final verification + archival) is superseded by ...". But 02.1.a and 02.1.b scope-fence the tack overview/index Quick Reference tables from edit — Section 02 does NOT flip tack QR row 09 from `Not Started` to anything. So after Section 02 runs, the Quick Reference tables still literally show row 09 as `Not Started`, contradicting the "every row Complete" claim in the blockquote inserted IMMEDIATELY ABOVE those tables.
  Resolved 2026-04-11: Rewrote the blockquote to acknowledge the literal table state: "rows 01–08 show `Complete`; row 09 literally still says `Not Started` in the table, but its scope is absorbed per the line below". **Note: iteration 9 discovered this fix still made a false structural claim for `index.md`** — see TPR-02-020 below for the final retarget fix.

### Iteration 10 findings (all resolved)

- [x] `[TPR-02-022-gemini][medium]` `plans/spec-conformance/section-02-tack-absorption.md:405` — **Baseline capture's `git stash push ... catalog` pathspec fails when catalog/ directory doesn't exist.** Gemini empirically verified in a mock repo: if `plans/spec-conformance/catalog/` does not exist yet (Section 01's bootstrap hasn't landed AND Section 02.4 hasn't run), `git stash push --include-untracked -- ... plans/spec-conformance/catalog` errors with `pathspec 'plans/spec-conformance/catalog' did not match any files` and exits with code 1, crashing the baseline capture script.
  Resolved 2026-04-11: Added two defenses: (a) `mkdir -p plans/spec-conformance/catalog` before building the pathspec, making it idempotently exist; (b) a `CATALOG_PATHSPEC` bash-array built conditionally — only includes `plans/spec-conformance/catalog` if `git ls-files` reports tracked OR untracked files under it. If catalog/ exists but is truly empty (just mkdir-created), it's omitted from the stash pathspec so `git stash push` doesn't error on an empty pathspec match.

- [x] `[TPR-02-023-codex][medium]` `plans/spec-conformance/section-02-tack-absorption.md:64` — **Present-tense claim that spec-23 "already declares" absorbing tack-09 didn't distinguish frontmatter from body.** Section 02's line 64 anchor paragraph said "its scope is exactly what `spec-23:19` already declares it is absorbing". That's literally true — line 19 is in the `inspired_by` frontmatter field and says "subsumed by this section". But the language was ambiguous: a reader might assume spec-23's BODY already contains a tack-absorption pointer, when actually 02.5.b's body-level pointer insertion is what creates the body-level claim. The frontmatter declaration is pre-existing; the body pointer is new.
  Resolved 2026-04-11: Expanded the anchor paragraph to explicitly distinguish the two surfaces: "specifically, `spec-23-*.md:19` (the `inspired_by` frontmatter field, pre-existing) reads `...`. That frontmatter claim is declarative intent; the matching BODY-level inline pointer is added by 02.5.b of this section." Now the two surfaces — declarative frontmatter intent and implemented body-pointer — are separated unambiguously.

### Iteration 9 findings (all resolved)

- [x] `[TPR-02-020-codex+gemini][medium]` `plans/spec-conformance/section-02-tack-absorption.md:104` — **Blockquote's "see the Quick Reference table below" claim worked for `00-overview.md` but NOT `index.md` (agreement finding).** Both reviewers independently verified that the canonical supersede blockquote is inserted into BOTH `tack-conformance/00-overview.md` and `tack-conformance/index.md`, but the two files have structurally different presentation surfaces for section status: `00-overview.md` has a 4-column Quick Reference table (`| ID | Title | File | Status |`) where iteration-8's "rows 01-08 show Complete" claim is accurate, while `index.md` uses per-section `**File:** ... | **Status:** ...` labels inside keyword clusters AND a separate 3-column Quick Reference table (`| ID | Title | File |` with NO Status column) at the bottom. A reader of the blockquote in `index.md` would look for the Status column in the Quick Reference table BELOW and not find it.
  Resolved 2026-04-11: Rewrote the blockquote to use a file-state-agnostic claim that works in BOTH target files: "each section file's frontmatter carries `status: complete`; `plans/tack-conformance/00-overview.md`'s Quick Reference table and the per-section `**Status:**` labels in `plans/tack-conformance/index.md`'s keyword clusters both reflect this". The claim now describes what the files LITERALLY contain (per-section frontmatter + 4-column table + per-cluster labels), not a structural claim about one presentation surface.

- [x] `[TPR-02-021-codex][medium]` `plans/spec-conformance/section-02-tack-absorption.md:233` — **02.3.a validation `grep -A1 'reroute:'` did not actually show the `status:` or `order:` lines.** The validation command was `grep -A1 'reroute:' plans/spec-conformance/index.md`, which prints the `reroute:` line plus 1 line of trailing context. Since the frontmatter order is `reroute: true` → `name: ...` → `full_name: ...` → `status: active` → `order: 2`, `grep -A1` prints only `reroute:` + `name:` — the `status:` and `order:` lines the validation claims to check are NOT in the grep output.
  Resolved 2026-04-11: Replaced the `grep -A1 'reroute:'` command with `head -8 plans/spec-conformance/index.md` which prints the whole frontmatter block (reroute + name + full_name + status + order). Added a note explaining why `-A1` was insufficient so future readers don't repeat the mistake.

---

## 02.N Completion Checklist

- [ ] `plans/tack-conformance/00-overview.md` frontmatter `status: in-progress` → `status: complete` (verified with `grep '^status:' plans/tack-conformance/00-overview.md`)
- [ ] `plans/tack-conformance/00-overview.md` carries the canonical supersede blockquote immediately after its H1
- [ ] `plans/tack-conformance/index.md` frontmatter `status: active` → `status: resolved`
- [ ] `plans/tack-conformance/index.md` carries the canonical supersede blockquote immediately after its H1
- [ ] `plans/tack-conformance/section-09-verification.md` top-level frontmatter `status: not-started` → `status: complete`
- [ ] `plans/tack-conformance/section-09-verification.md` subsection array — ALL seven entries (`09.1`, `09.2`, `09.3`, `09.4`, `09.5`, `09.R`, `09.N`) flipped to `status: complete`
- [ ] `plans/tack-conformance/section-09-verification.md` body has the supersede notice after the H1, including the explicit override of 09.5's `git mv` archival instructions
- [ ] `plans/spec-conformance/index.md` frontmatter `status: queued` → `status: active`; `order: 2` LEFT UNCHANGED (per `.claude/skills/create-plan/plan-schema.md:658` promotion algorithm)
- [ ] `plans/spec-conformance/00-overview.md` line 351 stale narrative replaced (no more "sections 01-05 complete, 07-09 not yet started")
- [ ] `plans/spec-conformance/00-overview.md` item 5 at approximate line 367 replaced (no more "new work 07-09 created directly under spec-conformance")
- [ ] `plans/spec-conformance/00-overview.md` Tack Absorption Strategy block marked as canonical single-source-of-truth
- [ ] `plans/spec-conformance/catalog/_legacy-tack-mapping.md` exists with header, empty table shell, tack-09 → spec-23 seed row, and status vocabulary
- [ ] `plans/spec-conformance/section-17-kitty-keyboard.md` has the absorbed-infrastructure pointer paragraph after `**Reference implementations:**`
- [ ] `plans/spec-conformance/section-23-cross-stack-regression-sweep.md` has the absorbed-from-tack-09 pointer paragraph after `**Reference implementations:**`
- [ ] Zero files moved (`git log --diff-filter=R HEAD -1` shows no renames; `git log --follow` on any tack-conformance section file resolves via its current path unchanged)
- [ ] Zero code files modified: `git diff --name-only HEAD~1 -- '*.rs'` is empty; `git diff --name-only HEAD~1 -- 'crates/**'` is empty; `git diff --name-only HEAD~1 -- 'oriterm*/**'` is empty
- [ ] All eight file edits landed in **one atomic commit** (`git log -1 --stat` shows exactly 8 files)
- [ ] **Set-difference plan-audit gate** (see 02.6 for the full command + rationale): pre-commit baseline captured from HEAD (stashing any pre-existing Section-02 edits), post-commit re-run executed, both plans report `post_set - baseline_set == {}` (empty NEW-finding set) where each fingerprint is the `(location, check, full_message)` tuple. Expected SHAPE: spec-conformance `NEW=0 resolved=2` (Section 02.4 resolves 2 `DEAD_PATH` findings on `_legacy-tack-mapping.md`), tack-conformance `NEW=0 resolved=0`. This is NOT a count-based gate and NOT an absolute-zero-MAJOR gate — plan-audit baseline already reports ~183 MAJOR findings on spec-conformance + ~42 on tack-conformance that are pre-existing noise across the whole plan corpus and NOT introduced by Section 02. Count comparison is insufficient (a plan that resolves 3 old MAJORs while introducing 2 new ones has `count_delta = -1` but hides 2 regressions); the finding-set diff is the authoritative gate. The gate is: "Section 02 introduces no NEW MAJOR findings."
- [ ] `python3 .claude/skills/plan-audit/plan-audit.py plans/spec-conformance --verify --json` exits 0 on the post-commit run (exit code comes from the run itself, not the finding count — existing baseline MAJORs do not block exit 0)
- [ ] `python3 .claude/skills/plan-audit/plan-audit.py plans/tack-conformance --verify --json` exits 0 on the post-commit run (same rationale)
- [ ] `00-overview.md` Quick Reference table in spec-conformance shows Section 02 status as Complete
- [ ] `00-overview.md` mission success criterion at line 41 (`tack-conformance plan absorbed and superseded`) is checked `[x]`
- [ ] `plans/spec-conformance/index.md` per-section `**Status:**` label for Section 02 (approximate line 47, shape `**File:** \`section-02-tack-absorption.md\` | **Status:** Complete`) shows `Complete` — NOT the Quick Reference table (which has no Status column)
- [ ] Section 02 frontmatter `status` → `complete`
- [ ] All subsection entries in this section's frontmatter `sections:` array → `status: complete` (02.1–02.6, 02.R, 02.N)
- [ ] `./build-all.sh` green (trivial no-op — zero code changes, retained as sanity gate)
- [ ] `./clippy-all.sh` green (trivial no-op)
- [ ] `timeout 150 ./test-all.sh` green (trivial no-op)
- [ ] **`supersedes:` frontmatter drift check**: verify `plans/spec-conformance/00-overview.md` frontmatter `supersedes:` field still lists `plans/tack-conformance/` — run `grep -A2 '^supersedes:' plans/spec-conformance/00-overview.md` and confirm the entry is present. This file is NOT part of Section 02's atomic commit (no edit is needed), but if a PRIOR commit between plan draft and Section 02 execution removed the entry, Section 02's mission-criterion claim becomes false. Checking catches that drift before commit.
- [ ] `/tpr-review` passed — independent Codex + Gemini review. Expected findings are few since the section is mechanical, but any finding MUST be addressed here per CLAUDE.md "NEVER reason out of TPR findings"
- [ ] `/impl-hygiene-review last commit` passed — hygiene review of a zero-code commit passes trivially but the invocation is still required to catch accidental code drift that slipped in
- [ ] **Out-of-scope BLOAT artifact verified**: the `plans/spec-conformance/index.md` 535-line BLOAT is tracked as `BUG-07-011` in `plans/bug-tracker/section-07-ci-build.md`. This bug was filed on 2026-04-11 during the `/review-plan` Phase 4 TPR gate BEFORE Section 02 was run (to resolve the TPR-02-002-codex + TPR-02-002-gemini convergence that the 8-file atomic-commit contract conflicted with the 9th-file bug-tracker edit). Section 02's implementer VERIFIES the bug exists by running: `grep -n 'BUG-07-011' plans/bug-tracker/section-07-ci-build.md`. If the grep returns zero results, STOP and invoke `AskUserQuestion` — the precondition has been lost. Do NOT file `BUG-07-011` again in this section's commit — the bug-tracker edit is NOT part of the 8-file atomic commit.

**Known false-positive findings from `plan-audit.py --verify`:**

These are expected to survive the verify run and do NOT block section completion:

- **`DEAD_PATH` on `plans/spec-conformance/catalog/_legacy-tack-mapping.md`** — Phase 1 `plan-audit` reports this against both sections 02 and 08 because the file is referenced before it exists. It is RESOLVED the moment Section 02.4 commits the file. If the post-commit verify run STILL reports this, Section 02.4's file creation did not actually land — STOP and investigate. (Section 08 continues to reference the file after Section 02 creates it, so the section-08 finding also clears once the Section 02 commit lands.)
- **`SIZE_VIOLATION` (~104 top-level items, heuristic cap 20)** — the plan-audit SIZE_VIOLATION heuristic reports this section as oversized. It is acceptable provided every item is load-bearing. The section's scope is deliberately broader than the previous draft because Phase 2 reviewers surfaced 10 consensus findings and Phase 4 TPR added 11 more (5 iteration-1 + 3 iteration-2 + 3 iteration-3) that together demanded wider file coverage (two tack files, one tack section, four spec files, one new catalog file — all under one atomic commit). Splitting into 02a/02b would re-introduce the atomicity violation the section is designed to prevent: any non-atomic commit leaves `/continue-roadmap` in a split-brain state. The heuristic is advisory; the atomicity requirement is structural.
- **`BLOAT_RISK` on `plans/spec-conformance/00-overview.md` (924 lines)** — the overview is intentionally large because it is the SSOT for mission criteria, architecture, verification taxonomy, authority ladder, cross-section interaction notes, and (after Section 02) the canonical Tack Absorption Strategy block. Trimming it would move SSOT knowledge into satellite files, re-creating the LEAK:scattered-knowledge finding Section 02 exists to fix. Acceptable. File an `/add-bug` (`medium`, `plan-hygiene`) only if the overview crosses 1100 lines — not here.
- **`BLOAT_RISK` on `plans/spec-conformance/index.md` (535 lines)** — this IS the BLOAT the previous draft declined to fix. Section 02 explicitly scopes it out (it would double-scope Section 02 with index-trimming work on the same file Section 01 bootstraps to). The `/add-bug` for this was filed as `BUG-07-011` during `/review-plan` Phase 4 (BEFORE Section 02 execution, per TPR-02-002-codex + TPR-02-002-gemini convergence). Section 02's 02.N checklist item "Out-of-scope BLOAT artifact verified" now reads as VERIFICATION-ONLY (`grep -n 'BUG-07-011' plans/bug-tracker/section-07-ci-build.md`) — Section 02 does NOT re-file the bug and does NOT include the bug-tracker file in its 8-file atomic commit. Filing via `/add-bug` is NOT "tracked for later" — it is a concrete tracked artifact that `/review-bugs` and `/fix-next-bug` will pick up.
- **`BLOAT_RISK` on `plans/spec-conformance/section-02-tack-absorption.md` itself (currently ~600 lines, over the 500-line heuristic ceiling)** — Phase 4 TPR iteration 1 added ~60 lines of findings + updated step instructions; iteration 2 added ~30 lines; iteration 3 added ~30 more; iteration 4 added ~10 more (1 HIGH carry-over fix + 2 LOW cosmetic agreement). Total ~130 lines of TPR-driven growth. 02.R is kept in terse one-line-per-resolution form (full finding context lives in `/tmp/ori-tpr-{8WWshCLi,2OciEdIL,NKEWo6RB,moX7wzYC}/merged.json` — the TPR scratch dirs — and in the git history of this file). Further trimming would remove the step-by-step instructions and pre-flight checks that the Phase 4 reviewers specifically requested. The 500-line limit is derived from `code-hygiene.md §File Organization` which scopes it to Rust source files (`excluding tests.rs`); for plan markdown it is a plan-audit advisory, not a mandatory ceiling. Section 02 is a one-shot plan-hygiene section (executed once, then frozen) so the maintenance-burden argument behind the 500-line rule does not apply — nobody is expected to edit this file after Section 02 lands. Original split threshold of 600 lines is revised to 700 lines (splitting 02.R into a sibling file introduces cross-file reference complexity that plan-audit flags as new DEAD_PATHs, which defeats the point of the split). If the file grows past 700 lines due to further TPR iterations, split `02.R` into a sibling `section-02-tack-absorption-tpr.md` file (canonical pattern) — do NOT split subsections 02.1-02.6 because the atomic commit contract depends on them remaining in one section.
- **`BLOAT_RISK` on `.claude/skills/create-plan/plan-schema.md` (904 lines)** — Section 02 cites plan-schema.md by full path (`.claude/skills/create-plan/plan-schema.md:609-613` etc.) to satisfy plan-audit's DEAD_PATH resolution check. Plan-audit then walks the referenced file's line count and flags it as BLOAT_RISK. This is meta-noise — plan-schema.md is a referenced-only authority file, not something Section 02 owns, modifies, or maintains. The plan-audit heuristic should not apply to authority files referenced by other plans, but until the heuristic is fixed this is an accepted false-positive. Do NOT attempt to trim plan-schema.md as part of Section 02's scope.
- **`DUPLICATE_ITEM`** boilerplate flag on `Section frontmatter status → complete` — appears in every section's completion checklist and is a known Phase 1 false-positive (the auditor counts boilerplate as duplicate across sections).

**Exit Criteria:** Both `plans/tack-conformance/` and `plans/spec-conformance/` are in a consistent, plan-audit-legal state after a single atomic commit. `plans/tack-conformance/` is effectively `resolved` (index + overview + section-09 all terminal), its remaining work is explicitly reassigned to spec-conformance Section 23, and its files remain in place for citation stability. `plans/spec-conformance/` is now the active reroute, its `00-overview.md` Tack Absorption Strategy block is the canonical single-source-of-truth, and its sections 17 and 23 carry pointers that prevent implementers from reinventing the tack-08 infrastructure. The `_legacy-tack-mapping.md` file exists as the empirical traceability artifact sections 08/17/23 will append to. Zero code changes. Plan-audit `--verify --json` passes on both plans. The `tack-conformance plan absorbed and superseded` mission criterion in `plans/spec-conformance/00-overview.md` at approximate line 41 is checked.
