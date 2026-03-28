---
section: 37
title: TUI Client
status: not-started
reviewed: false
third_party_review:
  status: none
  updated: null
tier: 7A
goal: Headless terminal-in-terminal client (`oriterm-tui`) that connects to a local or remote `oriterm-mux` daemon, rendering panes inside the host terminal — the tmux-replacement experience
sections:
  - id: "37.1"
    title: "`oriterm-tui` Binary + Crate"
    status: not-started
  - id: "37.2"
    title: TUI Rendering Engine
    status: not-started
  - id: "37.3"
    title: Input Routing + Prefix Key
    status: not-started
  - id: "37.4"
    title: Attach / Detach / Session Management
    status: not-started
  - id: "37.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "37.5"
    title: Section Completion
    status: not-started
---

# Section 37: TUI Client

**Status:** Not Started
**Goal:** Build `oriterm-tui`, a terminal-based client that connects to an `oriterm-mux` daemon and renders panes inside the host terminal using escape sequences. This is the headless counterpart to the GPU-rendered GUI — it gives you the full `tmux attach` experience without a graphical display.

**Crate:** `oriterm_tui` (new workspace member — TUI client binary)
**Dependencies:** Section 36 (remote attach + network transport), Section 34 (wire protocol)
**Prerequisite:** Section 36 complete (for remote connections). Can also connect locally via Section 34.

**Inspired by:**
- tmux: the gold standard for TUI multiplexers — prefix key, attach/detach, split rendering
- Zellij: modern TUI multiplexer with floating panes, WASM plugins, better UX than tmux
- Byobu: tmux wrapper with enhanced status bar and key bindings
- abduco + dvtm: detach/attach + tiling, minimal and composable

**Key differentiator:** tmux and Zellij are both the server AND the client. ori_term separates them: `oriterm-mux` is the server (with session persistence, crash recovery, scrollback archiving from Section 35), and `oriterm-tui` is just one of two clients (the other being the GPU-rendered GUI). Attach from SSH with `oriterm-tui`, switch to the GUI when at your desk — same session, same panes, no disruption.

**Why this matters:** You SSH into your dev server. No X11, no Wayland, no GPU. But your `oriterm-mux` daemon is running with 12 panes across 4 tabs. `oriterm-tui attach` gives you everything — splits, floating panes, tab bar — rendered right in your SSH terminal. Disconnect, walk to your desk, `oriterm connect dev-server` opens the GPU GUI with the same session. Seamless.

---

## 37.1 `oriterm-tui` Binary + Crate

Set up the new workspace member. The TUI client is a standalone binary that depends on `oriterm_mux` (for `MuxClient`, protocol types) and `oriterm_core` (for terminal cell types).

**File:** `oriterm_tui/Cargo.toml`, `oriterm_tui/src/main.rs`

- [ ] Workspace member:
  ```toml
  # ori_term/Cargo.toml [workspace]
  members = ["oriterm_core", "oriterm_mux", "oriterm", "oriterm_tui"]
  ```
- [ ] `oriterm_tui/Cargo.toml`:
  ```toml
  [package]
  name = "oriterm-tui"
  version = "0.1.0"
  edition = "2024"

  [dependencies]
  oriterm_core = { path = "../oriterm_core" }
  oriterm_mux = { path = "../oriterm_mux" }
  crossterm = "0.28"
  log = "0.4"
  env_logger = "0.11"
  clap = { version = "4", features = ["derive"] }
  ```
  - [ ] No `ratatui` — raw crossterm for maximum control over cell-by-cell rendering
  - [ ] No `wgpu`, `winit`, `swash`, or any GUI dependencies — TUI is headless
- [ ] Crate structure:
  ```
  oriterm_tui/
  ├── Cargo.toml
  └── src/
      ├── main.rs           CLI entry point (clap)
      ├── app.rs            TuiApp — event loop, state, MuxClient
      ├── render.rs         Terminal-in-terminal rendering engine
      ├── input.rs          Input routing, prefix key handling
      ├── session.rs        Attach/detach/list/new-session commands
      ├── status_bar.rs     Bottom status bar (tabs, domain, time)
      └── theme.rs          TUI color adaptation (host terminal capabilities)
  ```
- [ ] `main.rs` CLI:
  ```
  oriterm-tui attach [session]     # attach to session (default: most recent)
  oriterm-tui new-session [name]   # create new session and attach
  oriterm-tui list                 # list sessions on daemon
  oriterm-tui detach               # send detach signal to attached client (rarely needed)
  oriterm-tui kill-session <name>  # close all panes in session
  ```
