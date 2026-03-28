---
section: 28
title: Extensibility
status: not-started
reviewed: false
third_party_review:
  status: none
  updated: null
tier: 7
goal: Lua scripting, custom shaders, smart paste, undo close tab, session recording, workspaces
sections:
  - id: "28.1"
    title: Scripting Layer
    status: not-started
  - id: "28.2"
    title: Custom Shaders
    status: not-started
  - id: "28.3"
    title: Smart Paste
    status: not-started
  - id: "28.4"
    title: Undo Close Tab
    status: not-started
  - id: "28.5"
    title: Session Recording + Playback
    status: not-started
  - id: "28.6"
    title: Workspaces
    status: not-started
  - id: "28.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "28.7"
    title: Section Completion
    status: not-started
---

# Section 28: Extensibility

**Status:** Not Started
**Goal:** Long-term differentiation features -- scripting, shaders, smart paste, and quality-of-life additions that make ori_term uniquely powerful.

**Crate:** `oriterm` (app + rendering layer)
**Dependencies:** `mlua` (Lua 5.4, vendored), existing wgpu pipeline

**Why this matters:** Ghostty has no scripting. WezTerm has Lua but poor performance. Kitty has kittens but they're Python. A well-designed scripting layer is the long-term moat -- the feature that creates an ecosystem around ori_term that competitors can't easily replicate.

**Current state:** No scripting engine, no custom shader support, no smart paste logic, no undo-close-tab stack. Bracketed paste mode is supported but pastes are forwarded raw. The wgpu rendering pipeline uses WGSL shaders that could be extended with a post-processing pass.

---

## 28.1 Scripting Layer

Programmable terminal via embedded Lua scripting engine.

**File:** `oriterm/src/scripting/mod.rs` (ScriptEngine), `oriterm/src/scripting/api.rs` (Rust-to-Lua API), `oriterm/src/scripting/events.rs` (event callbacks)

**Reference:** WezTerm's Lua config (`~/projects/reference_repos/console_repos/wezterm/`), `_old/src/app/mod.rs` (action dispatch)

**Recommended: Lua via `mlua`** -- proven by WezTerm, largest ecosystem for terminal scripting, fast (LuaJIT-class performance), tiny memory footprint, battle-tested embedding story. `mlua` supports Lua 5.4 and LuaJIT in pure Rust bindings (no C toolchain needed with `vendored` feature).

- [ ] Add `mlua` dependency with `lua54` + `vendored` features
- [ ] Create `src/scripting/` module:
  - [ ] `mod.rs` -- `ScriptEngine` struct, initialization, error handling
  - [ ] `api.rs` -- Rust functions exposed to Lua
  - [ ] `events.rs` -- event callback registration
- [ ] Lua API surface:
  ```lua
  -- Terminal state
  oriterm.active_tab()          -- returns TabId
  oriterm.active_pane()         -- returns PaneId
  oriterm.cursor_position()     -- returns {row, col}
  oriterm.cwd()                 -- returns current working directory
  oriterm.grid_size()           -- returns {cols, rows}
  oriterm.title()               -- returns tab title
  oriterm.scrollback_size()     -- returns number of scrollback rows

  -- Actions
  oriterm.new_tab(opts?)        -- create new tab, optional {cwd=...}
  oriterm.close_tab(tab_id?)    -- close tab (default: active)
  oriterm.split(direction)      -- "horizontal" or "vertical"
  oriterm.close_pane()          -- close active pane
  oriterm.focus_pane(direction) -- "up", "down", "left", "right"
  oriterm.set_theme(name)       -- switch color scheme
  oriterm.reload_config()       -- reload config file
  oriterm.send_text(text)       -- write text to active PTY
  oriterm.copy()                -- copy selection to clipboard
  oriterm.paste()               -- paste from clipboard
  oriterm.zoom_pane()           -- toggle pane zoom
  oriterm.scroll_to(position)   -- scroll to absolute position

  -- Appearance
  oriterm.set_tab_title(title)  -- override tab title
  oriterm.set_badge(text)       -- set tab badge (small text overlay)

  -- Events
  oriterm.on("tab_created", function(tab_id) ... end)
  oriterm.on("tab_closed", function(tab_id) ... end)
  oriterm.on("pane_focused", function(pane_id) ... end)
  oriterm.on("output", function(text) ... end)  -- PTY output
  oriterm.on("key", function(key, mods) ... end)
  oriterm.on("resize", function(cols, rows) ... end)
  oriterm.on("cwd_changed", function(path) ... end)
  ```
- [ ] Script loading:
  - [ ] Config: `scripting.init = "~/.config/ori_term/init.lua"` (single init script)
  - [ ] Auto-load: `config_dir/scripts/*.lua` (all scripts in directory)
  - [ ] Hot-reload on file change (via config monitor)
  - [ ] Errors logged to debug log, shown in terminal inspector (Section 26.5)
