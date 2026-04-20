# `roadmap_scan.py` + `/continue-roadmap` workflow.md — Design Notes and Improvement Log

**Purpose of this file.** Institutional memory for the scanner that powers `/continue-roadmap`. Captures the **design philosophy** of the scanner's auto-fix pipeline (so future edits don't regress it), the **load-bearing invariants** (what must not change without a plan), and a **running log of drift patterns discovered in the wild**.

**Scope.** Covers `compiler/.claude/skills/continue-roadmap/roadmap_scan.py` (the scanner) and `compiler/.claude/skills/continue-roadmap/workflow.md` (the sub-agent protocol that consumes the scanner's JSON). These two files are one logical tool — the scanner produces gate decisions, the workflow applies them.

**When to update this file.** Any time `/continue-roadmap` loops, blocks on a gate that should auto-clear, or an auto-fix is added to handle a new drift class. Add a `- [ ]` item under §Improvement Log for in-the-wild findings; add a `- [x]` when a fix lands.

---

## §1 — Core Design Philosophy (KEEP THIS)

1. **Scanner is the brain; workflow is the hands.** `roadmap_scan.py` pre-computes every gate decision, focus-context field, and next-unblocked pointer into one JSON envelope. `workflow.md` is a transcription contract — no re-derivation, no re-parsing, no re-running of logic that the scanner already produced. If the sub-agent needs new information, it goes in the scanner output, not in new workflow logic.

2. **Auto-fix cleanup is silent and mandatory.** Any gate with `severity: "auto-fix"` is applied inline without user prompts, without `AskUserQuestion`, without surfacing options. The user has pre-approved mechanical cleanup (see user memory `feedback_auto_fix_cleanup.md`). A gate that "needs human judgment" is NOT an auto-fix — it is a `block` gate.

3. **Every drift pattern must be healable.** If a gate fires (`block`), there must be a skill or auto-fix path that clears it. A gate that fires but has no clearing mechanism is a loop — `/continue-roadmap` blocks on it, the suggested skill does nothing, the scan re-fires the gate, forever. Every new `block` gate MUST have a corresponding skill or auto-fix.

4. **Mismatches flow through one channel.** `detect_all_mismatches()` is the SSOT for "what stale frontmatter exists". Scanner-internal detection lives on `Section.mismatch`, `Section.tpr_mismatch`, `Subsection.mismatch`, and is aggregated by `detect_all_mismatches()`. Gate 1.5 `stale_frontmatter` reads that one aggregate. Adding a new drift pattern means adding a new `*_mismatch` property, NOT a new gate.

5. **Focus plan only for auto-fixes.** The scanner reports mismatches across every plan, but the sub-agent applies fixes ONLY inside the focus plan. Editing sibling plans during a `/continue-roadmap` scan creates cross-plan churn that the user didn't request; let those plans be handled when they are the focus.

6. **Workflow.md Step 2a is the authoritative fix table.** When the scanner emits a new mismatch string, workflow.md Step 2a's table MUST have a row that maps that string to a concrete fix. Adding a detector without adding the fix row is a half-shipped change and the sub-agent will hit the drift and not know what to do.

---

## §2 — Load-Bearing Invariants

| Invariant | Failure mode it prevents |
|-----------|--------------------------|
| Every `block` gate has a clearing path (skill or auto-fix) | `/continue-roadmap` loop — gate fires, suggested skill no-ops, next scan re-fires (the bug this design log was created for, 2026-04-17) |
| Scanner pre-computes every gate in JSON; workflow transcribes | Workflow logic drift — if the sub-agent re-derives gate logic, it drifts from the scanner; the two must always agree |
| `detect_all_mismatches()` is the sole aggregator for stale-frontmatter drift | New drift classes added as parallel gates instead of rows in one table — fractures the auto-fix pipeline |
| TPR status drift is detected per-section via `Section.tpr_mismatch` | Silent `tpr_findings` blocker (the 2026-04-17 incident: `03.R` complete, 0 open findings, frontmatter says `findings`, gate fires forever) |
| Workflow.md Step 2a row strings match scanner's mismatch strings VERBATIM | Sub-agent sees a mismatch but can't find a matching row → skips the fix → the drift persists |
| Auto-fix is scoped to the focus plan only | Cross-plan churn — editing sibling plans during one focus plan's workflow creates noisy commits and risks blind edits to plans the user hasn't reviewed |
| Cleanup is silent (no `AskUserQuestion`) | User fatigue — mechanical cruft does not need approval, approving 5 stale frontmatter fixes per session destroys the flow |

---

## §3 — File Inventory

| Path | Lines | Role |
|------|-------|------|
| `.claude/skills/continue-roadmap/roadmap_scan.py` | ~2000 | Scanner — parses every plan's sections/subsections/frontmatter, detects drift, pre-computes gates |
| `.claude/skills/continue-roadmap/workflow.md` | ~240 | Sub-agent protocol — transcribes scanner JSON into handoff block, applies auto-fixes |
| `.claude/skills/continue-roadmap/SKILL.md` | ~80 | Caller-facing dispatcher (Agent call + handoff parsing) |
| `scripts/plan_corpus/bug_validators.py` | ~370 | `Superseded by:` marker auto-fix pipeline (Gate 1.6) |
| `scripts/plan_corpus/schema.py` | N/A | Frontmatter schema validator (TPR status/updated field shape) — does NOT detect TPR-vs-findings-checkbox drift (pool-level data not in its scope) |

**Recently changed (2026-04-17):** Added `Section.tpr_mismatch` property and wired it into `detect_all_mismatches()`. Added 3 rows to `workflow.md` Step 2a's fix table covering TPR status drift.

**Recently changed (2026-04-18):** Added `_resolve_dependency_ref()` helper + Gate 1.65 `unmet_dependencies` to `_build_gates()`. Added gate-table row in `workflow.md` Step 3 and corrected the JSON-envelope gate count from a stale "7" to the accurate "10" (the doc was already out of date — pre-existed gates total 9, plus the new one makes 10).

---

## §4 — Lessons from Dogfood / Production Runs

### 2026-04-18 — `depends_on` parsed but ignored

**Symptom.** User ran `/continue-roadmap`, which routed to `plans/spec-conformance/section-10-osc-suite.md` and escalated on `unreviewed_plan` (`reviewed: false`). User selected `/review-plan`. Mid-review, the user noticed §10's frontmatter declares `depends_on: ["03", "08", "plans/effect-cutover/section-01-migrate-mux-consumer.md"]`, and effect-cutover §01 is `in-progress` with all subsections `not-started`. §10 cannot be implemented without §01 — and reviewing §10 NOW is wasted work because §01's design (sink swap, Effect→MuxEvent router, idle-wake channel, `register_host_request_response` activation) drives §10's scope.

**Root cause.** `roadmap_scan.py` parses `depends_on` from each section's frontmatter (`parse_section_file` line ~589), stores it on `Section.depends_on` (line ~630), and *renders it for human display* in `render_focus_section` (line ~1255). It does NOT consult `depends_on` during focus selection (`crawl_workspace` line ~880-888 just picks the first `status != "complete"` section in the focus plan). And `_build_gates()` had no gate for unmet cross-section dependencies. The field was structurally orphaned: parsed, displayed, ignored.

The pre-existing `parse_dependency_graph()` (line ~753) and `classify_blocker_readiness()` (line ~896) machinery operates on a DIFFERENT graph — the one parsed from `00-overview.md` blocker chains — and is only used for rendering blocker descriptions, not for gate evaluation. So even when an in-plan `depends_on` could have been cross-checked, no caller did the lookup.

**Fix.** Two parts:
1. Added `_resolve_dependency_ref(ref, ws)` helper that resolves a `depends_on` entry to a `Section` across the workspace. Path-like refs (containing `/`) match by resolved `Section.path`; bare IDs (e.g., `"03"`) match by `Section.number` scoped to the focus plan first, then any plan.
2. Added Gate 1.65 `unmet_dependencies` to `_build_gates()` — fires `block` when any resolved dep is not `complete`, fires `info` when refs are unresolvable (stale frontmatter). Block payload offers `Switch focus to <plan> §<section>` (re-runs `/continue-roadmap` with the blocker's `<plan_dir> <section>` args), proceed-anyway, pick-different. Placed BEFORE `unreviewed_plan` (1.7) so a `/review-plan` pass is not wasted on a dependent whose scope can shift once its dep lands.
3. Updated `workflow.md` Step 3 gate table with the new row + bumped the JSON-envelope gate count from 7 to 8.

**Verification.** Re-running the scanner on `plans/spec-conformance` now returns `unmet_dependencies.fires == True, severity == "block"` with `unmet[0]` pointing at `effect-cutover §01 (in-progress, 35/292)` and `next_skill_arg` correctly formatted as `<absolute path to plans/effect-cutover> 01`. Running on `plans/effect-cutover` (whose only dep `plans/spec-conformance/section-03-effect-boundary-migration.md` is `complete`) returns `unmet_dependencies.fires == False, severity == "none"` — silent on satisfied deps.

**Why this matters beyond §10.** Out of ~30 sections with `depends_on`, the scanner had no way to detect violations. Any cross-plan dep declaration (`plans/<plan>/section-NN-...md`) was effectively a comment, not a constraint. This bug had been latent since `depends_on` was added; it surfaced when a cross-plan reroute (effect-cutover) had been queued behind an active reroute (spec-conformance) whose §10 declared the dep.

**Prior art surfaced during the fix.** `Section.plan` back-reference (line ~159) was already in place, making the resolver trivial — no new data plumbing needed. The `Workspace.all_plans` dict (line ~344) is the right SSOT for cross-plan lookup; no new aggregation.

### 2026-04-18 — deps-before-ordering (transitive dep-chain walk)

**Symptom.** Follow-up to the earlier 2026-04-18 `depends_on` gate landing. User ran `/continue-roadmap` (no args) mid-`effect-cutover §01` WIP; the scanner surfaced `spec-conformance §10` as focus and escalated with `unmet_dependencies` block (recommending a switch to `effect-cutover §01`) PLUS `unreviewed_plan` (recommending `/review-plan section-10`). Both questions fired with contradictory intents: switching focus AWAY from §10 makes reviewing §10 pointless. The user selected both options, which led the parent to start `/review-plan` on §10 despite also redirecting. User reaction: "Why the fuck do you keep re-reviewing this motherfucking plan over and over" + "Deps come before ordering, I thought that would be obvious."

**Root cause.** The Gate 1.65 `unmet_dependencies` change from earlier in the day made the scanner *detect* dep violations but left *routing* unchanged — focus selection still picked the first incomplete section in the focus plan and asked the user to approve a reroute. When multiple gates fire on a dep-blocked section (typical case: dep-blocked section is also `reviewed: false` because it hasn't been reached yet), the user sees both questions and can accidentally authorize wasted work. The design philosophy said "every `block` gate must have a clearing path" — the clearing path existed (reroute option), but it required a user prompt to take, which collides with unrelated gates on the same section.

The correct mental model the user articulated: **deps are ordering constraints, not informational.** If the ordered-first pick has unmet deps, the ordered-first pick is wrong. Walk the dep chain transitively until you find the section that's actually unblocked. No prompt needed — this is the same category of silent mechanical routing as the stale-frontmatter auto-fixes, not a human-judgment decision.

**Fix.** Added `_follow_unmet_deps_chain(start, ws) -> (terminal, hops)` in `roadmap_scan.py` just below `_resolve_dependency_ref()`. Walks each section's `depends_on` picking the first incomplete dep, recurses into that dep, and continues until reaching a section whose deps are all `complete` (or unresolvable — those stay with the `unmet_dependencies` info gate for surface). Cycle guard via `visited: set[Path]`: revisit stops and returns the revisit as terminal, letting the gate layer surface the broken graph.

Wired into `crawl_workspace` right after the ordered-first pick (lines ~890 ff): after `ws.focus_section` is set, call the walker; if the terminal differs from the start, update `ws.focus_section`, `ws.focus_plan` (deps can cross plan boundaries — the §10 → §01 case does), `ws.focus_section_reason` (describes the hop trail), and `ws.focus_reason` (describes the plan-level redirect). When terminal has no unmet deps, Gate 1.65 silently does not fire → `unreviewed_plan` / `reviewed: false` check now applies to the ACTUAL work item (effect-cutover §01 is `reviewed: true`, so no review gate either).

**Verification.** Three scenarios on the post-fix scanner:
1. `python3 roadmap_scan.py --json` (no args): focus_plan = `effect-cutover`, section = §01, `unmet_dependencies.fires == False`. No gates fire on the redirected target.
2. `python3 roadmap_scan.py --json plans/spec-conformance` (explicit arg into the blocked plan): same result — redirects to effect-cutover §01. Explicit focus still follows deps; the user asking for §10 doesn't override the physical reality that it's blocked.
3. `python3 roadmap_scan.py --json plans/effect-cutover` (explicit arg into a section with satisfied deps): stays put at §01, `unmet_dependencies.fires == False`. The walker is a no-op when the start is unblocked.

**Why this matters beyond §10.** Previously, every `block`-severity `unmet_dependencies` gate was a user-prompt; ~11 workspace-wide declared-but-unmet deps each meant a potential interrupt. Now all of them resolve silently at scan time, and the gate reduces to a safety net for unresolvable refs (stale frontmatter — still `info` severity) and for cycle breaks (if a section routing lands on itself, the gate surfaces it). The user-visible effect: `/continue-roadmap` with no args always lands on a section that is actually workable. No prompt theater around ordering constraints.

**Prior art surfaced during the fix.** The walker uses `_resolve_dependency_ref()` (added earlier 2026-04-18) unchanged — the resolver was already cross-plan aware. `Section.plan` back-reference + `Workspace.all_plans` dict handled the plan-boundary crossing with no new data plumbing. The fix is ~40 lines of new code + 22 lines wiring it into focus selection.

**Design lesson.** The gate-then-prompt pattern is correct for decisions that require human judgment (dirty tree, TPR findings, cross-plan invalidation weight). It is wrong for mechanical routing constraints (ordering, deps). Categorize gates at creation: if the clearing action is always the same deterministic choice, that's auto-fix territory, not a `block`. §1 Design Philosophy now has an implicit corollary: "routing constraints are auto-fixes, not block gates." The `unmet_dependencies` gate stays for safety-net surface (cycles, unresolvable refs), but its `block` severity on resolvable unmet deps is now effectively dead code — the walker resolves them before the gate sees them.

### 2026-04-17 — TPR status loop

**Symptom.** User ran `/continue-roadmap`, it focused on `plans/empty-container-typeck-phase-contract/section-03-bodies-pass-integration.md`, and escalated on `tpr_findings` gate. Running the suggested `/verify-tpr` had nothing to verify (0 open finding checkboxes). User re-ran `/continue-roadmap` → same escalation → loop.

**Root cause.** `section-03` has `third_party_review.status: findings` in its frontmatter, `03.R` subsection `complete`, and all 3 TPR finding checkboxes `[x]`. The scanner's `tpr_fires` logic at `roadmap_scan.py:1669`:

```python
tpr_fires = ws.focus_section.tpr_status == "findings" or bool(open_tpr)
```

fires on the frontmatter flag *or* open findings. Commit `a92ae501` flipped `03.R` status to `complete` via the stale-frontmatter auto-fixer, but the auto-fixer had no rule for `third_party_review.status` — it stayed `findings`. `/verify-tpr` triages finding checkboxes (none open → nothing to do → frontmatter stays `findings`). No path cleared the drift.

**Fix.** Added `Section.tpr_mismatch` property detecting three drift cases:
1. `status == "findings"` AND all parsed findings resolved → should be `resolved`
2. `status == "findings"` AND no findings parsed at all → should be `resolved` (or `none` if `updated` is null)
3. `status == "resolved"` AND any finding still open → should be `findings`

Wired into `detect_all_mismatches()` so Gate 1.5 `stale_frontmatter` surfaces it with the same silent-auto-fix treatment as status-vs-checkbox drift. Added three rows to `workflow.md` Step 2a's fix table so the sub-agent knows how to apply each case.

**Verification.** Re-running the scanner on `section-03` after flipping the frontmatter to `resolved` shows `stale_frontmatter.fires == False` and `tpr_findings.fires == False`. The loop is broken.

**Prior art surfaced during the fix.** `scripts/plan_corpus/schema.py::_validate_tpr_info` already validates `tpr_status` vs the `updated` field, but it operates only on frontmatter — it does not have access to the section's `.R` subsection checkbox state. That's why the detection lives in `roadmap_scan.py` (scanner-level, with per-section checkbox data) and not in `schema.py`.

### 2026-04-18 — `depends_on` parsed but ignored

**Symptom.** User ran `/continue-roadmap`, which routed to `plans/spec-conformance/section-10-osc-suite.md` and escalated on `unreviewed_plan` (`reviewed: false`). User selected `/review-plan`. Mid-review, the user noticed §10's frontmatter declares `depends_on: ["03", "08", "plans/effect-cutover/section-01-migrate-mux-consumer.md"]`, and effect-cutover §01 is `in-progress` with all subsections `not-started`. §10 cannot be implemented without §01 — and reviewing §10 NOW is wasted work because §01's design (sink swap, Effect→MuxEvent router, idle-wake channel, `register_host_request_response` activation) drives §10's scope.

**Root cause.** `roadmap_scan.py` parses `depends_on` from each section's frontmatter (`parse_section_file` line ~589), stores it on `Section.depends_on` (line ~630), and *renders it for human display* in `render_focus_section` (line ~1255). It does NOT consult `depends_on` during focus selection (`crawl_workspace` line ~880-888 just picks the first `status != "complete"` section in the focus plan). And `_build_gates()` had no gate for unmet cross-section dependencies. The field was structurally orphaned: parsed, displayed, ignored.

The pre-existing `parse_dependency_graph()` (line ~753) and `classify_blocker_readiness()` (line ~896) machinery operates on a DIFFERENT graph — the one parsed from `00-overview.md` blocker chains — and is only used for rendering blocker descriptions, not for gate evaluation. So even when an in-plan `depends_on` could have been cross-checked, no caller did the lookup.

**Fix.** Two parts:
1. Added `_resolve_dependency_ref(ref, ws)` helper that resolves a `depends_on` entry to a `Section` across the workspace. Path-like refs (containing `/`) match by resolved `Section.path`; bare IDs (e.g., `"03"`) match by `Section.number` scoped to the focus plan first, then any plan.
2. Added Gate 1.65 `unmet_dependencies` to `_build_gates()` — fires `block` when any resolved dep is not `complete`, fires `info` when refs are unresolvable (stale frontmatter). Block payload offers `Switch focus to <plan> §<section>` (re-runs `/continue-roadmap` with the blocker's `<plan_dir> <section>` args), proceed-anyway, pick-different. Placed BEFORE `unreviewed_plan` (1.7) so a `/review-plan` pass is not wasted on a dependent whose scope can shift once its dep lands.
3. Updated `workflow.md` Step 3 gate table with the new row + bumped the JSON envelope gate count from 7 to 8.

**Verification.** Re-running the scanner on `plans/spec-conformance` now returns `unmet_dependencies.fires == True, severity == "block"` with `unmet[0]` pointing at `effect-cutover §01 (in-progress, 35/292)` and `next_skill_arg` correctly formatted as `<absolute path to plans/effect-cutover> 01`. Running on `plans/effect-cutover` (whose only dep `plans/spec-conformance/section-03-effect-boundary-migration.md` is `complete`) returns `unmet_dependencies.fires == False, severity == "none"` — silent on satisfied deps.

**Why this matters beyond §10.** Out of ~30 sections with `depends_on`, the scanner had no way to detect violations. Any cross-plan dep declaration (`plans/<plan>/section-NN-...md`) was effectively a comment, not a constraint. This bug had been latent since `depends_on` was added; it surfaced when a cross-plan reroute (effect-cutover) had been queued behind an active reroute (spec-conformance) whose §10 declared the dep.

**Prior art surfaced during the fix.** `Section.plan` back-reference (line ~159) was already in place, making the resolver trivial — no new data plumbing needed. The `Workspace.all_plans` dict (line ~344) is the right SSOT for cross-plan lookup; no new aggregation.

### 2026-04-18 — deps-before-ordering (transitive dep-chain walk)

**Symptom.** Follow-up to the earlier 2026-04-18 `depends_on` gate landing. User ran `/continue-roadmap` (no args) mid-`effect-cutover §01` WIP; the scanner surfaced `spec-conformance §10` as focus and escalated with `unmet_dependencies` block (recommending a switch to `effect-cutover §01`) PLUS `unreviewed_plan` (recommending `/review-plan section-10`). Both questions fired with contradictory intents: switching focus AWAY from §10 makes reviewing §10 pointless. The user selected both options, which led the parent to start `/review-plan` on §10 despite also redirecting. User reaction: "Why the fuck do you keep re-reviewing this motherfucking plan over and over" + "Deps come before ordering, I thought that would be obvious."

**Root cause.** The Gate 1.65 `unmet_dependencies` change from earlier in the day made the scanner *detect* dep violations but left *routing* unchanged — focus selection still picked the first incomplete section in the focus plan and asked the user to approve a reroute. When multiple gates fire on a dep-blocked section (typical case: dep-blocked section is also `reviewed: false` because it hasn't been reached yet), the user sees both questions and can accidentally authorize wasted work. The design philosophy said "every `block` gate must have a clearing path" — the clearing path existed (reroute option), but it required a user prompt to take, which collides with unrelated gates on the same section.

The correct mental model the user articulated: **deps are ordering constraints, not informational.** If the ordered-first pick has unmet deps, the ordered-first pick is wrong. Walk the dep chain transitively until you find the section that's actually unblocked. No prompt needed — this is the same category of silent mechanical routing as the stale-frontmatter auto-fixes, not a human-judgment decision.

**Fix.** Added `_follow_unmet_deps_chain(start, ws) -> (terminal, hops)` in `roadmap_scan.py` just below `_resolve_dependency_ref()`. Walks each section's `depends_on` picking the first incomplete dep, recurses into that dep, and continues until reaching a section whose deps are all `complete` (or unresolvable — those stay with the `unmet_dependencies` info gate for surface). Cycle guard via `visited: set[Path]`: revisit stops and returns the revisit as terminal, letting the gate layer surface the broken graph.

Wired into `crawl_workspace` right after the ordered-first pick (lines ~890 ff): after `ws.focus_section` is set, call the walker; if the terminal differs from the start, update `ws.focus_section`, `ws.focus_plan` (deps can cross plan boundaries — the §10 → §01 case does), `ws.focus_section_reason` (describes the hop trail), and `ws.focus_reason` (describes the plan-level redirect). When terminal has no unmet deps, Gate 1.65 silently does not fire → `unreviewed_plan` / `reviewed: false` check now applies to the ACTUAL work item (effect-cutover §01 is `reviewed: true`, so no review gate either).

**Verification.** Three scenarios on the post-fix scanner:
1. `python3 roadmap_scan.py --json` (no args): focus_plan = `effect-cutover`, section = §01, `unmet_dependencies.fires == False`. No gates fire on the redirected target.
2. `python3 roadmap_scan.py --json plans/spec-conformance` (explicit arg into the blocked plan): same result — redirects to effect-cutover §01. Explicit focus still follows deps; the user asking for §10 doesn't override the physical reality that it's blocked.
3. `python3 roadmap_scan.py --json plans/effect-cutover` (explicit arg into a section with satisfied deps): stays put at §01, `unmet_dependencies.fires == False`. The walker is a no-op when the start is unblocked.

**Why this matters beyond §10.** Previously, every `block`-severity `unmet_dependencies` gate was a user-prompt; ~11 workspace-wide declared-but-unmet deps each meant a potential interrupt. Now all of them resolve silently at scan time, and the gate reduces to a safety net for unresolvable refs (stale frontmatter — still `info` severity) and for cycle breaks (if a section routing lands on itself, the gate surfaces it). The user-visible effect: `/continue-roadmap` with no args always lands on a section that is actually workable. No prompt theater around ordering constraints.

**Prior art surfaced during the fix.** The walker uses `_resolve_dependency_ref()` (added earlier 2026-04-18) unchanged — the resolver was already cross-plan aware. `Section.plan` back-reference + `Workspace.all_plans` dict handled the plan-boundary crossing with no new data plumbing. The fix is ~40 lines of new code + 22 lines wiring it into focus selection.

**Design lesson.** The gate-then-prompt pattern is correct for decisions that require human judgment (dirty tree, TPR findings, cross-plan invalidation weight). It is wrong for mechanical routing constraints (ordering, deps). Categorize gates at creation: if the clearing action is always the same deterministic choice, that's auto-fix territory, not a `block`. §1 Design Philosophy now has an implicit corollary: "routing constraints are auto-fixes, not block gates." The `unmet_dependencies` gate stays for safety-net surface (cycles, unresolvable refs), but its `block` severity on resolvable unmet deps is now effectively dead code — the walker resolves them before the gate sees them.

---

## §5 — Regressions To Watch For

- [ ] `tpr_findings` gate firing on a section whose `.R` subsection is `complete` and all finding checkboxes are `[x]`. If seen, the `tpr_mismatch` auto-fix pipeline is broken — either the detector isn't running, or workflow.md Step 2a's row strings have drifted from the scanner's emitted strings.
- [ ] `/continue-roadmap` escalating twice in a row with the same gate and the suggested skill having nothing to do. This is the "loop" pattern — ANY gate that can't be cleared by its suggested skill is a loop waiting to happen.
- [ ] Adding a new gate without adding a clearing path. Every new `block` gate MUST have a corresponding skill invocation, auto-fix, or documented manual step.
- [ ] Adding a scanner detector without adding a matching row in `workflow.md` Step 2a. The sub-agent will see the mismatch string and have no fix to apply.
- [ ] Cross-plan mismatch auto-fix — the sub-agent should NEVER fix mismatches outside the focus plan, even when the scanner reports them. Those reports are informational-only.
- [ ] Focus selection picking a section whose `depends_on` includes a non-`complete` cross-plan ref. If seen, `unmet_dependencies` gate is broken — either `_resolve_dependency_ref()` is not matching the ref shape, or the gate fires but the parent isn't surfacing the reroute option (check `workflow.md` Step 3 row + parent `AskUserQuestion` dispatch).
- [ ] Adding a new frontmatter field with semantic meaning (constraint, dependency, lifecycle marker) without adding the gate that enforces it. The `depends_on` incident (2026-04-18) is the canonical example: parsing + display alone is structural orphaning. EVERY new constrained field needs a gate OR an explicit "this field is informational" comment in §1.
- [ ] `/continue-roadmap` prompting the user to "Switch focus to <dep>" when no human judgment is needed. If seen, the `_follow_unmet_deps_chain()` walker has regressed — either it's not being called from focus selection, or it's not correctly cascading cross-plan. The walker must land on the terminal silently; the gate should never fire `block` on a resolvable dep chain. The previous pattern (prompt + `next_skill: continue-roadmap` reroute) is banned for routing constraints.
- [ ] Focus redirect not updating `focus_plan` when the dep chain crosses a plan boundary. If the walker changes `ws.focus_section` but not `ws.focus_plan`, the rendered "Focus" banner and downstream gate evaluation will contradict. The cross-plan case (spec-conformance §10 → effect-cutover §01) is the canonical smoke test; any change to the wiring should re-run this scenario.

---

## §6 — Improvement Log

### Open items

- [ ] **[p2] Surface load-bearing embedded notes from subsection prose at dispatch time.** Plan subsections frequently embed invariants in prose — e.g., `plans/spec-conformance/section-10-osc-suite.md` §10.0 Files block carries `REGISTRATION SYNC: for every new Handler::iterm2_* method added to crates/vte/src/ansi/handler.rs, a matching delegate arm must be added here`. The current `/continue-roadmap` focus context renders subsection titles + checkbox counts but NOT these inline invariants. Consequence: implementers miss registration-sync / SSOT / CRITICAL-prefixed notes until TPR catches them (observed 2026-04-19 on §10.0 — cost one TPR round + one fix commit `2ba96455`). Candidate fix: scanner greps each subsection's Files / Implementation / Validation blocks for all-caps anchors (`REGISTRATION SYNC`, `CRITICAL`, `MUST`, `SSOT`, `DRIFT`, `VENDORED`) and the focus-context block renders them under a `Load-bearing notes` heading. Heuristic design needs prototyping (false-positive risk: `MUST` appears in prose unrelated to invariants). Not implemented inline during the §10.0 retrospective because the heuristic would touch a non-trivial amount of scanner logic relative to a single avoided TPR round — file for a dedicated tooling session.

### Recently closed

- [x] **2026-04-18** — deps-before-ordering walk: added `_follow_unmet_deps_chain()` + wired it into `crawl_workspace` focus selection. Focus now follows `depends_on` transitively and silently to the terminal unblocked section (including across plan boundaries). `unmet_dependencies` gate `block` severity is now only reachable on cycles or unresolvable refs; ordered-first picks on dep-blocked sections no longer surface prompts. Closes the "keep re-reviewing this motherfucking plan" loop — every invocation now lands on the section the user can actually work on. See §4 dated entry for incident. Commit: pending.
- [x] **2026-04-18** — `depends_on` enforcement: added `_resolve_dependency_ref()` cross-plan resolver + Gate 1.65 `unmet_dependencies` (block on incomplete deps; info on unresolvable refs). Updated `workflow.md` Step 3 gate table + bumped JSON envelope gate count from 7 to 8. Closes the routing bug where the scanner would pick a section whose declared `depends_on` was not satisfied (the spec-conformance §10 → effect-cutover §01 incident — see §4 entry). Commit: pending.
- [x] **2026-04-17** — TPR status drift auto-fix: added `Section.tpr_mismatch` detector (scanner) + 3 rows in `workflow.md` Step 2a (sub-agent fix table). Closes the `/continue-roadmap` → `/verify-tpr` loop on sections where `third_party_review.status: findings` persists after all finding checkboxes are closed. See §4 lesson entry. Commit: pending.
- [x] **2026-04-18** — deps-before-ordering walk: added `_follow_unmet_deps_chain()` + wired it into `crawl_workspace` focus selection. Focus now follows `depends_on` transitively and silently to the terminal unblocked section (including across plan boundaries). `unmet_dependencies` gate `block` severity is now only reachable on cycles or unresolvable refs; ordered-first picks on dep-blocked sections no longer surface prompts. Closes the "keep re-reviewing this motherfucking plan" loop — every invocation now lands on the section the user can actually work on. See §4 dated entry for incident. Commit: pending.
- [x] **2026-04-18** — `depends_on` enforcement: added `_resolve_dependency_ref()` cross-plan resolver + Gate 1.65 `unmet_dependencies` (block on incomplete deps; info on unresolvable refs). Updated `workflow.md` Step 3 gate table + bumped JSON envelope gate count from 7 to 8. Closes the routing bug where the scanner would pick a section whose declared `depends_on` was not satisfied (the spec-conformance §10 → effect-cutover §01 incident — see §4 entry). Commit: pending.

---

## §7 — How To Use This File In Future Sessions

**When to open.** Any time `/continue-roadmap` loops, escalates with no cleared path, or a new drift class surfaces. Also open when modifying `roadmap_scan.py`'s gate logic or `workflow.md`'s auto-fix table — check §2 Load-Bearing Invariants before the edit.

**When to update.** (1) A new drift pattern is found in the wild → add to §4 Lessons + §6 Improvement Log as `- [ ]`. (2) A fix is landed → flip to `- [x]` with commit sha. (3) An invariant from §2 needs to change → add a dated §4 entry explaining the failure mode the old invariant caused, flip the §2 row, and update `workflow.md` to match.

**Grep cheatsheet.** `grep "tpr_mismatch" roadmap_scan.py workflow.md` — verify the detector + fix-table contract is consistent. `grep "Section.mismatch\|tpr_mismatch\|Subsection.mismatch" roadmap_scan.py` — the full aggregator contract for `detect_all_mismatches()`.
