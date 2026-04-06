---
name: tpr-review
description: "Run a third-party review via Codex CLI — TRIGGER proactively after completing ANY non-trivial work: bug fixes, new features, refactors, multi-file changes, changes to rendering, grid, mux, UI, GPU, or anything touching correctness-sensitive code. When in doubt, run it. The cost of an unnecessary review is near zero; the cost of a missed bug is high."
allowed-tools: Bash, Read, Edit, Write, Grep, Glob
---

# TPR Review via Codex

Run the Codex CLI non-interactively to perform an independent review-work pass, then fix any findings and re-run until clean. Codex has its own context, rules, and skills — it will figure out scope on its own.

## Step 0 — MANDATORY: Re-read CLAUDE.md

**Before doing ANYTHING else, re-read the entire project CLAUDE.md.** This is non-negotiable. Even if you believe it is in memory, you MUST physically read it with the Read tool. Context compression may have dropped critical rules. Do this every single time this skill runs.

```
Read CLAUDE.md (the project root one)
```

## ABSOLUTE: You May NEVER Reason Out of Findings

**There is NO circumstance under which you may dismiss, rationalize, scope-note, or defer a TPR finding.** The ONLY valid responses to a finding are:

1. **Fix it NOW** — write code, write tests, verify, commit
2. **Create a plan and execute it** — if too large for inline fix, create concrete implementation steps, then implement them
3. **AskUserQuestion** — if genuinely blocked (need user decision, missing domain knowledge)

**BANNED responses to findings — using ANY of these is a violation:**
- "Pre-existing issue" / "was already broken"
- "Architectural limitation" / "requires major refactor"
- "Out of scope" / "not related to current work"
- "Conservative/safe" / "only minor impact"
- "Not a regression" / "not introduced by this work"
- "Future improvement" / "tracked for later"
- "Scoped as known limitation"
- Marking `[x] Resolved:` with an explanation instead of a code fix

**The size of the fix is irrelevant.** If the correct fix requires cross-crate refactoring across 10 files, that IS the work. "Requires architectural change" is not a reason to skip — it IS the work.

**"Future improvement" requires a concrete artifact.** If you ever say something will be tracked, you MUST in the same response create: a bug-tracker entry (`/add-bug`), plan section `- [ ]` item, or roadmap checkbox. Ask yourself: "When would this get done? Who would find it?" If nobody/never, fix it now.

## ABSOLUTE: Correct Architectural Solutions Only

**Before fixing ANY finding, read `.claude/rules/impl-hygiene.md`.** This is non-negotiable. The hygiene rules define module boundaries, canonical homes, rendering discipline, and finding categories (BLOAT, WASTE, DRIFT, EXPOSURE, LEAK, STYLE). Every fix must respect these principles.

**Fixes must be the correct, proper architectural solution — never quick fixes, workarounds, counters, flags, or hacks.** Specifically:

- **Module boundaries**: if the finding reveals logic in the wrong crate, the fix is to move it — not to patch it in place
- **Crate ownership**: oriterm_core owns terminal emulation, oriterm_ui owns widgets/interaction, oriterm_mux owns pane I/O, oriterm owns app shell/GPU. Fixes must respect these boundaries.
- **No side logic**: if logic lives outside its canonical home, the fix is to move it — not to add another copy that "works"
- **Rendering discipline**: pure computation in draw paths, no allocation in hot loops, buffer output
- **Cross-platform**: every `#[cfg(target_os)]` must have counterparts for all three platforms

**The "quick fix" test**: if your fix would not survive a code review by someone who has read `impl-hygiene.md`, it's wrong. The correct fix may touch 10 files across 3 crates — that IS the fix. A workaround that passes tests is not a fix.

## When to Trigger — Bias Toward Running

**Run this skill after completing ANY of the following:**
- Bug fixes (any severity)
- New features or feature extensions
- Refactors or code reorganization
- Multi-file changes (2+ files)
- Any change to core crates (`oriterm_core`, `oriterm_ui`, `oriterm_mux`, `oriterm_gpu`)
- Any change to rendering, grid, or terminal emulation
- Any change to the mux/PTY pipeline
- Test additions or test infrastructure changes
- Plan section implementations
- Widget or interaction system changes

**Also run when:**
- You're unsure whether the change warrants review (default: run it)
- The work involved multiple steps or non-obvious decisions
- The change touches code paths shared across subsystems
- You fixed something that was interfering with other code

**The only time NOT to run:** purely cosmetic single-line changes (typo fixes, comment edits, formatting-only).

## Loop Protocol — MANDATORY

```
+---------------------------------------------------------+
|                   TPR REVIEW LOOP                        |
|                                                          |
|  0. CLAUDE re-reads CLAUDE.md (MANDATORY)                |
|        |                                                 |
|  1. CODEX reviews (independent, external)                |
|        |                                                 |
|  2. CLAUDE reads findings                                |
|        |                                                 |
|  3. Zero findings? --YES--> DONE (clean pass)            |
|        |                                                 |
|       NO                                                 |
|        |                                                 |
|  4. CLAUDE files findings in plan/bug-tracker            |
|  5. CLAUDE fixes each finding (code + tests)             |
|  6. CLAUDE commits fixes via /commit-push                |
|        |                                                 |
|  7. Go to step 1 (CODEX re-reviews the fixed code)      |
|                                                          |
+---------------------------------------------------------+
```

