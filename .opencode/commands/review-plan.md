---
description: Review a plan for problems -- technical accuracy, completeness, hygiene compliance, and crate/module dependency ordering
---

# Review Plan Command

Read a plan, cross-reference it against the codebase and hygiene rules, then fix problems directly via 4 sequential review agents. Report findings as a verdict.

## Third Party Review Semantics

Plan sections may include this frontmatter block:

```yaml
third_party_review:
  status: none|findings|resolved
  updated: YYYY-MM-DD|null
```

This tracks whether the section's `## {NN}.R Third Party Review Findings` block has unresolved findings.

## Usage

```
/review-plan <plan-path>
```

- `plan-path`: **Required.** Path to the plan directory or a specific plan file (e.g., `plans/mux-flatten/`, `plans/roadmap/section-05.md`).
  - If a directory: reviews all files in the directory
  - If a single file: reviews that file (and reads siblings for context)

**Arguments:** `$ARGUMENTS`

## Workflow

### Step 1: Read the Plan

Read the plan file(s) specified in `$ARGUMENTS`. If the path doesn't exist, report the error and stop.

- If a directory, read all `.md` files: `index.md`, `00-overview.md`, and all `section-*.md` files
- If a single file, read it plus any sibling plan files for context

### Step 2: Load Hygiene Rules

Read the following rule files. These are the source of truth for the review:

**Implementation Hygiene Rules** (`.claude/rules/impl-hygiene.md`):
!`cat .claude/rules/impl-hygiene.md`

**Code Hygiene Rules** (`.claude/rules/code-hygiene.md`):
!`cat .claude/rules/code-hygiene.md`

**Test Organization Rules** (`.claude/rules/test-organization.md`):
!`cat .claude/rules/test-organization.md`

### Step 3: Initial Assessment

Before launching agents, do a quick read-through and report to the user:
- Plan name and scope
- Number of sections/files
- Note: "Running 4 sequential review passes..."

### Step 4: Sequential Independent Review (4 Agents)

Run **4 review agents in sequence** (NOT parallel) using the Task tool. Each agent:

- Receives **only the plan files** -- no conversation context, no reasoning behind the plan
- Is instructed to **read the plan, review it, and edit the files directly** to fix issues
- Sees edits made by all previous agents (because they run sequentially)

This creates an iterative refinement pipeline: each reviewer builds on the last.

**IMPORTANT**: Run these agents ONE AT A TIME. Wait for each to complete before starting the next.

#### Agent 1: Technical Accuracy Review

Spawn a subagent (via Task tool) with the following prompt (substitute `{plan_dir}` with the actual plan directory path):

```
You are reviewing an existing plan for ori_term (a GPU-accelerated terminal emulator in Rust) at {plan_dir}/.

INSTRUCTIONS:
1. Read ALL files in {plan_dir}/ (index.md, 00-overview.md, and all section-*.md files)
2. Cross-reference every technical claim against the actual codebase:
   - Do referenced files, types, functions, modules exist?
   - Are crate/module dependency assumptions correct? (oriterm_core is the library crate, oriterm is the binary crate that depends on it)
   - Are described code patterns accurate?
   - Are references to external crates (wgpu, winit, vte, fontdue, etc.) correct?
3. Check claims against reference repos in ~/projects/reference_repos/console_repos/ where relevant (Alacritty, WezTerm, Ghostty patterns)
4. For every inaccuracy found, EDIT the plan files directly to fix them
5. If a section references nonexistent code paths or wrong file locations, correct them
6. Add a brief comment near each fix: <!-- reviewed: accuracy fix -->
7. When reviewing Third Party Review findings, you MUST NOT dismiss findings because they are "unrelated", "out of scope", or "pre-existing". Accept any finding that identifies a real issue in the codebase. Reject only findings that are factually incorrect.

You may add missing sections, expand scope, or restructure if the plan is genuinely incomplete.
After editing, list what you changed and why.
```

#### Agent 2: Completeness & Gap Review

