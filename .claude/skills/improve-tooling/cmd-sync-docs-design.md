# `/sync-docs` — Design Notes and Improvement Log

## §Purpose + Context

`/sync-docs` is the project's comprehensive nightly documentation sync command at `.claude/commands/sync-docs.md`. It runs unattended overnight, iterates through 13 batches of related doc files, and for each batch launches `/tpr-review --autonomous` with a custom objective — Codex and Gemini independently cross-check every claim against code/spec/missions, then Claude fixes findings and loops until both reviewers report clean. The command implements a two-tier Fact-Pairing contract (Tier A intent + Tier B corroboration) with enumerated exception categories (`TRIVIAL`, `INTENT-ORPHAN/operational`, `INTENT-ORPHAN/deletion`).

**Last significant change (2026-04-18):** `/improve-tooling` invocation that (a) wired `/sync-docs` as the 21st consumer of the intelligence-graph SSOT (`compose-intel-summary.md`), (b) added Phase 0.6 "Intelligence Pre-Sweep" materializing `file-symbols`/preset/`plan-status`/`similar` output to `/tmp/sync-docs-intel/` before batch 1, (c) added a shared "Graph-First Verification" instruction block prepended to every batch's `/tpr-review --autonomous` ARGS so reviewer sub-agents use the graph too, and (d) fixed a critical autonomous-flag bug where `/sync-docs` was invoking `/tpr-review` in interactive mode — any cap-exit / ambiguous-input / transport-failure branch would hit `AskUserQuestion` and hang the nightly run waiting for a user who is asleep.

**Approved plan:** none — `/improve-tooling` changes go through direct commit per CLAUDE.md §Scope (skills/tooling/infra exempt from `/tpr-review` gating because `/fix-bug` + `/tpr-review` would be circular).

## §1 Core Design Philosophy (KEEP THIS)

1. **Nightly-unattended contract.** The command runs for HOURS with ZERO human interaction. Every subsystem it invokes (currently only `/tpr-review`, but future integrations too) MUST have an autonomous/non-interactive mode, and `/sync-docs` MUST invoke them with that mode engaged. An interactive sub-command is a contract violation — the entire nightly sync hangs the first time a branch falls back to `AskUserQuestion`. Enforce via explicit flag-presence in §Batch Execution Protocol step 2 + §Automation Protocol step 3.

2. **Worktree isolation (Phase 0).** All work happens on a branch inside `.claude/worktrees/sync-docs-YYYY-MM-DD`. Never modify the `dev` branch in place. User reviews and merges when ready.

3. **Discovery is rule-based, not enumerated.** Phase 0.5 uses `git ls-files '*.md'` filtered by the Banned Paths list. New `.md` files land in Batch 13 (the catch-all) automatically — there is no maintenance obligation when a new doc surface appears.

4. **13 sequential batches, never parallel.** Earlier batches establish Tier-A intent surfaces (canon, missions, rules) that later batches verify against. Running them in parallel would break the intent-propagation ordering.

5. **Per-batch sequential launch of `/tpr-review --autonomous`.** The TPR skill handles reviewer launch, fix-and-rerun loops, autonomous exit semantics. `/sync-docs` is a dispatcher — it does NOT re-implement TPR transport or reviewer orchestration.

6. **Two-tier Fact-Pairing contract.** Every Edit carries a `Fact-Pairing:` block with ONE Tier A (intent) citation + ONE Tier B (corroboration) citation. Tier A sources are ranked: spec clause → approved proposal → mission → design-philosophy header. Tier B is implementation fact, cross-doc coherence, or graph-verified fact with source spot-check. `TRIVIAL`, `INTENT-ORPHAN/operational` (enumerated surfaces only), and `INTENT-ORPHAN/deletion` are the exceptions, explicitly tagged.

7. **Intent NEVER edited to match code.** When intent and code disagree, the bug is filed against the CODE side — never against the intent surface (missions, spec, proposals). This is the load-bearing drift-surfacing invariant.

