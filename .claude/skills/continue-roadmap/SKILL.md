---
name: continue-roadmap
description: Resume work on the ori_term rebuild roadmap, picking up where we left off
argument-hint: "[section]"
---

# Continue Roadmap

Resume work on the ori_term rebuild roadmap, picking up where we left off.

## Usage

```
/continue-roadmap [section]
```

- No args: Auto-detect first incomplete item sequentially (01 → 02 → ...)
- `section-5`, `5`, or `gpu`: Continue Section 5 (Window + GPU Rendering)
- Any section number or keyword: Use `plans/roadmap/index.md` to find sections by keyword

## Finding Sections by Topic

Use `plans/roadmap/index.md` to find sections by keyword. The index contains searchable keyword clusters for each section.

---

## Workflow

### ABSOLUTE RULE: Commits via /commit-push ONLY

**NEVER run `git add`, `git commit`, or any direct git commit command.** All commits MUST go through the `/commit-push` skill. This applies everywhere in this workflow: clean-tree gates, subsection pauses, after-work commits, final commits. Invoke `/commit-push` via the Skill tool.

### Step -1: Read CLAUDE.md (ABSOLUTE FIRST — NO EXCEPTIONS)

**Before doing ANYTHING else**, use the Read tool to read the ENTIRE CLAUDE.md file — every single line, top to bottom:

```
Read file: CLAUDE.md
```

**This is a BLOCKING requirement.** You MUST issue a Read tool call for CLAUDE.md and process every line of the result. Do not skip, skim, summarize, or partially read. Do not assume you already know the contents from earlier in the conversation — the file may have changed. Do not rely on CLAUDE.md content loaded into system context — issue the Read tool call explicitly. The rules in CLAUDE.md govern ALL behavior in this command. Proceed to Step 0 only after reading the complete file via the Read tool.

### Step -1B: Re-read CLAUDE.md Between Tasks (MANDATORY)

**Every time you finish a task and start the next one** (e.g., completing one checklist item and moving to the next, finishing TPR triage and starting implementation, switching between subsections), you MUST re-read CLAUDE.md in full via the Read tool before beginning the new task:

```
Read file: CLAUDE.md
```

This is not optional. Context window compression can silently drop CLAUDE.md rules that were loaded earlier. A fresh read ensures every rule is active in your working context. This applies within a single `/continue-roadmap` session — not just at the start.

### Step 0: Check for Active Reroute

The scanner automatically detects reroutes from `plans/*/index.md` frontmatter. Each plan's `index.md` has:

```yaml
reroute: true       # or parallel: true
name: "Short Name"
full_name: "Full Plan Name"
status: active       # active | queued | resolved
order: 1             # queue priority (lower = promoted first, default 999)
```

The scanner outputs an `=== REROUTES ===` block at the top with `[ACTIVE reroute]` and `[queued reroute]` lines.

**If an ACTIVE reroute exists:**

1. **Read the rerouted plan** — its `index.md` and `00-overview.md`
2. **Run the scanner on the rerouted plan**:
   ```bash
   .claude/skills/continue-roadmap/roadmap-scan.sh plans/<rerouted-plan>
   ```
3. **Follow the rerouted plan's execution order** — use the plan's recommended section order, not the roadmap's
4. **Present the rerouted plan status** to the user, making clear this is a reroute from the main roadmap
5. **When the rerouted plan is complete** — update its frontmatter `status: resolved`, then promote queued reroutes (see below)

**When an ACTIVE reroute completes (promotion protocol):**

1. Update the completed plan's frontmatter: `status: resolved`
2. **Verify frontmatter consistency** (see "Plan Completion Frontmatter Gate" below)
3. **Move the plan to `plans/completed/`**: `git mv plans/<plan-dir> plans/completed/<plan-dir>`
4. If queued reroutes exist, pick the one with the lowest `order` value:
   - Update its frontmatter: `status: active`
   - Inform the user that the next reroute has been promoted to active
5. If no queued reroute exists, inform the user that normal roadmap work resumes

**Active parallel plans** (`parallel: true`) run alongside the roadmap — they don't block normal work. Only `reroute: true` plans with `status: active` take priority.

**Do NOT skip reroutes.** They exist because continuing normal roadmap work without completing the rerouted plan would compound architectural debt.

