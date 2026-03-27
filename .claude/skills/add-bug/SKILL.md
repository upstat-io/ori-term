---
name: add-bug
description: File a bug in the bug tracker plan. PROACTIVELY use this whenever you discover ANY bug, issue, broken behavior, missing wiring, incorrect rendering, or architectural violation during ANY work — code review, implementation, investigation, debugging, or testing. Don't be shy about filing bugs — they are re-verified at fix time, so false positives cost nothing. When in doubt, file it.
---

# Add Bug Command

Add a discovered bug to the parallel bug tracker plan. Each bug is categorized by domain/section and tracked as a checklist item. The bug tracker plan is never marked as finished.

## Usage

```
/add-bug [description]
```

- `description`: Optional one-line description of the bug. If omitted, you'll be asked to describe it.

---

## Workflow

### Step 0: Read CLAUDE.md

**Before doing ANYTHING else**, read the ENTIRE CLAUDE.md file:

```
Read file: CLAUDE.md
```

### Step 1: Gather Bug Details

If the bug description wasn't provided as an argument, use `AskUserQuestion` to ask:

1. **What's the bug?** — Clear description of the incorrect behavior
2. **Where is it?** — File path(s), function(s), or area of the codebase affected
3. **How was it discovered?** — Test failure, code review, user report, etc.

If the description WAS provided, research the bug yourself:
- Search the codebase for the affected area
- Read the relevant files to understand the current (buggy) behavior
- Identify the root cause if possible

### Step 2: Categorize the Bug

Determine which domain/category the bug belongs to. Categories map to sections in the bug tracker plan:

| Category | Covers |
|----------|--------|
| core | Grid, VTE handler, cell, palette, selection, search (`oriterm_core`) |
| ui-framework | Widget trait, layout solver, interaction, focus, animation (`oriterm_ui`) |
| ui-widgets | Button, toggle, dropdown, slider, dialog, settings panel, tab bar widgets |
| gpu | Renderer, atlas, shader pipelines, scene conversion (`oriterm_gpu`) |
| fonts | Font discovery, collection, shaping, rasterization, UI font sizes |
| mux | Pane server, PTY I/O, pane lifecycle, mux backend (`oriterm_mux`) |
| session | Tab/window management, split tree, floating panes, navigation |
| input | Keyboard encoding, mouse input, key dispatch, keybindings |
| icons | Icon path definitions, rasterization, resolution, SVG import |
| platform-windows | Windows-specific: ConPTY, DWM, title bar, named pipes |
| platform-macos | macOS-specific: vibrancy, traffic lights, app bundle |
| platform-linux | Linux-specific: Wayland/X11, PTY |
| config | Configuration loading, settings serialization, defaults |
| ipc | IPC transport, protocol, daemon mode (`oriterm_ipc`) |

If the bug doesn't fit an existing category, create a new one.

### Step 3: Check if Bug Tracker Plan Exists

Check for the bug tracker plan directory:

```
Glob: plans/bug-tracker/
```

**If the plan does NOT exist** — go to Step 4 (Create Plan Structure).
**If the plan exists** — go to Step 5 (Add Bug to Existing Plan).

### Step 4: Create Bug Tracker Plan Structure (First Time Only)

Create the directory and initial files:

#### 4a: Create `plans/bug-tracker/index.md`

```markdown
---
parallel: true
name: "Bug Tracker"
full_name: "Active Bug Tracker"
status: active
order: 999
---

# Bug Tracker Index

> **Maintenance Notice:** Update this index when adding new bug categories.

## How to Use

1. Search this file for keywords related to the bug area.
2. Find the matching section/category.
3. Open the section file.
4. Use `/continue-roadmap plans/bug-tracker` to work on fixes.

---

## Keyword Clusters by Section

### Section 01: {First Bug's Category}
**File:** `section-01-{category}.md` | **Status:** In Progress

```
{keywords for this category}
```

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | {Category Title} | `section-01-{category}.md` |
```

#### 4b: Create `plans/bug-tracker/00-overview.md`

