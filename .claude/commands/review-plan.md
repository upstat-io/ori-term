---
name: review-plan
description: Review a plan as one cohesive sequential strategy. Ensure every section is executable, fulfills the mission, expands where needed, and meets CLAUDE.md testing rigor.
allowed-tools: Read, Grep, Glob, Agent, AskUserQuestion, Bash, Edit, Write
---

# Review Plan Command

Review a plan as **one cohesive sequential strategy**. Cross-reference against the codebase and hygiene rules, ensure every section is executable and fulfills the mission in its entirety, expand the plan where coverage is insufficient (adding sections, adding checkboxes), and enforce CLAUDE.md testing rigor. Fix problems directly via 4 sequential review agents. Report findings as a verdict.

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

### Step 2: Load Rules

The full rule set is embedded below (source of truth files — do not maintain separate copies). These rules inform all review agents for checking module boundaries, file size limits, rendering discipline, event flow, testing rigor, and other hygiene requirements.

**Implementation Hygiene Rules** (`.claude/rules/impl-hygiene.md`):
@.claude/rules/impl-hygiene.md

**Code Hygiene Rules** (`.claude/rules/code-hygiene.md`):
@.claude/rules/code-hygiene.md

**Test Organization Rules** (`.claude/rules/test-organization.md`):
@.claude/rules/test-organization.md

**Crate Boundary Rules** (`.claude/rules/crate-boundaries.md`):
@.claude/rules/crate-boundaries.md

**CLAUDE.md Testing Requirements** (extracted — these are the testing standards every plan section must meet):

- **Buffer/TestBackend approach** for rendering tests (Ratatui pattern)
- **WidgetTestHarness** (`oriterm_ui/src/testing/`) for all widget tests — headless, no GPU/display/platform
- **Test Unicode width** with CJK, emoji, combining marks, ZWJ sequences
- **Test every env var combination** for color detection
- **Platform matrix** in CI (macOS, Windows, Linux)
- **Visual regression tests** where applicable
- **Verify behavior, not implementation** — test what the code does, not how
- **Sibling `tests.rs` pattern** — no inline test modules
- **Architecture tests** (`cargo test -p oriterm --test architecture`) for cross-crate boundary enforcement
- **Every section must specify**: what tests are written, what they cover, what harness/pattern is used, and what edge cases are exercised

### Step 3: Initial Assessment

Before launching agents, do a quick read-through and report to the user:
- Plan name and stated mission
- Number of sections/files
- Quick sequencing check: do sections form a logical chain where each builds on the previous?
- Obvious gaps: are there goals in the overview that no section addresses?
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

#### Agent 1: Architecture & Correctness

Spawn an Agent with the following prompt (substitute `{plan_dir}` with the actual plan directory path):

```
You are reviewing an existing plan for ori_term (a GPU-accelerated terminal emulator in Rust) at {plan_dir}/.

YOUR GOAL: Make the plan MORE CORRECT and MORE LIKELY TO SUCCEED as ONE COHESIVE SEQUENTIAL STRATEGY. The plan must work as a whole — sections must chain together so that implementing them in order achieves the full mission. Do not scope down. If the plan is incomplete, ADD what's missing — including new sections.

INSTRUCTIONS:
1. Read ALL files in {plan_dir}/ (index.md, 00-overview.md, and all section-*.md files)
2. Restate the plan's mission:
   - What is the plan trying to achieve?
   - What would success look like in the running application?
   - Does every goal stated in 00-overview.md have at least one section that delivers it?
3. COHESION CHECK — verify the plan works as a sequential chain:
   - Can sections be implemented in order (1, 2, 3, ...) without forward dependencies?
   - Does each section's output become the next section's input? If not, what's the gap?
   - Are there goals in the overview that NO section addresses? → ADD a section for each.
   - Are there sections that duplicate work or contradict each other?
   - After implementing ALL sections, would the stated mission be FULLY achieved?
   - If not: what's missing? Add sections, expand existing sections, or restructure.
4. Cross-reference EVERY architectural assumption against the actual codebase:
   - Read the actual source files referenced. Do the types, functions, modules exist?
   - Are the crate/module dependency assumptions correct?
   - Is the proposed approach actually the right way to achieve the goal, or is there a better path?
   - Would this approach ACTUALLY WORK if implemented as described?
5. For each section, ask: "If I implemented exactly what this section says, would the stated goal be achieved?"
   - If NO: fix the approach. Add missing steps. Correct wrong assumptions.
   - If UNCERTAIN: investigate the codebase deeper. Read more files. Resolve the uncertainty.
6. Check claims against reference repos in ~/projects/reference_repos/ where relevant
7. EDIT the plan files to:
   - Fix incorrect approaches with ones that will actually work
   - Add missing steps that are necessary for success
   - Add NEW SECTIONS for goals that no existing section addresses
   - Add checkboxes for concrete implementation tasks that are implied but not listed
   - Expand scope where the plan is too narrow to achieve its goals
   - Correct wrong file paths, type names, function signatures
   - Reorder sections if dependency order is wrong
   - NEVER reduce scope unless the plan genuinely overreaches beyond its stated mission
   - Update index.md to reflect any added/reordered sections
8. Add a brief comment near each fix: <!-- reviewed: architecture fix -->
9. When reviewing Third Party Review findings, you MUST NOT dismiss findings because they are "unrelated", "out of scope", or "pre-existing". Accept any finding that identifies a real issue.

You may add missing sections, expand scope, restructure, or completely rewrite approaches that won't work.
After editing, report: Plan understanding, Verified, Inferred / uncertain, Prior agent audit, Edits made.
```

