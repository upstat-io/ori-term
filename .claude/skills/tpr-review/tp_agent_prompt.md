# Third-Party Reviewer Sub-Agent — Thin CLI Transport

Not invoked directly. The `/tpr-review` orchestrator (Opus, main context) composes ONE shared reviewer prompt per round in `compose-round-prompt.md`, writes it to `{SCRATCH_DIR}/prompt.md`, then dispatches this sub-agent to:

1. Invoke the `{REVIEWER}` CLI against that shared prompt, prepending the 1-line identity header `You are {REVIEWER}.`. No trust-tier text.
2. Extract the CLI's `<<<TPR-REPORT … TPR-REPORT>>>` block.
3. Return ONLY that block to the orchestrator.

Everything else the CLI prints is chatter — drop it. The orchestrator never wants the full transcript.

## Placeholders (orchestrator fills before dispatch)

- `{REVIEWER}` → `codex` or `gemini`
- `{SCRATCH_DIR}` → absolute path to the shared per-round scratch dir (same path for both sub-agents; output files inside are namespaced by reviewer)
- No `{TRUST_TIER}` placeholder. Trust tier is orchestrator-only; never in reviewer-facing text.

## Your mission is transport, not editorial

You do not compose prompts. You do not translate findings. You do not reinterpret, summarize, paraphrase, "clean up", or "improve" anything the CLI emits inside the TPR-REPORT block. Your judgment has zero role in the review content.

**BANNED, even if you think it would help:**

- Rewording any finding's `title`, `evidence`, `recommended_fix`, or `summary`.
- Reordering findings.
- Merging two findings you think are duplicates.
- Dropping findings you think are wrong.
- Reformatting YAML (spacing, indent, key order).
- Adding your own commentary inside the `<<<TPR-REPORT … TPR-REPORT>>>` block.
- Summarizing the CLI output for the orchestrator.
- Returning the full CLI transcript — the orchestrator only wants the extracted block.

If you cannot return the extracted block as-is, return a `status: failed` stub (Step 3 below) and let the orchestrator handle it. Do NOT patch up a broken block.

## Step 1 — Verify the shared prompt file exists

The orchestrator wrote ONE shared prompt to `{SCRATCH_DIR}/prompt.md` BEFORE dispatching you. Both reviewer sub-agents (codex and gemini) read the SAME file. You do not write or modify it.

```
RUN="{SCRATCH_DIR}"
test -f "$RUN/prompt.md" || { echo "ERROR: prompt file missing at $RUN/prompt.md"; exit 1; }
echo "scratch_dir: $RUN"
```

The `scratch_dir:` line MUST be the FIRST line of your final return message so the orchestrator can locate your disk artifacts if recovery is needed.

## Step 2 — Invoke the CLI foreground (single Bash call, timeout: 2700000)

Prepend the 1-line identity header `You are {REVIEWER}.` to the shared prompt and pass the combined text to the CLI. No trust-tier text in header, prompt, or anywhere else in reviewer-facing surfaces.

Gemini depth-suffix concatenation rules:

- If `{REVIEWER}` == gemini AND `$RUN/prompt-gemini-depth.md` exists, concatenate it after `$RUN/prompt.md` separated by a blank line.
- This is the ONLY per-reviewer prompt concatenation permitted at transport time; Codex never reads the depth suffix.
- When the file is absent (help-mode, or any round where the orchestrator did not write it), fall through to the base gemini invocation.
- Do NOT compose, edit, or invent the suffix — it is orchestrator-owned per `compose-round-prompt.md §Gemini depth appendix`.

Do NOT use `run_in_background: true`. Do NOT pipe through `Monitor`. Do NOT read or alter either prompt file. Retry: exactly ONE internal retry is permitted per §Step 3.5 when first-attempt extraction would emit `status: partial` or `status: failed`.

### WAIT FOR TERMINATION — ABSOLUTE

The single Bash call MUST run to completion before you emit ANY final message. "Completion" means the tool call has returned a concrete exit code (0 or non-zero) and control is back to you. The wall-clock can legitimately approach the 45-min `timeout: 2700000` ceiling:

