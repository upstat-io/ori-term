---
description: Create a new plan directory with index and section files using the standard template
---

# Create Plan Command

Create a new plan directory with index and section files using the standard template.

## Incremental Design — Non-Negotiable

**Every section must touch the real system.** No section should build types, traits, abstractions, or infrastructure in isolation. Every section starts from the production code path, modifies it, and produces an observable, verifiable change in the running application.

**The anti-pattern (banned):** Design an entire type hierarchy / trait system / abstraction layer across sections 01-05, then wire it into the actual render loop / event loop / terminal in section 06 and hope it all works. This produces thousands of lines of dead code behind `#[allow(dead_code)]`, zero observable behavior, and inevitable reverts.

**The correct pattern:** Each section:
1. **Starts from the production code path** that needs to change (the render loop, the event handler, the grid operation — the real thing)
2. **Builds only what's needed** to make that specific change work
3. **Wires it in immediately** — new code has a production caller in the same section
4. **Produces observable behavior** — you can see it working, measure it, test it
5. **Ends with build + clippy + test verification** — `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` all pass, plus new tests proving the change works

**Concretely:**
- If a section introduces a new type, that type must have a production caller by section end — no `#[allow(dead_code)]`
- If a section introduces a new trait, something must implement and use it by section end
- If a section is "infrastructure" with no observable terminal behavior change, it's wrong — restructure it to start from the behavior change and pull in only the infrastructure needed
- A section that can't be independently verified against the running application is too abstract — split it so each piece touches reality

## Usage

```
/create-plan <name> [description]
```

- `name`: Directory name for the plan (kebab-case, e.g., `gpu-refactor`, `mux-architecture`)
- `description`: Optional one-line description of the plan's goal

**Arguments:** `$ARGUMENTS`

## Workflow

### Step 1: Gather Information

If not provided via arguments, ask the user:

1. **Plan name** -- kebab-case directory name
2. **Plan title** -- Human-readable title (e.g., "GPU Renderer Refactor")
3. **Goal** -- One-line description of what this plan accomplishes
4. **Sections** -- List of major sections (at least 2-3)

### Step 2: Read the Template

Read `plans/_template/plan.md` for the structure reference.

### Step 3: Load Hygiene Rules

Read the following rule files and use them when structuring plan sections. They ensure plans account for module boundary discipline, file size limits, rendering pipeline purity, and other hygiene requirements from the start.

**Implementation Hygiene Rules** -- read `.claude/rules/impl-hygiene.md`:
!`cat .claude/rules/impl-hygiene.md`

**Code Hygiene Rules** -- read `.claude/rules/code-hygiene.md`:
!`cat .claude/rules/code-hygiene.md`

### Step 4: Create Directory Structure

Create the plan directory and files:

```
plans/{name}/
+-- index.md           # Keyword index for discovery
+-- 00-overview.md     # High-level goals and section summary
+-- section-01-*.md    # First section
+-- section-02-*.md    # Additional sections...
+-- section-NN-*.md    # Final section
```

### Step 5: Generate index.md

Create the keyword index with:
- **Reroute frontmatter** (if this is a reroute plan -- i.e., a parallel track alongside the main roadmap):
  ```yaml
  ---
  reroute: true
  name: "{Short Name}"
  full_name: "{Full Plan Name}"
  status: queued
  order: N
  ---
  ```
  The `name`, `full_name`, `status`, and `order` fields are the single source of truth.
  `order` controls queue priority -- lower value = promoted first (default 999 if omitted).
  `key` and `dir` are derived at load time from the directory name.
- Maintenance notice at the top
- How to use instructions
- Keyword cluster for each section (initially with placeholder keywords)
- Quick reference table

### Step 6: Generate 00-overview.md

Create overview with:
- Plan title and goal
- Section list with brief descriptions
- Dependencies (if any)
- Success criteria

### Step 7: Generate Section Files

For each section, create `section-{NN}-{name}.md` with:
- YAML frontmatter (section ID, title, status: not-started, goal, `reviewed`, `third_party_review: { status: none, updated: null }`)
- **`reviewed: true` for Section 01 ONLY** — it's the starting point and was vetted during plan creation
- **`reviewed: false` for ALL other sections** — they need re-review before implementation because earlier sections will cause deviations that have downstream impacts
- Section header with status emoji
- Placeholder subsections with `- [ ]` checkboxes
- **`## {NN}.R Third Party Review Findings` block** before the final build gate, initialized with:
  ```markdown
  ## {NN}.R Third Party Review Findings
  - None.
  ```
