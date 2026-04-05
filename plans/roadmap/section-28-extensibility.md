---
section: 28
title: Extensibility
status: not-started
reviewed: true
third_party_review:
  status: none
  updated: null
tier: 7
goal: "Lua 5.4 as the canonical policy/routing layer — Rust owns terminal semantics and execution, Lua owns behavioral decisions. Plus custom shaders, smart paste, undo close tab, session recording, workspaces."
success_criteria:
  - "Lua VM initializes at startup, lives for process lifetime, zero-overhead when idle"
  - "Built-in tab title formatting runs in Lua (first dogfood target)"
  - "All meaningful MuxNotification variants (output, close, bell, metadata, command-complete, config) fire through Lua dispatch table"
  - "User scripts sandboxed — no io/os/debug/package/require/load/dofile/loadfile"
  - "Keystroke-to-Lua-action < 100μs, dispatch per pump < 0.25ms typical"
  - "Hot-reload re-executes scripts without terminal restart"
  - "Rust fallbacks for all critical paths — Lua crash = degraded mode, not broken terminal"
  - "`./test-all.sh` green — no regressions"
  - "All section success criteria met"
sections:
  # Lua Runtime (28.1–28.7)
  - id: "28.1"
    title: "Core Runtime & Sandbox"
    status: not-started
  - id: "28.2"
    title: "Event System & Dispatch Architecture"
    status: not-started
  - id: "28.3"
    title: "API Surface"
    status: not-started
  - id: "28.4"
    title: "Built-in Lua Behaviors"
    status: not-started
  - id: "28.5"
    title: "Keybinding Integration"
    status: not-started
  - id: "28.6"
    title: "User Scripts & Hot-Reload"
    status: not-started
  - id: "28.7"
    title: "Lua Verification & Security"
    status: not-started
  # Other Extensibility (28.8–28.12, renumbered)
  - id: "28.8"
    title: "Custom Shaders"
    status: not-started
  - id: "28.9"
    title: "Smart Paste"
    status: not-started
  - id: "28.10"
    title: "Undo Close Tab"
    status: not-started
  - id: "28.11"
    title: "Session Recording + Playback"
    status: not-started
  - id: "28.12"
    title: "Workspaces"
    status: not-started
  - id: "28.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "28.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 28: Extensibility

**Status:** Not Started
**Goal:** Make Lua 5.4 the canonical policy and routing layer of ori_term. Rust is the engine (grid, VTE, GPU, PTY). Lua is the brain (what to do when things happen). ori_term's own behaviors ARE Lua scripts — users extend/override the same system. Best-in-class performance and security. Plus custom shaders, smart paste, undo close tab, session recording, workspaces.

**Success Criteria:**

- [ ] Lua VM initializes at startup, lives for process lifetime, zero-overhead when idle (satisfies mission criterion 1)
- [ ] Built-in `format_tab_title()` runs in Lua, produces identical output to current Rust logic (satisfies mission criterion 2)
- [ ] All meaningful MuxNotification variants dispatch through Lua callback table — clipboard and internal plumbing events excluded (satisfies mission criterion 3)
- [ ] User scripts cannot access io, os, debug, package, require, load, dofile, loadfile (satisfies mission criterion 4)
- [ ] Keystroke-to-Lua-action < 100μs measured via perf stats; dispatch per pump < 0.25ms typical, < 1ms worst-case (satisfies mission criterion 5)
- [ ] `init.lua` changes detected by file watcher → scripts re-executed without terminal restart (satisfies mission criterion 6)
- [ ] Lua crash/OOM → terminal enters degraded mode with Rust fallbacks for title, bell, notifications — no user-visible breakage (satisfies mission criterion 7)
- [ ] `./test-all.sh` green — no regressions (satisfies mission criterion 8)

**Context:** Ghostty has no scripting. WezTerm has Lua but with poor architecture (fresh state per reload, zero sandboxing, async wrapping overhead, blocking background threads). Kitty has Python kittens (slow, out-of-process). A properly designed Lua layer — where the terminal's own behaviors are Lua scripts that users can override — is the long-term differentiator. This is the Neovim model applied to terminal emulation.

**Architecture:**

```
                    ┌─────────────────────────────────────────────┐
                    │               Lua Policy Layer               │
                    │  ┌──────────┐  ┌──────────┐  ┌───────────┐ │
                    │  │ Built-in │  │  User     │  │ Callback  │ │
                    │  │ Scripts  │  │  Scripts  │  │ Registry  │ │
                    │  │(include_ │  │(init.lua) │  │(per-event │ │
                    │  │  str!)   │  │(sandboxed)│  │ bitsets)  │ │
                    │  └────┬─────┘  └────┬──────┘  └─────┬─────┘ │
                    │       └──────┬──────┘               │       │
                    │              ▼                       │       │
                    │     ┌────────────────┐              │       │
                    │     │  ScriptEngine  │◄─────────────┘       │
                    │     │  (Lua 5.4 VM)  │                      │
                    │     └───────┬────────┘                      │
                    └─────────────┼────────────────────────────────┘
                                  │
                    ╔═════════════╧══════════════════════════╗
                    ║         Host Bridge (LuaContext)        ║
                    ║  Queries (sync) │ Mutations (deferred)  ║
                    ║  get_title()    │ → Vec<LuaCommand>     ║
                    ║  get_cwd()      │ → execute_after_cb()  ║
                    ╚═════════════╤══════════════════════════╝
                                  │
    ┌─────────────────────────────┼─────────────────────────────────┐
    │                     Rust Engine (App)                          │
    │                                                               │
    │  PTY Reader ──► MuxEvent ──► MuxNotification ──► Lua dispatch │
    │       │              │              │                    │     │
    │       │              │              │           ┌────────┤     │
    │       │              │              │           │  Rust  │     │
    │       │              │              │           │Fallback│     │
    │       ▼              ▼              ▼           └────────┘     │
    │  [oriterm_core] [oriterm_mux]  [oriterm/app]                  │
    │   Grid, VTE      Pane I/O     Event loop, GPU, Session        │
    └───────────────────────────────────────────────────────────────┘
```

**Design Principles:**

1. **Rust owns semantics, Lua owns policy.** VTE parsing, grid mutation, GPU rendering, PTY I/O, snapshot transfer — all stay in Rust. Tab title formatting, notification decisions, shell event responses, keybinding interception — all move to Lua. The dividing line: if it mutates terminal state or touches I/O, it's Rust. If it decides *what to do* in response to an event, it's Lua.

2. **Zero-overhead when idle.** Per-event registration bitsets mean the cost of having Lua installed but no callbacks registered is a single branch (bit check) per notification — nanoseconds. No Lua VM call, no stack setup, no allocation.

