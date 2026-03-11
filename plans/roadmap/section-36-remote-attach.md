---
section: 36
title: Remote Attach + Network Transport
status: not-started
reviewed: false
tier: 7A
goal: Network transport for the wire protocol, authentication, MuxDomain for remote daemon connections, `oriterm connect` CLI, bandwidth-aware rendering over high-latency links
sections:
  - id: "36.1"
    title: Network Transport Layer
    status: not-started
  - id: "36.2"
    title: Authentication
    status: not-started
  - id: "36.3"
    title: MuxDomain (Remote Mux Connection)
    status: not-started
  - id: "36.4"
    title: "`oriterm connect` CLI"
    status: not-started
  - id: "36.5"
    title: Bandwidth-Aware Rendering
    status: not-started
  - id: "36.6"
    title: Section Completion
    status: not-started
---

# Section 36: Remote Attach + Network Transport

**Status:** Not Started
**Goal:** Extend the local IPC wire protocol (Section 34) to work over the network. Authenticate remote connections. Implement `MuxDomain` for connecting a local GUI to a remote `oriterm-mux` daemon. Build `oriterm connect` CLI. Add latency-aware rendering for smooth remote sessions.

**Crate:** `oriterm_mux` (transport, auth, domain), `oriterm` (CLI integration)
**Dependencies:** Section 34 (wire protocol + daemon working)
**Prerequisite:** Section 34 complete.

**Inspired by:**
- WezTerm: `wezterm connect` to remote mux server, SSH as transport, codec protocol over SSH channel
- tmux: `tmux attach -t <session>` over SSH, sessions survive disconnects
- Mosh: predictive local echo over UDP, latency-tolerant remote terminal
- Eternal Terminal (ET): reconnect-on-roam, SSH + encrypted channel

**Key differentiator:** Section 35 gives you "spawn an SSH shell from your local GUI" (SshDomain). This section gives you the inverse: "your entire session — all tabs, splits, floating panes — lives on a remote server, and you can attach from any machine." Disconnect, reconnect from a different machine, pick up exactly where you left off. Combined with Section 35's session persistence, this gives you sessions that survive reboots of both client and server.

**Two remote modes:**
1. **SSH tunnel mode** (simple, secure) — tunnel the existing local IPC protocol over `ssh -L`. Zero new attack surface. Works anywhere SSH works.
2. **Native TLS mode** (lower latency) — direct TCP + TLS connection. Eliminates SSH overhead for dedicated setups.

---

## 36.1 Network Transport Layer

Extend the wire protocol transport from local-only (Unix socket / named pipe) to also support TCP with TLS encryption. Keep local transport as the default; network transport is opt-in.

**File:** `oriterm_mux/src/transport.rs`, `oriterm_mux/src/transport/tls.rs`

**Reference:** WezTerm `wezterm-mux-server-impl/src/sessionhandler.rs`, ET `src/terminal/TerminalServer.cpp`

- [ ] `Transport` trait — abstracts over local and network transports:
  ```rust
  pub trait Transport: Send + 'static {
      type Listener: TransportListener;
      type Stream: AsyncRead + AsyncWrite + Send + Unpin;
      fn bind(config: &TransportConfig) -> Result<Self::Listener>;
  }

  pub trait TransportListener: Send + 'static {
      type Stream: AsyncRead + AsyncWrite + Send + Unpin;
      async fn accept(&mut self) -> Result<(Self::Stream, PeerInfo)>;
  }
  ```
- [ ] `LocalTransport` — wraps existing Unix socket / named pipe (Section 34.1):
  - [ ] No changes to existing local path — this is just a trait wrapper
  - [ ] `PeerInfo` includes PID (on Unix, via `SO_PEERCRED`)
- [ ] `TcpTlsTransport` — TCP listener with mandatory TLS:
  - [ ] `rustls` for TLS (no OpenSSL dependency)
  - [ ] Server certificate: auto-generated self-signed Ed25519 cert on first run
  - [ ] Certificate stored at `$XDG_STATE_HOME/oriterm/server.{cert,key}`
  - [ ] Client verifies server via TOFU (Trust On First Use) — fingerprint pinned after first connection
  - [ ] Known hosts file: `$XDG_STATE_HOME/oriterm/known_hosts` (similar to SSH)
  - [ ] TLS 1.3 only — no negotiation of older versions