**Do NOT skip the queue.** Queued reroutes must complete before resuming normal roadmap work.

### Step 1: Run the Scanner

Run the roadmap scanner script to get current status:

```bash
.claude/skills/continue-roadmap/roadmap-scan.sh plans/roadmap
```

This outputs:
- Reroute status block (if any active/queued reroutes detected from `plans/*/index.md` frontmatter)
- One line per section: `[done]` or `[open]` with progress stats
- Detail block for the **first incomplete section**: subsection statuses (with blocked counts), first 5 **unblocked** items, blocker summary, and blocker chain

### Step 1.5: Fix Stale Frontmatter

The scanner detects frontmatter/body mismatches (`!! MISMATCH` annotations) at both section and subsection level. **When mismatches are found, fix them immediately** — do not proceed to the focus section with stale data.

**Auto-fix rules (no user prompt needed):**

1. **`frontmatter=complete` but unchecked items exist** — Set frontmatter to `in-progress` (or `not-started` if 0 checked)
2. **`frontmatter=not-started` but checked **non-TPR** items exist** — Set frontmatter to `in-progress` (or `complete` if 0 unchecked). TPR checkboxes (items in `## X.R Third Party Review Findings`) do NOT count for promoting `not-started` — resolved review findings are not implementation progress.
3. **`frontmatter=in-progress` but all items checked** — Set frontmatter to `complete`
4. **`frontmatter=in-progress` but 0 items checked** — Set frontmatter to `not-started`
5. **Subsection status stale** — Apply the same rules per subsection, then recalculate section status
6. **Section status stale after subsection fix** — If all subsections are `complete`, set section to `complete`
7. **TPR consistency** — If `third_party_review.status: findings` but no unchecked TPR items exist, set to `resolved`. If unchecked TPR items exist but `third_party_review.status` is `none` or `resolved`, set to `findings`. If section `status` is `complete` but `third_party_review.status: findings`, set section to `in-progress`.

**When to ask instead of auto-fix:**

- If a section shows `complete` but has many unchecked items (>5), use AskUserQuestion — the checkboxes may be stale rather than the frontmatter
- If items are marked `[ ]` but have a `<!-- blocked:` or `<!-- deferred:` comment indicating they were intentionally left open, call them out and ask

After fixing, briefly note what was corrected.

### Step 1.6: Schema Compliance Check

The plan schema lives at `.claude/skills/create-plan/plan-schema.md`. When working on a plan (reroute or roadmap section), verify the focus section's frontmatter conforms to the schema:

**Required frontmatter fields for section files:**
- `section` — section number (string or number)
- `title` — section title
- `status` — `not-started | in-progress | complete`
- `reviewed` — `true | false`
- `goal` — one-line measurable goal
- `sections` — array of `{ id, title, status }` subsection entries
- `third_party_review` — `{ status: none | findings | resolved, updated: date | null }`

**Auto-fix:** If a field is missing or uses a non-standard value (e.g., `status: done` instead of `status: complete`), fix it silently. If the structure is fundamentally wrong, note it and fix.

This check is lightweight — only verify the focus section and its parent overview/index.

### Step 1.7: Unreviewed Plan Gate

After the scanner identifies the focus section, **check its frontmatter for `reviewed: false`**. This flag means the section's assumptions have NOT been validated against the current codebase.

**If `reviewed: false` is present on the focus section:**

1. **STOP** — do not begin implementation
2. **Warn the user** via AskUserQuestion:
   - "Section N has `reviewed: false` — its assumptions haven't been validated against the current codebase (which may have changed during earlier section work). Implementing an unreviewed plan risks wasted work."
   - Options: **Run /review-plan now (Recommended)** | **Proceed anyway** | **Pick a different section**
3. **If user chooses to review**: Run `/review-plan` on the **specific section file**. After review agents confirm accuracy, flip to `reviewed: true`.
4. **If user chooses to proceed**: Continue, but note the risk. Leave `reviewed: false`.

**If `reviewed: false` is NOT present** (field absent or `reviewed: true`), proceed normally.

### Step 1.9: Third Party Review Triage Gate

After identifying the focus section, **check its frontmatter for `third_party_review.status: findings`**. This means an external reviewer has recorded unresolved findings in the section's `## {NN}.R Third Party Review Findings` block.