#### Agent 2: Technical Accuracy & Feasibility

```
You are reviewing an existing plan for ori_term (a GPU-accelerated terminal emulator in Rust) at {plan_dir}/.

YOUR GOAL: Ensure every technical claim is CORRECT, every proposed approach is FEASIBLE, and every section is EXECUTABLE — meaning someone can sit down and implement it without guessing. Do not scope down. If something is wrong, replace it with something that works. If something is missing, add it.

INSTRUCTIONS:
1. Read ALL files in {plan_dir}/ (index.md, 00-overview.md, and all section-*.md files)
2. Fresh verification pass:
   - What is the plan trying to achieve?
   - Did Agent 1 improve the plan or introduce drift?
3. For EVERY technical claim, verify against the actual codebase:
   - Read the actual source files. Do referenced types, functions, modules exist as described?
   - Are code examples accurate? Would they compile?
   - Are dependency assumptions correct? Are sections ordered so prerequisites come first?
   - If the plan proposes new types or APIs: are ALL consumers and sync points listed?
4. For EVERY proposed approach, assess feasibility:
   - Would this actually work as described? What could go wrong?
   - Are there edge cases, race conditions, or integration issues not accounted for?
   - Is the complexity assessment realistic, or is the plan underestimating difficulty?
   - Are there simpler approaches that achieve the same goal? Are there more robust approaches?
5. EXECUTABILITY CHECK — for each section:
   - Does the section have enough checkboxes to cover every implementation task?
   - Could an implementer complete the section by working through the checkboxes in order?
   - Are there implied tasks (file creation, type definition, import wiring, test writing) that aren't listed? → Add them as checkboxes.
   - If a section says "add X" — is there a checkbox for creating the file, writing the impl, wiring it in, AND testing it?
   - Vague checkboxes like "implement feature X" must be broken into concrete sub-tasks.
6. DEFERRAL HANDLING — if a section claims something "cannot be done yet" or "will be handled later":
   - Is there an actual section later in the plan that handles it? If not, ADD one.
   - If truly impossible now (missing upstream dependency, needs user decision), mark it explicitly with rationale and add a deferred section at the appropriate point in the plan.
   - The plan must be SELF-CONTAINED: every deferred item must resolve within the plan itself.
7. EDIT the plan to:
   - Replace wrong approaches with correct ones
   - Add missing feasibility considerations
   - Fix all inaccurate technical claims with verified information
   - Add steps and checkboxes that are necessary but missing
   - Break vague checkboxes into concrete, implementable sub-tasks
   - Add new sections for deferred items that have no home
   - Flag HIGH-RISK subsections where the approach might not work
   - NEVER scope down — if the plan needs MORE to achieve its goals, add more
8. Add a brief comment near each fix: <!-- reviewed: accuracy fix -->
9. When reviewing TPR findings, reject only findings that are factually incorrect.

After editing, report: Plan understanding, Verified, Inferred / uncertain, Prior agent audit, Edits made.
```

#### Agent 3: Completeness & Goal Fulfillment