- [ ] Minimum host terminal requirements:
  - [ ] 256-color support (TERM contains "256color" or COLORTERM set)
  - [ ] UTF-8 locale
  - [ ] Minimum 80x24 terminal size
  - [ ] Truecolor preferred but not required — downgrade gracefully

**Tests:**
- [ ] CLI parsing: all subcommands parse correctly
- [ ] Crate compiles with minimal dependencies (no GUI crates)
- [ ] Workspace: `cargo build -p oriterm-tui` succeeds

---

## 37.2 TUI Rendering Engine

Render remote pane content inside the host terminal. This is terminal-in-terminal rendering: translate the remote `RenderableContent` (cells with 24-bit color, attributes, Unicode) into escape sequences that the host terminal understands.

**File:** `oriterm_tui/src/render.rs`, `oriterm_tui/src/theme.rs`

**Reference:** tmux `tty.c` (terminal output), Zellij `zellij-server/src/output/mod.rs`

- [ ] `TuiRenderer`:
  - [ ] Owns a `BufWriter<Stdout>` — all output buffered, flushed once per frame
  - [ ] Synchronized output: `CSI ? 2026 h` before frame, `CSI ? 2026 l` after (if host supports)
  - [ ] Double-buffered: maintain `current` and `previous` cell grids, only emit changes (diff rendering)
  - [ ] Frame rate: render on `PaneOutput` notification, not on a timer (event-driven)
- [ ] Screen layout:
  ```
  ┌─────────────────────────────────────────┐
  │ [Tab 1] [Tab 2] [Tab 3*]  domain:local │  ← tab bar (1 line)
  ├────────────────────┬────────────────────┤
  │                    │                    │
  │   Pane 1 (focused) │   Pane 2          │  ← pane area (rows - 2)
  │                    │                    │
  ├────────────────────┴────────────────────┤
  │ [session] 3 panes │ 14:32 │ 50ms RTT   │  ← status bar (1 line)
  └─────────────────────────────────────────┘
  ```
  - [ ] Tab bar: top line, shows tab titles, active tab highlighted
  - [ ] Status bar: bottom line, shows session info, pane count, time, connection quality
  - [ ] Pane area: remaining rows, split according to mux layout
- [ ] Cell-by-cell rendering:
  - [ ] For each visible pane: get `RenderableContent` from `MuxClient`
  - [ ] Map remote cells to local escape sequences:
    - [ ] Foreground/background color: `CSI 38;2;r;g;b m` (truecolor) or nearest 256 or 16
    - [ ] Bold, italic, underline, strikethrough, dim, inverse, hidden: standard SGR
    - [ ] Cursor position: `CSI row;col H`
  - [ ] Unicode passthrough: emit the same graphemes the remote pane has
  - [ ] Wide characters: CJK characters occupy 2 cells, emit with correct spacing
- [ ] Split border rendering:
  - [ ] Use Unicode box-drawing characters for split dividers:
    - [ ] Vertical: `│` (U+2502)
    - [ ] Horizontal: `─` (U+2500)
    - [ ] Corners/junctions: `┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼`
  - [ ] Active pane border: highlighted color (accent from theme)
  - [ ] Inactive pane border: dim/gray
- [ ] Color adaptation:
  - [ ] Detect host terminal color capability (from `COLORTERM`, `TERM`)
  - [ ] Truecolor → truecolor: direct passthrough
  - [ ] Truecolor → 256-color: nearest color mapping
  - [ ] Truecolor → 16-color: palette mapping with ANSI approximation
  - [ ] Respect `NO_COLOR`: strip all colors, use bold/underline for emphasis
- [ ] Floating pane rendering:
  - [ ] Floating panes rendered as overlays with box-drawing borders
  - [ ] Shadow effect: dim characters behind floating pane (if truecolor)
  - [ ] Z-order: highest floating pane drawn last (on top)
- [ ] Cursor rendering:
  - [ ] Show cursor in focused pane at correct position
  - [ ] Hide cursor in unfocused panes
  - [ ] Cursor shape passthrough: `DECSCUSR` to match remote cursor shape
- [ ] Resize handling:
  - [ ] `SIGWINCH` → re-query terminal size → resize all pane viewports via `MuxClient`
  - [ ] Redraw entire screen after resize (full frame, not diff)
  - [ ] Debounce rapid resizes (50ms)

**Tests:**
- [ ] Diff rendering: only changed cells emit escape sequences
- [ ] Color downgrade: truecolor → 256 → 16 mapping accuracy
- [ ] Box-drawing: split borders render correctly for 2x1, 1x2, 2x2 layouts
- [ ] Wide chars: CJK characters occupy correct width in output
- [ ] Cursor: correct position and shape in focused pane
- [ ] Resize: SIGWINCH triggers re-layout and redraw