- [ ] Execution model:
  - [ ] Scripts run on the main thread (event loop) -- no async complexity
  - [ ] Event callbacks invoked synchronously after the event is processed
  - [ ] Script execution has a timeout (100ms default) to prevent hangs
  - [ ] No access to filesystem or network from Lua (sandboxed)
- [ ] Use cases enabled:
  - [ ] Auto-rename tabs based on running command or CWD
  - [ ] Custom status bar with git branch, time, hostname
  - [ ] Complex keybindings with conditional logic
  - [ ] Auto-split layouts on startup (workspace presets)
  - [ ] Session save/restore (named workspaces)

**Tests:**
- [ ] ScriptEngine initializes Lua VM successfully
- [ ] `oriterm.active_tab()` returns correct TabId from Lua
- [ ] `oriterm.new_tab()` creates a tab from Lua script
- [ ] `oriterm.on("tab_created", ...)` callback fires on tab creation
- [ ] Script timeout kills long-running Lua code after 100ms
- [ ] Sandboxed Lua cannot access `os.execute` or `io.open`
- [ ] Hot-reload re-executes init script on file change
- [ ] Malformed Lua script logs error without crashing

---

## 28.2 Custom Shaders

Post-processing WGSL fragment shaders for visual effects.

**File:** `oriterm/src/gpu/pipeline.rs` (shader compilation), `oriterm/src/gpu/renderer.rs` (off-screen render target)

**Reference:** `_old/src/gpu/pipeline.rs` (current WGSL pipeline), `_old/src/gpu/renderer.rs` (render passes)

**Current pipeline:** Two render passes (background + foreground) output to the wgpu surface directly. Custom shaders add a third pass.

- [ ] Config: `window.custom_shader = "/path/to/shader.wgsl"`
- [ ] Pipeline changes:
  1. Render terminal to an off-screen texture (render target) instead of directly to surface
  2. Run custom shader as a full-screen quad with the terminal texture as input
  3. Output to the surface
- [ ] Shader interface (WGSL):
  ```wgsl
  @group(0) @binding(0) var terminal_texture: texture_2d<f32>;
  @group(0) @binding(1) var terminal_sampler: sampler;

  struct Uniforms {
      resolution: vec2<f32>,    // window size in pixels
      time: f32,                // elapsed seconds (for animations)
      cursor_pos: vec2<f32>,    // cursor position in pixels
  }
  @group(0) @binding(2) var<uniform> uniforms: Uniforms;

  @fragment
  fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
      // Default: passthrough
      return textureSample(terminal_texture, terminal_sampler, uv);
  }
  ```
- [ ] Shader uniforms updated each frame:
  - [ ] `time`: `Instant::elapsed().as_secs_f32()` (modulo to prevent precision loss)
  - [ ] `resolution`: window physical size
  - [ ] `cursor_pos`: cursor cell position in pixels
- [ ] Built-in example shaders:
  - [ ] `crt.wgsl` -- CRT effect (scanlines, curvature, vignette, bloom)
  - [ ] `grayscale.wgsl` -- desaturate terminal output
  - [ ] `invert.wgsl` -- invert colors
- [ ] Hot-reload: detect shader file changes, recompile pipeline
  - [ ] On compile error: log error, fall back to passthrough (no shader)
  - [ ] Show compilation errors in terminal inspector (Section 26.5)
- [ ] Performance: custom shader adds one texture sample per pixel per frame
  - [ ] For simple shaders this is negligible
  - [ ] Complex shaders (blur, multi-pass) may need frame rate consideration

**Tests:**
- [ ] Passthrough shader produces identical output to no-shader rendering
- [ ] Shader compile error falls back to passthrough gracefully
- [ ] Hot-reload detects shader file change and recompiles
- [ ] Uniforms (time, resolution, cursor_pos) update each frame
- [ ] Off-screen render target matches surface dimensions

---

## 28.3 Smart Paste

Intelligent paste behavior for safety and convenience.

**File:** `oriterm/src/app.rs` (paste handling), `oriterm/src/gpu/render_overlay.rs` (confirmation overlay)

**Reference:** `_old/src/app/input_keyboard.rs` (paste handling), `_old/src/gpu/render_overlay.rs` (overlay rendering)

- [ ] Multi-line paste warning:
  - [ ] If pasted text contains `\n` (newlines), show confirmation overlay
  - [ ] "You are about to paste N lines. Continue?"
  - [ ] Options: "Paste", "Paste as single line", "Cancel"
  - [ ] "Paste as single line": replace `\n` with spaces
  - [ ] Config: `behavior.warn_multiline_paste = true | false` (default: true)
  - [ ] Bypass: if bracketed paste mode is active, always paste (app handles it)