```
You are reviewing an existing plan for ori_term (a GPU-accelerated terminal emulator in Rust) at {plan_dir}/.

YOUR GOAL: Ensure the plan is COMPLETE enough to achieve its stated goals AND meets the project's testing rigor standards. Fill gaps. Add missing steps and tests. Expand where needed. Do NOT scope down or simplify away necessary work.

INSTRUCTIONS:
1. Read ALL files in {plan_dir}/ (index.md, 00-overview.md, and all section-*.md files)
2. Read the hygiene rules at .claude/rules/impl-hygiene.md, .claude/rules/code-hygiene.md, .claude/rules/test-organization.md, and .claude/rules/crate-boundaries.md
3. Fresh verification pass:
   - What are the plan's stated goals?
   - After Agents 1-2, is the plan actually on track to achieve those goals?
   - Are there gaps where implementation would stall because a step is missing?
4. For EACH section, walk through the implementation mentally:
   - If I sat down to implement this section right now, would I have enough information?
   - Are there decisions left unresolved that would block me?
   - Are there dependencies on other code that the plan doesn't account for?
   - Does each checkbox represent ONE concrete task (not multiple tasks crammed together)?
5. TESTING RIGOR CHECK — this is critical. For EACH section, verify:
   a. Does the section specify WHAT tests will be written? Not "add tests" — specific test functions.
   b. Does the section specify WHAT EACH TEST COVERS? Edge cases, happy path, error cases.
   c. Does the section use the correct test harness/pattern?
      - Widget behavior → WidgetTestHarness (headless, no GPU)
      - Rendering output → Buffer/TestBackend approach
      - Grid/terminal logic → direct unit tests in sibling tests.rs
      - Cross-crate boundaries → architecture tests
      - Unicode handling → CJK, emoji, combining marks, ZWJ sequences
      - Color detection → env var combinations (NO_COLOR, CLICOLOR, COLORTERM, TERM)
      - Platform behavior → platform matrix consideration
   d. Are there MISSING test scenarios? Common gaps:
      - Edge cases (empty input, max values, boundary conditions)
      - Error paths (what happens when it fails?)
      - Integration between this section and previous sections
      - Regression tests for bugs this section fixes
      - Performance assertions where the section touches hot paths
   e. Does the section follow sibling tests.rs organization? (not inline test modules)
   f. For EVERY new public type or function — is there a test checkbox?
   g. For EVERY behavior change — is there a test that proves the new behavior?

   If ANY of these are missing: ADD specific test checkboxes with concrete test names and what they verify.

6. Check against hygiene rules:
   - File size limits (500 lines) — flag files that would exceed after changes
   - Module boundary discipline — no circular imports, one-way data flow
   - Test organization — sibling tests.rs pattern
   - Rendering discipline — pure computation in draw paths
   - Implementation order — library crate before binary crate
   - Crate boundaries — pure UI logic in oriterm_ui, not oriterm
7. EDIT the plan to:
   - Add missing steps that would block implementation
   - Add missing edge cases and error handling
   - ADD SPECIFIC TEST CHECKBOXES where testing is vague or absent — name the test, describe what it verifies
   - Expand test strategies that say "add tests" into concrete test lists
   - Add a "### Tests" subsection to any section that lacks one
   - Add warnings for high-complexity or high-risk subsections
   - Reorder steps that violate dependency ordering
   - NEVER remove scope that is necessary to achieve the plan's goals
   - Add hygiene compliance steps where the plan would violate rules
8. Add a brief comment near each change: <!-- reviewed: completeness/hygiene fix -->
9. Preserve Third Party Review history.

After editing, report: Plan understanding, Verified, Inferred / uncertain, Prior agent audit, Edits made.
```

#### Agent 4: Final Challenge — Will This Actually Work?