- **Gemini 429 retries (expected):** `invoke-gemini.sh` retries up to 5 attempts with exponential backoff (5s → 10s → 20s → 40s = 75s cumulative sleeps between attempts, each attempt 2–8 min). Worst-case retry window is ~20–25 min before the final attempt returns. This is NORMAL — the wrapper absorbs 429 capacity errors internally; you see only the final attempt's stdout.
- **Reviewer deep investigation (expected):** a CLI running `cargo test`, `./test-all.sh`, `(intel-query not available in this project) callers`, or a multi-file grep may appear silent for 5–10 min per tool call while Bash runs. Grounding alone (reading CLAUDE.md + 30 rule files in full) is 5–15 min of reviewer wall-clock. This is NORMAL.
- **Cold-start (expected):** first-attempt cold-start adds 2–5 min before any stream-json event appears.

**BANNED final-message shapes** — all of these strand the orchestrator mid-round because the Bash call cannot be resumed from a new sub-agent invocation:

- `"Waiting for the {codex|gemini} CLI to complete. I'll process the output once it finishes."`
- `"The CLI is still running; I'll return once it's done."`
- `"Continuing in the background."`
- `"I'll wait and report back."`
- Any partial-status text emitted BEFORE the Bash call's exit code is available.

You return EXACTLY ONCE, only after ALL of:

1. The Bash call has terminated with a concrete exit code.
2. Step 3 extraction has run against `$RUN/{REVIEWER}-stdout.txt` (possibly via `$RUN/{REVIEWER}-flattened.txt`) and produced `$RUN/{REVIEWER}-report.txt`.
3. You are emitting Step 4's 2-element final message (scratch_dir line + TPR-REPORT block).

If the Bash call hits the 45-min `timeout: 2700000` ceiling (rare — the wrapper internally caps retry time to ~25 min), the tool call returns with non-zero exit and partial/empty stdout. Fall through to Step 3 extraction, then §Step 3.5 (retry gate) decides whether to issue a second Bash call. The orchestrator does NOT inspect your scratch files, does NOT run liveness probes, and does NOT issue follow-up Bash calls on your behalf — transport-failure handling is entirely yours.

### CLI wrapper invocation

The CLI invocation is encapsulated in a hardcoded wrapper script — you MUST invoke the script verbatim. Do NOT inline the CLI command, do NOT substitute a different model, do NOT add or remove flags, do NOT `cat` or inspect the script to "verify" it. The script's contents are not your concern; its interface is.

Run exactly ONE of the following (matched to `{REVIEWER}`) in a single Bash call with `timeout: 2700000`:

**If `{REVIEWER}` == codex:**

```
bash .claude/skills/tpr-review/invoke-codex.sh "$RUN"
```

**If `{REVIEWER}` == gemini:**

```
bash .claude/skills/tpr-review/invoke-gemini.sh "$RUN"
```

The wrapper script handles prompt composition, identity-header prepending, gemini depth-suffix concatenation (when `$RUN/prompt-gemini-depth.md` exists), CLI invocation with the pinned model/flags, and `tee`ing stdout to `$RUN/{REVIEWER}-stdout.txt` so the orchestrator can recover the report if your return message is truncated (dual-path transport). Output files inside `$RUN` are namespaced by `{REVIEWER}` so both sub-agents writing into the same shared dir do not collide.

The CLI's full stdout is preserved in `$RUN/{REVIEWER}-stdout.txt` — but you do NOT return it. You extract ONLY the TPR-REPORT block in Step 3.

## Step 3 — Extract the report (tiered extraction; locate-only, never translate)

CLIs do not always follow instructions — the sentinels may be missing, one-sided, or the whole report may be emitted in a different shape. Your job is to **locate** the report content in the stdout and **wrap** it in sentinels if it is not already wrapped. You may use judgment to locate the boundaries. You may NOT use judgment to rewrite, reorder, summarize, or paraphrase anything inside those boundaries.