**Two actors:**
- **Codex** (external reviewer): runs `/review-work`, produces findings. Does NOT fix anything.
- **Claude** (you): reads Codex's findings, fixes the code, commits, then invokes Codex again.

**A TPR review is NOT complete until Codex produces zero actionable findings.** Filing findings without fixing and re-running is deferral. Fixing findings without re-running Codex to confirm clean is incomplete.

**Maximum iterations: 10.** If after 10 cycles findings are still surfacing, present the remaining findings to the user via AskUserQuestion and ask how to proceed.

## Steps (Per Iteration)

### 1. Run Codex

**CRITICAL: Use an Agent, NOT direct Bash.** The Bash tool has a 120-second default timeout that kills codex before it finishes. Spawn a foreground Agent that runs the codex command.

**IMPORTANT: The Agent MUST be foreground (NOT `run_in_background`).** The user wants to see all Codex output as it runs.

```
Launch Agent with prompt:
  "Run the following codex command and return the full output:
   
   codex exec 'run the /review-work skill' --full-auto --json 2>/dev/null | tail -200
   
   Then parse the JSONL output to extract agent_message items:
   
   cat <output> | python3 -c \"
   import sys, json
   for line in sys.stdin:
       line = line.strip()
       if not line: continue
       try:
           obj = json.loads(line)
           if obj.get('type') == 'item.completed' and obj.get('item', {}).get('type') == 'agent_message':
               print(obj['item']['text'])
       except json.JSONDecodeError: pass
   \" | tail -3000
   
   Return the extracted messages."
```

### 2. Parse Output

Extract the final agent messages from the JSONL output — the last few `agent_message` items contain the findings summary.

### 3. Classify Findings

For each finding in the Codex output, determine if it's actionable:

- **Actionable finding**: a real code issue — bug, hygiene violation, missing test, incorrect behavior, file size limit exceeded, dead code path, etc. Must be fixed.
- **Non-actionable observation**: a style preference or observation about behavior that isn't a defect AND isn't dead code. Note it but don't block the loop on it.

**IMPORTANT: Err on the side of "actionable".** If you're unsure, it's actionable. The following are ALWAYS actionable:
- Dead code paths (code that can never execute)
- Performance regressions (allocation in hot paths, unnecessary redraws)
- Missing tests for new functionality
- Crate boundary violations (logic in wrong crate)
- Cross-platform gaps (missing `#[cfg]` counterparts)

### 4. If Zero Actionable Findings -> Clean Pass (EXIT)

Report to the user:
- "TPR review passed clean — no actionable findings."
- Note the iteration count (e.g., "Clean on iteration 1" or "Clean on iteration 3 after fixing N findings").
- **This is the ONLY exit from the loop.**

### 5. If Actionable Findings Exist -> Fix and Re-run

#### 5a. File Findings

For each validated finding:

1. **Check if an owning plan section exists** — is there an active plan (roadmap or reroute) with a section covering the affected code?
2. **If yes** — record as a TPR finding in that section's `## {NN}.R Third Party Review Findings` block:
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

#### 5b. Fix Each Finding

**YOU (Claude) fix the code.** This means actual implementation — not just filing. Not scope notes. Not rationalizations. CODE CHANGES.

- **Read `.claude/rules/impl-hygiene.md` before fixing** — understand module boundaries, canonical homes, rendering discipline. Every fix must be the correct architectural solution, not a quick patch.
- Read the affected code and understand the issue
- Identify the **canonical home** for the knowledge/logic involved — fix must respect it
- Follow TDD if appropriate (write failing test -> fix -> test passes)
- Run `timeout 150 ./test-all.sh` after fixes
- **Self-check**: would this fix survive a `/impl-hygiene-review`? If it introduces scattered knowledge, duplicated dispatch, or a shadow source of truth, it's wrong — find the proper architectural fix
- Mark the TPR finding as `[x]` resolved in the plan with a note:
  ```md
  - [x] `[TPR-03-038][medium]` ...
    Resolved: Fixed on YYYY-MM-DD. [description of CODE fix].
  ```

#### 5c. Commit Fixes

Run `/commit-push` to commit the fixes. The commit message should reference the TPR IDs fixed.

#### 5d. Re-run Codex (GO TO STEP 1)

Go back to Step 1. Codex reviews the FIXED code to confirm the issues are actually resolved and no new issues were introduced by the fixes. **This re-run is not optional.**

### 6. Report (After Loop Exits)

Tell the user:
- Total iterations run
- Findings surfaced and fixed per iteration
- Final status: "clean" or "max iterations reached with N remaining findings"
- Where each finding was filed (plan TPR section or bug-tracker)
