# `/verify-tpr` — Design Notes and Improvement Log

**Purpose.** Institutional memory for `/verify-tpr` — the TPR-findings triage orchestrator invoked by `/continue-roadmap` when a section's `third_party_review.status = findings`. Captures design philosophy, load-bearing invariants, and running log of improvements.

**Context.** `/verify-tpr` is the main-context triage path for a plan section's `## {NN}.R Third Party Review Findings` block. It reads each unchecked `- [ ]` finding, validates it against actual code / spec / tests, and resolves it as ACCEPT-and-fix / ACCEPT-with-blocker-anchor / REJECT-with-evidence. Related design log: `tpr-review-design.md` (sibling review-family skill).

**When to update.** Any time you encounter a bug, surprise, or improvement idea while running `/verify-tpr` or its caller `/continue-roadmap`. Add a `- [ ]` item under §6 Improvement Log. Real-use design-violations → §5 Regressions To Watch For.

---

## §1 — Core Design Philosophy (KEEP THIS)

1. **Main-context execution.** `/verify-tpr` runs in the caller's conversation (typically `/continue-roadmap`). No sub-agent dispatch. Each finding is read, validated, and resolved visibly. Triage decisions would be invisible in a sub-agent.
2. **Inlined grounding, no `@`-includes.** The skill uses `scripts/intel-query.sh` directly for bounded blast-radius calibration. It does NOT `@`-include `compose-intel-summary.md`. This matches the 2026-04-16 review-family rewrite.
3. **Bounded graph usage.** At most 5 intel queries per triage run. The graph calibrates accept/reject decisions — it never authorizes them.
4. **Verify-before-resolve.** Every finding is read against actual code before marking resolved. Resolutions cite a specific `file:line`, test name, or spec clause — never "I believe this was fixed" or "the graph shows X".
5. **Zero-deferral on ACCEPT.** Accepted findings are either fixed now or anchored to a concrete `- [ ]` task with a real verifiable blocker. "Tracked as future work" is banned.
6. **REJECT requires evidence, not rationalization.** The banned-phrase list (pre-existing, out of scope, architectural, conservative, future improvement, not our problem) is load-bearing. REJECT is only valid when the finding is factually wrong about the code.

## §2 — Load-Bearing Invariants

| # | Invariant | Why (which failure mode it prevents) |
|---|-----------|--------------------------------------|
| V1 | No `@`-include of `compose-intel-summary.md` (or any SSOT); grounding inlined via bullets in Step 2.5 | Matches 2026-04-16 review-family rewrite; avoids auto-loading 1000+ lines of SSOT + rules into the caller's context on every invocation. Parent may have already `@`-included rules; double-load is pure context bloat. |
| V2 | `scripts/intel-query.sh status` availability probe before any graph queries; skip silently on failure | Graph degrades gracefully — triage continues without calibration. Hard-failing on graph outage would block TPR resolution on infrastructure the skill doesn't own. |
| V3 | 5-query cap per triage run | Prevents query exhaustion on findings-heavy sections. Callers count is a calibration input, not a per-finding lookup. |
| V4 | Graph results are DISCOVERY, not authority | Caller counts inform scrutiny depth; they never serve as resolution evidence. All resolutions cite actual code / spec / tests per Step 3 §5. |
| V5 | REJECT never uses banned-phrase rationalization | Findings routed through "pre-existing", "out of scope", "architectural", "conservative", "future improvement", "not our problem" are soft-accepts disguised as rejects; they defeat the skill's purpose. |
| V6 | ACCEPT-with-blocker requires a verifiable dependency anchor | "Blocked by future work" without a specific `- [ ]` target is deferral. The blocker must be something a future session can resolve deterministically. |
| V7 | Section `status` stays `in-progress` while `third_party_review.status: findings` | A section cannot be `complete` while unchecked TPR items exist. Status transition to `resolved` happens only after accepted tasks are completed and revalidated, not at triage time. |

## §3 — File Inventory (canonical)

| Path | Lines (~) | Role |
|------|-----------|------|
| `.claude/skills/verify-tpr/SKILL.md` | 160 | Skill contract (5 workflow steps + status rules + quality standard) |

