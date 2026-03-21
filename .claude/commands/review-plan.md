---
name: review-plan
description: Review a plan for problems — technical accuracy, completeness, hygiene compliance, and crate/module dependency ordering.
allowed-tools: Read, Grep, Glob, Agent, AskUserQuestion, Bash, Edit, Write
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

- `none` — no findings recorded (`- None.`)
- `findings` — one or more unchecked TPR items exist
- `resolved` — findings exist historically, but all are resolved

When this command edits a plan, it must preserve and normalize this block instead of deleting it.

## Usage

```
/review-plan <plan-path>
```

- `plan-path`: **Required.** Path to the plan directory or a specific plan file (e.g., `plans/mux-flatten/`, `plans/roadmap/section-05.md`).
  - If a directory: reviews all files in the directory
  - If a single file: reviews that file (and reads siblings for context)

## Workflow

### Step 1: Read the Plan

Read the plan file(s) specified in `$ARGUMENTS`. If the path doesn't exist, report the error and stop.

- If a directory, read all `.md` files: `index.md`, `00-overview.md`, and all `section-*.md` files
- If a single file, read it plus any sibling plan files for context

### Step 2: Load Hygiene Rules

The full rule set is embedded below (source of truth files — do not maintain separate copies). These rules inform all review agents for checking module boundaries, file size limits, rendering discipline, event flow, and other hygiene requirements.

**Implementation Hygiene Rules** (`.claude/rules/impl-hygiene.md`):
@.claude/rules/impl-hygiene.md

**Code Hygiene Rules** (`.claude/rules/code-hygiene.md`):
@.claude/rules/code-hygiene.md

**Test Organization Rules** (`.claude/rules/test-organization.md`):
@.claude/rules/test-organization.md

### Step 3: Initial Assessment

Before launching agents, do a quick read-through and report to the user:
- Plan name and scope
- Number of sections/files
- Note: "Running 4 sequential review passes..."

### Step 4: Sequential Verification + Edit Review (4 Agents)

Run **4 review agents in sequence** (NOT parallel). Each agent:

- Receives **only the plan files** — no conversation context, no hidden rationale beyond the plan itself
- Must begin with a **fresh verification pass** over the whole plan and the current codebase state
- Must explicitly check whether prior agents' edits improved the plan or introduced new drift
- May **edit only after** it has re-established what the plan is trying to do, what is verified, and what remains uncertain
- Sees edits made by all previous agents (because they run sequentially)

This keeps the sequential pipeline honest: each reviewer independently re-understands the plan, validates the prior work, then applies its own corrections.

**IMPORTANT**: Run these agents ONE AT A TIME. Wait for each to complete before starting the next.

Each agent response must include:
- `Plan understanding`: what the reviewed section(s) are trying to achieve
- `Verified`: claims confirmed against the codebase or reference repos
- `Inferred / uncertain`: claims that seem plausible but were not fully proven
- `Prior agent audit`: whether the previous edits improved the plan or need correction
- `Edits made`: concrete file changes and why

If an earlier agent added detail that is incorrect, unnecessary, or distracts from the actual implementation path, a later agent should remove or rewrite it.

#### Agent 1: Intent & Architecture Verification

Spawn an Agent with the following prompt (substitute `{plan_dir}` with the actual plan directory path):

```
You are reviewing an existing plan for ori_term (a GPU-accelerated terminal emulator in Rust) at {plan_dir}/.

INSTRUCTIONS:
1. Read ALL files in {plan_dir}/ (index.md, 00-overview.md, and all section-*.md files)
2. Before editing anything, restate the plan's thesis in your own words:
   - What real system behavior is this plan trying to change?
   - What existing code paths, crates, modules, or abstractions does it depend on?
   - What would count as proof that the plan is correct?
3. Cross-reference the plan's architectural assumptions against the actual codebase:
   - Do the referenced files, types, functions, modules, and boundaries exist?
   - Are the crate/module dependency assumptions correct? (oriterm_core is the library crate, oriterm is the binary crate that depends on it)
   - Is this still the right implementation shape, or has the codebase evolved past the proposed approach?
4. Check claims against reference repos in ~/projects/reference_repos/console_repos/ where relevant (Alacritty, WezTerm, Ghostty patterns)
5. Audit the plan itself before editing:
   - Which core claims are VERIFIED?
   - Which are only INFERRED or still UNCERTAIN?
   - Which sections appear to optimize wording without proving the actual approach?
6. Only after that audit, EDIT the plan files directly to fix architectural drift, incorrect assumptions, wrong paths, or stale implementation strategy
7. Add a brief comment near each fix: <!-- reviewed: architecture fix -->
8. When reviewing Third Party Review findings, you MUST NOT dismiss findings because they are "unrelated", "out of scope", or "pre-existing". Accept any finding that identifies a real issue in the codebase. Reject only findings that are factually incorrect.

You may add missing sections, remove incorrect detail, expand scope, or restructure if the plan is genuinely pointing at the wrong shape of implementation.
After editing, report: Plan understanding, Verified, Inferred / uncertain, Prior agent audit, Edits made.
```