```markdown
---
plan: "bug-tracker"
title: "Active Bug Tracker"
status: in-progress
references:
  - "CLAUDE.md"
---

# Active Bug Tracker

## Mission

Track and fix all discovered bugs across the ori_term codebase. This is a living plan — it is never marked as complete. Bugs are added as they are discovered and marked as fixed when resolved.

## How This Plan Works

- **Parallel plan**: Runs alongside the main roadmap. Never blocks normal work.
- **Never finished**: Sections and the plan itself are never marked `complete` or `status: complete`.
- **Categories as sections**: Each section represents a domain/area of the codebase.
- **Additive**: New bugs are appended to existing sections or new sections are created.
- **Fixed bugs**: Individual bug items are marked `[x]` with a resolution note, but the section stays `in-progress`.
- **Last choice in /continue-roadmap**: When presented as an option, bug fixes are always the last item in the choice list, never recommended as the top choice.

## Design Principles

- **Fix at source**: Every bug fix addresses the root cause, not a workaround.
- **No deferrals**: Per CLAUDE.md, discovered bugs are fixed immediately. This plan tracks them for visibility and prioritization when multiple bugs exist.
- **Test every fix**: Every bug fix includes a test that would have caught the bug.

## Quick Reference

| ID | Title | File | Bugs |
|----|-------|------|------|
| 01 | {Category} | `section-01-{category}.md` | 1 |
```

#### 4c: Create the first section file

See Step 5 for the section file format — create it with the first bug.

Then proceed to Step 6.

### Step 5: Add Bug to Existing Plan

#### 5a: Find or Create the Category Section

Search existing section files for the bug's category:

```
Glob: plans/bug-tracker/section-*.md
```

Read each section file's frontmatter to find a matching category.

**If matching section exists**: Read the file, find the next available bug ID within that section.

**If no matching section exists**: Determine the next section number (highest existing + 1) and create a new section file.

#### 5b: Bug Entry Format

Each bug is a checklist item with this format:

```markdown
- [ ] **BUG-{section}.{number}**: {Short description}
  - **File(s)**: `{file path(s)}`
  - **Root cause**: {Brief explanation of why this happens}
  - **Found**: {date} — {how discovered}
  - **Fix**: {Brief description of the fix needed, or "TBD — needs investigation"}
```

When a bug is fixed, update it to:

```markdown
- [x] **BUG-{section}.{number}**: {Short description}
  - **File(s)**: `{file path(s)}`
  - **Root cause**: {Brief explanation}
  - **Found**: {date} — {how discovered}
  - **Fixed**: {date} — {brief description of the fix applied}
```

#### 5c: Section File Format (for new sections)

```yaml
---
section: "{NN}"
title: "{Category Title} Bugs"
status: in-progress
reviewed: true
goal: "Track and fix bugs in the {category} domain"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "{NN}.1"
    title: "Active Bugs"
    status: in-progress
---

# Section {NN}: {Category Title} Bugs

**Status:** In Progress
**Goal:** Track and fix all discovered bugs in the {category} domain.

**Note:** This section is never marked complete. New bugs are appended as discovered.

---

## {NN}.1 Active Bugs

- [ ] **BUG-{NN}.1**: {First bug description}
  - **File(s)**: `{file path}`
  - **Root cause**: {explanation}
  - **Found**: {date} — {how discovered}
  - **Fix**: {description}
```

#### 5d: Update Index and Overview

After adding the bug:

1. **If new section was created**: Add keyword cluster and quick reference entry to `index.md`
2. **Update `00-overview.md`**: Update the Quick Reference table with current bug counts
3. **Update `index.md`**: Update the Quick Reference table

### Step 6: Confirm to User

Present a summary:

```
Bug added to tracker:

**BUG-{id}**: {description}
**Category**: {category} (Section {NN})
**File**: plans/bug-tracker/section-{NN}-{category}.md

To fix bugs: `/continue-roadmap plans/bug-tracker`
```

---

## Rules

### Never Complete

- The bug tracker plan is NEVER marked `status: complete`
- Individual sections are NEVER marked `status: complete`
- Individual bug items ARE marked `[x]` when fixed
- The plan-level and section-level status stays `in-progress` permanently

### Parallel Plan

- The `index.md` has `parallel: true` (not `reroute: true`)
- This means it runs alongside normal work, never blocks it
- `order: 999` ensures it's always last in priority

### /continue-roadmap Integration

When `/continue-roadmap` detects this parallel plan and presents options to the user:

- Bug tracker items are ALWAYS the **last option** in the multiple choice list
- Bug tracker items are NEVER the **recommended** option (never marked "Recommended")
- The option text should read: `Work on bug fixes (Bug Tracker — N open bugs)`
- Only present this option if there are unchecked (`[ ]`) bug items

### Bug IDs

- Format: `BUG-{section}.{sequential}`
- Example: `BUG-03.2` = Section 03, second bug
- IDs are never reused — if BUG-03.2 is fixed, the next bug in section 03 is BUG-03.3

### Categorization

- One bug per checklist item — don't combine related bugs
- If a bug spans multiple domains, put it in the domain where the root cause lives
- If unsure about categorization, ask the user
