---
section: 16
title: Tab Bar & Chrome
status: in-progress
reviewed: false
third_party_review:
  status: findings
  updated: 2026-03-26
tier: 4
goal: Tab bar layout, rendering, and hit testing with DPI awareness
sections:
  - id: "16.1"
    title: Tab Bar Layout + Constants
    status: complete
  - id: "16.2"
    title: Tab Bar Rendering
    status: complete
  - id: "16.3"
    title: Tab Bar Hit Testing
    status: in-progress
  - id: "16.5"
    title: Tab Icons & Emoji
    status: complete
  - id: "16.6"
    title: "Tab Title Inline Editing"
    status: complete
  - id: "16.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "16.4"
    title: Section Completion
    status: in-progress
---

# Section 16: Tab Bar & Chrome
**Status:** In Progress (16.1-16.2 complete, 16.3+ in progress)
**Goal:** Tab bar layout, rendering, and hit testing with DPI awareness. Deterministic layout computation, GPU-rendered tab bar with bell pulse animation and drag overlay, and priority-based hit testing for click/hover dispatch.

**Crate:** `oriterm` (binary only — no core changes)
**Dependencies:** `wgpu`, `winit`
**Reference:** `_old/src/tab_bar.rs`, `_old/src/gpu/render_tab_bar.rs`, `_old/src/gpu/render_overlay.rs`

**Prerequisite:** Section 13 complete (Tab struct and management operations available).

---

## 16.1 Tab Bar Layout + Constants

Compute the pixel layout of tabs in the tab bar. All measurements are DPI-scaled. The layout is deterministic — given tab count, window width, and scale factor, the output is identical.

**File:** `oriterm_ui/src/widgets/tab_bar/` (constants, layout, colors modules)

**Reference:** `_old/src/tab_bar.rs`

**Deviation:** Layout computes in logical pixels (matching `ChromeLayout` pattern); scale applied at render boundary. Colors use `oriterm_ui::color::Color` (not `[f32; 4]`) and derive from `UiTheme` (not `Palette`), matching existing widget conventions. `window_width: f32` stored instead of `scale: f64` since scale is not needed for logical-pixel layout.

