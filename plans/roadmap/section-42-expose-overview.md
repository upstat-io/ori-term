---
section: 42
title: "Expose / Overview Mode"
status: not-started
reviewed: false
third_party_review:
  status: none
  updated: null
tier: 5
goal: "macOS Mission Control-style Expose mode: a live GPU-rendered thumbnail grid of ALL panes across ALL windows, tabs, and panes. Full-frame modal state with keyboard/mouse navigation, type-to-filter, and instant pane switching."
sections:
  - id: "42.1"
    title: Expose State Machine
    status: not-started
  - id: "42.2"
    title: Thumbnail Rendering Pipeline
    status: not-started
  - id: "42.3"
    title: Grid Layout Engine
    status: not-started
  - id: "42.4"
    title: "Navigation & Selection"
    status: not-started
  - id: "42.5"
    title: "Filter & Search"
    status: not-started
  - id: "42.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "42.6"
    title: Section Completion
    status: not-started
---

# Section 42: Expose / Overview Mode

**Status:** Not Started
**Goal:** A macOS Mission Control-style Expose mode that shows a live GPU-rendered thumbnail grid of ALL panes across ALL windows, tabs, and panes. Users press one keybind to see everything, navigate with keyboard or mouse, type to filter by title/CWD, and confirm to jump directly to any pane. No existing terminal emulator does live thumbnails — tmux/WezTerm/Kitty all use text lists or label overlays. This is a differentiating feature.

**Crate:** `oriterm` (state machine, rendering, input dispatch)
**Dependencies:** Section 31 (In-Process Mux — pane enumeration, `prepare_pane_into()`), Section 32 (Tab & Window Management — cross-window pane access), Section 07 (2D UI Framework — label rendering, layout), Section 05 (GPU pipeline — offscreen `RenderTarget`, texture sampling)
**Prerequisite:** Sections 31, 32, 07, 05 complete.

**Reference:**
- macOS Mission Control / Expose: full-screen thumbnail grid of all windows with live updates
- tmux `choose-tree` / `choose-window`: text-based tree list (no thumbnails)
- WezTerm tab navigator: text list overlay (no thumbnails)
- Kitty `goto_layout`: layout switching (no global overview)
- Ghostty: no equivalent

**Why this matters:** With multi-window, multi-tab, multi-pane multiplexing, users lose track of where things are. Expose gives instant spatial awareness — see all terminals at once, find the one running `htop` or showing a compile error, and jump to it in one action. This is the feature that makes multiplexing feel natural rather than overwhelming.

**Design decisions:**
- **Not an overlay widget** — full-frame modal state (`App.expose: Option<ExposeMode>`), like search mode. The entire viewport is replaced by the thumbnail grid.
- **Staggered updates** — render all thumbnails on entry (burst), then round-robin N/8 per frame for live updates. The selected pane always updates every frame.
- **Texture pool** — fixed 320×200 offscreen `RenderTarget`s, reused across expose sessions. LRU eviction on pane close.
- **Image pipeline** — new wgpu pipeline with per-thumbnail bind groups (textured quad shader), not texture arrays. Simpler, debuggable, no array texture limits.
- **V1 no animation** — `Entering`/`Exiting` phases exist in the state enum for future use, but V1 transitions are instant.

---

## 42.1 Expose State Machine

The core state machine that manages entering, navigating, and exiting Expose mode.

**File:** `oriterm/src/app/expose.rs`

- [ ] `ExposePhase` enum:
  - [ ] `Entering` — reserved for future animation (V1: instant transition)
  - [ ] `Active` — thumbnail grid visible, accepting input
  - [ ] `Exiting` — reserved for future animation (V1: instant transition)
- [ ] `ExposeTile` struct:
  - [ ] `pane_id: PaneId` — which pane this tile represents
  - [ ] `tab_id: TabId` — owning tab
  - [ ] `window_id: WindowId` — owning window
  - [ ] `title: String` — pane title (shell process name or explicit title)
  - [ ] `cwd: Option<String>` — current working directory if known
  - [ ] `rect: Rect` — computed screen rect for this thumbnail (set by layout)
  - [ ] `label_rect: Rect` — rect for the title label below the thumbnail
  - [ ] `label_char: Option<char>` — a-z hint character for O(1) selection (42.4)
  - [ ] `is_visible: bool` — false when filtered out (42.5)
- [ ] `ExposeMode` struct:
  - [ ] `phase: ExposePhase`
  - [ ] `tiles: Vec<ExposeTile>` — one per pane, ordered by (window, tab, tree position)
  - [ ] `selected: usize` — index into `tiles` of currently highlighted tile
  - [ ] `filter: String` — current filter text (42.5)
  - [ ] `previous_pane: PaneId` — pane that was focused before entering expose
