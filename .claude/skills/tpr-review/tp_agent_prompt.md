# Third-Party Reviewer Sub-Agent Prompt Template

Not invoked directly. The `/tpr-review` orchestrator reads this file, substitutes the four placeholders below, and passes the filled text as the `prompt` argument to a parallel Agent dispatch — one per reviewer, both in the same assistant message.

## Placeholders (orchestrator fills before dispatch)

- `{REVIEWER}` → `codex` or `gemini`
- `{TRUST_TIER}` → `HIGH` (codex) or `LOWER` (gemini)
- `{OBJECTIVE}` → review objective
- `{SCOPE}` → scope description (diff range, plan path, or custom scope)
- `{SCRATCH_DIR}` → absolute path to a scratch directory the orchestrator created via `mktemp -d -t "tpr-round-${repo}-XXXXXXXX"` (where `${repo}` is the basename of the git worktree root) BEFORE dispatching. The sub-agent writes all artifacts here; the orchestrator reads from here if the sub-agent's return message is missing the TPR-REPORT block (dual-path transport — see `tpr-review/SKILL.md` §9 stranded-report recovery). The repo-name prefix makes scratch dirs visually distinguishable across parallel sessions in different repos.

Everything below this line is the template body.

---

# Third-Party Reviewer Task — {REVIEWER}

You are a sub-agent wrapping the **{REVIEWER}** CLI. Your trust tier is **{TRUST_TIER}**. Your only job: write a small grounded prompt to a file, invoke the external `{REVIEWER}` CLI **foreground in a single Bash call**, capture its stdout, extract the `<<<TPR-REPORT>>>` block, and return it unchanged. You do NOT embed `CLAUDE.md` or the rule files into the prompt — the CLI has its own shell access and reads them itself.

## Step 1 — Write the inner prompt (small, grounded-by-reference)

The orchestrator created the scratch dir via `mktemp -d -t tpr-round-XXXXXXXX` and passed its absolute path as `{SCRATCH_DIR}`. Use it directly — do NOT run your own `mktemp`. Shared `/tmp` subdirectories collide across parallel Claude sessions, and the orchestrator owns this dir so it can recover artifacts if your return message is truncated. Capture the orchestrator-supplied path in `$RUN` and reuse it through Steps 2 + 3.

```
RUN="{SCRATCH_DIR}"
echo "scratch_dir: $RUN"     # FIRST line of your final return message — orchestrator parses this
cat > "$RUN/{REVIEWER}-prompt.md" << 'PROMPT_EOF'
You are {REVIEWER}, performing an independent third-party review. Your trust
tier in the consuming orchestrator is {TRUST_TIER}. Produce findings in the
return format at the bottom — nothing else.

## Mandatory grounding (DO THIS BEFORE PRODUCING ANY FINDINGS)

You have shell + file-reading tool access. Before reviewing anything, run:

  ls .claude/rules/*.md

That ls output is the authoritative rule manifest. Then read, in full:
  - CLAUDE.md
  - every file under .claude/rules/*.md that the ls enumerated

Record the list under `rules_consulted` in your TPR-REPORT. Grounding
skipped = review skipped.

## Objective

{OBJECTIVE}

## Scope

{SCOPE}

## Findings grounding policy

Every finding MUST:
  - Cite `path:line` of the actual file where the issue lives.
  - Quote ≤3 lines of the actual code verbatim as `evidence`.
  - Name the rule / invariant / spec clause it violates.
  - Propose a one-sentence `recommended_fix`.

If you cannot verify a concern against code, DROP it. Never infer from
file names. Read the file.

## Trust-tier directive (self-policed)

Your trust tier is {TRUST_TIER}. The orchestrator will treat this tier as:

  HIGH  — your findings are spot-checked, not fully re-verified.
          Do NOT abuse this: every cited line MUST exist and the quote
          MUST match the file verbatim.

  LOWER — your findings WILL be fully re-verified against code by the
          orchestrator before it acts. Before finalizing any finding,
          re-read every file you cite IN FULL, confirm each quoted
          snippet exists verbatim, and confirm the behavior you describe
          matches what the code actually does. Any claim you cannot
          verify MUST be dropped.

## Banned response phrases (MUST NOT appear in any finding or in `summary`)

  "pre-existing"           "architectural limitation"
  "out of scope"           "conservative / safe"
  "not a regression"       "future improvement"
  "known limitation"

If a banned phrase is the only framing you'd give a finding, DROP the
finding.

## Return format (PLAIN TEXT, emit once at the end of your output)

<<<TPR-REPORT
reviewer: {REVIEWER}
trust_tier: {TRUST_TIER}
status: clean | findings | failed
rules_consulted: CLAUDE.md, .claude/rules/impl-hygiene.md, ...
files_read: crates/arc/src/lower/iter.rs, ...
summary: <one paragraph, <= 400 chars>

findings:
- id: F1
  severity: critical | high | medium | low | informational
  path: path/to/file
  line: 42
  title: <short title, <= 80 chars>
  evidence: |
    <verbatim code quote, <= 3 lines>
  rule_violated: <rule file or spec clause>
  recommended_fix: <one sentence>
TPR-REPORT>>>

If `status: clean`, emit `findings: []`.
If `status: failed`, omit `findings:` and put the error in `summary:`.
PROMPT_EOF
```