- [ ] Strip leading prompt characters:
  - [ ] Detect when pasted text starts with `$ `, `# `, `> `, `% `
  - [ ] Strip the prompt prefix (user likely copied from a tutorial/README)
  - [ ] Only strip from first line (not every line)
  - [ ] Config: `behavior.strip_paste_prompt = true | false` (default: false)
- [ ] Sanitize pasted text:
  - [ ] Strip ESC (0x1B) characters to prevent escape injection attacks
  - [ ] Only when bracketed paste mode is NOT active (app handles raw paste)
  - [ ] Log warning when ESC characters are stripped
- [ ] Large paste warning:
  - [ ] If paste > configurable threshold (default 1MB), show warning
  - [ ] "You are about to paste X.X MB. This may be slow. Continue?"
  - [ ] Config: `behavior.large_paste_threshold = 1048576` (bytes, 0 = disabled)
- [ ] Confirmation overlay rendering:
  - [ ] Reuse command palette / dropdown overlay infrastructure
  - [ ] Show preview of first 3-5 lines of paste content
  - [ ] Keyboard: Enter = confirm, Escape = cancel

**Tests:**
- [ ] Single-line paste passes through without warning
- [ ] Multi-line paste triggers confirmation overlay
- [ ] "Paste as single line" replaces newlines with spaces
- [ ] Bracketed paste mode bypasses multi-line warning
- [ ] Leading `$ ` stripped from first line when config enabled
- [ ] ESC characters stripped outside bracketed paste mode
- [ ] Large paste (>1MB) triggers size warning
- [ ] Escape cancels paste, Enter confirms

---

## 28.4 Undo Close Tab

Restore accidentally closed tabs.

**File:** `oriterm/src/app.rs` (tab close + reopen), `oriterm/src/gpu/render_overlay.rs` (toast notification)

**Reference:** `_old/src/app/tab_management.rs` (tab close), `_old/src/app/mod.rs` (tab lifecycle)

