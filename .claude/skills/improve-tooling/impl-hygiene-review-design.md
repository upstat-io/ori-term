# `/impl-hygiene-review` — Design Log

## Purpose + Context

`/impl-hygiene-review` runs a deep implementation-hygiene review against `.claude/rules/impl-hygiene.md` — phase boundaries, data flow, naming, comments, visibility, file organization, lint discipline, algorithmic DRY, and side-logic detection. The review pipeline runs as 7 phases (0–6) with a coordinator that owns judgment and dispatches Sonnet sub-agents for bounded work.

Last significant change: 2026-04-25 — Phase 4 (third-party cross-check) flipped from a Sonnet sub-agent wrapper to inline coordinator execution, mirroring `/review-plan` Step 4 and `/continue-roadmap` Step 6.7.

Canonical files:
- `.claude/skills/impl-hygiene-review/SKILL.md` — coordinator + target modes + dependency map.
- `.claude/skills/impl-hygiene-review/phase-{0..6}-*.md` — per-phase protocols.

## §1 Core Design Philosophy

1. **Coordinator owns judgment.** The coordinator (main context, Opus) parses target mode, decides scope, threads phase outputs, decides per-finding interpretation, and decides whether Phase 6 fires. Phases that ARE judgment-heavy run in main context inline (Phase 4 cross-check) or are dispatched as Opus sub-agents (Phase 3 deep analysis, Phase 6 plan authoring).
2. **Sonnet sub-agents are bounded workers.** Phases 0/1/2/5 are dispatch targets — they produce structured `phase-N.json` output that the coordinator reads and threads forward. Each Sonnet phase is narrow enough that its output schema is fixed.
3. **Phase outputs are JSON files, not return strings.** Each phase writes `{run_id}/phase-N.json` to the orchestrator-owned scratch dir (`mktemp -d -t "impl-hygiene-${repo}-XXXXXXXX"`). Sub-agents return short summaries; the coordinator reads the full JSON only when needed.
4. **Foreground-only Agent dispatch.** Every `Agent({})` call runs foreground — no `run_in_background: true`. The pipeline is sequential by construction; each phase's output informs the next dispatch.
5. **Inline execution for skills that already dispatch sub-agents.** Phase 4 (`/tp-help`/`/tpr-review`) and Phase 6 (`/create-plan`-style work) run via `Skill: <name>` from main context, NOT wrapped in another Sonnet layer. `/tp-help` and `/tpr-review` already dispatch their own codex+gemini reviewer sub-agents; wrapping them again pushes reviewer-response interpretation to the wrong model tier.
6. **Cross-check is NOT optional.** Phase 4 is mandatory in full-project mode and recommended in all other modes. Three-brain review (coordinator + codex + gemini) is the protection against single-brain blind spots.
7. **Phase 6 is conditional, not always-fired.** When findings are few or scope is small, the coordinator emits the Phase 5 report and stops. Phase 6 fires only when findings count or scope exceeds inline-fix capacity.

## §2 Load-Bearing Invariants

| Invariant | Failure mode it prevents |
|---|---|
| Phase 4 runs INLINE in main context (Opus), not wrapped in a Sonnet sub-agent. | Sonnet would interpret reviewer responses, decide which findings to validate, and judge per-reviewer trust — judgment work that belongs in main context. Plus, double-wrapping sub-agent dispatch (`Agent` → `Skill: tp-help` → `Agent` × 2) hits sub-agent context limits and produces "Extra usage required for 1M context" errors when a Sonnet wrapper inherits the parent's compaction state but tries to do new heavy work. (2026-04-25 incident — initial impl wrapped Phase 4 in Sonnet; user surfaced the regression mid-review.) |
| Phase 3 (deep analysis) is Opus. | Multi-lens analysis requires reading many files, comparing function bodies for algorithmic DRY, tracing data flow end-to-end, and producing finding-level judgment. Sonnet at this scope produces shallow reads and weak findings. |
| Phase 6 (plan authoring) is Opus. | Authoring a plan from findings requires section-level architectural judgment — what to group, where to split, what depends on what. Sonnet drafts a flat list. |
| Coordinator owns the `mktemp -d` scratch dir. | Mirrors `/tpr-review` §8 invariant I1. Per-invocation prefix prevents cross-session collision when parallel Claude sessions run in different repos. |
| Each phase writes its own `{run_id}/phase-N.json`. | Orchestrator-readable handoff format — sub-agent return strings get truncated under load; on-disk JSON survives. |
| Phase outputs are short summaries; full payloads on disk. | Keeps coordinator main-context tokens under ~15K per invocation. The coordinator reads the full JSON only when needed (e.g., Phase 5 reads Phase 3 findings to format the report). |
| Findings file inline into the owning plan section's `### Findings` subsection — NEVER via `/add-bug`. | The bug tracker is for correctness defects. Hygiene findings are plan-scope close-out gates owned by the plan's completion checklist. Routing through `/add-bug` decouples the finding from the plan that should resolve it. |
| Foreground-only Agent dispatch. | The pipeline is sequential; each phase's output informs the next. Background dispatch would drift the coordinator's view of "which phase has run" out of sync with reality. |

## §3 File Inventory

