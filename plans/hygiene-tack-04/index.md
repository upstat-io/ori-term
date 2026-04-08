---
plan: "hygiene-tack-04"
title: "Hygiene Cleanup — tack-conformance Section 04 slice"
status: in-progress
disposable: true
---

# Hygiene Tack 04 Index

> **Disposable cleanup plan.** Tracks 24 hygiene findings (8 Major + 16 Minor) from
> the impl-hygiene review of the Section 04 tack-conformance scenario framework slice
> (`ce305091..efec3818`). The 4 Critical SSOT/algorithmic LEAKs from that review were
> fixed inline in commit `0b0806f2`. Once the items in this plan are complete, the
> entire `plans/hygiene-tack-04/` directory should be deleted via the cleanup step in
> `section-01-cleanup.md`.

## How to Use

1. Pick the highest-severity unchecked item from `section-01-cleanup.md`
2. Fix it via the standard `/fix-bug` workflow (or inline if trivial)
3. Run `./test-all.sh` + `./clippy-all.sh` to verify no regressions
4. Mark the item `[x]` and commit
5. Repeat until clean, then delete this plan

## Sections

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Cleanup | `section-01-cleanup.md` | Not Started |

## Keyword Clusters

```
hygiene, cleanup, disposable, tack-conformance, oriterm_test_support
DRY, scattered-knowledge, BLOAT, EXPOSURE, magic-number, named-constant
test_helpers extraction, spawn_quit_on_keystroke duplicate, spawn_silent_long_lived duplicate
panic_payload_to_string downcast helper, shell_command CommandBuilder factory
parse_modes_screen header re-derivation, default_parser SSOT
LiveSession Drop guard, must_use cleanup contract, runner/mod.rs polish
session/sync impl block ordering, feed_and_flush placement
spec.rs missing module doc, modes.rs missing must_use
grid_text double-allocation in poll loop
named constants: POST_SEND_QUIESCE_MS, POLL_DRAIN_BLOCK_MS, QUIT_PHASE2_TIMEOUT_MS
INTER_Q_DRAIN_MS plan/comment/impl drift
send_raw silent error swallow, reader thread silent error swallow
TackNavigator catch_unwind grep gate already tightened (TPR-04-007)
Section 07 LiveSession::session pub field exposure
```
