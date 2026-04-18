# /continue-roadmap — Design Log

## Purpose + Context

`/continue-roadmap [section]` resumes roadmap work. It is a two-phase skill:

1. **Phase 1 (this skill, Sonnet sub-agent)** — run the scanner
   (`roadmap_scan.py --json`), apply mechanical auto-fixes the scanner flagged,
   and emit a structured handoff block (`### Focus context`, `### Gate
   results`, `### Next unblocked item`, optional `<escalate-to-parent>`).
   This is the **status reporter**.
2. **Phase 2 (parent Opus + `/roadmap-work`)** — act on the handoff: print
   Focus Context, fire any escalations via `AskUserQuestion`, then dispatch
   `/roadmap-work` for the selected subsection. This is the **investigator
   and code-writer**.

Canonical files:

- `.claude/skills/continue-roadmap/SKILL.md` — caller-facing dispatcher (the `Agent({})` call that launches the sub-agent).
- `.claude/skills/continue-roadmap/workflow.md` — the sub-agent's protocol (Steps 1–5, bans, tool-call budget).
- `.claude/skills/continue-roadmap/roadmap_scan.py` — the scanner. (Its own design log is `script-roadmap-scan-design.md`.)
- `.claude/skills/continue-roadmap/roadmap-scan.sh` — thin wrapper around the Python scanner.

## §1 Core Design Philosophy

1. **Speed-first status reporter, not an investigator.** A clean run must
   complete in 3–8 tool calls and well under a minute. The sub-agent exists
   to transcribe scanner JSON into a handoff block; all investigation
   (compiler runs, test analysis, code reads, git archaeology) is
   `/roadmap-work`'s job (Opus, full capability).
2. **Scanner JSON is the complete world-state.** The sub-agent may NOT
   observe anything the scanner did not pre-compute. If information is
   missing from the scanner's JSON, fix the scanner — do NOT let the
   sub-agent improvise by reading source code or running the compiler.
3. **Escalate on block, never peek.** When any `block`-severity gate fires,
   the sub-agent MUST escalate immediately with the payload's pre-built
   `AskUserQuestion` options. It MUST NOT inspect dirty-tree file contents,
   re-run test harnesses, or read sibling plans to "understand" why the
   gate fired.
4. **Sonnet for mechanical transcription, Opus for judgment.** Per the user's
   Skill Model Policy, Sonnet handles the JSON-to-handoff rewrite because it
   is pure mechanical work; Opus takes over for `/roadmap-work` because
   code execution requires judgment. Promoting the sub-agent's model is NOT
   a fix for bad prompts — the bans and budget below are the fix.
5. **Tool-call budget is load-bearing.** The budget (3–8 for clean; ~15 hard
   cap) is not a soft guideline — it is the primary speed invariant. Passing
   15 means the sub-agent has drifted off-contract and must return what it
   has rather than continuing.
6. **Parent owns the user interface.** The sub-agent NEVER calls
   `AskUserQuestion` directly; it pre-structures option arrays inside
   `<escalate-to-parent>` and the parent (main context, Opus) surfaces
   them. Sub-agent `AskUserQuestion` calls don't round-trip to the user
   through Agent({}) boundaries reliably.

## §2 Load-Bearing Invariants