**If `third_party_review.status` is `findings`:**

1. **STOP** — do not begin new implementation work
2. **Read all unchecked items** in the `## {NN}.R Third Party Review Findings` block
3. **Triage findings in priority order** (high → medium → low):
   - For each finding, validate it against the codebase and current plan
   - **CRITICAL: You MUST NOT dismiss a TPR finding because it is "not related" to the current plan or work.** Per CLAUDE.md: there is no "unrelated", "pre-existing", or "out of scope." If a TPR finding identifies a real issue in the codebase, it must be accepted and addressed. The only valid reason to reject a finding is that it is factually incorrect.
   - **Accepted findings**: Add or update concrete implementation tasks in the relevant subsection(s). Mark the review item resolved:
     ```markdown
     - [x] `[TPR-02-001][high]` `file:line` — Description.
       Resolved: Validated and integrated into 02.2 and 02.5 on YYYY-MM-DD.
     ```
   - **Rejected findings**: Do not delete — mark resolved with rejection rationale. A finding may ONLY be rejected if it is factually incorrect:
     ```markdown
     - [x] `[TPR-02-002][medium]` `file:line` — Description.
       Resolved: Rejected after validation on YYYY-MM-DD. [Rationale].
     ```
4. **After all findings are triaged**:
   - Update `third_party_review.updated` to today's date
   - If ALL findings were rejected: set `third_party_review.status` to `resolved`
   - If ANY accepted findings created new `[ ]` items: **keep** `third_party_review.status: findings` — transitions to `resolved` only when the accepted tasks are complete
5. **Continue** to normal implementation only after all open review findings are triaged

**If `third_party_review.status` is `none` or `resolved`**, proceed normally.

### Step 1.92: Bug Tracker Check

After identifying the focus section, **check the bug tracker for relevant known bugs** in the area being worked on.

Read the mapped bug-tracker section file(s) and check for `- [ ]` items.

**If `critical` bugs exist in the mapped area:**

1. **STOP** — present them to the user as blockers
2. List each critical bug with its ID, title, and repro
3. Use AskUserQuestion:
   - **Fix critical bugs first (Recommended)** — address these before starting new work
   - **Proceed anyway** — user accepts the risk

**If `high` bugs exist:**

1. **Mention them** — "There are N high-severity bugs in this area you may want to address"
2. Continue to the next step — high bugs are informational, not blocking

**If only `medium`/`low` or no bugs exist**, proceed normally.

### Step 1.95: Clean Working Tree Gate

Before starting implementation work, **check for pending changes** in the working tree:

```bash
git status --short
```

**If the working tree is clean**, proceed to Step 2.

**If there are pending changes:**

1. **STOP** — do not proceed to implementation work
2. **Show a brief summary** of what's pending
3. **Use AskUserQuestion** with these options:
   - **Run /commit-push (Recommended)** — commit and push all pending changes before continuing
   - **Proceed anyway** — continue with a dirty working tree

**Why:** A clean working tree ensures the next section's work is cleanly separable in git history.

### Step 2: Determine Focus Section

**If argument provided**, find the matching section file and skip to Step 3.

**If no argument provided**, check the **Priority Queue** in `plans/roadmap/index.md` first. The first incomplete priority section becomes the focus. If all priority sections are complete (or the queue is empty), fall back to the scanner's `=== FOCUS ===` section.

#### Dependency Skip Rule

Only skip a section if **all** of these are true:
1. The section has explicit dependencies listed in the Dependency DAG
2. One or more of those dependencies has `status: not-started` or `status: in-progress`
3. The incomplete work in the current section actually **requires** the blocker

If a section has some blocked items and some unblocked items, **work the unblocked items** rather than skipping.

#### Blocker References (2-Way)

When you discover a blocker, you **must** add a 2-way reference:

1. **On the blocked item** — Add `<!-- blocked-by:X -->` where X is the blocker section number
2. **On the blocker item** — Add `<!-- unblocks:X.Y -->` where X.Y is the blocked subsection ID

**Tag format**: Machine-readable, no free text.
- `<!-- blocked-by:18 -->` — blocked by Section 18
- `<!-- blocked-by:18 --><!-- blocked-by:3 -->` — blocked by multiple sections
- `<!-- unblocks:5.3 -->` — unblocks subsection 5.3

