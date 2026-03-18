# Create Plan Command

Create a new plan directory with index and section files using the standard template.

## Usage

```
/create-plan <name> [description]
```

- `name`: Directory name for the plan (kebab-case, e.g., `gpu-refactor`, `mux-architecture`)
- `description`: Optional one-line description of the plan's goal

## Workflow

### Step 1: Gather Information

If not provided via arguments, ask the user:

1. **Plan name** — kebab-case directory name
2. **Plan title** — Human-readable title (e.g., "GPU Renderer Refactor")
3. **Goal** — One-line description of what this plan accomplishes
4. **Sections** — List of major sections (at least 2-3)

Use AskUserQuestion if needed to clarify scope.

### Step 2: Read the Template

Read `plans/_template/plan.md` for the structure reference.

### Step 3: Load Hygiene Rules

The full rule set is embedded below (source of truth files — do not maintain separate copies). Use these rules when structuring plan sections to ensure plans account for module boundary discipline, file size limits, rendering pipeline purity, and other hygiene requirements from the start.

**Implementation Hygiene Rules** (`.claude/rules/impl-hygiene.md`):
@.claude/rules/impl-hygiene.md

**Code Hygiene Rules** (`.claude/rules/code-hygiene.md`):
@.claude/rules/code-hygiene.md

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
- **Reroute frontmatter** (if this is a reroute plan — i.e., a parallel track alongside the main roadmap):
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
  `order` controls queue priority — lower value = promoted first (default 999 if omitted).
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
- YAML frontmatter (section ID, title, status: not-started, goal)
- **`reviewed: true` for Section 01 ONLY** — it's the starting point and was vetted during plan creation
- **`reviewed: false` for ALL other sections** — they need re-review before implementation because earlier sections will cause deviations that have downstream impacts
- Section header with status emoji
- Placeholder subsections with `- [ ]` checkboxes
- Completion checklist at the end

**Why the reviewed gate matters:** As you implement sections sequentially, reality diverges from the original plan — you discover new constraints, make architectural decisions, and deviate from assumptions. Later sections were written against the *original* assumptions, not the *actual* state after prior sections landed. `reviewed: false` forces a review checkpoint before each section to catch stale assumptions, incorrect file paths, wrong dependencies, and outdated design decisions. Without this gate, you'd implement plans that are already wrong.

### Step 8: Report Progress

Show the user:
- Files created
- Note: "Running 4 independent review passes..."

### Step 9: Sequential Independent Review (4 Agents)

After the plan is fully created, run **4 review agents in sequence** (NOT parallel). Each agent:

- Receives **only the plan files** — no conversation context, no reasoning behind the plan
- Is instructed to **read the plan, review it, and edit the files directly** to fix issues
- Sees edits made by all previous agents (because they run sequentially)

This creates an iterative refinement pipeline: each reviewer builds on the last.

**IMPORTANT**: Run these agents ONE AT A TIME. Wait for each to complete before starting the next.

#### Agent 1: Technical Accuracy Review

Spawn an Agent with the following prompt (substitute `{plan_dir}` with the actual plan directory path):

```
You are reviewing a plan for ori_term (a GPU-accelerated terminal emulator in Rust) at {plan_dir}/.

INSTRUCTIONS:
1. Read ALL files in {plan_dir}/ (index.md, 00-overview.md, and all section-*.md files)
2. Cross-reference every technical claim against the actual codebase:
   - Do referenced files, types, functions, modules exist?
   - Are module dependency assumptions correct? (grid, gpu/, app, tab, etc.)
   - Are described code patterns accurate?
3. Check claims against reference terminal emulators in ~/projects/reference_repos/console_repos/ (Alacritty, WezTerm, Ghostty)
4. For every inaccuracy found, EDIT the plan files directly to fix them
5. If a section references nonexistent code paths or wrong file locations, correct them
6. Add a brief comment near each fix: <!-- reviewed: accuracy fix -->

You may add missing sections, expand scope, or restructure if the plan is genuinely incomplete.
After editing, list what you changed and why.
```

#### Agent 2: Completeness & Gap Review

```
You are reviewing a plan for ori_term (a GPU-accelerated terminal emulator in Rust) at {plan_dir}/.

INSTRUCTIONS:
1. Read ALL files in {plan_dir}/ (index.md, 00-overview.md, and all section-*.md files)
2. Review each section for completeness:
   - Are there missing steps that would block implementation?
   - Are edge cases and error handling accounted for?
   - Are dependencies between sections correctly identified?
   - Are test strategies adequate for each section?
3. Check for missing sync points — if the plan adds enum variants, new types, or registration entries, does it list ALL locations that must be updated together?
4. For every gap found, EDIT the plan files directly to add the missing content
5. Add missing checklist items, missing steps, missing test requirements
6. Add a brief comment near each addition: <!-- reviewed: completeness fix -->

You may add new sections, restructure, or expand scope if the plan has genuine gaps.
After editing, list what you changed and why.
```

#### Agent 3: Hygiene & Feasibility Review

```
You are reviewing a plan for ori_term (a GPU-accelerated terminal emulator in Rust) at {plan_dir}/.

INSTRUCTIONS:
1. Read ALL files in {plan_dir}/ (index.md, 00-overview.md, and all section-*.md files)
2. Read the hygiene rules at .claude/rules/impl-hygiene.md and .claude/rules/code-hygiene.md
3. Review the plan against these rules:
   - Does the plan respect file size limits (500 lines)?
   - Does it maintain module boundary discipline?
   - Does it follow the test file conventions (sibling tests.rs)?
   - Are implementation steps ordered correctly (upstream before downstream)?
   - Are there steps that are impractical or underestimate complexity?
4. For every hygiene violation or feasibility concern, EDIT the plan files directly to fix them
5. Reorder steps if they violate module dependency ordering
6. Add warnings for steps that are particularly complex or risky
7. Add a brief comment near each change: <!-- reviewed: hygiene fix -->

You may expand scope, add sections, or restructure if needed to satisfy hygiene and feasibility requirements.
After editing, list what you changed and why.
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