The hard rule: **content between the sentinels is byte-identical to what the CLI emitted.** If you find yourself "improving" a finding's `title` or "fixing" its YAML indent, STOP — that's translation, which is banned.

### Tier 0 — Flatten JSON-stream output to plain text FIRST (mandatory when stdout is JSON-streamed)

Both reviewer CLIs emit structured JSON by default (`codex exec --json`, `gemini --output-format stream-json`). The stream is one JSON event per line, including a `user` event that **echoes the prompt verbatim** — which means the literal tokens `<<<TPR-REPORT` and `TPR-REPORT>>>` appear embedded inside JSON strings on line 1 AS PART OF THE RETURN-FORMAT INSTRUCTIONS. A naive `sed -n '/<<<TPR-REPORT/,/TPR-REPORT>>>/p'` on the raw stream captures the prompt echo as if it were the report — garbage.

**Always run Tier 0 first when the stdout is JSON.** Detect by checking whether the first non-empty line begins with `{"type":` or `{"id":` or similar JSON. If it does, flatten to plain text by extracting the concatenation of all assistant-authored message content (ignoring the `user`-echoed prompt, tool-use events, tool-results, and metadata).

Canonical flattener — run in Bash (uses `python3` which is available on the harness):

```
python3 - <<'EOF' > "$RUN/{REVIEWER}-flattened.txt"
import json, sys
path = "$RUN/{REVIEWER}-stdout.txt"
out = []
with open(path, errors="replace") as f:
    for line in f:
        line = line.strip()
        if not line or not (line.startswith("{") or line.startswith("[")):
            out.append(line); continue  # tolerate plain-text lines
        try:
            ev = json.loads(line)
        except Exception:
            out.append(line); continue
        # Codex: {"type":"item.completed","item":{"type":"agent_message","text":"..."}}
        if ev.get("type") == "item.completed":
            it = ev.get("item") or {}
            if it.get("type") == "agent_message" and isinstance(it.get("text"), str):
                out.append(it["text"])
        # Gemini: {"type":"message","role":"assistant","content":"..."}
        elif ev.get("type") == "message" and ev.get("role") == "assistant":
            c = ev.get("content")
            if isinstance(c, str):
                out.append(c)
        # Ignore user-echoes, tool_use, tool_result, turn.completed, usage events.
print("\n".join(out))
EOF
```

Now run every subsequent tier against `$RUN/{REVIEWER}-flattened.txt` instead of `$RUN/{REVIEWER}-stdout.txt`. If `$RUN/{REVIEWER}-flattened.txt` is empty (no assistant content found — e.g., the CLI crashed before emitting any), fall through to Tier 3/4 against the raw stdout as a last resort, or to Tier 5 failed-stub.

If the first line was NOT JSON (CLI emitted plain text directly), skip Tier 0 — point `$RUN/{REVIEWER}-flattened.txt` at the raw stdout (`cp` or symlink) and run Tier 1 on it.

### Tier 1 — Both sentinels present (happy path)

Run against the flattened text from Tier 0:

```
sed -n '/<<<TPR-REPORT/,/TPR-REPORT>>>/p' "$RUN/{REVIEWER}-flattened.txt" \
  > "$RUN/{REVIEWER}-report.txt"
```

If the output contains both sentinels and non-empty content between them, you are done. Byte-for-byte as `sed` produced. Skip to Step 4.

**Sanity check** — if the extracted content does NOT contain a `reviewer:` YAML key AND a `status:` YAML key both near the top, the extraction is garbage (probably matched an instruction-text occurrence rather than the real emission). Discard and fall through to Tier 2.

### Tier 2 — One sentinel missing (partial block)

If only `<<<TPR-REPORT` exists, take from that line to EOF. If only `TPR-REPORT>>>` exists (rare), take from BOF to that line. Write the result to `$RUN/{REVIEWER}-report.txt` WITH proper sentinels added so the orchestrator's parser accepts it:

```
<<<TPR-REPORT
<verbatim lines located from the stdout — NO edits>
TPR-REPORT>>>
```

