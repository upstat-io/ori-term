# `/fix-bug` — Design Notes and Improvement Log

**Purpose of this file.** Institutional memory for the `/fix-bug` skill. Captures the **design philosophy** (so future edits don't regress the architecture), the **load-bearing invariants** (things you must NOT change without a concrete plan and full understanding of the failure mode each prevents), and a **running log of improvements and bugs** found during real use.

**When to update this file.** Any time you encounter a bug, surprise, or improvement idea while running `/fix-bug` or any of its consumers (`/fix-next-bug`, `/continue-roadmap` gate, `/review-bugs`). Add a `- [ ]` item under §6 Improvement Log. When you find design-violating behavior in the wild, add it to §5 Regressions To Watch For.

---

## §1 — Core Design Philosophy (KEEP THIS)

1. **Two-part skill: Sonnet Phase 0 dispatcher + Opus inline workflow.** Phase 0 (bug context lookup — purely mechanical file reads from `plans/bug-tracker/`) runs in a Sonnet sub-agent to keep Opus context clean. Phases -1 through 6 (grounding, investigation, TDD, implementation, review) run inline on Opus where judgment matters. The split exists because directory traversal + frontmatter parsing is cheap-to-delegate; root-cause synthesis is not.

2. **Strict status-flag precedence in Part 2 routing.** The handoff carries multiple status flags (Already resolved, Superseded by, Lifecycle markers, Resume mode, fresh start) and the parent picks an action by **first-match precedence**. Adding a new status without placing it correctly in the precedence chain creates either token waste (when a higher-priority condition is missed) or skipped fixes (when a lower-priority condition shadows a real fix path). The chain today is: `ERROR > Already resolved > Superseded by > Lifecycle markers > Resume mode > Phase -1 fresh start`.

3. **Superseded skips Phase -1 entirely.** When a bug is superseded by a plan, Phase -1's "re-read CLAUDE.md + relevant rules files" is **wasteful and wrong** — the bug isn't being fixed by `/fix-bug`, it's being fixed by `/continue-roadmap` on a plan. Reading rules files just to discover this is a ~170k token tax. The Superseded branch routes immediately to `/continue-roadmap` via `AskUserQuestion` and stops.

4. **Phase -1 re-grounding is mandatory for all REAL fix paths.** When `/fix-bug` actually proceeds to investigate-and-fix, re-reading CLAUDE.md and subsystem rules is non-negotiable per `CLAUDE.md` §"context drift across long sessions causes rule violations." Do NOT optimize Phase -1 away to save tokens on real fixes — the savings are negative when a fix violates a rule that was loaded but forgotten. The Superseded skip is the **only** Phase -1 bypass; do not add others.

5. **TDD discipline is enforced by Phase 3 + Phase 4 sequencing.** Tests are written BEFORE implementation. The fix section file's §2 TDD Matrix is filled in BEFORE Phase 3 begins. Phase 4 implementation must make Phase 3 tests pass UNCHANGED. If tests need modification after the fix, either the tests were wrong or the fix was wrong — investigate, don't paper over.

6. **`/tp-help` consensus gate (Phase 1.75) precedes implementation.** Independent dual-source design consensus catches wrong-approach errors BEFORE they're locked into the test matrix or implementation. Skipping it for "obvious" fixes is banned — the consensus *cost* is small (~20-45 min); the cost of a wrong-approach fix that has to be unwound after Phase 4 is enormous.

7. **Plan TPR (Phase 2.5) gate is severity + subsystem-driven, not opinion-driven.** Mandatory for: `critical`/`high` severity OR complexity-elevated subsystems (AIMS, CodeGen, LLVM integration, AOT, Runtime). Skippable only when ALL of: medium/low severity AND non-elevated subsystem AND `/tp-help` converged in round 1. The gate exists because consultative `/tp-help` and adversarial Plan TPR catch different failure classes.

8. **Code TPR (Phase 5) and Hygiene Review are MANDATORY for ALL severities.** No exceptions. The cost of an unnecessary review is small; the cost of a missed correctness bug is large. `/improve-tooling` retrospective is also mandatory at Phase 5 step 5 — this is how the diagnostic surface stays sharp.

9. **Capability regression tracking — fix-time invariant.** Phase 4 step 6 + Phase 5 step 6 jointly enforce: "if this fix disabled, removed, or weakened any capability for soundness, the disabled capability MUST have a concrete re-enablement `- [ ]` checkbox in the owning plan." A fix that disables a capability without a tracked re-enablement is a deferral; deferral is banned per `CLAUDE.md` §Zero Deferral.

10. **Autopilot mode is autonomous, not abbreviated.** When invoked with `--autopilot` (by `/fix-next-bug`), every phase still runs to completion. The only differences are: zero `AskUserQuestion`, zero pausing, zero stopping mid-flow. "Autopilot" never means "skip TPR" or "skip TDD" — those gates are load-bearing regardless of mode.

## §2 — Load-Bearing Invariants

Changing any of these without a concrete plan risks re-introducing the bugs the dogfood runs surfaced. Re-read §4 ("Lessons from Production Runs") before changing.

| # | Invariant | Why (which failure mode it prevents) |
|---|-----------|--------------------------------------|
| I1 | Phase 0 lives in Sonnet sub-agent; Phases -1+ live in Opus inline | Mechanical file reads burn Opus context if inlined; judgment work loses fidelity if delegated |
| I2 | Status-flag precedence: ERROR > Already resolved > Superseded > Lifecycle markers > Resume mode > Phase -1 | Out-of-order matching causes either token waste (Resume runs when Superseded should win) or skipped fixes (Resume shadows lifecycle markers) |
| I3 | Superseded branch skips Phase -1 entirely (no rules-file reads) | Reading types.md (~85k) + typeck.md (~85k) + others when the answer is "use /continue-roadmap" wastes ~170k tokens per invocation; user reported 230k+ total waste in repeated incidents |
| I4 | `**BLOCKER**:` is informational text, NOT a lifecycle marker; `**Blocked**:` IS a marker | Substring matching causes false positives — `**BLOCKER**` matches `**Blocked**` substring and triggers stop-and-report when it shouldn't |
| I5 | Bug entry's `Superseded by:` and plan frontmatter `supersedes:` MUST agree | Single-direction declaration causes drift — a bug marked Superseded but with no plan claiming it leaves the fix orphaned; a plan claiming a supersede target with no bug-entry marker leaves /fix-bug invocations re-discovering the relationship |
| I6 | Phase -1 mandatory for all REAL fix paths (only Superseded skips) | Adding more Phase -1 bypasses ("for simple bugs", "for one-line fixes") accumulates rule violations — the rules files exist precisely because context drifts |
| I7 | Phase 1.6 creates fix-section file IMMEDIATELY after Phase 1.5 scope confirmation, BEFORE `/tp-help` consensus | If consensus runs first, the fix file doesn't exist yet — recovery from a mid-consensus crash loses all investigation context |
| I8 | `/tp-help` (Phase 1.75) cap of 3 rounds with autopilot deadlock fallback | Unbounded consensus rounds can loop forever; deadlock fallback ensures autopilot never hangs; autopilot deadlock MUST surface in `/fix-next-bug` session report so user can audit |
| I9 | Plan TPR (Phase 2.5) mandatory triggers (severity + subsystem) are non-negotiable | Letting reviewers opt out of Plan TPR for "easy bugs" loses the adversarial-review failure-mode coverage; consultative `/tp-help` does NOT replace it |
| I10 | Capability regression tracking REQUIRES owning-plan `- [ ]` anchor when capability is disabled | A disabled capability without a re-enablement anchor is silent deferral; surfaced by BUG-04-077 and BUG-04-078 incidents |
| I11 | `/improve-tooling` retrospective at Phase 5 step 5 is MANDATORY | Pain memory decays within hours; retrospectives at finer granularity (per-bug-fix) capture more than per-section sweeps |
| I12 | Bug entries route via `/fix-bug` ONLY when not superseded; superseded routes via `/continue-roadmap` | Routing a superseded bug through `/fix-bug` re-creates the recovery-playbook fossil that the supersede declaration was meant to retire |
| I13 | Phase 4 step 7 commits via `/commit-push` only — never raw `git commit` | `/commit-push` enforces conventional-commit format, lefthook hooks, and re-grounding on hook failure — bypassing it loses those guarantees |
| I14 | Inline mode Phase 0 validates `kind: plan-blocker-inline` on the frontmatter `sections:` entry before proceeding | Prevents `/fix-bug inline:...#04.2` style misdispatch against planned subsections (which are implementation scope, not blocker fixes); without this gate, a user typo routes `/fix-bug` at a subsection with no fix-section template skeleton, and Phase 1+ edits would corrupt planned-work content |
| I15 | Inline mode refuses dispatch when the parent plan section's `status: complete` | Reopening a closed plan section is a large-blast-radius decision outside `/fix-bug`'s scope; silently appending new findings to a closed section backdates its closure commit and confuses `/verify-roadmap` / `/review-plan` state |
| I16 | Inline mode creates ZERO `plans/bug-tracker/fix-BUG-XX-NNN.md` files | The subsection body IS the fix artifact; minting a sibling tracker fix-file breaks the "plan-owned, not tracker-owned" contract the user explicitly required at 2026-04-19 design confirmation ("no, because it's no longer a bug tracker bug, it's a plan owned bug") |
| I17 | Inline mode's Phase -1 grounding runs for every REAL fix (only the Phase 0 INLINE ERROR branch skips it) | Mirrors tracker mode I6 — context drift affects inline fixes identically to tracker fixes; skipping Phase -1 for "small" inline blockers re-creates the same rules-violation failure mode that I6 was added to prevent |
| I18 | Inline mode's Phase 5 tracker cross-ref closure is CONDITIONAL on the subsection carrying a `**Cross-ref:**` line | Never-mint-tracker rule (I16) has a corollary: never-close-tracker-without-existing-ref. A Phase 5 that aggressively searches the tracker for "matching" entries to close would invent false relationships; closures happen only where `/add-bug --inline` Step I4 already recorded the link |

## §3 — File Inventory (canonical)

Active files:

| Path | Lines (~) | Role |
|------|-----------|------|
| `.claude/skills/fix-bug/SKILL.md` | ~560 | Two-part dispatcher + Opus inline workflow (Phases -1 through 6); includes §Inline Mode Phase Overrides for `/add-bug --inline` companion dispatch |
| `.claude/skills/fix-bug/workflow.md` | ~240 | Sonnet Phase 0 sub-agent: Step 0 dispatch mode detection + Tracker Steps 1–5 + Inline Steps 1b/2b |
| `.claude/skills/fix-bug/fix-section-template.md` | ~150 | Template for `plans/bug-tracker/fix-BUG-XX-NNN.md` files (tracker mode only — inline mode uses the subsection body from `/add-bug --inline` directly) |

SSOT cross-references (these files describe `/fix-bug`'s contract from outside):

| Path | What it owns |
|------|--------------|
| `plans/bug-tracker/00-overview.md` | Bug entry format SSOT (lifecycle markers including `Superseded by:`, `Resolved:`, `Escalated:`, `**Blocked**:`); precedence ordering; status flag definitions |
| `CLAUDE.md` §"Bug fix rigor with `/fix-bug`" | Process commitment ("every bug gets a fix section, even obvious ones") |
| `CLAUDE.md` §"Fix Completeness" | Acceptance criteria checklist that maps to Phase 5 |
| `CLAUDE.md` §"TDD for Bugs" | Phase 3 + Phase 4 discipline |
| `.claude/rules/impl-hygiene.md` §"INVERTED-TDD" | Rule that Phase 5 hygiene review enforces |

## §4 — Lessons from Production Runs

### 2026-04-16 — BUG-04-074 Superseded-Bug Token Waste Incident

**Symptom.** Every invocation of `/fix-bug BUG-04-074` consumed ~230k tokens before reaching the conclusion "this bug is superseded by `plans/empty-container-typeck-phase-contract/`, route to `/continue-roadmap` instead." User reported the pattern repeats "every single time."

**Token decomposition** (approximate):
- `/continue-roadmap` scan agent (when invoked from gate): ~50k
- `/fix-bug` Phase 0 Sonnet sub-agent: ~10k
- Phase -1 grounding (CLAUDE.md is loaded by harness; rules files re-read inline): ~170k
  - `.claude/rules/impl-hygiene.md`: ~5k
  - `.claude/rules/compiler.md`: ~3k
  - `.claude/rules/tests.md`: ~5k
  - `.claude/rules/canon.md`: ~10k
  - `.claude/rules/types.md`: ~85k (largest single waste)
  - `.claude/rules/typeck.md`: ~85k (second-largest)
- Plan overview + fix-file reads: ~30k
- AskUserQuestion deliberation + output: ~5k

**Root cause analysis.**

1. **No `Superseded by:` lifecycle marker existed in the bug-tracker schema.** Only `Resolved:`, `Escalated:`, `**Blocked**:`, `<!-- blocked-by:` were recognized. Bugs that should route to a plan had no canonical way to declare it.
2. **Sonnet's Phase 0 handoff buried the supersede relationship in prose.** The handoff said "Plan that supersedes the fix file: `plans/empty-container-typeck-phase-contract/`... the parent Opus should determine how execution continues" — the answer was present but unstructured. Opus had to discover it through fix-file frontmatter inspection, not flag-driven dispatch.
3. **Substring false positive on `**BLOCKER**:` matched `**Blocked**:`.** The bug entry contained `**BLOCKER**: Wiring validate_body_types into check_function (§03.1) causes...` (informational impact text). Sonnet's grep for `**Blocked**` substring matched, classified as a lifecycle marker, but reported the full text. Opus saw "this is impact text, not a stop signal" and proceeded — but the marker check was supposed to STOP. The recognition pattern conflated two different lexical patterns with similar substrings.
4. **Resume mode shadowed the lifecycle marker check in practice.** The fix file existed with `status: in-progress`, so the handoff carried `Resume mode: yes — pick up at Phase 2.5`. Even when lifecycle markers were technically "present", Resume mode's "yes — pick up" shape was actionable while lifecycle markers' shape was just narrative — Opus picked the actionable one.
5. **Phase -1 fired unconditionally.** Even after determining the right answer was "stop and route to `/continue-roadmap`", the Phase -1 mandatory rules-file reads had already happened. The grounding cost was paid before the routing decision was made.

**Fixes applied (commit pending).**

1. Added `Superseded by:` to `plans/bug-tracker/00-overview.md` Bug Entry Format with explicit precedence documentation (§Lifecycle marker precedence block).
2. Marked BUG-04-074 in `plans/bug-tracker/section-04-codegen-llvm.md` with `Superseded by: plans/empty-container-typeck-phase-contract/` line.
3. Updated `.claude/skills/fix-bug/workflow.md` Step 3 to:
   - Add explicit `Superseded by` check at position #2 (highest after Already resolved).
   - Cross-check bug-entry marker against plan frontmatter `supersedes:` SSOT (drift detection).
   - Distinguish `**Blocked**:` (marker) from `**BLOCKER**:` (impact text) explicitly.
   - Force Resume mode to `no — superseded` when Superseded fires, preventing precedence inversion.
4. Updated `.claude/skills/fix-bug/SKILL.md` Part 2 to add a `Superseded by` branch with HIGHEST priority after Already resolved. The branch:
   - STOPS IMMEDIATELY before Phase -1 (the ~170k save).
   - Reports concisely without pre-loading plan context.
   - Offers `AskUserQuestion` routing: `/continue-roadmap`, just-report, or mark-fix-section-superseded.
5. Created this design log to capture the failure mode for future sessions.

**Verification.** Re-running `/fix-bug BUG-04-074` on a fresh session should now:
- Phase 0 (Sonnet) detects `Superseded by:` line and emits `Superseded by: plans/empty-container-typeck-phase-contract/ — sourced from both` in the handoff.
- Phase 0 forces `Resume mode: no — superseded` regardless of fix-file status.
- Part 2 reads the handoff, matches the Superseded branch, reports concisely, and emits the AskUserQuestion routing prompt.
- **Phase -1 is never executed.** No rules-file reads. Token cost: ~10k Phase 0 + ~5k routing = ~15k total (down from ~230k, a ~93% reduction).

### 2026-04-16 — Latent issue: BUG-04-042 lifecycle marker precedence

While investigating the BUG-04-074 incident, observed that BUG-04-042 has a recognized `**Blocked**:` lifecycle marker but the prior session also let Resume mode override it. The new precedence in workflow.md Step 3 (Superseded > Lifecycle > Resume) addresses this for Superseded but doesn't fix Lifecycle vs Resume precedence directly. Filed as §6 Open item — needs a separate audit.

### 2026-04-16 — Latent issue: BUG-04-084 informational `**BLOCKER**:` text

BUG-04-084 contains `**BLOCKER**:` impact-statement text that triggers the same false-positive substring match. Per the new workflow.md Step 3 distinction, Phase 0 should now skip this — but only if Sonnet correctly applies the marker-vs-impact distinction. Needs verification on next invocation.

### 2026-04-19 — Inline-subsection dispatch (paired with `/add-bug --inline`)

**Motivation.** Same-session companion to the `/add-bug --inline` feature landed in `add-bug-design.md` §4. That feature created plan-blocker subsections with the full `/fix-bug` template skeleton inside plan sections. Without matching `/fix-bug` dispatch support, callers had to apply Phases -1 through 6 manually against the subsection body — an obvious friction point flagged as `[p1]` in `add-bug-design.md` §6 Open items at feature-land time. The user's continuation prompt ("continue") explicitly invoked the follow-up pass to close that gap.

**Design (four-part split mirroring tracker mode):**

1. **Argument form** — `/fix-bug` accepts two new argument shapes: `inline:<plan-section-path>#<subsection-id>` (explicit prefix) and `<plan-section-path>#<subsection-id>` (shorthand, detected when arg starts with `plans/` and contains `#`). Tracker shapes (`BUG-XX-NNN`, free-form description) are unchanged.
2. **Phase 0 (Sonnet sub-agent) branch** — `workflow.md` gains a "Step 0: Dispatch Mode Detection" block at the top. Inline mode routes to new Steps 1b + 2b; tracker mode routes to the original Steps 1–5 unchanged. Inline Phase 0 does three things: (a) parse path + subsection id, (b) validate `kind: plan-blocker-inline` on the frontmatter `sections:` entry AND parent `status:` ≠ `complete`, (c) extract the body subsection + parse skeleton state per §-block to determine Resume-mode phase. Returns a new `[INLINE]` handoff format or an `INLINE ERROR` handoff.
3. **Part 2 (Opus) mode dispatch** — a new mode-dispatch layer sits above the existing tracker precedence chain. `[INLINE]` handoff → Inline Mode. `INLINE ERROR` handoff → report and stop (refuse fallthrough to tracker). Plain tracker handoff → tracker precedence chain unchanged.
4. **§Inline Mode Phase Overrides** — tabulated per-phase deltas. The key differences: Phase 1.6 (create fix-section-file) is SKIPPED (the subsection body IS the fix section); Phases 1/1.5/1.75/2/2.5/3/4/5 edit the subsection body in-place; Phase 5 closure flips the frontmatter `sections:` entry `status:` to `complete`, fills §R/§N in the body, and closes any tracker cross-ref the `/add-bug --inline` Step I4 recorded. No fix-BUG-XX-NNN.md file is created — honoring the user's explicit "no, because it's no longer a bug tracker bug, it's a plan owned bug" decision.

**What this feature does NOT change:**
- Tracker-mode behavior is entirely untouched. Every precedence rule, handoff field, and phase step works exactly as before for `BUG-XX-NNN` invocations.
- Phase -1 grounding is still mandatory for inline mode — inherited from I6 and strengthened by new invariant I17. The only Phase -1 skip remains Superseded (I3 + §1 #3).
- `fix-section-template.md` is unchanged. Inline mode doesn't use it — the subsection body was written by `/add-bug --inline` with the same shape embedded.

**Cross-reference:** the user's design confirmation answers drove four explicit decisions recorded in the §Inline Mode Phase Overrides table:
- Subsection lands under the currently-active section (Option b) — enforced by `/add-bug --inline` Step I1 parsing.
- No `BUG-XX-NNN` ID minted — enforced by invariant I16.
- Full `/fix-bug` rigor — enforced by "every phase runs with same rigor as tracker mode; only the artifact changes" preamble.
- Big-picture analysis as `/add-bug` Step 0 — orthogonal, belongs to `/add-bug`, not `/fix-bug`; `/fix-bug` inherits the classification via `kind: plan-blocker-inline`.

**Verification.** Dogfood: next time `/add-bug --inline` seeds a blocker subsection, run `/fix-bug inline:<path>#<id>` and observe: (a) Phase 0 Sonnet returns an `[INLINE]` handoff, (b) Phase -1 + Phase 1 run unchanged, (c) Phase 1.6 is SKIPPED and no `fix-BUG-*.md` appears in `plans/bug-tracker/`, (d) Phase 5 closure edits only the plan-section file + (if cross-refs) the tracker file.

## §5 — Regressions To Watch For

Pre-edit sanity check before changing `/fix-bug` SKILL.md or workflow.md. If any of these patterns is true, you're about to re-introduce a known regression:

- [ ] Removing or weakening the Superseded-branch Phase -1 skip — re-introduces 170k+ token waste per invocation
- [ ] Re-adding precedence inversions where Resume mode shadows higher-priority status flags — re-introduces fossil-fix-file recovery
- [ ] Conflating `**BLOCKER**:` (text) with `**Blocked**:` (marker) in regex/grep patterns — re-introduces false-positive lifecycle stops
- [ ] Optimizing Phase -1 away for "simple" bug fixes — re-introduces context-drift rule violations
- [ ] Adding a Phase -1 bypass for any non-Superseded path — see I6
- [ ] Skipping `/tp-help` Phase 1.75 for "obvious" fixes — wrong-approach errors propagate to Phase 4 and require unwinding
- [ ] Skipping Plan TPR (Phase 2.5) for `critical`/`high` severity or complexity-elevated subsystems — see I9
- [ ] Marking a fix complete without capability-regression tracking — re-introduces silent deferral
- [ ] Routing a superseded bug through `/fix-bug` execution instead of `/continue-roadmap` — wastes work the plan owns
- [ ] Allowing `/fix-bug --autopilot` to skip TDD or TPR — autopilot is autonomous, not abbreviated
- [ ] Inline mode falling through to tracker mode when the `[INLINE]` handoff arrives — mode dispatch must halt the tracker precedence chain; fallthrough would route an inline target through bug-tracker lookup logic that doesn't apply
- [ ] Inline mode creating a `plans/bug-tracker/fix-BUG-XX-NNN.md` file in Phase 1.6 under a future "consistency" refactor — see I16; the user's explicit decision is that inline subsections are plan-owned, not tracker-owned
- [ ] Inline mode's `kind: plan-blocker-inline` validation being relaxed ("be forgiving and work with planned subsections too") — see I14; this re-opens the door to corrupting planned-work subsections during a fix
- [ ] Inline mode ignoring the parent section's `status: complete` gate — see I15; silent reopening of closed plan sections
- [ ] Inline mode's Phase 5 minting tracker cross-refs where `/add-bug --inline` didn't record them — see I18; breaks the "never-mint-tracker" contract from the companion feature
- [ ] An `INLINE ERROR` handoff triggering a fallthrough to tracker mode under the description-search branch — `INLINE ERROR` must stop the invocation; the user asked for a specific inline target, and substituting a different one (or no target) is wrong behavior, not graceful degradation

## §6 — Improvement Log

### Open items

- [ ] **[p2]** Add unit tests for `scripts/plan_corpus/bug_markers.py` and `bug_validators.py` in `tests/plan-audit/test_bug_markers.py`. Cover: marker precedence (Superseded > Escalated > Blocked > blocked-by), `**BLOCKER**` vs `**Blocked**` substring distinction, supersede-target extraction, drift detection (missing/orphan), idempotent auto-fix application. The end-to-end synthetic test in this round's verification is informal; promote to a proper pytest case.
- [ ] **[p2]** Consider promoting `superseded_by:` to a fix-section frontmatter field (parallel to `plan` and `references`), with `status: superseded-by-plan` as a recognized fix-file lifecycle state in `scripts/plan_corpus/types.py` (extending the existing `SUPERSEDED = "superseded"` status). Lets `plan_corpus` enforce the relationship at the fix-file level too.
- [ ] **[p2]** Update `/fix-next-bug` SKILL.md (lines 158-159, 199-201, 218-224) to add `Superseded` to the lifecycle-state list and the session-summary report categories. Today the SKILL.md only mentions `Escalated` and `Blocked`; `bug_queue_scan.py` excludes Superseded but the SKILL doc lags.
- [ ] **[p2]** Update `/review-bugs` command (`.claude/commands/review-bugs.md`) to recognize `Superseded by:` as a valid open-but-not-actionable state during triage. Today it would surface superseded bugs as "open bugs needing OBE check" — wasted triage cycles.
- [ ] **[p3]** The Phase 0 Sonnet handoff format adds a `Superseded by` line but doesn't yet pre-fetch the plan's current section progress. Could optionally read the plan's `00-overview.md` Quick Reference table to surface "current section: 03.2" so the routing prompt is even more informative. Low priority — `/continue-roadmap` figures it out anyway.

### Recently closed

- [x] **2026-04-19** — **Inline-subsection dispatch (companion to `/add-bug --inline`).** Extended `/fix-bug` to accept `inline:<plan-section-path>#<subsection-id>` (and path-shorthand) as a target argument. Changes: `workflow.md` gained "Phase 0 — Step 0: Dispatch Mode Detection" and new Steps 1b/2b for inline extraction (parse + `kind: plan-blocker-inline` + parent-`status` validation, body extraction, skeleton-state Resume-mode inference); `SKILL.md` Part 1 dispatcher instruction updated to route via Step 0, Part 2 gained a mode-dispatch layer above the tracker precedence chain (matches `[INLINE]` / `INLINE ERROR` / tracker handoffs), and a new `§Inline Mode Phase Overrides` block tabulates per-phase deltas. Key design decisions (from user confirmation earlier in session): Phase 1.6 SKIPPED in inline mode, no `fix-BUG-XX-NNN.md` minted, subsection body is the fix artifact, Phase 5 closure edits only the plan-section file (+ tracker cross-ref if one exists). New invariants I14–I18 added to §2. New §4 Lessons entry "Inline-subsection dispatch". New §5 regression rows (7 total). Strip-as-you-go prose-lint pass cleaned 6 pre-existing paragraph violations alongside the new content. Companion entry in `add-bug-design.md` §6 Recently closed (closes the `[p1]` Open item that the `/add-bug --inline` feature had left pointing here). Commit: pending.
- [x] **2026-04-16** — **Round 3 audit (user-prompted schema codification + auto-fix automation):** elevated the `Superseded by:` marker from "documented in two scanners" to first-class schema enforced programmatically.
  - **New SSOT module:** `scripts/plan_corpus/bug_markers.py` (~250 lines) — marker regex constants, `BugEntry` dataclass, `parse_bug_entries()` generator, `classify_bug_exclusion()` precedence-ordered classifier, `extract_supersede_target()`, `extract_repro()`, `extract_subsystem()`, `normalize_severity()` (handles `[critical→medium]` reclassification). Header regex tolerates trailing text after `**Title**` (closes the BUG-04-042 false-negative I introduced in Round 2's first-cut header regex).
  - **New validator module:** `scripts/plan_corpus/bug_validators.py` (~250 lines) — `find_supersede_drift()` cross-checks plan frontmatter `supersedes:` declarations against bug-entry `Superseded by:` markers in both directions; `plan_auto_fixes()` generates `PlannedEdit` objects for missing markers (auto-fixable); orphan markers surfaced for manual review (NOT auto-fixed — plan frontmatter is the SSOT). `apply_planned_edits()` is idempotent.
  - **Migrated both scanners to SSOT:** `bug_queue_scan.py` and `roadmap_scan.py` now import marker regexes + classifier from `plan_corpus.bug_markers`. Removed ~30 lines of duplicated regex/classifier from each. Closes the `LEAK:algorithmic-duplication` finding from Round 2's open items. `roadmap_scan.py`'s `parse_bug_tracker_bugs` now delegates parsing entirely to `_ssot_parse_bug_entries`, then converts BugEntry → local Bug for Gate 1.92.
  - **New auto-fix gate:** `roadmap_scan.py` Gate 1.6 `bug_marker_drift` (`auto-fix` severity) — computes drift report into JSON payload with planned edits + orphan findings. `/continue-roadmap` workflow.md Step 2c (new) applies the edits via Edit tool; Step 2d commits everything together. Step 3 gate table updated with the new gate.
  - **Schema documented in `/create-plan/plan-schema.md`** (the user's ask — "schema defined inside the /create-plan skill folder") — new `## Bug Tracker Section Schema` section covering: section file structure, bug entry format, lifecycle markers + precedence, **`**BLOCKER**` vs `**Blocked**:` substring distinction, bidirectional supersede invariant, enforcement-module references.
  - **`plans/bug-tracker/00-overview.md` updated** to point at `/create-plan/plan-schema.md` as canonical and list the enforcement modules (`bug_markers.py`, `bug_validators.py`, Gate 1.6).
  - **End-to-end verified:** synthetic drift test in tmpdir → drift detector found missing marker → `apply_planned_edits` inserted properly-formatted line with provenance attribution → re-run drift detector clean (idempotent). Real-corpus run: `bug_marker_drift` gate fires=False (BUG-04-074's marker is present from Round 1 work). All 624 `tests/plan-audit/` tests pass post-migration.
  - Pending commit (this work).
- [x] **2026-04-16** — **Round 2 audit (user-prompted SSOT-spread close-out):** added `Superseded by:` detection to BOTH consumer scanners that previously didn't know about it. (1) `bug_queue_scan.py`: added `SUPERSEDED_RE` constant, new exclusion branch with Superseded > Escalated > Blocked > blocked-by precedence, docstring update, `**BLOCKER**:` vs `**Blocked**:` distinction documented. (2) `roadmap_scan.py`: added 4 marker regex constants (`BUG_SUPERSEDED_RE`, `BUG_ESCALATED_RE`, `BUG_BLOCKED_RE`, `BUG_BLOCKED_BY_COMMENT_RE`), added `excluded_reason` field to `Bug` dataclass, refactored `parse_bug_tracker_bugs` to read body lines and populate exclusion reason, updated `_bug_tracker_relevance` to filter by `excluded_reason is None`. **Verified empirically:** `critical_bugs` gate count dropped from 3 → 1 (BUG-04-074 superseded, BUG-04-042 already had `**Blocked**:` marker now respected, BUG-04-084 correctly retained as actionable since `**BLOCKER**:` is impact text not a marker). **Bonus fix:** the `**Blocked**:` marker on BUG-04-042 was previously bypassed by the gate's severity-only filter — now correctly excluded. Pending commit (this work). Surfaced by user follow-up: "is this marker documented in places? did you add it to plan schema?"
- [x] **2026-04-16** — **Round 1:** add `Superseded by:` lifecycle marker to bug-tracker schema, mark BUG-04-074, update `/fix-bug` workflow.md + SKILL.md to detect-and-route via `/continue-roadmap` without entering Phase -1. Saves ~170k tokens per superseded-bug invocation. Surfaced by user feedback citing repeated 230k-token waste pattern. Pending commit.

## §7 — How To Use This File In Future Sessions

**Before editing `/fix-bug`'s SKILL.md or workflow.md:** open this file. Read §1 (Design Philosophy), §2 (Load-Bearing Invariants), and §5 (Regressions To Watch For). The invariants in §2 each map to a specific failure mode in §4 — changing them without understanding the failure mode means re-discovering the bug. The regressions in §5 are pattern-matched checks: if your edit matches a `- [ ]` item, you're about to re-break something.

**After every `/improve-tooling` session that touches `/fix-bug`:** add a `- [x]` entry under §6 Recently closed with today's date + one-line description + commit sha (or "pending commit"). If the session surfaced new failure modes, add a §4 Lessons entry with full root-cause analysis.

**When debugging a misbehaving `/fix-bug` invocation:** scan §4 Lessons first — your symptom may match a known failure mode and the fix may already be documented. If it's a new failure, add it to §6 Open items even if you can't fix it immediately. Tracking is non-optional; fixing is best-effort.

**When adding a new lifecycle marker, status flag, or routing branch:** verify the precedence chain in §1.2 still holds. New status flags MUST be inserted at the correct precedence tier with explicit documentation in both `workflow.md` Step 3 and `SKILL.md` Part 2. Mismatched precedence between the two files is a recurring source of bugs.
