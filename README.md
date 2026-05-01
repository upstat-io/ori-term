<img src="assets/icon.svg" width="128" height="128" alt="ori-term">

# ori-term

<p>
  A GPU-accelerated, multiplexer-native terminal emulator written from scratch in Rust.
  <br /><br />
  <a href="#about">About</a>
  ·
  <a href="https://oriterm.com/install">Install</a>
  ·
  <a href="https://oriterm.com/features">Features</a>
  ·
  <a href="https://oriterm.com/docs">Documentation</a>
  ·
  <a href="https://oriterm.com/roadmap">Roadmap</a>
</p>

> **Status: alpha.** Daily-driver-capable on Linux, Windows, and macOS. Core terminal emulation, GPU rendering, splits, floating panes, and the multiplexer foundation are functional. Daemon mode is the default `ProcessModel` with auto-start. Session persistence, remote attach, and the headless TUI client are still landing — see [Roadmap](#roadmap).

## About

ori-term is a terminal emulator that collapses three layers — terminal + multiplexer + UI shell — into a single coherent application:

- **GPU-rendered cells** through wgpu (Vulkan / DX12 / Metal). Per-row damage tracking, instance buffer caching, and skip-present when idle keep idle CPU near the cursor blink rate.
- **Native multiplexing.** Splits, tabs, floating panes, and cross-window tab movement live in `oriterm_mux`. A separate daemon process is the default; embedded mode is the fallback.
- **Cross-platform from day one.** Same source tree builds and runs on Windows, Linux, and macOS — ConPTY, fontconfig, CoreText, no platform left behind.

Full feature list and screenshots live on the website: [oriterm.com/features](https://oriterm.com/features).

## Install

Pre-built binaries for Linux, macOS, and Windows are on the [GitHub releases page](https://github.com/upstat-io/ori-term/releases).

On Linux and macOS, the install script picks the right binary, downloads it, and drops it in `~/.local/bin`:

```sh
curl -fsSL https://oriterm.com/install.sh | sh
```

See [oriterm.com/install](https://oriterm.com/install) for Windows downloads and per-platform notes.

## Build from source

The Cargo workspace lives in `term_repo/`. The toolchain is pinned in `rust-toolchain.toml` — `rustup` selects it automatically.

```sh
git clone https://github.com/upstat-io/ori-term.git
cd ori-term/term_repo
cargo build --release
```

To cross-compile for Windows from Linux/WSL:

```sh
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

`./build-all.sh` runs both targets; `./test-all.sh` runs the workspace test suite.

## Roadmap

| Step | Status |
|------|--------|
| Standards-compliant terminal emulation (VTE, modes, palette, SGR, OSC, DCS, CSI) | Complete |
| GPU pipeline (Extract → Prepare → Render, atlas, font shaping, damage tracking) | Complete |
| Cross-platform window chrome (frameless, Aero Snap, vibrancy) | Complete |
| Splits, floating panes, tabs, multi-window | Complete |
| Multiplexer foundation (mux daemon binary, IPC transport, pane lifecycle, embedded fallback) | Complete |
| Image protocols (Kitty Graphics, Sixel, iTerm2) — decoding shipped, animation/z-index hardening in progress | Partial |
| Multi-process window architecture — process-per-window topology over the mux daemon | Partial |
| Terminal protocol extensions (XTGETTCAP, broader CSI t window manipulation) | Partial |
| Session persistence + remote domains (SSH, WSL) | Todo |
| Remote attach + network transport (TCP+TLS, Mosh-style predictive echo) | Todo |
| Headless TUI client (`oriterm-tui` — tmux replacement) | Todo |
| Lua scripting + command palette + vi mode + hints | Todo |
| macOS app bundle, native scrollbars, minimap | Todo |

Live roadmap: [oriterm.com/roadmap](https://oriterm.com/roadmap).

## Inspiration

ori-term borrows ideas from many terminal emulators and UI frameworks: Alacritty (`Term<T>`, `FairMutex`, VTE crate), Ghostty (cell-by-cell reflow, comptime mode tables), WezTerm (multiplexer domain model, portable PTY), Chrome (tab drag state machine, GPU UI), VS Code (frameless chrome), Bevy (staged render pipeline), tmux (daemon architecture), Mosh (predictive echo), Catppuccin (default palette), Ratatui (clippy lint configuration), termenv / lipgloss (color profile detection cascade).

## The Name

**ori** — from the Japanese 折り (folding). Tabs fold between windows the way you fold paper.

## License

MIT.
