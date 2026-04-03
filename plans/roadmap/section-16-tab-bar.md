---
section: 16
title: Tab Bar & Chrome
status: in-progress
reviewed: true
last_verified: "2026-04-03"
tier: 4
goal: Tab bar layout, rendering, and hit testing with DPI awareness
third_party_review:
  status: none
  updated: null
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
  - id: "16.4"
    title: Section Completion
    status: in-progress
---

# Section 16: Tab Bar & Chrome

**Status:** In Progress (16.1, 16.2, 16.5 complete; 16.3 mostly complete -- hover preview blocked on Section 39 image pipeline via 07.11 offscreen texture)
**Goal:** Tab bar layout, rendering, and hit testing with DPI awareness. Deterministic layout computation, GPU-rendered tab bar with bell pulse animation and drag overlay, and priority-based hit testing for click/hover dispatch.

**Crate:** `oriterm_ui` (tab bar widget, layout, hit testing, colors) + `oriterm` (input dispatch, session wiring)
**Dependencies:** `oriterm_ui` (Widget, Scene, UiTheme), `oriterm_mux` (PaneId), `oriterm/src/session/` (Tab, Window, SessionRegistry)
**Reference:** `_old/src/tab_bar.rs`, `_old/src/gpu/render_tab_bar.rs`, `_old/src/gpu/render_overlay.rs`

**Prerequisite:** Section 32 complete (mux-aware tab & window management available — `Tab`, `Window`, `SessionRegistry` in `oriterm/src/session/`).
---

## 16.1 Tab Bar Layout + Constants

Compute the pixel layout of tabs in the tab bar. All measurements are DPI-scaled. The layout is deterministic — given tab count, window width, and scale factor, the output is identical.

**File:** `oriterm_ui/src/widgets/tab_bar/` — `constants.rs`, `layout.rs`, `colors.rs`

**Reference:** `_old/src/tab_bar.rs`

**Deviation:** Layout computes in logical pixels (matching `ChromeLayout` pattern); scale applied at render boundary. Colors use `oriterm_ui::color::Color` (not `[f32; 4]`) and derive from `UiTheme` (not `Palette`), matching existing widget conventions. `window_width: f32` stored instead of `scale: f64` since scale is not needed for logical-pixel layout. Constants evolved to a brutal flat design: `TAB_BAR_HEIGHT` = 36 (not 46), `TAB_LEFT_MARGIN` = 0 (not 16), `TAB_MAX_WIDTH` = 200 (not 260), `TAB_PADDING` = 14 (not 8). Style-dependent dimensions factored into `TabBarMetrics` struct (DEFAULT + COMPACT presets).