#### Agent 2: Technical Accuracy & Dependency Review

```
You are reviewing an existing plan for ori_term (a GPU-accelerated terminal emulator in Rust) at {plan_dir}/.

INSTRUCTIONS:
1. Read ALL files in {plan_dir}/ (index.md, 00-overview.md, and all section-*.md files)
2. Start with a fresh verification pass before making any edits:
   - Restate what the plan is trying to achieve after Agent 1's edits
   - Identify any places where Agent 1 improved the plan versus any places where Agent 1 added drift or unnecessary detail
3. Cross-reference every technical claim against the actual codebase:
   - Do referenced files, types, functions, modules exist?
   - Are described code patterns accurate?
   - Are dependency assumptions and section ordering correct?
   - If the plan adds new types, enum variants, or module registrations, does it list ALL sync points that must be updated together?
4. For every inaccuracy found, EDIT the plan files directly to fix it
5. Remove or rewrite detail added by prior agents if it is technically wrong, underspecified, or distracts from the real implementation path
6. Add a brief comment near each fix: <!-- reviewed: accuracy fix -->
7. When reviewing Third Party Review findings, you MUST NOT dismiss findings because they are "unrelated", "out of scope", or "pre-existing". Accept any finding that identifies a real issue. Reject only findings that are factually incorrect.

You may add missing steps, delete stale ones, restructure ordering, or tighten technical claims if the plan is inaccurate.
After editing, report: Plan understanding, Verified, Inferred / uncertain, Prior agent audit, Edits made.
```

#### Agent 3: Completeness, Hygiene & Feasibility Review

```
You are reviewing an existing plan for ori_term (a GPU-accelerated terminal emulator in Rust) at {plan_dir}/.

INSTRUCTIONS:
1. Read ALL files in {plan_dir}/ (index.md, 00-overview.md, and all section-*.md files)
2. Read the hygiene rules at .claude/rules/impl-hygiene.md, .claude/rules/code-hygiene.md, and .claude/rules/test-organization.md
3. Start with a fresh verification pass before editing:
   - Restate what the plan is trying to achieve after Agents 1 and 2
   - Identify whether prior edits made the plan more executable or merely more detailed
4. Review the plan for completeness and against the hygiene rules:
   - Are there missing steps that would block implementation?
   - Are edge cases and error handling accounted for?
   - Are test strategies adequate for each section?
   - Does the plan respect file size limits (500 lines)?
   - Does it maintain module boundary discipline (one-way data flow, no circular imports)?
   - Does it follow the test file conventions (sibling tests.rs)?
   - Does it respect rendering discipline (pure computation in draw_frame, no state mutation during render)?
   - Does it respect event flow discipline (events through event loop, explicit state transitions)?
   - Are implementation steps ordered correctly (library crate before binary crate)?
   - Are there steps that are impractical or underestimate complexity?
5. For every missing step, hygiene violation, or feasibility concern, EDIT the plan files directly to fix them
6. Reorder steps if they violate dependency ordering
7. Add warnings for steps that are particularly complex or risky
8. Remove checklist churn that adds detail without improving implementability or verification
9. Add a brief comment near each change: <!-- reviewed: completeness/hygiene fix -->
10. Preserve Third Party Review history. If accepted findings imply missing checklist work, weave those tasks into the relevant subsections instead of deleting the findings.

You may expand scope, add sections, trim misleading detail, or restructure if needed to satisfy completeness, hygiene, and feasibility.
After editing, report: Plan understanding, Verified, Inferred / uncertain, Prior agent audit, Edits made.
```

#### Agent 4: Clarity, Consistency & Final Challenge Review

```
You are reviewing an existing plan for ori_term (a GPU-accelerated terminal emulator in Rust) at {plan_dir}/.

INSTRUCTIONS:
1. Read ALL files in {plan_dir}/ (index.md, 00-overview.md, and all section-*.md files)
2. Start with a fresh verification pass before editing:
   - Restate what the plan is trying to achieve after Agents 1-3
   - Challenge whether the current version of the plan is now actually implementable, not just more polished
   - Identify any prior-agent changes that should be reverted, simplified, or clarified
3. Review for clarity and internal consistency:
   - Are section descriptions clear and unambiguous?
   - Do checklist items describe concrete, actionable tasks (not vague goals)?
   - Is terminology consistent across sections?
   - Does the overview (00-overview.md) accurately reflect the section contents?
   - Does index.md have accurate keyword clusters for each section?
   - Are there contradictions between sections?
4. For every issue found, EDIT the plan files directly to improve clarity
5. Sharpen vague checklist items into specific, verifiable tasks
6. Delete or simplify detail that bloats the plan without making implementation or verification clearer
7. Fix inconsistent terminology
8. Update the overview if sections have changed during prior reviews
9. Remove all <!-- reviewed: ... --> comments left by previous reviewers (clean up)
10. Verify every section frontmatter includes `reviewed` plus `third_party_review.status` / `third_party_review.updated`. Add missing `third_party_review` blocks with `status: none` and `updated: null`.

After editing, report: Plan understanding, Verified, Inferred / uncertain, Prior agent audit, Edits made.
```

