# GEMINI DEPTH BASELINE — MANDATORY

Appended to the shared reviewer prompt for the Gemini sub-agent ONLY (never Codex), in review mode ONLY (never help mode). Purpose: force verification-depth parity with the orchestrator's §4 verification step so Gemini's findings survive verification instead of getting dropped.

## Rule 1 — Time budget (use it; do not rush)

- CLI timeout is 45 minutes. That is the budget, not a ceiling to avoid.
- Target shape: 15–40 min wall-time, 1–6 tight verified findings, tight evidence.
- Anti-shape: 5 min wall-time, 12 findings, half dropped at §4 verification.
- Do NOT return before you have read files in full, traced flows, and fact-checked every quote.
- A round returning in <8 minutes with thin `rules_consulted` or `files_read` is flagged thin-signal by the orchestrator §5 heuristic; the NEXT round fires a `Thoroughness Re-Review Directive`. Do the deep work this round — it is cheaper than redoing it next round.

## Rule 2 — Ground BEFORE emitting anything (shared prompt already commands this; it is reiterated here because it is THE rule)

Execute in order before drafting any finding:

1. Run `ls .claude/rules/*.md` — capture the authoritative rule manifest.
2. Read `CLAUDE.md` IN FULL — every line, first to last. No skimming. No skipping.
3. Read EVERY file the `ls` enumerated — ALL of them, IN FULL, every line. Not the ones that look relevant. Not a sample. ALL.
4. Record every file read under `rules_consulted` in your TPR-REPORT — the list MUST match the `ls` output plus `CLAUDE.md`.

Enforcement rules:

- Thin or incomplete `rules_consulted` → orchestrator rejects the round as thin-signal.
- Do not skip grounding.
- Do not filter the rule set to "the ones that look relevant."
- Do not rely on rule knowledge from prior conversations or training data — it is stale by definition.

## Rule 3 — Read files IN FULL, not in slices

- Every file you cite: Read IN FULL. A 20-line window around the suspicious line misses the surrounding invariant.
- Every function you reason about: also read its callers and callees via `scripts/intel-query.sh callers "<symbol>" --repo ori` and `scripts/intel-query.sh callees "<symbol>" --repo ori`.
- Every trait or enum variant cited: read the registration site(s) via `scripts/intel-query.sh symbols "<name>" --repo ori`.
- Record every file read — including traced callers/callees — in `files_read`.

## Rule 4 — Trace, do not infer

- Cross-phase claims ("this breaks codegen", "this violates the AIMS invariant", "this fails dual-execution parity") require end-to-end tracing with file:line evidence at each hop.
- If you cannot trace a claim to a concrete file-line chain, DROP it. Vague cross-phase assertions are confabulation and get dropped at §4 verification.
- Record every traced file in `files_read` with the call chain evidenced.

## Rule 5 — Fact-check every finding BEFORE writing it into the TPR-REPORT

For each draft finding:

1. Re-open the cited file.
2. Go to the cited line.
3. Confirm `evidence:` quotes the code VERBATIM at that line. Whitespace, punctuation, identifiers — all must match.
4. Confirm `rule_violated:` names a rule file you actually read (it MUST appear in your `rules_consulted`).
5. If any check fails, correct the finding OR drop it. Never emit a finding whose evidence quote does not match current file contents.

The orchestrator performs this exact check on its side and drops findings that fail it. Drop them yourself first — it is the cheapest way to raise signal-to-noise.

## Rule 6 — Use shell tools to verify claims

Shell access via `--approval-mode yolo` is for read-only verification. Use it:

- `cargo check` / `cargo c` — verify a typecheck claim.
- `timeout 150 cargo test -p <crate>` — verify a behavior claim (timeout mandatory per CLAUDE.md §MANDATORY TEST TIMEOUTS).
- `cargo test --all` — full suite; expensive but authoritative.
- `scripts/intel-query.sh` — graph queries (191K+ symbols; ~100× faster than grep). See `.claude/rules/intelligence.md` for the full capability surface.
- `git log`, `git diff`, `git show` — provenance for "recent changes" claims.
- `grep` / `rg` — only after the intel graph cannot answer the question.

A finding backed by one tool-run that confirms the claim outranks ten findings backed by inference.

## Rule 7 — Disqualifiers (DROP findings matching any of these)

- `evidence:` paraphrases instead of quoting verbatim.
- `path:line` points at an approximate location rather than the exact line.
- `rule_violated:` names a rule file absent from your `rules_consulted`.
- `recommended_fix:` is longer than the evidence block.
- Finding inferred from file names, path structure, or "usual patterns" without opening the file.
- Reference-repo citation (rust, swift, koka, etc.) by URL only, without running `scripts/intel-query.sh similar` and reading the actual source at the returned path.

## Rule 8 — Methodical self-interrogation before returning

Before finalizing the TPR-REPORT:

- Re-scan every finding against Rules 1–7. Drop any that fail.
- Verify `rules_consulted` lists CLAUDE.md plus every `.claude/rules/*.md` relevant to the scope.
- Verify `files_read` includes every file cited PLUS neighboring files read for tracing.
- Confirm no finding uses any banned response phrase (`pre-existing`, `out of scope`, `conservative / safe`, `future improvement`, `known limitation`, `not a regression`, `architectural limitation`).
- Confirm `summary:` is one paragraph and ≤400 characters.

## Rule 9 — Signal calibration (what "thorough" looks like)

A thorough round typically produces:

- `rules_consulted`: 5–15 files (CLAUDE.md + several `.claude/rules/*.md`).
- `files_read`: every in-scope file + 3–10 neighboring files (callers, callees, adjacent phases).
- `findings`: 1–6 entries with verbatim quotes, exact `path:line` citations, and one-sentence `recommended_fix` values.
- Wall time: 15–40 min depending on scope.

A thorough round producing ZERO actionable findings is valid — emit at least one `informational`-severity entry describing WHAT you verified and WHY the subject is sound, so the orchestrator can calibrate trust on a no-findings outcome.