- [x] Layout constants (all in logical pixels, multiply by `scale_factor` for physical): (verified 2026-03-29, all 15 constants present, sanity tests pass)
  - [x] `TAB_BAR_HEIGHT: f32 = 36.0` — full height of the tab bar
  - [x] `TAB_MIN_WIDTH: f32 = 80.0` — minimum tab width before they start overlapping
  - [x] `TAB_MAX_WIDTH: f32 = 200.0` — maximum tab width (tabs grow to fill available space, clamped here)
  - [x] `TAB_LEFT_MARGIN: f32 = 0.0` — horizontal margin before the first tab
  - [x] `TAB_TOP_MARGIN: f32 = 0.0` — vertical margin between top of window and top of tabs
  - [x] `TAB_PADDING: f32 = 14.0` — internal horizontal padding within each tab
  - [x] `TAB_BAR_BORDER_BOTTOM: f32 = 2.0` — height of the bottom border line beneath the tab bar
  - [x] `CLOSE_BUTTON_WIDTH: f32 = 24.0` — clickable area for the x button
  - [x] `CLOSE_BUTTON_RIGHT_PAD: f32 = 8.0` — spacing between x button and tab's right edge
  - [x] `NEW_TAB_BUTTON_WIDTH: f32 = 38.0` — width of the "+" button
  - [x] `DROPDOWN_BUTTON_WIDTH: f32 = 30.0` — width of the dropdown (settings/scheme) button
  - [x] `CONTROLS_ZONE_WIDTH` — platform-specific, derived from window chrome constants:
    - [x] Windows: `3 * CONTROL_BUTTON_WIDTH` (derived from `window_chrome::constants`)
    - [x] Linux/macOS: `12 + 3×24 + 2×8 + 12 = 100.0` (circular buttons with margins/spacing)
  - [x] `MACOS_TRAFFIC_LIGHT_WIDTH: f32 = 82.0` — reserved space for macOS native traffic lights (macOS only)
  - [x] `DRAG_START_THRESHOLD: f32 = 10.0` — pixels of movement before drag begins (matches Chrome's `tab_drag_controller.cc`)
  - [x] `TEAR_OFF_THRESHOLD: f32 = 40.0` — pixels outside tab bar before tear-off
  - [x] `TEAR_OFF_THRESHOLD_UP: f32 = 15.0` — reduced threshold for upward dragging (more natural for tear-off)
  - [x] `TabBarMetrics` struct — style-dependent dimensions (`height`, `top_margin`, `tab_padding`, `min_width`, `max_width`) with `DEFAULT` and `COMPACT` presets
- [x] `TabBarLayout` struct: (verified 2026-03-29, 20+ layout tests pass)
  - [x] `tab_width: f32` — base (uniform) width per tab
  - [x] `tab_count: usize` — number of tabs
  - [x] `window_width: f32` — window width used for layout
  - [x] `left_inset: f32` — extra left margin for platform chrome (macOS traffic lights)
  - [x] `tab_positions: Vec<f32>` — pre-computed X position of each tab (cumulative with multipliers)
  - [x] `per_tab_widths: Vec<f32>` — effective width of each tab (`tab_width * multiplier`)
  - [x] `tab_padding: f32` — from metrics
- [x] `TabBarLayout::compute(tab_count, window_width, tab_width_lock, left_inset, metrics) -> Self`
  - [x] If `tab_width_lock` is `Some(w)`: use locked width (prevents jitter during rapid close clicks or drag)
  - [x] Available width = `window_width - TAB_LEFT_MARGIN - left_inset - NEW_TAB_BUTTON_WIDTH - DROPDOWN_BUTTON_WIDTH - CONTROLS_ZONE_WIDTH`
  - [x] `tab_width = (available / tab_count).clamp(metrics.min_width, metrics.max_width)`
  - [x] Return layout struct
- [x] `TabBarLayout::compute_with_multipliers(...)` — variant with per-tab width scaling for open/close animations
- [x] `tab_width_lock: Option<f32>` on `TabBarWidget` (managed by App via `set_tab_width_lock`): (verified 2026-03-29)
  - [x] **Acquired** when: cursor enters tab bar (hovering), prevents tabs from expanding when quickly closing tabs
  - [x] **Released** when: cursor leaves tab bar, window resizes, tab count changes in ways that invalidate the lock (new tab, drag reorder)
  - [x] Purpose: If you have 5 tabs and close one, the remaining 4 tabs would normally expand. But if you're rapidly clicking close buttons, the expansion moves the next close button, causing you to miss. The lock freezes tab width during hover, so close buttons don't move.
- [x] `TabBarColors` struct — all colors needed for tab bar rendering: (verified 2026-03-29, 11 tab colors + separate ControlButtonColors)
  - [x] `bar_bg: Color` — tab bar background
  - [x] `active_bg: Color` — active tab background
  - [x] `inactive_bg: Color` — inactive tab background
  - [x] `tab_hover_bg: Color` — inactive tab background on hover
  - [x] `text_fg: Color` — active tab title text
  - [x] `inactive_text: Color` — inactive tab title text (dimmer)
  - [x] `separator: Color` — 1px vertical separator between tabs
  - [x] `close_fg: Color` — close button color (unhovered)
  - [x] `button_hover_bg: Color` — "+" and dropdown hover background
  - [x] `accent_bar: Color` — 2px accent bar on active tab top edge
  - [x] `bar_border: Color` — 2px bottom border line beneath the tab bar
  - [x] ~~`control_hover_bg`~~, ~~`control_fg`~~, ~~`control_fg_dim`~~, ~~`control_close_hover_bg`~~, ~~`control_close_hover_fg`~~ — moved to `ControlButtonColors` in `window_chrome::controls` (verified 2026-03-29, cleaner separation of concerns)
  - [x] `bell_pulse(phase) -> Color` method for bell animation interpolation
  - [x] Derived from theme: `TabBarColors::from_theme(theme: &UiTheme) -> Self`

---

## 16.2 Tab Bar Rendering

Render the tab bar as GPU instances. The tab bar is rendered in the overlay pass, after the terminal grid bg+fg passes. The dragged tab is rendered separately in a second overlay pass so it floats above everything.

**File:** `oriterm_ui/src/widgets/tab_bar/widget/` — `mod.rs` (TabBarWidget struct), `draw.rs` + `draw_helpers.rs` (tab rendering), `drag_draw.rs` (drag overlay), `controls_draw.rs` (window controls), `animation.rs` (hover/drag/open/close lifecycle), `edit_draw.rs` (inline title editing), `control_state.rs` (control button dispatch)

**Reference:** `_old/src/gpu/render_tab_bar.rs`, `_old/src/gpu/render_overlay.rs`

**Deviation:** Implemented as `TabBarWidget` (Widget→Scene→GPU pipeline) rather than `build_tab_bar_instances()` with `InstanceWriter`. The architecture evolved after the plan was written — widgets draw to a `Scene` in logical pixels, which is converted to GPU instances at the rendering boundary. `drag_visual` is `Option<(usize, f32)>` on the widget (single window, tab index + X). Animation offsets are `Vec<f32>` on the widget, populated by compositor-driven `TabSlideState`. Window control buttons are `[WindowControlButton; 3]` embedded in `TabBarWidget` (not a separate `WindowChromeWidget`), with `ControlButtonColors` from `window_chrome::controls` and controller-based press/release dispatch. Inline title editing added (double-click to edit, `TextEditingState` buffer, commit/cancel). Per-tab `AnimProperty` hover progress + close button opacity for smooth transitions. Width multiplier `AnimProperty` per tab for open/close animations. `TabBarMetrics` for style switching.

- [x] `TabBarWidget::paint()` — primary rendering function (verified 2026-03-29, Widget trait impl drawing to Scene)
  - [x] Input: `DrawCtx` (provides `Scene`, `UiTheme`, bounds, scale)
  - [x] Output: `Scene` primitives (quads, text, icons) in logical pixels, converted to GPU instances at render boundary
- [x] Rendering order (draw order matters for layering):
  1. [x] Tab bar background: full-width rectangle across top of window
  2. [x] Inactive tabs (drawn first, behind active tab):
     - [x] Background rectangle (with hover color if `hover_hit == Tab(idx)`)
     - [x] Title text: shaped with UI font collection, truncated with ellipsis if too wide
     - [x] Close button: vector x icon (visible on hover only, or always — configurable)
  3. [x] Active tab (drawn on top of inactive tabs):
     - [x] Background rectangle with `active_bg` color + **2px accent bar** on top edge (`accent_bar` color)
     - [x] Title text: brighter color than inactive (`text_fg`)
     - [x] Close button: always visible
  4. [x] Separators: 1px right-edge border per tab, with **suppression rules**:
     - [x] Active tab's right-edge separator suppressed (`i == active_index` skipped)
     - [x] Hovered tab's right-edge separator suppressed (`i == hovered_index` skipped)
     - [x] Dragged tab's right-edge separator suppressed (`i == dragged_index` skipped)
  5. [x] New tab "+" button: after the last tab
  6. [x] Dropdown button: after "+" button
  7. [x] Window control buttons: rightmost (see section 16)
- [x] Bell badge animation: (verified 2026-03-29, decaying sine wave + 6 tests)
  - [x] `bell_phase: f32` (0.0–1.0) — sine wave pulse
  - [x] Inactive tab with bell: `Color::lerp(inactive_bg, tab_hover_bg, bell_phase)` — smooth pulsing background (via `TabBarColors::bell_pulse()`)
  - [x] Phase computed from `bell_start: Option<Instant>` on the tab's terminal state
  - [x] Clear badge when tab becomes active
- [x] Dragged tab overlay: (verified 2026-03-29, drop shadow + button repositioning)
  - [x] When dragging: the dragged tab is **not rendered in the normal tab bar pass**
  - [x] Instead, rendered in a separate overlay pass via `draw_dragged_tab_overlay()` (called from `draw_drag_overlay()` public entry point)
  - [x] Rendering:
    1. `push_layer_bg(active_bg)` + flat rect with `active_bg` fill and drop shadow (4-component `Shadow` struct: offset, blur, spread, color)
    2. Tab content (title text + close icon) drawn at `drag_visual_x` position
    3. `pop_layer_bg()` closes the layer
  - [x] "+" and dropdown buttons reposition during drag: `max(default_x, drag_x + tab_w)` — keeps buttons visible even when dragging far right
- [x] `drag_visual: Option<(usize, f32)>` on `TabBarWidget`:
  - [x] `(tab_index, logical_x)` — which tab is dragged and its visual X position
  - [x] Separate from the tab's actual index in the vec — allows smooth visual feedback without real-time list manipulation
  - [x] Updated via `set_drag_visual()` on every mouse move during drag
- [x] Tab animation offsets: (verified 2026-03-29, compositor-driven TabSlideState, 20 tests)
  - [x] `anim_offsets: Vec<f32>` on `TabBarWidget` — per-tab pixel offsets for smooth transitions
  - [x] Compositor-driven via `TabSlideState` (`oriterm_ui/src/widgets/tab_bar/slide/`) — creates ephemeral `Group` layers with `Transform2D` translations, animated by `LayerAnimator`
  - [x] `swap_anim_offsets()` — zero-alloc buffer exchange between compositor and widget
  - [x] When tabs reorder during drag: displaced tabs get a non-zero offset that decays proportional to distance (80-200ms)
  - [x] Chrome-style behavior: tabs **snap immediately** to new positions during drag. Animation only applies on drag-end.
- [x] Tab title rendering: (verified 2026-03-29) <!-- unblocks:6.13 -->
  - [x] Use UI font collection (separate from terminal font, possibly different family/weight)
  - [x] `ctx.measurer.shape(title, &text_style.with_overflow(TextOverflow::Ellipsis), max_w)` — truncates with ellipsis if too wide
  - [x] Max text width = `tab_width - 2*TAB_PADDING - CLOSE_BUTTON_WIDTH - CLOSE_BUTTON_RIGHT_PAD` (computed by `TabBarLayout::max_text_width()`)
- [x] Inline title editing: (implemented, verified via `edit_draw.rs` + `tab_bar_input/editing.rs` + 19 unit tests)
  - [x] Double-click on tab starts editing via `start_editing(index)` — copies title to `TextEditingState`, selects all
  - [x] `edit_draw.rs` renders editing cursor + selection highlight in place of normal title
  - [x] `tab_edit_key_action()` pure function routes keyboard events to `TabEditAction` enum
  - [x] `commit_editing()` returns `(tab_index, new_title)` — trims whitespace, restores original on empty
  - [x] `cancel_editing()` restores original title (Escape)
  - [x] `Tab.title_override: Option<String>` in session model stores user-set title (persists across OSC updates)

---

## 16.3 Tab Bar Hit Testing

Map mouse coordinates to tab bar actions. Hit testing determines whether a click or hover targets a tab, a button, or the drag area.

**File:** `oriterm_ui/src/widgets/tab_bar/hit.rs` (hit test logic + `TabBarHit` enum), `oriterm/src/app/tab_bar_input/mod.rs` (mouse dispatch), `oriterm/src/app/chrome/mod.rs` (`update_tab_bar_hover`, `cursor_in_tab_bar`)

**Reference:** `_old/src/tab_bar.rs`

- [x] `TabBarHit` enum: (verified 2026-03-29, all 9 variants present)
  - [x] `Tab(usize)` — clicked on tab at index
  - [x] `CloseTab(usize)` — clicked close button on tab at index
  - [x] `NewTab` — clicked the "+" button
  - [x] `Dropdown` — clicked the dropdown/settings button
  - [x] `Minimize` — clicked window minimize
  - [x] `Maximize` — clicked window maximize/restore
  - [x] `CloseWindow` — clicked window close
  - [x] `DragArea` — clicked empty tab bar area (for window dragging or double-click maximize)
  - [x] `None` — click is below tab bar (terminal area)
- [x] `hit_test(x: f32, y: f32, layout: &TabBarLayout, bar_height: f32) -> TabBarHit` (logical pixels, no scale param) (verified 2026-03-29, 25+ hit tests)
  - [x] Priority order (checked first = higher priority):
    1. [x] If `y` outside `0..bar_height`: return `None` (below/above tab bar)
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
- [x] Tab bar hover tracking: (verified 2026-03-29)
  - [x] `hover_hit` on `TabBarWidget` (updated via `set_hover_hit` in `animation.rs`)
  - [x] `set_hover_hit()` also drives per-tab `hover_progress` and `close_btn_opacity` AnimProperty transitions
  - [x] Updated on every `CursorMoved` event (via `App::update_tab_bar_hover` in `oriterm/src/app/chrome/mod.rs`)
  - [x] When hover changes: mark dirty, request redraw
  - [x] Hover entering tab bar: acquire `tab_width_lock`
  - [x] Hover leaving tab bar: release `tab_width_lock` (skipped when tab drag is active to avoid premature release during tear-off)
  - [x] `clear_tab_bar_hover()` — resets hover state + control button hover when cursor leaves window entirely
  - [x] `update_control_hover_animation()` — drives `VisualStateAnimator` on each `WindowControlButton` based on hit result (not on macOS)
- [ ] Tab hover preview (Chrome/Windows-style): <!-- blocked-by: 07.11 (offscreen texture rendering, which itself is blocked on Section 39 image pipeline) -->
  - [ ] When hovering an inactive tab for > 300ms, show a `TerminalPreviewWidget` overlay
  - [ ] Preview appears below the tab bar, anchored to the hovered tab
  - [ ] Preview shows a live scaled-down render of that tab's terminal content
  - [ ] `TerminalPreviewWidget` scaffold exists at `oriterm/src/widgets/terminal_preview/mod.rs` — currently draws a placeholder rounded rect. Needs offscreen texture rendering wired to `push_image` (07.1 DrawCommand).
  - [ ] Fade-in animation (07.9 complete), dismiss on hover leave
  - [ ] Preview updates if the terminal content changes while hovering
  - [ ] No preview for the active tab (it's already visible)
  - [ ] **Tests (when unblocked):**
    - [ ] `WidgetTestHarness` test: hover inactive tab for >300ms shows overlay (`has_overlays() == true`)
    - [ ] `WidgetTestHarness` test: hover active tab never shows overlay
    - [ ] `WidgetTestHarness` test: hover leave dismisses overlay
    - [ ] `WidgetTestHarness` test: rapid tab switching does not leave stale preview
    - [ ] Unit test: `TerminalPreviewWidget::layout()` returns correct dimensions (`DEFAULT_PREVIEW_WIDTH` x `DEFAULT_PREVIEW_HEIGHT`)
- [x] Mouse press dispatch (in `App::try_tab_bar_mouse` at `oriterm/src/app/tab_bar_input/mod.rs`): (verified 2026-03-29, all branches implemented)
  - [x] `Tab(idx)`: switches tab via `switch_to_tab_index(idx)` + initiates drag via `try_start_tab_drag(idx)`. Double-click starts inline title editing.
  - [x] `CloseTab(idx)`: acquire `tab_width_lock`, close tab via `close_tab_at_index(idx)`
  - [x] `NewTab`: creates new tab via `new_tab_in_window(win_id)`
  - [x] `Dropdown`: opens dropdown menu via `open_dropdown_menu()`
  - [x] `Minimize`/`Maximize`/`CloseWindow`: routed through control button press/release cycle — press sets visual state, release fires action via `route_control_mouse()` (not on macOS — native traffic lights)
  - [x] `DragArea`:
    - [x] Double-click: toggle maximize (500ms threshold)
    - [x] Single-click: start window drag via `window.drag_window()`
  - [x] Right-click on `Tab(idx)`: opens tab context menu via `open_tab_context_menu(idx)`

---

## 16.5 Tab Icons & Emoji

Render emoji and icon characters in tab titles. The font pipeline already supports color emoji (Section 6.10 — CBDT/CBLC, COLR/CPAL via swash + Segoe UI Emoji / Noto Color Emoji fallback). This subsection wires that capability into the tab bar so that process icons, user-set emoji, and OSC-set icons display correctly next to tab titles.

**File:** `oriterm_ui/src/widgets/tab_bar/widget/draw.rs` (icon rendering), `oriterm_ui/src/widgets/tab_bar/widget/mod.rs` (`TabEntry`, `TabIcon`)

**Reference:** Windows Terminal profile `"icon"` setting, iTerm2 per-tab icon, OSC 1 (icon name)

- [x] Per-tab icon state: (verified 2026-03-29)
  - [x] `icon: Option<TabIcon>` on `TabEntry` (`oriterm_ui/src/widgets/tab_bar/widget/mod.rs`)
  - [x] `TabIcon` enum: `Emoji(String)` (single grapheme cluster)
  - [x] Default: `None` (no icon, title only — current behavior)
  - [x] `with_icon()` and `with_modified()` builder methods on `TabEntry`
- [x] Icon sources (priority order, highest wins): (verified 2026-03-29, OSC 0/1/2 pipeline tested)
  1. [x] **OSC 1 (Set Icon Name)**: VTE handler differentiated from OSC 0/2. `set_icon_name()` added to Handler trait. `Term.icon_name` field stores it. `Event::IconName`/`Event::ResetIconName` flow through mux pipeline to `Pane.icon_name`. `extract_emoji_icon()` detects leading emoji grapheme.
  2. [ ] **Profile config**: `[tab] icon = "🐍"` in TOML config per-profile (future, when profiles exist)
  3. [ ] **Process detection** (stretch goal): detect foreground process name — requires shell integration (Section 20)
- [x] Tab bar rendering changes: (verified 2026-03-29)
  - [x] When `tab.icon` is `Some(Emoji(s))`: shape emoji through UI font, render before title, shift title right by `icon_width + ICON_TEXT_GAP`
  - [x] When `tab.icon` is `None`: current behavior (title only, no shift)
  - [x] Same logic applied in both `draw_tab()` and `draw_dragged_tab_overlay()`
- [x] Color emoji atlas integration: (verified 2026-03-29)
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

## 16.4 Section Completion

- [ ] All 16.1–16.3, 16.5 items complete (blocked: hover preview in 16.3 requires offscreen texture rendering from Section 07.11, which is blocked on Section 39 image pipeline)
- [x] Tab bar layout: DPI-aware, width lock, platform-specific control zone, `TabBarMetrics` for style switching (verified 2026-03-29)
- [x] Tab bar rendering: separators with suppression, bell pulse, dragged tab overlay, animation offsets, accent bar, inline editing (verified 2026-03-29)
- [x] Hit testing: correct priority order, close button inset, platform-specific controls (verified 2026-03-29)
- [x] Tab width lock prevents close button shifting during rapid close clicks (verified 2026-03-29)
- [x] `cargo build -p oriterm --target x86_64-pc-windows-gnu` — compiles (verified 2026-03-29)
- [x] `cargo clippy -p oriterm --target x86_64-pc-windows-gnu` — no warnings (verified 2026-03-29)
- [x] **Close stress test**: rapidly close many tabs while hovering tab bar — close buttons don't shift unexpectedly (tab width lock works) (verified 2026-03-29)
- [x] **Visual test**: tab bar renders correctly at 100%, 125%, 150%, 200% DPI scales (verified 2026-03-29)
- [x] `control_state.rs` tested via tab_bar/tests.rs: `interactive_rects` (4 tests), `set_maximized_does_not_panic`, `dispatch_control_input_handles_click`, `action_for_control_maps_minimize`, `action_for_control_maps_close`, `update_control_hover_state_returns_animating`, `clear_control_hover_state_after_hover` (10 tests total, verified 2026-04-03)
- [x] `edit_draw.rs` rendering logic tested indirectly via inline editing unit tests in tab_bar/tests.rs: `start_editing_sets_index_and_selects_all`, `commit_editing_returns_trimmed_title`, `commit_editing_empty_restores_original`, `cancel_editing_restores_original_title`, `is_editing_false_after_commit`, `is_editing_false_after_cancel`, `start_editing_out_of_bounds_is_noop`, `editing_backspace_deletes_character`, `editing_move_and_selection` (9 tests, verified 2026-04-03). Visual output (cursor position, selection highlight) not directly asserted — covered by manual visual testing.
- [ ] Golden tests or scene capture tests for `Widget::draw()` output (gap: rendering output has no automated verification, only manual visual testing) <!-- deferred: requires scene serialization/comparison infrastructure -->
- [x] Unit tests for `tab_bar_input.rs` dispatch logic — extracted `tab_edit_key_action()` pure function, 19 tests covering all editing key paths. Added 2026-04-01.
- [x] Mux-aware wiring: tab switch (`switch_to_tab_index`), new tab (`new_tab_in_window`), close tab (`close_tab_at_index`) all wired through `SessionRegistry` + mux layer (Section 32 complete)
- [x] Context menus: tab right-click menu + dropdown menu wired through `open_tab_context_menu` / `open_dropdown_menu`
- [x] Chrome layout tests: 26 tests in `oriterm/src/app/chrome/tests.rs` covering `compute_window_layout` + `grid_origin_y` — DPI correctness at 100%/125%/150%/175%/200%/225%, hidden tab bar, compact style, status bar, border inset, integer-pixel alignment invariants (verified 2026-04-03)

**Exit Criteria:** Tab bar layout computes deterministically for any tab count and window width. GPU-rendered tab bar includes bell animation, drag overlay, and separator suppression. Hit testing dispatches clicks with correct priority ordering.

**Verification Notes (2026-04-03):** 202 tab_bar tests pass across 4 test files (146 in tab_bar/tests.rs, 25 in slide/tests.rs, 12 in emoji/tests.rs, 19 in tab_bar_input/tests.rs). Additionally, 26 layout/chrome tests in chrome/tests.rs cover `compute_window_layout` and `grid_origin_y` for DPI correctness. All source files under 500-line limit (largest: widget/mod.rs at 492 lines, draw.rs at 404, chrome/mod.rs at 404). Correct crate boundaries (no wgpu/winit in oriterm_ui tab bar code). Proper platform gating (`#[cfg(not(target_os = "macos"))]` for control buttons, `#[cfg(target_os = "windows")]` for controls zone). Two justified lint suppressions (`clippy::too_many_arguments` on `compute_with_multipliers`). Sibling tests.rs pattern followed everywhere. Mux-aware tab CRUD wired through Section 32. Inline title editing with `TextEditingState` + pure `tab_edit_key_action()` (19 tests). Compositor-driven `TabSlideState` for slide animations. Per-tab `AnimProperty` hover/close/width animations.