8. **Graph-first discipline (2026-04-18).** Every reviewer sub-agent uses the intelligence graph BEFORE manual grep. Phase 0.6 materializes the baseline inventory once; per-batch objectives cite the shared Graph-First Protocol block (§Batch Definitions) so reviewers extend with on-demand queries. Graph results are DISCOVERY; source verification is still load-bearing.

9. **Graceful degradation when graph unavailable.** `scripts/intel-query.sh status` probe; if down, Phase 0.6 is skipped and every batch's findings are prefixed `graph-unavailable:` in the Phase 3 report. Never fail the sync over an unavailable graph.

10. **Commit per batch, no push from worktree.** Each batch's changes are committed via `/commit-push` to the worktree branch. User merges at their discretion. Never push from the worktree.

## §2 Load-Bearing Invariants

Changing any of these without a concrete plan risks re-introducing the failure modes listed:

| # | Invariant | Why (which failure mode it prevents) |
|---|-----------|--------------------------------------|
| I1 | Every `/tpr-review` launch from `/sync-docs` MUST include `--autonomous` as the first flag | Without it, any `/tpr-review §5 §9 §1` terminal branch (cap-exit / transport-failure / ambiguous-input) falls back to `AskUserQuestion` and hangs the nightly sync indefinitely. The user launches this command and sleeps; a hung prompt breaks the entire contract. |
| I2 | 13 batches run SEQUENTIALLY, not in parallel | Earlier batches (canon.md, missions.md) establish Tier-A intent that later batches verify against. Parallel execution would inject cycles into the intent-propagation DAG. |
| I3 | Phase 0.6 pre-sweep runs ONCE per nightly run, materialized to `/tmp/sync-docs-intel/` | Running per-batch would re-pay the graph-query cost 13× and produce inconsistent snapshots mid-sync. Single materialization + batch reuse is the intent. |
| I4 | Phase 0.5 Batch 13 catch-all captures anything not matched by Batches 1-12 | New `.md` files land in-scope automatically. Removing the catch-all would silently skip new surfaces — the exact failure mode this sync exists to prevent. |
| I5 | Banned-paths list is the ONLY filter on `git ls-files '*.md'` | Anything else creates implicit coverage holes. A new rules file outside the enumeration would be silently excluded. |
| I6 | `Fact-Pairing:` blocks are MANDATORY; missing pairings fail the per-batch verification grep | The skill's core contract. Without the grep gate, edits can land without citations and the drift-surfacing property degrades silently. |
| I7 | Tier A NEVER edited to match code — the bug is filed on the code | Directional intent (`missions.md §How to use this file`). Weakening Tier A to match drifted code converts the sync from a drift-surfacing tool into a drift-hiding tool. |
| I8 | Shared "Graph-First Verification" block is OUT-OF-LINE, prepended to every batch's ARGS at launch | Embedding it in each batch's objective body risks 13-way drift. Out-of-line composition at launch time means one change propagates to all batches atomically. |
| I9 | Graceful degradation on graph unavailability — annotate, never fail | Graph is additive per `.claude/rules/intelligence.md §Availability`. A graph outage must not block the nightly sync; it must degrade to manual verification with clearly-marked reduced-confidence findings. |
| I10 | `/sync-docs` commits stay on the worktree branch; no push from inside the worktree | User controls when to merge. Pushing from inside the worktree would publish unreviewed changes and bypass the user's merge-time review. |
| I11 | Phase 3 final report is the ONLY user-visible output; batches 1-12 produce NO interim reports | User-visible output mid-sync invites "let me check in on progress" interruptions that break the unattended contract. Final report at the end is the sync's single deliverable. |

## §3 File Inventory

