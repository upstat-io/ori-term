# ori_term Rebuild — Overview

<!-- Last verified: 2026-04-02 -->

## Mandate

Rebuild ori_term from scratch. The old prototype proved the feature set (GPU window, PTY, VTE, tabs, fonts) but the architecture grew organically and became untenable: god objects, single-mutex contention, coupled VTE handler, rendering holding locks during GPU work, circular imports. The rebuild keeps all features but fixes the foundation with a multi-crate workspace, clean threading, and proper separation of concerns.

## Design Principles

1. **Bottom-up, one layer at a time** — Each layer solid and tested before the next begins.
2. **Crate boundary enforces separation** — `oriterm_core` knows nothing about GUI, fonts, PTY, config, or platform.
3. **Lock discipline** — Snapshot terminal state under lock (microseconds), release, then GPU work without lock.
4. **No god objects** — No struct exceeds ~12 fields. Responsibilities are singular.
5. **Term<T: EventListener>** — Generic terminal state machine, testable with `VoidListener`.
6. **Do it properly** — No workarounds, no hacks, no atomics-as-contention-fix. If it feels wrong, stop and redesign.

## Workspace Structure

```
ori_term/                           Workspace root
├── Cargo.toml                      [workspace] members: oriterm_core, oriterm_ui, oriterm_ipc, oriterm_mux, oriterm
├── oriterm_core/                   Pure terminal library (standalone, no workspace deps)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── unicode.rs              Unicode width utilities
│       ├── cell/                   Cell, CellFlags, CellExtra
│       ├── index/                  Point, Line, Column newtypes
│       ├── event/                  Event enum, EventListener trait
│       ├── sync/                   FairMutex
│       ├── grid/                   Grid, Row, Cursor, ring, scroll, editing
│       ├── term/                   Term<T>, VTE Handler, TermMode, CharsetState
│       ├── color/                  Palette, color resolution
│       ├── selection/              Selection model, boundaries, text extraction
│       ├── search/                 SearchState, find_matches
│       ├── image/                  Image storage, Kitty/Sixel/iTerm2 protocol parsing, cache
│       ├── paste/                  Paste filtering, bracketed paste
│       └── theme/                  Color scheme definitions
├── oriterm_ui/                     UI framework (depends on oriterm_core only)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── widgets/                Widget trait + all widget implementations
│       ├── window_root/            WindowRoot (per-window composition unit)
│       ├── interaction/            InteractionManager, resize geometry, cursor hiding
│       ├── pipeline/               Pipeline orchestration (layout → prepaint → paint → dispatch)
│       ├── compositor/             LayerTree, LayerAnimator, composition pass
│       ├── testing/                WidgetTestHarness (headless testing)
│       ├── action/                 Action types, keymap, dispatch
│       ├── icons/                  Vector icon path definitions
│       ├── animation/              Animation engine, easing, transitions
│       ├── controllers/            HoverController, ClickController, DragController, etc.
│       ├── draw/                   DrawList, DrawCommand, drawing primitives
│       ├── focus/                  FocusManager, tab order, focus ring
│       ├── geometry/               Point, Size, Rect, Insets
│       ├── layout/                 LayoutNode, flex, constraints
│       ├── overlay/                OverlayManager, modal, context menu
│       ├── visual_state/           VisualStateAnimator, state transitions
│       ├── text/                   ShapedText, TextStyle, TextMeasurer
│       └── theme/                  UiTheme, color tokens
├── oriterm_ipc/                    Platform IPC transport (standalone, no workspace deps)
│   ├── Cargo.toml
│   └── src/                        Unix domain sockets, Windows named pipes, mio integration
├── oriterm_mux/                    Pane server — flat pane lifecycle + PTY I/O (depends on oriterm_core, oriterm_ipc)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── in_process/             InProcessMux (pane CRUD, event pump)
│       ├── registry/               PaneRegistry (flat pane storage)
│       ├── pane/                   Pane (terminal state, PTY I/O)
│       ├── backend/                MuxBackend trait (embedded + daemon)
│       ├── server/                 Daemon server (IPC protocol)
│       ├── protocol/               Wire protocol (PDU codec)
│       ├── domain/                 Domain trait, LocalDomain, WslDomain
│       ├── id/                     PaneId, DomainId, ClientId newtypes
│       ├── mux_event/              MuxEvent, MuxNotification, MuxEventProxy
│       ├── pty/                    PTY spawning, reader thread
│       ├── discovery/              Daemon discovery, PID file
│       └── shell_integration/      Shell detection, injection scripts
├── oriterm/                        Application shell (consumes all other crates)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── app/                    App, event loop, input dispatch
│       ├── session/                GUI session model (tabs, windows, split trees, floating, nav)
│       ├── gpu/                    GpuState, renderer, atlas, pipelines
│       ├── font/                   FontCollection, shaping, discovery
│       ├── config/                 TOML config, file watcher
│       ├── key_encoding/           Kitty + legacy encoding
│       ├── clipboard/              Platform clipboard (Windows, Unix)
│       ├── cli/                    CLI argument parsing, subcommands
│       ├── event.rs                TermEvent user event enum
│       ├── keybindings/            Keybinding actions, dispatch
│       ├── platform/               Platform-specific window glue
│       ├── scheme/                 Color scheme loading
│       ├── url_detect/             URL detection, hover underline
│       ├── widgets/                App-level widget wrappers
│       ├── window/                 Window context, per-window state
│       └── window_manager/         Multi-window lifecycle management
├── crates/
│   └── vte/                        Vendored VTE parser fork (APC support added)
├── oriterm_tui/                    TUI client binary (FUTURE — not yet a workspace member)
│   └── (planned: terminal-in-terminal client, see Section 37)
├── _old/                           Old prototype (reference only)
├── assets/
└── plans/
```