```
You are reviewing an existing plan for ori_term (a GPU-accelerated terminal emulator in Rust) at {plan_dir}/.

INSTRUCTIONS:
1. Read ALL files in {plan_dir}/ (index.md, 00-overview.md, and all section-*.md files)
2. Review each section for completeness:
   - Are there missing steps that would block implementation?
   - Are edge cases and error handling accounted for?
   - Are dependencies between sections correctly identified?
   - Are test strategies adequate for each section?
3. Check for missing sync points -- if the plan adds new types, enum variants, or module registrations, does it list ALL locations that must be updated together?
4. For every gap found, EDIT the plan files directly to add the missing content
5. Add missing checklist items, missing steps, missing test requirements
6. Add a brief comment near each addition: <!-- reviewed: completeness fix -->
7. When reviewing Third Party Review findings, you MUST NOT dismiss findings because they are "unrelated", "out of scope", or "pre-existing". Accept any finding that identifies a real issue. Reject only findings that are factually incorrect.

You may add new sections, restructure, or expand scope if the plan has genuine gaps.
After editing, list what you changed and why.
```

#### Agent 3: Hygiene & Feasibility Review (Codebase-Aware)

```
You are reviewing an existing plan for ori_term (a GPU-accelerated terminal emulator in Rust) at {plan_dir}/.

Your job is twofold: (1) ensure the plan itself follows hygiene rules, and (2) scan the actual codebase areas the plan will touch to find existing issues that should be cleaned up along the way. The principle: every plan section should leave the code better and cleaner than before.

INSTRUCTIONS:

## Part 1: Plan-Level Hygiene

1. Read ALL files in {plan_dir}/ (index.md, 00-overview.md, and all section-*.md files)
2. Read the hygiene rules at .claude/rules/impl-hygiene.md and code hygiene rules at .claude/rules/code-hygiene.md
3. Review the plan against these rules:
   - Does the plan respect file size limits (500 lines)?
   - Does it maintain module boundary discipline?
   - Does it follow the test file conventions (sibling tests.rs)?
   - Does it respect rendering discipline (pure computation in draw_frame, no state mutation during render)?
   - Does it respect event flow discipline (events through event loop, explicit state transitions)?
   - Are implementation steps ordered correctly (upstream before downstream)?
   - Are there steps that are impractical or underestimate complexity?
4. Reorder steps if they violate crate dependency ordering
5. Add warnings for steps that are particularly complex or risky

## Part 2: Codebase Scan -- "Leave It Better Than You Found It"

6. Extract from the plan every file path, crate, and module that will be touched
7. Actually READ those files (up to 30 files; prioritize files mentioned in multiple sections)
8. Audit each file against the hygiene rules, looking for existing issues:
   - **BLOAT**: Files over 500 lines that the plan will touch but doesn't plan to split
   - **WASTE**: Unnecessary clones, allocations, stale comments, dead code
   - **DRIFT**: Registration sync points that are already out of sync
   - **EXPOSURE**: Internal state leaking through boundary types
   - **LEAK**: Layer bleeding in files the plan modifies
   - **STYLE**: Missing docs on pub items, bare TODOs, decorative banners, inline test modules
9. EDIT the plan files to weave "fix along the way" checklist items into the appropriate sections
10. Do NOT fabricate findings. Every finding must reference a real file:line with a real issue.
11. Preserve Third Party Review history. If accepted findings imply missing checklist work, weave those tasks into the relevant subsections instead of deleting the findings.

## Output

Add a brief comment near each change: <!-- reviewed: hygiene fix -->
After editing, list plan-level fixes, codebase findings by category, and files scanned vs files with findings.
```

#### Agent 4: Clarity & Consistency Review