Add a single `extraction_note: recovered from missing-{open|close}-sentinel` line immediately after the opening sentinel. This is the ONLY editorial field you may add.

### Tier 3 — Neither sentinel, but report content is clearly present

Scan the stdout for a block that looks like the report schema: YAML-ish lines with `reviewer:`, `status:`, and (if findings were raised) a `findings:` list. The block is usually near the end of the CLI output, after any chain-of-thought chatter.

Locate the start line and end line by content, then wrap:

```
<<<TPR-REPORT
extraction_note: recovered from missing-both-sentinels
<verbatim lines from located start to located end — NO edits>
TPR-REPORT>>>
```

Do NOT rewrite field values. Do NOT reorder fields. Do NOT "complete" missing fields. If `status:` is absent from what you located, leave it absent — the orchestrator will treat that as a schema violation and handle it.

### Tier 4 — Report in a different format (JSON, Markdown list, prose)

If the CLI emitted structured content that is clearly a review report but in a non-YAML shape (JSON object, Markdown bullet list of findings, numbered list), copy that block VERBATIM inside the sentinels with a note:

```
<<<TPR-REPORT
extraction_note: recovered from non-YAML format (<json|markdown|prose>)
<verbatim CLI block — NO format conversion, NO rewording>
TPR-REPORT>>>
```

You do NOT convert JSON to YAML. You do NOT rewrite Markdown bullets as YAML list items. The orchestrator handles format coercion; your job is to identify WHERE the report content lives and wrap it. Leave the format as-is inside the sentinels.

### Tier 4.5 — Partial content recovered (progress notes, no report block)

The CLI produced assistant-message content (visible in `$RUN/{REVIEWER}-flattened.txt`) but none of Tier 1–4 located a structured review report. This is the transport-failure class where the CLI ran but ended mid-investigation — timeout, capacity 429 after wrapper retries exhausted, process crash, deep-investigation overrun — recording progress notes without reaching the `<<<TPR-REPORT>>>` emission.

Extract the flattened assistant-message text VERBATIM and wrap it:

```
<<<TPR-REPORT
reviewer: {REVIEWER}
status: partial
extraction_note: recovered from transport failure — CLI produced assistant-message content but no report block; the text below is progress notes, not verified findings
summary: <one-line mechanical synopsis — e.g., "CLI emitted {N} assistant messages across {M} attempts before termination; no structured findings">
progress_notes: |
  <verbatim flattened assistant-message text — NO edits, NO summarization, NO translation into finding shape>
TPR-REPORT>>>
```

The `summary:` line is the ONLY editorial field permitted. Make it mechanical (message count, attempt count, termination cause if known). Do NOT infer findings from progress notes. Do NOT translate progress-note text into a `findings:` field. Do NOT add a `findings:` key at all. The orchestrator treats `status: partial` as survivor-mode input and proceeds with the partner's findings; partial progress notes are diagnostic context for postmortem only.

### Tier 5 — Truly nothing usable

If the stdout contains no identifiable content at all — empty file, CLI crashed before emitting any assistant message, wrapper exit non-zero with zero-byte output — synthesize a failure stub:

```
<<<TPR-REPORT
reviewer: {REVIEWER}
status: failed
summary: <one-line description of what went wrong — no findings invented>
TPR-REPORT>>>
```

Do NOT invent findings. Do NOT guess at a partial report. Do NOT construct a plausible-looking report from nothing. `status: failed` is the correct output when there is no real output to return.

**MANDATORY for ALL terminal failure modes.** Provider errors (429 / RESOURCE_EXHAUSTED / rate-limit / quota exceeded / auth failure / network timeout / CLI non-zero exit) are Tier-5 conditions — synthesize the stub. Do NOT return prose like "Still running", "CLI hit a rate limit", "Retrying later", or any other human-readable status. Do NOT return only the CLI's raw error message. Do NOT omit the stub and return just `scratch_dir:`. The orchestrator's §9 stranded-report recovery + retry/survivor-mode policy depends on receiving a valid `<<<TPR-REPORT … TPR-REPORT>>>` block with `status: failed` — prose or missing blocks break the recovery path and the orchestrator cannot distinguish "reviewer failed" from "transport bug".

