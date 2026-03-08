---
section: "05"
title: "NOTE Fixes"
status: complete
goal: "Clean up documentation gaps, decorative banners, import grouping, and minor style inconsistencies"
depends_on: []
sections:
  - id: "05.1"
    title: "Module Doc Comments"
    status: complete
  - id: "05.2"
    title: "Decorative Banner Removal"
    status: complete
  - id: "05.3"
    title: "Type Alias and Ownership Documentation"
    status: complete
  - id: "05.4"
    title: "Allow Attribute Reason Fixes"
    status: complete
  - id: "05.5"
    title: "Import Grouping and Style Consistency"
    status: complete
  - id: "05.6"
    title: "Unsafe Code Scoping"
    status: complete
  - id: "05.7"
    title: "Dead Code and Feature Gating"
    status: complete
  - id: "05.8"
    title: "Completion Checklist"
    status: complete
---

# Section 05: NOTE Fixes

**Status:** Complete
**Goal:** All source files have module docs, no decorative banners remain, import groups follow the standard 3-group pattern, and minor style inconsistencies are normalized.

**Context:** NOTE findings are the lowest-priority category — style, documentation, and consistency issues that don't affect correctness or performance. They accumulate over time and erode readability. Fixing them as a batch is more efficient than addressing them one-by-one during feature work.

---

## 05.1 Module Doc Comments

**File(s):** `oriterm_mux/src/mux_event/tests.rs`

**Finding 6:** Missing `//!` module doc on `mux_event/tests.rs`.

- [x] Add `//! Tests for mux event types and the PTY-to-mux event bridge.` at the top of `mux_event/tests.rs`

---

## 05.2 Decorative Banner Removal

**File(s):** `oriterm_mux/src/mux_event/tests.rs`, `oriterm_mux/src/in_process/event_pump.rs`, `oriterm_mux/src/in_process/mod.rs`

**Finding 7:** Decorative `// --- Section name ---` banners in `mux_event/tests.rs` (lines 202, 314, 363, 416).

**Finding 19:** Decorative `// -- Section name --` banners in `in_process/event_pump.rs` and `in_process/mod.rs`.

- [x] Replace all `// --- Section name ---` and `// -- Section name --` with plain `// Section name` (no dashes)
- [x] Files checked:
  - `mux_event/tests.rs`
  - `in_process/event_pump.rs`
  - `in_process/mod.rs`
  - `server/notify/tests.rs`
  - `backend/embedded/tests.rs`
  - `backend/client/tests.rs`

---

## 05.3 Type Alias and Ownership Documentation

**File(s):** `oriterm_mux/src/pane/mod.rs`

**Finding 14:** `Pane::terminal()` returns `&Arc<FairMutex<Term<MuxEventProxy>>>`, leaking `MuxEventProxy` into the type signature visible to callers.

- [x] Added doc comment on `Pane::terminal()` explaining the return type and noting that callers should prefer `PaneSnapshot` for IPC/render paths

**Finding 15:** Dual ownership of `has_explicit_title` flag on both `Pane` and `Term`. Unclear which is authoritative.

- [x] Add doc comment on `Pane::has_explicit_title` explaining which is authoritative
- [x] Add cross-referencing comment on the `Term` counterpart (in `oriterm_core`)

---

## 05.4 Allow Attribute Reason Fixes

**File(s):** `oriterm_mux/src/pty/signal.rs`

**Finding 16:** `InitState::Ok` has `#[allow(dead_code)]` with misleading reason. Says "flag read in check()" but `check()` is never called. The real purpose is holding the `Arc` alive so the signal_hook handler stays registered.

- [x] Change the `#[allow(dead_code)]` reason to `"holds Arc alive so signal_hook handler stays registered"`

---

## 05.5 Import Grouping and Style Consistency

**File(s):** `oriterm_mux/src/in_process/event_pump.rs`, `oriterm_mux/src/bin/oriterm_mux.rs`

**Finding 20:** Import grouping violation in `event_pump.rs` — internal imports split by a spurious blank line.

- [x] Remove the blank line within the internal import group in `event_pump.rs` to form a single group

**Finding 22:** Downstream code uses both root re-exports and direct module paths for `MuxNotification`.

- [x] Update `oriterm/src/app/mod.rs` to use the root re-export (`oriterm_mux::MuxNotification`) consistently

**Finding 31:** `unwrap_or_else` vs `match` inconsistency in `bin/oriterm_mux.rs`. Sibling function uses `match`.

- [x] Normalize to `match` pattern to match the sibling function's style

---

## 05.6 Unsafe Code Scoping

**File(s):** `oriterm_mux/src/backend/client/transport/mod.rs`, `oriterm_mux/src/backend/client/transport/reader.rs`

**Finding 30:** File-level `#![allow(unsafe_code)]` is overly broad. Unsafe blocks are narrowly scoped to platform FFI.

- [x] Remove file-level `#![allow(unsafe_code)]` from both files
- [x] Add `#[allow(unsafe_code, reason = "platform FFI: {specific reason}")]` on each individual `unsafe` block
- [x] Verify the crate-level `unsafe_code = "deny"` lint still works correctly with the per-block allows

---

## 05.7 Dead Code and Feature Gating

**File(s):** `oriterm_mux/src/protocol/snapshot.rs`, `oriterm_mux/src/server/snapshot.rs`

**Finding 33:** `#[allow(dead_code)]` on `WireColor` enum in `protocol/snapshot.rs`. Reserved for future use but never constructed.

- [x] `WireColor` already has proper `#[allow(dead_code, reason = "reserved for future incremental wire format")]` — no change needed

**Finding 32:** `build_snapshot` in `server/snapshot.rs` is `pub fn` but internal.

- [x] Changed both `build_snapshot` and `build_snapshot_into` to `pub(crate) fn`

**Finding 17:** `inject_wsl` assumes `--cd` is always valid for WSL (requires Windows 10 build 18362+).

- [x] Add comment documenting minimum Windows version requirement

---

## 05.8 Completion Checklist

- [x] `mux_event/tests.rs` has `//!` module doc
- [x] Zero decorative banners in `mux_event/tests.rs`, `in_process/event_pump.rs`, `in_process/mod.rs`
- [x] `Pane::terminal()` return type documented with doc comment
- [x] `has_explicit_title` ownership documented with cross-references
- [x] `InitState::Ok` allow reason corrected
- [x] Import groups follow 3-group pattern in `event_pump.rs`
- [x] `MuxNotification` import paths consistent across `oriterm`
- [x] `unwrap_or_else`/`match` normalized in `bin/oriterm_mux.rs`
- [x] File-level `#![allow(unsafe_code)]` replaced with per-block allows
- [x] `WireColor` has proper dead_code reason
- [x] `build_snapshot` visibility narrowed
- [x] `inject_wsl` has Windows version comment
- [x] `cargo test -p oriterm_mux` passes
- [x] `./clippy-all.sh` green
- [x] `./build-all.sh` green

**Exit Criteria:** All NOTE findings resolved. Every source file has a module doc. No decorative banners. Import groups follow the standard pattern. Allow attributes have accurate reasons. No dead code without justification.
