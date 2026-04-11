---
paths:
  - "oriterm_ipc/src/**"
  - "oriterm_ipc/tests/**"
---

# oriterm_ipc — IPC Transport

The canonical home for the platform IPC transport: Unix domain sockets on Linux/macOS, Windows named pipes on Windows, connection lifecycle (listen, accept, connect), and mio integration for async I/O. Standalone crate — depends on nothing else in the workspace (see `.claude/rules/crate-boundaries.md`).

The crate is **transport-only**. Protocol semantics (PDU types, serialization, framing) live in `oriterm_mux/src/protocol/`. `oriterm_ipc` just moves bytes.

## Cross-Platform Discipline

Every public API MUST have a `#[cfg(unix)]` and a `#[cfg(windows)]` implementation. macOS is covered by the Unix path but is also exercised by the macOS CI runner. **No platform left behind** — the project targets Linux, macOS, and Windows and all three must build and pass tests.

- **Listen**: `UnixListener::bind(path)` on Unix / `NamedPipeServer::new(name)` on Windows
- **Accept**: `listener.accept()` on both, returns a per-connection stream
- **Connect**: `UnixStream::connect(path)` on Unix / `NamedPipeClient::open(name)` on Windows
- **Mio integration**: both platforms register the stream with `mio::Poll` using the appropriate `Token` / `Interest`

The `#[cfg]` branches MUST be tested on all three platforms. Windows cross-compile from WSL: `cargo build --target x86_64-pc-windows-gnu`. Any branch without a counterpart is a broken platform — file as a bug.

## Safety

IPC transport is the one place in the workspace where `unsafe` FFI is unavoidable (platform socket / pipe primitives). Keep it minimal and well-scoped:

- Every `unsafe` block has a `// SAFETY:` comment explaining the invariants
- FFI boundary modules are the only `#[allow]`-ed exceptions to the workspace-level `unsafe_code = "deny"`
- FFI types use `std::ffi` (`c_char`, `c_int`, `OsStr`) not raw primitives
- Socket / pipe handles MUST be RAII-wrapped — leaking a file descriptor on error paths is a bug

## Connection Lifecycle

Every socket / pipe the crate opens MUST have a corresponding close on ALL exit paths:
- Normal close: `drop(stream)` releases the handle
- Error close: the `?` operator must propagate errors without leaking
- Process exit: kernel cleanup handles the rest, but a well-behaved daemon still closes explicitly

Never leak a connection into a global / static — the crate's public API takes and returns concrete stream types, not `'static` references.

## Testing

- **Unit tests** live in `oriterm_ipc/src/**/tests.rs` siblings per `.claude/rules/test-organization.md`
- **Integration tests** in `oriterm_ipc/tests/` cover listen → accept → send → recv on both platforms
- **Platform matrix**: CI runs on Linux, macOS, Windows. Local dev uses Linux native + Windows cross-compile

## Forbidden

- No protocol semantics — PDU types and serialization live in `oriterm_mux/src/protocol/`
- No dependency on `oriterm_core`, `oriterm_ui`, `oriterm_mux`, or `oriterm` — `oriterm_ipc` is standalone
- No session / pane / domain types — those live in `oriterm_mux`
- No `println!` debugging — use `log` macros
- No `unwrap()` outside of test code
