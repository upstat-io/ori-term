---
section: 53
title: "Raw Pipe Bypass for VT-Native Shells"
status: not-started
reviewed: true
tier: 5
goal: "Bypass ConPTY entirely for VT-native children (WSL) by connecting pipes directly. Image protocols (Sixel, Kitty) pass through unmangled. OriTerm becomes the first Windows terminal with zero-overhead VT passthrough. SSH raw pipe transport deferred to Section 35 (SshDomain)."
success_criteria:
  - "WSL shell spawns via raw pipes — no conhost.exe process created"
  - "Image protocol data (Sixel DCS, Kitty APC) passes through byte-identical"
  - "WslDomain selects raw pipe transport by default"
  - "Fallback to ConPTY when raw pipe mode fails — log::warn emitted"
  - "Resize works or documented limitation with ConPTY fallback for interactive sessions"
  - "Shell integration (OSC 133, OSC 7) works in raw pipe mode"
  - "Ctrl+C delivery via 0x03 stdin write stops flooding processes"
  - "Key encoding correct for raw pipe sessions (no Win32 input mode dependency)"
  - "No regressions — ./test-all.sh green"
inspired_by:
  - "No prior art — no existing Windows terminal bypasses ConPTY for VT-native shells"
  - "Unix PTY model (the gold standard this emulates on Windows)"