- [ ] Enter expose:
  - [ ] Keybind: `Ctrl+Shift+Space` (configurable)
  - [ ] Enumerate all panes from `InProcessMux` → build `tiles` vec
  - [ ] Set `selected` to the tile matching the currently focused pane
  - [ ] Record `previous_pane` for cancel-restore
  - [ ] Request thumbnail render for all panes (burst render — 42.2)
  - [ ] Set `App.expose = Some(ExposeMode { ... })`
- [ ] Exit expose (confirm):
  - [ ] Switch focus to `tiles[selected].pane_id`
  - [ ] If pane is in a different tab: switch to that tab
  - [ ] If pane is in a different window: raise that window, switch tab, focus pane
  - [ ] Set `App.expose = None`
- [ ] Exit expose (cancel):
  - [ ] Restore focus to `previous_pane`
  - [ ] Set `App.expose = None`
- [ ] Input interception:
  - [ ] When `App.expose.is_some()`, ALL keyboard and mouse input routes to expose handler
  - [ ] No input reaches the terminal PTY while in expose mode
  - [ ] Escape = cancel, Enter = confirm, arrow keys = navigate (42.4)
  - [ ] Printable characters = filter (42.5)
- [ ] **Tests:**
  - [ ] Enter expose builds tiles from all panes
  - [ ] Confirm switches to selected pane
  - [ ] Cancel restores previous pane
  - [ ] Input interception blocks PTY input during expose

---

## 42.2 Thumbnail Rendering Pipeline

Offscreen rendering of pane content into thumbnail-sized textures, plus a new GPU pipeline for compositing thumbnails into the expose grid.

**File:** `oriterm/src/gpu/renderer/thumbnails.rs`, `oriterm/src/gpu/pipeline/image_pipeline.rs`

- [ ] `ThumbnailCache` struct:
  - [ ] Pool of offscreen `RenderTarget`s at fixed 320×200 resolution
  - [ ] `targets: HashMap<PaneId, RenderTarget>` — one per visible pane
  - [ ] `last_updated: HashMap<PaneId, Instant>` — track freshness
  - [ ] Pool size grows on demand, shrinks on pane close (LRU eviction)
  - [ ] Targets created via existing `GpuState::create_render_target(320, 200)`
- [ ] Thumbnail rendering:
  - [ ] Reuse existing 3-phase pipeline: Extract → Prepare → Render
  - [ ] `extract_frame()` with the pane's `Term` lock (same as normal rendering)
  - [ ] `prepare_frame()` with thumbnail dimensions (320×200 cells scaled)
  - [ ] `render_to_target()` into the offscreen `RenderTarget` (not the surface)
  - [ ] Thumbnail cell size: derive from `320px / pane_cols` — cells are tiny but proportional
- [ ] Staggered update strategy:
  - [ ] On expose entry: burst-render ALL pane thumbnails (synchronous, all in one frame)
  - [ ] After entry: round-robin update max `ceil(pane_count / 8)` thumbnails per frame
  - [ ] Exception: the selected pane's thumbnail ALWAYS updates every frame
  - [ ] Dirty tracking: skip thumbnail re-render if pane's `grid_dirty` is false
- [ ] `RenderTarget` as texture source:
  - [ ] Each `RenderTarget` already has a `wgpu::Texture` (from Section 05)
  - [ ] Create a `wgpu::TextureView` and `wgpu::BindGroup` per thumbnail target
  - [ ] Bind group layout: `TEXTURE_BINDING` + `SAMPLER` (linear filtering for minification)
- [ ] `ImagePipeline` (new):
  - [ ] WGSL vertex shader: textured quad (4 vertices, 2 triangles)
  - [ ] Vertex attributes: position (clip space), UV coordinates
  - [ ] WGSL fragment shader: sample from bound texture, output color
  - [ ] Uniform buffer: `screen_size` for NDC conversion (shared with existing pipelines)
  - [ ] Per-thumbnail instance data: `rect` (x, y, w, h in pixels), `opacity` (f32)
  - [ ] Render pass: clear to background color, draw N quads (one per visible tile)
- [ ] Selection highlight:
  - [ ] Selected thumbnail: 2px border in accent color (theme-aware)
  - [ ] Hovered thumbnail: 1px border in dimmed accent
  - [ ] Non-selected thumbnails: subtle shadow or dimming
- [ ] **Tests:**
  - [ ] `ThumbnailCache` creates and pools `RenderTarget`s
  - [ ] Thumbnail render produces non-empty texture (pixel readback)
  - [ ] Staggered update renders correct subset per frame
  - [ ] `ImagePipeline` compiles WGSL shader without errors
  - [ ] Selected pane always updates every frame

