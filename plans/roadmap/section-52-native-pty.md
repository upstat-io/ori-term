---
section: 52
title: "Native PTY Layer"
status: not-started
reviewed: false
tier: 5
goal: "Replace portable-pty with direct platform syscalls (libc/windows-sys), gaining control over pipe creation, buffer sizes, and ConPTY configuration. Overlapped named pipes + passthrough flag on Windows. Sideloaded conpty.dll support. Eliminates ~15 transitive dependencies."
success_criteria:
  - "portable-pty does not appear in any Cargo.toml"
  - "crates/portable-pty/ directory removed"
  - "./build-all.sh green on all platforms"
  - "./clippy-all.sh green"
  - "./test-all.sh green — no regressions"
  - "Windows ConPTY uses overlapped named pipes"
  - "ConPTY passthrough flag (0x8) attempted at creation"
  - "Sideloaded conpty.dll loaded when present"
  - "Unix PTY handles EIO→EOF, setsid, TIOCSCTTY, signal reset, fd hygiene"
inspired_by:
  - "Alacritty tty/ module (alacritty_terminal/src/tty/ — 1601 lines, direct syscalls)"
  - "WezTerm portable-pty (crates/portable-pty/ — the code we're replacing)"
  - "Ghostty src/termio/Exec.zig (ConPTY with windows-sys equivalent)"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "52.1"
    title: "PtyCommand + Shared Types"
    status: not-started
  - id: "52.2"
    title: "Unix PTY"
    status: not-started
  - id: "52.3"
    title: "Windows ConPTY + Overlapped Pipes"
    status: not-started
  - id: "52.4"
    title: "Sideloaded conpty.dll"
    status: not-started
  - id: "52.5"
    title: "Migration"
    status: not-started
  - id: "52.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "52.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 52: Native PTY Layer

**Status:** Not Started
**Goal:** Replace `portable-pty` with direct platform syscalls via `libc` (Unix) and `windows-sys` (Windows) — both already workspace dependencies. This eliminates ~15 transitive dependencies, removes trait-object dispatch, and gives ori_term full control over PTY creation, pipe configuration, and ConPTY flags. On Windows, overlapped named pipes enable cancelable I/O and the ConPTY passthrough flag (`0x8`). Sideloaded `conpty.dll` support loads newer ConPTY builds with Sixel support. Section 53 (Raw Pipe Bypass) builds on the pipe infrastructure established here.

**Success Criteria:**

- [ ] `portable-pty` removed from all `Cargo.toml` files (satisfies mission criterion: dependency elimination)
- [ ] `crates/portable-pty/` directory deleted (satisfies mission criterion: clean removal)
- [ ] All existing vttest tests pass with the new PTY layer (satisfies: no regressions)
- [ ] Windows ConPTY creates overlapped named pipes via `CreateNamedPipeW` (satisfies: overlapped pipes)
- [ ] ConPTY passthrough flag `PSEUDOCONSOLE_PASSTHROUGH_MODE` (`0x8`) passed to `CreatePseudoConsole` (satisfies: passthrough attempt)
- [ ] Sideloaded `conpty.dll` loaded when found next to binary (satisfies: sideloaded DLL)
- [ ] Unix PTY: `stty size` inside spawned shell reports correct dimensions (satisfies: Unix correctness)
- [ ] `PtyCommand` uses `OsString`/`PathBuf` for all paths, args, and env values
- [ ] `PtyConfig.shell` and `PtyConfig.env` converted to `OsString`/`OsString` at the `PtyConfig` → `PtyCommand` boundary (currently `String`/`Vec<(String, String)>`)
- [ ] ConPTY drop ordering enforced: conout pipe handle must outlive `ConPty` to prevent deadlock in `ClosePseudoConsole`
- [ ] Windows ConPTY size test passes (`mode con` reports correct dimensions) — closes BUG-07-004