| File | Lines | Role |
|---|---|---|
| `SKILL.md` | ~210 | Coordinator: target-mode parsing, dependency map, phase dispatch sequence, escalation contract. |
| `phase-0-static-analysis.md` | — | Sonnet: runs `hygiene-lint.py`, `enum-drift.py`, `fn-rename.py`, `plan-annotations.py`. |
| `phase-1-context.md` | — | Sonnet: loads `CLAUDE.md` + `.claude/rules/*.md` + active plan. |
| `phase-2-landscape.md` | — | Sonnet: symbol + call-graph inventory. No intelligence-graph available; uses Grep/Glob. |
| `phase-3-analysis.md` | — | **Opus**: multi-lens deep analysis, finding generation. |
| `phase-4-cross-check.md` | ~70 | **INLINE (main context)**: dispatches `/tp-help` or `/tpr-review` via Skill tool. |
| `phase-5-present.md` | — | Sonnet: format findings into report template. |
| `phase-6-plan.md` | — | **Opus, CONDITIONAL**: author a plan when scope warrants it. |

Tooling alongside (unchanged): `hygiene-lint.py/.sh`, `enum-drift.py/.sh`, `fn-rename.py/.sh`, `hygiene-fix.py/.sh`, `plan-annotations.py/.sh`.

## §4 Lessons from Dogfood / Production Runs

### 2026-04-25 — Phase 4 wrapper regression (caught mid-review)

A `/impl-hygiene-review` invocation against the `gpu-prepare-html-algorithmic-dry` Section 01 close-out triggered the Phase 4 Sonnet sub-agent dispatch and immediately failed with: `API Error: Extra usage is required for 1M context`. Diagnosis: the Sonnet sub-agent inherits the parent context's compaction footprint AND was being asked to do its own heavy work (read CLAUDE.md + all rule files + dispatch `/tp-help` which itself fans out to two reviewers). The combined token budget exceeded Sonnet's standard context.

Root cause: SKILL.md's Phase 4 specification used `Agent({model: "sonnet"})` instead of inline `Skill: tp-help`. The wrapper served no purpose — `/tp-help` already dispatches its own codex+gemini reviewer sub-agents. Wrapping it in another Sonnet layer:
1. Burns context on a redundant dispatch hop.
2. Pushes reviewer-response interpretation to the wrong model tier (Sonnet, not Opus).
3. Loses fidelity — Sonnet may "summarize" findings before the coordinator sees them.

Fix: Phase 4 now runs inline in the coordinator's main context. The coordinator reads `phase-4-cross-check.md` directly, invokes `Skill: tp-help <focused question>`, and writes `{run_id}/phase-4.json` itself. Mirrors `/review-plan` Step 4 and `/continue-roadmap` Step 6.7.

Why it took until 2026-04-25 to surface: the skill was rarely invoked in modes that hit Phase 4 (Phase 4 is conditional on Phase 3 producing findings). The Section 01 review was the first real-use Phase 4 invocation post-coordinator-pipeline-rewrite.

## §5 Regressions To Watch For

- [ ] **Phase 4 re-wrapped in Sonnet sub-agent.** If a future edit moves Phase 4 back to `Agent({model: "sonnet", ...})` dispatch, the 2026-04-25 incident recurs. Phase 4 MUST stay inline.
- [ ] **Phase 3 downgraded to Sonnet.** Phase 3 is the judgment-heavy phase. Sonnet at that scope produces shallow findings. If you see "let's save tokens by Sonnet-ing Phase 3" in a future edit, REJECT.
- [ ] **Phase 6 always-fired instead of conditional.** Always-firing Phase 6 wastes Opus dispatches on review runs that produced 0–2 findings. The condition is `findings_warrant_a_plan` — keep it.
- [ ] **Coordinator reading full Phase 3 finding payloads inline.** The coordinator reads `phase-N.json` summaries, not full payloads. Reading the full Phase 3 finding payload inline blows past the ~15K-token main-context budget.
- [ ] **Sub-agents dispatched with `run_in_background: true`.** The pipeline is sequential. Background dispatch drifts coordinator state.
- [ ] **Findings routed through `/add-bug`.** Hygiene findings are plan-scope close-out gates. Routing through the bug tracker decouples them from the plan that owns the resolution.

## §6 Improvement Log

### Open items

(none)

### Recently closed

- [x] **2026-04-25 — Flip Phase 4 from Sonnet sub-agent wrapper to inline coordinator execution.** SKILL.md and `phase-4-cross-check.md` both updated. Mirrors `/review-plan` Step 4 / `/continue-roadmap` Step 6.7 inline-Skill pattern. Resolves the "Extra usage required for 1M context" failure mode caught during gpu-prepare-html-algorithmic-dry Section 01 close-out.

## §7 How To Use This File In Future Sessions

Open this file when:
- A `/impl-hygiene-review` invocation produces unexpected behavior — check §5 Regressions To Watch For first.
- You're about to edit `SKILL.md` or any `phase-N-*.md` — re-read §1 Design Philosophy and §2 Load-Bearing Invariants before changing model assignments, dispatch shapes, or phase ordering.
- You hit a context-limit error on a sub-agent — check whether the phase wraps a skill that already dispatches its own sub-agents (§4 2026-04-25 lesson).

Update this file when:
- A `/improve-tooling` retrospective surfaces a finding against this skill — add a `- [ ]` open item under §6.
- An invariant in §2 needs to flip — add a dated §4 entry explaining the new failure mode the old invariant caused, flip the §2 row, AND update the corresponding `SKILL.md` / `phase-N-*.md` content to match.
- A new phase is added or removed — update §3 File Inventory and §1 Design Philosophy.
