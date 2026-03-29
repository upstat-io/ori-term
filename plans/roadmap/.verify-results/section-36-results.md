# Section 36: Remote Attach + Network Transport -- Verification Results

**Verified:** 2026-03-29
**Status in plan:** not-started
**Actual status:** CONFIRMED NOT STARTED

---

## Codebase Search Evidence

### 36.1 Network Transport Layer

| Search | Result |
|--------|--------|
| `Transport` trait (generic) | **Not found** -- the existing transport is hardcoded to local IPC |
| `TcpTlsTransport` / `rustls` / `TLS` | **Not found** in any *.rs file |
| `SshTunnelTransport` | **Not found** |
| `authentication` / `AuthMethod` | **Not found** |
| `listen_address` / port 4622 | **Not found** |
| Current transport | `oriterm_ipc` provides `IpcListener`, `IpcStream`, `ClientStream` -- Unix sockets + Windows named pipes only |
| `oriterm_mux/src/transport.rs` | **Does not exist** |
| `rustls` in Cargo.toml | **Not found** |

**Verdict:** Truly not started. No network transport, no TLS, no Transport trait abstraction.

### 36.2 Authentication

| Search | Result |
|--------|--------|
| `AuthMethod` / `AuthChallenge` / `AuthNonce` | **Not found** |
| `authorized_keys` | **Not found** |
| `SO_PEERCRED` | **Not found** |
| `rate_limit` / `auth_failure` | **Not found** |
| `oriterm_mux/src/auth.rs` | **Does not exist** |
| Current auth model | None -- local IPC is implicitly trusted (localhost only, same user) |

**Verdict:** Truly not started. The current architecture has zero authentication (appropriate for local-only IPC).

### 36.3 MuxDomain (Remote Mux Connection)

| Search | Result |
|--------|--------|
| `RemoteMuxDomain` / `RemoteMuxConfig` | **Not found** |
| `oriterm_mux/src/domain/remote.rs` | **Does not exist** |
| Domain implementations | Only `LocalDomain` (active) and `WslDomain` (stub) exist |
| "Reconnecting..." overlay | **Not found** |

**Verdict:** Truly not started.

### 36.4 `oriterm connect` CLI

| Search | Result |
|--------|--------|
| `oriterm connect` | **Not found** in CLI code |
| `oriterm/src/cli.rs` | **Not found** -- CLI is in `oriterm/src/cli/mod.rs` |
| CLI subcommands | `oriterm/src/cli/mod.rs` exists but grep for "connect" shows nothing |
| `oriterm/src/app/connect.rs` | **Does not exist** |

**Verdict:** Truly not started.

### 36.5 Bandwidth-Aware Rendering

| Search | Result |
|--------|--------|
| `bandwidth` / `RTT` / `EWMA` / `predictive_echo` | **Not found** |
| `oriterm_mux/src/transport/bandwidth.rs` | **Does not exist** |
| `oriterm/src/app/remote_render.rs` | **Does not exist** |
| Connection quality measurement | **Not found** -- existing Ping/PingAck is for diagnostics only, not adaptive |
| Existing Ping/PingAck PDUs | YES -- `MuxPdu::Ping` and `MuxPdu::PingAck` exist, `MuxClient::ping_rpc()` measures round-trip |

**Verdict:** Truly not started. The Ping/PingAck mechanism exists but is purely diagnostic.

---

## Infrastructure Available from Other Sections

1. **oriterm_ipc crate** -- clean platform abstraction for local IPC. A `Transport` trait could generalize this to also cover TCP+TLS, but the current types are not trait-based.
2. **MuxBackend trait** -- the unified API. A `RemoteMuxDomain` would use `MuxClient` under the hood, which already implements `MuxBackend`.
3. **Domain trait** -- ready for `RemoteMuxDomain` to implement.
4. **MuxClient::ping_rpc()** -- measures IPC round-trip. Could be extended for network quality measurement.
5. **Push-based snapshot delivery** -- the 16ms push interval + backpressure already provides a foundation for network-aware coalescing.
6. **MuxPdu variants** -- the wire protocol already supports all pane operations. No new PDU types needed for basic remote operation (only for auth handshake).

---

## Gap Analysis

### Plan Completeness

The plan is ambitious but well-structured. Six subsections with clear responsibilities.

### Issues Found

1. **Heavy dependency chain**: Section 36 depends on Section 34 which depends on Section 44. But Section 34's plan is stale (see section-34-results.md). The actual dependency should be: Section 36 depends on Section 44 (already complete) + the genuine hardening from Section 34 (compression, version negotiation).

2. **Transport trait generalization**: The plan describes a `Transport` trait that abstracts over local and network transports. This is sound, but the existing `oriterm_ipc` types (`IpcListener`, `IpcStream`, `ClientStream`) are concrete, not trait-based. Retrofitting them behind a trait is non-trivial -- the `mio` integration (polling, event registration) differs significantly between Unix sockets and TCP sockets.

3. **rustls dependency**: Adding `rustls` is a significant dependency. The plan correctly avoids OpenSSL. TLS 1.3-only is a good choice.

4. **SSH tunnel mode is simpler**: The SSH tunnel approach (`ssh -L`) is much simpler than native TLS and should be the primary path. The plan correctly identifies this but puts TLS first in the subsection ordering. Consider reordering to prioritize SSH tunnel.

5. **TOFU certificate management**: Trust-On-First-Use is a good choice for self-signed certs but needs careful UX (what happens on cert change? how does the user verify fingerprints?). The plan mentions a `known_hosts` file but doesn't detail the UX.

6. **Port choice**: 4622 is reasonable. Not in IANA's assigned range for well-known services. Low collision risk.

7. **Predictive local echo (36.5)**: This is marked "optional, experimental" which is appropriate. Mosh's implementation is complex and requires careful state reconciliation. The plan correctly defaults to "off".

8. **Scope is very large**: This section spans network transport, TLS, authentication (SSH key challenge-response!), a new domain type, a CLI subcommand, and adaptive rendering. This is realistically 3-4 sections of work.

### Missing Items

1. **Connection encryption for SSH tunnel mode** -- the plan assumes SSH provides encryption, but doesn't mention that the inner protocol (local socket forwarded through SSH) is still unencrypted. This is fine because the SSH tunnel encrypts the transport, but it should be explicitly documented.

2. **Firewall/NAT traversal** -- no mention of how remote connections work through firewalls. SSH tunnel handles this implicitly; TLS mode does not.

3. **Graceful degradation** -- what happens if TLS handshake fails? Fallback to SSH tunnel? Or hard fail?

---

## Recommendation

1. This section is too large. Consider splitting: (a) SSH tunnel transport + `oriterm connect`, (b) Native TLS + authentication, (c) Bandwidth-aware rendering.
2. Fix the dependency chain to reference Section 44 directly (not the stale Section 34).
3. Prioritize SSH tunnel mode over native TLS -- it's simpler, more secure (reuses SSH infrastructure), and works through firewalls.
4. Predictive echo should be a stretch goal or deferred to a later section.
