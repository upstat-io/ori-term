# `diagnostics/state.sh` — Design Notes and Improvement Log

**Purpose of this file.** Institutional memory for `diagnostics/state.sh` and its cache file `.claude/state/known-state.json` (schema v1). Captures the **design philosophy** that justifies a cache file over re-running test-all.sh each session, the **load-bearing invariants** that keep the cache from lying, and a **running log of improvements and bugs** from real use.

**Context.** Created 2026-04-18 in response to `/improve-tooling` invocation: "We need a global state indicator, because right now we know the state is broken but every time you have to figure this out all over again, run the tests, do research, go through the whole thing and it takes forever." Initial implementation during the `empty-container-typeck-phase-contract` §03.5 close-out session — the same session that ran `cargo test --all`, parsed 843 failures, and cross-referenced 35 file names purely to re-derive state that the plan already documented.

**When to update this file.** Any time a bug, surprise, or improvement lands in `state.sh` or `known-state.json`. Add a `- [ ]` under §6 Improvement Log (Open) or `- [x]` under §6 Recently closed. When consumers start to drift (new skills reading the cache, new subcommands, new fields), update §1 + §2. When real-use bugs surface (wrong staleness classification, parser failures, atomic-write race), add a dated §4 entry.

---

## §1 — Core Design Philosophy (KEEP THIS)

1. **Cache is an index, not a source of truth.** The plan-documented "Known Failing Tests" sections remain the SSOT for intent. `known-state.json` is a fast index over that intent, keyed by the commit SHA it was computed at. When in doubt, the plan wins; the cache is the shortcut.

2. **Fail-safe toward "unknown", never toward "clean".** Every consumer MUST treat SHA mismatch, a dirty working tree, or a missing state file as "unknown" — they run real checks, not trust the cache. The cache is only trusted when it is DEMONSTRABLY fresh.

3. **Cheap updates on every commit; expensive updates at explicit boundaries.** `commit-push` post-commit calls `state.sh refresh --sha-only` (sub-100 ms: just updates HEAD SHA + timestamp + `updated_by`). Full test-suite re-runs happen at section close (`/continue-roadmap` close-out), on explicit request, or nightly — never on every commit. This prevents 2-3 min per-commit latency while keeping the cache's "what was true at SHA X" claim honest.

3. **The SHA is the claim.** `known-state.json.head_sha` records "this is what was true at commit X". Consumers verify by comparing against `git rev-parse HEAD`. If they differ, the cache is **obsolete** (a commit happened, state is unknown until a refresh). If they match but the tree is dirty, the cache is **stale** (valid for the committed tree but unknown for uncommitted edits). Only "matches + clean" = **fresh**.

4. **Git-tracked so parallel sessions share it.** The cache file is checked in. When two sessions pull the same HEAD, they see the same cached state. Merge conflicts on `known-state.json` ARE possible but are cheap to resolve (run `state.sh refresh --full` and commit the winner). Git tracking matters more than conflict rarity — a gitignored per-dev cache would defeat the whole point of session-shared knowledge.

5. **One canonical tool, one canonical cache.** `diagnostics/state.sh` is the only writer. Skills NEVER edit `known-state.json` directly — they call `state.sh refresh` with the appropriate subcommand/flag. Direct edits by skills = drift.

6. **Subcommand per use case, not one monolithic flag table.** `show`, `check`, `refresh`, `known-failing`. Each has a clear purpose (summary / freshness verdict / mutation / machine-friendly file list). This is the same pattern as `scripts/intel-query.sh` and `scripts/plan_corpus` — subcommand-per-use-case ages better than flag matrices.

7. **Atomic writes via `write-tmp-then-rename`.** Concurrent writes (e.g., two parallel sessions both committing at the same moment) can't clobber — each writes to `known-state.json.tmp.$$` and does a single `mv` into place. The OS guarantees rename atomicity on the same filesystem.

8. **`jq` as hard dep for mutations, optional for reads.** Writing JSON by hand in bash is a bug factory. `jq` is universal on dev machines. `show --json` avoids `jq` (cat the file); everything else requires it.

9. **Known-failing list is editorial, not auto-derived.** `refresh --full` updates test-suite TOTALS but does NOT auto-populate `known_failing_files`. That list maps failing tests to the plan section that owns remediation — that mapping is editorial. Auto-populating from test-all.sh output would confuse "the validator correctly flags this" with "we've scoped this as a known-failing test under plan X" — two different claims.

