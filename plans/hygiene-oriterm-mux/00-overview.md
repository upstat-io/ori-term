---
plan: "hygiene-oriterm-mux"
title: "oriterm_mux Implementation Hygiene: Exhaustive Fix Plan"
status: complete
references:
  - ".claude/rules/impl-hygiene.md"
  - ".claude/rules/code-hygiene.md"
---

# oriterm_mux Implementation Hygiene: Exhaustive Fix Plan

## Mission

Fix all 36 implementation hygiene findings in `oriterm_mux` — semantic drift between event vocabularies, missing data propagation, wasteful allocations in server hot paths, over-exposed internal APIs, and style/documentation violations — so the crate's implementation faithfully realizes its architecture with tight joints and no leaks.

## Architecture

```
oriterm_mux crate boundary map (findings by boundary):

  MuxEvent <──> MuxNotification        [findings 1-7]
    mux_event/mod.rs
    in_process/event_pump.rs
    server/notify/mod.rs

  Pane <──> PTY <──> ShellIntegration  [findings 8-17]
    pane/mod.rs
    pty/spawn.rs, pty/event_loop/mod.rs, pty/signal.rs
    shell_integration/mod.rs, shell_integration/inject.rs

  Registry <──> InProcessMux           [findings 18-22]
    in_process/event_pump.rs, in_process/mod.rs
    registry/mod.rs, lib.rs

  Protocol <──> Server <──> Backend    [findings 23-33]
    protocol/mod.rs, protocol/snapshot.rs, protocol/msg_type.rs
    server/mod.rs, server/dispatch/mod.rs, server/push/mod.rs
    server/connection.rs, server/snapshot.rs, server/notify/mod.rs
    backend/client/transport/mod.rs, backend/client/transport/reader.rs
    bin/oriterm_mux.rs

  ID Types (cross-cutting)             [findings 34-36]
    id/mod.rs
```

## Design Principles

1. **Vocabulary agreement.** When two modules communicate via shared types (events, notifications, IDs), the input and output vocabularies must agree — no silent discards, no semantic overloading. Motivated by findings 1-2 where three distinct MuxEvent variants collapse into one MuxNotification variant.

2. **No leaked internals.** Public API surface should be intentional, not accidental. `pub` items used only within the crate violate the minimal-surface principle and create fragile coupling if downstream code starts depending on them. Motivated by 10 EXPOSURE findings.

3. **No allocation in server hot paths.** The server event loop runs per-frame and per-pane. Allocating fresh `Vec`s, format strings, or snapshot buffers on each cycle is wasteful when scratch buffers exist. Motivated by findings 23-25.

## Section Dependency Graph

```
  01-drift ──────────┐
  02-gaps ────────────┤
  03-waste ───────────┼──> 06-verification
  04-exposure ────────┤
  05-notes ───────────┘
```

Sections 01 through 05 are fully independent — they touch different files and concerns. They can be worked in any order or in parallel. Section 06 (verification) depends on all prior sections being complete.

**Cross-section interactions (must be co-implemented):**
- **Section 01 (DRIFT) + Section 02 (GAP)**: Both modify `MuxNotification` — Section 01 renames `PaneTitleChanged` to `PaneMetadataChanged`, Section 02 adds `exit_code` to `PaneClosed`. Apply together to avoid editing the same enum twice. The `Debug` impl, `notification_to_pdu`, tests, and `oriterm` consumers all need updating for both changes.
- **Section 01 (DRIFT) + Section 04 (EXPOSURE)**: Finding 5 (MuxEvent/MuxEventProxy visibility) is in EXPOSURE but the types are also modified in DRIFT finding 1. Apply DRIFT changes first, then EXPOSURE visibility narrowing, to avoid double-editing.
- **Section 02 (GAP) + Section 05 (NOTE)**: Finding 32 (snapshot `build_snapshot` visibility) in NOTES depends on whether Section 03.2 eliminates its last call site. Apply Section 03 first, then decide whether to remove or narrow `build_snapshot`.

## Implementation Sequence

```
Phase 1 — Semantic fixes (highest priority)
  +-- section-01: DRIFT fixes (event vocabulary alignment, env var dedup)
  +-- section-02: GAP fixes (exit code propagation, dead code, overflow)

Phase 2 — Efficiency fixes
  +-- section-03: WASTE fixes (allocation reuse, log level, trait delegation)

Phase 3 — Surface cleanup
  +-- section-04: EXPOSURE fixes (visibility narrowing)
  +-- section-05: NOTE fixes (docs, banners, style)

Phase 4 — Verification
  +-- section-06: build, clippy, test, fmt
  Gate: `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` all green
```

**Why this order:**
- Phase 1 fixes semantic correctness issues that affect program behavior.
- Phase 2 reduces unnecessary work in hot paths.
- Phase 3 is safe mechanical cleanup — no behavioral changes.
- Phase 4 proves nothing regressed.

## Estimated Effort

| Section | Findings | Est. Lines Changed | Complexity |
|---------|----------|-------------------|------------|
| 01 DRIFT | 4 | ~120 | Medium |
| 02 GAP | 4 | ~80 | Medium |
| 03 WASTE | 5 | ~60 | Low-Medium |
| 04 EXPOSURE | 10 | ~30 | Low |
| 05 NOTE | 13 | ~80 | Low |
| 06 Verification | — | 0 | Low |
| **Total** | **36** | **~370** | |

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | DRIFT Fixes | `section-01-drift.md` | Complete |
| 02 | GAP Fixes | `section-02-gaps.md` | Complete |
| 03 | WASTE Fixes | `section-03-waste.md` | Complete |
| 04 | EXPOSURE Fixes | `section-04-exposure.md` | Complete |
| 05 | NOTE Fixes | `section-05-notes.md` | Complete |
| 06 | Verification | `section-06-verification.md` | Complete |