## Dependency Graph

```
oriterm_ipc  (standalone — no oriterm_* deps)
oriterm_core (standalone — no oriterm_* deps)
oriterm_ui   → oriterm_core
oriterm_mux  → oriterm_core, oriterm_ipc
oriterm      → oriterm_core, oriterm_ui, oriterm_mux

oriterm (GUI binary)
     ├── oriterm_core, oriterm_ui, oriterm_mux
     ├── winit, wgpu, swash, rustybuzz
     ├── portable-pty, serde, toml, notify
     ├── window-vibrancy, tiny-skia
     └── clipboard-win / arboard

oriterm_ui (UI framework)
     ├── oriterm_core (Color reuse, geometry)
     ├── winit (WindowConfig, create_window only)
     └── (no GPU, no PTY, no mux, no config)

oriterm_mux (pane server)
     ├── oriterm_core, oriterm_ipc
     ├── portable-pty, serde, bincode
     └── (no GUI, no fonts, no windows, no session model)

oriterm_ipc (IPC transport)
     ├── mio
     └── (no oriterm_* deps)

oriterm_tui (FUTURE — TUI binary, not yet a workspace member)
     ├── oriterm_mux, oriterm_core
     ├── crossterm, clap
     └── (no GPU, no fonts, no windowing)
```

Strictly one-way. `oriterm_core` has zero knowledge of GUI, fonts, PTY, config, mux, or platform APIs. `oriterm_ui` depends only on `oriterm_core` for terminal types — no GPU, no PTY, no config. `oriterm_mux` is a flat pane server with zero knowledge of GUI, fonts, session model, or platform APIs. `oriterm` is the application shell that consumes all other crates and owns the session model (tabs, windows, split trees, floating panes, navigation). `oriterm_tui` will be a future headless client (Section 37).

## Threading Model

| Thread | Per | Owns | Lock Holds |
|--------|-----|------|------------|
| Main (UI) | process | winit EventLoop, windows, GpuState, GpuRenderer, FontCollection | microseconds (snapshot) |
| PTY Reader | pane | PTY read handle, read buffer, VTE Processor | microseconds (parse chunk) |
| Mux Server | process (daemon mode) | InProcessMux, socket listener, connections | microseconds (dispatch) |
| TUI Main | process (oriterm-tui) | crossterm event loop, MuxClient, TuiRenderer | microseconds (render frame) |