- [ ] `SshTunnelTransport` — local socket forwarded over SSH:
  - [ ] Server side: daemon listens on local socket only (no TCP port open)
  - [ ] Client side: `ssh -L <local_sock>:<remote_sock> <host>` establishes tunnel
  - [ ] Client connects to local forwarded socket — existing local transport works unchanged
  - [ ] Advantage: zero new attack surface, reuses SSH auth, works behind firewalls
  - [ ] Managed by `oriterm connect --ssh <host>` (see 36.4)
- [ ] Server configuration:
  ```toml
  [mux_server]
  # Local transport (always enabled)
  local_socket = "auto"  # default: $XDG_RUNTIME_DIR/oriterm-mux.<pid>.sock

  # Network transport (opt-in)
  listen_address = "0.0.0.0:4622"  # empty = disabled
  tls_cert = "auto"                # "auto" = self-signed, or path to cert
  tls_key = "auto"
  ```
- [ ] Port: `4622` default (mnemonic: "462" ≈ "OT2", avoids conflicts with common services)
- [ ] Dual-stack: IPv4 + IPv6 (`[::]:4622` binds both on most systems)
- [ ] Connection limit: max 16 concurrent remote clients (configurable)

**Tests:**
- [ ] `Transport` trait: `LocalTransport` and `TcpTlsTransport` both satisfy trait bounds
- [ ] TLS handshake: server with self-signed cert, client connects with TOFU
- [ ] Known hosts: first connection pins fingerprint, subsequent connections verify
- [ ] Known hosts: changed fingerprint → connection refused with warning
- [ ] SSH tunnel: local socket forwarded, client connects through tunnel
- [ ] Connection limit: 17th client gets `ServerBusy` error
- [ ] Bind failure: port already in use → clear error message

---

## 36.2 Authentication

Authenticate remote clients before granting access to the mux session. Local connections (Unix socket) skip authentication — PID-based trust via `SO_PEERCRED`.

**File:** `oriterm_mux/src/auth.rs`

**Reference:** WezTerm uses SSH auth for remote mux. ET uses a passphrase-derived key.

- [ ] `AuthMethod` enum:
  ```rust
  pub enum AuthMethod {
      /// Local socket — trusted by PID (no auth needed)
      Local,
      /// SSH public key — client proves ownership of private key
      SshKey,
      /// Pre-shared token — for headless/CI/automated connections
      Token,
  }
  ```
- [ ] SSH key authentication (primary for interactive use):
  - [ ] Server has `authorized_keys` file: `$XDG_CONFIG_HOME/oriterm/authorized_keys`
  - [ ] Format: one public key per line (same as `~/.ssh/authorized_keys`)
  - [ ] Default: auto-populated with all keys from `~/.ssh/*.pub` on first run
  - [ ] Auth flow:
    1. Client sends `AuthChallenge { method: SshKey, public_key: Vec<u8> }`
    2. Server checks key against `authorized_keys`
    3. Server sends `AuthNonce { nonce: [u8; 32] }` (random challenge)
    4. Client signs nonce with private key, sends `AuthResponse { signature: Vec<u8> }`
    5. Server verifies signature → `AuthSuccess { session_token: [u8; 32] }` or `AuthFailed`
  - [ ] Supported key types: Ed25519, RSA (4096+), ECDSA
  - [ ] SSH agent integration: client uses `ssh-agent` to sign the nonce (never touches private key directly)
- [ ] Token authentication (for headless/CI use):
  - [ ] Server generates token: `oriterm-mux --generate-token` → prints token to stdout
  - [ ] Token stored in `$XDG_STATE_HOME/oriterm/tokens/<name>.token`
  - [ ] Client provides token: `oriterm connect --token <token> <host>`
  - [ ] Auth flow: client sends `AuthChallenge { method: Token, token: [u8; 32] }` → `AuthSuccess` or `AuthFailed`
  - [ ] Tokens are revocable: `oriterm-mux --revoke-token <name>`