- **Mandatory Build/Verify/Test gate** as the final subsection of EVERY section (not just verification sections):
  ```markdown
  ## {NN}.N Build & Verify
  - [ ] `./build-all.sh` passes
  - [ ] `./clippy-all.sh` passes
  - [ ] `./test-all.sh` passes
  - [ ] New tests exist proving this section's changes work
  - [ ] No `#[allow(dead_code)]` on new items — everything has a production caller
  ```

**Incremental design enforcement:** Each section must identify which production code path it modifies and what observable behavior changes. If a section can't name a specific production code path it touches, it needs restructuring — merge it with the section that actually uses its output.

**Why the reviewed gate matters:** As you implement sections sequentially, reality diverges from the original plan — you discover new constraints, make architectural decisions, and deviate from assumptions. Later sections were written against the *original* assumptions, not the *actual* state after prior sections landed. `reviewed: false` forces a review checkpoint before each section to catch stale assumptions, incorrect file paths, wrong dependencies, and outdated design decisions. Without this gate, you'd implement plans that are already wrong.

### Step 8: Report Progress

Show the user:
- Files created
- Note: "Running 4 independent review passes..."

### Step 9: Sequential Independent Review (4 Agents)

After the plan is fully created, run **4 review agents in sequence** (NOT parallel). Each agent:

- Receives **only the plan files** -- no conversation context, no reasoning behind the plan
- Is instructed to **read the plan, review it, and edit the files directly** to fix issues
- Sees edits made by all previous agents (because they run sequentially)

This creates an iterative refinement pipeline: each reviewer builds on the last.

**IMPORTANT**: Run these agents ONE AT A TIME using the Task tool. Wait for each to complete before starting the next.

#### Agent 1: Technical Accuracy Review

Spawn a subagent (via Task tool) with the following prompt (substitute `{plan_dir}` with the actual plan directory path):

```
You are reviewing a plan for ori_term (a GPU-accelerated terminal emulator in Rust) at {plan_dir}/.

INSTRUCTIONS:
1. Read ALL files in {plan_dir}/ (index.md, 00-overview.md, and all section-*.md files)
2. Cross-reference every technical claim against the actual codebase:
   - Do referenced files, types, functions, modules exist?
   - Are crate/module dependency assumptions correct? (oriterm_core is the library crate, oriterm is the binary crate)
   - Are described code patterns accurate?
   - Are references to external crates (wgpu, winit, vte, etc.) correct?
3. Check claims against reference repos in ~/projects/reference_repos/console_repos/ (Alacritty, WezTerm, Ghostty)
4. For every inaccuracy found, EDIT the plan files directly to fix them
5. If a section references nonexistent code paths or wrong file locations, correct them
6. Add a brief comment near each fix: <!-- reviewed: accuracy fix -->
7. When reviewing Third Party Review findings, you MUST NOT dismiss findings because they are "unrelated", "out of scope", or "pre-existing". Accept any finding that identifies a real issue. Reject only findings that are factually incorrect.

You may add missing sections, expand scope, or restructure if the plan is genuinely incomplete.
After editing, list what you changed and why.
```

#### Agent 2: Completeness & Incremental Design Review

```
You are reviewing a plan for ori_term (a GPU-accelerated terminal emulator in Rust) at {plan_dir}/.

INSTRUCTIONS:
1. Read ALL files in {plan_dir}/ (index.md, 00-overview.md, and all section-*.md files)
2. **CRITICAL — Incremental design check.** For EVERY section, verify:
   - Does it start from a production code path (render loop, event handler, grid operation)?
   - Does every new type/trait/abstraction have a production caller by section end?
   - Does it produce an observable, verifiable behavior change?
   - Does it end with build/clippy/test verification?
   - RED FLAG: A section that builds types, traits, or infrastructure without wiring them into the running terminal is WRONG. Restructure it to start from the behavior change and pull in only the infrastructure needed.
   - RED FLAG: A plan that builds an entire abstraction layer across sections 01-05 then "integrates" in section 06 is WRONG. Each section must touch the real system.
3. Review each section for completeness:
   - Are there missing steps that would block implementation?
   - Are edge cases and error handling accounted for?
   - Are dependencies between sections correctly identified?
   - Are test strategies adequate for each section?
4. Check for missing sync points -- if the plan adds enum variants, new types, or registration entries, does it list ALL locations that must be updated together?
5. For every gap found, EDIT the plan files directly to add the missing content
6. Add missing checklist items, missing steps, missing test requirements
7. Add a brief comment near each addition: <!-- reviewed: completeness fix -->
8. When reviewing Third Party Review findings, you MUST NOT dismiss findings because they are "unrelated", "out of scope", or "pre-existing". Accept any finding that identifies a real issue. Reject only findings that are factually incorrect.

You may add new sections, restructure, or expand scope if the plan has genuine gaps.
If sections need to be restructured to satisfy incremental design, do it — move integration work earlier, merge "infrastructure" sections with the sections that use them.
After editing, list what you changed and why.
```

#### Agent 3: Hygiene & Feasibility Review (Codebase-Aware)

```
You are reviewing a plan for ori_term (a GPU-accelerated terminal emulator in Rust) at {plan_dir}/.

Your job is twofold: (1) ensure the plan itself follows hygiene rules, and (2) scan the actual codebase areas the plan will touch to find existing issues that should be cleaned up along the way. The principle: every plan section should leave the code better and cleaner than before.

INSTRUCTIONS:

## Part 1: Plan-Level Hygiene

