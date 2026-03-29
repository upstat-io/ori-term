---
name: add-bug
description: Add a bug to the bug-tracker plan. Minimal research at add-time — capture repro, location, severity, and source. TRIGGER proactively when ANY bug is encountered during ANY work — unrelated bugs, edge cases, test failures, suspicious behavior, code smells that look like bugs. If in doubt, file it. Better safe than sorry — verification happens at review time.
allowed-tools: Read, Grep, Glob, Edit, Write, Bash
argument-hint: "[description or file:line]"
---

# Add Bug

File a bug in `plans/bug-tracker/` under the correct domain section.

## Proactive Triggering — MANDATORY

This skill MUST be invoked proactively whenever you encounter a bug that is **not part of your current task**. Do NOT:
- Gloss over it as "not related"
- Note it mentally and move on
- Say "this is a separate issue" without filing
- Assume someone else will catch it
- Skip it because you're "in the middle of something"

**If in doubt, file it.** Verification happens when bugs are reviewed (`/review-bugs`). A false positive costs nothing; a missed bug costs everything.

### When to trigger (non-exhaustive)
- You see a test failure unrelated to your current work
- You notice suspicious behavior while reading code
- A code review or exploration reveals unexpected output
- You encounter an edge case that probably doesn't work
- You find a TODO/FIXME/HACK comment that describes an unfixed bug
- You notice a rendering glitch or incorrect layout behavior
- You find a mismatch between expected and actual widget behavior
- A platform-specific code path looks broken or incomplete

## Usage

```
/add-bug [description]
```

The description can be:
- A free-text bug description: `/add-bug tab bar doesn't render close buttons on hover`
- A file reference: `/add-bug oriterm_ui/src/widgets/button/mod.rs:45 — click doesn't fire on keyboard Enter`
- Context from the current conversation (no args needed if a bug was just discussed)

## Workflow

### Step 1: Determine Domain

Map the bug to one of the domain sections:

| Section | Domain | Crates/Paths |
|---------|--------|--------------|
| 01 | Core | Grid, VTE handler, cell, palette, selection, search (`oriterm_core`) |
| 02 | UI Framework | Widget trait, layout solver, interaction, focus, animation (`oriterm_ui`) |
| 03 | UI Widgets | Button, toggle, dropdown, slider, dialog, settings panel, tab bar widgets |
| 04 | GPU | Renderer, atlas, shader pipelines, scene conversion (`oriterm_gpu`) |
| 05 | Fonts | Font discovery, collection, shaping, rasterization, UI font sizes |
| 06 | Mux | Pane server, PTY I/O, pane lifecycle, mux backend (`oriterm_mux`) |
| 07 | Session | Tab/window management, split tree, floating panes, navigation |
| 08 | Input | Keyboard encoding, mouse input, key dispatch, keybindings |
| 09 | Icons | Icon path definitions, rasterization, resolution, SVG import |
| 10 | Platform Windows | ConPTY, DWM, title bar, named pipes |
| 11 | Platform macOS | Vibrancy, traffic lights, app bundle |
| 12 | Platform Linux | Wayland/X11, PTY |
| 13 | Config | Configuration loading, settings serialization, defaults |
| 14 | IPC | IPC transport, protocol, daemon mode (`oriterm_ipc`) |

If unclear, check the file path or ask. If it spans domains, file in the one where the **fix** belongs (not where the symptom appears). If the bug doesn't fit an existing category, create a new one.

### Step 2: Check for Duplicates

Before adding, scan the target section file for existing bugs that match:

```
Read plans/bug-tracker/section-{NN}-*.md
```

If a duplicate exists, note it to the user instead of adding a new entry.

### Step 3: Assign ID and Severity

**ID format:** `BUG-{section}-{ordinal}` — ordinal is the next sequential number in that section (count existing bugs + 1).

**Severity:**
- `critical` — blocks correctness, data corruption, crash, unusable feature
- `high` — wrong output, silent failure, should fix when touching adjacent code
- `medium` — edge case failure, workaround exists, fix opportunistically
- `low` — cosmetic, minor inconvenience, tracked for dedicated passes

### Step 4: Minimal Research

Do just enough to write a useful bug entry. DO NOT deep-dive — the code may change before the fix:

1. Confirm the bug exists (quick grep or test run if trivial)
2. Identify the approximate location (crate + file, not exact line)
3. Note any obvious repro steps

### Step 5: Write the Bug Entry

Append to the `## Open Bugs` or `## {NN}.1 Active Bugs` section of the target file:

```markdown
- [ ] `[BUG-{section}-{ordinal}][{severity}]` **{Short title}** — found by {source}.
  Repro: {test file path or minimal repro steps}
  Subsystem: {crate/file path}
  Found: {YYYY-MM-DD} | Source: {tpr-review | manual | continue-roadmap | review-work}
```

**Source values:**
- `tpr-review` — found by Codex TPR
- `manual` — found by the user or during manual work
- `continue-roadmap` — found while working on the roadmap
- `review-work` — found by /review-work or code review

### Step 6: Cross-Reference Check

Quick check: is there an active roadmap section or reroute plan touching this area?

```
Grep for the affected file/function in plans/roadmap/section-*.md and plans/*/section-*.md
```

If an active plan section covers this area, note it in the bug entry:
```markdown
  Note: Active work in roadmap section {NN} touches this area.
```

### Step 7: Update Index and Overview

After adding the bug:

1. **If new section was created**: Add keyword cluster and quick reference entry to `index.md`
2. **Update `00-overview.md`**: Update the Quick Reference table with current bug counts
3. **Update `index.md`**: Update the Quick Reference table

### Step 8: Confirm to User

Report what was filed:
```
Filed: [BUG-{section}-{ordinal}][{severity}] {title}
  Section: {section name} (plans/bug-tracker/section-{NN}-*.md)
  Cross-ref: {any active plan sections, or "none"}
```

---

## Bug Tracker Plan Structure

If the bug tracker plan doesn't exist yet, create it following the structure in the existing SKILL.md (Step 4 in old version). The key properties:

- `parallel: true` in `index.md` (not `reroute: true`)
- `order: 999` (always last in priority)
- Never marked `status: complete` — it's a living plan
- Individual bug items ARE marked `[x]` when fixed
- Section-level status stays `in-progress` permanently

### Bug Entry Format (when fixed)

```markdown
- [x] `[BUG-{section}-{ordinal}][{severity}]` **{Short title}** — found by {source}.
  Resolved: {YYYY-MM-DD}. {What fixed it — commit, plan, or rewrite}.
```

### Bug IDs

- Format: `BUG-{section}-{sequential}`
- Example: `BUG-03-2` = Section 03, second bug
- IDs are never reused

### /continue-roadmap Integration

When `/continue-roadmap` detects this parallel plan and presents options:
- Bug tracker items are ALWAYS the **last option** in the multiple choice list
- Bug tracker items are NEVER the **recommended** option
- The option text should read: `Work on bug fixes (Bug Tracker — N open bugs)`
- Only present this option if there are unchecked (`[ ]`) bug items