| Path | Lines (~) | Role |
|------|-----------|------|
| `.claude/commands/sync-docs.md` | 690 | The slash command; orchestrator + 13 batch objectives + Phase 0-3 workflow |
| `.claude/skills/query-intel/compose-intel-summary.md` | 310 | Intel-query SSOT; `/sync-docs` registered as Step F consumer |
| `.claude/rules/intelligence.md` | 165 | `/sync-docs` listed under §When to Query |
| `.claude/skills/tpr-review/SKILL.md` | 360 | Downstream — defines `--autonomous` flag semantics (§1) and exit-reason mapping (§5 Autonomous-mode carve-out) |

Added on 2026-04-18:

- `.claude/skills/improve-tooling/cmd-sync-docs-design.md` (this file)

## §4 Lessons from Dogfood / Production Runs

### 2026-04-18 — Autonomous-flag bug + graph-first wiring

**Context.** User invoked `/improve-tooling` with ARGS "I want to make a change to /sync-docs to rely very very heavily on /query-intel, it will instruct the tp agents to use it as well" and then added "Also fix bug where it's not passing the automated or non-interactive flag or whatever we called it ot the /tpr-review command".

**Autonomous-flag root cause.** `/sync-docs` was authored with §Automation Protocol §3 "No `AskUserQuestion` calls" in mind, but the rule applied only to `/sync-docs` itself — not to the `/tpr-review` sub-invocations. `/tpr-review` added an `--autonomous` flag during the 2026-04-17 help-mode refactor, but `/sync-docs` was never updated to pass it. Result: any batch that hit a `/tpr-review §5` cap-exit, §9 transport-failure, or §1 ambiguous-input branch would invoke `AskUserQuestion` mid-nightly and hang forever. The bug is a silent kind — it only manifests when TPR hits an edge case, so morning-after runs that converged cleanly would mask it while the first convergence-failure would strand the sync.

**Fix.** Batch Execution Protocol step 2 now explicitly requires `/tpr-review --autonomous` as the launch form, Automation Protocol step 3 calls out the flag as a MANDATORY contract, and Load-Bearing Invariant I1 above makes it grep-verifiable (`grep -c 'tpr-review --autonomous' .claude/commands/sync-docs.md` should equal the number of launch sites).

**Graph-first root cause.** Pre-refactor, every batch objective told reviewers WHAT to verify against (e.g., `crates/arc/src/`) but not HOW to verify — implicitly licensing manual `cat`/`grep` workflows across 13 batches over hundreds of `.md` files. CLAUDE.md §General Discipline mandates graph-first fact-check but the command didn't enforce it.

**Fix.** Added Phase 0.6 (one-shot materialization), added Graph-First Verification block prepended to every batch's ARGS at launch time (DRY — no per-batch duplication), registered `/sync-docs` as Step F consumer, added Tier B "graph-verified fact" option to Fact-Pairing.

**What to watch.** First real nightly run post-change will either (a) exercise the `--autonomous` path (good — autonomous exits + bug-filing, no hangs) or (b) converge everywhere in interactive-equivalent semantics (also good — just slower validation). If a `/tpr-review` launch in the logs shows missing `--autonomous`, treat as a P0 regression against I1.

## §5 Regressions To Watch For

- [ ] `/tpr-review` invocation from `/sync-docs` missing `--autonomous` — P0. Grep check: every Skill-tool call site referencing `/tpr-review` in `sync-docs.md` must carry the flag. Any future batch addition or refactor that drops it is a contract violation.
- [ ] Graph-First Verification block inlined into a specific batch's objective body — P1. The block is shared OUT-OF-LINE (§Batch Definitions) per I8. A per-batch inline copy risks 13-way drift where one batch updates and others lag.
- [ ] `/sync-docs` dropped from `compose-intel-summary.md` consumer count or Step F registry — P1. The Registry contract (§Step F) requires consumer-file edits to co-commit the Step F entry. Drift produces `DRIFT:intel-extension-registry` findings in future reviewer cycles.
- [ ] Phase 0.6 materialization running per-batch instead of once — P1. Re-running the pre-sweep inside the batch loop would multiply the graph-query cost by 13 and produce inconsistent snapshots mid-sync.
- [ ] `Fact-Pairing:` block citing a graph result without a source `file:line` spot-check — P1. Graph results are DISCOVERY, not authority; per §"The One Rule" Tier B they require a representative source spot-check to count as corroboration.
- [ ] New `.md` file added to the repo without landing in Batch 13 (catch-all) — P2. If the discovery rule (`git ls-files '*.md'` minus Banned Paths) skips it, a glob-filter change is the root cause — find and fix.
- [ ] Phase 3 final-report output appearing mid-sync — P2. Violates I11 (unattended contract). A mid-sync "Batch N report" sounds final and invites user interruption.
- [ ] A new sibling of `/sync-docs` that launches `/tpr-review` without `--autonomous` — P2. The I1 rule applies to ANY nightly-unattended caller of `/tpr-review`, not just `/sync-docs`. Future doc-sync family members must carry the same invariant.