---

## 37.3 Input Routing + Prefix Key

Route keyboard and mouse input from the host terminal to the correct remote pane. Use a prefix key (like tmux's `Ctrl+B`) for TUI client commands vs. pane input.

**File:** `oriterm_tui/src/input.rs`

**Reference:** tmux `key-bindings.c`, Zellij `zellij-client/src/input_handler.rs`

- [ ] Input modes:
  - [ ] **Normal mode**: all input forwarded to focused pane via `MuxClient::send_input()`
  - [ ] **Prefix mode**: entered via prefix key, next key is a TUI command
  - [ ] **Copy mode** (stretch): vi-like scrollback navigation (like tmux copy-mode)
- [ ] Prefix key:
  - [ ] Default: `Ctrl+B` (tmux default, familiar)
  - [ ] Configurable: `prefix_key = "Ctrl+B"` in config
  - [ ] Double-press: `Ctrl+B Ctrl+B` sends literal `Ctrl+B` to the pane
  - [ ] Timeout: if no second key within 2 seconds, cancel prefix mode
- [ ] Prefix commands (single key after prefix):
  ```
  c       Create new pane (split default direction)
  |       Split horizontal
  -       Split vertical
  x       Close focused pane (with confirmation)
  z       Zoom/unzoom focused pane
  f       Toggle floating pane
  n       Next tab
  p       Previous tab
  <num>   Switch to tab <num>
  w       Tab picker (interactive list)
  d       Detach from session
  [       Enter copy mode (stretch)
  ]       Paste from copy buffer
  o       Cycle focus to next pane
  ;       Toggle between last two panes
  {       Swap pane with previous
  }       Swap pane with next
  Space   Cycle through preset layouts
  ?       Show key bindings help
  :       Command prompt (stretch)
  ```
- [ ] Arrow keys in prefix mode:
  - [ ] `Prefix + Arrow`: navigate to pane in direction
  - [ ] `Prefix + Alt+Arrow`: resize focused pane in direction
- [ ] Mouse input (if host terminal supports mouse reporting):
  - [ ] Enable mouse: `CSI ? 1003 h` (all motion) + `CSI ? 1006 h` (SGR format)
  - [ ] Click in pane → focus that pane + forward click to remote pane
  - [ ] Click on tab bar → switch tab
  - [ ] Click on split border → start drag resize
  - [ ] Scroll → forward to focused pane (or scroll in copy mode)
- [ ] Input passthrough:
  - [ ] In normal mode: raw bytes forwarded to `MuxClient::send_input(pane_id, bytes)`
  - [ ] Special keys (arrows, F-keys, Home, End, etc.): encode per remote pane's keyboard mode
  - [ ] `Ctrl+C`, `Ctrl+Z`, etc.: forwarded to remote pane, NOT interpreted locally
  - [ ] Only the prefix key is intercepted by the TUI client

**Tests:**
- [ ] Normal mode: printable chars forwarded to correct pane
- [ ] Prefix mode: `Ctrl+B c` creates new pane
- [ ] Prefix timeout: no second key within 2s → cancel, resume normal mode
- [ ] Double prefix: `Ctrl+B Ctrl+B` sends literal `Ctrl+B` to pane
- [ ] Arrow navigation: prefix + arrow → focus moves to correct pane
- [ ] Mouse click: focus changes to clicked pane
- [ ] Special keys: Ctrl+C forwarded to remote, not caught locally

---

## 37.4 Attach / Detach / Session Management

Connect to a running daemon, show its session, and cleanly detach. Handle multiple sessions, session naming, and the first-attach experience.

**File:** `oriterm_tui/src/session.rs`, `oriterm_tui/src/app.rs`

**Reference:** tmux `session.c`, `client.c`

- [ ] `TuiApp` — main application struct:
  ```rust
  pub struct TuiApp {
      client: MuxClient,
      renderer: TuiRenderer,
      input: InputHandler,
      domain_id: DomainId,
      focused_pane: PaneId,
      should_quit: bool,
  }
  ```
- [ ] Attach flow:
  1. Parse CLI args → determine target (local daemon, remote host, session name)
  2. Connect to daemon via `MuxClient` (local socket or network via Section 36)
  3. Authenticate if remote
  4. List sessions → pick target session (or create new)
  5. Enter raw mode on host terminal (`crossterm::terminal::enable_raw_mode()`)
  6. Subscribe to all visible panes
  7. Enter event loop: read input → route; receive `PaneOutput` → render
- [ ] Detach flow:
  - [ ] `Prefix + d`: detach
  - [ ] Unsubscribe from all panes
  - [ ] Disconnect from daemon (session stays alive on daemon)
  - [ ] Restore host terminal: disable raw mode, restore cursor, clear alternate screen
  - [ ] Print "detached from session '<name>'" to stdout
  - [ ] Exit with code 0
- [ ] Session management:
  - [ ] `oriterm-tui list` → print sessions with format:
    ```
    SESSION     WINDOWS  PANES  CREATED           ATTACHED
    dev         3        8      2026-02-17 09:00  (attached)
    monitoring  1        4      2026-02-16 14:30
    ```
  - [ ] `oriterm-tui new-session [name]` → create session, attach
  - [ ] `oriterm-tui attach [session]` → attach to existing (default: most recent)
  - [ ] `oriterm-tui kill-session <name>` → close all panes, remove session
- [ ] RAII terminal cleanup:
  - [ ] Drop guard: `TuiCleanup` struct that restores terminal on drop
  - [ ] Panic hook: restore terminal before printing panic message
  - [ ] Signal handling: `SIGINT`, `SIGTERM`, `SIGHUP` → clean detach + restore
  - [ ] Never leave host terminal in raw mode on any exit path
- [ ] Event loop:
  - [ ] `crossterm::event::poll()` for host terminal input (10ms timeout)
  - [ ] `MuxClient` notification channel for pane output
  - [ ] Select/poll over both sources (input + pane output)
  - [ ] No busy-waiting — block on I/O when idle
- [ ] Remote attach:
  - [ ] `oriterm-tui attach --host dev-server` or `oriterm-tui attach --ssh dev.example.com`
  - [ ] Uses `RemoteMuxDomain` from Section 36 under the hood
  - [ ] Connection status shown in status bar
  - [ ] Disconnect → automatic detach with "connection lost" message
- [ ] Multiple clients:
  - [ ] Multiple `oriterm-tui` clients can attach to the same session
  - [ ] Each client has independent viewport (one can scroll while another types)
  - [ ] Focus is per-client (different clients can focus different panes)
  - [ ] All clients see the same output (server pushes to all subscribers)

**Tests:**
- [ ] Attach: connect to local daemon, enter raw mode, render panes
- [ ] Detach: `prefix + d` → raw mode disabled, terminal restored, exit 0
- [ ] Panic cleanup: simulate panic → terminal state restored
- [ ] Signal cleanup: SIGINT → clean detach
- [ ] Session list: correct format, correct pane/window counts
- [ ] New session: creates session, attaches, default tab created
- [ ] Remote attach: SSH tunnel established, session rendered
- [ ] Multi-client: two clients attached, both receive output

---

## 37.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 37.5 Section Completion

- [ ] All 37.1–37.4 items complete
- [ ] `oriterm-tui` binary: compiles as standalone, no GUI dependencies
- [ ] TUI rendering: cell-by-cell with diff, box-drawing borders, color adaptation
- [ ] Input: prefix key, pane forwarding, mouse support
- [ ] Attach/detach: clean lifecycle, RAII terminal cleanup
- [ ] Session management: list, new, attach, kill
- [ ] `cargo build --target x86_64-pc-windows-gnu -p oriterm-tui` — compiles
- [ ] `cargo clippy --target x86_64-pc-windows-gnu -p oriterm-tui` — no warnings
- [ ] `cargo test -p oriterm-tui` — all tests pass
- [ ] **Attach test**: `oriterm-tui attach` shows panes from running daemon
- [ ] **Detach test**: `prefix + d` cleanly exits, daemon keeps running
- [ ] **Reattach test**: detach → reattach → same session, no lost output
- [ ] **Split rendering test**: 2x2 pane layout renders correctly with box-drawing borders
- [ ] **Color test**: truecolor host → full color; 256-color host → downgraded; NO_COLOR → stripped
- [ ] **Remote test**: `oriterm-tui attach --ssh dev-server` connects and renders
- [ ] **Multi-client test**: two `oriterm-tui` clients showing same session
- [ ] **Interop test**: attach with `oriterm-tui`, detach, open same session with GUI `oriterm connect`

- [ ] `/tpr-review` passed — independent Codex review found no critical or major issues (or all findings triaged)

**Exit Criteria:** A user can SSH into a remote server and run `oriterm-tui attach` to get a full multiplexed terminal experience — splits, tabs, floating panes — all rendered in their SSH terminal. Detach, walk to their desk, open `oriterm connect` and see the exact same session in the GPU-rendered GUI. This is the tmux replacement: same sessions, two interfaces (TUI and GUI), seamless switching.
