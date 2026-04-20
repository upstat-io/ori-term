# scripts/plan-cleanup.py — Design Log

## Purpose + Context

`scripts/plan-cleanup.py` applies scanner-detected plan-doc auto-fixes (stale frontmatter status, stale plan annotations, bug-marker drift). Invoked from `/commit-push` Step 4 alongside `fmt-all.sh`, before `git add -A`. Created 2026-04-19 when plan-doc cleanup moved out of `/continue-roadmap`.

## §1 Core Design Philosophy

1. **Idempotent.** Second run on a clean tree is a no-op.
2. **Silent when clean.** No output when nothing to fix. Summary line only when fixes applied.
3. **Always exit 0.** Cleanup failures never block commits — cleanup is best-effort.
4. **No detection logic of its own.** Consumes `roadmap_scan.py --json` output; applies the fixes the scanner flagged. If the scanner is unavailable, exit silently.
5. **Plan-docs only.** Edits `plans/**/*.md` and invokes `plan-annotations.sh`. Never touches code, tests, config, or any file outside `plans/`.

## §2 Load-Bearing Invariants

| Invariant | Failure mode it prevents |
|---|---|
| Exit 0 on every outcome (scanner unavailable, file missing, edit failure, etc.). | Cleanup blocking `/commit-push` would break the user's intent to commit their own work. |
| Idempotent on each sub-fix (frontmatter, annotations, bug markers). | Re-runs across `/commit-push` invocations would pile up spurious edits; a tight loop would never terminate. |
| Only fixes mismatches flagged for the focus plan (`focus_plan_mismatches`). | Cross-plan edits during someone else's `/commit-push` cause blame confusion. |
| Runs BEFORE `git add -A` in `/commit-push` Step 4. | Running after staging creates post-commit dirty tree (same ordering reason as `fmt-all.sh`). |
| Detection lives in `roadmap_scan.py`, never duplicated here. | Parallel detection paths drift silently. Scanner is SSOT for "what is stale". |

## §3 File Inventory

| File | Lines | Role |
|---|---|---|
| `scripts/plan-cleanup.py` | 155 | Full implementation. Entry: `main()`. Fix functions: `fix_frontmatter` / `run_plan_annotations_cleanup` / `apply_bug_marker_edit`. |

Consumers: `.claude/skills/commit-push/workflow.md` §Step 4.

## §4 Lessons

### 2026-04-19 — Created alongside /continue-roadmap simplification

Previously: `/continue-roadmap`'s Sonnet sub-agent applied cleanup inline (workflow.md Steps 2a/2b/2c), then invoked `/commit-push` separately. Two commits per cleanup pass — one for user work, one for auto-fix. User flagged this as the "dirty-tree loop" and asked for the cleanup to move into `/commit-push` at the same slot as `fmt-all.sh`. Created `plan-cleanup.py` as that slot; `/continue-roadmap` Step 2 was deleted and the auto-fix gate severities dropped from `auto-fix` to `info`.

## §5 Regressions To Watch For

- [ ] Script starts its own detection (re-scans, re-parses plan files). Scanner JSON is the only input.
- [ ] Script exits non-zero on cleanup failure → blocks `/commit-push`. Must always exit 0.
- [ ] Script edits files outside `plans/**/*.md` or invokes tools beyond `plan-annotations.sh`. Out of scope.
- [ ] Step 4 invocation order in `/commit-push` reverses (plan-cleanup before fmt-all.sh, or after staging). Order is load-bearing.
- [ ] Cross-plan mismatches get auto-fixed. Only `focus_plan_mismatches` are in scope.
- [ ] Cleanup re-appears in `/continue-roadmap` as "defense in depth". Single source is `/commit-push` Step 4.

## §6 Improvement Log

### Open items

- [ ] [p3] Annotations summary line reports `1 annotations` even when `plan-annotations.sh --cleanup-only` found nothing to clean (the script always "ran" if the gate fired). Minor noise. Fix: diff file tree before/after the invocation and report based on actual changes, not invocation count.

### Recently closed

- [x] 2026-04-19 — **Initial implementation.** 155 lines Python. Handles stale_frontmatter (rewrite `status:` line in YAML, top-level or subsection entry), stale_plan_annotations (invoke `plan-annotations.sh --cleanup-only --plan <focus>`), bug_marker_drift (insert `Superseded by:` line after bug entry header, preserving indentation, idempotent). Wired into `/commit-push workflow.md §Step 4`. Tested on itself: picked up the stale §08.2 frontmatter (complete → in-progress with 4 unchecked matrix cells). Commit: `db747ab7`.

## §7 How To Use This File In Future Sessions

Open if `plan-cleanup.py` misbehaves during `/commit-push`. §2 invariants define what's load-bearing. §1 item 4 ("no detection logic of its own") is the main architectural guardrail — if you are tempted to scan independently here, add the detector to `roadmap_scan.py` instead. If `/commit-push` users report that "cleanup broke my commit," check §2 row 1 first.