## §6 Improvement Log

### Open items

- [ ] [p2] Consider replacing the `/tmp/sync-docs-intel/` scratch-dir convention with a stable in-worktree location (`.claude/worktrees/sync-docs-YYYY-MM-DD/intel-pre-sweep/`) so the pre-sweep artifact is auditable from the worktree branch rather than session-scratch. Source: design observation 2026-04-18 — the current `/tmp` path is ephemeral, so a crashed run loses the map.
- [ ] [p2] Add a Phase 0.7 "objective composer" sub-step that actually materializes the GRAPH-FIRST + batch-objective + pre-sweep-path concatenation as a file committed to the worktree before Phase 1 launches — lets the user audit the exact ARGS every batch received. Currently the composition happens in the orchestrator's head at launch time; verification is indirect.
- [ ] [p3] Consider tightening I1 via a lint script that runs at the top of Phase 0 and greps `sync-docs.md` for every `/tpr-review` mention missing `--autonomous`, aborting the sync if any are found. Currently the invariant is maintained by humans reading the rule — a mechanical check would be cheap.

### Recently closed

- [x] 2026-04-18 — Fixed P0 bug: `/sync-docs` invoking `/tpr-review` without `--autonomous` (Batch Execution Protocol step 2 + Automation Protocol step 3 + Load-Bearing Invariant I1). Commit pending.
- [x] 2026-04-18 — Wired `/sync-docs` as consumer of intelligence-graph SSOT (`compose-intel-summary.md`) — added Phase 0.6 Intelligence Pre-Sweep, Graph-First Verification block, Tier B "graph-verified fact" option, registry Step F entry, intelligence.md §When to Query entry. Commit pending.
- [x] 2026-04-18 — Created this design log (§Per-Tool Design Logs mandatory for `/improve-tooling` invocations on tools under active evolution). Commit pending.

## §7 How To Use This File In Future Sessions

**When to open it.** Before editing `.claude/commands/sync-docs.md` for any non-typo change: re-read §1 (Design Philosophy) + §2 (Invariants) + §5 (Regressions To Watch For). The sync-docs command touches 13 batch objectives, the Fact-Pairing contract, and cross-invokes `/tpr-review` — a seemingly-local edit can violate multiple invariants simultaneously. When debugging a failed nightly run: read §4 (Lessons) for similar past failure modes.

**When to update it.** (1) Every `/improve-tooling` invocation against `/sync-docs` adds a `- [x]` under §6 Recently closed with date + commit sha + one-line description. (2) Whenever a regression from §5 is caught in the wild, tick the box and add an explanatory note. (3) Whenever an invariant from §2 is changed, dated §4 entry explains the new failure mode, the §2 row flips, and the sync-docs command file changes in the same commit. (4) Whenever a real-use bug surfaces during a nightly run, add a `- [ ]` under §6 Open even if not fixed immediately — tracking non-optional, fixing best-effort.

**What NOT to put here.** (1) Implementation-level description of what the command does — that belongs in `sync-docs.md` itself. (2) Justifications for invariants already stated in §2. (3) Any content that would make this file grow past ~500 lines — at that size it stops being institutional memory and becomes a second command file. Split into topic files if that day comes.