| Primitive | Per | Purpose |
|-----------|-----|---------|
| `FairMutex<Term<MuxEventProxy>>` | pane | Terminal state |
| `mpsc::channel<MuxEvent>` | process | Pane reader threads → mux event pump |
| `mpsc::channel<MuxNotification>` | process | Mux → GUI notification channel |
| `EventLoopProxy<TermEvent>` | process | Mux notifications → winit event loop wakeup |

**Critical pattern:** Lock → snapshot `RenderableContent` → unlock → GPU work (no lock held).

**Mux event flow (in-process):** PTY Reader → `MuxEvent` channel → `InProcessMux::poll_events()` → `MuxNotification` channel → GUI `about_to_wait()` → redraw.

**Mux event flow (daemon mode):** PTY Reader → `MuxEvent` → MuxServer → `OutputCoalescer` (1ms/16ms/100ms tiered) → push to client via IPC → GUI renders.

## Section Overview

### Tier 0 — Core Library + Cross-Platform Architecture
| Section | Title | What |
|---------|-------|------|
| 01 | Cell + Grid | Cell, Row, Grid, Cursor, scrollback, editing, navigation |
| 02 | Term + VTE | Terminal state machine, VTE Handler, modes, palette, SGR |
| 03 | Cross-Platform | Platform abstractions for PTY, fonts, clipboard, GPU, window (day one) |
| 44 | Multi-Process Window Architecture | Process-per-window, mux daemon, IPC protocol, tab migration |

### Tier 1 — Process Layer
| Section | Title | What |
|---------|-------|------|
| 04 | PTY + Event Loop | PTY spawning, reader thread, event proxy, lock discipline |

### Tier 2 — Rendering Foundation
| Section | Title | What |
|---------|-------|------|
| 05 | Window + GPU | winit window, wgpu pipeline (Vulkan/DX12/Metal), staged render pipeline (Extract→Prepare→Render), atlas, offscreen targets |
| 05B | Startup Performance | Zero-delay startup: parallel GPU init + font discovery, shader/glyph pre-caching |
| 05C | Window Chrome | Title bar, minimize/maximize/close controls, Aero Snap, caption hit testing |
| 06 | Font Pipeline | Multi-face loading, shaping, ligatures, fallback, built-in glyphs, emoji |
| 07 | 2D UI Framework | Drawing primitives, layout engine, widgets, overlay system (oriterm_ui crate) |
| 50 | Runtime Efficiency | Idle CPU elimination (`ControlFlow::Wait`), memory stability, allocation audit, profiling infrastructure |

### Tier 3 — Interaction
| Section | Title | What |
|---------|-------|------|
| 08 | Keyboard Input | Legacy + Kitty encoding, keyboard dispatch, IME |
| 09 | Selection & Clipboard | 3-point selection, word/line/block modes, clipboard, paste filtering |
| 10 | Mouse Input & Reporting | Mouse reporting modes, selection state machine, auto-scroll |
| 11 | Search | Plain text + regex search, search UI overlay, match highlighting |
| 12 | Resize & Reflow | Window resize, grid reflow, PTY resize notification |
| 13 | Config & Keybindings | TOML config, hot reload, file watcher, keybinding system, CLI subcommands |
| 14 | URL Detection | Implicit URL detection, hover underline, Ctrl+click open |
| 40 | Vi/Copy Mode | Modal navigation (hjkl), word/line/bracket motions, visual selection, yank, search integration |
| 41 | Hints & Quick Select | Regex-based pattern matching, keyboard-selectable labels, configurable actions |