**Both references must be added at the same time.**

**Parent inheritance**: Nested `- [ ]` items inherit their parent's blocker. Only tag the top-level item.

### Step 2.5: Blocker Chain Resolution

When the scanner shows blocked items, analyze the blocker chain:

1. Read the **Blocker summary** and **Blocker chain** from scanner output
2. Classify each blocker:
   - **READY**: All its dependencies are `[complete]` — can start implementing now
   - **IN PROGRESS**: Section already being worked on
   - **WAITING**: Has incomplete dependencies — blocked itself
3. Build and present a blocker tree:
   ```
   Blocker Tree:
   ├─ Section 07: 2D UI Framework [not-started] — READY (deps satisfied: 06 [complete])
   │  └─ blocks 12 items here
   ├─ Section 03: Cross-Platform [in-progress, 40%] — IN PROGRESS
   │  └─ blocks 5 items here
   └─ Section 05: Window + GPU [not-started] — WAITING on Section 04
      └─ blocks 3 items here
   ```

### Step 3: Load Section Details

Read the focus section file. Extract:

1. **Section title** from the `# Section N:` header
2. **Completion stats**: from scanner output
3. **First incomplete item**: The first `- [ ]` line and its context
4. **Recently completed items**: Last few `- [x]` items for context

### Step 4: Present Summary

Present to the user:

```
## Section N: [Name]

**Progress:** X/Y items complete (Z%)
**Actionable:** A unblocked, B blocked (by N sections)

### Recently Completed
- [last 2-3 completed items]

### Next Up (Unblocked)
**Subsection X.Y: [Subsection Name]**
- [ ] [First unblocked incomplete item]
  - [sub-items if any]

### Blockers
[Blocker tree from Step 2.5]

### Remaining in This Section
- [count of remaining unblocked items]
- [count of blocked items]
```

### Step 5: Ask What to Do

Use AskUserQuestion with options:

**When there are unblocked items:**
1. **Start next task (Recommended)** — Begin implementing the first unblocked item
2. **Show task details** — See more context about the task
3. **Pick different task** — Choose a specific unblocked task from this section
4. **Tackle a blocker** — Work on a READY blocker to unblock items
5. **Switch sections** — Work on a different section

**When ALL remaining items are blocked:**
1. **Tackle deepest ready blocker (Recommended)** — Work on the READY blocker that unblocks the most items
2. **Show blocker details** — See what the blocker requires
3. **Switch sections** — Work on a different section

### Step 5.5: Subsection Pacing

**After the user chooses to start work**, ask how they want to pace the section using AskUserQuestion:

1. **Full section** — Run all subsections continuously without pausing
2. **Subsection-by-subsection (Recommended)** — Pause after completing each subsection for review

If "Subsection-by-subsection", after completing each subsection's checkboxes, present a brief status and use AskUserQuestion:
1. **Continue to next subsection** — Proceed
2. **Run /commit-push and continue** — Commit current work, then proceed
3. **Stop here** — End work for now (run `/commit-push` first if there are changes)

### Step 6: Execute Work

Based on user choice:
- **Start next task**: Begin implementing, following the Implementation Guidelines below
- **Show task details**: Read relevant section content, explore codebase, check reference repos
- **Pick different task**: List all unblocked incomplete items, let user choose
- **Tackle a blocker**: Switch to the blocker section. When complete, return to update blocked items.
- **Switch sections**: Ask which section to switch to

---

## Implementation Guidelines

### ZERO DEFERRAL — Implement, Don't Document For Later

**If you understand a task well enough to write an implementation plan, you implement it.** The following are ALL banned:

- Labeling an item "requires architectural change" and skipping it
- Moving items to a different roadmap section "for later"
- Writing "deferred to roadmap X.Y" on an item
- Marking a section complete while unchecked items remain
- Describing an implementation approach in prose instead of implementing it
- Labeling items "lower priority" or "bonus" as justification for skipping

**The ONLY valid reason to not implement an item is if you literally cannot** (missing information, blocked on external dependency). In that case, use `AskUserQuestion` immediately.

### Plan Boundary Integrity