- [ ] Closed-tab stack:
  - [ ] `closed_tabs: Vec<ClosedTabInfo>` on `App`, max capacity 10
  - [ ] `ClosedTabInfo`:
    ```rust
    struct ClosedTabInfo {
        title: String,
        cwd: Option<String>,
        scrollback_snapshot: Vec<Row>,  // or compressed
        closed_at: Instant,
    }
    ```
  - [ ] When closing a tab: snapshot scrollback + metadata, push to stack
  - [ ] Do NOT store the running process (can't resurrect a PTY)
  - [ ] Do NOT store the grid -- just scrollback for reference
- [ ] Reopen: `Ctrl+Shift+T`:
  - [ ] Pop most recent `ClosedTabInfo` from stack
  - [ ] Create new tab with stored CWD (if available)
  - [ ] Set tab title to stored title
  - [ ] Optionally: prepopulate scrollback with snapshot (read-only history)
  - [ ] New shell starts fresh -- no process restoration
- [ ] UI feedback:
  - [ ] Brief toast/overlay: "Tab closed -- Ctrl+Shift+T to undo" (3 seconds)
  - [ ] Or: show in command palette as "Reopen Closed Tab (N available)"
- [ ] Stack management:
  - [ ] FIFO with max 10 entries
  - [ ] Memory: each entry stores scrollback rows (could be large)
  - [ ] Consider: only store last 1000 lines of scrollback per entry
  - [ ] Or: compress scrollback with zstd before storing
- [ ] Edge cases:
  - [ ] If no CWD stored, open in home directory
  - [ ] If stack is empty, Ctrl+Shift+T does nothing (or shows message)
  - [ ] Closing all tabs + closing window: stack survives if app has other windows

**Tests:**
- [ ] Close tab pushes ClosedTabInfo to stack
- [ ] Ctrl+Shift+T pops stack and creates new tab
- [ ] Reopened tab has correct CWD and title
- [ ] Stack caps at 10 entries, oldest evicted
- [ ] Empty stack: Ctrl+Shift+T is a no-op
- [ ] Scrollback snapshot stored and retrievable
- [ ] Toast notification appears on tab close

---

## 28.5 Session Recording + Playback

Record terminal sessions for replay, debugging, and demos.

**File:** `oriterm/src/recording/mod.rs`

**Reference:** WezTerm `wezterm record`, asciinema format

- [ ] Recording format:
  - [ ] Use asciicast v2 format (JSON lines) for ecosystem compatibility
  - [ ] Header: `{"version": 2, "width": 80, "height": 24, "timestamp": ..., "env": {...}}`
  - [ ] Events: `[time_offset, "o", "data"]` — time (float seconds), type, payload
  - [ ] Input events: `[time_offset, "i", "data"]` — optional, for recording typed input
- [ ] `oriterm record` subcommand:
  - [ ] `oriterm record -o session.cast` — record to file
  - [ ] `oriterm record` — record to default path (`~/.local/share/oriterm/recordings/`)
  - [ ] Auto-name: `recording-YYYY-MM-DD-HHMMSS.cast`
  - [ ] Recording indicator: subtle "REC" badge in tab bar (red dot)
- [ ] Recording engine:
  - [ ] Tee PTY output: duplicate all bytes from PTY reader to recording file
  - [ ] Timestamp each chunk relative to session start
  - [ ] Optional: record input events (keystrokes sent to PTY)
  - [ ] Flush periodically (every 1s) to prevent data loss on crash
- [ ] `oriterm play` subcommand:
  - [ ] `oriterm play session.cast` — replay recording in a new terminal window
  - [ ] Playback at original speed (honor timestamps)
  - [ ] `--speed <factor>` — 2x, 0.5x playback speed
  - [ ] Pause/resume with spacebar
  - [ ] Seek with arrow keys (skip forward/backward 5s)
- [ ] Integration with running sessions:
  - [ ] `Ctrl+Shift+R` — toggle recording of current pane
  - [ ] Action: `ToggleRecording` in keybinding system
  - [ ] Recording state stored per pane
- [ ] **Tests:**
  - [ ] Recording produces valid asciicast v2 JSON
  - [ ] Timestamps are monotonically increasing
  - [ ] Playback reproduces original output
  - [ ] Speed factor scales timing correctly

---

## 28.6 Workspaces

Named groups of tabs/panes with layout persistence and quick switching.

**File:** `oriterm/src/app/workspaces.rs`, `oriterm_mux/src/registry.rs` (workspace registry)

**Reference:** WezTerm workspaces (`SwitchToWorkspace`, `SwitchWorkspaceRelative`)

- [ ] `Workspace` concept:
  - [ ] Named collection of tabs within a window
  - [ ] Each workspace has its own tab list and active tab
  - [ ] Switching workspaces swaps the visible tab set
  - [ ] Think of it like virtual desktops, but for terminal tabs
- [ ] `WorkspaceId(u64)` newtype
- [ ] `Workspace` struct:
  - [ ] `id: WorkspaceId`
  - [ ] `name: String` — user-visible name (e.g., "default", "project-x", "devops")
  - [ ] `tabs: Vec<TabId>` — tab order within this workspace
  - [ ] `active_tab: usize` — index into `tabs`
- [ ] Workspace management:
  - [ ] Default workspace: "default" — all tabs start here
  - [ ] `SwitchToWorkspace(name)` action — switch to named workspace (create if needed)
  - [ ] `SwitchWorkspaceRelative(offset)` action — cycle through workspaces
  - [ ] `RenameWorkspace(name)` action — rename current workspace
  - [ ] Moving tabs between workspaces
- [ ] Workspace presets via config/Lua:
  ```toml
  [[workspace]]
  name = "dev"
  tabs = [
    { title = "editor", cwd = "~/projects/myapp" },
    { title = "server", cwd = "~/projects/myapp", command = "cargo run" },
    { title = "tests", cwd = "~/projects/myapp" },
  ]
  ```
  - [ ] Lua: `oriterm.create_workspace({ name = "dev", tabs = {...} })`
- [ ] Workspace persistence:
  - [ ] Save workspace layouts as part of session persistence (Section 35)
  - [ ] Restore workspace names and tab assignments on session restore
- [ ] Keybindings:
  - [ ] `Ctrl+Shift+W` — workspace switcher (shows list)
  - [ ] `Ctrl+Shift+N` — new workspace (prompts for name)
- [ ] **Tests:**
  - [ ] Create workspace with name
  - [ ] Switch workspace swaps visible tabs
  - [ ] Tab moves between workspaces
  - [ ] Default workspace always exists
  - [ ] Workspace preset creates tabs from config

---

## 28.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 28.7 Section Completion

- [ ] All 28.1-28.6 items complete
- [ ] Lua scripting engine loads and executes user scripts
- [ ] Scripts can react to events (tab_created, output, key, etc.)
- [ ] Scripts can invoke actions (new_tab, split, set_theme, etc.)
- [ ] Custom WGSL shaders render as post-processing pass
- [ ] Shader hot-reload works (edit file, see change)
- [ ] Multi-line paste shows confirmation dialog
- [ ] Paste stripping works for prompt characters
- [ ] Large paste warning appears for >1MB pastes
- [ ] Ctrl+Shift+T reopens last closed tab in stored CWD
- [ ] Undo-close stack holds last 10 tabs
- [ ] All features documented and configurable

- [ ] `/tpr-review` passed — independent Codex review found no critical or major issues (or all findings triaged)

**Exit Criteria:** ori_term has a Lua scripting layer that enables user-created extensions, custom visual effects via WGSL shaders, and quality-of-life paste safety features.