| Invariant | Failure mode it prevents |
|---|---|
| Sub-agent NEVER runs `cargo`, `cargo check/run/test/clippy/b/t/st`, `cargo test --all`, `cargo clippy --all -- -D warnings`, `cargo build --all`, `cargo test --all`, `ori`, `oric`, `~/.local/bin/ori`, `./target/**`, or any binary in `diagnostics/`. | 2026-04-18 incident: sub-agent ran `cargo run -- check` on `tests/compiler/typeck/collections.ori`, then grepped the output to analyze a lambda type-inference error at line 154. Burned ~10 minutes of wall time and 67 tool calls before returning a handoff that was actually correct from the scanner alone. Every second of that investigation was duplicative — `/roadmap-work` would have re-discovered the same signal. |
| Sub-agent NEVER reads `.rs`, `.ori`, `.toml` files or anything under `compiler/`, `library/`, `tests/`, `scripts/`. | Investigation masquerades as "context gathering". A sub-agent that reads source files will build a mental model of the code, then use that model to second-guess the scanner's gate decisions — which is exactly the Opus-level judgment work that belongs to `/roadmap-work`. Plan-doc edits in `plans/` are fine (Step 2 auto-fix); source reads are not. |
| Sub-agent NEVER investigates test failures, typeck errors, dirty-tree contents, or bug repros. When a gate fires, it escalates immediately. | The scanner's classification is the contract. If `dirty_tree` fires, the sub-agent's job is to list the dirty files (already in the scanner's payload) and offer `/commit-push` as the recommended option. Peeking inside to "understand why the file is dirty" is both redundant (the user already knows — they're the one who left it dirty in a parallel session) and unsafe (the sub-agent might then argue against escalating). |
| Sub-agent NEVER runs `git log` / `git blame` / `git show` / `git diff` / `git bisect`. | The scanner captured the only git state that matters (`dirty_tree`). Git archaeology to "determine if the bug is pre-existing" is itself banned project-wide (CLAUDE.md §Never Investigate "Pre-Existing?"), and applies doubly here because `/continue-roadmap` has no legitimate need for commit history. |
| Sub-agent NEVER runs intelligence-graph queries (`scripts/intel-query.sh ...`). | `/review-plan` and `/roadmap-work` are the intelligence-graph consumers per `.claude/rules/intelligence.md` §When to Query. Scan+gates has no query use case — its inputs are plan frontmatter and file-system state, both already read by the scanner. |
| Sub-agent NEVER reads `CLAUDE.md` or `.claude/rules/*.md`. | Pre-computation guarantee: the scanner has already derived every field the handoff needs. Reading rules files re-introduces the 170k-token preamble waste that the plan-bug graph's `supersedes:` lifecycle marker was designed to avoid. |
| Tool-call budget: 3–8 for a clean run; ~15 hard cap. | Without a budget, a well-intentioned sub-agent can loop through auto-fix edits, re-verify scanner outputs, open adjacent section files "to check the subsection list", etc. The budget is the circuit breaker: if crossed, return what you have and escalate. |
| Parent surfaces Focus Context block verbatim BEFORE any `AskUserQuestion` prompt. | Without the Focus block, gate questions arrive with no context — the user sees "Working tree has 1 pending file(s), how do you want to proceed?" without knowing which plan or section the question applies to. The sub-agent builds the block; the parent's job is to print it first, unchanged. |
| Parent NEVER re-executes the sub-agent's steps inline. A re-invocation of `/continue-roadmap` dispatches a fresh sub-agent. | Scanner state, frontmatter, and bug-tracker state can change across an escalation boundary (e.g., the user ran `/commit-push` to resolve `dirty_tree`). Reusing prior scan output would act on stale state. |

## §3 File Inventory

| File | Lines | Role |
|---|---|---|
| `.claude/skills/continue-roadmap/SKILL.md` | ~90 | Parent-facing dispatcher. Contains the `Agent({})` call, the `Step A` Focus-Context surfacing rule, the `Step B` normal/escalation branch, the `AskUserQuestion` contract. Minimal — all protocol lives in workflow.md. |
| `.claude/skills/continue-roadmap/workflow.md` | ~260 | Sub-agent's protocol. Steps 1–5 (scanner → auto-fix → gate eval → pacing → handoff). Now includes §"Tool-call budget" and §"Hard bans" as load-bearing sections. |
| `.claude/skills/continue-roadmap/roadmap_scan.py` | see `script-roadmap-scan-design.md` | The scanner. Its own design history is logged separately. |
| `.claude/skills/continue-roadmap/roadmap-scan.sh` | tiny | Bash wrapper around the Python scanner, mostly for PATH normalization. |