**Fixes must not silently cross section boundaries.** When implementing a task in Section X:

1. **Before modifying code**: Check if the code is referenced by another section's tasks
2. **If cross-section modification is needed**: Update the other section's plan to reflect the change
3. **After completing a task**: Verify no changes require updates to other sections

### Scope Rule: ALL Checkboxes in the Section Are In Scope

**Every `- [ ]` checkbox within the current section is part of that section's work — no exceptions.** This includes:

- **Testing** checkboxes (unit tests, integration tests, visual regression tests)
- **Build verification** checkboxes (clippy, cross-compilation)
- **Platform-specific** checkboxes (Windows, Linux, macOS)
- Any other sub-item checkboxes nested under a parent item

**A subsection is only complete when ALL its checkboxes are checked.**

### Verification Rule: Empty Checkboxes Must Be Verified

**Never check off a `[ ]` item without verifying it.** Before marking any item `[x]`:

1. **Read the relevant code** — confirm the feature/test actually exists
2. **Run the test** — if it's a test item, run it and confirm it passes
3. **Check the plan** — if it's an implementation item, verify behavior matches the plan

### Skills Are Tools — Run Them, Don't Reimplement Them

**When a plan item says to run a skill (e.g., "Run `/review-plan`"), invoke it using the `Skill` tool.** Do NOT manually read the skill's SKILL.md and re-execute its steps yourself.

### Before Writing Code

1. **Read the plan** — Understand exactly what the section requires
2. **Check reference repos** — Look in `~/projects/reference_repos/console_repos/` for established patterns
3. **Read the old code** — Check `_old/src/` for the prototype implementation (reference only)
4. **Explore the codebase** — Use Explore agent to find where features should be implemented

### While Writing Code

1. **Follow existing patterns** — Match the style of surrounding code
2. **Follow CLAUDE.md coding standards** — Error handling, unsafe rules, linting, module organization
3. **Add tests** — Unit tests for `oriterm_core`/`oriterm_ui`, integration tests for `oriterm`
4. **Check off items** — Update section file checkboxes as you complete sub-items

### After Writing Code

1. **Run checks** — `./clippy-all.sh` and `./test-all.sh` to verify everything passes
2. **Build** — `./build-all.sh` to verify cross-compilation
3. **Check plan boundary integrity** — did this fix modify code referenced by another section?
4. **Update section file** — Check off completed items with `[x]`
5. **Update YAML frontmatter** — See "Updating Section File Frontmatter" below
6. **Run `/commit-push`** — NEVER commit directly with `git commit`. Always use the `/commit-push` skill.
7. **Run `/tpr-review` after section completion — MUST PASS CLEAN** — When ALL checkboxes in a section are checked and the section is about to be marked `complete`, run `/tpr-review` for an independent Codex review. **The TPR must come back completely clean before the section can be closed out.** If `/tpr-review` surfaces ANY findings: (1) triage them through Step 1.9, (2) fix all accepted findings, (3) **re-run `/tpr-review`** to confirm clean. Repeat until the review passes with zero unresolved findings. A section CANNOT be marked `complete` until a clean `/tpr-review` pass is achieved. **This rule is definitive and non-negotiable.**

---

## Gap Detection and Escalation Protocol

When implementing a roadmap item and you discover that a required feature is missing or incomplete:

### STOP — Do Not Work Around

**Never silently substitute a workaround.** The workaround hides the gap from the user and from the roadmap.

### Flag Immediately

Use AskUserQuestion to escalate:

1. **What's missing**: Describe the exact gap
2. **Where it's documented** (or not): Check roadmap for the feature
3. **Impact**: What current work is blocked or degraded
4. **Recommendation**: Fix now (if small, < 30 min), track and fix later (if large), or ask user

### Track in Roadmap

If the gap is deferred:
1. Add a `<!-- gap: description -->` comment on the blocked roadmap item
2. Add a `- [ ]` checkbox for the missing feature in the appropriate section
3. Add blocker references (`<!-- blocked-by:X -->` / `<!-- unblocks:X.Y -->`)

---

## Updating Section File Frontmatter

Section files use YAML frontmatter for machine-readable status tracking. **You must keep this in sync** when completing tasks.

### Frontmatter Structure

