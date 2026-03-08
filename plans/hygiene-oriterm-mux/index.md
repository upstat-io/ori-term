# oriterm_mux Hygiene Fixes — Index

> **Maintenance Notice:** Update this index when adding/modifying sections.

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

## Keyword Clusters by Section

### Section 01: DRIFT Fixes
**File:** `section-01-drift.md` | **Status:** Complete

```
drift, semantic drift, vocabulary mismatch, diverged
MuxEvent, MuxNotification, PaneTitleChanged, PaneIconChanged, PaneCwdChanged
PaneMetadataChanged, event pump, event_pump.rs, mux_event/mod.rs
TERM_PROGRAM, env var, set_common_env, build_command, shell_integration
WSLENV, inject_wsl, compute_wslenv, build_wslenv, dedup
spawn.rs, inject.rs, shell_integration/mod.rs
```

---

### Section 02: GAP Fixes
**File:** `section-02-gaps.md` | **Status:** Complete

```
gap, missing, discarded, dead code, overflow
exit_code, PaneExited, PaneClosed, notification_to_pdu
PaneOutput, server/notify, drain_mux_events
trace log, panic, backwards slice, unprocessed, buf
IdAllocator, alloc, u64 overflow, counter, wrap
event_loop/mod.rs, id/mod.rs
```

---

### Section 03: WASTE Fixes
**File:** `section-03-waste.md` | **Status:** Complete

```
waste, allocation, hot path, reuse, buffer
log::warn, synchronized output, trace, downgrade
SnapshotCache, build_snapshot, subscribe, snapshot_cache
Vec allocation, scratch_panes, scratch, per-cycle
trailing edge, push, flush, PushContext
from_raw, raw, MuxId, duplicate, inherent, trait impl, delegate
server/dispatch/mod.rs, server/mod.rs, server/push/mod.rs, id/mod.rs
```

---

### Section 04: EXPOSURE Fixes
**File:** `section-04-exposure.md` | **Status:** Complete

```
exposure, visibility, pub, pub(crate), pub(super), dead accessor
MuxEvent, MuxEventProxy, build_command, default_shell, compute_wslenv
pane_registry, should_push, push_snapshot_to_subscribers, defer_all_subscribers
ClientConnection, FrameHeader, MsgType, MuxId, IdAllocator
registry module, re-export, lib.rs
mux_event/mod.rs, pty/spawn.rs, in_process/event_pump.rs
server/push/mod.rs, server/mod.rs, protocol/mod.rs, server/connection.rs, id/mod.rs
```

---

### Section 05: NOTE Fixes
**File:** `section-05-notes.md` | **Status:** Complete

```
note, doc comment, module doc, banners, decorative
import grouping, terminology, consistency, style
TermHandle, type alias, has_explicit_title, dual ownership
allow dead_code, InitState, signal.rs, inject_wsl
unwrap_or_else, match, normalize, oriterm_mux.rs
WireColor, feature flag, snapshot.rs, build_snapshot
unsafe_code, file-level allow, transport, reader.rs
mux_event/tests.rs, pane/mod.rs, pty/signal.rs
in_process/event_pump.rs, in_process/mod.rs, lib.rs
backend/client/transport/mod.rs, bin/oriterm_mux.rs
protocol/snapshot.rs, server/snapshot.rs
```

---

### Section 06: Verification
**File:** `section-06-verification.md` | **Status:** Complete

```
verify, verification, test, clippy, build, cleanup
build-all.sh, clippy-all.sh, test-all.sh, fmt-all.sh
regression, green, no warnings, cargo test
```

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | DRIFT Fixes | `section-01-drift.md` |
| 02 | GAP Fixes | `section-02-gaps.md` |
| 03 | WASTE Fixes | `section-03-waste.md` |
| 04 | EXPOSURE Fixes | `section-04-exposure.md` |
| 05 | NOTE Fixes | `section-05-notes.md` |
| 06 | Verification | `section-06-verification.md` |
