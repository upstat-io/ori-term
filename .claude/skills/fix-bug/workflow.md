# /fix-bug — Phase 0: Bug Context Setup (Sonnet Sub-agent)

**This file is read by the Sonnet sub-agent dispatched from `SKILL.md`.** Execute Phase 0 end-to-end and return the structured handoff. Do NOT proceed past Phase 0 — the parent (Opus) handles all investigation, TDD, implementation, and reviews.

**You do NOT:**
- Read compiler source files (`.rs`, `.ori`, `compiler/`, `library/`, `tests/`)
- Analyze root cause, write tests, or write code
- Run `git commit` directly — commits go through `/commit-push`
- Execute any phase beyond Phase 0

**You DO:**
- Read `plans/bug-tracker/` section files (tracker mode)
- Read `plans/<plan-name>/section-{NN}-*.md` files (inline mode — plan-owned blocker subsection)
- Check for existing fix section files (tracker mode only)
- Run quick greps against plan files
- Compile the full bug context for the parent Opus agent

---

## Phase 0 — Step 0: Dispatch Mode Detection

The caller's args can target one of two modes. Dispatch by inspecting the arg string:

| Arg shape | Mode | Example |
|---|---|---|
| `inline:<plan-section-path>#<subsection-id>` | **Inline-subsection mode** — plan-owned blocker | `inline:plans/foo/section-04-X.md#04.BLOCKER-1` |
| `<plan-section-path>#<subsection-id>` (no `inline:` prefix but starts with `plans/` and contains `#`) | **Inline-subsection mode** (shorthand) | `plans/foo/section-04-X.md#04.BLOCKER-1` |
| `BUG-XX-NNN` | **Tracker mode** | `BUG-04-033` |
| Any other free-form description | **Tracker mode** (description search) | `lambda capture type var survives` |

**Routing:**
- Inline-subsection mode → execute **Step 1b → Step 2b → Return the Inline Handoff** (skip Steps 1–5 below; those are tracker-only).
- Tracker mode → execute **Step 1 → Step 2 → Step 3 → Step 4 → Step 5 → Return the Handoff** (the original flow, unchanged).

---

## Tracker Mode — Steps 1–5 (original flow, unchanged)

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

---

## Inline Mode — Steps 1b + 2b (plan-owned blocker subsection)

Execute ONLY when Step 0 classified the arg as inline-subsection mode.

### Step 1b: Parse the Arg and Read the Plan Section

1. Strip the optional `inline:` prefix. Split the remaining string on `#` — the left side is `<plan-section-path>`, the right side is `<subsection-id>` (e.g. `04.BLOCKER-1`).
2. Verify `<plan-section-path>` starts with `plans/` and ends with `.md`. If not, return an INLINE ERROR handoff — invalid path shape.
3. Read the file:
   ```
   Read <plan-section-path>
   ```
4. Parse the YAML frontmatter:
   - Extract the top-level `section:` field (the section number, e.g. `"04"`).
   - Extract the full `sections:` list.
   - Find the entry whose `id:` equals `<subsection-id>`.
5. Validation gates (if ANY fail, return INLINE ERROR with the specific reason):
   - The `sections:` list must contain an entry matching `<subsection-id>`.
   - That entry's `kind:` must be `plan-blocker-inline`. If missing or different, the target is NOT an `/add-bug --inline` subsection — refuse to dispatch. Reason string: `"target subsection is not kind: plan-blocker-inline (got kind: {actual})"`.
   - The parent section's frontmatter `status:` must NOT be `complete`. If it is, reason string: `"parent plan section is complete — inline fix would reopen a closed section"`.

### Step 2b: Extract Subsection Body

1. Locate the body heading `## <subsection-id> — <title>` in the file.
2. Capture all lines from that heading up to (but not including) the next `## ` heading at the same level, or EOF if none.
3. Parse these fields from the captured body (regex/grep-style — do NOT deep-read):
   - `**Status:**` line → subsection status (`not-started`, `in-progress`, or `complete`)
   - `**Severity:**` line → severity
   - `**Found:**` line → date
   - `**Source:**` line → provenance
   - `**Cross-ref:**` line (optional) → tracker BUG-XX-NNN if any
   - `### Repro` block contents → repro steps/path
4. Also parse the body's skeleton state — for each of these headings, record whether the section is "pending" (placeholder text like "Pending — /fix-bug Phase X will fill") or "populated" (real content):
   - `### 1. Root Cause Analysis` → pending vs populated
   - `### 1.5 Fix Consensus` → pending vs populated
   - `### 2. TDD Matrix` → pending vs populated
   - `### 2.5 Fix Plan TPR` → pending vs populated
   - `### 3. Implementation` → pending vs populated
   - `### R. TPR Findings` → pending vs populated
5. Determine Resume mode from the skeleton state:
   - All sections pending → fresh start (Phase 1)
   - §1 populated, §1.5 pending → resume at Phase 1.75
   - §1.5 populated, §2 pending → resume at Phase 2
   - §2 populated, §2.5 pending → resume at Phase 2.5 (or skip if gate criteria not met — parent decides)
   - §2.5 populated, §3 pending → resume at Phase 3
   - §3 populated, §R pending → resume at Phase 4 or 5 (parent decides based on whether implementation commit exists)
   - §R populated AND subsection status is `complete` → already resolved

### Return the Inline Handoff

Return this EXACT format — every field must be present:

```
## Handoff to parent (Opus) — fix-bug Phase 0 [INLINE]

**Mode**: inline-subsection
**Plan section file**: {path}
**Section number**: {e.g. 04}
**Subsection ID**: {e.g. 04.BLOCKER-1}
**Kind**: plan-blocker-inline
**Title**: {from body heading "## {id} — {title}"}
**Severity**: {critical|high|medium|low}
**Found**: {YYYY-MM-DD}
**Source**: {canonical source value}
**Subsection status** (from frontmatter sections list): {not-started | in-progress | complete}
**Tracker cross-ref**: {BUG-XX-NNN | none}
**Repro**: {from body Repro block}

**Skeleton state** (which /fix-bug phases are populated vs pending):
- §1 Root Cause Analysis: {pending | populated}
- §1.5 Fix Consensus: {pending | populated}
- §2 TDD Matrix: {pending | populated}
- §2.5 Fix Plan TPR: {pending | populated}
- §3 Implementation: {pending | populated}
- §R TPR Findings: {pending | populated}

**Status flags**:
- Already resolved: {yes if subsection status: complete AND §R populated | no}
- Resume mode: {yes — pick up at Phase {N} based on skeleton state | no — fresh start from Phase 1}

**Full subsection body** (verbatim):
{all captured lines from `## {id}` to the next `## ` heading or EOF}
```

**INLINE ERROR handoff** (any Step 1b validation gate fails):

```
## Handoff to parent (Opus) — fix-bug Phase 0 INLINE ERROR

**Mode**: inline-subsection
**Arg**: {original arg string}
**Plan section file**: {path or "invalid"}
**Subsection ID**: {id or "invalid"}
**Reason**: {specific validation failure — one of:
  "invalid path shape — expected plans/.../section-*.md"
  "plan section file not found"
  "subsection id not in frontmatter sections list"
  "target subsection is not kind: plan-blocker-inline (got kind: {actual})"
  "parent plan section is complete — inline fix would reopen a closed section"
  "body heading ## {id} not found (frontmatter/body drift)"}
```