depends_on: ["52"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "53.1"
    title: "Raw Pipe Transport"
    status: not-started
  - id: "53.2"
    title: "Resize Strategy (experimental — gates 53.3+)"
    status: not-started
  - id: "53.3"
    title: "Key Encoding Transport Awareness"
    status: not-started
  - id: "53.4"
    title: "Domain-Level Transport Selection + WslDomain"
    status: not-started
  - id: "53.5"
    title: "Image Protocol Verification"
    status: not-started
  - id: "53.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "53.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 53: Raw Pipe Bypass for VT-Native Shells

**Status:** Not Started
**Goal:** For VT-native children (WSL), bypass ConPTY entirely by connecting pipes directly between the terminal and the child process. ConPTY is a translation layer — it consumes VT sequences, applies them to a virtual screen buffer, then re-encodes them. For children that already output VT (every Unix shell via WSL), this translation is pure overhead that actively mangles output: DCS sequences (Sixel) get consumed and dropped, APC sequences (Kitty graphics) vanish, OSC sequences get selectively eaten, and sequence ordering changes due to screen buffer diffing.

By owning the PTY layer (Section 52), we can create raw pipe pairs and connect them directly — the terminal and child speak VT to each other, exactly like Unix. Image protocols work perfectly. No translation, no mangling, no dropped sequences.

This would make OriTerm the first Windows terminal to offer zero-overhead VT passthrough for WSL sessions.

**SSH scope boundary:** SSH raw pipe transport is Section 35 (SshDomain) territory, not this section. Local `ssh.exe` spawned as a subprocess via `LocalDomain` still uses ConPTY (ssh.exe is a Win32 console app that needs the translation layer for its own console I/O). When SshDomain is implemented in Section 35, it will use the raw pipe infrastructure established here to connect directly to the SSH channel.

**Success Criteria:**

- [ ] WSL shell (`wsl.exe`) spawns via raw pipes — `tasklist | findstr conhost` shows no new `conhost.exe` process for this session (satisfies: raw pipe for WSL)
- [ ] `printf '\x1bPq#0;2;0;0;0\x1b\\'` inside WSL produces byte-identical DCS data at the reader thread — verified via hex dump comparison (satisfies: Sixel passthrough)
- [ ] `printf '\x1b_Ga=T,f=100,s=1,v=1,m=0;AAAA\x1b\\'` inside WSL arrives byte-for-byte at the reader thread (satisfies: Kitty APC passthrough)
- [ ] `WslDomain::spawn_pane()` calls `spawn_raw_pipe()` before falling back to `spawn_conpty()` (satisfies: domain selection)
- [ ] Terminal resize updates the child's terminal size (mechanism depends on 53.2 findings); if no mechanism found, limitation is documented and ConPTY is used for interactive sessions (satisfies: resize works or graceful limitation)
- [ ] When raw pipe spawn fails (e.g., `wsl.exe` missing), `log::warn!` is emitted and ConPTY is used instead — verified by test mocking a spawn failure (satisfies: fallback)
- [ ] `printf '\x1b]133;A\x07'` inside WSL via raw pipe arrives intact at the reader thread — OSC 133 shell integration continues to work (satisfies: no regressions)
- [ ] `printf '\x1b]7;file://host/path\x07'` inside WSL via raw pipe arrives intact — OSC 7 CWD reporting works (satisfies: no regressions)
- [ ] Ctrl+C delivery in raw pipe mode writes `0x03` to the stdin pipe and stops a flooding `yes` process — verified end-to-end (satisfies: signal delivery)
- [ ] Key encoding works correctly for raw pipe sessions — `KeyInput.transport` field gates Win32 encoding; raw pipe sessions use standard kitty/legacy VT encoding written to stdin (satisfies: input correctness)

**Context:** ConPTY was built in 2018 to make legacy Win32 console apps (cmd.exe, PowerShell using `WriteConsoleW`) work inside modern VT terminals. It is architecturally a translation layer: `Child → Win32 Console API → Screen Buffer → VtEngine → VT sequences → Terminal`. Every escape sequence the child sends is consumed by conhost, applied to a screen buffer, then re-encoded. This is necessary for Win32 console apps, but for VT-native children it is pure overhead that actively breaks features.

The August 2024 ConPTY rewrite (PR #17510) improved the translation fidelity but did not change the fundamental architecture — it is still a translation, not a passthrough. The passthrough flag (`0x8`) attempts to fix this but may not be stable on all Windows builds.

**WARNING — Experimental territory:** WSL resize with raw pipes has no documented Microsoft API. Section 53.2 must experimentally validate the resize mechanism before raw pipe mode can be the default. If resize cannot be made to work, raw pipe mode may be limited to fixed-size sessions or applications that query terminal size via escape sequences.

**Reference implementations:**
- **None** — no existing Windows terminal does this. This is novel.
- **Unix PTY model** — the behavior we're emulating. Unix PTYs are dumb byte pipes.

**Depends on:** Section 52 (Native PTY Layer) — overlapped pipe infrastructure, `PtyTransportKind` enum, `PtyControl` abstraction.

---

## 53.1 Raw Pipe Transport

**File(s):** `oriterm_mux/src/pty/windows/raw.rs` (new)

Create a raw pipe backend that connects overlapped named pipes directly to a child process's stdin/stdout — no ConPTY in between. Reuses the pipe infrastructure from Section 52.3 (`pipe.rs`).

- [ ] Write failing tests:
  - `test_raw_pipe_echo` — spawn `cmd.exe /c echo hello` via raw pipes (no ConPTY), capture output. Note: `cmd.exe` is NOT VT-native (it uses Win32 console API for color), but `echo` produces plain text which validates the pipe plumbing. `CREATE_NO_WINDOW` allows `cmd.exe` to run without a visible console window.
  - `test_raw_pipe_wsl` — spawn `wsl.exe echo hello`, capture output (`#[cfg(windows)]`, may need `#[ignore]` if WSL not available in CI)
  - `test_vt_passthrough` — spawn a process that outputs a DCS sequence, verify it arrives byte-for-byte at the reader. Use `wsl.exe -e printf '\x1bPq...\x1b\\'` or a small helper binary.
  - Semantic pin: DCS `\x1bPq` sequence must arrive intact (ConPTY would strip it)

- [ ] Create `oriterm_mux/src/pty/windows/raw.rs`:
  - `pub(crate) fn spawn_raw_pipe(cmd: &PtyCommand, buffer_size: u32) -> io::Result<PtyHandle>`:
    - Create two overlapped named pipe pairs (one for stdin, one for stdout)
    - `CreateProcessW` with pipe handles as `hStdInput`/`hStdOutput`/`hStdError` — NO `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE`
    - Set `STARTF_USESTDHANDLES` flag
    - `CREATE_UNICODE_ENVIRONMENT | CREATE_NO_WINDOW` — prevent console window allocation
    - Assemble `PtyHandle` with `PtyTransportKind::RawPipe`
  - Reader: overlapped `ReadFile` on stdout pipe
  - Writer: overlapped `WriteFile` on stdin pipe
  - `PtyControl` for raw pipe: resize dispatches to strategy determined in 53.2

- [ ] Handle `CREATE_NO_WINDOW` vs `DETACHED_PROCESS` — both prevent console allocation but have different semantics. `CREATE_NO_WINDOW` is preferred (child can still call `AllocConsole()` if needed).

- [ ] stderr handling: merge stderr into stdout pipe (same fd) so error output is visible. Alternative: separate stderr pipe with its own reader. Start with merged (simpler, matches Unix behavior).

- [ ] **Ctrl+C signal delivery for raw pipe processes (CRITICAL).** Without ConPTY, there is no console, so `GenerateConsoleCtrlEvent()` (used in `oriterm_mux/src/pane/mod.rs:471-488`) will fail. The current write-stall + direct-signal mechanism (`write_stalled` flag + `signal_child()`) depends on `GenerateConsoleCtrlEvent` working. For raw pipe sessions, this must be replaced:
  - For WSL: write Ctrl+C byte (`0x03`) directly to the stdin pipe — WSL's internal PTY will deliver SIGINT. This is the simple case.
  - For other raw pipe processes: `TerminateProcess()` as last resort (ungraceful).
  - Update `send_signal_platform()` (currently `oriterm_mux/src/pane/mod.rs:471-488`, to be extracted to `pane/signal.rs` per Section 52.5) to accept transport kind. When `PtyTransportKind::RawPipe`, write `0x03` to the stdin pipe instead of calling `GenerateConsoleCtrlEvent`. The current Windows implementation uses `GenerateConsoleCtrlEvent(CTRL_C_EVENT, pid)` which requires a console — raw pipe processes have no console, so this call would fail.
  - The `PaneNotifier` already has an `mpsc::Sender<Msg>` channel to the writer thread (`oriterm_mux/src/pty/mod.rs:33`). Add a `Msg::SignalInterrupt` variant that writes `0x03` directly (bypasses the write stall check since `0x03` is tiny and won't block). The writer thread's `drain_channel()` already handles all variants.
  - `Pane::signal_child()` (called from `oriterm/src/app/keyboard_input/mod.rs` when Ctrl+C is pressed and the writer is stalled) must check the pane's transport kind: `ConPty` -> existing `GenerateConsoleCtrlEvent` path; `RawPipe` -> send `Msg::SignalInterrupt` via the notifier channel.
  - Test: `test_ctrl_c_raw_pipe_0x03_write` — spawn `wsl.exe -e yes` via raw pipes, send `0x03` to stdin, verify process exits. `#[cfg(windows)]`, `#[ignore]` (requires WSL).

- [ ] **Reader thread behavior.** The `PtyReader` (from Section 52) must NOT use the 1ms ConPTY sleep for raw pipe transport. Verify that `PtyReader` receives `PtyTransportKind` and skips the sleep for `RawPipe` (this is set up in Section 52.5).

- [ ] Source file must not exceed 500 lines. Split into `raw/mod.rs` + `raw/tests.rs` if needed.

**Test verification taxonomy for 53.1:**
- **CI-runnable** (`cargo test`): `test_raw_pipe_echo` (spawns `cmd.exe /c echo` — works on Windows CI), pipe creation/round-trip tests.
- **`#[ignore]` (requires WSL)**: `test_raw_pipe_wsl`, `test_vt_passthrough`. Run manually on Windows with WSL installed: `cargo test -- --ignored`.
- **Manual real-Windows**: Ctrl+C delivery end-to-end with `wsl.exe -e yes` (hard to automate — interactive process flooding).

---

## 53.2 Resize Strategy

**File(s):** `oriterm_mux/src/pty/windows/raw.rs` (extended), `oriterm_mux/src/pty/windows/resize.rs` (new if needed)

Without ConPTY, there is no `ResizePseudoConsole()`. The resize mechanism depends on the child type. This subsection must experimentally validate each mechanism before committing to it.

- [ ] **Experimental validation (MUST BE DONE FIRST):**
  - Spawn `wsl.exe` with raw pipes on a real Windows machine
  - Attempt each resize mechanism below
  - Document which one works, which one doesn't, and any side effects
  - Results go in this section as findings — do not guess

- [ ] `ResizeStrategy` enum in `oriterm_mux/src/pty/windows/raw.rs`:
  ```rust
  /// How to notify the child of terminal resize in raw pipe mode.
  pub(crate) enum ResizeStrategy {
      /// No resize support — child sees initial size only.
      None,
      /// Send SIGWINCH via WSL interop (wsl.exe --resize or similar).
      /// Only added if 53.2 experimentation confirms it works.
      WslInterop,
      // NOTE: CSI 8;rows;cols t is NOT usable for resize — it's a
      // response FROM the terminal TO the application. No known VT
      // escape exists for resize commands.
      //
      // SSH window-change (RFC 4254 section 6.7) is Section 35's scope.
      // When SshDomain is implemented, it will add its own variant or
      // handle resize at the domain level.
  }
  ```

- [ ] For **WSL sessions** (`wsl.exe`):
  - Investigation: does `wsl.exe` create its own PTY inside the Linux VM? If so, does it poll the Windows console for size changes? With raw pipes (no console), does it have any resize path?
  - Test: spawn `wsl.exe` via raw pipes, sleep 2s, resize, check `stty size` output
  - Possible approach: `wsl.exe` might read terminal size from the pipe's `GetConsoleScreenBufferInfo` — but raw pipes aren't console handles, so this would fail
  - If no resize works: document limitation, fall back to ConPTY for interactive WSL sessions, use raw pipes only for non-interactive/fixed-size use cases (or when user opts in knowing resize won't work)

- [ ] For **other VT-native programs** (including local `ssh.exe` subprocess — SSH domain integration is Section 35's scope): default `ResizeStrategy::None` with a log warning. Programs that query terminal size via `CSI 18 t` (report window size) will get the current size from our response.

- [ ] Wire `ResizeStrategy` into `PtyControl`:
  ```rust
  impl PtyControl {
      pub fn resize(&self, rows: u16, cols: u16) -> io::Result<()> {
          match self.transport {
              PtyTransportKind::ConPty => { /* ResizePseudoConsole */ }
              PtyTransportKind::RawPipe { strategy } => {
                  match strategy {
                      ResizeStrategy::None => Ok(()),
                      ResizeStrategy::WslInterop => { /* TBD based on 53.2 findings */ }
                  }
              }
          }
      }
  }
  ```

**Test verification taxonomy for 53.2:**
- **Manual real-Windows ONLY**: all resize experiments require a real Windows machine with WSL. These are experimental validation steps, not automated tests. Document findings as comments in the plan section.
- **CI-runnable**: `ResizeStrategy` enum construction and `PtyControl::resize()` dispatch logic (unit tests on all platforms).

- [ ] **IO thread handler resize awareness.** The IO thread handler (`oriterm_mux/src/pane/io_thread/handler.rs:20-33`) calls `ctl.resize(rows, cols)` and logs on failure. For `ResizeStrategy::None`, `PtyControl::resize()` returns `Ok(())` (no-op), which is fine — the IO thread still does grid reflow. The grid always reflects the new size even if the child doesn't know. No changes needed to handler.rs itself — the abstraction in `PtyControl` handles this correctly. However, add a test: verify that `process_resize()` with a raw-pipe `PtyControl` (no-op resize) still reflows the grid and produces a snapshot.

- [ ] **TPR checkpoint** — `/tpr-review` covering 53.1–53.2 implementation and experimental findings

**ORDERING GATE**: 53.2's experimental findings determine whether `WslDomain` uses `RawPipeWithFallback` (resize works) or `ConPty` with raw-pipe opt-in (resize doesn't work). Sections 53.3 and 53.4 MUST NOT begin until 53.2's findings are documented.

---

## 53.3 Key Encoding Transport Awareness

**File(s):** `oriterm/src/key_encoding/mod.rs` (updated), `oriterm/src/app/keyboard_input/mod.rs` (updated)

Win32 input mode (`PSEUDOCONSOLE_WIN32_INPUT_MODE` / `DECSET ?9001`) encodes keystrokes as Win32 `INPUT_RECORD` structures. This only works with ConPTY — raw pipe children expect standard VT-encoded keystrokes. The key encoding path must be transport-aware.

**Codebase context:**
- `oriterm/src/key_encoding/mod.rs` (228 lines): `encode_key()` currently dispatches solely on `TermMode` flags. The win32 module (`win32.rs`, 227 lines) exists but is `#[allow(dead_code)]` and NOT wired into `encode_key()` (comment at line 9-16). The recent commit `a2b20abe` added win32 input mode support but did not wire it into production encoding.
- `oriterm/src/app/keyboard_input/mod.rs:260`: `encode_key_to_pty()` calls `key_encoding::encode_key()` with `mode` from the pane's mode cache. No transport kind is available at this call site.
- `KeyInput` struct (line 80-99) has no transport field.
- `TermMode::WIN32_INPUT_MODE` is set by the VTE handler when ConPTY sends `DECRPM ?9001`. For raw pipe sessions, ConPTY never sends this, so the mode flag is never set.

**Analysis:** Currently, the win32 module is dead code and `WIN32_INPUT_MODE` is never set for raw pipe sessions (no ConPTY to send the DECRPM). So the key encoding path already works correctly for raw pipe sessions by default — they use kitty/legacy encoding. The risk is future activation of win32 encoding: if someone wires `encode_win32()` into `encode_key()`, it must be gated on ConPTY transport, not just the mode flag.

- [ ] Write failing tests in `oriterm/src/key_encoding/tests.rs`:
  - `test_raw_pipe_never_uses_win32_encoding` — semantic pin: create a `KeyInput` with `TermMode::WIN32_INPUT_MODE` set AND `transport: RawPipe`. `encode_key()` must produce VT encoding (kitty or legacy), never win32 INPUT_RECORD format. Guards against future regressions when win32 encoding is wired in.
  - `test_conpty_transport_allows_win32_encoding` — when `transport: ConPty` AND `WIN32_INPUT_MODE` is set, `encode_key()` may produce win32 encoding (once wired in). Verifies the gate permits ConPTY usage.
  - `test_key_input_default_transport_is_conpty` — `KeyInput` with no explicit transport field set defaults to `ConPty`. Backward compatibility pin: existing code that doesn't set transport must not break.
  - Matrix: {ConPty, RawPipe} x {WIN32_INPUT_MODE set, unset} x {Kitty mode, legacy mode} = 8 combinations. At minimum, the two semantic pins above cover the critical raw-pipe-blocks-win32 and conpty-permits-win32 axes.
  - **CI-runnable** — all tests are pure encoding logic, no platform dependency.

- [ ] Add `transport` field to `KeyInput` struct in `oriterm/src/key_encoding/mod.rs:80`:
  ```rust
  /// PTY transport kind — determines whether Win32 input mode is available.
  ///
  /// ConPTY sessions may use Win32 INPUT_RECORD encoding when the mode is
  /// active. Raw pipe sessions always use VT encoding regardless of mode flags.
  pub transport: PtyTransportKind,
  ```
  Import `PtyTransportKind` from `oriterm_mux` — but wait, `oriterm` depends on `oriterm_mux`, so this import is valid per crate boundaries. However, `PtyTransportKind` is a simple 2-variant enum with no dependencies. To avoid `oriterm` importing a mux type into its encoding module, define the enum in `oriterm_core` (which both crates depend on) or duplicate it as a local enum in `key_encoding`. **Decision: define `PtyTransportKind` in `oriterm_mux/src/pty/mod.rs` and re-export it. `oriterm` already depends on `oriterm_mux`.** The `KeyInput` transport field defaults to `ConPty` for backward compatibility.

- [ ] Update `encode_key()` in `oriterm/src/key_encoding/mod.rs:111`: when win32 encoding is eventually wired in (currently dead code), gate it on `input.transport == PtyTransportKind::ConPty`. Add a comment:
  ```rust
  // Win32 input mode is ConPTY-only. Raw pipe sessions use VT encoding
  // even if WIN32_INPUT_MODE flag is set (it shouldn't be, but defensive).
  ```

- [ ] Update `encode_key_to_pty()` in `oriterm/src/app/keyboard_input/mod.rs:260`: pass the active pane's transport kind into `KeyInput`. The transport kind is known at `Pane` creation time — store it on the pane and expose via the snapshot or mode cache. Concrete path:
  - Add `transport: PtyTransportKind` field to `PaneParts` struct and `Pane` struct.
  - `LocalDomain::spawn_pane()` sets `PtyTransportKind::ConPty`.
  - `WslDomain::spawn_pane()` sets the appropriate transport kind.
  - `Pane` exposes `pub fn transport(&self) -> PtyTransportKind`.
  - `encode_key_to_pty()` reads transport from the pane via the mux handle.

- [ ] Verify: `timeout 150 cargo test -p oriterm` (key encoding tests pass with new transport field).

---

## 53.4 Domain-Level Transport Selection + WslDomain

**File(s):** `oriterm_mux/src/domain/local.rs` (updated), `oriterm_mux/src/domain/wsl.rs` (updated), `oriterm_mux/src/pty/spawn.rs` (updated)

**PREREQUISITE**: 53.2 findings must be documented. The `WslDomain` default transport depends on whether resize works with raw pipes.

Transport selection happens at the domain level, not globally. `LocalDomain` always uses ConPTY (Win32 console apps need translation). `WslDomain` uses raw pipes (WSL shells are VT-native). Future domains (SshDomain in Section 35) can choose their transport.

- [ ] Add `PtyTransport` preference to domain configuration:
  ```rust
  /// Which PTY transport to use for this domain.
  pub enum PtyTransport {
      /// ConPTY (default for local shells on Windows).
      ConPty,
      /// Raw pipes — no ConPTY translation (for VT-native shells).
      RawPipe,
      /// Try raw pipe first, fall back to ConPTY on failure.
      RawPipeWithFallback,
  }
  ```

- [ ] Update `LocalDomain::spawn_pane()` — on Windows, always use ConPTY (unchanged behavior)

- [ ] Update `WslDomain` (`oriterm_mux/src/domain/wsl.rs`, currently 45 lines) — currently a stub with `can_spawn() = false` and `state() = Detached`. The existing struct has `id: DomainId` and `distro: String` fields. Implement `spawn_pane()`:
  - Convert to directory module: `domain/wsl/mod.rs` + `domain/wsl/tests.rs` (sibling test pattern). The current `wsl.rs` becomes `wsl/mod.rs`.
  - Add `state: DomainState` field (replace hardcoded `Detached` return).
  - Implement `spawn_pane()` method (mirrors `LocalDomain::spawn_pane()` structure at `domain/local.rs:73-196` — same 8-step assembly pattern):
    - On Windows, use `PtyTransport::RawPipeWithFallback` (or `ConPty` if 53.2 showed resize doesn't work)
    - Build `PtyCommand` with `wsl.exe -d {distro} -- {shell}` (if shell specified) or `wsl.exe -d {distro}` (default login shell)
    - Try `spawn_raw_pipe()` first; if it fails, fall back to `spawn_conpty()` with `log::warn!`
    - Pass `PtyTransportKind::RawPipe` (or `ConPty` on fallback) to `PtyReader::new()` and `PaneParts`
    - Set `state` to `Attached` and `can_spawn()` to `true` after successful probe (detect WSL availability at construction time via `wsl.exe --status`)
  - On Unix, `WslDomain` is not applicable (WSL is a Windows concept) — keep `can_spawn() = false`
  - Remove `#[allow(dead_code)]` annotations from the struct and impl (they will be used)
  - Test: `test_wsl_domain_spawn_fallback` — mock a spawn failure, verify fallback to ConPTY and `log::warn!` emission. CI-runnable (mock-based).

- [ ] Update `spawn_pty()` to accept transport preference. Current signature: `pub fn spawn_pty(config: &PtyConfig) -> io::Result<PtyHandle>`. New signature adds transport:
  ```rust
  #[cfg(windows)]
  pub fn spawn_pty(config: &PtyConfig, transport: PtyTransport) -> io::Result<PtyHandle> {
      let cmd = build_command(config);
      match transport {
          PtyTransport::ConPty => windows::spawn_conpty(config, &cmd),
          PtyTransport::RawPipe => windows::spawn_raw_pipe(&cmd, DEFAULT_BUFFER_SIZE),
          PtyTransport::RawPipeWithFallback => {
              match windows::spawn_raw_pipe(&cmd, DEFAULT_BUFFER_SIZE) {
                  Ok(handle) => Ok(handle),
                  Err(e) => {
                      log::warn!("Raw pipe spawn failed, falling back to ConPTY: {e}");
                      windows::spawn_conpty(config, &cmd)
                  }
              }
          }
      }
  }
  
  // Unix: transport parameter is ignored (always uses native PTY).
  #[cfg(unix)]
  pub fn spawn_pty(config: &PtyConfig, _transport: PtyTransport) -> io::Result<PtyHandle> {
      let cmd = build_command(config);
      unix::spawn_unix_pty(config, &cmd)
  }
  ```
  Note: `LocalDomain::spawn_pane()` (`oriterm_mux/src/domain/local.rs:90`) calls `spawn_pty(&pty_config)` — update to `spawn_pty(&pty_config, PtyTransport::ConPty)` (Windows) or `spawn_pty(&pty_config, PtyTransport::default())` (cross-platform).

- [ ] Config integration: add `pty_transport` option to terminal config (default: `auto` which maps to `ConPty` for local, `RawPipeWithFallback` for WSL). This goes in `oriterm/src/config/` as part of the terminal profile config. Default `auto` means no user action needed.

- [ ] **WSLENV propagation for raw pipe WSL sessions.** The current `build_wslenv()` in `spawn.rs` sets `WSLENV` on the `CommandBuilder` to propagate env vars across the Win32/WSL boundary. For raw pipe WSL sessions, `wsl.exe` must still receive `WSLENV` so that `TERM`, `COLORTERM`, `ORITERM`, `TERM_PROGRAM`, and `TERM_PROGRAM_VERSION` propagate into the WSL environment. Verify that `WslDomain::spawn_pane()` calls `build_command()` (which calls `build_wslenv()`) before `spawn_raw_pipe()`, or sets WSLENV explicitly on the `PtyCommand`. Test: spawn `wsl.exe` via raw pipes, run `echo $TERM $COLORTERM` inside WSL, verify both are set.

- [ ] **Working directory (`--cd`) for raw pipe WSL sessions.** When `PtyConfig.working_dir` is set, `inject_wsl()` in `shell_integration/inject.rs:111-122` adds `--cd <path>` to the wsl.exe command. Verify this still works with raw pipe spawning — the `PtyCommand` args must include `--cd <path>` when working_dir is specified and the domain is WSL. Test: spawn `wsl.exe` with `--cd /tmp` via raw pipes, verify `pwd` output is `/tmp`. `#[cfg(windows)]`, `#[ignore]` (requires WSL).

**Test verification taxonomy for 53.4:**
- **CI-runnable**: `PtyTransport` enum construction, `spawn_pty()` transport parameter dispatch logic, `WslDomain` fallback mock tests, `spawn_pty` signature tests.
- **`#[ignore]` (requires WSL)**: WSLENV propagation, `--cd` working directory, `WslDomain::spawn_pane()` end-to-end.
- **Manual real-Windows**: Full interactive WSL session via raw pipes.

---

## 53.5 Image Protocol Verification

**File(s):** Test files only — no production code changes (images already work in `oriterm_core`)

Verify that image protocol data passes through raw pipes unmodified. This is the whole point — ConPTY strips DCS/APC sequences, raw pipes don't.

- [ ] Write verification tests (`#[cfg(windows)]`, may need real Windows for full validation):
  - `test_sixel_passthrough` — send a Sixel DCS sequence through raw pipe, verify byte-for-byte arrival:
    - Input: `\x1bPq#0;2;0;0;0#1;2;100;100;100#0!6~-#1!6~-\x1b\\`
    - Expected: identical bytes at reader (ConPTY would strip the entire DCS)
  - `test_kitty_graphics_passthrough` — send Kitty APC sequence:
    - Input: `\x1b_Ga=T,f=100,s=1,v=1,m=0;/9j/4A\x1b\\`
    - Expected: identical bytes at reader (ConPTY drops APC entirely)
  - `test_osc_passthrough` — send OSC 133 (shell integration), verify it arrives:
    - Input: `\x1b]133;A\x07`
    - Expected: identical bytes (ConPTY may pass some OSCs but not all)

- [ ] Write CI-runnable byte-identity unit tests (no WSL needed, pure comparison logic):
  - `test_dcs_sequence_structure` — verify the test DCS byte string used in passthrough tests is a well-formed Sixel sequence (starts with `\x1bP`, ends with `\x1b\\`). Guards against broken test fixtures.
  - `test_apc_sequence_structure` — same for Kitty APC (starts with `\x1b_G`, ends with `\x1b\\`).
  - These validate the test data itself, ensuring passthrough assertions are meaningful.

**Test verification taxonomy for 53.5:**
- **`#[ignore]` (requires WSL on Windows)**: All passthrough tests that spawn `wsl.exe`. These are the core verification tests.
- **CI-runnable**: Sequence structure tests (validate test fixtures).
- **Manual real-Windows**: visual confirmation that Sixel/Kitty images render correctly in a WSL raw pipe session.

- [ ] Document which image protocols work with each transport:
  | Protocol | ConPTY Classic | ConPTY Passthrough | ConPTY Sideloaded | Raw Pipe |
  |----------|---------------|-------------------|-------------------|----------|
  | Sixel (DCS) | Stripped | TBD | Supported (v1.22+) | Passed through |
  | Kitty (APC) | Stripped | TBD | TBD | Passed through |
  | iTerm2 (OSC 1337) | Partial | TBD | TBD | Passed through |

---

## 53.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 53.N Completion Checklist

- [ ] WSL shell spawns via raw pipes on real Windows — no `conhost.exe` created
- [ ] Image data (DCS, APC, OSC) passes through unmodified — verified with byte comparison (hex dump)
- [ ] Resize behavior documented with experimental findings from 53.2
- [ ] `WslDomain` uses `RawPipeWithFallback` (or `ConPty` if resize doesn't work) on Windows
- [ ] Fallback to ConPTY works when raw pipe spawn fails — `log::warn!` emitted
- [ ] Shell integration (OSC 133, OSC 7) works in raw pipe mode — verified with byte capture
- [ ] Ctrl+C (`0x03` byte write) stops processes in raw pipe sessions — end-to-end test
- [ ] Key encoding: `KeyInput` has `transport` field; raw pipe sessions use VT encoding only (not Win32 INPUT_RECORD)
- [ ] WSLENV propagation verified: `TERM`, `COLORTERM`, `ORITERM` visible inside WSL raw pipe session
- [ ] Working directory (`--cd`) propagation verified for WSL raw pipe sessions
- [ ] `pane/signal.rs` extracted from `pane/mod.rs` (Section 52.5 prerequisite — verify the extraction landed)
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `timeout 150 ./test-all.sh` green
- [ ] All source files under 500 lines
- [ ] Tests follow sibling `tests.rs` pattern
- [ ] Plan annotation cleanup: all temporary scaffolding removed
- [ ] All intermediate TPR checkpoint findings resolved
- [ ] **Plan sync** — update plan metadata:
  - [ ] This section's frontmatter `status` → `complete`
  - [ ] `00-overview.md` Quick Reference updated
  - [ ] `index.md` section status updated
  - [ ] Section 39 (Image Protocols) updated with note that raw pipe bypass enables image protocols on Windows
  - [ ] Section 35 (SSH Domain) updated with note that raw pipe infrastructure is available
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed — MUST run AFTER `/tpr-review` is clean

**Exit Criteria:** WSL shell runs via raw pipes with image protocol data passing through byte-identical. Resize behavior documented (working mechanism or documented limitation with ConPTY fallback for interactive sessions). Ctrl+C delivery works via `0x03` stdin write. Key encoding uses VT encoding (not Win32 INPUT_RECORD) for raw pipe sessions. Shell integration (OSC 133/7) and WSLENV propagation verified. Fallback to ConPTY tested and functional. `./build-all.sh && ./clippy-all.sh && timeout 150 ./test-all.sh` all green.