- [ ] Session tokens (post-auth):
  - [ ] After successful auth, server issues a session token (valid for connection lifetime)
  - [ ] Session token used for reconnection without re-auth (within 5 minutes of disconnect)
  - [ ] Session tokens are random 256-bit values, stored in-memory only (not persisted)
- [ ] Rate limiting:
  - [ ] Max 5 failed auth attempts per source IP per minute
  - [ ] After limit: 30-second cooldown before accepting new attempts
  - [ ] Brute-force resistant: challenge-response, not password-based
- [ ] Server config:
  ```toml
  [mux_server.auth]
  authorized_keys = "~/.config/oriterm/authorized_keys"
  allow_token_auth = true
  max_auth_failures = 5
  ```

**Tests:**
- [ ] Local connection: no auth required, `SO_PEERCRED` trusted
- [ ] SSH key auth: valid key → success, wrong key → failure
- [ ] SSH key auth: nonce is unique per attempt (replay protection)
- [ ] Token auth: valid token → success, invalid token → failure
- [ ] Token revocation: revoked token → auth failure
- [ ] Session token: reconnect within 5 min → no re-auth needed
- [ ] Session token: reconnect after 5 min → re-auth required
- [ ] Rate limiting: 6th failure → cooldown response

---

## 36.3 MuxDomain (Remote Mux Connection)

A `Domain` implementation that proxies all operations to a remote `oriterm-mux` daemon. From the GUI's perspective, remote panes behave identically to local panes — the domain abstraction hides the network boundary.

**File:** `oriterm_mux/src/domain/remote.rs`

**Reference:** WezTerm `wezterm-mux-server-impl/src/domain/mod.rs`

- [ ] `RemoteMuxDomain`:
  - [ ] Implements `Domain` trait (Section 30)
  - [ ] `RemoteMuxDomain::new(id: DomainId, config: RemoteMuxConfig) -> Self`
  - [ ] Owns a `MuxClient` connected to the remote daemon
  - [ ] All `Domain` methods delegate to `MuxClient` calls over the network
- [ ] `RemoteMuxConfig`:
  ```rust
  pub struct RemoteMuxConfig {
      pub name: String,
      pub host: String,
      pub port: u16,           // default: 4622
      pub transport: RemoteTransport, // SshTunnel or DirectTls
      pub auth: RemoteAuth,    // SshKey or Token
      pub user: Option<String>,
      pub identity_file: Option<PathBuf>,
      pub connect_timeout: Duration,  // default: 10s
      pub auto_reconnect: bool,       // default: true
  }
  ```
- [ ] Connection lifecycle:
  1. `connect()` — establish transport (SSH tunnel or TLS), authenticate
  2. `sync()` — fetch remote mux state: all windows, tabs, panes, layout
  3. Populate local registries with remote state (marked as `domain_id` = this domain)
  4. Subscribe to all visible panes — start receiving `PaneOutput` push notifications
  5. GUI renders remote panes alongside any local panes
- [ ] Mixed local + remote:
  - [ ] A single GUI window can have both local tabs and remote tabs
  - [ ] Tab bar shows domain indicator: `[local]` vs `[dev-server]`
  - [ ] New pane defaults to same domain as the active pane
  - [ ] User can explicitly choose domain: `Ctrl+Shift+N` → domain picker
- [ ] Reconnection:
  - [ ] On disconnect: attempt reconnect every 2 seconds (exponential backoff, max 30s)
  - [ ] During disconnect: remote panes show "Reconnecting..." overlay
  - [ ] On reconnect: re-sync state, re-subscribe, shadow grid provides instant display
  - [ ] If reconnect fails after `max_reconnect_attempts` (default 30): mark domain as disconnected
  - [ ] User can manually reconnect: `oriterm reconnect <domain>`
- [ ] Pane state divergence:
  - [ ] Remote shell may have produced output while disconnected
  - [ ] On reconnect: server sends full `PaneContent` snapshot → client catches up
  - [ ] Shadow grid (Section 34.2) ensures no blank-screen flash
