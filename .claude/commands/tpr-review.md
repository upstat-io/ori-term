---
name: tpr-review
description: Run a third-party review via Codex CLI (codex exec) using the review-work skill — fully independent, zero shared context.
allowed-tools: Bash, Read, Edit, Write, Grep, Glob
---

# TPR Review via Codex

Run the Codex CLI non-interactively to perform an independent review-work pass. Codex has its own context, rules, and skills — it will figure out scope on its own.

## Steps

### 1. Run Codex

```bash
codex exec "run the /review-work skill" --full-auto --json
```

### 2. Parse Output

Read the JSONL output. Extract `agent_message` items (type: `item.completed`, item.type: `agent_message`) — the last few messages contain the findings.

### 3. Present Summary

Summarize findings to the user with severity, file references, and reviewer consensus.

### 4. File Findings

For each validated finding from the Codex review:

1. **Check if an owning plan section exists** — is there an active plan (roadmap or reroute) with a section covering the affected code?
2. **If yes** — record as a TPR finding in that section's `Third Party Review Findings` block using standard TPR format:
   ```md
   - [ ] `[TPR-{section}-{ordinal}][{severity}]` `file:line` — Finding summary.
     Evidence: {from Codex output}
     Impact: {from Codex output}
   ```
   Update plan metadata (`third_party_review.status: findings`, `updated: {today}`).

3. **If no owning plan exists** — file as a bug in `plans/bug-tracker/` under the appropriate domain section:
   ```md
   - [ ] `[BUG-{section}-{ordinal}][{severity}]` **{Short title}** — found by tpr-review.
     Repro: {from Codex output}
     Subsystem: {crate/file path}
     Found: {YYYY-MM-DD} | Source: tpr-review
   ```

   Domain mapping:
   - `oriterm_core` → section matching core domain
   - `oriterm_ui/src/widgets/` → ui-widgets domain
   - `oriterm_ui` (other) → ui-framework domain
   - `oriterm_gpu` → gpu domain
   - `oriterm_mux` → mux domain
   - `oriterm_ipc` → ipc domain
   - `oriterm/src/session/` → session domain
   - `oriterm/src/app/` → check specific area (input, platform, etc.)
   - Platform-specific (`#[cfg(target_os)]`) → matching platform domain

### 5. Report

Tell the user:
- How many findings were surfaced
- Where each was filed (plan TPR section or bug-tracker)
- Any that couldn't be classified (present for manual decision)
