<img src="assets/icon.svg" width="128" height="128" alt="ori-term">

# ori-term

A GPU-accelerated terminal emulator written from scratch in Rust.

ori-term combines terminal emulation, pane multiplexing, and a custom GPU-rendered UI framework into one application. Instead of stacking `terminal + tmux + scripts + glue`, ori-term collapses that stack into one coherent system.

Cross-platform (Windows, Linux, macOS) from day one — no platform is primary, no platform is an afterthought.

> **Status: alpha.** Core terminal emulation, GPU rendering, split/floating panes, tabs, and the UI framework are functional. Daemon mode, session persistence, remote attach, and the TUI client are on the [roadmap](#roadmap).

## Architecture

Five-crate workspace with strictly one-way dependencies:

```
oriterm_ipc  (standalone — platform IPC transport)
oriterm_core (standalone — terminal emulation library)
oriterm_ui   → oriterm_core (UI framework)
oriterm_mux  → oriterm_core, oriterm_ipc (pane server)
oriterm      → oriterm_core, oriterm_ui, oriterm_mux (application shell)
```

- **oriterm_core** — Pure terminal emulation (grid, VTE handler, selection, search, image protocols). No GUI, no PTY, no platform code.
- **oriterm_ui** — GPU-rendered widget framework (layout, interaction, animation, theming). Testable headlessly via `WidgetTestHarness`.
- **oriterm_mux** — Flat pane server (pane lifecycle, PTY I/O, lock-free snapshots). Each pane has a dedicated IO thread.
- **oriterm_ipc** — Platform IPC transport (Unix domain sockets, Windows named pipes).
- **oriterm** — Application shell (winit event loop, wgpu rendering, font pipeline, session model, configuration).

## Features

### Terminal Emulation
- Full VTE escape sequence handling — SGR, cursor, erase, scroll regions, alternate screen, OSC, DCS, CSI
- Text reflow on resize — cell-by-cell reflow handling wide characters, wrapped lines, and scrollback
- Ring buffer scrollback with configurable history
- Mouse reporting — X10, normal, button-event, any-event with SGR encoding
- Synchronized output (Mode 2026)
- Hyperlinks — OSC 8 explicit + implicit URL detection with hover underline and Ctrl+click
- Kitty keyboard protocol — progressive enhancement, CSI u encoding, mode stack
- Bracketed paste, focus events (Mode 1004), charset support (G0/G1/G2/G3, DEC special graphics)
- Device attributes (DA1/DA2/DA3), device status reports, mode query (DECRQM)
- Terminfo query (XTGETTCAP), settings query (DECRQSS)
- Color queries and reset (OSC 4/10/11/12/104/110/111/112)
- Extended underline styles (curly, dotted, dashed, double) and underline color (SGR 58/59)
- Overline (SGR 53/55), window manipulation (CSI t)

### Image Protocols
- Kitty Graphics Protocol — chunked transmission, placement, z-indexing, deletion, animation
- Sixel graphics — DCS-based decoding and display
- iTerm2 inline images — OSC 1337 imgcat-compatible
- GPU compositing — images rendered as GPU textures alongside the terminal grid
- LRU eviction with configurable memory limits

### GPU Rendering
- wgpu with Vulkan + DX12 on Windows, Vulkan on Linux, Metal on macOS
- Staged render pipeline — Extract (snapshot) → Prepare (pure CPU) → Render (GPU submission)
- Custom frameless window chrome — tab bar as title bar, pixel-drawn platform-specific window controls
- Window transparency — Mica/Acrylic on Windows, vibrancy on macOS
- Per-row damage tracking, instance buffer caching, partial updates, skip-present when idle

### Font Pipeline
- Text shaping via rustybuzz (HarfBuzz) with two-phase shaping and ligature support
- Multi-face fallback — Regular/Bold/Italic/BoldItalic + configurable fallback chain with cap-height normalization
- Built-in glyphs — box drawing, block elements, braille, powerline via lookup table rasterization
- Color emoji — RGBA atlas pages, VS15/VS16 presentation selectors
- Subpixel rendering — LCD ClearType-style per-channel alpha (RGB/BGR)
- Subpixel positioning — fractional offset quantization
- Font synthesis — synthetic bold (embolden) and synthetic italic (14-degree skew) when faces are unavailable
- Configurable OpenType feature tags (liga, calt, kern, custom)
- Guillotine-packed multi-page glyph atlas (2048x2048), LRU eviction, R8Unorm + Rgba8Unorm pages
- DPI-aware auto-detected hinting modes
- ASCII pre-cache for fast startup

### UI Framework
- GPU-rendered widgets — buttons, checkboxes, toggles, sliders, text inputs, dropdowns, labels, panels, scrollbars
- Drawing primitives — rects, rounded rects, shadows, gradients, borders
- Flexbox-style two-pass layout with Row/Column containers, Fixed/Fill/Hug sizing, padding, gap
- Overlay system — modals, context menus, tooltips
- Animation — easing functions, property transitions, animated values
- Dark/light themes derived from terminal palette with accent colors
- Widget-level hit testing with mouse capture and focus management
- Tab order, focus ring, keyboard accessibility
- Compositor layer system with scene caching and invalidation

### Tabs & Window Chrome
- Tab reordering with smooth animation
- Tab tear-off into new windows and merge back (Chrome-style drag)
- Tab hover preview — scaled-down live terminal thumbnail via offscreen render targets
- Bell indicator on inactive tabs
- GPU-rendered context menus with shadows and rounded corners
- Settings overlay with color scheme selector
- Platform-specific window controls (Windows rectangular, macOS circular)
- Frameless drag, double-click to maximize, Aero Snap on Windows

### Split Panes
- Horizontal (Ctrl+Shift+D) and vertical (Ctrl+Shift+E) splits
- Immutable split tree with structural sharing via Arc
- Spatial navigation — Alt+Arrow to focus by direction, Alt+[/] to cycle
- Drag-to-resize dividers with 5px hit zone, keyboard resize with Alt+Shift+Arrow
- Equalize pane sizes, zoom/unzoom (Ctrl+Shift+Z)
- Undo/redo split history
- Focus border — accent-colored on active, dimming on inactive
- Per-pane render cache with dirty checking

### Floating Panes
- Toggle floating overlay (Ctrl+Shift+F)
- Float-tile toggle (Ctrl+Shift+G) — move panes between floating and tiled
- Mouse-driven drag and resize with snap-to-edge
- Z-ordering with drop shadows

### Selection & Clipboard
- 3-point selection — anchor/pivot/end with sub-cell precision
- Word/line/block modes — double-click, triple-click, Alt+click
- Drag threshold (1/4 cell width) to prevent accidental selection
- Word boundaries — delimiter-class-aware expansion across soft wraps
- HTML formatted copy for pasting into rich text editors
- Copy on select (configurable), OSC 52 clipboard access
- Bracketed paste wrapping
- File drag-and-drop with auto-quoted path insertion
- Paste confirmation dialog for large/multi-line pastes (configurable)

### Mark Mode
- Modal navigation toggled with Ctrl+Shift+Space
- Cursor movement — hjkl, arrows, word/line motions, page scrolling
- Visual selection — character, word, line modes
- Yank (y) to copy selected text
- Search integration — /, ? forward/backward with n, N to cycle
- zz to center view

### Search
- Ctrl+F overlay with match count and navigation
- Plain text and regex search modes
- Highlighted matches across viewport and scrollback
- Next/previous keyboard navigation

### Multi-Window
- Multiple windows sharing GPU device and surface management
- Cross-window tab movement
- Per-window DPI / scale factor handling

### Color & Theming
- 24-bit truecolor, 256-color palette, 16-color ANSI
- 100+ built-in themes — Catppuccin, Dracula, Nord, Gruvbox, Solarized, Tokyo Night, and more
- TOML theme files with hot-reload via file watcher
- Light/dark auto-switch following system appearance
- Import from iTerm2, Ghostty, and base16 formats
- Minimum contrast enforcement (WCAG 2.0 luminance-based)

### Configuration
- TOML config — fonts, colors, keybindings, behavior, window settings
- Hot reload — file watcher triggers live config updates
- Configurable keybindings with action binding
- Font size zoom — Ctrl+=/Ctrl+-

### Shell Integration
- Shell detection — bash, zsh, fish, PowerShell
- OSC 7 CWD tracking
- OSC 133 semantic zone marking (prompt/input/output)
- Title management — OSC 2 or CWD short path
- Bell notification on background panes
- XTVERSION (CSI >q) identification response

### Visual Polish
- Cursor blink with DECSCUSR steady/blinking styles
- Hide cursor while typing (reappears on mouse move)
- HiDPI with proper per-monitor scale factor handling

### Cross-Platform
- **Windows** — ConPTY, DirectWrite font discovery, Vulkan + DX12, frameless with Aero Snap, Mica/Acrylic
- **Linux** — PTY, fontconfig, Vulkan, X11 + Wayland, D-Bus theme detection
- **macOS** — PTY, CoreText, Metal, native vibrancy (NSVisualEffectView)

### Performance
- Per-row dirty tracking to minimize GPU work
- Instance buffer caching with partial updates for changed rows
- Ring buffer scrollback — O(1) push
- Zero allocations in hot render path — lock-free snapshot transfer via `std::mem::swap()`
- Zero idle CPU beyond cursor blink (~1.89 Hz)
- Buffer shrink discipline — grow-only buffers with post-render `maybe_shrink()`

## Roadmap

These features are planned but not yet implemented:

- **Daemon mode & IPC** — background daemon keeping sessions alive, binary wire protocol, auto-start with fallback to in-process
- **Session persistence** — auto-save/restore of window/tab/pane layout, crash recovery, disk-backed scrollback archive
- **Remote domains** — SSH and WSL shell spawning with mixed local+remote panes
- **Remote attach** — TCP+TLS transport, predictive local echo (Mosh-style), bandwidth-aware rendering
- **TUI client** — terminal-in-terminal multiplexer (tmux replacement) connecting to the same daemon
- **Vi mode** — full vi-style navigation with count prefixes, f/t motions, bracket matching
- **Hints / Quick select** — vimium-style pattern matching for URLs, paths, git hashes with keyboard labels
- **Command palette** — Ctrl+Shift+P fuzzy-search action picker
- **Lua scripting** — event hooks, custom commands, post-processing shaders (WGSL)
- **Terminal inspector** — debug overlay with escape sequence log
- **Quick terminal** — global hotkey dropdown (Quake-style)
- **Background images** — PNG/JPEG background textures with configurable opacity
- **Smooth kinetic scrolling** — momentum scrolling for trackpads
- **Progress indicators** — OSC 9;4 taskbar progress
- **Desktop notifications** — native toast integration for OSC 9/99/777
- **Native scrollbars**
- **Rich status bar**
- **macOS app bundle**

See `plans/roadmap/` for detailed section-by-section status.

## Building

```bash
# Debug
cargo build --target x86_64-pc-windows-gnu

# Release
cargo build --target x86_64-pc-windows-gnu --release

# Checks
./clippy-all.sh
./test-all.sh
./build-all.sh
```

Cross-compiled from WSL targeting `x86_64-pc-windows-gnu`.

## Inspiration

| Project | What inspired us |
|---------|-----------------|
| Ghostty | Cell-by-cell text reflow approach |
| Alacritty | Term\<T\> architecture, FairMutex, VTE crate, strict clippy |
| WezTerm | Cross-platform PTY abstraction, multiplexer domain model |
| Chrome | Tab drag state machine, GPU-rendered UI, tab previews |
| VS Code | Frameless window chrome pattern |
| Windows Terminal | Selection behavior and clipboard UX |
| Bevy | Staged render pipeline (Extract → Prepare → Render) |
| tmux | Session persistence, daemon architecture, TUI multiplexing |
| Mosh | Predictive local echo, bandwidth-aware rendering |
| Catppuccin | Default color palette (Mocha) |
| Ratatui | Clippy lint configuration, testing patterns |
| termenv / lipgloss | Color profile detection cascade |

## The Name

**ori** — from the Japanese 折り (folding). Tabs fold between windows the way you fold paper.

## License

MIT