Cross-skill dependencies:

- Invoked by `/continue-roadmap` Step 3 when the scanner's `tpr_findings` gate fires with `severity: block`.
- Uses `scripts/intel-query.sh` for blast-radius calibration (inline protocol in Step 2.5 — no `@`-include).
- Does NOT delegate to `/tpr-review` or `/tp-help` — triage is main-context work.

## §4 — Lessons from Dogfood / Production Runs

### 2026-04-19 — `@`-include retrofit missed in 2026-04-16 rewrite

Surfaced via `/improve-tooling` during a `/continue-roadmap` run on `empty-container-typeck-phase-contract/section-08`. The scanner fired its `tpr_findings` block gate, the user answered "run /verify-tpr", and the skill's `@.claude/skills/query-intel/compose-intel-summary.md` directive auto-loaded 301 lines of SSOT + 180 lines of `intelligence.md` + 500+ lines of `missions.md` into the caller's context before triage could begin.

Root cause: the 2026-04-16 rewrite enumerated review-family consumers as `/tpr-review` + `/tp-help` + `/review-work` (skill) + `/review-plan` when dropping `@`-includes. `/verify-tpr` belongs to the same family (review-family skill invoked by an orchestrator) but was not in the enumeration. The SSOT's "Current consumers (21)" still counted it; the "Dropped consumers" block did not.

Fix: removed the `@`-include from Step 2.5, inlined the availability-check + callers-query protocol as prescriptive bullets. Updated SSOT to decrement count 21 → 20 and add `/verify-tpr` to the dropped consumers enumeration. Created this design log to prevent a recurrence — future additions to the review-family rewrite enumeration now have an owning file that must be updated in the same commit.

## §5 — Regressions To Watch For

- [ ] Any new `@.claude/` reference inside `.claude/skills/verify-tpr/SKILL.md` — regresses V1.
- [ ] Intel graph queries without the `status` availability probe — regresses V2; hard-fails triage when the graph is down.
- [ ] More than 5 intel queries per triage run — regresses V3.
- [ ] Resolutions citing "the graph shows X" or a caller-count as evidence — regresses V4; evidence must be actual code / spec / tests.
- [ ] REJECT entries using "pre-existing" / "out of scope" / "architectural" / "conservative" / "future improvement" / "not our problem" — regresses V5.
- [ ] ACCEPT-with-blocker where the blocker is "future work" / "when we refactor X" without a specific `- [ ]` anchor — regresses V6.
- [ ] Section frontmatter flipped to `status: complete` while `third_party_review.status: findings` — regresses V7.

## §6 — Improvement Log

### Open items

(none)

### Recently closed

- [x] 2026-04-19 — Drop `@`-include of `compose-intel-summary.md` from `verify-tpr/SKILL.md`; inline Step 2.5 blast-radius protocol as prescriptive bullets. Updated SSOT: count 21 → 20, `/verify-tpr` moved to Dropped consumers, Step F TPR/verification entry removed. [p1, surfaced by user via `/improve-tooling` during a `/continue-roadmap` run on `empty-container-typeck-phase-contract/section-08`.] Commit: pending.

## §7 — How To Use This File In Future Sessions

Open this file when:

- `/verify-tpr` misbehaves (confusing output, context bloat, wrong triage routing).
- Modifying `verify-tpr/SKILL.md` — re-read §1 and §2 before editing to preserve invariants.
- A `/tpr-review` or `/impl-hygiene-review` finding calls out `/verify-tpr` behavior.
- Running an `/improve-tooling` retrospective after a `/verify-tpr` invocation.

Update this file when:

- Any invariant in §2 needs to change — add a dated §4 entry explaining the new failure mode the old invariant caused, flip the §2 row, and update the SKILL.md to match.
- A regression from §5 is caught in the wild — add a `- [ ]` under §6 Open and bump priority if it recurs.
- A production-run finding (TPR / hygiene / retrospective) surfaces a tooling gap — add to §6 regardless of whether you can fix it immediately.
- Every `/improve-tooling` session on the skill — add a `- [x]` to §6 Recently closed with date + description + commit sha.