- [ ] Config:
  ```toml
  [[mux_domains]]
  name = "dev-server"
  host = "dev.example.com"
  port = 4622
  transport = "ssh"    # "ssh" (default, tunneled) or "tls" (direct)
  user = "eric"
  identity_file = "~/.ssh/id_ed25519"
  auto_connect = false  # connect on demand, not on startup
  ```

**Tests:**
- [ ] `RemoteMuxDomain` implements `Domain` trait — compile-time check
- [ ] Connect: SSH tunnel established, auth succeeds, state synced
- [ ] Spawn pane: remote shell created, output reaches local GUI
- [ ] Mixed mode: local pane + remote pane in same window, both functional
- [ ] Disconnect: panes show "Reconnecting..." overlay
- [ ] Reconnect: state re-synced, panes resume, no output lost
- [ ] Config parsing: all fields deserialize correctly

---

## 36.4 `oriterm connect` CLI

CLI subcommand to launch the GUI connected to a remote daemon. Also supports managing remote connections: list sessions, disconnect, reconnect.

**File:** `oriterm/src/cli.rs`, `oriterm/src/app/connect.rs`

- [ ] CLI subcommands:
  ```
  oriterm                          # normal launch (local daemon or in-process)
  oriterm connect <name|host>      # launch GUI connected to remote daemon
  oriterm connect --ssh <host>     # explicit SSH tunnel mode
  oriterm connect --list           # list configured mux domains
  oriterm connect --status         # show connection status for all domains
  ```
- [ ] `oriterm connect <name>`:
  - [ ] Look up `<name>` in `[[mux_domains]]` config
  - [ ] Establish connection (SSH tunnel or TLS based on config)
  - [ ] Authenticate
  - [ ] Sync remote state
  - [ ] Open GUI window showing remote session
  - [ ] If `<name>` not in config: treat as hostname, use defaults (SSH tunnel, port 4622)
- [ ] `oriterm connect --ssh <host>`:
  - [ ] Force SSH tunnel mode regardless of config
  - [ ] Spawn `ssh -L <local_sock>:/run/user/<uid>/oriterm-mux.<pid>.sock <host>`
  - [ ] Auto-detect remote socket path via `ssh <host> ls /run/user/*/oriterm-mux.*.sock`
  - [ ] Connect to local forwarded socket
  - [ ] If no remote daemon running: offer to start one (`ssh <host> oriterm-mux --daemon`)
- [ ] `oriterm connect --list`:
  - [ ] Print configured `[[mux_domains]]` with connection status
  - [ ] Format: `name | host:port | transport | status`
- [ ] First-connect experience:
  - [ ] If remote daemon has an existing session: attach to it (show all tabs/panes)
  - [ ] If remote daemon is empty: create default tab with shell in home directory
  - [ ] If no remote daemon: offer to start one
- [ ] Disconnect handling:
  - [ ] GUI close: disconnect from remote (remote session stays alive)
  - [ ] `Ctrl+Shift+D`: detach from remote domain (close remote tabs, keep local)

**Tests:**
- [ ] CLI parsing: all subcommands parse correctly
- [ ] Connect by name: resolves from config, connects
- [ ] Connect by hostname: uses defaults, connects
- [ ] SSH tunnel: spawns ssh process, forwards socket
- [ ] Remote daemon discovery: finds socket path on remote host
- [ ] List: shows configured domains with correct status

---

## 36.5 Bandwidth-Aware Rendering

Adapt rendering behavior based on connection quality. High-latency or low-bandwidth links need different strategies than local connections.

**File:** `oriterm_mux/src/transport/bandwidth.rs`, `oriterm/src/app/remote_render.rs`

**Reference:** Mosh `src/network/transport-sender.cc` (predictive echo), WezTerm codec compression