**Context:** `portable-pty` was adopted in Section 03 (Cross-Platform) as a quick path to PTY abstraction. It served its purpose but brings ~15 transitive deps (including unmaintained `winapi`, `nix`, `filedescriptor`, `downcast-rs`, `serial2`), uses trait objects with `downcast-rs` for runtime dispatch we never need (one implementation per platform), and its `CommandBuilder` duplicates env management we already handle in `spawn.rs`. Most critically, its synchronous anonymous pipes on Windows prevent overlapped I/O and may be why the ConPTY passthrough flag broke output.

**Important migration detail:** `PtyConfig` currently uses `String` for `shell` and `Vec<(String, String)>` for `env` (see `oriterm_mux/src/pty/spawn.rs:86-98`). The new `PtyCommand` uses `OsString` throughout. The `spawn_pty()` function must convert `PtyConfig` strings to `OsString` at the boundary — this is the only conversion point.

**Reference implementations:**
- **Alacritty** `alacritty_terminal/src/tty/unix.rs`: Direct `openpty()` via `rustix_openpty`, `std::process::Command` with `pre_exec` for setsid/TIOCSCTTY/signal reset. 448 lines.
- **Alacritty** `alacritty_terminal/src/tty/windows/conpty.rs`: Direct `CreatePseudoConsole` via `windows-sys`, `miow::pipe::anonymous()` for pipes, `CreateProcessW` with `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE`. 316 lines. Supports sideloaded `conpty.dll`.
- **WezTerm** `crates/portable-pty/src/unix.rs`: The code we're replacing. 414 lines. Key behaviors to preserve: EIO→EOF normalization, `close_random_fds()`, EOT-on-drop for writer.
- **WezTerm** `crates/portable-pty/src/win/`: ConPTY via `shared_library` dynamic loading. 513 lines. We replace this with direct `windows-sys` linking.

**Depends on:** Nothing — this is foundation work.

---

## 52.1 PtyCommand + Shared Types

**File(s):** `oriterm_mux/src/pty/command.rs` (new), `oriterm_mux/src/pty/mod.rs` (updated)

Cross-platform command specification replacing `portable_pty::CommandBuilder`. Uses `OsString` throughout to prevent encoding bugs on both platforms (Unix allows non-UTF-8 paths, Windows uses UTF-16). The type is used by `spawn.rs`, `shell_integration/inject.rs`, and `shell_integration/mod.rs`.

- [ ] Write failing tests for `PtyCommand` construction, env setting, arg building in `oriterm_mux/src/pty/command/tests.rs` (sibling pattern: `command/mod.rs` + `command/tests.rs`)
  - Matrix: empty command, command with args, env set/override/remove, cwd set, Windows case-insensitive env keys
  - Semantic pin: `PtyCommand::env()` with `OsString` keys that aren't valid UTF-8 must round-trip correctly

- [ ] Create `oriterm_mux/src/pty/command/mod.rs` with `PtyCommand` struct:
  ```rust
  /// Cross-platform command specification for PTY spawning.
  ///
  /// Replaces `portable_pty::CommandBuilder`. All paths and strings use
  /// `OsString` to prevent encoding bugs on both platforms.
  pub struct PtyCommand {
      program: OsString,
      args: Vec<OsString>,
      envs: BTreeMap<OsString, OsString>,
      cwd: Option<PathBuf>,
  }
  ```
  - Methods: `new(program)`, `arg(arg)`, `env(key, value)`, `cwd(dir)`, `program()`, `args()`, `envs()`, `get_cwd()`
  - Windows: env key lookups are case-insensitive (store lowercase key → `EnvEntry { preferred_key, value }`)
  - `PtyCommand` does NOT inherit the process environment — `spawn_pty()` handles that