### Tier 4 — Chrome (Tab Bar, Drag, Routing, Shell, Menus)
| Section | Title | What |
|---------|-------|------|
| ~~15~~ | ~~Tab Struct & Management~~ | *Superseded → Sections 30, 32* |
| 16 | Tab Bar & Chrome | Layout, rendering, hit testing, bell pulse, tab hover preview |
| 17 | Drag & Drop | Chrome-style drag, tear-off, OS drag, merge detection |
| ~~18~~ | ~~Multi-Window & Lifecycle~~ | *Superseded → Section 32* |
| 19 | Event Routing & Scheduling | Coordinate systems, dispatch, frame budget, cursor blink |
| 20 | Shell Integration | Shell detection, injection, OSC 7/133, prompt state, two-parser, semantic zones, command notifications |
| 21 | Context Menu & Window Controls | GPU-rendered menus, config reload, settings UI, window controls, taskbar jump list |

### Tier 4M — Multiplexing Foundation
| Section | Title | What |
|---------|-------|------|
| 29 | Mux Crate + Layout Engine | `oriterm_mux` crate (flat pane server), newtype IDs; SplitTree, FloatingLayer, spatial navigation, layout computation (implemented in `oriterm/src/session/`) |
| 30 | Pane Extraction + Domain System | Pane struct (from Tab), Domain trait, LocalDomain, WslDomain stub, registries, MuxEventProxy |
| 31 | In-Process Mux + Multi-Pane Rendering | InProcessMux, App rewiring, `prepare_pane_into()` with origin offsets, dividers, focus border, PaneRenderCache |
| 32 | Tab & Window Management (Mux-Aware) | Multi-tab via mux, multi-window shared GPU, tab CRUD, window lifecycle, cross-window tab movement, ConPTY-safe shutdown |
| 33 | Split Navigation + Floating Panes | Spatial navigation keybinds, divider drag resize, zoom/unzoom, floating pane creation/drag/resize, float-tile toggle, undo/redo |

### Tier 5 — Hardening
| Section | Title | What |
|---------|-------|------|
| 22 | Terminal Modes | Comprehensive DECSET/DECRST table, mode interactions |
| 23 | Performance & Damage Tracking | Row/column damage tracking, row-level dirty skip, fast ASCII path, alt screen lazy alloc, rendering benchmarks (ring buffer + frame throttling + pane caching already done) |
| 38 | Terminal Protocol Extensions | Capability reporting (DA, DECRQM, XTGETTCAP), color queries, extended SGR (underline styles/colors), window manipulation, DCS passthrough |
| 39 | Image Protocols | Kitty Graphics Protocol, Sixel, iTerm2 inline images, GPU compositing |
| 42 | Expose / Overview Mode | Mission Control-style live thumbnail grid of all panes, type-to-filter, keyboard/mouse navigation |
| 43 | Compositor Layer System | GPU-backed layer tree, render-to-texture composition, property animation (opacity, transform, bounds) |
| 45 | Security Hardening | Clipboard security (OSC 52), paste injection protection, escape sequence sandboxing, resource limits |
| 47 | Semantic Prompt State | Cell/row-level OSC 133 content tracking, prompt-aware resize, prompt navigation |
| 48 | Native OS Scrollbars | Overlay scrollbars, thumb drag, fade animation, platform-native look and feel |
| 49 | Advanced Keybinding System | Key tables (modal bindings), chained keybinds, catch-all keys, key remapping |

### Tier 6 — Polish
| Section | Title | What |
|---------|-------|------|
| 24 | Visual Polish | Cursor blink, hide-while-typing, minimum contrast, HiDPI, vector icons, background images, gradients, backdrop effects, scrollable menus |
| 25 | Theme System | 100+ themes, TOML theme files, discovery, light/dark auto-switch |
| 46 | macOS App Bundle & Platform Packaging | .app bundle, Info.plist, universal binary (x86_64+aarch64), DMG, CI build-macos job |