```
You are reviewing an existing plan for ori_term (a GPU-accelerated terminal emulator in Rust) at {plan_dir}/.

YOUR GOAL: Be the final skeptic. Challenge whether the plan will ACTUALLY ACHIEVE ITS FULL MISSION when implemented section-by-section. Verify it works as one cohesive strategy. If it won't, fix it. Do not scope down — scope UP if needed.

INSTRUCTIONS:
1. Read ALL files in {plan_dir}/ (index.md, 00-overview.md, and all section-*.md files)
2. MISSION FULFILLMENT — the most important check:
   - Re-read 00-overview.md. List every goal and success criterion stated.
   - For EACH goal: trace which section(s) deliver it. If no section delivers a goal, that's a critical gap.
   - After implementing ALL sections in order, would the mission be FULLY achieved? Not partially — fully.
   - If not: ADD sections, expand existing sections, or flag what's missing.
   - Are there "deferred" items that never get resolved within the plan? → They must be resolved — add sections or expand.
3. SEQUENTIAL COHESION — final verification:
   - Walk through sections 1, 2, 3, ... in order. At each step:
     - Does this section depend on anything not yet built by a prior section?
     - Does this section produce something the next section needs?
     - After this section, is the codebase in a valid, buildable, testable state?
   - If ANY section would leave the codebase in a broken state → fix the ordering or add bridge steps.
4. Final challenge pass — ask hard questions:
   - Are there sections that sound good but would fail in practice?
   - Are prior agents' edits making the plan better or just more verbose?
   - Is the plan honest about difficulty, or is it hand-waving over hard parts?
   - Are there integration risks between sections that nobody flagged?
5. TESTING FINAL CHECK:
   - Does every section have concrete, named test checkboxes (not just "add tests")?
   - Do test strategies match the correct harness (WidgetTestHarness for widgets, sibling tests.rs, etc.)?
   - Are there sections where tests were added by Agent 3 that are wrong or redundant? Fix them.
   - After all sections: would the test suite actually catch regressions for the plan's goals?
6. Review for clarity and consistency:
   - Are section descriptions clear enough to implement without guessing?
   - Do checklist items describe concrete, verifiable tasks?
   - Is terminology consistent across sections?
   - Does the overview accurately reflect section contents (including any new sections added)?
   - Are there contradictions between sections?
7. EDIT the plan to:
   - Fix approaches that won't work in practice (even if they sound correct in theory)
   - Sharpen vague items into specific, implementable tasks
   - Add integration steps between sections if the plan assumes they'll "just work"
   - Add missing sections for unfulfilled goals
   - Remove bloat that makes the plan harder to follow without improving it
   - Fix inconsistent terminology
   - Update index.md and 00-overview.md if sections were added, reordered, or restructured during review
   - NEVER scope down unless something is genuinely unnecessary for the goals
8. Clean up:
   - Remove all <!-- reviewed: ... --> comments from previous agents
   - Verify every section has `reviewed` + `third_party_review` in frontmatter
   - Add missing `third_party_review` blocks with `status: none` and `updated: null`
   - Verify index.md lists ALL sections (including any added during review) in correct order

After editing, report: Plan understanding, Verified, Inferred / uncertain, Prior agent audit, Edits made.
```

### Step 5: Present Verdict

After all four agents complete, consolidate their findings into a summary ranked by severity (**Critical** > **Major** > **Minor**).

```
## Plan Review: {plan name}

### Mission & Cohesion

- {What the plan's stated mission is}
- {Whether all goals are covered by sections — list any gaps found and sections added}
- {Whether sections chain sequentially without forward dependencies}
- {Whether implementing all sections in order would fully achieve the mission}

### Verified Claims

- {Claims confirmed against the codebase or reference repos}

### Inferred / Unverified Claims

- {Claims that still rely on inference, judgement, or incomplete evidence}

### Changes Made

#### Agent 1 — Architecture, Cohesion & Sequencing
- {list of edits made, especially: sections added, reordered, or restructured}

#### Agent 2 — Technical Accuracy, Executability & Deferral Resolution
- {list of edits made, especially: checkboxes added, vague items sharpened, deferrals resolved}

#### Agent 3 — Completeness, Testing Rigor & Hygiene
- {list of edits made, especially: test checkboxes added, test strategies expanded, harness patterns specified}

#### Agent 4 — Mission Fulfillment, Cohesion & Final Challenge
- {list of edits made, especially: missing sections added, integration steps added, overview updated}

### Testing Coverage

- {Summary of testing rigor: are all sections covered?}
- {Which harness patterns are used where?}
- {Any remaining testing gaps?}

### Remaining Concerns

{Any issues the agents flagged but could not fix automatically,
ranked by severity: Critical > Major > Minor}

---

## Verdict

**{CLEAN | MINOR FIXES APPLIED | SIGNIFICANT REWORK APPLIED | NEEDS MANUAL ATTENTION}**

{2-3 sentence overall assessment. Note the plan's strengths as well as weaknesses.
State total number of edits made across all agents (including sections and checkboxes added).
Flag anything that requires human judgement rather than mechanical fixes.
Explicitly say whether the final plan is a complete, executable, cohesive strategy
that fulfills the stated mission with adequate testing rigor.}
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
11. **The plan must be self-contained** — Every deferred item must resolve within the plan. If something "can't be done yet", there must be a later section that handles it. No loose ends.
12. **Expand, don't trim** — When the mission isn't fully covered, agents MUST add sections and checkboxes. A plan that doesn't fulfill its mission is worse than a plan that's too long.
13. **Testing is not optional** — Every section must have specific, named test checkboxes with the correct harness pattern. "Add tests" is not a valid checkbox. "Add `test_hover_state_transitions` in `tests.rs` using WidgetTestHarness — verify hot→active→idle cycle" is.
14. **Sections must chain** — After implementing section N, the codebase must be buildable, testable, and ready for section N+1. No section may leave the codebase in a broken state.