- [x] Layout constants (all in logical pixels, multiply by `scale_factor` for physical):
  - [x] `TAB_BAR_HEIGHT: f32 = 46.0` — full height of the tab bar
  - [x] `TAB_MIN_WIDTH: f32 = 80.0` — minimum tab width before they start overlapping
  - [x] `TAB_MAX_WIDTH: f32 = 260.0` — maximum tab width (tabs grow to fill available space, clamped here)
  - [x] `TAB_LEFT_MARGIN: f32 = 16.0` — padding before the first tab
  - [x] `TAB_PADDING: f32 = 8.0` — internal horizontal padding within each tab
  - [x] `CLOSE_BUTTON_WIDTH: f32 = 24.0` — clickable area for the x button
  - [x] `CLOSE_BUTTON_RIGHT_PAD: f32 = 8.0` — spacing between x button and tab's right edge
  - [x] `NEW_TAB_BUTTON_WIDTH: f32 = 38.0` — width of the "+" button
  - [x] `DROPDOWN_BUTTON_WIDTH: f32 = 30.0` — width of the dropdown (settings/scheme) button
  - [x] `CONTROLS_ZONE_WIDTH` — platform-specific:
    - [x] Windows: `174.0` (three 58px buttons: minimize, maximize, close)
    - [x] Linux/macOS: `100.0` (three circular buttons with spacing)
  - [x] `DRAG_START_THRESHOLD: f32 = 10.0` — pixels of movement before drag begins (matches Chrome's `tab_drag_controller.cc`)
  - [x] `TEAR_OFF_THRESHOLD: f32 = 40.0` — pixels outside tab bar before tear-off
  - [x] `TEAR_OFF_THRESHOLD_UP: f32 = 15.0` — reduced threshold for upward dragging (more natural for tear-off)
- [x] `TabBarLayout` struct:
  - [x] `tab_width: f32` — computed width per tab (all tabs same width)
  - [x] `tab_count: usize` — number of tabs
  - [x] `window_width: f32` — window width used for layout (replaces `scale: f64` since layout is in logical pixels)
- [x] `TabBarLayout::compute(tab_count: usize, window_width: f32, tab_width_lock: Option<f32>) -> Self`
  - [x] If `tab_width_lock` is `Some(w)`: use locked width (prevents jitter during rapid close clicks or drag)
  - [x] Available width = `window_width - TAB_LEFT_MARGIN - NEW_TAB_BUTTON_WIDTH - DROPDOWN_BUTTON_WIDTH - CONTROLS_ZONE_WIDTH`
  - [x] `tab_width = (available / tab_count).clamp(TAB_MIN_WIDTH, TAB_MAX_WIDTH)`
  - [x] Return layout struct
- [x] `tab_width_lock: Option<f32>` on App:
  - [x] **Acquired** when: cursor enters tab bar (hovering), prevents tabs from expanding when quickly closing tabs
  - [x] **Released** when: cursor leaves tab bar, window resizes, tab count changes in ways that invalidate the lock (new tab, drag reorder)
  - [x] Purpose: If you have 5 tabs and close one, the remaining 4 tabs would normally expand. But if you're rapidly clicking close buttons, the expansion moves the next close button, causing you to miss. The lock freezes tab width during hover, so close buttons don't move.
- [x] `TabBarColors` struct — all colors needed for tab bar rendering:
  - [x] `bar_bg: Color` — tab bar background
  - [x] `active_bg: Color` — active tab background (rendered with rounded corners)
  - [x] `inactive_bg: Color` — inactive tab background
  - [x] `tab_hover_bg: Color` — inactive tab background on hover
  - [x] `text_fg: Color` — active tab title text
  - [x] `inactive_text: Color` — inactive tab title text (dimmer)
  - [x] `separator: Color` — 1px vertical separator between tabs
  - [x] `close_fg: Color` — close button color (unhovered)
  - [x] `button_hover_bg: Color` — "+" and dropdown hover background
  - [x] `control_hover_bg: Color` — window control button hover
  - [x] `control_fg: Color` — window control icon color
  - [x] `control_fg_dim: Color` — dimmed window control icon
  - [x] `control_close_hover_bg: Color` — close button red hover (platform standard)
  - [x] `control_close_hover_fg: Color` — close button text on red hover (white)
  - [x] Derived from theme: `TabBarColors::from_theme(theme: &UiTheme) -> Self`

---

## 16.2 Tab Bar Rendering

Render the tab bar as GPU instances. The tab bar is rendered in the overlay pass, after the terminal grid bg+fg passes. The dragged tab is rendered separately in a second overlay pass so it floats above everything.

**File:** `oriterm_ui/src/widgets/tab_bar/widget/` (TabBarWidget + draw), `oriterm/src/app/redraw.rs` (pipeline integration)

**Reference:** `_old/src/gpu/render_tab_bar.rs`, `_old/src/gpu/render_overlay.rs`

**Deviation:** Implemented as `TabBarWidget` (Widget→DrawList→GPU pipeline) rather than `build_tab_bar_instances()` with `InstanceWriter`. The architecture evolved after the plan was written — widgets draw to a `DrawList` in logical pixels, which is converted to GPU instances at the rendering boundary. `drag_visual_x` simplified from `Option<(WindowId, f32)>` to `Option<(usize, f32)>` on the widget (single window). Animation offsets simplified from `HashMap<WindowId, Vec<f32>>` to `Vec<f32>` on the widget. Window control buttons (item 7) already handled by `WindowChromeWidget`.

- [x] `build_tab_bar_instances()` — primary rendering function
  - [x] Input: `InstanceWriter` (bg + fg), `FrameParams`, `TabBarColors`, `FontCollection`, `wgpu::Queue`
  - [x] Output: populated instance buffers ready for GPU submission
- [x] Rendering order (draw order matters for layering):
  1. [x] Tab bar background: full-width rectangle across top of window
  2. [x] Inactive tabs (drawn first, behind active tab):
     - [x] Background rectangle (with hover color if `hover_hit == Tab(idx)`)
     - [x] Title text: shaped with UI font collection, truncated with ellipsis if too wide
     - [x] Close button: vector x icon (visible on hover only, or always — configurable)
  3. [x] Active tab (drawn on top of inactive tabs):
     - [x] Background rectangle with **rounded top corners** (radius ~8px x scale)
     - [x] Title text: brighter color than inactive
     - [x] Close button: always visible
  4. [x] Separators: 1px vertical lines between tabs, with **suppression rules**:
     - [x] No separator adjacent to active tab (left or right edge)
     - [x] No separator adjacent to hovered tab
     - [x] No separator adjacent to dragged tab
  5. [x] New tab "+" button: after the last tab
  6. [x] Dropdown button: after "+" button
  7. [x] Window control buttons: rightmost (see section 16)
- [x] Bell badge animation:
  - [x] `bell_phase: f32` (0.0–1.0) — sine wave pulse
  - [x] Inactive tab with bell: `lerp_color(inactive_bg, tab_hover_bg, bell_phase)` — smooth pulsing background
  - [x] Phase computed from `bell_start: Option<Instant>` on the tab's terminal state
  - [x] Clear badge when tab becomes active
- [x] Dragged tab overlay:
  - [x] When dragging: the dragged tab is **not rendered in the normal tab bar pass**
  - [x] Instead, rendered in a separate overlay pass via `build_dragged_tab_overlay()`
  - [x] Rendering:
    1. Opaque backing rect (hides underlying text from fg pass)
    2. Rounded tab shape with active background
    3. Tab content (text + close button) at `drag_visual_x` position
  - [x] "+" and dropdown buttons reposition during drag: `max(default_x, drag_x + tab_w)` — keeps buttons visible even when dragging far right
- [x] `drag_visual_x: Option<(WindowId, f32)>` on App:
  - [x] The pixel X position where the dragged tab is drawn
  - [x] Separate from the tab's actual index in the vec — allows smooth visual feedback without real-time list manipulation
  - [x] Updated on every mouse move during drag
- [x] Tab animation offsets:
  - [x] `tab_anim_offsets: HashMap<WindowId, Vec<f32>>` — per-tab pixel offsets for smooth transitions
  - [x] When tabs reorder during drag: displaced tabs get a non-zero offset that decays to 0 over ~100ms
  - [x] `decay_tab_animations(&mut self) -> bool` — returns true if any animation is still active (needs continued rendering)
  - [x] Chrome-style behavior: tabs **snap immediately** to new positions during drag. Animation only applies on drag-end.
- [x] Tab title rendering: <!-- unblocks:6.13 -->
  - [x] Use UI font collection (separate from terminal font, possibly different family/weight)
  - [x] `ui_collection.truncate_to_pixel_width(title, max_text_px)` — truncates with `...` (U+2026) if too wide
  - [x] Max text width = `tab_width - 2*TAB_PADDING - CLOSE_BUTTON_WIDTH - CLOSE_BUTTON_RIGHT_PAD`

---

## 16.3 Tab Bar Hit Testing

Map mouse coordinates to tab bar actions. Hit testing determines whether a click or hover targets a tab, a button, or the drag area.

**File:** `oriterm/src/chrome/tab_bar.rs`

**Reference:** `_old/src/tab_bar.rs`

- [x] `TabBarHit` enum:
  - [x] `Tab(usize)` — clicked on tab at index
  - [x] `CloseTab(usize)` — clicked close button on tab at index
  - [x] `NewTab` — clicked the "+" button
  - [x] `DropdownButton` — clicked the dropdown/settings button (named `Dropdown` in code)
  - [x] `Minimize` — clicked window minimize
  - [x] `Maximize` — clicked window maximize/restore
  - [x] `CloseWindow` — clicked window close
  - [x] `DragArea` — clicked empty tab bar area (for window dragging or double-click maximize)
  - [x] `None` — click is below tab bar (terminal area)
- [x] `hit_test(x: f32, y: f32, layout: &TabBarLayout) -> TabBarHit` (logical pixels, no scale param)
  - [x] Priority order (checked first = higher priority):
    1. [x] If `y` outside `0..TAB_BAR_HEIGHT`: return `None` (below/above tab bar)
    2. [x] Check window controls zone (rightmost):
       - [x] **Windows**: three `CONTROL_BUTTON_WIDTH` buttons, left-to-right: Minimize, Maximize, Close
       - [x] **Linux/macOS**: three circular buttons (24px diameter, 8px spacing, 12px margins)
       - [x] Return `CloseWindow`, `Maximize`, or `Minimize`
    3. [x] Check tabs region (starts at `TAB_LEFT_MARGIN`):
       - [x] For each tab: check close button rect **first** (inset from right edge)
       - [x] Then check tab rect — return `Tab(idx)`
    4. [x] Check new-tab button (after last tab)
    5. [x] Check dropdown button (after new-tab button)
    6. [x] If still within tab bar height: return `DragArea`
- [x] Tab bar hover tracking:
  - [x] `hover_hit` on `TabBarWidget` (updated via `set_hover_hit`)
  - [x] Updated on every `CursorMoved` event (via `update_tab_bar_hover`)
  - [x] When hover changes: mark dirty, request redraw
  - [x] Hover entering tab bar: acquire `tab_width_lock`
  - [x] Hover leaving tab bar: release `tab_width_lock`
- [ ] Tab hover preview (Chrome/Windows-style): <!-- blocked-by:7 -->
  - [ ] When hovering an inactive tab for > 300ms, show a `TerminalPreviewWidget` overlay
  - [ ] Preview appears below the tab bar, anchored to the hovered tab
  - [ ] Preview shows a live scaled-down render of that tab's terminal content
  - [ ] Uses offscreen render target (Section 05) + `TerminalPreviewWidget` (Section 07)
  - [ ] Fade-in animation (07.9), dismiss on hover leave
  - [ ] Preview updates if the terminal content changes while hovering
  - [ ] No preview for the active tab (it's already visible)
- [x] Mouse press dispatch (in `try_tab_bar_mouse`):
  - [x] `Tab(idx)`: consumes click (multi-tab switching deferred to Section 15/30) <!-- blocked-by:30 -->
  - [x] `CloseTab(idx)`: acquire `tab_width_lock`, close tab
  - [x] `NewTab`: consumes click (tab creation deferred to Section 15/30) <!-- blocked-by:30 -->
  - [x] `DropdownButton`: consumes click (dropdown menu deferred to Section 21) <!-- blocked-by:21 -->
  - [x] `Minimize`: `window.set_minimized(true)`
  - [x] `Maximize`: toggle `window.set_maximized()`
  - [x] `CloseWindow`: close window
  - [x] `DragArea`:
    - [x] Double-click: toggle maximize (500ms threshold)
    - [x] Single-click: start window drag via `window.drag_window()`

---

## 16.5 Tab Icons & Emoji

Render emoji and icon characters in tab titles. The font pipeline already supports color emoji (Section 6.10 — CBDT/CBLC, COLR/CPAL via swash + Segoe UI Emoji / Noto Color Emoji fallback). This subsection wires that capability into the tab bar so that process icons, user-set emoji, and OSC-set icons display correctly next to tab titles.

**File:** `oriterm_ui/src/widgets/tab_bar/widget/draw.rs` (icon rendering), `oriterm_core/src/tab.rs` (icon state)

**Reference:** Windows Terminal profile `"icon"` setting, iTerm2 per-tab icon, OSC 1 (icon name)

- [x] Per-tab icon state:
  - [x] `icon: Option<TabIcon>` on `TabEntry` (`oriterm_ui/src/widgets/tab_bar/widget/mod.rs`)
  - [x] `TabIcon` enum: `Emoji(String)` (single grapheme cluster)
  - [x] Default: `None` (no icon, title only — current behavior)
  - [x] `with_icon()` builder method on `TabEntry`
- [x] Icon sources (priority order, highest wins):
  1. [x] **OSC 1 (Set Icon Name)**: VTE handler differentiated from OSC 0/2. `set_icon_name()` added to Handler trait. `Term.icon_name` field stores it. `Event::IconName`/`Event::ResetIconName` flow through mux pipeline to `Pane.icon_name`. `extract_emoji_icon()` detects leading emoji grapheme.
  2. [ ] **Profile config**: `[tab] icon = "🐍"` in TOML config per-profile (future, when profiles exist)
  3. [ ] **Process detection** (stretch goal): detect foreground process name — requires shell integration (Section 20)
- [x] Tab bar rendering changes:
  - [x] When `tab.icon` is `Some(Emoji(s))`: shape emoji through UI font, render before title, shift title right by `icon_width + ICON_TEXT_GAP`
  - [x] When `tab.icon` is `None`: current behavior (title only, no shift)
  - [x] Same logic applied in both `draw_tab()` and `draw_dragged_tab_overlay()`
- [x] Color emoji atlas integration:
  - [x] Uses `push_text()` — color emoji automatically routed through color atlas pipeline when the glyph is rasterized (AtlasKind::Color detection in draw list converter)
- [x] Constants:
  - [x] `ICON_TEXT_GAP: f32 = 4.0` — pixels between icon and title text
- [x] **Bonus fix**: Title changes (OSC 0/2) now emit `MuxNotification::PaneTitleChanged` to re-sync tab bar (previously only `set_title()` was called but no notification was emitted)

**Tests:**
- [x] VTE: OSC 0 calls both `set_title` and `set_icon_name` (VTE crate + handler tests)
- [x] VTE: OSC 1 calls only `set_icon_name` (VTE crate + handler tests)
- [x] VTE: OSC 2 calls only `set_title` (VTE crate + handler tests)
- [x] Emoji detection: `extract_emoji_icon("🐍python")` → `Some(Emoji("🐍"))`
- [x] Emoji detection: plain text, empty, flags, ZWJ sequences
- [x] Event pipeline: `IconName`/`ResetIconName` events + `PaneIconChanged` mux event
- [x] `MuxNotification::PaneTitleChanged` debug format

---

## 16.6 Tab Title Inline Editing

Double-click a tab label to rename it inline. The label text becomes an editable text field with cursor, selection, and standard keyboard editing. Enter commits, Escape cancels, click-outside commits.

**File:** `oriterm_ui/src/widgets/tab_bar/widget/mod.rs` (editing state fields, start/commit/cancel methods), `oriterm_ui/src/widgets/tab_bar/widget/draw.rs` (render editor instead of label during editing), `oriterm/src/app/tab_bar_input.rs` (double-click detection on `TabBarHit::Tab`), `oriterm_ui/src/action/mod.rs` (new `TabTitleChanged` action variant)

**Reference:** VS Code tab rename (double-click → inline edit → select all → type replaces), Windows Terminal (right-click → Rename Tab)

**Design:** Reuse `TextEditingState` from `oriterm_ui/src/text/editing/mod.rs` directly on `TabBarWidget`. This avoids nested widget complexity — the tab bar is a monolithic widget that draws directly to the scene, not a container with child widgets. `TextEditingState` provides all editing logic (cursor movement, selection, insert, delete, home/end) and is already used by `TextInputWidget` and sidebar search.

- [x] **Editing state on `TabBarWidget`** (`widget/mod.rs`, 472 lines):
  - [x] Add `editing_index: Option<usize>` field — which tab is being edited (`None` = not editing)
  - [x] Add `editing: TextEditingState` field — the editing buffer (reused, not allocated per edit)
  - [x] Add `original_title: String` field — for Escape cancellation (restore original)
  - [x] Add `pub fn start_editing(&mut self, index: usize)` — sets `editing_index`, copies title, calls `select_all()`
  - [x] Add `pub fn commit_editing(&mut self) -> Option<(usize, String)>` — trims whitespace, restores original if empty
  - [x] Add `pub fn cancel_editing(&mut self)` — restores original title, clears `editing_index`
  - [x] Add `pub fn is_editing(&self) -> bool` — returns `editing_index.is_some()`
  - [x] Add `pub fn editing_tab_index(&self) -> Option<usize>` — returns `editing_index`
  - [x] Add forwarding methods: `editing_insert_char`, `editing_backspace`, `editing_delete`, `editing_move_left/right`, `editing_home/end`, `editing_select_all`
- [x] **Double-click detection** (`oriterm/src/app/tab_bar_input.rs`, 444 lines):
  - [x] Add `last_tab_press: Option<(usize, Instant)>` field to `WindowContext`
  - [x] In `try_tab_bar_mouse` → `TabBarHit::Tab(idx)`: check double-click on same tab within `DOUBLE_CLICK_THRESHOLD`
  - [x] If double-click: call `ctx.tab_bar.start_editing(idx)`, mark dirty (do NOT start drag)
  - [x] If single-click: existing behavior (switch tab + start drag). Commit any active edit first
  - [x] Update `last_tab_press` timestamp on every `Tab(idx)` press
- [x] **Keyboard input during editing** (`oriterm/src/app/tab_bar_input.rs`):
  - [x] Add `handle_tab_editing_key(&mut self, event: &KeyEvent) -> bool` on `App`
  - [x] Called from `handle_keyboard_input()` BEFORE overlay/search/PTY dispatch
  - [x] Enter/Tab: commit edit → mark dirty
  - [x] Escape: cancel edit → restore original title
  - [x] Character input, Backspace, Delete, Arrow keys, Home/End, Ctrl+A all handled
  - [x] All other keys consumed during editing (prevents PTY leakage)
- [x] **Click-outside commits** (`oriterm/src/app/tab_bar_input.rs` + `event_loop.rs`):
  - [x] In `try_tab_bar_mouse`: if editing and hit is not `Tab(editing_index)`, commit first
  - [x] Grid click handler (`event_loop.rs` MouseInput): commit tab edit on press before `handle_mouse_input`
- [x] **Rendering during editing** (extracted to `widget/edit_draw.rs`, 90 lines; `draw.rs` at 479 lines):
  - [x] In `draw_tab`: when `self.editing_index == Some(index)`, call `draw_tab_editor` instead of `draw_tab_label`
  - [x] Editor rendering: shape editing text with same `TextStyle` as normal label
  - [x] Draw selection highlight: filled rect with `accent_bg.with_alpha(0.4)` behind selected range
  - [x] Draw cursor: 1px vertical line at cursor position
  - [x] Extracted to `widget/edit_draw.rs` submodule (draw.rs was at 541, now 479)
- [x] **New action variant** (`oriterm_ui/src/action/mod.rs`):
  - [x] Add `TabTitleChanged { index: usize, title: String }` to `WidgetAction`
  - [x] Added wildcard match in `content_actions.rs` (dialog context)
- [x] **Focus and editing lifecycle**:
  - [x] Window focus loss (`event_loop.rs` Focused(false)): commit any active tab edit
  - [x] Tab drag: double-click consumes event before drag starts — no drag during editing

**Tests:** (in `oriterm_ui/src/widgets/tab_bar/tests.rs` — 10 tests, all passing)
- [x] `start_editing_sets_index_and_selects_all`
- [x] `commit_editing_returns_trimmed_title`
- [x] `commit_editing_empty_restores_original`
- [x] `cancel_editing_restores_original_title`
- [x] `is_editing_false_after_commit`
- [x] `is_editing_false_after_cancel`
- [x] `keyboard_typing_inserts_characters`
- [x] `start_editing_out_of_bounds_is_noop`
- [x] `editing_backspace_deletes_character`
- [x] `editing_move_and_selection`

---

## 16.R Third Party Review Findings

- [ ] `[TPR-16-001][medium]` `oriterm_ui/src/widgets/tab_bar/widget/animation.rs:119` — The frame-based animation refactor still mutates `width_multipliers` for tab open/close, but nothing applies those animated widths back into `TabBarLayout`, so the width animation path is effectively dead. `update_animated_layout()` is now the only place that calls `recompute_layout_animated()`, yet no caller invokes it; meanwhile `draw_tab()` still renders geometry from `self.layout.tab_width_at(index)` and `prepaint()` only ticks the animation counters.
  Evidence: `animate_tab_open()` / `animate_tab_close()` update `width_multipliers`, `closing_complete()` polls those values, and `draw_tab()` uses them only for content opacity. The actual tab rectangle continues to come from the static layout cache because `update_animated_layout()` is unused.
  Impact: Tab open/close transitions no longer visually expand or shrink tabs. Closing now waits on the timer and then removes the tab without the planned width-collapse animation, which regresses section 16’s rendering and close-stress expectations.
  Required plan update: Recompute animated layout each frame before draw whenever `has_width_animation()` is true, and add a regression test that sampled tab widths change over the lifetime of an open/close animation rather than only the opacity values.

## 16.4 Section Completion

- [ ] All 16.1–16.3, 16.5 items complete
- [x] Tab bar layout: DPI-aware, width lock, platform-specific control zone
- [x] Tab bar rendering: separators with suppression, bell pulse, dragged tab overlay, animation offsets
- [x] Hit testing: correct priority order, close button inset, platform-specific controls
- [x] Tab width lock prevents close button shifting during rapid close clicks
- [x] `cargo build -p oriterm --target x86_64-pc-windows-gnu` — compiles
- [x] `cargo clippy -p oriterm --target x86_64-pc-windows-gnu` — no warnings
- [x] **Close stress test**: rapidly close many tabs while hovering tab bar — close buttons don't shift unexpectedly (tab width lock works)
- [x] **Visual test**: tab bar renders correctly at 100%, 125%, 150%, 200% DPI scales

- [ ] `/tpr-review` passed — independent Codex review found no critical or major issues (or all findings triaged)

**Exit Criteria:** Tab bar layout computes deterministically for any tab count and window width. GPU-rendered tab bar includes bell animation, drag overlay, and separator suppression. Hit testing dispatches clicks with correct priority ordering.