### Step 5: Present Verdict

After all four agents complete, consolidate their findings into a summary ranked by severity (**Critical** > **Major** > **Minor**).

```
## Plan Review: {plan name}

### Plan Understanding

- {What the reviewed section(s) are actually trying to change}
- {What code paths / modules / crates the plan depends on}
- {What successful implementation would prove}

### Verified Claims

- {Claims confirmed against the codebase or reference repos}

### Inferred / Unverified Claims

- {Claims that still rely on inference, judgement, or incomplete evidence}

### Changes Made

#### Agent 1 — Intent & Architecture Verification
- {list of edits made}

#### Agent 2 — Technical Accuracy & Dependency Review
- {list of edits made}

#### Agent 3 — Completeness, Hygiene & Feasibility
- {list of edits made}

#### Agent 4 — Clarity, Consistency & Final Challenge
- {list of edits made}

### Remaining Concerns

{Any issues the agents flagged but could not fix automatically,
ranked by severity: Critical > Major > Minor}

---

## Verdict

**{CLEAN | MINOR FIXES APPLIED | SIGNIFICANT REWORK APPLIED | NEEDS MANUAL ATTENTION}**

{2-3 sentence overall assessment. Note the plan's strengths as well as weaknesses.
State total number of edits made across all agents. Flag anything that
requires human judgement rather than mechanical fixes. Explicitly say whether
the final plan is merely cleaner, or actually better-validated against reality.}
```

**Verdict definitions:**
- **CLEAN**: No issues found. Plan is ready for implementation.
- **MINOR FIXES APPLIED**: Small corrections made (typos, wrong paths, minor gaps). Plan is ready.
- **SIGNIFICANT REWORK APPLIED**: Substantial edits (reordered steps, added missing sections, fixed incorrect assumptions). Review the diff before proceeding.
- **NEEDS MANUAL ATTENTION**: Issues found that require human judgement — architectural decisions, ambiguous scope, conflicting requirements. Cannot be auto-fixed.

### Step 6: Update Review Gate

After the review completes (any verdict except NEEDS MANUAL ATTENTION), update `reviewed: false` → `reviewed: true` **ONLY on the specific section file that was reviewed**.

- The review target must be a **single section file** (e.g., `plans/roadmap/section-05.md`). That file — and only that file — gets `reviewed: true`.
- Do NOT mark any other section files as reviewed. Ever.
- If a **directory** was specified (e.g., `plans/mux-flatten/`), run the review agents across the plan for context, but do NOT flip `reviewed` on any section. The caller (`/continue-roadmap` or the user) decides which specific section to gate.

**Why only one section at a time:** As you implement Section N, reality diverges from the plan — new constraints, architectural decisions, deviations from assumptions. Sections N+1, N+2, etc. were written against the *original* assumptions. Marking them `reviewed: true` before they're about to be implemented defeats the purpose — they'd be "reviewed" against stale context. Each section gets reviewed right before implementation, either by the user running `/review-plan` on that section directly, or by `/continue-roadmap` triggering a review when it encounters `reviewed: false`.

## Important Rules

1. **Every agent verifies before editing** — The first task is to re-understand the plan and audit prior edits against the live codebase.
2. **Agents still edit directly** — This is not a report-only review. Validation without repair is incomplete.
3. **Sequential, not parallel** — Each agent sees prior agents' edits. Order matters because later agents are expected to challenge earlier ones.
4. **Be specific** — Every change needs evidence: a file:line reference, a crate API, or concrete reasoning.
5. **Cross-reference, don't guess** — Agents must actually read source code and reference repos.
6. **Check module dependency order** — Implementation steps must respect: `oriterm_core` (library) before `oriterm` (binary). Within modules, upstream before downstream.
7. **Remove bad prior edits when necessary** — Later agents should rewrite or delete earlier changes that added drift, churn, or false precision.
8. **Clean up after yourself** — Agent 4 removes all `<!-- reviewed: ... -->` markers.
9. **Flag what can't be auto-fixed** — Architectural decisions and scope questions go in "Remaining Concerns" for human review.
10. **Do not dismiss TPR findings as unrelated** — A finding may only be rejected if the described issue does not actually exist.