10. **Schema version is load-bearing.** `schema_version: 1` is the contract between writer and consumers. When a future change needs to break schema (e.g., rename a field, split totals), bump to `schema_version: 2` and let consumers branch on it. Never silently reshape without bumping.

## §2 — Load-Bearing Invariants

Changing any of these without a concrete plan risks re-introducing the bugs the initial session surfaced.

| # | Invariant | Why (which failure mode it prevents) |
|---|-----------|--------------------------------------|
| S1 | `state.sh` never writes when `--sha-only` is passed on a MISSING state file | First-time setup must use `refresh --full` or manual creation. `--sha-only` on missing would create a state file with no meaningful content but the illusion of cached data. |
| S2 | `state.sh check` exits 0 ONLY when SHA matches HEAD AND tree is clean | Any weaker definition of "fresh" invites consumers to trust stale state on a dirty tree. |
| S3 | The `known_failing_files` list is authoritative about the plan-documented failing set, NOT the current test-all.sh output | A diff between the cached list and current output = a finding the plan doesn't cover → consumer must investigate, not silently accept. |
| S4 | Atomic write via `$STATE_FILE.tmp.$$` + `mv` | Parallel sessions can trigger concurrent writes; without atomicity the cache can end up half-written and crash `jq` on next read. |
| S5 | `schema_version` field is always present; consumers MUST check it before parsing | Schema evolution over time WILL happen; without version-gating, old consumers on new files will misread fields and silently produce wrong results. |
| S6 | `refresh --full` does NOT auto-populate `known_failing_files` from test-all.sh output | That list is editorial (plan-section scoped); auto-population would conflate "test fails" with "scoped as expected failure under plan X". |
| S7 | `updated_by` field records the trigger (`commit-push`, `manual`, `full-check`, `section-close`) | Auditing drift requires knowing who wrote the last update; pre-refresh consumers can read `updated_by` to decide whether the state is trustable for their purpose. |
| S8 | Default output for `show` is human-readable; JSON requires explicit `--json` | Skill output accidentally dumped to a terminal should be readable; JSON-as-default is machine-only and hostile to manual inspection. |
| S9 | `state.sh` lives in `diagnostics/`, not `scripts/` | Per `impl-hygiene.md §Documentation Surfaces`, diagnostic scripts MUST live in `diagnostics/` and their canonical doc surface is `diagnostics/README.md`. |
| S10 | `state.sh` never destroys existing state on error; it refuses and exits non-zero | If a refresh fails partway (test-all.sh timeout, parse failure, jq error), the prior state file stays intact — the `.tmp.$$` file is orphaned but the live cache is unchanged. Better stale-but-valid than corrupt-or-missing. |

## §3 — File Inventory (canonical)

Active files as of 2026-04-18:

| Path | Lines (~) | Role |
|------|-----------|------|
| `diagnostics/state.sh` | 250 | The tool: subcommands `show` / `check` / `refresh` / `known-failing` with `--json` / `--human` / `--sha-only` / `--full` / `--hygiene-only` / `--by` options |
| `.claude/state/known-state.json` | ~85 | The cache (schema v1): test_suite, clippy, hygiene, remediation metadata |
| `diagnostics/README.md` §`state.sh` | 40 | Canonical user docs per `impl-hygiene.md §Documentation Surfaces` |
| `.claude/rules/diagnostic.md` | 1 row | Secondary quick-reference table entry |
| This file (`script-state-design.md`) | 150 | Design log per `/improve-tooling` §Per-Tool Design Logs |

## §4 — Lessons from Dogfood / Production Runs

### 2026-04-18 — Initial pain point that motivated the tool

- **Symptom:** During `empty-container-typeck-phase-contract` §03.5 close-out, Claude ran `cargo test --all` (~2 min), grepped the output for file names (2 more tool calls), cross-referenced against the plan's Known Failing Tests table, and confirmed the 35 files were the expected set. This happened despite the plan already documenting the state in prose — the information was just not session-queryable.
- **Cost:** ~3 tool-call rounds of pure rediscovery per fresh session. Multiplied across the 6-8 active plan sections, the cost compounds fast.
- **Fix:** Introduce this tool + cache. Consumers consult `state.sh show --json` before running tests.
- **Follow-up observation:** The `test-all.sh` summary table (`TOTAL` row) is machine-parseable but not obviously so — `state.sh refresh --full` uses `awk '/^TOTAL/ {print $2, $3, $4}'` which works today but would break if the summary format changes. Consider adding a `test-all.sh --json` flag separately to harden the parser.