## §4 Lessons from Dogfood / Production Runs

### 2026-04-18 — Sub-agent freelanced into compiler investigation for ~10 minutes

**Finding source:** User observed the sub-agent had been running for "nearly 10 minutes" on an empty `/continue-roadmap` (no args). Investigation of the agent transcript (`~/.claude/projects/.../subagents/agent-<id>.jsonl`) showed the sub-agent had:

1. Read `workflow.md` (expected, 1 call)
2. Ran `roadmap_scan.py --json` (expected, 1 call) — scanner reported `stale_plan_annotations` (auto-fix) + `dirty_tree` (block)
3. Ran `plan-annotations.sh --cleanup-only` and `--count` (expected, 2 calls) — scanner's flag was a false positive; nothing to clean
4. **Then drifted**: read `section-03-bodies-pass-integration.md` at two different offsets (banned — scanner already provides section fields)
5. **Then invoked the compiler**: ran `timeout 30 cargo run --quiet -- check tests/compiler/typeck/collections.ori 2>&1 | grep -A 6 "154:"` to investigate a lambda type-inference error unrelated to the gate
6. **Then looped**: ran `for f in tests/compiler/typeck/control_flow.ori tests/compiler/typeck/functions.ori ...` iterating across multiple test files with more `cargo check` invocations
7. Eventually returned a correct handoff (the scanner's `dirty_tree` escalation was intact) after 67 total tool calls

**Root cause:** The sub-agent's prompt + `workflow.md`'s "You do NOT" list only forbade (a) reading CLAUDE.md, (b) reading section files, (c) running intel-graph queries, (d) validating blockers by opening sibling plan files, (e) editing source, (f) direct git commit. It did NOT explicitly forbid running `cargo`, running test harnesses, reading `.rs`/`.ori` files, or investigating test failures. When the sub-agent saw that `dirty_tree` had a `.rs` file in it (`crates/types/src/check/bodies/tests.rs`), it apparently decided to "check related test files for typeck issues" as part of the gate analysis. None of that was necessary for the handoff.

The architectural mismatch: the skill was designed as a speed-first status reporter, but the rules document read as a list of narrow prohibitions rather than a positive charter. A Sonnet sub-agent with permissive tool access will fill latitude if given it — especially when the scanner surfaces a `block` gate and the sub-agent's instinct is to "understand the problem" before escalating.

**Fix (same session):**

1. Expanded `workflow.md` with three new top-level sections:
   - **§"The goal is SPEED"** — one-paragraph positive charter ("status reporter, not investigator").
   - **§"Tool-call budget"** — explicit table (3–8 clean, ~15 hard cap) so the sub-agent can self-monitor.
   - **§"Hard bans — what you MUST NOT do"** — explicit enumerated ban list covering `cargo` and all flavors, test harnesses, `ori`/`oric` binaries, `.rs`/`.ori` reads, compiler/library/tests source dirs, git archaeology, intelligence-graph queries, and investigation in general.
2. Tightened `SKILL.md`'s dispatcher prompt with a mirrored HARD BANS block so the sub-agent internalizes the rules from the dispatch prompt, not just from a file it reads later. The prompt now leads with "GOAL IS SPEED. A clean run completes in 3–8 tool calls."
3. Created this design log (`continue-roadmap-design.md`) — did not previously exist.

**Invariants added to §2:** the entire cargo/binary ban, the source-read ban, the investigation ban, the git-archaeology ban, the intel-graph-query ban, and the tool-call budget. All previously implicit or scattered; now load-bearing rows.

**What this fix does NOT do:** it does not add hook-level enforcement. A settings.json `PreToolUse` hook could block `cargo` calls from the `/continue-roadmap` sub-agent specifically, but implementing that cleanly requires the hook to know which agent is running (agent-ID plumbing) and adds false-positive risk to other skills that legitimately use `cargo`. The rules-level fix is the right level; if a regression recurs despite the explicit bans, revisit hook enforcement.

## §5 Regressions To Watch For

- [ ] `workflow.md`'s §"Hard bans" section is trimmed or softened ("NEVER run cargo" → "prefer not to run cargo", etc.). This re-opens the 2026-04-18 failure mode.
- [ ] The tool-call budget table is removed from `workflow.md` or raised without justification. 3–8 clean / ~15 hard cap was sized to the scanner's real workload — raising it without adding a new legitimate step means the sub-agent is being given latitude to investigate.
- [ ] `SKILL.md`'s dispatcher prompt drops its HARD BANS block (or moves the bans into a file-only reference). The sub-agent's initial prompt is where compliance is highest; banishing rules to a separate read weakens them.
- [ ] A new step is added to `workflow.md` that requires the sub-agent to "inspect the dirty files" or "run a quick sanity check" or "verify the scanner's gate classification against the actual code". All of these re-introduce investigation.
- [ ] The sub-agent is promoted from Sonnet to Opus under the theory that "more capability helps". Opus will freelance BETTER, not less — the fix is bans + budget, not model. The Skill Model Policy documents this: Sonnet is correct for mechanical transcription.
- [ ] `continue-roadmap` grows intelligence-graph queries or similar "reconnaissance" steps. Per `.claude/skills/query-intel/compose-intel-summary.md`, `/continue-roadmap` Step 2.1 is a PLANNED consumer — if it ever gets wired, the wiring must live in the scanner (Python, deterministic), not in the sub-agent's judgment.
- [ ] The parent starts caching scanner output across escalation boundaries ("we already scanned 3 minutes ago, reuse it") to save time. Scanner state changes across escalations; caching is banned per `SKILL.md` §"Never re-execute the sub-agent's steps inline."
- [ ] The sub-agent is allowed to call `AskUserQuestion` directly (skipping the `<escalate-to-parent>` round-trip). `Agent({})` boundaries don't reliably surface mid-dispatch `AskUserQuestion` prompts to the user — this would silently drop escalations.
- [ ] A "dry-run" or "preview" flag is added that makes the sub-agent read source code "just to see what's there". No such flag — the scanner is the only source of truth.

## §6 Improvement Log

### Open items

_None currently tracked._

### Recently closed

- [x] 2026-04-18 — Add `§The goal is SPEED`, `§Tool-call budget`, and `§Hard bans — what you MUST NOT do` sections to `workflow.md`; mirror the bans + budget in `SKILL.md`'s dispatcher prompt. Created this design log. Root cause: sub-agent ran `cargo check` on test files during a gate-check pass, burning ~10 min and 67 tool calls. See §4 for full transcript analysis. Commit: pending.

## §7 How To Use This File In Future Sessions

Open this file before editing anything in `.claude/skills/continue-roadmap/`. The load-bearing invariants are §2; the failure mode they prevent is §4. If your proposed change would relax any §2 row — the cargo ban, the source-read ban, the budget, etc. — STOP and re-examine. A "small" relaxation ("just let it read the section file, that can't hurt") is how the 2026-04-18 regression started.

If a user reports "the sub-agent is slow" or "the sub-agent is investigating", open the agent transcript (see §4 for the path pattern), count the tool calls, and check which ban was crossed. Add the crossing as a new §4 entry and the ban as a new §2 row if not already covered. Add an item to §5 so the pattern is grep-findable.

If the scanner (`roadmap_scan.py`) needs a new field to keep the sub-agent's rules tight (e.g., "we'd have to read the section file to get subsection count" → add `subsection_count` to the scanner's JSON envelope), that is always the right move. Scanner extensions are cheap and deterministic; sub-agent latitude is expensive and non-deterministic. See `script-roadmap-scan-design.md` for scanner-side evolution.