```
You are reviewing an existing plan for ori_term (a GPU-accelerated terminal emulator in Rust) at {plan_dir}/.

INSTRUCTIONS:
1. Read ALL files in {plan_dir}/ (index.md, 00-overview.md, and all section-*.md files)
2. Review for clarity and internal consistency:
   - Are section descriptions clear and unambiguous?
   - Do checklist items describe concrete, actionable tasks (not vague goals)?
   - Is terminology consistent across sections?
   - Does the overview (00-overview.md) accurately reflect the section contents?
   - Does index.md have accurate keyword clusters for each section?
   - Are there contradictions between sections?
3. For every issue found, EDIT the plan files directly to improve clarity
4. Sharpen vague checklist items into specific, verifiable tasks
5. Fix inconsistent terminology
6. Update the overview if sections have changed during prior reviews
7. Remove all <!-- reviewed: ... --> comments left by previous reviewers (clean up)
8. Verify every section frontmatter includes `reviewed` plus `third_party_review.status` / `third_party_review.updated`. Add missing `third_party_review` blocks with `status: none` and `updated: null`.

After editing, list what you changed and why.
```

### Step 5: Present Verdict

After all four agents complete, consolidate their findings into a summary ranked by severity (**Critical** > **Major** > **Minor**).

```
## Plan Review: {plan name}

### Changes Made

#### Agent 1 -- Technical Accuracy
- {list of edits made}

#### Agent 2 -- Completeness & Gaps
- {list of edits made}

#### Agent 3 -- Hygiene & Feasibility
- {list of edits made}

#### Agent 4 -- Clarity & Consistency
- {list of edits made}

### Remaining Concerns

{Any issues the agents flagged but could not fix automatically,
ranked by severity: Critical > Major > Minor}

---

## Verdict

**{CLEAN | MINOR FIXES APPLIED | SIGNIFICANT REWORK APPLIED | NEEDS MANUAL ATTENTION}**

{2-3 sentence overall assessment. Note the plan's strengths as well as weaknesses.
State total number of edits made across all agents. Flag anything that
requires human judgement rather than mechanical fixes.}
```

**Verdict definitions:**
- **CLEAN**: No issues found. Plan is ready for implementation.
- **MINOR FIXES APPLIED**: Small corrections made (typos, wrong paths, minor gaps). Plan is ready.
- **SIGNIFICANT REWORK APPLIED**: Substantial edits (reordered steps, added missing sections, fixed incorrect assumptions). Review the diff before proceeding.
- **NEEDS MANUAL ATTENTION**: Issues found that require human judgement -- architectural decisions, ambiguous scope, conflicting requirements. Cannot be auto-fixed.

### Step 6: Update Review Gate

After the review completes (any verdict except NEEDS MANUAL ATTENTION), update `reviewed: false` → `reviewed: true` **ONLY on the specific section file that was reviewed**.

- The review target must be a **single section file** (e.g., `plans/roadmap/section-05.md`). That file — and only that file — gets `reviewed: true`.
- Do NOT mark any other section files as reviewed. Ever.
- If a **directory** was specified (e.g., `plans/mux-flatten/`), run the review agents across the plan for context, but do NOT flip `reviewed` on any section. The caller (`/continue-roadmap` or the user) decides which specific section to gate.

**Why only one section at a time:** As you implement Section N, reality diverges from the plan — new constraints, architectural decisions, deviations from assumptions. Sections N+1, N+2, etc. were written against the *original* assumptions. Marking them `reviewed: true` before they're about to be implemented defeats the purpose — they'd be "reviewed" against stale context. Each section gets reviewed right before implementation, either by the user running `/review-plan` on that section directly, or by `/continue-roadmap` triggering a review when it encounters `reviewed: false`.

## Important Rules

1. **Agents edit directly** -- This is not a report-only review. Agents fix what they find.
2. **Sequential, not parallel** -- Each agent sees prior agents' edits. Order matters.
3. **Be specific** -- Every change needs evidence: a file:line reference, a crate API, or concrete reasoning.
4. **Cross-reference, don't guess** -- Agents must actually read source code and reference repos.
5. **Check module dependency order** -- Implementation steps must respect: `oriterm_core` (library) before `oriterm` (binary). Within modules, upstream before downstream.
6. **Clean up after yourself** -- Agent 4 removes all `<!-- reviewed: ... -->` markers.
7. **Flag what can't be auto-fixed** -- Architectural decisions and scope questions go in "Remaining Concerns" for human review.
8. **Do not dismiss TPR findings as unrelated** -- A finding may only be rejected if the described issue does not actually exist.
