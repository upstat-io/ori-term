# /fix-bug — Phase 0: Bug Context Setup (Sonnet Sub-agent)

**This file is read by the Sonnet sub-agent dispatched from `SKILL.md`.** Execute Phase 0 end-to-end and return the structured handoff. Do NOT proceed past Phase 0 — the parent (Opus) handles all investigation, TDD, implementation, and reviews.

**You do NOT:**
- Read compiler source files (`.rs`, `.ori`, `compiler/`, `library/`, `tests/`)
- Analyze root cause, write tests, or write code
- Run `git commit` directly — commits go through `/commit-push`
- Execute any phase beyond Phase 0

**You DO:**
- Read `plans/bug-tracker/` section files (markdown reads only)
- Check for existing fix section files
- Run quick greps against plan files
- Compile the full bug context for the parent Opus agent

---

## Phase 0: Locate and Extract Bug Context

### Step 1: Find the Bug Entry

If a BUG-ID was provided (e.g., `BUG-04-033`), determine the section number and read the corresponding file:

| Section | File |
|---------|------|
| 01 | `plans/bug-tracker/section-01-parser-lexer.md` |
| 02 | `plans/bug-tracker/section-02-typeck.md` |
| 03 | `plans/bug-tracker/section-03-eval.md` |
| 04 | `plans/bug-tracker/section-04-codegen-llvm.md` |
| 05 | `plans/bug-tracker/section-05-runtime-arc.md` |
| 06 | `plans/bug-tracker/section-06-stdlib.md` |
| 07 | `plans/bug-tracker/section-07-tooling-cli.md` |
| 08 | `plans/bug-tracker/section-08-spec-docs.md` |

If a description was provided instead of an ID, search all section files for the closest matching entry.

If no bug entry exists at all: note this in the handoff — the parent will create one.

### Step 2: Extract Full Bug Context

From the matching `- [ ]` or `- [x]` entry and its indented body, extract:
- **Bug ID**: `BUG-{section}-{ordinal}`
- **Severity**: as labeled
- **Title**: the bold text after severity tag
- **Repro**: repro line (test file path or repro steps)
- **Subsystem**: crate/file path
- **Found**: date
- **Source**: provenance value
- **Notes**: any `Escalated:`, `Blocked:`, `Note:`, `Fix:` lines in the entry body
- **Full entry text**: the complete raw markdown of the entire entry (checkbox line + all indented body lines)

### Step 3: Check Status Flags

Evaluate these flags for the handoff. **Precedence order (first match wins):** Superseded > Lifecycle markers > Existing fix file (Resume).

1. **Already resolved**: Is the entry `- [x]`?
2. **Superseded by**: Does the body contain a `Superseded by:` line pointing at a plan path? Match the EXACT marker `Superseded by:` (case-sensitive, with the trailing colon and space). Capture the plan path. **ALSO** cross-check: grep `^supersedes:` (and the following `  - "plans/bug-tracker/fix-BUG-{section}-{ordinal}` lines) across all `plans/**/00-overview.md` files — if any plan declares this fix file as a supersede target, treat that as superseded even if the bug entry lacks the marker (the plan frontmatter is the SSOT; the bug entry should mirror it). If both checks find a target plan path, they MUST agree; mismatch is a documentation drift finding.
3. **Lifecycle markers** (other than Superseded): Does the body contain `Escalated to plan:`, `Escalated:`, `**Blocked**:` (the marker — note trailing colon distinguishes it from `**BLOCKER**:` informational impact-statement text), `Blocked:`, or `<!-- blocked-by:`? Scan the ENTIRE multi-line entry. **DO NOT match `**BLOCKER**:`** (uppercase, no `**Blocked**:` substring with trailing colon-space-text-pattern); that prefix is informational impact-statement text used in many entries and is NOT a lifecycle marker. The marker test: `**Blocked**:` is followed by a reason explaining why the bug cannot proceed; `**BLOCKER**:` is followed by impact text describing what the bug blocks.
4. **Existing fix file**: Does `plans/bug-tracker/fix-BUG-{section}-{ordinal}.md` exist? If yes, read its frontmatter (`status`, `severity`) and first ~20 lines to determine what phase was reached.
5. **Resume mode**: If the fix file exists, has `status: in-progress`, AND no Superseded marker fired in step 2, this is a RESUME — not a fresh start. **If Superseded fired, force Resume mode to `no — superseded` regardless of the fix file's status** — superseded fix files are fossils, the plan supersedes them.

### Step 4: Check Active Plan Context

Quick grep: is there an active roadmap or reroute plan section touching the bug's subsystem crate/file?

```bash
grep -r "{subsystem_crate_or_path_fragment}" plans/roadmap/section-*.md plans/*/section-*.md 2>/dev/null | head -5
```

Note any matching active plan sections for the parent.

### Step 5: Check the Overview

Read `plans/bug-tracker/00-overview.md` — extract the current open bug count for the bug's section (used by the parent for Phase 5 overview update).

---

## Return the Handoff

Return this EXACT format — every field must be present:

```
## Handoff to parent (Opus) — fix-bug Phase 0

**Bug ID**: {BUG-XX-NNN or "not found — will create from description: {text}"}
**Title**: {title or "n/a"}
**Severity**: {critical|high|medium|low or "n/a"}
**Section file**: {plans/bug-tracker/section-NN-*.md}
**Subsystem**: {crate/file path or "n/a"}
**Repro**: {repro steps/path or "n/a"}
**Found**: {YYYY-MM-DD or "n/a"}
**Source**: {provenance value or "n/a"}

**Status flags**:
- Already resolved: {yes | no}
- Superseded by: {none | `plans/{plan-name}/` — sourced from {bug-entry marker | plan frontmatter | both (must agree)}}
- Lifecycle markers (other than Superseded): {none | list them}
- Existing fix file: {plans/bug-tracker/fix-BUG-XX-NNN.md (status: {status}, reached phase: {N}) | none}
- Resume mode: {yes — pick up at Phase {N} | no — fresh start | no — superseded (forced; fix file is fossil)}

**Active plan context**: {plan/section paths touching this subsystem, or "none"}

**Overview open count for this section**: {N open bugs in section NN}

**Full bug entry text** (verbatim):
{The complete raw markdown of the bug entry, including the - [ ] line and all indented body lines}
```

**If the bug entry was not found** (no ID match, no description match), return an error handoff:

```
## Handoff to parent (Opus) — fix-bug Phase 0 ERROR

**Bug not found**: {ID or description provided}
**Searched**: {list of section files checked}
**Closest match**: {closest matching entry title + ID, or "none"}
```
