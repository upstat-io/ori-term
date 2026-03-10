---
section: 48
title: Native OS Scrollbars
status: not-started
reviewed: false
tier: 5
goal: Platform-native overlay scrollbars for mouse-driven scrollback navigation, matching the host OS look and feel
sections:
  - id: "48.1"
    title: Scrollbar Model
    status: not-started
  - id: "48.2"
    title: Scrollbar Rendering
    status: not-started
  - id: "48.3"
    title: Scrollbar Input
    status: not-started
  - id: "48.4"
    title: Configuration
    status: not-started
  - id: "48.5"
    title: Section Completion
    status: not-started
---

# Section 48: Native OS Scrollbars

**Status:** Not Started
**Goal:** Platform-native overlay scrollbars that appear on scroll and fade out after inactivity, providing mouse-driven scrollback navigation. Match the host OS style: macOS overlay scrollbars, Windows 11 thin scrollbar, GTK/Wayland overlay scrollbar.

**Crate:** `oriterm` (rendering, input), `oriterm_ui` (scrollbar widget)
**Dependencies:** Section 05 (Window + GPU), Section 07 (UI Framework), Section 10 (Mouse Input)

**Reference:**
- Ghostty PR [#9225](https://github.com/ghostty-org/ghostty/pull/9225) and [#9232](https://github.com/ghostty-org/ghostty/pull/9232) — native scrollbar implementation
- Ghostty 1.3.0 release: "Platform-specific overlay scrollbars controlled via new `scrollbar` configuration option"
- Alacritty: no scrollbar (scroll via keyboard/mouse wheel only)
- WezTerm: native scrollbar on all platforms
- iTerm2: overlay scrollbar matching macOS style
- Windows Terminal: thin overlay scrollbar

**Why this matters:** Mouse-wheel scrolling is fine for small distances, but for navigating large scrollback (thousands of lines), a scrollbar is essential. It also provides visual feedback about position within the buffer. Ghostty's implementation proves users want this — it was one of the most requested features.

---

## 48.1 Scrollbar Model

Track scrollbar state: position, thumb size, visibility, and fade animation.

**File:** `oriterm_ui/src/widgets/scroll/scrollbar.rs` (may already exist partially)

- [ ] `ScrollbarState` struct:
  - [ ] `content_length: usize` — total rows (scrollback + visible)
  - [ ] `viewport_length: usize` — visible rows
  - [ ] `scroll_offset: usize` — current `display_offset`
  - [ ] `thumb_position: f32` — normalized 0.0..1.0
  - [ ] `thumb_size: f32` — normalized, proportional to viewport/content ratio
  - [ ] `visible: bool` — whether scrollbar is currently shown
  - [ ] `fade_alpha: f32` — current opacity for fade animation (1.0 → 0.0)
  - [ ] `hovered: bool` — mouse is over the scrollbar track
  - [ ] `dragging: bool` — actively dragging the thumb
- [ ] Update from grid state: `ScrollbarState::update(scrollback_len, visible_lines, display_offset)`
- [ ] Thumb position maps scroll position to track pixel range
- [ ] Minimum thumb size: 20px (prevent tiny thumb on huge scrollback)
- [ ] **Tests:**
  - [ ] Thumb size proportional to viewport/content ratio
  - [ ] Thumb position at 0.0 when at bottom, 1.0 when at top of scrollback
  - [ ] Minimum thumb size enforced

---

## 48.2 Scrollbar Rendering

Draw the scrollbar as an overlay on the right edge of the terminal grid. Overlay style — does not consume grid columns.

**File:** `oriterm/src/gpu/window_renderer/` or `oriterm_ui/src/widgets/scroll/scrollbar.rs`

- [ ] **Track** (background):
  - [ ] Right edge of terminal area, full height
  - [ ] Width: 8px (thin overlay), expands to 12px on hover
  - [ ] Background: semi-transparent (e.g., `rgba(128, 128, 128, 0.1)` idle, `rgba(128, 128, 128, 0.3)` on hover)
  - [ ] Only visible when hovered or recently scrolled
- [ ] **Thumb** (draggable indicator):
  - [ ] Rounded rectangle within the track
  - [ ] Color: semi-transparent foreground (adapts to theme)
  - [ ] Darker on hover, darkest while dragging
  - [ ] Corner radius: 4px
- [ ] **Fade animation**:
  - [ ] On scroll: instantly show at full opacity
  - [ ] After 1.5s inactivity: fade to 0 over 0.3s
  - [ ] On hover: cancel fade, show at full opacity
  - [ ] On drag: stay visible until drag ends + 1.5s delay
- [ ] Rendered AFTER terminal grid (on top, as overlay)
- [ ] Respects `ScaleFactor` for crisp rendering at all DPI levels
- [ ] **Tests:**
  - [ ] Scrollbar not rendered when `visible == false && fade_alpha == 0`
  - [ ] Thumb dimensions match state calculations
  - [ ] Fade animation progresses correctly over time

---

## 48.3 Scrollbar Input

Handle mouse interaction with the scrollbar: click-to-scroll, thumb dragging, and hover effects.

**File:** `oriterm/src/app/mouse_input.rs`

- [ ] **Hit testing**:
  - [ ] Scrollbar track occupies rightmost 12px of terminal area
  - [ ] Hit test checks if mouse is within track bounds
  - [ ] Only active when scrollbar is visible or when mouse enters track area
- [ ] **Click on track** (outside thumb):
  - [ ] Page-scroll toward click position (jump viewport by one page)
  - [ ] Alternative: scroll to position proportional to click location (instant jump)
  - [ ] Configurable behavior
- [ ] **Thumb drag**:
  - [ ] Mouse down on thumb: enter drag state, capture mouse
  - [ ] Mouse move: update `display_offset` proportional to thumb position delta
  - [ ] Mouse up: exit drag state
  - [ ] Smooth: map pixel delta to scroll offset continuously
- [ ] **Hover**:
  - [ ] Mouse enter track: expand width, show darker background
  - [ ] Mouse leave track: contract width, start fade timer
- [ ] **Mouse wheel passthrough**: when mouse is over scrollbar, wheel events still scroll normally
- [ ] **Interaction with mouse reporting**: scrollbar clicks are NOT forwarded to applications via mouse reporting
- [ ] **Tests:**
  - [ ] Click on track scrolls viewport
  - [ ] Drag thumb updates display_offset proportionally
  - [ ] Scrollbar clicks not sent to PTY

---

## 48.4 Configuration

User-configurable scrollbar behavior.

**File:** `oriterm/src/config/mod.rs`

- [ ] `[appearance] scrollbar` enum:
  - [ ] `overlay` — show overlay scrollbar on scroll, fade after inactivity (default)
  - [ ] `always` — always show scrollbar (visible track and thumb)
  - [ ] `never` — no scrollbar
- [ ] `[appearance] scrollbar_width` — thumb width in pixels (default: 8)
- [ ] Config hot-reload: scrollbar visibility updates immediately on config change
- [ ] **Tests:**
  - [ ] `never` mode: scrollbar never rendered
  - [ ] `always` mode: scrollbar always visible
  - [ ] `overlay` mode: scrollbar fades after inactivity

---

## 48.5 Section Completion

- [ ] All 48.1–48.4 items complete
- [ ] Overlay scrollbar appears on scroll, fades after inactivity
- [ ] Thumb drag scrolls proportionally through scrollback
- [ ] Track click page-scrolls
- [ ] Scrollbar hover expands width and darkens
- [ ] Three modes: `overlay`, `always`, `never`
- [ ] Scrollbar does not consume grid columns
- [ ] Scrollbar clicks not forwarded to PTY mouse reporting
- [ ] Works at all DPI/scale factors
- [ ] `cargo build --target x86_64-pc-windows-gnu` — clean build
- [ ] `cargo clippy --target x86_64-pc-windows-gnu` — no warnings
- [ ] `cargo test` — all tests pass

**Exit Criteria:** Users can visually see their position in scrollback and drag to navigate. The scrollbar matches modern OS conventions (thin overlay, fade on idle) and does not interfere with terminal content or mouse reporting.