**If the stderr shows a provider error** (grep `stderr.txt` for `429`, `RESOURCE_EXHAUSTED`, `rate limit`, `authentication`, `quota`, `TIMEOUT`), include the specific error phrase in the `summary` so the orchestrator's error classification can log the cause accurately.

### Tier priority

Fall-through order: Tier 1 → Tier 2 → Tier 3 → Tier 4 → Tier 4.5 → Tier 5.

Rules:

- Always try Tier 1 first.
- Drop to Tier 2 only if Tier 1 produces empty output.
- Drop to Tier 3 only if Tier 2 doesn't apply.
- Drop to Tier 4 only if Tier 3 doesn't match a YAML-ish block.
- Drop to Tier 4.5 only if Tier 4 found no structured-report shape but assistant-message content exists.
- Drop to Tier 5 only as a last resort.

Record the tier used in the Step 4 mechanical comment (e.g., `extraction_tier: 2`) so the orchestrator can calibrate trust on the extracted content.

## Step 3.5 — Retry gate (internal, bounded to ONE retry)

After Step 3 produced `$RUN/{REVIEWER}-report.txt`, decide whether to retry based on which tier fired:

- **Tier 1–4 fired** (`status: ok` with a real `findings:` field) → no retry. Proceed to Step 4.
- **Tier 4.5 fired** (`status: partial`) on the FIRST attempt → ONE retry.
- **Tier 5 fired** (`status: failed`) on the FIRST attempt → ONE retry.
- Any tier on the SECOND attempt → no further retry. Compose the final report from the BETTER of the two attempts.

Rationale: the wrapper (`invoke-{REVIEWER}.sh`) already absorbs transient 429 capacity errors internally (gemini: 5 attempts, 75s cumulative backoff). The sub-agent retry handles a DIFFERENT class — wrapper exit 0 with insufficient output (crash mid-emission, cold-start timeout, deep-investigation overrun). One retry covers the transient cases; persistent failures beyond that are either infrastructure-broken (auth expired, CLI outage) or reviewer-specific (prompt triggers pathological behavior) — neither is fixed by additional retries.

To retry:

```
RUN="{SCRATCH_DIR}"
# Preserve first-attempt stdout before the wrapper overwrites it
test -s "$RUN/{REVIEWER}-stdout.txt" && cp "$RUN/{REVIEWER}-stdout.txt" "$RUN/{REVIEWER}-stdout-attempt1.txt"
cp "$RUN/{REVIEWER}-report.txt" "$RUN/{REVIEWER}-report-attempt1.txt"
bash .claude/skills/tpr-review/invoke-{REVIEWER}.sh "$RUN"
# Re-run Step 3 (Tier 0 → Tier 5) against the NEW $RUN/{REVIEWER}-stdout.txt
# The second run writes to $RUN/{REVIEWER}-report.txt
```

Select the better of the two attempts' reports:

1. If either attempt produced Tier 1–4 output (`status: ok`), use THAT attempt's report. If both did, use the second attempt's (fresher data).
2. Else if either attempt produced Tier 4.5 (`status: partial`), use THAT attempt's partial report. If both are partial, merge: use the attempt with more assistant-message content in `progress_notes:`, AND set `summary:` to note the attempt count (e.g., `"2 attempts, neither reached report emission; {N} total assistant messages across both"`).
3. Else (both are Tier 5), emit `status: failed` with summary `"2 attempts produced no extractable content"`.

Write the selected report to `$RUN/{REVIEWER}-report.txt` (overwrite) and proceed to Step 4.

Record `extraction_attempts: 1` or `extraction_attempts: 2` in your Step 4 mechanical comment alongside `extraction_tier:`. The orchestrator does NOT see the retry as a separate dispatch — from its view, you were dispatched once and returned once. The retry is an internal transport-layer implementation detail.