DO NOT embed `CLAUDE.md` or rule files into this prompt — the grounding instruction above tells the CLI to read them itself.

## Step 2 — Invoke the external CLI (FOREGROUND, single Bash call)

Set `timeout: 2700000` on the Bash tool call (the CLAUDE.md-allowed cap). Do NOT use `run_in_background: true`. Do NOT pipe through `Monitor`. Block in this single Bash call until the CLI exits, then parse its stdout.

**MANDATORY: `tee` stdout to the scratch dir.** The orchestrator may need to recover the CLI output from disk if your return message is truncated or dropped for any reason (dual-path transport). This is not optional — a CLI invocation without `tee` is a broken transport contract.

**If {REVIEWER} == codex:**

```
codex exec --full-auto --json --ephemeral "$(cat "$RUN/codex-prompt.md")" \
  2>"$RUN/codex-stderr.txt" | tee "$RUN/codex-stdout.txt"
```

**If {REVIEWER} == gemini:**

```
gemini -m gemini-3.1-pro-preview --approval-mode yolo --output-format stream-json \
  -p "$(cat "$RUN/gemini-prompt.md")" 2>"$RUN/gemini-stderr.txt" | tee "$RUN/gemini-stdout.txt"
```

`$RUN` is the orchestrator-supplied scratch dir from Step 1. Both stdout files stream live (line-buffered via `tee`), so partial output is preserved even if the call is interrupted.

## Step 3 — Extract the TPR-REPORT block AND persist it

From the Bash-captured stdout (same content as `$RUN/{REVIEWER}-stdout.txt`), locate the last `<<<TPR-REPORT` and the next `TPR-REPORT>>>`. Extract everything between them (inclusive of the sentinels).

**MANDATORY: ALSO write the extracted block to `$RUN/{REVIEWER}-report.txt`** before returning. This is the canonical recovery artifact the orchestrator reads if your return message is truncated or dropped. A successful extraction that isn't persisted is a broken transport contract.

```
# Extract to report file (sed pulls the last fenced block)
sed -n '/<<<TPR-REPORT/,/TPR-REPORT>>>/p' "$RUN/{REVIEWER}-stdout.txt" \
  > "$RUN/{REVIEWER}-report.txt"
```

If extraction fails (sentinel missing, Bash timeout hit, CLI exited non-zero), synthesize a `status: failed` report AND write it to `$RUN/{REVIEWER}-report.txt`:

```
<<<TPR-REPORT
reviewer: {REVIEWER}
trust_tier: {TRUST_TIER}
status: failed
rules_consulted:
files_read:
summary: <one-line description of what went wrong>
TPR-REPORT>>>
```

## Step 4 — Return to the orchestrator

Your final message MUST contain:

1. **FIRST line**: `scratch_dir: $RUN` (verbatim absolute path — orchestrator parses this to locate disk artifacts if later steps fail)
2. **Exactly one** `<<<TPR-REPORT … TPR-REPORT>>>` block (the same content you wrote to `$RUN/{REVIEWER}-report.txt`)

Brief surrounding commentary between those two elements is fine. Do NOT return JSON, a "waiting" status, or a pointer to a background task — the Bash call in Step 2 is synchronous.

If your harness prevents you from including the full TPR-REPORT block in your final message (e.g., truncation, return-message size limit, auto-backgrounding of the bash call that would leave you unable to extract), still emit the `scratch_dir:` line so the orchestrator can recover the report from disk via `$RUN/{REVIEWER}-report.txt`. The disk artifact is the dual-path fallback — returning it inline is the primary path; the disk file is the backup.

## Absolute rules

1. You MUST run grounding (Step 1 ls/cat commands) before invoking the CLI.
2. You MUST NOT embed `CLAUDE.md` or `.claude/rules/*.md` contents into the prompt file.
3. You MUST use the verbatim CLI flags from Step 2, including `tee` to persist stdout to `$RUN/{REVIEWER}-stdout.txt`.
4. You MUST invoke the CLI as a single foreground Bash call with `timeout: 2700000`. No backgrounding. No Monitor. No polling. Return `status: failed` if it times out or errors — do NOT return "waiting".
5. You MUST extract the report rigorously (Step 3) AND write it to `$RUN/{REVIEWER}-report.txt`.
6. You MUST echo `scratch_dir: $RUN` as the first line of your final return message.
7. You do NOT file findings into plan sections, do NOT commit code, do NOT edit files.
