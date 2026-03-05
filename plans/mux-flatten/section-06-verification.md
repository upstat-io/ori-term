---
section: "06"
title: "Verification"
status: not-started
goal: "Full test suite passes, no UI leaks remain, behavioral equivalence confirmed"
depends_on: ["02", "03", "04", "05"]
sections:
  - id: "06.1"
    title: "Audit for UI Leaks"
    status: not-started
  - id: "06.2"
    title: "Test Migration"
    status: not-started
  - id: "06.3"
    title: "Behavioral Equivalence"
    status: not-started
  - id: "06.4"
    title: "Documentation"
    status: not-started
  - id: "06.5"
    title: "Completion Checklist"
    status: not-started
---

# Section 06: Verification

**Status:** Not Started
**Goal:** The flattened mux is verified correct: all tests pass, no UI
concepts leak, and the GUI behaves identically to before the refactor.

**Depends on:** All previous sections.

---

## 06.1 Audit for UI Leaks

- [ ] Run targeted greps across `oriterm_mux/src/`:
  ```
  grep -rn "TabId\|WindowId\|SessionId\|MuxTab\|MuxWindow\|SessionRegistry" \
    oriterm_mux/src/ --include="*.rs"
  grep -rn "GUI\|winit\|tab.bar\|frontend" \
    oriterm_mux/src/ --include="*.rs"
  ```
  First grep: verify zero tab/window/session type references remain.
  Second grep: verify zero UI-layer concept references remain.
  Filter out false positives (`tab` in "table", "stable", etc.).

- [ ] Verify `oriterm_mux/Cargo.toml` has no GUI-related dependencies
  (no winit, wgpu, softbuffer, etc. — should already be clean)

- [ ] Verify one-way dependency: `oriterm_mux/Cargo.toml` does NOT list `oriterm`
      as a dependency (mux must not know about the GUI)
- [ ] Verify no circular imports within `oriterm/src/session/`: layout modules
      import `PaneId` from `oriterm_mux`, not from `crate::session::id`

- [ ] Verify `oriterm_mux` public API surface:
  - Exports only: `PaneId`, `DomainId`, `ClientId`, `IdAllocator`,
    `MuxId`, `Pane`, `MarkCursor`, `PaneEntry`, `PaneRegistry`,
    `InProcessMux`, `ClosePaneResult`,
    `MuxEvent`, `MuxEventProxy`, `MuxNotification`,
    `Domain`, `DomainState`, `SpawnConfig`,
    `PtyConfig`, `PtyHandle`, `PtyControl`, `ExitStatus`, `spawn_pty`,
    `MuxBackend`, `EmbeddedMux`, `MuxClient`,
    protocol wire types (pane-only: `PaneSnapshot`, `WireCell`, etc.),
    server types
  - Does NOT export: `TabId`, `WindowId`, `SessionId`, `MuxTab`,
    `MuxWindow`, `SessionRegistry`, `SplitTree`, `SplitDirection`,
    `FloatingLayer`, `FloatingPane`, `Rect`, `PaneLayout`,
    `DividerLayout`, `Direction`, `MuxTabInfo`, `MuxWindowInfo`

---

## 06.2 Test Migration

- [ ] All mux unit tests updated for flat pane model

### Integration tests: `tests/contract.rs`

- [ ] `TestContext` struct: remove `window_id: WindowId` and `tab_id: TabId` fields
      (keep only `pane_id: PaneId`)
- [ ] Remove `use oriterm_mux::{TabId, WindowId}` import
- [ ] Rewrite `embedded_context()`:
  - Remove `mux.create_window()` call
  - Replace `mux.create_tab(window_id, &config, theme)` with
    `mux.spawn_pane(&config, theme, &wakeup)` (new API)
  - Remove `window_id` and `tab_id` from `TestContext` construction
- [ ] Rewrite `daemon_context()`:
  - Remove `client.create_window()` call
  - Replace `client.create_tab(window_id, &config, theme)` with
    `client.spawn_pane(&config, theme)` (new RPC — sends `SpawnPane` PDU)
  - Remove `client.claim_window(window_id)` call
  - Remove `window_id` and `tab_id` from `TestContext` construction
- [ ] All `muxbackend_contract_tests!` test bodies are pane-only
      (they only use `ctx.pane_id` — no changes needed to test bodies)
- [ ] Verify test count matches pre-refactor

### Integration tests: `tests/e2e.rs`

- [ ] Same factory function changes as contract.rs:
  - Remove `create_window()` / `create_tab()` / `claim_window()` calls
  - Use `spawn_pane()` instead
- [ ] Remove `TabId`, `WindowId` imports
- [ ] Update test helper struct to remove window/tab ID fields
- [ ] Review each test — most are pane-centric (send_input, snapshot, search)
      and only need factory function changes

### Other test files

- [ ] GUI tests in `oriterm/src/app/tests.rs` use local session types
- [ ] Layout tests relocated and passing in `oriterm/src/session/`
- [ ] Test count: verify no tests were silently dropped
  ```
  cargo test --workspace 2>&1 | grep "test result"
  ```
  Compare test count before and after refactor.

---

## 06.3 Behavioral Equivalence

- [ ] GUI starts, spawns a pane, displays terminal output
- [ ] Tab creation works (local session creates tab + mux spawns pane)
- [ ] Tab closing works (local session removes tab + mux closes pane)
- [ ] Split pane works (local session splits + mux spawns second pane)
- [ ] Window creation/closing works (local session only)
- [ ] Tab drag/tear-off/merge works (local session only)
- [ ] Pane title updates flow: mux notification -> GUI session -> tab bar
- [ ] Bell/alert flows: mux notification -> GUI session -> tab bar
- [ ] Resize flows: GUI -> `mux.resize_pane_grid()` -> PTY resize
- [ ] Keyboard input flows: GUI -> `mux.send_input()` -> PTY stdin

---

## 06.4 Documentation

- [ ] Update `CLAUDE.md` "Key Paths" to reflect new session module location
- [ ] Update `CLAUDE.md` "Architecture" section if it references mux
      session model
- [ ] Update memory files if they reference mux tab/window types
- [ ] `oriterm_mux/src/lib.rs` module doc describes the flat pane server
- [ ] `oriterm/src/session/mod.rs` module doc describes the GUI session model
- [ ] Remove or update `plans/muxbackend-boundary/` if it references
      the old session model

---

## 06.5 Completion Checklist

- [ ] `./build-all.sh` passes
- [ ] `./clippy-all.sh` passes
- [ ] `./test-all.sh` passes
- [ ] Zero UI references in `oriterm_mux/src/` (verified by grep)
- [ ] Test count is equal to or greater than pre-refactor count
- [ ] GUI behavioral equivalence confirmed (manual smoke test)
- [ ] Documentation updated

**Exit Criteria:** `oriterm_mux` is a flat pane server with zero UI
awareness. `oriterm` owns all session/layout/presentation state. The
application behaves identically to before the refactor. All automated
tests pass. Documentation is current.