1. Read ALL files in {plan_dir}/ (index.md, 00-overview.md, and all section-*.md files)
2. Read the hygiene rules at .claude/rules/impl-hygiene.md and code hygiene rules at .claude/rules/code-hygiene.md
3. Review the plan against these rules:
   - Does the plan respect file size limits (500 lines)?
   - Does it maintain module boundary discipline?
   - Does it follow the test file conventions (sibling tests.rs)?
   - Are implementation steps ordered correctly (upstream before downstream)?
   - Are there steps that are impractical or underestimate complexity?
4. **Verify every section has a Build/Verify/Test gate at the end:**
   - `./build-all.sh` passes
   - `./clippy-all.sh` passes
   - `./test-all.sh` passes
   - New tests exist proving the section's changes work
   If a section is missing this gate, ADD it. No exceptions.
5. **Verify no section introduces dead code.** If a section creates types, modules, or abstractions that won't have a production caller until a later section, that's a hygiene violation. The section must be restructured to include the wiring.
6. Reorder steps if they violate crate dependency ordering
7. Add warnings for steps that are particularly complex or risky

## Part 2: Codebase Scan -- "Leave It Better Than You Found It"

8. Extract from the plan every file path, crate, and module that will be touched (look for file:line references, crate names, module paths in checklist items and prose)
9. Actually READ those files (up to 30 files; prioritize files mentioned in multiple sections or that are core to the plan's goal)
10. Audit each file against the hygiene rules, looking for existing issues:
   - **BLOAT**: Files over 500 lines that the plan will touch but doesn't plan to split
   - **WASTE**: Unnecessary clones, allocations, stale comments, dead code, commented-out code
   - **DRIFT**: Registration sync points that are already out of sync
   - **EXPOSURE**: Internal state leaking through boundary types
   - **LEAK**: Layer bleeding in files the plan modifies
   - **STYLE**: Missing docs on pub items, bare TODOs, decorative banners, inline test modules
   - Any other violations from impl-hygiene.md
11. For each finding, identify which plan section touches that file/area
12. EDIT the plan files to weave "fix along the way" checklist items into the appropriate sections, using this format:
    - [ ] **[BLOAT]** `file:line` -- Split into submodules (currently N lines, exceeds 500-line limit)
    - [ ] **[WASTE]** `file:line` -- Remove stale comment / dead code / unnecessary clone
    - [ ] **[DRIFT]** `file:line` -- Sync missing variant with parallel location at `other_file:line`
    Place these items near the existing checklist items that touch the same file, so the implementer fixes them in the same pass. Group them under a "Cleanup" sub-heading within the section if there are 3+ findings for that section.
13. If findings cluster (5+ in one module), add a note: "Warning: Clustered findings suggest deeper design issue -- consider architectural review before proceeding"
14. Do NOT fabricate findings. Every finding must reference a real file:line with a real issue. If the touched code is already clean, say so.
15. Preserve Third Party Review history. If accepted findings imply missing checklist work, weave those tasks into the relevant subsections instead of deleting the findings.

## Output

Add a brief comment near each change: <!-- reviewed: hygiene fix -->
After editing, list:
- Plan-level fixes made (reordering, warnings, etc.)
- Codebase findings woven in, by category (e.g., 3 BLOAT, 2 WASTE, 1 DRIFT)
- Files scanned vs files with findings
```

#### Agent 4: Clarity & Consistency Review

```
You are reviewing a plan for ori_term (a GPU-accelerated terminal emulator in Rust) at {plan_dir}/.

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

### Step 10: Report Summary

Show the user:
- Files created (with paths)
- Summary of what each review agent changed
- Next steps (fill in details, add keywords to index)

---

## Example

**Input:** `/create-plan gpu-refactor "Restructure GPU rendering pipeline for per-window renderers"`

**Creates:**
```
plans/gpu-refactor/
+-- index.md
+-- 00-overview.md
+-- section-01-renderer-architecture.md
+-- section-02-atlas-management.md
+-- section-03-pipeline-consolidation.md
```

---

## Section Naming Conventions

| Section Type | Naming Pattern |
|--------------|----------------|
| Setup/Infrastructure | `section-01-setup.md` |
| Core Implementation | `section-02-core.md` |
| Integration | `section-03-integration.md` |
| Testing | `section-04-testing.md` |
| Documentation | `section-05-docs.md` |

---

## After Creation

Remind the user to:
1. Fill in section details with specific tasks
2. Add relevant keywords to `index.md` clusters
3. Update `00-overview.md` with dependencies and success criteria
4. **If performance-sensitive** (GPU rendering, VTE parsing, grid operations): Add benchmark/profiling checkpoints to relevant sections

## Performance-Sensitive Plans

For plans touching hot paths, include a "Performance Validation" section in `index.md`:

```markdown
## Performance Validation

Use profiling after modifying hot paths.

**When to benchmark:** [list specific sections]
**Skip benchmarks for:** [list non-perf sections]
```

See `plans/_template/plan.md` for full guidance.

---

## Template Reference

The command uses `plans/_template/plan.md` as the structure reference. See that file for:
- Complete index.md template
- Section file template
- Status conventions
- The roadmap (`plans/roadmap/`) as a working example