```yaml
---
section: 5
title: Window + GPU Rendering
status: in-progress
reviewed: true
tier: 2
goal: Open a frameless window...
sections:
  - id: "5.1"
    title: Render Pipeline Architecture
    status: complete
  - id: "5.2"
    title: winit Window Creation
    status: in-progress
---
```

### Status Values

- `not-started` — No checkboxes completed
- `in-progress` — Some checkboxes completed, some pending
- `complete` — All checkboxes completed

### When to Update

**After completing task checkboxes**, update the frontmatter:

1. **Update subsection status** based on checkboxes under that `## X.Y` header:
   - All `[x]` → `status: complete`
   - Mix of `[x]` and `[ ]` → `status: in-progress`
   - All `[ ]` → `status: not-started`

2. **Update section status** based on subsection statuses:
   - All subsections complete → `status: complete`
   - Any subsection in-progress → `status: in-progress`
   - All subsections not-started → `status: not-started`

3. **Update `third_party_review` frontmatter** if the TPR block was modified:
   - All TPR items resolved (checked) → `third_party_review.status: resolved`
   - Unchecked TPR items remain → `third_party_review.status: findings`
   - No TPR items (`- None.`) → `third_party_review.status: none`
   - A section cannot be `complete` while `third_party_review.status: findings`

---

## Verification/Audit Workflow

When auditing roadmap accuracy (verifying status rather than implementing features):

### Step 1: Compare Frontmatter to Body

Check if frontmatter matches checkbox state.

### Step 2: Test Claimed Status

Don't trust checkboxes blindly. Verify actual implementation:

1. **For `[x]` items**: Confirm feature works
2. **For `[ ]` items**: Confirm feature is missing
3. **Document discrepancies**

### Step 3: Update Body Checkboxes

Fix checkboxes to match verified reality.

### Step 4: Update Frontmatter Immediately

**Never leave frontmatter stale.** Recalculate statuses from checkboxes.

---

## Checklist

When completing a roadmap item:

- [ ] Read plan section thoroughly
- [ ] Check reference repos for established patterns
- [ ] Implement feature
- [ ] Add unit tests and/or integration tests
- [ ] Run `./clippy-all.sh` — no warnings
- [ ] Run `./test-all.sh` — all tests pass
- [ ] Run `./build-all.sh` — cross-compilation succeeds
- [ ] Update section file:
  - [ ] Check off completed items with `[x]`
  - [ ] Update subsection `status` in YAML frontmatter
  - [ ] Update section `status` in YAML frontmatter
- [ ] Run `/tpr-review` — MUST PASS CLEAN. If findings surface: fix, re-run, repeat until clean.
- [ ] Update parent plan files (if section status changed):
  - [ ] Update `00-overview.md` Quick Reference table
  - [ ] Update `index.md` section status
  - [ ] If plan complete: run "Plan Completion Frontmatter Gate", then move to `plans/completed/`
- [ ] Run `/commit-push` — NEVER commit directly with `git commit`

---

## Plan Completion Frontmatter Gate

**When ALL sections of a plan are complete**, run this gate before archival:

1. **Verify `00-overview.md` frontmatter**: `status` must be `complete`
2. **Verify `index.md` frontmatter** (if it exists): `status` must be `resolved`
3. **Verify Quick Reference table**: every section row must show `Complete`
4. **Scan for stale `Not Started` or `In Progress`**: grep the overview and index — fix if found

If any check fails, fix the issue first, then proceed.

## Plan Archival Protocol

After the frontmatter gate passes:

1. **Move the plan directory**: `git mv plans/<plan-dir> plans/completed/<plan-dir>`
2. **Verify the move**: `ls plans/completed/<plan-dir>/` to confirm files are present
3. **Commit**: use `/commit-push` with a message like `chore: archive completed plan <plan-name>`

---

## Maintaining the Roadmap Index

**IMPORTANT:** When adding new items to the roadmap, update `plans/roadmap/index.md`:

1. **Adding items to existing section**: Add relevant keywords to that section's keyword cluster
2. **Creating a new section**: Add a new keyword cluster block and table entry
3. **Removing/renaming sections**: Update the corresponding entries

The index enables quick topic-based navigation. Keep keyword clusters concise and include both formal names and common aliases.