- [ ] Define shared types in `oriterm_mux/src/pty/mod.rs` (these already exist in `spawn.rs` — refactor, don't duplicate):
  - `ExitStatus` — already exists, keep as-is but remove `From<portable_pty::ExitStatus>`
  - `PtyControl` — already exists, update inner type from `Box<dyn MasterPty>` to platform-specific concrete type
  - `PtyHandle` — already exists, update `child` field from `Box<dyn portable_pty::Child>` to concrete `ChildProcess`
  - `PtyConfig` — already exists. Keep `shell: Option<String>` and `env: Vec<(String, String)>` as-is (user-facing config is UTF-8 strings). The `spawn_pty()` function converts to `OsString` when building `PtyCommand`. Do NOT change `PtyConfig` to `OsString` — it's a config type, not a command type.
  - New: `PtyTransportKind { ConPty, RawPipe }` — stored in `PtyControl`, queryable for resize behavior. Section 53 adds the `RawPipe` variant implementation.

- [ ] `ChildProcess` type — concrete child process wrapper per platform:
  ```rust
  // Unix: wraps std::process::Child
  #[cfg(unix)]
  pub struct ChildProcess(std::process::Child);
  
  // Windows: wraps process handle (HANDLE via OwnedHandle)
  #[cfg(windows)]
  pub struct ChildProcess { /* process handle, thread handle */ }
  ```
  - Methods: `process_id() -> Option<u32>`, `kill() -> io::Result<()>`, `wait() -> io::Result<ExitStatus>`, `try_wait() -> io::Result<Option<ExitStatus>>`

- [ ] Source file must not exceed 500 lines. If `command.rs` approaches limit, split env handling into `command/env.rs`.
- [ ] Verify all tests pass: `timeout 150 cargo test -p oriterm_mux`

---

## 52.2 Unix PTY

**File(s):** `oriterm_mux/src/pty/unix.rs` (new)

Direct PTY creation via `libc::openpty()`, child spawning via `std::process::Command` with `pre_exec`, resize via `ioctl(TIOCSWINSZ)`. All behaviors from portable-pty's `unix.rs` are preserved: EIO→EOF normalization, fd hygiene (`close_random_fds`), session leader setup, controlling terminal, signal disposition reset.

- [ ] Write failing tests in `oriterm_mux/src/pty/unix/tests.rs`:
  - `test_openpty_returns_valid_fds` — open PTY pair, verify both fds are valid
  - `test_pty_size_propagated` — spawn `stty size`, verify output matches requested dimensions (port from `oriterm_core/tests/vttest/pty_size.rs`)
  - `test_eio_returns_eof` — close slave fd, read from master, expect `Ok(0)` not `Err(EIO)`
  - `test_child_exit_status` — spawn `true`/`false`, verify exit codes
  - Semantic pin: EIO→EOF normalization must return `Ok(0)`, not `Err`

- [ ] Create `oriterm_mux/src/pty/unix/mod.rs` (`#[cfg(unix)]` gated):
  - `pub(crate) fn open_pty(rows: u16, cols: u16) -> io::Result<(OwnedFd, OwnedFd)>` — calls `libc::openpty()`, sets `FD_CLOEXEC` on both fds. Returns `(master, slave)` as `OwnedFd`.
    ```rust
    // SAFETY: openpty is a standard POSIX syscall. master/slave are
    // output parameters populated by the kernel. OwnedFd takes
    // ownership and closes on drop.
    ```
  - `pub(crate) fn spawn_child(slave: &OwnedFd, cmd: &PtyCommand) -> io::Result<ChildProcess>` — builds `std::process::Command`, sets stdin/stdout/stderr to slave fd, configures `pre_exec` closure:
    - `libc::setsid()` — establish session leader
    - `libc::ioctl(0, TIOCSCTTY, 0)` — set controlling terminal
    - Signal disposition reset: `SIG_DFL` for SIGCHLD, SIGHUP, SIGINT, SIGQUIT, SIGTERM, SIGALRM
    - Signal mask clear: `sigprocmask(SIG_SETMASK, &empty_set, null)`
    - `close_random_fds()` — close all fds > 2 (prevent fd leaks from parent)
  - `pub(crate) fn resize_pty(master: RawFd, rows: u16, cols: u16) -> io::Result<()>` — `ioctl(TIOCSWINSZ)`
  - `pub(crate) fn clone_reader(master: RawFd) -> io::Result<PtyReader>` — dup master fd, wrap in `Read` impl with EIO→EOF normalization

- [ ] EIO→EOF normalization: master read returns `EIO` when slave closes on macOS/BSD. Normalize to `Ok(0)` (EOF) so `PtyReader` terminates cleanly. Reference: `crates/portable-pty/src/unix.rs:94-105`.

- [ ] `close_random_fds()`: enumerate `/dev/fd/`, close all fds > 2. On macOS, Cocoa leaks fds to child processes. On Linux, gnome/mutter leak shell extension fds. Reference: `crates/portable-pty/src/unix.rs:152-177`.

- [ ] Source file must not exceed 500 lines. Split into `unix/mod.rs` + `unix/tests.rs`.
- [ ] `#[cfg(unix)]` gate on the entire module. macOS-specific behavior (EIO from slave close) must be tested when running on macOS.
- [ ] Verify: `timeout 150 cargo test -p oriterm_mux`

---

## 52.3 Windows ConPTY + Overlapped Pipes

**File(s):** `oriterm_mux/src/pty/windows/mod.rs`, `windows/conpty.rs`, `windows/pipe.rs`, `windows/process.rs` (all new, `#[cfg(windows)]` gated)

ConPTY creation via `windows-sys` (direct link, no dynamic loading from kernel32). Overlapped named pipes replace anonymous pipes for cancelable I/O. Passthrough flag (`0x8`) attempted at creation for VT-transparent mode.

- [ ] Write failing tests in `oriterm_mux/src/pty/windows/tests.rs` (will compile but skip on non-Windows):
  - `test_named_pipe_creation` — create overlapped named pipe pair, write/read round-trip
  - `test_conpty_creation` — create ConPTY with overlapped pipes, verify handle is valid
  - `test_conpty_passthrough_flag` — create with `PSEUDOCONSOLE_PASSTHROUGH_MODE`, verify no error
  - `test_spawn_cmd_exe` — spawn `cmd.exe /c echo hello`, capture output
  - `test_conpty_size_propagated` — spawn `cmd.exe /c mode con`, verify output contains the requested rows/cols dimensions. This closes **BUG-07-004** (Windows PTY size test coverage gap). The old `pty_size_is_propagated` test was Unix-only (`stty size`); this is the Windows equivalent using `mode con` which reports console dimensions on Windows.
  - Semantic pin: overlapped pipe `ReadFile` with `OVERLAPPED` struct returns `ERROR_IO_PENDING`, then completes

- [ ] Create `oriterm_mux/src/pty/windows/pipe.rs` — overlapped named pipe creation:
  ```rust
  /// Create a pair of overlapped named pipes for ConPTY I/O.
  ///
  /// Uses `CreateNamedPipeW` + `CreateFileW` with `FILE_FLAG_OVERLAPPED`
  /// instead of `CreatePipe`'s synchronous anonymous pipes. This enables
  /// `CancelIoEx()` for clean shutdown and is required for ConPTY's
  /// passthrough mode (flag 0x8).
  pub(crate) fn create_overlapped_pipe_pair(
      buffer_size: u32,
  ) -> io::Result<(OwnedHandle, OwnedHandle)>
  ```
  - Generate unique pipe name: `\\.\pipe\oriterm-pty-{pid}-{counter}`
  - Server end: `CreateNamedPipeW` with `PIPE_ACCESS_INBOUND | FILE_FLAG_OVERLAPPED` (or `OUTBOUND`)
  - Client end: `CreateFileW` with `FILE_FLAG_OVERLAPPED`
  - Buffer size configurable (default: 128 KB, matching `READ_BUFFER_SIZE`)

- [ ] Create `oriterm_mux/src/pty/windows/conpty.rs` — ConPTY lifecycle:
  ```rust
  /// RAII ConPTY handle. Drop calls ClosePseudoConsole.
  pub(crate) struct ConPty {
      handle: HPCON,
  }
  ```
  - `ConPty::new(rows, cols, input_handle, output_handle) -> io::Result<Self>` — calls `CreatePseudoConsole` via `windows-sys` with flags: `PSEUDOCONSOLE_INHERIT_CURSOR | PSEUDOCONSOLE_RESIZE_QUIRK | PSEUDOCONSOLE_WIN32_INPUT_MODE | PSEUDOCONSOLE_PASSTHROUGH_MODE`
  - If passthrough flag fails (HRESULT != S_OK), retry without it and log the fallback
  - `ConPty::resize(rows, cols) -> io::Result<()>` — `ResizePseudoConsole`
  - `ConPty::shutdown()` — explicit shutdown method (NOT Drop-based) that executes a concrete three-step protocol:
    1. **Close the conin pipe** — signal EOF to ConPTY's input
    2. **Call `ClosePseudoConsole(handle)`** — this blocks until conhost drains the conout pipe. Because the PtyReader thread is still alive and reading, the pipe drains normally and `ClosePseudoConsole` returns promptly.
    3. **Close the conout pipe** — PtyReader sees EOF and exits.
    This ordering is critical: `ClosePseudoConsole` deadlocks if the conout reader is already gone (nobody draining the pipe). Alacritty uses struct field drop ordering (conpty first = dropped first, conout below = dropped after; `alacritty_terminal/src/tty/windows/mod.rs:28-31`). Our architecture is different — the conout reader lives on the PtyReader thread, not as a struct field. The explicit `shutdown()` method makes the sequencing visible and testable rather than relying on Rust's field-drop order.
    
    Wire into `Pane::drop()` (`oriterm_mux/src/pane/shutdown.rs`): between step 1 (signal writer) and step 2 (kill child), call `pty.shutdown_conpty()` which delegates to `ConPty::shutdown()`. The existing `drop_pane_background()` in `oriterm_mux/src/server/dispatch/helpers.rs` already runs pane drops on a background thread, so the blocking `ClosePseudoConsole` call does not stall the server event loop.
    
    Test: `test_conpty_shutdown_no_deadlock` — create ConPTY, start a reader thread on the conout pipe, call `shutdown()`, verify it returns within 5 seconds (not deadlocked). `#[cfg(windows)]`.
  - `Drop for ConPty` — calls `shutdown()` if not already called (defensive fallback). Logs a warning if `shutdown()` was not called explicitly — this means the caller missed the shutdown protocol.
  - `// SAFETY:` comment on every unsafe block calling windows-sys FFI
  - `unsafe impl Send for ConPty` — ConPTY handle is thread-safe per Microsoft docs

- [ ] Create `oriterm_mux/src/pty/windows/process.rs` — child process spawning:
  - `ProcThreadAttributeList` RAII wrapper: `InitializeProcThreadAttributeList`, `UpdateProcThreadAttribute` (set `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE`), `DeleteProcThreadAttributeList` on drop
  - `spawn_with_conpty(conpty: &ConPty, cmd: &PtyCommand) -> io::Result<ChildProcess>`:
    - Build `STARTUPINFOEXW` with attribute list
    - Set `STARTF_USESTDHANDLES` with `INVALID_HANDLE_VALUE` for all stdio (prevents handle inheritance)
    - Build UTF-16 command line from `PtyCommand` args (port arg quoting from `crates/portable-pty/src/cmdbuilder.rs:700-745`)
    - Build UTF-16 environment block from `PtyCommand` envs + inherited process env
    - `CreateProcessW` with `EXTENDED_STARTUPINFO_PRESENT | CREATE_UNICODE_ENVIRONMENT`
  - `WinChild` struct: wraps process handle, implements `process_id()`, `kill()`, `wait()`, `try_wait()`

- [ ] `oriterm_mux/src/pty/windows/mod.rs` — orchestration:
  - `pub(crate) fn spawn_conpty(config: &PtyConfig, cmd: &PtyCommand) -> io::Result<PtyHandle>` — creates pipe pairs, ConPTY, spawns child, assembles `PtyHandle`
  - Store `PtyTransportKind::ConPty` in the returned `PtyControl`

- [ ] Each source file must not exceed 500 lines. The `windows/` directory is already split by concern.
- [ ] All `windows-sys` features needed: add to `oriterm_mux/Cargo.toml` `[target.'cfg(windows)'.dependencies]`:
  - `Win32_System_Pipes` (CreateNamedPipeW)
  - `Win32_Storage_FileSystem` (CreateFileW, FILE_FLAG_OVERLAPPED)
  - `Win32_System_IO` (OVERLAPPED, CancelIoEx)
  - `Win32_System_Threading` (CreateProcessW, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, etc.)
  - `Win32_Security` (SECURITY_ATTRIBUTES for pipe creation)
- [ ] Verify: `timeout 150 cargo test -p oriterm_mux` (tests compile on all platforms, Windows-specific tests `#[cfg(windows)]` gated)
- [ ] Cross-compile check: `cargo build --target x86_64-pc-windows-gnu` succeeds

- [ ] **TPR checkpoint** — `/tpr-review` covering 52.1–52.3 implementation work

---

## 52.4 Sideloaded conpty.dll

**File(s):** `oriterm_mux/src/pty/windows/conpty.rs` (extended)

Support loading a newer ConPTY implementation from a `conpty.dll` placed next to the oriterm binary. Windows Terminal's NuGet package (`Microsoft.Windows.Console.ConPTY`) ships a ConPTY with native Sixel support and bug fixes. When found, its `CreatePseudoConsole`/`ResizePseudoConsole`/`ClosePseudoConsole` are used instead of the kernel32 versions.

- [ ] Write failing tests in `oriterm_mux/src/pty/windows/tests.rs` (extend from 52.3):
  - `test_sideloaded_dll_preferred` — mock DLL loading, verify sideloaded functions are called when DLL exists. Semantic pin: when a sideloaded DLL is present, the `ConPtyApi::load()` return value must use sideloaded function pointers, not kernel32 pointers.
  - `test_conpty_api_fallback_to_kernel32` — when no sideloaded DLL exists, `ConPtyApi::load()` returns kernel32 function pointers. Verify via log output or API source flag.
  - `test_conpty_api_cached_in_oncelock` — call `ConPtyApi::load()` twice, verify the second call returns the same instance (pointer equality on function pointers). Verifies `OnceLock` caching.
  - CI-runnable: all three tests use mock/unit logic, no real DLL needed.

- [ ] Extend `conpty.rs` with `ConPtyApi` struct:
  ```rust
  /// Function pointers for ConPTY API — either from kernel32 (default)
  /// or from a sideloaded conpty.dll (preferred when available).
  struct ConPtyApi {
      create: unsafe extern "system" fn(COORD, HANDLE, HANDLE, u32, *mut HPCON) -> HRESULT,
      resize: unsafe extern "system" fn(HPCON, COORD) -> HRESULT,
      close: unsafe extern "system" fn(HPCON),
  }
  ```
  - `ConPtyApi::load()`: try `LoadLibraryW("conpty.dll")`, fall back to direct `windows-sys` function pointers
  - `GetProcAddress` for each function from the DLL
  - Log which source is in use: `log::info!("ConPTY: using sideloaded conpty.dll")` or `log::info!("ConPTY: using kernel32")`
  - Cache in `OnceLock<ConPtyApi>` — loaded once at first PTY creation

- [ ] Source file must not exceed 500 lines (conpty.rs with the new `ConPtyApi` additions).
- [ ] Verify: `timeout 150 cargo test -p oriterm_mux` (tests pass in debug)
- [ ] Verify: cross-compile check `cargo build --target x86_64-pc-windows-gnu` passes

---

## 52.5 Migration

**File(s):** Multiple existing files updated, `crates/portable-pty/` removed

Rewire all `portable_pty::` imports to use the new native PTY types. Remove the `portable-pty` dependency from all `Cargo.toml` files. Delete the vendored crate.

- [ ] Update `oriterm_mux/src/pty/spawn.rs` (currently 394 lines):
  - Replace `use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system}` with `use super::command::PtyCommand` and platform-specific spawn functions
  - `build_command()` returns `PtyCommand` instead of `CommandBuilder`
  - `spawn_pty()` calls `unix::spawn_unix_pty()` or `windows::spawn_conpty()` based on `#[cfg]`
  - Remove `pty_err()` helper (line 9-11 — no longer needed, platform code returns `io::Error` directly)
  - Remove `From<portable_pty::ExitStatus>` impl (lines 47-54)
  - **File size check**: after migration, `spawn.rs` should shrink (removing portable-pty glue) — verify it stays under 500 lines. If Section 53 adds `PtyTransport` parameter handling and it grows past 400, proactively extract `PtyConfig`/`ExitStatus`/`PtyControl`/`PtyHandle` types into `oriterm_mux/src/pty/types.rs`.

- [ ] Update `oriterm_mux/src/shell_integration/mod.rs`:
  - Replace `use portable_pty::CommandBuilder` with `use crate::pty::command::PtyCommand`
  - Update `set_common_env(cmd: &mut PtyCommand)` signature

- [ ] Update `oriterm_mux/src/shell_integration/inject.rs`:
  - Replace `use portable_pty::CommandBuilder` with `use crate::pty::command::PtyCommand`
  - All `cmd.env()`, `cmd.arg()`, `cmd.cwd()` calls unchanged (PtyCommand has same API)

- [ ] Update `oriterm_mux/src/shell_integration/tests.rs`:
  - Replace `portable_pty::CommandBuilder::new(...)` with `PtyCommand::new(...)`

- [ ] Update test PTY helpers (these use portable-pty directly for vttest):
  - `oriterm_core/tests/vttest/session.rs` — replace `portable_pty::{CommandBuilder, PtySize, native_pty_system}` with lightweight test helper that calls `libc::openpty()` directly (avoids `oriterm_core → oriterm_mux` dependency)
  - `oriterm_core/tests/vttest/pty_size.rs` — same replacement
  - `oriterm/src/gpu/visual_regression/vttest/mod.rs` — same replacement
  - Create `oriterm_core/tests/vttest/test_pty.rs` with `~60` line helper: `open_test_pty(rows, cols) -> (master_reader, master_writer, child)` using direct `libc::openpty()` + `std::process::Command`

- [ ] Remove `portable-pty` from Cargo.toml files:
  - `Cargo.toml` (workspace): remove `portable-pty = { path = "crates/portable-pty" }` from `[patch.crates-io]`
  - `oriterm_mux/Cargo.toml`: remove `portable-pty = "0.9.0"` from `[dependencies]`
  - `oriterm_core/Cargo.toml`: remove `portable-pty = "0.9.0"` from `[dev-dependencies]`
  - `oriterm/Cargo.toml`: remove `portable-pty = "0.9.0"` from `[dependencies]`
  - `oriterm_core/Cargo.toml`: add `libc = "0.2"` under `[target.'cfg(unix)'.dev-dependencies]` (currently only macOS-gated; needed for `libc::openpty()` in new test helper on Linux too)

- [ ] Delete `crates/portable-pty/` directory entirely

- [ ] Update `plans/roadmap/00-overview.md` dependency graph: remove `portable-pty` from `oriterm` and `oriterm_mux` dependency lists

- [ ] Update doc comments that reference portable-pty (grep for `portable.pty` in `.rs` files):
  - `oriterm_mux/src/domain/local.rs:1` (`//! Local domain — spawns shells on the local machine via portable-pty`) — update to reference native PTY layer
  - `oriterm_mux/src/domain/local.rs:22` (`/// The simplest domain — creates a PTY via portable-pty`) — same
  - `oriterm_mux/src/pty/mod.rs:1-6` (`//! Cross-platform PTY abstraction... Uses portable-pty for platform abstraction`) — rewrite module doc to describe native PTY layer

- [ ] Update `oriterm_mux/src/pty/reader/mod.rs` (110 lines): the 1ms `thread::sleep` after each read (line 104) is ConPTY-specific (gives conhost scheduling time for Ctrl+C — BUG-11-1 fix). This sleep must be conditional on transport type:
  - Add `transport: PtyTransportKind` field to `PtyReader` struct (line 30-37). Update constructor `PtyReader::new()` to accept the transport kind.
  - In `run()` (line 69), wrap the `thread::sleep` at line 104 with `if matches!(self.transport, PtyTransportKind::ConPty)`.
  - Update `LocalDomain::spawn_pane()` in `domain/local.rs:179` to pass `PtyTransportKind::ConPty` when constructing `PtyReader::new()`.
  - Test: `test_reader_no_sleep_for_raw_pipe` — mock reader with `RawPipe` transport, verify no 1ms sleep overhead (measure elapsed time for N reads vs ConPTY reader).

- [ ] Update comment in `oriterm_mux/src/pane/mod.rs:454` and `mod.rs:478` ("The PID comes from portable-pty's Child::process_id()") to reference the new native `ChildProcess` type.
  - **BLOAT warning**: `pane/mod.rs` is currently 491 lines — 9 lines from the 500-line limit. Section 53 adds `PtyTransportKind`-aware signal dispatch to `send_signal_platform()` (lines 442-488), which will push it over. Before making changes, extract `send_signal_to_child()` + `send_signal_platform()` + the `Signal` enum into `pane/signal.rs` (~50 lines). Add `mod signal;` to `pane/mod.rs` and use `signal::send_signal_to_child` in the existing call site.

- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `timeout 150 ./test-all.sh` green

---

## 52.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 52.N Completion Checklist

- [ ] `portable-pty` does not appear in any `Cargo.toml` in workspace
- [ ] `crates/portable-pty/` deleted
- [ ] `grep -r "portable.pty\|portable_pty" --include="*.rs" --include="*.toml"` returns only plan files and comments
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `timeout 150 ./test-all.sh` green — all existing tests pass
- [ ] Windows cross-compile: `cargo build --target x86_64-pc-windows-gnu` succeeds
- [ ] No new `unsafe` blocks without `// SAFETY:` comments
- [ ] All source files under 500 lines
- [ ] Tests follow sibling `tests.rs` pattern
- [ ] Plan annotation cleanup: all temporary scaffolding removed from `.rs` files
- [ ] All intermediate TPR checkpoint findings resolved
- [ ] **Plan sync** — update plan metadata:
  - [ ] This section's frontmatter `status` → `complete`
  - [ ] `00-overview.md` Quick Reference updated
  - [ ] `index.md` section status updated
  - [ ] Section 03 updated with note that portable-pty was replaced by Section 52
  - [ ] BUG-07-004 marked resolved in `plans/bug-tracker/section-07-ci-build.md`
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed — MUST run AFTER `/tpr-review` is clean

**Exit Criteria:** `./build-all.sh && ./clippy-all.sh && timeout 150 ./test-all.sh` all green. `portable-pty` no longer appears in any Cargo.toml or Cargo.lock. Cross-compile for `x86_64-pc-windows-gnu` succeeds. All vttest tests pass with the new PTY layer.