- [ ] Connection quality measurement:
  - [ ] Ping: periodic `Ping` → `Pong` messages on the wire protocol (every 1 second)
  - [ ] RTT tracking: exponentially weighted moving average (EWMA)
  - [ ] Bandwidth estimation: track bytes sent/received per second
  - [ ] Jitter: standard deviation of RTT samples
  - [ ] Quality classification:
    - [ ] **Local** (< 1ms RTT): full fidelity, no adaptation
    - [ ] **LAN** (1-10ms RTT): minor coalesce increase
    - [ ] **WAN** (10-100ms RTT): aggressive coalescing, higher compression
    - [ ] **High-latency** (> 100ms RTT): predictive echo, viewport-first rendering
- [ ] Adaptive coalescing:
  - [ ] Increase `OutputCoalescer` windows based on RTT:
    - [ ] Local: 1ms focused / 16ms visible / 100ms hidden (default)
    - [ ] WAN: 16ms focused / 50ms visible / 250ms hidden
    - [ ] High-latency: 33ms focused / 100ms visible / 500ms hidden
  - [ ] Smooth transitions — don't oscillate on RTT fluctuations
- [ ] Adaptive compression:
  - [ ] Low latency: zstd level 1 (fast, ~3x ratio)
  - [ ] High latency: zstd level 3 (slower but ~4-5x ratio) — trade CPU for bandwidth
  - [ ] Very high latency: consider delta encoding (diff from last frame)
- [ ] Viewport-first rendering:
  - [ ] On reconnect or large state change: send visible viewport first
  - [ ] Scrollback synced in background chunks (don't block rendering)
  - [ ] User scrolls into un-synced region → fetch on demand
- [ ] Predictive local echo (optional, experimental):
  - [ ] For printable characters: display locally before server confirmation
  - [ ] Track unconfirmed predictions, reconcile when server state arrives
  - [ ] If prediction wrong (e.g., tab completion changed output): snap to server state
  - [ ] Disable for non-printable keys (arrows, Ctrl+*, etc.)
  - [ ] Config: `predictive_echo = "off" | "cautious" | "aggressive"` — default "off"
- [ ] Connection quality indicator:
  - [ ] Small indicator in tab bar or status area for remote tabs
  - [ ] Color: green (< 50ms) / yellow (50-200ms) / red (> 200ms)
  - [ ] Shows RTT on hover

**Tests:**
- [ ] RTT measurement: synthetic ping/pong, EWMA converges
- [ ] Quality classification: correct bucket for various RTT values
- [ ] Adaptive coalesce: higher RTT → wider coalesce window
- [ ] Viewport-first: on reconnect, visible panes get data before scrollback
- [ ] Predictive echo: printable char displayed immediately, reconciled on ack
- [ ] Predictive echo: wrong prediction snaps to server state

---

## 36.6 Section Completion

- [ ] All 36.1–36.5 items complete
- [ ] Network transport: TCP+TLS with rustls, TOFU certificate pinning
- [ ] SSH tunnel: `ssh -L` forwarding, auto-detect remote socket
- [ ] Authentication: SSH key challenge-response, token auth, rate limiting
- [ ] MuxDomain: remote panes rendered alongside local panes, reconnection
- [ ] `oriterm connect`: CLI for connecting to remote daemons
- [ ] Bandwidth-aware: adaptive coalescing, compression, viewport-first, quality indicator
- [ ] `cargo build --target x86_64-pc-windows-gnu` — compiles
- [ ] `cargo clippy --target x86_64-pc-windows-gnu` — no warnings
- [ ] `cargo test` — all tests pass
- [ ] **LAN test**: connect to daemon on same network, type commands, < 5ms visible latency
- [ ] **WAN test**: connect over internet (or simulated 50ms RTT), smooth rendering
- [ ] **Reconnect test**: kill SSH tunnel, re-establish → session resumes, no output lost
- [ ] **Mixed mode test**: local tab + remote tab in same window, both fully functional
- [ ] **Auth test**: unauthorized client rejected, authorized client connects
- [ ] **TOFU test**: first connection pins cert, changed cert → warning

**Exit Criteria:** A user can run `oriterm connect dev-server` from their laptop, get a full GUI window rendering their remote session with all tabs and splits intact. Disconnect WiFi, reconnect, and the session resumes exactly where it was. Remote panes are indistinguishable from local panes in day-to-day use. SSH tunnel mode works with zero server configuration beyond having `oriterm-mux` installed.
