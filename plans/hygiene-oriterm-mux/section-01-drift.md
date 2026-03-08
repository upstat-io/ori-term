---
section: "01"
title: "DRIFT Fixes"
status: complete
goal: "Eliminate semantic drift between event/notification vocabularies and environment variable setup paths"
depends_on: []
sections:
  - id: "01.1"
    title: "MuxEvent/MuxNotification Vocabulary Alignment"
    status: complete
  - id: "01.2"
    title: "PaneTitleChanged Overloading"
    status: complete
  - id: "01.3"
    title: "TERM_PROGRAM Env Var Dedup"
    status: complete
  - id: "01.4"
    title: "WSLENV Construction Unification"
    status: complete
  - id: "01.5"
    title: "Completion Checklist"
    status: complete
---

# Section 01: DRIFT Fixes

**Status:** Complete
**Goal:** All event/notification type pairs have documented, intentional vocabulary mappings. Environment variable setup has a single source of truth with no duplicate assignments.

**Context:** Semantic drift is the highest-priority hygiene category because it causes silent data loss and makes the codebase misleading. When `MuxEvent` has three variants that collapse into one `MuxNotification`, or when `TERM_PROGRAM` is set in two places with different scope, the implementation disagrees with itself about what the code means.

---

## 01.1 MuxEvent/MuxNotification Vocabulary Alignment

**File(s):** `oriterm_mux/src/mux_event/mod.rs`

**Finding 1:** `MuxEvent` and `MuxNotification` have partially overlapping variant sets with no shared source of truth. `PaneIconChanged` and `PaneCwdChanged` exist as `MuxEvent` variants but have no `MuxNotification` counterparts. This is undocumented — it is unclear whether the omission is intentional or accidental.

- [x] Add doc comment on `MuxNotification` enum explaining which `MuxEvent` variants are intentionally not forwarded and why
  - List each `MuxEvent` variant and its notification mapping (or explicit "not forwarded" designation)
  - Explain the design intent: `MuxNotification` is the downstream-facing subset of `MuxEvent`

- [x] Consider adding a compile-time exhaustiveness check in the event pump
  - The `match` in `event_pump.rs` already covers all variants, but a doc comment on `MuxEvent` should note that adding a variant requires updating the event pump match

---

## 01.2 PaneTitleChanged Overloading

**File(s):** `oriterm_mux/src/in_process/event_pump.rs`

**Finding 2:** `MuxNotification::PaneTitleChanged` is overloaded for three distinct `MuxEvent` variants (`PaneTitleChanged`, `PaneIconChanged`, `PaneCwdChanged`). The input vocabulary (3 events) disagrees with the output vocabulary (1 notification). Downstream consumers cannot distinguish which metadata actually changed.

**Fix applied:** Option (b) — renamed to `PaneMetadataChanged`. All three events trigger the same downstream behavior (re-read pane metadata).

- [x] Rename `MuxNotification::PaneTitleChanged` to `MuxNotification::PaneMetadataChanged`
- [x] Update the `Debug` impl for `MuxNotification` in `mux_event/mod.rs` (line 298)
- [x] Update all match arms and consumers of this variant within `oriterm_mux`:
  - `in_process/event_pump.rs` (lines 40, 47, 54 -- all three event mappings)
  - `server/notify/mod.rs` (line 36 -- `notification_to_pdu`)
  - `server/notify/tests.rs` (any tests matching on this variant)
  - `mux_event/tests.rs` (tests for notification debug format)
  - `in_process/tests.rs` (tests checking for `PaneTitleChanged` notifications)
- [x] Update consumers in `oriterm` crate:
  - `oriterm/src/app/mux_pump/mod.rs` (line 69)
- [x] Update `notification_to_pdu` in `server/notify/mod.rs` — rename the PDU variant too if applicable (`MuxPdu::NotifyPaneTitleChanged` to `MuxPdu::NotifyPaneMetadataChanged`)
- [x] Update wire protocol PDU if `PaneTitleChanged` has a wire representation
  - Check `protocol/messages.rs` for the PDU enum variant

