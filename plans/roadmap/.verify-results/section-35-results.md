# Section 35: Session Persistence + Remote Domains -- Verification Results

**Verified:** 2026-03-29
**Status in plan:** not-started
**Actual status:** CONFIRMED NOT STARTED (with partial infrastructure from Section 44)

---

## Codebase Search Evidence

### 35.1 Session Save + Load

| Search | Result |
|--------|--------|
| `SessionSnapshot` in *.rs | **Not found** -- no session persistence types exist |
| `save_session` / `load_session` | **Not found** |
| `persistence` in oriterm_mux/src | **Not found** as module/directory -- no `persistence/` directory exists |
| `session save` anywhere | Only found in `oriterm/src/config/io.rs` (config persistence, not session) and `oriterm/src/gpu/state/pipeline_cache.rs` (GPU pipeline cache) |
| Config auto-save | `oriterm/src/config/mod.rs` has config saving, but nothing for session state |

**Verdict:** Truly not started. No session serialization infrastructure exists.

### 35.2 Crash Recovery

| Search | Result |
|--------|--------|
| `is_clean_shutdown` | **Not found** |
| `crash recovery` / `recover` | **Not found** |
| PID file handling | EXISTS in `oriterm_mux/src/server/pid_file.rs` -- basic PID file create/read/drop |
| Stale PID cleanup | EXISTS in `oriterm_mux/src/discovery/mod.rs` -- `validate_pid_file()` cleans stale PIDs |

**Verdict:** Not started. PID file handling exists (from Section 44) but crash detection/recovery (clean shutdown flag, recovery prompt) does not.

### 35.3 Scrollback Archive

| Search | Result |
|--------|--------|
| `ScrollbackArchive` | **Not found** |
| `archive` in oriterm_mux | Found only in snapshot.rs (render snapshot, not scrollback archive) |
| `scrollback` in oriterm_mux | Found in 14 files -- all relate to scrollback buffer *in memory*, not disk persistence |
| `mmap` | **Not found** |

**Verdict:** Truly not started. No disk-based scrollback archiving exists.

### 35.4 SshDomain

| Search | Result |
|--------|--------|
| `SshDomain` | **Not found** in any *.rs file |
| `ssh` in domain/ | **Not found** -- `oriterm_mux/src/domain/` has only `mod.rs`, `local.rs`, `wsl.rs`, `tests.rs` |
| `openssh` / `thrussh` | **Not found** in Cargo.toml dependencies |

**Verdict:** Truly not started. No SSH domain exists.

### 35.5 WslDomain Full Implementation

| Search | Result |
|--------|--------|
| `WslDomain` | EXISTS as stub in `oriterm_mux/src/domain/wsl.rs` (45 lines) |
| `can_spawn()` | Returns `false` -- stub only |
| `DomainState::Detached` | Always returns Detached |
| `wsl.exe` invocation | **Not found** |
| Path mapping (`win_to_wsl`, `wsl_to_win`) | **Not found** |
| `#[allow(dead_code)]` on WslDomain | Present with reason "used when WSL domain spawning is implemented" |
| `#[allow(unused_imports)]` on WslDomain export | Present with reason "used when WSL domain is wired in Section 35" |

**Verdict:** Stub exists (from earlier work), full implementation not started. The stub correctly implements the `Domain` trait but all methods return disabled/detached state.

### 35.6 tmux Control Mode Integration

| Search | Result |
|--------|--------|
| `TmuxDomain` / `tmux_control` / `tmux control` | **Not found** |
| `tmux` in domain code | **Not found** |
| Layout string parser | **Not found** |

**Verdict:** Truly not started.

---

## Infrastructure Available from Other Sections

The following infrastructure from Section 44 partially supports Section 35:

1. **Domain trait** (`oriterm_mux/src/domain/mod.rs`) -- `Domain` trait with `id()`, `name()`, `state()`, `can_spawn()`. SshDomain/WslDomain/TmuxDomain would implement this. However, the trait intentionally omits `spawn_pane()` (see doc comment: "Actual spawning requires I/O types...so spawn_pane is a concrete method").
2. **SpawnConfig** -- already exists with `cols`, `rows`, `shell`, `cwd`, `env`, `scrollback`, `shell_integration`.
3. **DomainState enum** -- `Attached` / `Detached` variants already defined.
4. **PID file + daemon lifecycle** -- basic infrastructure for daemon management exists.
5. **MuxBackend trait** -- the unified API over embedded/daemon mode. Session persistence would save the state accessible through this trait.
6. **InProcessMux** -- the pane-only multiplexer. Session persistence would serialize its state.

---

## Gap Analysis

### Plan Completeness

The plan is well-structured with 7 subsections covering distinct areas. Key observations:

1. **35.1 Session Save + Load** -- Plan references `WindowId`, `TabId`, `WindowSnapshot`, `TabSnapshot` in its data model. Per CLAUDE.md architecture, the mux is pane-only -- tabs/windows/sessions live in `oriterm/src/session/`. The plan correctly places the file in `oriterm_mux/src/persistence/session.rs` but the snapshot types reference session-layer concepts (windows, tabs) that live in `oriterm`. This is a crate boundary concern: either persistence belongs in `oriterm` (not `oriterm_mux`), or the session registry needs to export serializable types that the mux can consume.

2. **35.2 Crash Recovery** -- Reasonable design. The "prompt user via first connecting GUI client" pattern works with the existing daemon architecture.

3. **35.3 Scrollback Archive** -- Good design for unlimited scrollback. However, `mmap` is mentioned as "optional" -- given cross-platform requirements (Windows, Linux, macOS), `mmap` needs platform-specific implementation or a crate like `memmap2`.

4. **35.4 SshDomain** -- References "Section 30.2 stub" for WSL (not SSH). Plan suggests `openssh` or `thrussh` crate -- these have very different maturity levels and APIs. Should pick one. `russh` (successor to `thrussh`) is more actively maintained.

5. **35.5 WslDomain** -- References "Section 30.2 stub" but the stub is in `oriterm_mux/src/domain/wsl.rs`. The plan says file is `oriterm/src/domain/wsl.rs` -- this is wrong per crate boundaries (domains belong in `oriterm_mux`).

6. **35.6 tmux Control Mode** -- Ambitious. The tmux control mode protocol is complex and poorly documented. This could easily be its own section.

### Issues Found

1. **Crate boundary violation**: `SessionSnapshot` containing `WindowSnapshot`/`TabSnapshot` -- these session-layer types don't belong in `oriterm_mux`. The mux doesn't know about windows or tabs. Either: (a) persistence goes in `oriterm`, or (b) the mux gains a thin session persistence interface that `oriterm` calls with its session state.

2. **File path error**: 35.5 says `oriterm/src/domain/wsl.rs` but the actual stub is at `oriterm_mux/src/domain/wsl.rs`. The mux owns domains per crate-boundaries.md.

3. **Missing dependency**: No SSH crate in Cargo.toml. Need to choose between `openssh`, `russh`, or `ssh2`.

4. **tmux scope creep**: 35.6 is large enough to be its own section. Layout string parsing, bidirectional sync, session management -- each is substantial.

---

## Recommendation

1. Fix the crate boundary issue for session persistence (35.1/35.2) -- decide where the serialization logic lives.
2. Correct the file path for WslDomain (35.5) to match actual location.
3. Consider splitting tmux control mode (35.6) into its own section.
4. Pick a specific SSH crate rather than listing alternatives.