### Tier 7 — Advanced
| Section | Title | What |
|---------|-------|------|
| ~~26~~ | ~~Split Panes~~ | *Superseded → Sections 29, 31, 33* |
| 27 | Command Palette & Quick Terminal | Fuzzy search palette, global hotkey dropdown, notifications |
| 28 | Extensibility | Lua scripting, custom shaders, smart paste, undo close tab, session recording, workspaces |

### Tier 7A — Server + Persistence + Remote (NEW)
| Section | Title | What |
|---------|-------|------|
| 34 | IPC Protocol + Daemon Mode | Wire protocol (15-byte header, bincode+zstd), MuxServer daemon, OutputCoalescer (1ms push), MuxClient, auto-start daemon |
| 35 | Session Persistence + Remote Domains | Session save/load, crash recovery, scrollback archive, SshDomain, WslDomain full impl, tmux control mode |
| 36 | Remote Attach + Network Transport | TCP+TLS transport, SSH tunnel mode, authentication, MuxDomain for remote daemon, `oriterm connect` CLI, bandwidth-aware rendering |
| 37 | TUI Client | `oriterm-tui` binary — terminal-in-terminal client, attach/detach, prefix key, split/float rendering via crossterm, tmux replacement |

## Milestones

| Milestone | Section | What You See |
|-----------|---------|-------------|
| **M1: Lib compiles** | 01-02 complete | `cargo test -p oriterm_core` passes, Grid + VTE verified |
| **M2: Cross-platform foundations** | 03 complete | Platform abstractions defined for PTY, fonts, clipboard, GPU |
| **M3: Shell runs** | 04 complete | PTY spawns shell, I/O relayed (logged, no window) |
| **M4: Terminal renders** | 05 complete | Window opens, staged render pipeline, terminal grid visible, shell works |
| **M5: Full font pipeline** | 06 complete | Ligatures, emoji, fallback chains, box drawing, text decorations |
| **M6: UI framework** | 07 complete | Drawing primitives, layout engine, widgets, overlay system |
| **M7: Interactive** | 08-14, 40-41 complete | Keyboard, mouse, selection, clipboard, search, config, resize, URLs, vi mode, hints |
| **M8: Multiplexing** | 29-33 complete | Split panes, floating panes, multi-tab, multi-window — all through mux layer |
| **M8b: Chrome** | 16-17, 19-21 complete | Tab bar, drag/drop, event routing, shell integration, menus |
| **M9: Hardened** | 22-23, 38-39 complete | All terminal modes, protocol extensions, image protocols, performance optimized, damage tracking |
| | _Tier 5 also includes: 42, 43, 45, 47-49_ | _Expose, compositor (done), security, semantic prompts, scrollbars, advanced keybindings — no milestone assignment yet_ |
| **M10: Polished** | 24-25 complete | Cursor blink, 100+ themes, light/dark auto |
| | _Tier 6 also includes: 46_ | _macOS app bundle + platform packaging — no milestone assignment yet_ |
| **M11: Advanced** | 27-28 complete | Command palette, Lua scripting |
| **M12: Server mode** | 34-35 complete | Daemon keeps sessions alive, session persistence, SSH/WSL domains |
| **M13: Remote attach** | 36 complete | Connect GUI to remote daemon, SSH tunnel or TLS, bandwidth-aware rendering |
| **M14: TUI client** | 37 complete | `oriterm-tui` — headless attach/detach, terminal-in-terminal rendering, tmux replacement |

## Key References

All paths relative to `~/projects/reference_repos/console_repos/`.

### Terminal Core & State Machine
| What | Alacritty | Ghostty |
|------|-----------|---------|
| Terminal state (Term<T>) | `alacritty/alacritty_terminal/src/term/mod.rs` | `ghostty/src/terminal/Terminal.zig` |
| Event/callback system | `alacritty/alacritty_terminal/src/event.rs` | `ghostty/src/termio/message.zig` |
| Threading/synchronization | `alacritty/alacritty_terminal/src/sync.rs` (FairMutex) | `ghostty/src/Surface.zig` (3-thread model + mailboxes) |
| PTY event loop | `alacritty/alacritty_terminal/src/event_loop.rs` | `ghostty/src/termio/Termio.zig`, `ghostty/src/termio/Exec.zig` |