---

## 01.3 TERM_PROGRAM Env Var Dedup

**File(s):** `oriterm_mux/src/pty/spawn.rs`, `oriterm_mux/src/shell_integration/mod.rs`

**Finding 8:** `TERM_PROGRAM` is set twice with divergent scope. `build_command` in `spawn.rs` sets `TERM_PROGRAM`. `set_common_env` in `shell_integration/mod.rs` also sets `TERM_PROGRAM` plus `TERM_PROGRAM_VERSION`. The two assignments can conflict, and the split makes it unclear which is authoritative.

- [x] Remove `TERM_PROGRAM` from `build_command` in `pty/spawn.rs` (line 234)
- [x] Keep `TERM` and `COLORTERM` in `build_command` — these are terminal identification vars needed regardless of shell integration
- [x] Move `TERM_PROGRAM` and `TERM_PROGRAM_VERSION` into `set_common_env` in `shell_integration/mod.rs` (already there)
- [x] Call `set_common_env` unconditionally from `build_command` — currently it is only called via `setup_injection` which is gated by `config.shell_integration`. If shell integration is disabled, `ORITERM`, `TERM_PROGRAM`, and `TERM_PROGRAM_VERSION` would not be set
  - **Critical:** Either call `set_common_env(cmd)` from `build_command` directly (before the shell integration gate), or split the env var setup into two functions: `set_terminal_env` (always called) and `set_injection_env` (only called when injecting)
- [x] Verify no other call sites set these env vars independently
- [x] Add doc comment on `set_common_env` noting it is the single source of truth for oriterm identification env vars (`ORITERM`, `TERM_PROGRAM`, `TERM_PROGRAM_VERSION`)

---

## 01.4 WSLENV Construction Unification

**File(s):** `oriterm_mux/src/shell_integration/inject.rs`, `oriterm_mux/src/pty/spawn.rs`

**Finding 9:** WSLENV construction has diverged between two code paths. `inject_wsl` in `inject.rs` naively appends keys without dedup. `build_wslenv`/`compute_wslenv` in `spawn.rs` uses proper dedup but a different key set (missing `ORITERM`, `TERM_PROGRAM_VERSION`).

- [x] Unify into a single `compute_wslenv` function (keep it in `spawn.rs` or move to a shared location)
  - `compute_wslenv` is gated by `#[cfg(any(windows, test))]` -- if called from `inject_wsl` in `inject.rs`, the cfg gate must be compatible (inject_wsl is only compiled on Windows via the WSL shell detection path, so this is fine)
- [x] Add missing keys (`ORITERM`, `TERM_PROGRAM_VERSION`) to the `builtin` array in `compute_wslenv`
- [x] Remove manual WSLENV construction from `inject_wsl` in `inject.rs` (lines 112-117) -- have it call `crate::pty::compute_wslenv` instead
- [x] Ensure dedup logic is always applied (no naive appends)
- [x] Add unit test verifying WSLENV dedup and completeness (test already exists for `compute_wslenv` in `pty/tests.rs` -- extend it)

---

## 01.5 Completion Checklist

- [x] `MuxNotification` has doc comments explaining intentional event-to-notification mapping
- [x] `PaneTitleChanged` renamed to `PaneMetadataChanged` (or distinct variants added)
- [x] `TERM_PROGRAM` set in exactly one place (`set_common_env`)
- [x] WSLENV constructed by a single function with dedup and complete key set
- [x] `cargo test -p oriterm_mux` passes
- [x] `./clippy-all.sh` green (no new warnings)
- [x] `./build-all.sh` green

**Exit Criteria:** All event/notification vocabulary mismatches are resolved with documented intent. Environment variable setup has zero duplication. All tests pass.