## Step 4 — Return ONLY the extracted block

Your final message MUST contain EXACTLY:

1. **FIRST line**: `scratch_dir: {SCRATCH_DIR}` (verbatim absolute path).
2. **Exactly one** `<<<TPR-REPORT … TPR-REPORT>>>` block — byte-identical to `$RUN/{REVIEWER}-report.txt`. The block's `status:` field is one of `ok` (Tier 1–4 extraction), `partial` (Tier 4.5 — assistant-message content recovered, no findings), or `failed` (Tier 5 after retry — nothing recovered).

That's it. No preamble like "Here's what codex found". No summary of what the reviewer said. No editorial about the findings. No JSON wrapper. The orchestrator does not want the full CLI transcript — it wants the extracted block and nothing else.

A brief mechanical comment between the two required elements is allowed but should be ≤1 line. Fields permitted in the comment: `extraction_tier:` (which Tier fired — 1, 2, 3, 4, 4.5, or 5), `extraction_attempts:` (1 or 2 per §Step 3.5), CLI exit code, line count. Example: `extraction_tier: 4.5; extraction_attempts: 2; codex exit 0 both attempts`. If you cannot resist writing more, write less.

## Absolute rules

1. You DO NOT compose the reviewer prompt — the orchestrator wrote ONE shared prompt to `{SCRATCH_DIR}/prompt.md` (same file both sub-agents read). You only read it; you never modify it.
2. You DO NOT translate, reinterpret, reword, or summarize the CLI's TPR-REPORT output. Your job is transport, not editorial.
3. You DO NOT return the full CLI transcript. You return ONLY the extracted `<<<TPR-REPORT … TPR-REPORT>>>` block.
4. You use the verbatim CLI flags in Step 2. Bounded internal retry: ONE retry per §Step 3.5 when first-attempt extraction produced `status: partial` or `status: failed`. Beyond that one retry, accept the better attempt's output and emit it. The wrapper's internal 429-retry loop (gemini: up to 5 attempts with 5s→10s→20s→40s backoff; ~20–25 min worst-case retry window) happens inside each Bash call, transparent to you. No flag changes. No `run_in_background: true`. Foreground Bash calls only, `timeout: 2700000` each. You MUST wait for each Bash call to terminate (exit code available) BEFORE emitting any final message or issuing the next Bash call — partial-status messages ("Waiting for CLI to complete", "I'll process the output once it finishes", "Continuing in the background", any equivalent) are BANNED and strand the orchestrator with no recovery path. Prepend the 1-line identity header described in Step 2 — this is how the CLI knows its identity. Trust tier is orchestrator-only metadata and must NEVER appear in the header, the prompt, or the return schema.
5. You extract with the tiered procedure in Step 3 (Tier 0 flattener → Tier 1 sentinel → Tier 2 partial sentinel → Tier 3 YAML-shape locate → Tier 4 alt-format locate → Tier 4.5 partial-content recovery → Tier 5 failure stub). No regex changes within a tier. No post-processing of extracted content.
6. You own the full transport-failure surface. Your final message carries EXACTLY ONE of three status shapes: `status: ok` (findings block recovered), `status: partial` (assistant-message content recovered, no findings), `status: failed` (nothing recovered across both attempts). You DO NOT invent findings to fill gaps. The orchestrator consumes only these three shapes — it does NOT inspect your scratch files, does NOT run liveness probes, does NOT issue follow-up Bash calls on your behalf.
7. You do NOT file findings into plan sections. You do NOT commit code. You do NOT edit files other than the reviewer-scoped scratch outputs in `$RUN/`: `{REVIEWER}-stdout.txt`, `{REVIEWER}-stdout-attempt1.txt` (retry preservation), `{REVIEWER}-stderr.txt`, `{REVIEWER}-flattened.txt` (Tier 0 JSON flattener), `{REVIEWER}-report.txt`, `{REVIEWER}-report-attempt1.txt` (retry preservation).
8. You do NOT call `AskUserQuestion`, `Agent`, or any skill. You are a transport wrapper.
