---
description: Deep code review of recent changes with minimal context to avoid bias. Runs 4 sequential cold-start subagents, then writes validated findings into a plan section's Third Party Review block.
---

# Independent Review Command

Perform an unbiased code review via a 4-agent sequential pipeline. Each agent starts cold and receives only the raw materials needed to review the code. Findings flow forward through the pipeline. The main context performs final triage and writes validated findings into the target plan section's `## {NN}.R Third Party Review Findings` block.

## Usage

```
/independent-review [plan-path] [commit-range]
```

- `plan-path`: Optional. Path to the plan directory or section file that should receive the Third Party Review findings. If omitted, use the most recently modified plan directory under `plans/`.
- `commit-range`: Optional. Git revision range (for example, `HEAD~5..HEAD`). If omitted, review commits on the current branch since it diverged from `master`.

## Workflow

### Step 1: Gather Raw Materials

1. Determine the commit range:
   - If provided, use it directly.
   - Otherwise compute `MERGE_BASE=$(git merge-base master HEAD)` and review `"$MERGE_BASE..HEAD"`.
2. Collect:
   - `git diff $RANGE --stat`
   - `git log $RANGE --oneline`
3. If a plan path is available, extract a stripped plan summary containing only:
   - Mission statement from `00-overview.md`
   - Completion / exit criteria
   - Checklist items
   - File paths referenced in checklist items
4. Report scope to the user:
   - Commit count
   - Changed file count
   - Plan target (if any)
   - "Pipeline: 4 sequential review agents -> validated TPR writeback"

### Step 2: Agent Pipeline

Run 4 subagents in sequence using the Task tool. Each subagent:

- Receives only the raw materials above plus prior agent findings.
- Reads the actual changed files itself.
- Produces findings with file/line references and severity.
- Must not assume project context beyond the prompt.

Use this output format for every subagent:

```
**[{CATEGORY}]** `file:line` -- severity: {critical|major|minor}
Description.
What could go wrong in practice.
```

Categories: `CORRECTNESS`, `SAFETY`, `CONSISTENCY`, `TEST`, `GOAL-GAP`

Agent responsibilities:

1. Agent 1: Cold discovery over all changed files.
2. Agent 2: Verify Agent 1 findings, expand around confirmed areas, and find new issues.
3. Agent 3: Deep verification across callers/callees, tests, and adjacent modules.
4. Agent 4: Final cold pass with the accumulated finding set, confirming and broadening the search.

### Step 3: Full-Context Triage

After all 4 agents complete:

1. Deduplicate overlapping findings.
2. Validate each finding against the actual codebase.
3. Do **not** reject a finding because it is "unrelated", "out of scope", or "pre-existing". Reject only findings that are factually incorrect.
4. Rank validated findings by severity.

### Step 4: Write Back to TPR

If a plan target is available, write the validated findings into that plan section's `## {NN}.R Third Party Review Findings` block.

Formatting rules:

- New unresolved finding:
  ```markdown
  - [ ] `[TPR-{NN}-001][high]` `path/to/file.rs:123` -- Finding summary.
    Validation: Confirmed during /independent-review on YYYY-MM-DD.
  ```
- Rejected finding:
  ```markdown
  - [x] `[TPR-{NN}-002][medium]` `path/to/file.rs:456` -- Finding summary.
    Resolved: Rejected on YYYY-MM-DD after validation. {Why the issue does not actually exist.}
  ```

Frontmatter rules:

- If any unchecked TPR items exist, set `third_party_review.status: findings`.
- If all TPR items are resolved, set `third_party_review.status: resolved`.
- If the block is empty / `- None.`, set `third_party_review.status: none`.
- Always set `third_party_review.updated` to today's date when modifying the block.

### Step 5: Report Results

Return:

- Reviewed commit range
- Files reviewed
- Validated findings by severity
- Plan file updated (if any)
- Whether `third_party_review.status` changed