---

## 42.3 Grid Layout Engine

Pure function that computes thumbnail positions given pane count and viewport size.

**File:** `oriterm/src/app/expose/layout.rs`

- [ ] `ExposeLayout` struct (output):
  - [ ] `tile_rects: Vec<Rect>` — one rect per tile, in pixel coordinates
  - [ ] `label_rects: Vec<Rect>` — one rect per tile for the title label
  - [ ] `columns: usize` — computed column count
  - [ ] `rows: usize` — computed row count
  - [ ] `thumbnail_size: (u32, u32)` — actual thumbnail pixel size (may differ from 320×200)
- [ ] `compute_expose_grid(viewport: Size, pane_count: usize, filter_bar_height: u32) -> ExposeLayout`:
  - [ ] Pure function — no side effects, no GPU state, fully deterministic
  - [ ] **Auto-column calculation:**
    - [ ] `columns = ceil(sqrt(pane_count))` as starting point
    - [ ] Adjust to maximize thumbnail size within viewport
    - [ ] `rows = ceil(pane_count / columns)`
  - [ ] **Thumbnail size:**
    - [ ] Available width: `viewport.width - padding * 2 - gap * (columns - 1)`
    - [ ] Available height: `viewport.height - filter_bar_height - padding * 2 - gap * (rows - 1) - label_height * rows`
    - [ ] `thumb_w = available_width / columns`
    - [ ] `thumb_h = available_height / rows`
    - [ ] Maintain 16:10 aspect ratio (match terminal proportions): clamp to fit
    - [ ] Minimum: 160×100 pixels (below this, thumbnails are unreadable)
    - [ ] Maximum: 640×400 pixels (above this, waste of space)
  - [ ] **Gap and padding:**
    - [ ] `gap`: 12px between thumbnails
    - [ ] `padding`: 24px around the grid edges
    - [ ] `label_height`: 20px below each thumbnail for title text
  - [ ] **Last-row centering:**
    - [ ] If the last row has fewer tiles than `columns`, center them horizontally
    - [ ] Example: 5 panes in 3 columns → row 1: [1, 2, 3], row 2: [  4, 5  ] (centered)
  - [ ] **Responsive:**
    - [ ] Recalculate on window resize
    - [ ] Fewer panes → larger thumbnails (1 pane fills most of viewport)
    - [ ] Many panes → smaller thumbnails (cap at minimum size, enable scrolling if needed)
- [ ] Label rendering:
  - [ ] Title text truncated with `…` to fit `label_rect` width
  - [ ] Format: `"[hint] title — cwd"` or `"[hint] title"` if no CWD
  - [ ] Hint character rendered in accent color, title in primary text color
  - [ ] Use existing UI text rendering from Section 07
- [ ] **Tests:**
  - [ ] 1 pane: single large thumbnail centered
  - [ ] 4 panes: 2×2 grid
  - [ ] 9 panes: 3×3 grid
  - [ ] 5 panes: 3×2 grid with last row centered (2 tiles centered)
  - [ ] Minimum size clamping prevents thumbnails below 160×100
  - [ ] Maximum size clamping prevents thumbnails above 640×400
  - [ ] Viewport resize recalculates layout
  - [ ] Label rects positioned below each thumbnail

---

## 42.4 Navigation & Selection

Keyboard and mouse input for navigating the thumbnail grid and selecting a pane.

**File:** `oriterm/src/app/expose/input.rs`

- [ ] Arrow key navigation:
  - [ ] `Left` / `Right`: move selection within the row
  - [ ] `Up` / `Down`: move selection between rows
  - [ ] Grid-aware wrapping: `Right` at end of row wraps to start of next row
  - [ ] `Up` from first row wraps to last row (same column or nearest)
  - [ ] `Down` from last row wraps to first row
  - [ ] Skip filtered-out tiles (jump to next visible tile in direction)
- [ ] Confirm/cancel:
  - [ ] `Enter`: confirm selection — switch to selected pane, exit expose
  - [ ] `Escape`: cancel — restore previous pane, exit expose (unless filter active — see 42.5)
- [ ] Tab/Shift+Tab cycling:
  - [ ] `Tab`: move selection to next visible tile (linear order)
  - [ ] `Shift+Tab`: move selection to previous visible tile
  - [ ] Wraps around at boundaries
- [ ] Character label hints:
  - [ ] Assign `a`-`z` labels to visible tiles (first 26 tiles)
  - [ ] When filter is empty: typing a single letter that matches a hint char = instant select + confirm
  - [ ] When filter is active: letters go to filter (42.5), hints disabled
  - [ ] Labels displayed on thumbnails (bottom-left corner, accent color badge)