3. **Command buffer, not direct mutation.** Lua callbacks receive data-only events and return a `Vec<LuaCommand>`. Rust executes the commands after the callback returns. This avoids borrow checker conflicts (Lua can't hold `&mut App`), prevents reentrancy bugs, and enables batching. Queries (read-only) execute synchronously for immediate feedback.

**Reference implementations:**
- **WezTerm** `config/src/lua.rs`: Lua VM lifecycle, event registration via Lua registry tables, UserData wrappers for pane/window IDs with on-demand lock resolution
- **WezTerm** `lua-api-crates/mux/src/window.rs`: `MuxWindow(WindowId)` UserData pattern — opaque ID, resolve on each method call
- **Neovim**: Single global Lua state, main thread, bytecode caching (`vim.loader`), built-in features in Lua (LSP, treesitter, diagnostics)

**Depends on:** None (cross-cutting, touches event loop + mux + config). Config system (Section 13) and shell integration (Section 20) are both complete.

**Crate impact:** All Lua code lives in `oriterm/src/lua/` — the application shell crate. No changes to `oriterm_core` or `oriterm_ui`. The one `oriterm_mux` change is expanding `MuxNotification` in `mux_event/mod.rs` and updating `event_pump.rs` to pass distinct metadata events (title/cwd/icon) instead of collapsing them into `PaneMetadataChanged`.

---

## 28.1 Core Runtime & Sandbox

**File(s):** `oriterm/src/lua/mod.rs` (new), `oriterm/src/lua/engine.rs` (new), `oriterm/src/lua/sandbox.rs` (new), `oriterm/src/app/mod.rs` (add field), `oriterm/src/app/constructors.rs` (init), `oriterm/Cargo.toml` (add mlua dep)

**Goal:** A single long-lived Lua 5.4 VM on the main thread with a safe stdlib baseline and comprehensive sandbox for user scripts. The VM is created at startup, lives for process lifetime, and has zero overhead when no callbacks are registered.

**Success Criteria:**
- [ ] `ScriptEngine::new()` creates Lua VM with TABLE + STRING + MATH + COROUTINE + UTF8 stdlib only — no io, os, debug, package
- [ ] `App` struct has `script_engine: Option<ScriptEngine>` field, initialized in `build_common()`
- [ ] User scripts execute in a sandboxed environment (via `Chunk::set_environment()`) that blocks io, os, debug, package, require, load, dofile, loadfile
- [ ] `set_memory_limit()` enforced (configurable, default 64MB VM-wide)
- [ ] `set_hook(every_nth_instruction(10_000_000))` kills runaway user scripts
- [ ] Built-in scripts have access to full safe stdlib (no instruction limit)
- [ ] `include_str!` loads built-in Lua source, compiled to bytecode at startup, cached in-memory
- [ ] `cargo test -p oriterm -- lua` passes all engine tests
- [ ] Satisfies mission criteria 1 (VM lifecycle) and 4 (sandbox)

**Context:** WezTerm uses `Lua::new()` (full unsafe stdlib, zero sandboxing). This is a security liability — user scripts have full filesystem/network access. Neovim's model is better: a safe baseline with explicit API surface. mlua's `Lua::new_with()` lets us choose exactly which stdlib modules to expose. The `vendored` feature cross-compiles to all platforms without system Lua.

**Reference implementations:**
- **WezTerm** `config/src/lua.rs:212`: `Lua::new()` — full stdlib, no limits. Counter-example of what NOT to do.
- **mlua docs** `Lua::new_with()`: Choose stdlib modules. `set_memory_limit()`, `set_hook()` for limits.

**Depends on:** None (foundation for all other Lua subsections).

### 28.1.1 Add mlua dependency

- [ ] Add to `oriterm/Cargo.toml`:
  ```toml
  mlua = { version = "0.10", features = ["lua54", "vendored"] }
  ```
  The `vendored` feature compiles Lua 5.4 C source from within mlua — no system Lua dependency, cross-compiles to `x86_64-pc-windows-gnu`, macOS, and Linux. Adds ~3-4MB to binary, ~1-2min to clean build.
- [ ] Verify `bitflags` is already a workspace dependency (used by `oriterm_core` for `CellFlags`). If not in `oriterm/Cargo.toml`, add it — needed for `CallbackFlags` in 28.2.2.
- [ ] Verify the `vendored` feature cross-compiles: `cargo build --target x86_64-pc-windows-gnu -p oriterm` must succeed. The vendored Lua C build uses `cc` crate — verify `CC_x86_64_pc_windows_gnu` is set in the build environment if needed.
- [ ] Audit mlua's `unsafe` usage: mlua contains unsafe FFI to liblua. Verify it does not conflict with the workspace `unsafe_code = "deny"` lint — mlua's unsafe is internal to the dependency, not in our code. No `unsafe` blocks should appear in `oriterm/src/lua/`.
- [ ] Verify: `./build-all.sh` succeeds on all targets
- [ ] Verify: `./clippy-all.sh` clean (mlua is well-behaved, no expected warnings)

### 28.1.2 Create `oriterm/src/lua/` module

Create a directory module with the following structure. Each file must stay under 500 lines (split proactively). All tests in sibling `tests.rs` files per `test-organization.md`. Every new file must begin with `//!` module doc comment. Every `pub(crate)` item must have `///` doc comment. No `unwrap()` in any production code — return `LuaResult` or provide defaults.

```
oriterm/src/lua/
├── mod.rs          # Module re-exports, #[cfg(test)] mod tests;
├── engine.rs       # ScriptEngine struct, lifecycle
├── sandbox.rs      # Sandbox setup, environment creation
├── bytecode.rs     # Bytecode compilation and caching
└── tests.rs        # All unit tests for the lua module
```

- [ ] `mod.rs` — Module declarations and re-exports:
  ```rust
  //! Lua 5.4 scripting engine — policy and routing layer.
  //!
  //! Rust owns terminal semantics and execution. Lua owns behavioral
  //! decisions: tab title formatting, notification policy, shell event
  //! responses, keybinding interception.

  mod bytecode;
  mod engine;
  mod sandbox;

  pub(crate) use engine::ScriptEngine;

  #[cfg(test)]
  mod tests;
  ```

- [ ] `engine.rs` — `ScriptEngine` struct:
  ```rust
  //! Lua VM lifecycle — single long-lived state on main thread.

  use mlua::{Lua, StdLib, Result as LuaResult};

  /// The Lua scripting engine. Created once at startup, lives for
  /// process lifetime. All Lua execution is synchronous on the main
  /// thread (Neovim model).
  pub(crate) struct ScriptEngine {
      lua: Lua,
      // Callback registry bitsets — per-event flags indicating whether
      // any callback is registered for that event type. Zero-overhead
      // when no callbacks: check one bit, skip Lua call entirely.
      callback_flags: CallbackFlags,
  }

  impl ScriptEngine {
      pub(crate) fn new(config: &LuaConfig) -> LuaResult<Self> {
          // Safe stdlib only — no io, os, debug, package.
          let stdlib = StdLib::TABLE
              | StdLib::STRING
              | StdLib::MATH
              | StdLib::COROUTINE
              | StdLib::UTF8;
          let lua = Lua::new_with(stdlib, mlua::LuaOptions::default())?;

          // VM-wide memory limit (configurable, default 64MB).
          // set_memory_limit returns the previous limit (usize), not Result.
          lua.set_memory_limit(config.memory_limit_bytes);

          Ok(Self {
              lua,
              callback_flags: CallbackFlags::empty(),
          })
      }
  }
  ```

- [ ] `sandbox.rs` — Two-tier environment creation:
  ```rust
  //! Sandbox setup for user scripts.
  //!
  //! Built-in scripts: full safe stdlib + oriterm.* API.
  //! User scripts: sandboxed environment blocking io, os, debug,
  //! package, require, load, dofile, loadfile. Memory-limited.
  //! Instruction-limited via hooks.

  /// Create a sandboxed environment for user scripts.
  /// Blocks: io, os, debug, package, require, load, dofile, loadfile.
  /// Allows: table, string, math, coroutine, utf8, oriterm.* API,
  ///         print (redirected to log), type, tostring, tonumber,
  ///         pairs, ipairs, next, select, unpack, error, pcall, xpcall.
  pub(crate) fn create_user_environment(lua: &Lua) -> LuaResult<LuaTable> { ... }

  /// Create the built-in environment. Full safe stdlib + oriterm.* API.
  /// No instruction hooks (trusted code, covered by tests).
  pub(crate) fn create_builtin_environment(lua: &Lua) -> LuaResult<LuaTable> { ... }
  ```

- [ ] `bytecode.rs` — Compile-once, execute-many:
  ```rust
  //! Bytecode compilation and caching.
  //!
  //! Built-in scripts are compiled to bytecode at startup from
  //! include_str! source. User scripts are compiled on first load
  //! and cached by source hash. Changed files are recompiled.

  /// Compiled bytecode with source hash for cache invalidation.
  pub(crate) struct CachedChunk {
      bytecode: Vec<u8>,
      source_hash: u64,
  }
  ```

### 28.1.3 Wire ScriptEngine into App

**File(s):** `oriterm/src/app/mod.rs`, `oriterm/src/app/constructors.rs`

- [ ] [BLOAT] **Pre-check:** `app/mod.rs` is 493 lines — at the 500-line limit. Adding the `script_engine` field is acceptable (single line), but if any subsequent subsection needs to add more code to this file, it must be extracted to a submodule first. Do NOT add helper methods to `app/mod.rs`.
- [ ] Add field to `App` struct (after `_config_monitor` field, ~line 213 of `app/mod.rs`):
  ```rust
  /// Lua scripting engine — policy and routing layer.
  /// None when scripting is disabled via config.
  script_engine: Option<crate::lua::ScriptEngine>,
  ```
- [ ] Initialize `script_engine` local variable in `build_common()` (after `event_sender` creation, ~line 127 of constructors.rs — before the `Self {` block at ~line 129). Then add `script_engine,` to the `Self { ... }` struct literal:
  ```rust
  let script_engine = if config.lua.enabled {
      match crate::lua::ScriptEngine::new(&config.lua) {
          Ok(engine) => {
              log::info!("lua: engine initialized (memory limit: {}MB)",
                  config.lua.memory_limit_bytes / (1024 * 1024));
              Some(engine)
          }
          Err(e) => {
              log::error!("lua: failed to initialize engine: {e}");
              None
          }
      }
  } else {
      None
  };
  ```
  Error handling follows the config reload pattern: log error, continue without Lua. Terminal is fully functional without scripting.
- [ ] Add `pub(crate) mod lua;` to `oriterm/src/lib.rs` (between `keybindings` and `platform` alphabetically, ~line 18)

### 28.1.4 Add LuaConfig to config system

**File(s):** `oriterm/src/config/mod.rs`, new `oriterm/src/config/lua_config.rs`

- [ ] Create `oriterm/src/config/lua_config.rs`:
  ```rust
  //! Lua scripting configuration.

  /// Configuration for the Lua scripting engine.
  #[derive(Debug, Clone, serde::Deserialize)]
  #[serde(default)]
  pub(crate) struct LuaConfig {
      /// Enable the Lua scripting engine.
      pub enabled: bool,
      /// Path to user init script. Default: `~/.config/oriterm/init.lua`.
      pub init_script: Option<String>,
      /// Directory for auto-loaded scripts. Default: `~/.config/oriterm/scripts/`.
      pub scripts_dir: Option<String>,
      /// VM-wide memory limit in bytes. Default: 67_108_864 (64MB).
      pub memory_limit_bytes: usize,
      /// Instruction limit for user scripts (per callback invocation).
      /// Default: 10_000_000. Set to 0 to disable.
      pub instruction_limit: u32,
      /// Auto-reload scripts when files change. Default: true.
      pub auto_reload: bool,
  }

  impl Default for LuaConfig {
      fn default() -> Self {
          Self {
              enabled: true,
              init_script: None,
              scripts_dir: None,
              memory_limit_bytes: 64 * 1024 * 1024, // 64MB
              instruction_limit: 10_000_000,
              auto_reload: true,
          }
      }
  }
  ```
- [ ] Add `pub lua: LuaConfig` field to `Config` struct in `oriterm/src/config/mod.rs`
- [ ] Add `mod lua_config;` and `pub(crate) use lua_config::LuaConfig;` to config module
- [ ] TOML section:
  ```toml
  [lua]
  enabled = true
  init_script = "~/.config/oriterm/init.lua"
  scripts_dir = "~/.config/oriterm/scripts/"
  memory_limit_bytes = 67108864
  instruction_limit = 10000000
  auto_reload = true
  ```
- [ ] Add serde tests to `oriterm/src/config/tests.rs` for LuaConfig parsing (default values, explicit values, partial)

### 28.1.5 Tests

**File:** `oriterm/src/lua/tests.rs`

Write failing tests FIRST (TDD), then implement to pass. Tests use sibling `tests.rs` pattern (no `mod tests {}` wrapper, use `super::` imports). No inline test modules in source files.

**Matrix dimensions:**
- **VM creation**: safe stdlib only, no unsafe modules accessible
- **Sandbox**: user env blocks io/os/debug/package/require/load/dofile/loadfile; built-in env has full safe stdlib
- **Memory limit**: allocation above limit returns error
- **Instruction limit**: infinite loop killed after N instructions
- **Bytecode**: source compiled once, cached, re-execution uses cache
- **Config**: LuaConfig default values, explicit values, partial TOML parsing

**Semantic pins:**
- `test_sandbox_blocks_io` — ONLY passes when sandbox correctly blocks `io.open`
- `test_sandbox_blocks_os` — ONLY passes when sandbox correctly blocks `os.execute`
- `test_instruction_limit_kills_loop` — ONLY passes when hook stops `while true do end`

```
- [ ] test_engine_creates_lua_vm — ScriptEngine::new() succeeds, Lua state is valid
- [ ] test_safe_stdlib_only — table/string/math/coroutine/utf8 available; io/os/debug/package return nil
- [ ] test_sandbox_blocks_io — user env: `io.open("/etc/passwd", "r")` errors
- [ ] test_sandbox_blocks_os — user env: `os.execute("ls")` errors
- [ ] test_sandbox_blocks_debug — user env: `debug.getinfo(1)` errors
- [ ] test_sandbox_blocks_require — user env: `require("socket")` errors
- [ ] test_sandbox_blocks_load — user env: `load("return 1")` errors
- [ ] test_sandbox_blocks_dofile — user env: `dofile("/etc/passwd")` errors
- [ ] test_sandbox_blocks_loadfile — user env: `loadfile("/etc/passwd")` errors
- [ ] test_sandbox_allows_safe_builtins — user env: pairs, ipairs, type, tostring, tonumber, pcall, xpcall, select, unpack, error all work
- [ ] test_builtin_env_has_full_stdlib — built-in env: table/string/math/coroutine/utf8 all accessible
- [ ] test_memory_limit_enforced — allocate beyond limit → Lua error (not process crash)
- [ ] test_instruction_limit_kills_loop — `while true do end` in user env errors after N instructions
- [ ] test_instruction_limit_not_applied_to_builtins — built-in script with long computation succeeds
- [ ] test_bytecode_cache_reuses_compilation — compile source, execute twice, second execution uses cached bytecode
- [ ] test_malformed_lua_logs_error — syntax error in Lua source → LuaResult::Err, not panic
```

- [ ] Verify all tests pass: `timeout 150 cargo test -p oriterm -- lua`
- [ ] Verify debug and release: `timeout 150 cargo test -p oriterm --release -- lua`

- [ ] **TPR checkpoint** — `/tpr-review` covering 28.1.1–28.1.5 implementation work

---

## 28.2 Event System & Dispatch Architecture

**File(s):** `oriterm/src/lua/events.rs` (new), `oriterm/src/lua/commands.rs` (new), `oriterm/src/lua/dispatch.rs` (new), `oriterm/src/app/mux_pump/mod.rs` (hook), `oriterm_mux/src/mux_event/mod.rs` (expand notifications)

**Goal:** A typed event dispatch system where MuxNotifications fire through a Lua callback table. Per-event bitsets ensure zero overhead when no callbacks are registered. Lua returns a command buffer that Rust executes after the callback.

**Success Criteria:**
- [ ] `LuaEvent` enum covers all MuxNotification variants with typed payloads
- [ ] `CallbackFlags` bitset tracks which events have registered callbacks — single bit check per notification
- [ ] `LuaCommand` enum covers all mutation actions Lua can request
- [ ] `oriterm.on("event_name", callback)` registers callbacks, stored in Lua registry
- [ ] `MuxNotification::PaneMetadataChanged` split into distinct `PaneTitleChanged`/`PaneCwdChanged`/`PaneIconChanged` notifications
- [ ] Dispatch budget: < 0.25ms typical per pump, < 1ms worst-case
- [ ] `cargo test -p oriterm -- lua::events` passes
- [ ] Satisfies mission criterion 3 (all notifications dispatch through Lua)

**Context:** The current `handle_mux_notification()` in `mux_pump/mod.rs:54-137` is an 8-arm match dispatcher. The Lua layer sits between notification arrival and the native handler — Lua decides, Rust executes. The `PaneMetadataChanged(PaneId)` notification currently collapses title/icon/cwd changes into one variant (lossy). Lua needs to know WHICH field changed, so we must pass distinct notification types through.

**Reference implementations:**
- **WezTerm** `config/src/lua.rs:722-793`: Callbacks stored in Lua registry table keyed by `"wezterm-event-{name}"`, dispatched in FIFO order, early exit if handler returns `false`.

**Depends on:** Section 28.1 (ScriptEngine must exist).

### 28.2.1 Expand MuxNotification with distinct metadata events

**File(s):** `oriterm_mux/src/mux_event/mod.rs`, `oriterm_mux/src/in_process/event_pump.rs`

The current event pump (`event_pump.rs:32-51`) collapses `MuxEvent::PaneTitleChanged`, `PaneCwdChanged`, `PaneIconChanged` into a single `MuxNotification::PaneMetadataChanged(PaneId)`. For Lua to provide meaningful callbacks (`on_title_changed`, `on_cwd_changed`), it needs distinct notifications.

- [ ] Add three new `MuxNotification` variants (in `mux_event/mod.rs` after `PaneMetadataChanged`, ~line 283):
  ```rust
  PaneTitleChanged { pane_id: PaneId, title: String },
  PaneCwdChanged { pane_id: PaneId, cwd: String },
  PaneIconChanged { pane_id: PaneId, icon_name: String },
  ```
- [ ] Update the manual `Debug` impl for `MuxNotification` (`mux_event/mod.rs:327-352`) — add match arms for the three new variants. This is a DRIFT sync point the compiler does NOT catch (manual Debug impl, not derived).
- [ ] Update `event_pump.rs` (`in_process/event_pump.rs:32-51`) to emit distinct notifications instead of collapsing to `PaneMetadataChanged`. Each `MuxEvent::PaneTitleChanged` now produces `MuxNotification::PaneTitleChanged { pane_id, title }`, etc.
- [ ] Keep `PaneMetadataChanged` as a catch-all for any metadata change that doesn't have a specific variant (backward compat)
- [ ] Update `handle_mux_notification()` in `oriterm/src/app/mux_pump/mod.rs:54-137` to handle the new variants — all three call `sync_tab_bar_from_mux()` + `mark_pane_window_dirty()` (same behavior as current `PaneMetadataChanged`, but Lua can now distinguish which field changed)
- [ ] Update exhaustive match in all consumers (compiler enforces this via `#[non_exhaustive]` or exhaustive match)
- [ ] Check for any other files matching on `MuxNotification` — `grep -r 'MuxNotification::' oriterm*/src/` to find all consumers
- [ ] Add tests in `oriterm_mux/src/mux_event/tests.rs` for new notification variants

### 28.2.2 Define LuaEvent and LuaCommand enums

**File(s):** `oriterm/src/lua/events.rs`, `oriterm/src/lua/commands.rs`

- [ ] `events.rs` — Typed events Lua receives. Every variant gets a `///` doc comment. `LuaEvent::from_notification(notification: &MuxNotification, is_focused: bool, ...)` is a **pure function** — takes the notification and pre-extracted context, NOT `&App`. This avoids borrow conflicts in dispatch.
  ```rust
  //! Lua event types — data-only events dispatched to Lua callbacks.

  /// Events dispatched to Lua callbacks. Each variant carries all data
  /// needed for the callback — no back-references to App state.
  pub(crate) enum LuaEvent {
      PaneOutput { pane_id: PaneId, is_focused: bool },
      PaneClosed { pane_id: PaneId, exit_code: i32 },
      PaneTitleChanged { pane_id: PaneId, title: String },
      PaneCwdChanged { pane_id: PaneId, cwd: String },
      PaneIconChanged { pane_id: PaneId, icon_name: String },
      CommandComplete { pane_id: PaneId, duration_secs: f64, is_focused: bool },
      Bell { pane_id: PaneId, is_focused: bool },
      ConfigReloaded,
      // Added by 28.9 (Smart Paste Lua hook):
      // PasteRequest { pane_id: PaneId, text: String, line_count: usize },
  }
  ```

- [ ] `commands.rs` — Actions Lua can request. Every variant gets a `///` doc comment. Include `LogLevel` enum in the same file (`pub(crate) enum LogLevel { Info, Warn, Error }`). `SplitDirection` reuses the existing type from `oriterm/src/session/` (do not duplicate — import it).
  ```rust
  //! Lua command types — mutations Lua requests, Rust executes.

  /// Commands returned by Lua callbacks. Rust executes these after
  /// the callback returns — never during callback execution.
  pub(crate) enum LuaCommand {
      // Pane I/O
      WriteToPane { pane_id: PaneId, text: String },
      // Tab operations
      NewTab { cwd: Option<String> },
      CloseTab { tab_id: TabId },
      // Pane operations
      ClosePane { pane_id: PaneId },
      SplitPane { direction: SplitDirection },
      FocusPane { pane_id: PaneId },
      // Appearance
      SetTabTitle { tab_id: TabId, title: String },
      SetTheme { name: String },
      // Clipboard
      CopySelection { pane_id: PaneId },
      PasteToPane { pane_id: PaneId },
      // Notifications
      SendNotification { title: String, body: String },
      RingBell { pane_id: PaneId },
      MarkUnseenOutput { pane_id: PaneId },
      // Config
      ReloadConfig,
      // Logging
      Log { level: LogLevel, message: String },
  }
  ```

- [ ] `CallbackFlags` bitset (in `events.rs`):
  ```rust
  bitflags::bitflags! {
      /// Per-event-type flags. If no bit is set for an event type,
      /// the dispatch loop skips Lua entirely — zero overhead.
      pub(crate) struct CallbackFlags: u32 {
          const PANE_OUTPUT        = 1 << 0;
          const PANE_CLOSED        = 1 << 1;
          const TITLE_CHANGED      = 1 << 2;
          const CWD_CHANGED        = 1 << 3;
          const ICON_CHANGED       = 1 << 4;
          const COMMAND_COMPLETE   = 1 << 5;
          const BELL               = 1 << 6;
          const CONFIG_RELOADED    = 1 << 7;
          const KEY_EVENT          = 1 << 8;  // Activated in 28.5 (keybinding integration)
          const PASTE_REQUEST      = 1 << 9;  // Activated in 28.9 (smart paste Lua hook)
      }
  }
  ```

### 28.2.3 Callback registration and dispatch

**File(s):** `oriterm/src/lua/dispatch.rs`

- [ ] Implement `oriterm.on(event_name, callback)` in Lua:
  - Callbacks stored in Lua registry table keyed by event name (WezTerm pattern: `"oriterm-event-{name}"`)
  - Each key maps to a Lua table: `{[1]=handler1, [2]=handler2, ...}`
  - Registration updates the `CallbackFlags` bitset
  - Multiple callbacks per event type, FIFO dispatch order

- [ ] Implement `dispatch_event(engine, event) -> Vec<LuaCommand>`:
  - Check bitset first — if no callbacks for this event type, return empty vec immediately
  - Convert `LuaEvent` to Lua table (data-only, no references)
  - Iterate registered callbacks for this event type
  - Each callback wrapped in `pcall` — errors logged, execution continues to next handler
  - Callbacks can return a table of commands (optional) — collected into `Vec<LuaCommand>`
  - Reuse command buffer across calls (clear + push, no alloc per dispatch)

- [ ] Error isolation: single broken callback does not block other handlers:
  ```lua
  -- Dispatch pseudocode (Rust-side):
  for handler in handlers {
      match pcall(handler, event_data) {
          Ok(result) => collect_commands(result),
          Err(e) => log::warn!("lua: callback error for '{}': {}", event_name, e),
      }
  }
  ```

### 28.2.4 Hook dispatch into mux pump

**File(s):** `oriterm/src/app/mux_pump/mod.rs`

The key integration point. Lua dispatch fires BEFORE native handlers for events with registered callbacks. If Lua is absent, crashed, or returns no commands, the native handler runs as fallback. This enables both replacement (28.4 — Lua provides the tab title) and augmentation (Lua adds a notification on top of native output handling).

- [ ] In `handle_mux_notification()`, dispatch to Lua FIRST, then fall back to native. **Borrow checker pattern**: (1) extract event data from `&notification` into local variables (no self borrow), (2) build `LuaEvent` from locals, (3) take `&mut self.script_engine` to dispatch (returns `Vec<LuaCommand>`), (4) collect commands into a local `Vec`, (5) iterate local `Vec` calling `self.execute_lua_command()`. This is the same borrow dance used in `try_dispatch_mark_mode` (keyboard_input/mod.rs:167-238).
  ```rust
  fn handle_mux_notification(&mut self, notification: MuxNotification) {
      // Step 0: Extract context that LuaEvent needs (is_focused, etc.)
      // BEFORE any engine borrows. These are cheap reads.
      let focused_pane = self.active_pane_id();

      // Step 1: Extract Lua event data into locals BEFORE borrowing engine.
      // LuaEvent::from_notification is a pure function — takes &notification
      // and pre-extracted context, NOT &self.
      let lua_event = self.script_engine.as_ref().and_then(|engine| {
          if engine.has_callbacks_for(&notification) {
              Some(LuaEvent::from_notification(&notification, focused_pane))
          } else {
              None
          }
      });

      // Step 2: Dispatch to Lua, collecting commands into a local Vec.
      // CRITICAL borrow checker pattern: engine.dispatch_event() borrows
      // &mut self.script_engine. We must collect commands into a local Vec
      // and DROP the engine borrow BEFORE calling self.execute_lua_command().
      let lua_commands = if let Some(event) = &lua_event {
          if let Some(engine) = &mut self.script_engine {
              engine.dispatch_event(event) // Returns Vec<LuaCommand>
          } else {
              Vec::new()
          }
      } else {
          Vec::new()
      };
      // Engine borrow is now dropped. Safe to call &mut self methods.
      let lua_handled = !lua_commands.is_empty();
      for cmd in lua_commands {
          self.execute_lua_command(cmd);
      }
      // Step 3: Native handler — always runs for engine semantics.
      // Variable `lua_handled` controls whether policy logic runs.

      // Native handler (Rust fallback) — always runs for augmentation
      // events, skipped for replacement events when Lua handled them.
      // IMPORTANT: No notification is a pure "replacement" event. Even Bell
      // and CommandComplete have engine-level side effects (set_bell(),
      // mark_pane_window_dirty()) that must always run. The correct pattern
      // is: native handler always runs for engine semantics, but the POLICY
      // portion (ring_bell on tab bar, send OS notification) is conditional
      // on Lua NOT having handled it.
      //
      // Implementation: split native handlers into engine + policy parts.
      // Engine part always runs. Policy part runs only when !lua_handled.
      // Example for Bell:
      //   Engine (always): mux.set_bell(id), mark_pane_window_dirty(id)
      //   Policy (fallback): tab_bar.ring_bell(), mark_unseen
      // Example for CommandComplete:
      //   Engine (always): mark_pane_window_dirty(id)
      //   Policy (fallback): threshold check, bell pulse, OS notification

      // Native handler — engine semantics always run, policy conditional
      match notification {
          MuxNotification::PaneOutput(id) => { /* existing logic (all engine) */ }
          MuxNotification::PaneBell(id) => {
              // Engine (always): update bell state, dirty window
              mux.set_bell(id);
              self.mark_pane_window_dirty(id);
              // Policy (only when Lua didn't handle):
              if !lua_handled {
                  // ring_bell on tab bar, mark unseen — existing logic
              }
          }
          MuxNotification::CommandComplete { pane_id, duration } => {
              // Engine (always): mark window dirty
              self.mark_pane_window_dirty(pane_id);
              // Policy (only when Lua didn't handle):
              if !lua_handled {
                  self.handle_command_complete(pane_id, duration);
              }
          }
          // ... all other match arms unchanged (no policy split needed) ...
      }
  }
  ```
  This ordering means: if Lua is broken or absent, all native behaviors still work (degraded mode). If Lua has callbacks, it can replace policy decisions (notifications, bell presentation) while engine-level state updates (set_bell, mark_dirty) always run. For augmentation events (output, title change), Lua commands execute additionally after native handling.

- [ ] Create `oriterm/src/app/lua_commands.rs` (new file) for `execute_lua_command()`. Do NOT add to `mux_pump/mod.rs` (258 lines) or `app/mod.rs` (493 lines, at limit). Add `mod lua_commands;` to `app/mod.rs`.
- [ ] Implement `execute_lua_command()` on App — match on `LuaCommand` variants, execute each:
  ```rust
  fn execute_lua_command(&mut self, cmd: LuaCommand) {
      match cmd {
          LuaCommand::WriteToPane { pane_id, text } => {
              self.write_pane_input(pane_id, text.as_bytes());
          }
          LuaCommand::NewTab { cwd } => {
              if let Some(win) = self.active_window {
                  // TODO(28.3): new_tab_in_window() currently uses config
                  // default CWD. Add cwd parameter support when implementing.
                  self.new_tab_in_window(win);
              }
          }
          LuaCommand::SendNotification { title, body } => {
              crate::platform::notify::send(&title, &body);
          }
          LuaCommand::RingBell { pane_id } => {
              if let Some(idx) = self.tab_index_for_pane(pane_id) {
                  if let Some(ctx) = self.focused_ctx_mut() {
                      ctx.tab_bar.ring_bell(idx, std::time::Instant::now());
                  }
              }
          }
          LuaCommand::Log { level, message } => {
              match level {
                  LogLevel::Info => log::info!("lua: {message}"),
                  LogLevel::Warn => log::warn!("lua: {message}"),
                  LogLevel::Error => log::error!("lua: {message}"),
              }
          }
          // ... all other command variants
      }
  }
  ```

### 28.2.5 Tests

**File(s):** `oriterm/src/lua/tests.rs` (extend)

**Matrix dimensions:**
- **Event types**: all 8 LuaEvent variants
- **Callback patterns**: no callbacks, single callback, multiple callbacks, error in callback, callback returning commands
- **Bitset**: zero flags (skip dispatch), specific flags set, all flags set
- **Command execution**: each LuaCommand variant

**Semantic pins:**
- `test_no_callbacks_zero_overhead` — with empty bitset, dispatch returns immediately (no Lua call)
- `test_callback_error_continues` — error in first callback, second callback still fires
- `test_commands_execute_after_callback` — mutation command executes after callback returns, not during

```
- [ ] test_callback_registration — oriterm.on("bell", fn) sets BELL flag in bitset
- [ ] test_no_callbacks_zero_overhead — empty bitset → dispatch returns empty vec without touching Lua
- [ ] test_single_callback_fires — register bell callback, dispatch bell event, callback called
- [ ] test_multiple_callbacks_fifo — register 3 callbacks for same event, all fire in order
- [ ] test_callback_error_continues — first callback errors, second still fires
- [ ] test_callback_returns_commands — callback returns {new_tab={}} → Vec<LuaCommand> contains NewTab
- [ ] test_lua_event_from_notification — each MuxNotification variant converts to correct LuaEvent
- [ ] test_execute_lua_command_write — LuaCommand::WriteToPane writes to PTY (via mux mock)
- [ ] test_execute_lua_command_notify — LuaCommand::SendNotification dispatches OS notification
- [ ] test_distinct_metadata_notifications — PaneTitleChanged fires title_changed callback, not cwd_changed
- [ ] test_bell_engine_always_runs — Lua handles Bell → set_bell() and mark_dirty() STILL run (engine part)
- [ ] test_bell_policy_skipped_when_lua_handled — Lua handles Bell with commands → tab_bar.ring_bell() does NOT run (policy replaced)
- [ ] test_bell_policy_runs_when_no_lua — no Lua callback → full native bell handler runs (engine + policy)
- [ ] test_command_complete_policy_lua_replaces — Lua handles CommandComplete → handle_command_complete() policy skipped
- [ ] test_augmentation_event_runs_native — Lua handles PaneOutput with commands → native output handler STILL runs
- [ ] test_metadata_change_always_syncs_tab_bar — Lua handles PaneTitleChanged → native sync_tab_bar_from_mux() STILL runs (tab title Lua is inside build_tab_entries, not dispatch)
- [ ] test_empty_commands_falls_through — Lua callback returns empty table → native handler runs fully (engine + policy)
```

- [ ] Verify: `timeout 150 cargo test -p oriterm -- lua`
- [ ] `./build-all.sh` green, `./clippy-all.sh` green

- [ ] **TPR checkpoint** — `/tpr-review` covering 28.2.1–28.2.5 implementation work

---

## 28.3 API Surface

**File(s):** `oriterm/src/lua/api.rs` (new), `oriterm/src/lua/api/` (directory if needed), `oriterm/src/lua/userdata.rs` (new)

**Goal:** The `oriterm.*` Lua namespace — typed API for querying terminal state and requesting mutations. UserData wrappers for PaneId/TabId provide zero-copy ID passing. Queries execute synchronously (immediate return). Mutations queue LuaCommands (executed after callback).

**Success Criteria:**
- [ ] `oriterm.active_tab()`, `oriterm.active_pane()`, `oriterm.cwd()`, `oriterm.title()`, `oriterm.grid_size()` return correct values from Lua
- [ ] `oriterm.new_tab()`, `oriterm.close_tab()`, `oriterm.send_text()`, `oriterm.split()` queue correct LuaCommands
- [ ] UserData wrappers for PaneId/TabId — Lua holds opaque IDs, Rust resolves on each method call
- [ ] `cargo test -p oriterm -- lua::api` passes
- [ ] Satisfies mission criterion 3 (API enables meaningful callbacks)

**Context:** WezTerm uses `MuxWindow(WindowId)` and `MuxPane(PaneId)` UserData wrappers — Lua holds an opaque ID, each method call resolves the ID to the actual resource via `Mux::try_get()`. If the resource is gone (pane closed), the method returns an error. This avoids lifetime issues — Lua never holds references to Rust structures, only IDs.

**Reference implementations:**
- **WezTerm** `lua-api-crates/mux/src/window.rs:8-22`: `MuxWindow(WindowId)` UserData with `resolve()` pattern
- **WezTerm** `lua-api-crates/mux/src/pane.rs`: `MuxPane(PaneId)` with methods like `get_title()`, `send_text()`

**Depends on:** Section 28.1 (ScriptEngine), Section 28.2 (LuaCommand).

### 28.3.1 UserData wrapper types

**File:** `oriterm/src/lua/userdata.rs`

- [ ] `LuaPaneId` — wraps `PaneId`, implements `mlua::UserData`:
  ```rust
  struct LuaPaneId(PaneId);

  impl UserData for LuaPaneId {
      fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
          // Queries (synchronous — read from snapshot)
          methods.add_method("id", |_, this, ()| Ok(this.0.as_u64()));
          methods.add_method("title", |lua, this, ()| { ... });
          methods.add_method("cwd", |lua, this, ()| { ... });
          methods.add_method("grid_size", |lua, this, ()| { ... });

          // Mutations (deferred — push to command buffer)
          methods.add_method("send_text", |lua, this, text: String| { ... });
          methods.add_method("close", |lua, this, ()| { ... });
      }
  }
  ```
- [ ] `LuaTabId` — wraps `TabId`, implements `UserData`
- [ ] Resolution pattern: queries call through a `LuaContext` reference stored in Lua app data, which holds read-only accessors to mux/session state

### 28.3.2 LuaContext host-bridge

**File:** `oriterm/src/lua/api.rs`

The host-bridge solves the borrow checker problem: Lua callbacks can't hold `&mut App`. Instead, before dispatching an event, Rust creates a `LuaContext` with read-only snapshots and a command accumulator. Lua methods read from the context and push mutations to the accumulator.

- [ ] `LuaContext` struct (uses `RefCell` for interior mutability — mlua UserData methods receive `&self`, commands need `&mut` access; safe because Lua VM is single-threaded):
  ```rust
  /// Read-only context provided to Lua during callback dispatch.
  /// Created fresh for each dispatch cycle, destroyed after.
  pub(crate) struct LuaContext {
      // Read-only state snapshots (populated before dispatch)
      active_pane_id: Option<PaneId>,
      active_tab_id: Option<TabId>,
      active_window_id: Option<WindowId>,
      /// Lightweight pane metadata extracted from MuxBackend snapshots.
      /// NOT the protocol PaneSnapshot — only title, cwd, grid size.
      pane_info: HashMap<PaneId, LuaPaneInfo>,
      // Command accumulator (mutations pushed here)
      commands: RefCell<Vec<LuaCommand>>,
  }
  ```
- [ ] Before each dispatch cycle, `App` creates `LuaContext` with current state snapshots
- [ ] After dispatch, `App` takes the accumulated commands and executes them

### 28.3.3 Register oriterm.* namespace

**File:** `oriterm/src/lua/api.rs`

- [ ] Register the `oriterm` global table with methods:
  ```lua
  -- State queries (synchronous, return immediately)
  oriterm.active_tab()          -- returns LuaTabId or nil
  oriterm.active_pane()         -- returns LuaPaneId or nil
  oriterm.pane(id)              -- returns LuaPaneId for given numeric ID
  oriterm.tab(id)               -- returns LuaTabId for given numeric ID

  -- Event registration
  oriterm.on(event_name, callback)  -- register callback for event

  -- Logging
  oriterm.log.info(msg)         -- log at INFO level
  oriterm.log.warn(msg)         -- log at WARN level
  oriterm.log.error(msg)        -- log at ERROR level

  -- User state persistence (survives hot-reload)
  oriterm.state                 -- table preserved across reloads

  -- Config access (read-only)
  oriterm.config                -- read-only table of TOML config values
  ```

- [ ] Action methods (queue LuaCommands):
  ```lua
  oriterm.new_tab(opts?)        -- {cwd="...", title="..."}
  oriterm.close_tab(tab_id?)    -- default: active tab
  oriterm.split(direction)      -- "right" or "down"
  oriterm.close_pane(pane_id?)  -- default: active pane
  oriterm.send_text(text)       -- write to active pane PTY
  oriterm.copy()                -- copy selection to clipboard
  oriterm.paste()               -- paste from clipboard
  oriterm.set_theme(name)       -- switch color scheme
  oriterm.reload_config()       -- reload config.toml
  oriterm.notify(title, body)   -- send OS notification
  ```

### 28.3.4 Tests

**File:** `oriterm/src/lua/tests.rs` (extend)

Write failing tests FIRST (TDD), then implement to pass. Tests use sibling `tests.rs` pattern.

**Matrix dimensions:**
- **Query methods**: active_tab, active_pane, pane title, pane cwd, grid_size — with and without active pane
- **Mutation methods**: new_tab, close_tab, send_text, split, close_pane, set_theme — each produces correct LuaCommand
- **UserData**: LuaPaneId methods, LuaTabId methods, resolution of closed pane (error)
- **State persistence**: oriterm.state table survives across simulated reloads

**Semantic pins:**
- `test_closed_pane_errors` — ONLY passes when UserData resolution correctly detects stale pane IDs
- `test_config_read_only` — ONLY passes when read-only metatable is correctly applied

```
- [ ] test_active_pane_returns_id — with active pane, returns LuaPaneId with correct ID
- [ ] test_active_pane_nil_when_none — no active pane, returns nil
- [ ] test_pane_title_query — pane:title() returns snapshot title
- [ ] test_pane_cwd_query — pane:cwd() returns snapshot CWD
- [ ] test_pane_grid_size — pane:grid_size() returns {cols, rows}
- [ ] test_send_text_queues_command — oriterm.send_text("hello") → LuaCommand::WriteToPane
- [ ] test_new_tab_queues_command — oriterm.new_tab() → LuaCommand::NewTab
- [ ] test_closed_pane_errors — pane:title() on closed pane → Lua error (not crash)
- [ ] test_state_table_persists — set oriterm.state.foo = 42, simulate reload, read back 42
- [ ] test_config_read_only — attempt to write oriterm.config.font = "x" → Lua error
```

- [ ] Verify all tests pass: `timeout 150 cargo test -p oriterm -- lua`
- [ ] Verify debug and release: `timeout 150 cargo test -p oriterm --release -- lua`
- [ ] `./build-all.sh` green, `./clippy-all.sh` green, `timeout 150 ./test-all.sh` green

---

## 28.4 Built-in Lua Behaviors

**File(s):** `oriterm/src/lua/builtins/` (new directory), `oriterm/src/lua/builtins/tab_title.lua` (embedded), `oriterm/src/lua/builtins/notifications.lua` (embedded), `oriterm/src/app/tab_management/mod.rs` (modify), `oriterm/src/app/mux_pump/mod.rs` (modify)

**Goal:** Move tab title formatting, notification policy, and shell event responses from hardcoded Rust into built-in Lua scripts that ship with ori_term. Parity tests verify Lua produces identical behavior to the current Rust implementation before cutover. Users override these by defining their own handlers.

**Success Criteria:**
- [ ] `format_tab_title()` Lua function produces identical output to current `build_tab_entries()` Rust logic for all inputs
- [ ] `on_command_complete()` Lua function makes identical decisions to current `handle_command_complete()` for all config/focus combinations
- [ ] `on_bell()` Lua function replicates current bell handling
- [ ] Parity tests compare Rust vs Lua output for 20+ input combinations — all match
- [ ] Current Rust logic becomes the fallback (runs when Lua is disabled or crashed)
- [ ] Satisfies mission criterion 2 (built-in tab title in Lua)

**Context:** This is the "eat your own dogfood" subsection. Moving real behaviors into Lua validates the entire architecture (event dispatch, command buffer, API surface, host bridge). If built-in behaviors can be cleanly expressed in Lua, user scripts certainly can.

The current tab title logic lives in `build_tab_entries()` at `oriterm/src/app/tab_management/mod.rs:407-459`. It reads snapshot title, checks override, extracts emoji icon, strips emoji from title, appends zoom indicator, checks unseen output. All of this becomes a Lua function.

The current notification logic lives in `handle_command_complete()` at `oriterm/src/app/mux_pump/mod.rs:143-192`. It reads config thresholds, checks focus state, decides bell/notification. This becomes a Lua function.

**Reference implementations:**
- **Neovim**: Built-in LSP client, treesitter integration, diagnostics — all Lua scripts that ship with Neovim. Users override by replacing functions.

**Depends on:** Section 28.1 (engine), 28.2 (events/dispatch), 28.3 (API).

### 28.4.1 Built-in tab title script

**File:** `oriterm/src/lua/builtins/tab_title.lua` (embedded via `include_str!`)

```lua
-- Built-in tab title formatter.
-- Users override by defining their own format_tab_title in init.lua.

function format_tab_title(ctx)
    -- ctx fields: title, icon_name, is_override, is_zoomed, has_unseen
    local title = ctx.title or ""

    -- Extract emoji icon from icon_name
    local icon = nil
    if ctx.icon_name and ctx.icon_name ~= "" then
        -- Extract first emoji character
        icon = extract_first_emoji(ctx.icon_name)
    end

    -- Strip leading emoji from title (only for OSC titles, not overrides)
    if not ctx.is_override and icon then
        local stripped = title:match("^" .. icon .. "%s*(.*)$")
        if stripped then title = stripped end
    end

    -- Append zoom indicator
    if ctx.is_zoomed then
        title = title .. " [Z]"
    end

    return {
        title = title,
        icon = icon,
        modified = ctx.has_unseen,
    }
end
```

- [ ] Write the Lua script matching current Rust logic exactly. Note: `extract_first_emoji()` must be provided as a Rust-to-Lua bridge function (wrapping `oriterm_ui::widgets::tab_bar::extract_emoji_icon`) or reimplemented in Lua. Register it in the built-in environment during engine setup.
- [ ] Embed via `include_str!("builtins/tab_title.lua")`
- [ ] Load and compile to bytecode at engine startup
- [ ] Modify `build_tab_entries()` to call Lua `format_tab_title()` when engine available, fall back to existing Rust logic when not. Migration pattern:
  1. Extract current title/icon/modified computation into a standalone `compute_tab_display_rust(...)` function (the fallback)
  2. Add `compute_tab_display_lua(engine, ctx) -> Option<TabDisplayResult>` that calls Lua's `format_tab_title()`
  3. Call site: `compute_tab_display_lua().unwrap_or_else(|| compute_tab_display_rust(...))`
  4. Both paths return the same `TabDisplayResult { title, icon, modified }` struct
- [ ] Parity tests: 20+ input combinations, Rust output == Lua output

### 28.4.2 Built-in notification script

**File:** `oriterm/src/lua/builtins/notifications.lua` (embedded)

```lua
-- Built-in notification handlers.
-- Users override by defining their own on_command_complete in init.lua.

function on_command_complete(ctx)
    -- ctx: pane_id, duration_secs, is_focused, config
    -- config: threshold_secs, mode ("never"/"unfocused"/"always"), bell_enabled
    if ctx.duration_secs < ctx.config.threshold_secs then return end
    if ctx.config.mode == "never" then return end
    if ctx.config.mode == "unfocused" and ctx.is_focused then return end

    local commands = {}
    if ctx.config.bell_enabled then
        table.insert(commands, { ring_bell = { pane_id = ctx.pane_id } })
    end
    table.insert(commands, {
        notify = {
            title = ctx.title ~= "" and ctx.title or "Command finished",
            body = format_duration(ctx.duration_secs),
        }
    })
    return commands
end

function on_bell(ctx)
    -- ctx: pane_id, is_focused
    return {
        { ring_bell = { pane_id = ctx.pane_id } },
        { mark_unseen = not ctx.is_focused and { pane_id = ctx.pane_id } or nil },
    }
end

function format_duration(secs)
    if secs >= 3600 then
        return string.format("Completed in %dh %dm", secs // 3600, (secs % 3600) // 60)
    elseif secs >= 60 then
        return string.format("Completed in %dm %ds", secs // 60, secs % 60)
    else
        return string.format("Completed in %ds", secs)
    end
end
```

- [ ] Write Lua script matching current Rust logic exactly
- [ ] Embed via `include_str!`
- [ ] Modify `handle_command_complete()` to call Lua when engine available, fall back to Rust when not
- [ ] Modify bell handler similarly
- [ ] Parity tests: all config combinations (Never/Unfocused/Always × focused/unfocused × threshold met/unmet)

### 28.4.3 Parity tests

**File:** `oriterm/src/lua/tests.rs` (extend)

Parity tests run BOTH the Rust and Lua implementations with identical inputs and assert identical outputs. This validates the Lua scripts before cutover and serves as ongoing regression tests.

**Matrix dimensions:**
- **Tab title**: title with emoji, without emoji, override, zoomed, unseen, empty title, CJK title
- **Notifications**: Never/Unfocused/Always × focused/unfocused × duration above/below threshold × bell enabled/disabled
- **Bell**: focused/unfocused

```
- [ ] test_parity_tab_title_basic — "bash" → same TabEntry from both Rust and Lua
- [ ] test_parity_tab_title_emoji — "🐍 python" → emoji extracted, title stripped, same result
- [ ] test_parity_tab_title_override — user override "My Tab" → override used, same result
- [ ] test_parity_tab_title_zoomed — zoomed pane → " [Z]" appended, same result
- [ ] test_parity_tab_title_unseen — unseen output → modified=true, same result
- [ ] test_parity_notification_never — mode=Never → no notification from both
- [ ] test_parity_notification_unfocused_focused — mode=Unfocused, is_focused → no notification
- [ ] test_parity_notification_unfocused_bg — mode=Unfocused, not focused → notification from both
- [ ] test_parity_notification_always — mode=Always → notification from both
- [ ] test_parity_notification_below_threshold — duration < threshold → no notification from both
- [ ] test_parity_bell_focused — bell on focused pane → ring_bell from both
- [ ] test_parity_bell_unfocused — bell on unfocused pane → ring_bell + mark_unseen from both
- [ ] test_parity_duration_format — 12s, 90s, 3700s → identical format strings
```

- [ ] `./build-all.sh` green, `./clippy-all.sh` green, `timeout 150 ./test-all.sh` green

- [ ] **TPR checkpoint** — `/tpr-review` covering 28.3–28.4 implementation work

---

## 28.5 Keybinding Integration

**File(s):** `oriterm/src/lua/keybindings.rs` (new), `oriterm/src/app/keyboard_input/mod.rs` (hook), `oriterm/src/app/keyboard_input/action_dispatch.rs` (extend)

**Goal:** Lua can intercept keystrokes after the keymap matches an action (post-match interception) and define custom actions. Lua sits between "keymap found an action" and "action executes" — it can inspect, veto, replace, or augment. Lua can also register custom action handlers for keystrokes that don't match any built-in binding.

**Success Criteria:**
- [ ] `oriterm.on("key", callback)` fires after keymap match, before action execution
- [ ] Lua callback receives `{action, key, modifiers, pane_id}` and returns `"pass"` (execute native), `"cancel"` (swallow), or a command table (replace)
- [ ] Lua can register fallback key handlers for unbound keystrokes
- [ ] Native keybinding dispatch unaffected when no Lua key callbacks registered
- [ ] Satisfies mission criterion 5 (keystroke-to-action < 100μs)

**Context:** The current dispatch priority in `handle_keyboard_input()` (`oriterm/src/app/keyboard_input/mod.rs:46-162`) is: tab drag escape → selection escape → IME suppression → tab editing → overlay → search → mark mode → keybinding lookup → PTY encoding. Lua inserts after keybinding lookup: if a binding was found, Lua can intercept before execution. If no binding was found, Lua gets a chance to handle the key before PTY encoding. The higher-priority handlers (overlay, search, mark mode) consume keys before Lua ever sees them — this is correct behavior.

This is approach (b) from the Codex consensus: post-match interception. Approach (a) (Lua defines the keymap) is deliberately out of scope — it collides with config validation, `show-keys`, and the `oriterm_ui` keymap system.

**Reference implementations:**
- **Neovim**: `vim.keymap.set()` — Lua defines key mappings that can call Lua functions. Post-match interception via `vim.on_key()`.

**Depends on:** Section 28.1 (engine), 28.2 (events/dispatch), 28.3 (API, LuaCommand).

### 28.5.1 Key event dispatch

**File:** `oriterm/src/lua/keybindings.rs`

- [ ] `engine.has_key_callbacks()` checks `CallbackFlags::KEY_EVENT` bit (set when `oriterm.on("key", fn)` is called in 28.2.3)
- [ ] Define key event data passed to Lua:
  ```rust
  pub(crate) struct LuaKeyEvent {
      pub key: String,           // Logical key name (e.g., "t", "Enter", "Escape")
      pub modifiers: String,     // "ctrl+shift", "alt", "" (comma-separated)
      pub action: Option<String>, // Matched action name, or None if unbound
      pub pane_id: Option<PaneId>,
  }
  ```

- [ ] Lua callback return values:
  - `nil` or `"pass"` → execute native action (or encode to PTY if unbound)
  - `"cancel"` → swallow the key entirely
  - `{commands}` table → execute LuaCommands instead of native action

### 28.5.2 Hook into keyboard_input

**File:** `oriterm/src/app/keyboard_input/mod.rs`

- [ ] **Pre-check**: `keyboard_input/mod.rs` is 396 lines. The Lua interception adds ~30 lines. If the total exceeds 450, extract the Lua key dispatch logic into a `keyboard_input/lua_dispatch.rs` submodule.
- [ ] After keybinding lookup (~line 148), before `execute_action()`:
  ```rust
  // Existing: keybinding lookup
  if let Some(action) = keybindings::find_binding(&self.bindings, &binding_key, mods) {
      // NEW: Lua key interception
      if let Some(engine) = &mut self.script_engine {
          if engine.has_key_callbacks() {
              let key_event = LuaKeyEvent {
                  key: format_key(&event.logical_key),
                  modifiers: format_mods(self.modifiers),
                  action: Some(action.name().to_owned()),
                  pane_id: self.active_pane_id(),
              };
              match engine.dispatch_key_event(&key_event) {
                  KeyResult::Pass => { /* continue to native execution */ }
                  KeyResult::Cancel => return,
                  KeyResult::Commands(cmds) => {
                      for cmd in cmds { self.execute_lua_command(cmd); }
                      return;
                  }
              }
          }
      }
      // Existing: execute native action
      let action = action.clone();
      if self.execute_action(&action) { return; }
  }

  // Existing: no binding found → encode to PTY
  // NEW: Lua fallback handler for unbound keys
  if let Some(engine) = &mut self.script_engine {
      if engine.has_key_callbacks() {
          let key_event = LuaKeyEvent {
              key: format_key(&event.logical_key),
              modifiers: format_mods(self.modifiers),
              action: None,
              pane_id: self.active_pane_id(),
          };
          if let KeyResult::Commands(cmds) = engine.dispatch_key_event(&key_event) {
              for cmd in cmds { self.execute_lua_command(cmd); }
              return;
          }
      }
  }
  self.encode_key_to_pty(event);
  ```

### 28.5.3 Tests

**File:** `oriterm/src/lua/tests.rs` (extend)

Write failing tests FIRST (TDD), then implement to pass.

**Matrix dimensions:**
- **Key events**: bound key (action found), unbound key (no action), modifier combinations
- **Lua responses**: pass, cancel, command replacement
- **No Lua callbacks**: native path unaffected (zero overhead via bitset check)

**Semantic pins:**
- `test_lua_cancel_swallows_key` — Lua returns "cancel" → key not sent to PTY
- `test_lua_replace_action` — Lua returns commands → native action not executed

```
- [ ] test_no_key_callbacks_native_path — no Lua key callbacks → existing dispatch unchanged
- [ ] test_key_callback_pass — Lua returns "pass" → native action executes normally
- [ ] test_key_callback_cancel — Lua returns "cancel" → key swallowed, nothing happens
- [ ] test_key_callback_replace — Lua returns {send_text="custom"} → custom text sent to PTY
- [ ] test_unbound_key_fallback — unbound key + Lua fallback → Lua handles it
- [ ] test_unbound_key_no_fallback — unbound key + no Lua callback → PTY encoding (normal)
- [ ] test_key_event_data_correct — callback receives correct key name, modifiers, action name, pane_id
```

- [ ] Verify: `timeout 150 cargo test -p oriterm -- lua`
- [ ] Verify debug and release: `timeout 150 cargo test -p oriterm --release -- lua`
- [ ] `./build-all.sh` green, `./clippy-all.sh` green, `timeout 150 ./test-all.sh` green

---

## 28.6 User Scripts & Hot-Reload

**File(s):** `oriterm/src/lua/loader.rs` (new), `oriterm/src/lua/reload.rs` (new), `oriterm/src/config/monitor/mod.rs` (extend watcher)

**Goal:** Load user scripts from `~/.config/oriterm/init.lua` and `~/.config/oriterm/scripts/*.lua`. Detect file changes and hot-reload without terminal restart. Reload semantics: clear user callbacks, preserve `oriterm.state` table, re-execute scripts.

**Success Criteria:**
- [ ] `init.lua` loaded at startup from config path, executed in sandboxed environment
- [ ] All `.lua` files in `scripts/` directory auto-loaded after `init.lua`
- [ ] File watcher detects changes to `init.lua` or `scripts/*.lua`
- [ ] Hot-reload clears user callbacks, preserves `oriterm.state`, re-executes
- [ ] Malformed Lua script: error logged, terminal continues with built-in behaviors
- [ ] Satisfies mission criterion 6 (hot-reload)

**Context:** Config monitoring already exists at `oriterm/src/config/monitor/mod.rs:34-147`. It watches the config directory for `.toml` and theme file changes, debounces 200ms, sends `TermEvent::ConfigReload`. The script watcher follows the same pattern: watch the same directory for `.lua` changes, send a new `TermEvent::ScriptReload` variant.

Hot-reload semantics matter. Re-executing `init.lua` in an existing Lua state would accumulate callbacks (each reload adds more). Instead: save `oriterm.state`, clear ALL user callbacks from the registry, reload scripts, restore `oriterm.state`. This gives users a clean slate for callbacks while preserving explicit runtime state.

**Depends on:** Section 28.1 (engine), 28.2 (events), 28.3 (API).

### 28.6.1 Script loader

**File:** `oriterm/src/lua/loader.rs`

- [ ] `load_user_scripts(engine, config) -> LuaResult<()>`:
  1. Load `init.lua` from config path (if exists)
  2. Scan `scripts/` directory for `*.lua` files
  3. Sort by filename (deterministic load order)
  4. For each script: compile to bytecode, cache, execute in sandboxed environment
  5. Errors: log and continue (don't abort on single broken script)

- [ ] `load_builtin_scripts(engine) -> LuaResult<()>`:
  1. Load embedded scripts via `include_str!`
  2. Compile to bytecode, cache
  3. Execute in built-in environment (full stdlib)
  4. Errors: these are bugs — log::error and continue, but this should be caught in tests

### 28.6.2 Hot-reload mechanism

**File:** `oriterm/src/lua/reload.rs`

- [ ] `reload_scripts(engine) -> LuaResult<()>`:
  1. Save `oriterm.state` table from Lua registry
  2. Clear all user callback registrations
  3. Reset `CallbackFlags` bitset
  4. Re-execute built-in scripts (refresh default behaviors)
  5. Re-execute user scripts (override with user's handlers)
  6. Restore `oriterm.state` table
  7. Recompute `CallbackFlags` from newly registered callbacks

- [ ] Add `TermEvent::ScriptReload` variant to `oriterm/src/event.rs` (after `ConfigReload`, ~line 15):
  ```rust
  /// The Lua script watcher detected a `.lua` file change.
  ScriptReload,
  ```
- [ ] Handle in `user_event()` in `oriterm/src/app/event_loop.rs` (~line 338, after `ConfigReload` arm):
  ```rust
  TermEvent::ScriptReload => {
      if let Some(engine) = &mut self.script_engine {
          if let Err(e) = engine.reload_scripts(&self.config.lua) {
              log::error!("lua: script reload failed: {e}");
          }
      }
  }
  ```

### 28.6.3 Extend config monitor for Lua files

**File:** `oriterm/src/config/monitor/mod.rs`

- [ ] Extend the existing `ConfigMonitor` watch loop to also detect `.lua` file changes:
  - Watch the config directory (already watched) for `.lua` files
  - Watch the `scripts/` subdirectory (new) for `.lua` files
  - On `.lua` change: send `TermEvent::ScriptReload` (separate from `TermEvent::ConfigReload`)
  - Same 200ms debounce as config changes

### 28.6.4 Tests

**File:** `oriterm/src/lua/tests.rs` (extend)

Write failing tests FIRST (TDD), then implement to pass.

**Matrix dimensions:**
- **Loading**: init.lua exists, doesn't exist, syntax error, runtime error, scripts/ with multiple files
- **Reload**: callbacks cleared, state preserved, new callbacks registered, error during reload
- **File watcher**: .lua change detected, .toml change doesn't trigger script reload

**Semantic pins:**
- `test_reload_clears_callbacks` — ONLY passes when reload correctly clears the callback registry
- `test_reload_preserves_state` — ONLY passes when state table is saved/restored across reload boundary

```
- [ ] test_load_init_lua — init.lua with oriterm.on("bell", fn) → callback registered
- [ ] test_load_missing_init_lua — no init.lua file → no error, built-in behaviors active
- [ ] test_load_syntax_error — init.lua with syntax error → error logged, terminal functional
- [ ] test_load_scripts_dir — 3 files in scripts/ → all loaded in alphabetical order
- [ ] test_reload_clears_callbacks — register callback, reload, callback gone
- [ ] test_reload_preserves_state — set oriterm.state.foo=42, reload, oriterm.state.foo==42
- [ ] test_reload_error_keeps_old — reload fails → previous state intact, error logged
```

- [ ] Verify: `timeout 150 cargo test -p oriterm -- lua`
- [ ] Verify debug and release: `timeout 150 cargo test -p oriterm --release -- lua`
- [ ] `./build-all.sh` green, `./clippy-all.sh` green, `timeout 150 ./test-all.sh` green

- [ ] **TPR checkpoint** — `/tpr-review` covering 28.5–28.6 implementation work

---

## 28.7 Lua Verification & Security

**File(s):** `oriterm/src/lua/tests.rs` (comprehensive test matrix), security test scripts

**Goal:** Comprehensive verification that the Lua integration is correct, performant, secure, and cross-platform. Parity tests for all migrated behaviors, performance benchmarks, security fuzzing, and degraded mode validation.

**Success Criteria:**
- [ ] Full test matrix covers all LuaEvent × callback pattern combinations
- [ ] Performance: dispatch per pump < 0.25ms typical measured in test
- [ ] Security: 10+ escape attempt tests all blocked by sandbox
- [ ] Degraded mode: Lua crash → Rust fallbacks take over seamlessly
- [ ] Cross-platform: tests pass on all three targets (`./build-all.sh` green)
- [ ] Satisfies mission criteria 5 (performance), 7 (degraded mode), 8 (test green)

**Depends on:** All Lua subsections (28.1–28.6).

### 28.7.1 Test Matrix

Build a comprehensive test matrix covering every feature through the Lua pipeline.

- [ ] **Event dispatch matrix** (8 event types × 4 callback patterns):
  - Each LuaEvent variant: no callback, single callback, error callback, command-returning callback
  - Verify bitset optimization: zero callbacks = zero Lua calls

- [ ] **API surface matrix** (15+ API methods × valid/invalid inputs):
  - Each oriterm.* method: valid args, wrong type, nil pane, closed pane
  - UserData resolution: valid ID, invalid ID, stale ID

- [ ] **Built-in behavior parity** (20+ input combinations):
  - Tab title: all combinations from 28.4.3
  - Notifications: all combinations from 28.4.3
  - Format matching: duration strings, emoji extraction, zoom indicators

- [ ] **Keybinding matrix** (bound/unbound × Lua response × modifier combos):
  - From 28.5.3

### 28.7.2 Performance Validation

- [ ] **Dispatch latency**: Measure time from notification arrival to command execution. Budget: < 0.25ms typical, < 1ms worst-case.
  ```rust
  #[test]
  fn test_dispatch_latency() {
      let engine = ScriptEngine::new(&default_config()).unwrap();
      // Register a simple callback
      engine.exec("oriterm.on('bell', function(ctx) return {} end)").unwrap();

      let start = Instant::now();
      for _ in 0..1000 {
          engine.dispatch_event(&LuaEvent::Bell { pane_id: PaneId::new(1), is_focused: true });
      }
      let avg = start.elapsed() / 1000;
      assert!(avg < Duration::from_micros(250), "dispatch too slow: {avg:?}");
  }
  ```

- [ ] **Zero idle overhead**: With no callbacks registered, dispatch is a bitset check only.
  ```rust
  #[test]
  fn test_zero_idle_overhead() {
      let engine = ScriptEngine::new(&default_config()).unwrap();
      // No callbacks registered
      let start = Instant::now();
      for _ in 0..100_000 {
          engine.dispatch_event(&LuaEvent::Bell { pane_id: PaneId::new(1), is_focused: true });
      }
      let avg = start.elapsed() / 100_000;
      assert!(avg < Duration::from_nanos(100), "idle overhead too high: {avg:?}");
  }
  ```

- [ ] **Allocation check**: Verify no allocations in dispatch hot path (reused buffers).

### 28.7.3 Security Validation

- [ ] **Sandbox escape tests** (must ALL fail from user environment):
  ```lua
  io.open("/etc/passwd", "r")
  os.execute("rm -rf /")
  debug.getinfo(1)
  require("socket")
  load("os.execute('ls')")()
  dofile("/etc/passwd")
  loadfile("/etc/passwd")
  package.loadlib("/usr/lib/libm.so", "sin")
  rawget(_G, "io")
  getmetatable("").__index = function() os.execute("ls") end
  ```
  Each must produce a Lua error, not a process crash or successful execution.

- [ ] **Memory limit tests**: Allocate above limit → Lua error, VM recoverable.
- [ ] **Instruction limit tests**: Infinite loop → killed by hook, VM recoverable.
- [ ] **Metatable tampering**: Attempt to modify string metatable → blocked.

### 28.7.4 Degraded Mode

- [ ] **Lua crash recovery**: Simulate OOM, instruction abort, unrecoverable error. Verify:
  - Terminal continues rendering
  - Tab titles fall back to Rust formatting
  - Bell/notification fall back to Rust handlers
  - Keybinding dispatch falls back to native
  - Log message indicates degraded mode
  - Manual script reload (`oriterm.reload_config()` or config file change) attempts recovery

### 28.7.5 Build & Verify

- [ ] `./build-all.sh` green (all platforms including `x86_64-pc-windows-gnu`)
- [ ] `./clippy-all.sh` green (no new warnings from Lua integration)
- [ ] `timeout 150 ./test-all.sh` green (all tests pass)
- [ ] Architecture tests pass if any exist for event dispatch

### 28.7.6 Documentation

- [ ] Update CLAUDE.md with Lua module paths and patterns
- [ ] Update `oriterm` crate ownership section in `.claude/rules/crate-boundaries.md` to include Lua scripting
- [ ] Add Lua scripting to the "Key Paths" section of CLAUDE.md:
  ```
  **oriterm/src/lua/**: Lua 5.4 scripting engine — ScriptEngine lifecycle,
  event dispatch, API surface, built-in behaviors, user script loading
  ```

---

## 28.7.N Lua Completion Checklist

- [ ] All 28.1–28.6 items complete
- [ ] Lua scripting engine initializes at startup, lives for process lifetime
- [ ] Built-in tab title formatting runs in Lua (parity tests pass)
- [ ] All MuxNotification variants dispatch through Lua
- [ ] User scripts sandboxed — escape tests all blocked
- [ ] Keybinding interception works (pass/cancel/replace)
- [ ] Hot-reload detects file changes and re-executes scripts
- [ ] Degraded mode: Lua crash → Rust fallbacks seamless
- [ ] Performance: < 0.25ms typical dispatch, zero idle overhead
- [ ] Cross-platform: builds and tests pass on all 3 targets
- [ ] Plan annotation cleanup: all temporary scaffolding removed from `.rs` files
- [ ] All intermediate TPR checkpoint findings resolved
- [ ] **Plan sync:**
  - [ ] Sections 28.1–28.7 frontmatter `status` → `complete`
  - [ ] `00-overview.md` Quick Reference table updated
  - [ ] `index.md` section statuses updated
  - [ ] CLAUDE.md updated with Lua paths and patterns
- [ ] `/tpr-review` passed (final, full-section) — independent review clean
- [ ] `/impl-hygiene-review last commit` passed — hygiene review clean (MUST run AFTER `/tpr-review`)

**Exit Criteria:** ori_term's Lua 5.4 integration is complete: ScriptEngine initializes with safe stdlib, per-event bitset dispatch delivers notifications to Lua callbacks with < 0.25ms latency, built-in behaviors (tab title, notifications, bell) run in Lua with parity-tested equivalence to the replaced Rust code, keybinding post-match interception works, user scripts load from `init.lua` + `scripts/` with sandboxing (10+ escape attempts blocked) and hot-reload (file watcher + clear-and-re-exec semantics), and Lua failure triggers seamless degraded mode with Rust fallbacks. `./test-all.sh` passes with 0 regressions.

---

## 28.8 Custom Shaders

Post-processing WGSL fragment shaders for visual effects.

**File:** `oriterm/src/gpu/pipeline/mod.rs` (shader compilation), `oriterm/src/gpu/window_renderer/render.rs` (render passes, off-screen render target)

**Current pipeline:** Renders terminal content through compositor passes. Custom shaders add a post-processing pass.

- [ ] Config: `window.custom_shader = "/path/to/shader.wgsl"`
- [ ] Pipeline changes:
  1. Render terminal to an off-screen texture (render target) instead of directly to surface
  2. Run custom shader as a full-screen quad with the terminal texture as input
  3. Output to the surface
- [ ] Shader interface (WGSL):
  ```wgsl
  @group(0) @binding(0) var terminal_texture: texture_2d<f32>;
  @group(0) @binding(1) var terminal_sampler: sampler;

  struct Uniforms {
      resolution: vec2<f32>,    // window size in pixels
      time: f32,                // elapsed seconds (for animations)
      cursor_pos: vec2<f32>,    // cursor position in pixels
  }
  @group(0) @binding(2) var<uniform> uniforms: Uniforms;

  @fragment
  fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
      // Default: passthrough
      return textureSample(terminal_texture, terminal_sampler, uv);
  }
  ```
- [ ] Shader uniforms updated each frame:
  - [ ] `time`: `Instant::elapsed().as_secs_f32()` (modulo to prevent precision loss)
  - [ ] `resolution`: window physical size
  - [ ] `cursor_pos`: cursor cell position in pixels
- [ ] Built-in example shaders:
  - [ ] `crt.wgsl` — CRT effect (scanlines, curvature, vignette, bloom)
  - [ ] `grayscale.wgsl` — desaturate terminal output
  - [ ] `invert.wgsl` — invert colors
- [ ] Hot-reload: detect shader file changes, recompile pipeline
  - [ ] On compile error: log error, fall back to passthrough (no shader)
  - [ ] Show compilation errors via `log::error!` and optional status bar message
- [ ] Performance: custom shader adds one texture sample per pixel per frame
  - [ ] For simple shaders this is negligible
  - [ ] Complex shaders (blur, multi-pass) may need frame rate consideration

**Tests:**

Write failing tests FIRST. Tests in `oriterm/src/gpu/pipeline/tests.rs` (sibling pattern).

**Matrix dimensions:**
- **Shader compilation**: valid WGSL, syntax error, missing entry point, wrong bindings
- **Render path**: no shader (passthrough), valid shader, shader removed mid-session
- **Uniforms**: time progresses, resolution matches window, cursor_pos tracks cursor
- **Hot-reload**: file change detected, compile error logged + fallback, fix error + re-detect

**Semantic pins:**
- `test_shader_compile_error_fallback` — ONLY passes when invalid shader triggers fallback instead of crash
- `test_offscreen_target_size` — ONLY passes when render target dimensions correctly track surface resize

- [ ] test_passthrough_shader_identity — passthrough shader produces identical output to no-shader rendering (use `render_frame_cached` for production path)
- [ ] test_shader_compile_error_fallback — invalid WGSL → log::error, render continues without shader
- [ ] test_hot_reload_detects_change — modify shader file → pipeline recompiled (same watcher as config)
- [ ] test_uniforms_update_per_frame — time, resolution, cursor_pos all change between frames
- [ ] test_offscreen_target_size — render target dimensions match surface dimensions after resize
- [ ] Cross-platform: WGSL is platform-independent, but verify the off-screen render target works on all three backends (Vulkan, Metal, DX12).
- [ ] `./build-all.sh` green, `./clippy-all.sh` green, `timeout 150 ./test-all.sh` green

---

## 28.9 Smart Paste

Intelligent paste behavior for safety and convenience.

**File:** `oriterm/src/app/` (paste handling), `oriterm/src/gpu/` (confirmation overlay)

**Lua integration:** When the Lua engine is available, fire a `PasteRequest` event before applying paste policy. Lua can inspect the paste content and return `"allow"`, `"cancel"`, or `{text = transformed_text}` to override. This lets users write custom paste transformations (e.g., strip ANSI from copied terminal output, auto-fix Windows paths to Unix paths). Native paste policy (multi-line warning, ESC stripping, size limit) runs as fallback when no Lua callback is registered.

- [ ] Multi-line paste warning:
  - [ ] If pasted text contains `\n` (newlines), show confirmation overlay
  - [ ] "You are about to paste N lines. Continue?"
  - [ ] Options: "Paste", "Paste as single line", "Cancel"
  - [ ] "Paste as single line": replace `\n` with spaces
  - [ ] Config: `behavior.warn_multiline_paste = true | false` (default: true)
  - [ ] Bypass: if bracketed paste mode is active, always paste (app handles it)
- [ ] Strip leading prompt characters:
  - [ ] Detect when pasted text starts with `$ `, `# `, `> `, `% `
  - [ ] Strip the prompt prefix (user likely copied from a tutorial/README)
  - [ ] Only strip from first line (not every line)
  - [ ] Config: `behavior.strip_paste_prompt = true | false` (default: false)
- [ ] Sanitize pasted text:
  - [ ] Strip ESC (0x1B) characters to prevent escape injection attacks
  - [ ] Only when bracketed paste mode is NOT active (app handles raw paste)
  - [ ] Log warning when ESC characters are stripped
- [ ] Large paste warning:
  - [ ] If paste > configurable threshold (default 1MB), show warning
  - [ ] "You are about to paste X.X MB. This may be slow. Continue?"
  - [ ] Config: `behavior.large_paste_threshold = 1048576` (bytes, 0 = disabled)
- [ ] Confirmation overlay rendering:
  - [ ] Note: `ConfirmationKind::Paste` and `TermEvent::OpenConfirmation` already exist in `oriterm/src/event.rs` (OS-level dialog). Evaluate whether to reuse this existing dialog or build an in-window overlay instead (command palette style). In-window overlay is faster and less jarring; OS dialog is already wired up.
  - [ ] Show preview of first 3-5 lines of paste content
  - [ ] Keyboard: Enter = confirm, Escape = cancel

**Tests:**

Write failing tests FIRST. Paste logic tests in `oriterm/src/app/paste/tests.rs` or wherever paste handling lives (currently in `clipboard_ops.rs`; if it grows, extract to `paste/` submodule).

**Matrix dimensions:**
- **Line count**: single-line, multi-line (2-5), very large (>1000 lines)
- **Bracketed paste**: on/off
- **Config toggles**: warn_multiline_paste on/off, strip_paste_prompt on/off
- **Content**: clean text, text with ESC bytes, text with prompt prefix, text > 1MB
- **Lua hook**: PasteRequest event fires when Lua callback registered; Lua can allow/cancel/transform

**Semantic pins:**
- `test_esc_stripped_outside_bracketed` — ONLY passes when ESC sanitization is correctly gated on bracketed paste mode
- `test_bracketed_paste_bypasses_warning` — ONLY passes when bracketed paste mode correctly skips the multi-line warning

- [ ] test_single_line_passthrough — single-line paste, no warning
- [ ] test_multi_line_triggers_warning — multi-line paste → confirmation overlay
- [ ] test_paste_as_single_line — "Paste as single line" replaces newlines with spaces
- [ ] test_bracketed_paste_bypasses_warning — bracketed paste mode → no warning
- [ ] test_strip_prompt_prefix — leading `$ ` stripped from first line when config enabled
- [ ] test_esc_stripped_outside_bracketed — ESC characters stripped when not in bracketed paste mode
- [ ] test_esc_preserved_in_bracketed — ESC characters kept when in bracketed paste mode
- [ ] test_large_paste_warning — paste >1MB triggers size warning
- [ ] test_escape_cancels_paste — Escape key cancels paste overlay
- [ ] test_lua_paste_hook — Lua `on("paste_request", fn)` callback fires before native paste policy; Lua returns `"cancel"` → paste blocked
- [ ] `./build-all.sh` green, `./clippy-all.sh` green, `timeout 150 ./test-all.sh` green

---

## 28.10 Undo Close Tab

Restore accidentally closed tabs.

**File:** `oriterm/src/app/` (tab close + reopen)

- [ ] Closed-tab stack:
  - [ ] `ClosedTabInfo` struct lives in `oriterm/src/session/closed_tabs.rs` (new file)
  - [ ] `closed_tabs: Vec<ClosedTabInfo>` as a field on `SessionRegistry` (not `App` — session-level state). Add `push_closed_tab()`, `pop_closed_tab()` methods to `SessionRegistry`.
  - [ ] `ClosedTabInfo`:
    ```rust
    struct ClosedTabInfo {
        title: String,
        cwd: Option<String>,
        /// Last N lines of scrollback, stored as plain text (not Row structs)
        /// to avoid holding Grid/Cell allocations. Max 1000 lines.
        scrollback_text: Vec<String>,
        closed_at: Instant,
    }
    ```
  - [ ] When closing a tab: extract last 1000 lines as plain text + metadata, push to stack
  - [ ] Do NOT store `Row` structs (would retain Cell allocations and CellExtra Arcs)
  - [ ] Do NOT store the running process (can't resurrect a PTY)
  - [ ] Do NOT store the grid — just scrollback text for reference
- [ ] Reopen: `Ctrl+Shift+T`:
  - [ ] Pop most recent `ClosedTabInfo` from stack
  - [ ] Create new tab with stored CWD (if available)
  - [ ] Set tab title to stored title
  - [ ] Optionally: prepopulate scrollback with stored text lines (read-only history)
  - [ ] New shell starts fresh — no process restoration
- [ ] UI feedback:
  - [ ] Brief toast/overlay: "Tab closed — Ctrl+Shift+T to undo" (3 seconds)
  - [ ] Or: show in command palette as "Reopen Closed Tab (N available)"
- [ ] Stack management:
  - [ ] FIFO with max 10 entries
  - [ ] 1000 lines max per entry (already enforced in ClosedTabInfo)
- [ ] Edge cases:
  - [ ] If no CWD stored, open in home directory
  - [ ] If stack is empty, Ctrl+Shift+T does nothing (or shows message)
  - [ ] Closing all tabs + closing window: stack survives if app has other windows

**Tests:**

Write failing tests FIRST. Tests in `oriterm/src/session/closed_tabs/tests.rs` (sibling pattern for `ClosedTabInfo` unit tests) and `oriterm/src/app/tab_management/tests.rs` (integration with tab close action).

**Matrix dimensions:**
- **Stack operations**: push, pop, capacity limit (FIFO eviction), empty stack
- **Data capture**: title captured, CWD captured, scrollback text capped at 1000 lines
- **Reopen behavior**: CWD used, fallback to home, title restored

**Semantic pins:**
- `test_stack_capacity_10` — ONLY passes when 11th entry correctly evicts the oldest (FIFO, not LIFO)
- `test_scrollback_text_cap` — ONLY passes when scrollback is correctly truncated to 1000 lines

- [ ] test_close_tab_pushes_info — closing a tab pushes ClosedTabInfo to stack
- [ ] test_reopen_pops_stack — Ctrl+Shift+T pops most recent entry, creates new tab
- [ ] test_reopened_tab_cwd — reopened tab starts shell in stored CWD
- [ ] test_reopened_tab_title — reopened tab has stored title
- [ ] test_stack_capacity_10 — 11th close evicts oldest entry (FIFO)
- [ ] test_empty_stack_noop — Ctrl+Shift+T with empty stack does nothing
- [ ] test_scrollback_text_stored — last 1000 lines stored as plain text, not Row structs
- [ ] test_scrollback_text_cap — scrollback text capped at 1000 lines even if scrollback is larger
- [ ] test_no_cwd_uses_home — if CWD not available, reopened tab uses home directory
- [ ] `./build-all.sh` green, `./clippy-all.sh` green, `timeout 150 ./test-all.sh` green

---

## 28.11 Session Recording + Playback

Record terminal sessions for replay, debugging, and demos.

**File:** `oriterm/src/recording/mod.rs` (new module — add `pub(crate) mod recording;` to `oriterm/src/lib.rs`). Each file under 500 lines. Recording state is per-pane — stored on the mux side or accessed via pane-level APIs.

- [ ] Recording format:
  - [ ] Use asciicast v2 format (JSON lines) for ecosystem compatibility
  - [ ] Header: `{"version": 2, "width": 80, "height": 24, "timestamp": ..., "env": {...}}`
  - [ ] Events: `[time_offset, "o", "data"]` — time (float seconds), type, payload
  - [ ] Input events: `[time_offset, "i", "data"]` — optional, for recording typed input
- [ ] `oriterm record` subcommand:
  - [ ] `oriterm record -o session.cast` — record to file
  - [ ] `oriterm record` — record to default path (`~/.local/share/oriterm/recordings/`)
  - [ ] Auto-name: `recording-YYYY-MM-DD-HHMMSS.cast`
  - [ ] Recording indicator: subtle "REC" badge in tab bar (red dot)
- [ ] Recording engine:
  - [ ] Tee PTY output: duplicate all bytes from PTY reader to recording file
  - [ ] Timestamp each chunk relative to session start
  - [ ] Optional: record input events (keystrokes sent to PTY)
  - [ ] Flush periodically (every 1s) to prevent data loss on crash
- [ ] `oriterm play` subcommand:
  - [ ] `oriterm play session.cast` — replay recording in a new terminal window
  - [ ] Playback at original speed (honor timestamps)
  - [ ] `--speed <factor>` — 2x, 0.5x playback speed
  - [ ] Pause/resume with spacebar
  - [ ] Seek with arrow keys (skip forward/backward 5s)
- [ ] Integration with running sessions:
  - [ ] `Ctrl+Shift+R` — toggle recording of current pane
  - [ ] Action: `ToggleRecording` in keybinding system
  - [ ] Recording state stored per pane
**Tests:**

Write failing tests FIRST. Tests in `oriterm/src/recording/tests.rs` (sibling pattern). Recording format is pure data — testable without GPU or PTY.

**Matrix dimensions:**
- **Format**: header fields correct, event types (o/i), timestamp precision
- **Recording lifecycle**: start, data arrives, stop, file closed
- **Playback**: original speed, 2x speed, 0.5x speed, pause/resume
- **Edge cases**: empty recording, very short recording (<1s), recording with only input events

**Semantic pins:**
- `test_timestamps_monotonic` — ONLY passes when recording engine correctly orders events by time
- `test_playback_speed_factor` — ONLY passes when playback engine correctly applies speed multiplier to inter-event delays

- [ ] test_asciicast_header_valid — recording header contains version=2, width, height, timestamp
- [ ] test_timestamps_monotonic — event timestamps are monotonically increasing
- [ ] test_output_events_format — output events use `["time", "o", "data"]` format
- [ ] test_playback_original_speed — playback at 1x reproduces original timing
- [ ] test_playback_speed_factor — 2x playback halves inter-event delay
- [ ] test_recording_flush — data flushed to file within 1s (no data loss on crash)
- [ ] Cross-platform: file I/O is platform-independent, but the PTY tee mechanism differs (Unix: dup fd; Windows: explicit copy in read loop).
- [ ] `./build-all.sh` green, `./clippy-all.sh` green, `timeout 150 ./test-all.sh` green

---

## 28.12 Workspaces

Named groups of tabs/panes with layout persistence and quick switching.

**File:** `oriterm/src/session/workspaces.rs` (new — workspace model), `oriterm/src/app/workspaces.rs` (new — workspace actions/commands)

**Reference:** WezTerm workspaces (`SwitchToWorkspace`, `SwitchWorkspaceRelative`)

- [ ] `Workspace` concept:
  - [ ] Named collection of tabs within a window
  - [ ] Each workspace has its own tab list and active tab
  - [ ] Switching workspaces swaps the visible tab set
  - [ ] Think of it like virtual desktops, but for terminal tabs
- [ ] `WorkspaceId(u64)` newtype
- [ ] `Workspace` struct:
  - [ ] `id: WorkspaceId`
  - [ ] `name: String` — user-visible name (e.g., "default", "project-x", "devops")
  - [ ] `tabs: Vec<TabId>` — tab order within this workspace
  - [ ] `active_tab: usize` — index into `tabs`
- [ ] Workspace management:
  - [ ] Default workspace: "default" — all tabs start here
  - [ ] `SwitchToWorkspace(name)` action — switch to named workspace (create if needed)
  - [ ] `SwitchWorkspaceRelative(offset)` action — cycle through workspaces
  - [ ] `RenameWorkspace(name)` action — rename current workspace
  - [ ] Moving tabs between workspaces
- [ ] Workspace presets via config/Lua:
  ```toml
  [[workspace]]
  name = "dev"
  tabs = [
    { title = "editor", cwd = "~/projects/myapp" },
    { title = "server", cwd = "~/projects/myapp", command = "cargo run" },
    { title = "tests", cwd = "~/projects/myapp" },
  ]
  ```
  - [ ] Lua: `oriterm.create_workspace({ name = "dev", tabs = {...} })` (requires extending 28.3 API surface with workspace methods: `oriterm.create_workspace()`, `oriterm.switch_workspace()`, `oriterm.active_workspace()`)
- [ ] Workspace persistence:
  - [ ] Save workspace layouts as part of session persistence (Section 35)
  - [ ] Restore workspace names and tab assignments on session restore
- [ ] Keybindings:
  - [ ] `Ctrl+Shift+W` — workspace switcher (shows list)
  - [ ] `Ctrl+Shift+N` — new workspace (prompts for name)
**Tests:**

Write failing tests FIRST. `Workspace` and `WorkspaceId` live in `oriterm/src/session/workspaces.rs` — tests in `oriterm/src/session/workspaces/tests.rs` (sibling pattern). Workspace is a session concept (per crate boundaries: session model lives in `oriterm/src/session/`).

**Matrix dimensions:**
- **Lifecycle**: create, switch, rename, delete (cannot delete default)
- **Tab management**: tabs belong to workspace, move between workspaces, active tab per workspace
- **Config presets**: workspace preset creates tabs with specified titles/CWDs

**Semantic pins:**
- `test_switch_workspace_swaps_tabs` — ONLY passes when switching workspace correctly swaps the visible tab set
- `test_default_workspace_always_exists` — ONLY passes when default workspace deletion is rejected

- [ ] test_create_workspace — create workspace with name, verify ID and name
- [ ] test_switch_workspace_swaps_tabs — switching workspace changes visible tab set
- [ ] test_move_tab_between_workspaces — tab moves from workspace A to workspace B
- [ ] test_default_workspace_always_exists — default workspace created on init, cannot be deleted
- [ ] test_workspace_preset — config preset creates workspace with specified tabs
- [ ] test_workspace_id_newtype — `WorkspaceId(u64)` is a proper newtype (not bare u64)
- [ ] `./build-all.sh` green, `./clippy-all.sh` green, `timeout 150 ./test-all.sh` green

---

## 28.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers.
If unresolved findings exist here:
- section frontmatter `status` must be `in-progress`
- `third_party_review.status` must be `findings`

When all findings are triaged:
- accepted findings are integrated into the relevant implementation subsection(s)
- rejected findings are closed with rationale
- all items in this block are marked resolved
- `third_party_review.status` becomes `resolved` or `none`
-->

- None.

---

## 28.N Completion Checklist

- [ ] All 28.1–28.12 items complete
- [ ] Lua scripting engine loads and executes user scripts
- [ ] Scripts can react to events (title_changed, output, key, bell, cwd_changed, command_complete)
- [ ] Scripts can invoke actions (new_tab, split, set_theme, send_text, notify)
- [ ] Built-in behaviors (tab title, notifications, bell) run in Lua with parity-tested equivalence
- [ ] Keybinding post-match interception works (pass/cancel/replace)
- [ ] Hot-reload detects file changes and re-executes scripts
- [ ] Degraded mode: Lua crash → Rust fallbacks seamless
- [ ] User scripts sandboxed — 10+ escape attempts all blocked
- [ ] Performance: < 0.25ms dispatch, zero idle overhead, < 100μs keystroke-to-action
- [ ] Custom WGSL shaders render as post-processing pass
- [ ] Shader hot-reload works (edit file, see change)
- [ ] Multi-line paste shows confirmation dialog
- [ ] Paste stripping works for prompt characters
- [ ] Ctrl+Shift+T reopens last closed tab in stored CWD
- [ ] Undo-close stack holds last 10 tabs
- [ ] Session recording in asciicast v2 format works (record + playback)
- [ ] Named workspaces with tab grouping and quick switching
- [ ] All features documented and configurable
- [ ] Plan annotation cleanup: all temporary scaffolding removed from `.rs` files
- [ ] All intermediate TPR checkpoint findings resolved
- [ ] **Plan sync:**
  - [ ] All section frontmatter statuses updated
  - [ ] `00-overview.md` Quick Reference table updated for Section 28
  - [ ] `index.md` section statuses updated
  - [ ] CLAUDE.md updated with Lua paths, patterns, and module ownership
  - [ ] `.claude/rules/crate-boundaries.md` updated with Lua ownership
- [ ] `/tpr-review` passed (final, full-section) — independent Codex review found no critical or major issues (or all findings triaged)
- [ ] `/impl-hygiene-review last commit` passed — implementation hygiene review found no critical or major findings (or all findings triaged and fixed). MUST run AFTER `/tpr-review` is clean.

**Exit Criteria:** ori_term has a Lua 5.4 scripting layer where the terminal's own behaviors are Lua scripts that users can override. Tab title formatting, notification policy, bell handling, and shell event responses run in Lua with parity-tested equivalence to the replaced Rust code. Keybinding interception allows custom actions. User scripts are sandboxed with memory + instruction limits. Hot-reload works via file watcher. Performance is sub-millisecond. Plus: custom WGSL post-processing shaders, smart paste safety, undo-close-tab, session recording, and named workspaces. `./test-all.sh` passes with 0 regressions across all platforms.
