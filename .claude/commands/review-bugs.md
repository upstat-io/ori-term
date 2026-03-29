---
name: review-bugs
description: Review open bugs in the bug-tracker plan — check for OBE, prioritize, and decide what to fix.
allowed-tools: Read, Grep, Glob, Bash, Edit, Write, AskUserQuestion
argument-hint: "[domain or 'all']"
---

# Review Bugs

Review open bugs in `plans/bug-tracker/`, check for OBE (Overtaken By Events), and help prioritize what to fix.

## Usage

```
/review-bugs [target]
```

- No args: review all domains
- `01` or `core`: review Core bugs only
- `02` or `ui-framework`: review UI Framework bugs only
- `all`: review everything
- `critical`: review only critical/high bugs across all domains

## Workflow

### Step 1: Gather Open Bugs

Read each section file (or the targeted one) and collect all `- [ ]` items:

```
plans/bug-tracker/section-01-core.md
plans/bug-tracker/section-02-ui-framework.md
plans/bug-tracker/section-03-ui-widgets.md
plans/bug-tracker/section-04-gpu.md
plans/bug-tracker/section-05-fonts.md
plans/bug-tracker/section-06-mux.md
plans/bug-tracker/section-07-session.md
plans/bug-tracker/section-08-input.md
plans/bug-tracker/section-09-icons.md
plans/bug-tracker/section-10-platform-windows.md
plans/bug-tracker/section-11-platform-macos.md
plans/bug-tracker/section-12-platform-linux.md
plans/bug-tracker/section-13-config.md
plans/bug-tracker/section-14-ipc.md
```

Not all sections may exist — only read what's present.

### Step 2: OBE Check

For each open bug, check if it's been overtaken by events:

1. **Check if the affected code was rewritten** — has the file/function been significantly changed since the bug was filed?
   ```bash
   git log --oneline --since="{found_date}" -- {subsystem_path} | head -10
   ```

2. **Check if a recent plan fixed it** — grep completed plans for the bug area:
   ```bash
   grep -r "{keyword}" plans/completed/ | head -5
   ```

3. **Run a quick test** if the bug has a clear repro:
   ```bash
   timeout 30 cargo test {test_name} 2>&1 | tail -5
   ```

If the bug is OBE, mark it resolved:
```markdown
- [x] `[BUG-{section}-{ordinal}][{severity}]` **{title}** — found by {source}.
  Resolved: OBE on {YYYY-MM-DD}. {What fixed it — commit, plan, or rewrite}.
```

### Step 3: Validate Remaining Bugs

For bugs that aren't OBE, do a quick sanity check:
- Is the severity still accurate? (code changes may have made it worse or better)
- Is the domain assignment still correct? (code may have moved)
- Is the repro still valid?

Update entries if needed.

### Step 4: Present Summary

```
## Bug Tracker Review — {date}

### Summary
- Total open: {N}
- Checked for OBE: {N}
- Resolved (OBE): {N}
- Still open: {N}

### By Severity
- Critical: {N} {list titles}
- High: {N} {list titles}
- Medium: {N}
- Low: {N}

### By Domain
| Domain | Open | Critical | High |
|--------|------|----------|------|
| Core | {N} | {N} | {N} |
| UI Framework | {N} | {N} | {N} |
| ... | | | |

### OBE Resolutions
{List of bugs resolved as OBE with brief explanation}

### Recommended Actions
{Prioritized list of bugs worth fixing now, considering:
 - Critical bugs block work
 - High bugs in areas with active roadmap sections
 - Clusters of bugs in the same file/function (fix together)
}
```

### Step 5: Ask What to Do

Use AskUserQuestion with options:
1. **Fix a specific bug** — pick one to work on now
2. **Fix all critical bugs** — work through critical bugs in priority order
3. **Done reviewing** — no action needed right now