### Grid, Memory & Storage
| What | Alacritty | Ghostty |
|------|-----------|---------|
| Screen/grid | `alacritty/alacritty_terminal/src/grid/mod.rs` | `ghostty/src/terminal/Screen.zig` |
| Storage backend | `alacritty/alacritty_terminal/src/grid/storage.rs` (ring buffer) | `ghostty/src/terminal/PageList.zig` (page linked list + memory pools) |
| Page-based memory | — | `ghostty/src/terminal/page.zig` (contiguous page layout, offset pointers) |
| Resize/reflow | `alacritty/alacritty_terminal/src/grid/resize.rs` | `ghostty/src/terminal/PageList.zig` (resize within page structure) |

### Parsing & Performance
| What | Alacritty | Ghostty |
|------|-----------|---------|
| VTE parser | `alacritty/alacritty_terminal/src/vte/` (crate) | `ghostty/src/terminal/Parser.zig` |
| Stream processing | — | `ghostty/src/terminal/stream.zig` (SIMD-optimized) |
| SIMD acceleration | — | `ghostty/src/simd/vt.zig`, `ghostty/src/simd/codepoint_width.zig` |
| Damage tracking | `alacritty/alacritty_terminal/src/term/mod.rs` (dirty state) | `ghostty/src/terminal/page.zig` (Row.dirty), `ghostty/src/terminal/render.zig` |

### Terminal Features
| What | Alacritty | Ghostty |
|------|-----------|---------|
| Modes (DECSET/DECRST) | `alacritty/alacritty_terminal/src/term/mod.rs` | `ghostty/src/terminal/modes.zig` (comptime-generated, 8-byte packed) |
| Color/palette | `alacritty/alacritty_terminal/src/term/color.rs` | `ghostty/src/terminal/color.zig` (DynamicPalette with mask) |
| SGR attributes | `alacritty/alacritty_terminal/src/vte/ansi.rs` | `ghostty/src/terminal/sgr.zig` |
| Selection | `alacritty/alacritty_terminal/src/selection.rs` | `ghostty/src/terminal/Selection.zig` (3-point, tracked/untracked) |
| Key encoding | `alacritty/alacritty_terminal/src/term/mod.rs` | `ghostty/src/input/key_encode.zig` (Kitty + legacy) |
| OSC/DCS/CSI | `alacritty/alacritty_terminal/src/vte/ansi.rs` | `ghostty/src/terminal/osc.zig`, `ghostty/src/terminal/dcs.zig` |

### Rendering & Threading
| What | Alacritty | Ghostty |
|------|-----------|---------|
| Renderer thread | `alacritty/alacritty/src/renderer/mod.rs` | `ghostty/src/renderer/Thread.zig` (120 FPS timer, cursor blink) |
| Platform abstractions | `alacritty/alacritty/src/platform/` | `ghostty/src/apprt/` (macOS/Linux/Windows backends) |

### Old Prototype
| What | Where |
|------|-------|
| Old Cell/CellFlags | `_old/src/cell.rs` |
| Old GPU renderer | `_old/src/gpu/renderer.rs` |
| Old Grid | `_old/src/grid/mod.rs` |
| Old VTE handler | `_old/src/term_handler/mod.rs` |

## Anti-Patterns (explicitly forbidden)

1. **No god objects** — Max ~12 fields per struct. Split responsibilities.
2. **No lock during GPU work** — Snapshot under lock, render without lock.
3. **No separate VTE handler struct** — `Term<T>` implements `Handler` directly.
4. **No atomic workarounds** — Lock is held microseconds, no shadow state needed.
5. **No circular imports** — Crate boundary prevents it. Within binary, renderer receives data not domain objects.
6. **No rendering constants in grid** — Grid knows nothing about pixels.
7. **No `unwrap()` in library code** — Return `Result` or provide defaults.