- [ ] Mouse interaction:
  - [ ] Click on thumbnail: select + confirm (immediate switch)
  - [ ] Hover over thumbnail: highlight (1px accent border), update `selected`
  - [ ] Click on empty space: no action
  - [ ] Right-click: reserved for future context menu
- [ ] **Tests:**
  - [ ] Arrow keys navigate grid correctly (3×3 grid)
  - [ ] Arrow wrapping at boundaries
  - [ ] Enter confirms and exits expose
  - [ ] Escape cancels and restores previous pane
  - [ ] Tab cycles through visible tiles linearly
  - [ ] Mouse click selects and confirms
  - [ ] Hint character 'a' selects first tile when filter is empty
  - [ ] Filtered-out tiles are skipped during navigation

---

## 42.5 Filter & Search

Type-to-filter for narrowing the thumbnail grid by pane title or CWD.

**File:** `oriterm/src/app/expose/filter.rs`

- [ ] Filter behavior:
  - [ ] Any printable character (when filter is active or when no hint label matches): append to `filter` string
  - [ ] Filter activation: first typed character that doesn't match a hint label activates filter mode
  - [ ] Case-insensitive substring match against `tile.title` and `tile.cwd`
  - [ ] Tiles that don't match: set `is_visible = false`, excluded from layout and navigation
  - [ ] Layout recomputes on every filter keystroke (only visible tiles participate)
  - [ ] Selection moves to nearest visible tile if current selection becomes hidden
- [ ] Filter bar:
  - [ ] Rendered at the top of the expose viewport
  - [ ] Shows: `"Filter: {filter_text}"` with blinking cursor
  - [ ] Height: 32px (subtracted from available grid space)
  - [ ] Only visible when `filter` is non-empty
  - [ ] Background: semi-transparent dark overlay matching theme
- [ ] Backspace:
  - [ ] Removes last character from `filter`
  - [ ] If filter becomes empty: return to unfiltered view, re-show all tiles
  - [ ] Layout recomputes
- [ ] Double-Escape:
  - [ ] First `Escape` when filter is active: clear filter (show all tiles again)
  - [ ] Second `Escape` (filter already empty): exit expose (cancel)
  - [ ] This prevents accidental exit when user just wants to clear their filter
- [ ] "No matches" empty state:
  - [ ] When filter matches zero tiles: show centered message `"No matching panes"`
  - [ ] Backspace still works to narrow/clear filter
  - [ ] Enter does nothing (no tile to confirm)
- [ ] **Tests:**
  - [ ] Typing "vim" filters to tiles with "vim" in title
  - [ ] Case-insensitive: "VIM" matches "vim"
  - [ ] CWD matching: typing "/home" matches tiles with that CWD
  - [ ] Backspace narrows filter
  - [ ] Empty filter shows all tiles
  - [ ] First Escape clears filter, second Escape exits expose
  - [ ] No-match state shows empty message
  - [ ] Selection adjusts when current tile becomes filtered out

---

## 42.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 42.6 Section Completion

- [ ] All 42.1–42.5 items complete
- [ ] `Ctrl+Shift+Space` enters expose mode from any state
- [ ] Thumbnail grid shows ALL panes across all windows/tabs
- [ ] Thumbnails are live GPU-rendered (staggered updates, selected always fresh)
- [ ] Grid layout is responsive (auto-columns, last-row centering, min/max clamping)
- [ ] Arrow keys, Tab, mouse click, and hint characters all navigate correctly
- [ ] Type-to-filter narrows thumbnails by title/CWD (case-insensitive substring)
- [ ] Double-Escape: clear filter first, then exit
- [ ] Enter confirms, single Escape (no filter) cancels
- [ ] Cross-window pane switching works (raise window, switch tab, focus pane)
- [ ] `cargo clippy --target x86_64-pc-windows-gnu` — no warnings
- [ ] `cargo test` — all expose tests pass

**Performance targets:**
- Entry latency: < 100ms (burst-render all thumbnails)
- Per-frame render: < 4ms (draw N textured quads + labels)
- Filter keystroke to layout update: < 2ms

- [ ] `/tpr-review` passed — independent Codex review found no critical or major issues (or all findings triaged)

**Exit Criteria:** Users press `Ctrl+Shift+Space`, see a live thumbnail grid of every pane in every window and tab, navigate with arrows/mouse/hints, type to filter by title or CWD, and press Enter to jump directly to any pane. The feature provides instant spatial awareness across the entire multiplexed session — something no other terminal emulator offers.