## §5 — Regressions To Watch For

- [ ] `state.sh show` without `--json` fails when `jq` is not installed — check if `jq` detection in `require_jq()` always fires a clear error (should exit 3 with install guidance, not cryptic bash failure).
- [ ] Concurrent `refresh --sha-only` from two parallel sessions committing at the same second: atomic rename should win the last one, but verify by hand-running two simultaneously.
- [ ] `refresh --full` parsing `TOTAL` row: if `test-all.sh` changes its summary format, the parser silently reads blanks → all totals become 0 → fake "clean" state.
- [ ] New `updated_by` value added to the semantic set but not documented in `--help` or the schema: drift in SSOT.
- [ ] Schema v2 upgrade without bumping `schema_version`: consumers on v1 semantics misread v2 fields.
- [ ] `known_failing_files` drifts from the plan's Known Failing Tests table: commit-push doesn't auto-sync them (S6 invariant); manual edits to one without the other create drift.
- [ ] Consumer uses `state.sh show --json` and doesn't check `check`'s exit code: trusts stale or obsolete state.

## §6 — Improvement Log

### Open

- [ ] [p2] Add `state.sh diff` subcommand: compare current `cargo test --all` output against cached `known_failing_files` and flag any NEW failing file not in the cache. This is the "did I regress?" check that would have caught any bug introduced by §03.5's test additions.
- [ ] [p2] Wire consumers: `/continue-roadmap` scanner gate reads `state.sh show --json` to skip dirty-tree discovery; `/roadmap-work` Step 5 compares test results against cached totals instead of interpreting absolute counts.
- [ ] [p3] Harden `refresh --full` parser: add a unit-style test (`diagnostics/self-test.sh`) that feeds it a canned `test-all.sh` log and asserts the parsed totals match expectation. Guards against summary-format drift.
- [ ] [p3] `state.sh history` subcommand showing the last N `updated_at` / `updated_by` pairs — requires per-update append rather than in-place rewrite. Defer until demand.
- [ ] [p3] Add `reviews.last_tpr_sha` / `reviews.last_hygiene_sha` fields for skill consumers that care about "has TPR run since this commit?". Defer until a concrete consumer asks.

### Recently closed

- [x] 2026-04-18 — Initial tool implementation + cache file + docs + design log. Committed at `4da13d37`.
- [x] 2026-04-18 — `/commit-push` workflow.md wired to call `state.sh refresh --sha-only --by commit-push` as Step 8 (after Step 7 push). Failure is non-fatal — consumers fall back to actual runs when cache is obsolete.
- [x] 2026-04-18 — `/continue-roadmap` wired as first consumer: new Step 1.5 reads `state.sh show --json` and carries cached state through to the handoff's new "Cached repo state" block. Hard-bans list carved out for read-only state.sh operations (show / check / known-failing) — they don't compile or run tests, just cat JSON. The scanner's JSON plus state.sh is the sub-agent's complete world-state.
- [x] 2026-04-18 — **Self-exclusion in `is_tree_dirty()`**: after `refresh --sha-only`, the state file itself is uncommitted, which would make `check` report STALE forever even when everything else is clean. Fix in `is_tree_dirty()`: grep-exclude `.claude/state/known-state.json` from `git status --porcelain` output. Load-bearing for `/commit-push` Step 8 — without it, the post-push refresh produces a perpetually-STALE cache that consumers always mistrust, defeating the tool's point. Surfaced during the same session as the tool's creation (FRESH path never demonstrable without it).

## §7 — How To Use This File In Future Sessions

Open this file when:
- You're about to change `diagnostics/state.sh` or `known-state.json` schema — re-read §1 + §2 first.
- You noticed `state.sh` misbehaved in the wild — add a dated §4 entry + a `- [ ]` under §6 Open.
- A consumer skill wants to read the cache — check §1 #2 (fail-safe invariant) and §2 S2 (check-exit-0 contract) before wiring it.
- You're adding a new subcommand — document the intent in §1, invariant in §2, and log the addition in §6 Recently closed.

Update this file when:
- Every `/improve-tooling` session on `state.sh` adds a `- [x]` entry under §6.
- A regression from §5 surfaces in real use — add it to §4 Lessons with date + symptom + fix.
- An invariant from §2 changes — add a dated §4 entry explaining the new failure mode the old invariant caused, flip the row, and update `state.sh` + `diagnostics/README.md` + `.claude/rules/diagnostic.md` in the same commit.
