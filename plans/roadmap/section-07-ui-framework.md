---
section: 7
title: 2D UI Framework
status: in-progress
reviewed: true
last_verified: "2026-03-29"
tier: 2
goal: A lightweight GPU-agnostic UI framework (oriterm_ui) — drawing primitives, layout engine, and widget kit for ori_term's rich cross-platform UI (GPU conversion via wgpu lives in oriterm)
sections:
  - id: "07.1"
    title: Drawing Primitives
    status: complete
  - id: "07.2"
    title: Text Rendering Integration
    status: complete
  - id: "07.3"
    title: Layout Engine
    status: complete
  - id: "07.4"
    title: Hit Testing & Input Routing
    status: complete
  - id: "07.5"
    title: Focus & Keyboard Navigation
    status: complete
  - id: "07.6"
    title: Core Widgets
    status: complete
  - id: "07.7"
    title: Container Widgets
    status: complete
  - id: "07.8"
    title: Overlay & Modal System
    status: in-progress
  - id: "07.9"
    title: Animation
    status: complete
  - id: "07.10"
    title: Theming & Styling
    status: complete
  - id: "07.11"
    title: Terminal Grid Widget
    status: in-progress
  - id: "07.12"
    title: Section Completion
    status: in-progress
third_party_review:
  status: none
  updated: null
---

# Section 07: 2D UI Framework

**Status:** In Progress

**Goal:** Build a lightweight, GPU-agnostic 2D UI framework (`oriterm_ui`). This is what makes ori_term fundamentally different from Alacritty, Ghostty, and WezTerm — those terminals have essentially no UI. ori_term has a rich, cross-platform UI with settings panels, controls, command palette, context menus, and more. `oriterm_ui` produces `DrawList` commands; GPU conversion to wgpu instance buffers lives in `oriterm/src/gpu/window_renderer/draw_list.rs`. All consistent across Windows/Linux/macOS.

**Crate:** `oriterm_ui` (created in Section 03.5 with geometry, scale, hit_test, window foundation) — reusable, not coupled to terminal logic

**Dependencies:** `winit`, `oriterm_core` (for emoji detection), `log`, `smallvec`, `unicode-segmentation` — GPU-agnostic by design (draw list conversion lives in `oriterm/src/gpu/window_renderer/draw_list.rs`)

**Platform deps:** `windows-sys` (Win32 window management APIs, Windows only). **Build deps:** `image` (PNG decoding for window icon embedding).

**Additional modules in `oriterm_ui` (covered in other sections):**
- `compositor/` — layer tree, layer animator, composition pass (Section 43)
- `icons/` — vector icon path definitions and `IconResolver` trait (actual `tiny_skia` rasterization lives in `oriterm` crate)
- `window/` — window management types
- `hit_test/` — window chrome hit testing (Section 03.5)
- `platform_linux.rs`, `platform_macos.rs`, `platform_windows/` — platform-specific window management glue

**Reference:**
- Chrome's Views framework (widget tree, layout, hit testing, focus)
- Flutter's widget/render tree split
- egui's immediate-mode patterns (for inspiration, not architecture — we use retained mode)
- Zed's GPUI framework (GPU-rendered UI in Rust, similar goals)

**Design Principles:**
- Retained-mode widget tree (not immediate-mode — state lives in widgets, not rebuilt every frame)
- Layout is separate from rendering (compute layout once, render many frames until dirty)
- All rendering batched into GPU instance buffers (same pipeline as terminal grid)
- Pixel-perfect across platforms — no native widgets, no platform inconsistencies
- Damage-tracked — only re-layout and re-render what changed

---

## 07.1 Drawing Primitives (verified 2026-03-29)

The low-level 2D drawing API. Everything visible on screen is drawn through these primitives.

**File:** `oriterm_ui/src/draw/mod.rs`, `oriterm_ui/src/draw/draw_list.rs`, `oriterm_ui/src/draw/rect_style.rs`, `oriterm_ui/src/draw/shadow.rs`, `oriterm_ui/src/draw/border.rs`, `oriterm_ui/src/draw/gradient.rs`

- [x] `DrawList` — ordered list of draw commands, batched into GPU instance buffers
  - [x] `push_rect(rect: Rect, style: RectStyle)` — filled rectangle
  - [x] `push_text(position: Point, shaped: ShapedText, color: Color)` — pre-shaped text run (also captures bg_hint from layer stack for subpixel compositing)
  - [x] `push_line(from: Point, to: Point, width: f32, color: Color)` — line segment
  - [x] `push_image(rect: Rect, texture_id: u32, uv: [f32; 4])` — textured quad
  - [x] `push_icon(rect: Rect, atlas_page: u32, uv: [f32; 4], color: Color)` — vector icon from mono atlas (rasterized via `tiny_skia`)
  - [x] `push_clip(rect: Rect)` / `pop_clip()` — scissor rect stack
  - [x] `push_layer(bg: Color)` / `pop_layer()` — background color stack for subpixel text compositing
  - [x] `clear()` — reset for next frame
  - [x] `commands() -> &[DrawCommand]` — returns draw commands in painter's order; GPU conversion to instance buffers is in `oriterm/src/gpu/window_renderer/draw_list.rs` (`append_ui_draw_list_with_text`), keeping `oriterm_ui` GPU-agnostic

- [x] `RectStyle` — how to draw a rectangle
  - [x] `fill: Option<Color>` — solid fill color
  - [x] `border: Option<Border>` — border (uniform width and color)
  - [x] `corner_radius: [f32; 4]` — per-corner radius (TL, TR, BR, BL)
  - [x] `shadow: Option<Shadow>` — drop shadow (offset, blur, color)
  - [x] `gradient: Option<Gradient>` — linear/radial gradient fill

- [x] `Shadow` — box shadow via blurred rect behind the element
  - [x] `offset_x: f32`, `offset_y: f32`, `blur_radius: f32`, `spread: f32`, `color: Color`
  - [x] Rendered as a separate instance with expanded bounds and alpha falloff
  - [x] No multi-pass blur needed — approximate with pre-computed Gaussian texture or SDF

- [x] `Color` (`oriterm_ui/src/color/mod.rs`) — struct with `r: f32, g: f32, b: f32, a: f32` fields, with helper constructors
  - [x] `Color::hex(0xRRGGBB)`, `Color::rgba(r, g, b, a)`, `Color::rgb(r, g, b)`, `Color::hex_alpha(0xRRGGBBAA)`, `Color::from_rgb_u8(r, g, b)`
  - [x] Constants: `Color::WHITE`, `Color::BLACK`, `Color::TRANSPARENT`
  - [x] `to_array() -> [f32; 4]` for GPU upload, `with_alpha(a) -> Color`

- [x] `Point`, `Size`, `Rect`, `Insets` — already established in Section 03.5 (`oriterm_ui/src/geometry/`)
  - [x] Extend as needed for drawing (e.g., `Rect` already has `contains`, `intersects`, `inset`, `offset`, `union`, `from_ltrb`)

- [x] **Type-safe coordinate spaces** — add phantom type parameters to geometry types
  - [x] Marker types: `Logical` (device-independent pixels), `Physical` (hardware pixels), `Screen` (screen-absolute)
  - [x] `Point<U = Logical>`, `Size<U = Logical>`, `Rect<U = Logical>` — default parameter preserves all existing code unchanged
  - [x] Manual `Copy`/`Clone`/`Debug`/`PartialEq`/`Default` impls (derive doesn't work with phantom generics — Rust issue #26925)
  - [x] `Insets` stays unit-agnostic (deltas, not positions)
  - [x] `Scale<Src, Dst>` type complements `ScaleFactor` — encodes conversion direction at type level
    - [x] `Scale::uniform(factor)` — single `f32` scale factor (no per-axis `new(x, y)`)
    - [x] `transform_point(Point<Src>) -> Point<Dst>`, `transform_size`, `transform_rect`
    - [x] `inverse() -> Scale<Dst, Src>` — flips direction
    - [x] `factor() -> f32` — raw value accessor
  - [x] Boundary annotations: `hit_test()` takes `Point<Logical>`, GPU submission uses `Point<Physical>`, Win32 FFI uses `Point<Screen>`
  - [x] **Reference:** WezTerm's `PixelUnit`/`ScreenPixelUnit` phantom types, euclid's `Scale<T, Src, Dst>`, Chromium's `dip_util.h` conversion functions
  - [x] **Migration:** incremental — existing code stays on `Point` (= `Point<Logical>`), new boundary code annotates explicitly

- [x] Shader support:
  - [x] Rounded rectangle SDF in fragment shader (same shader, branched on corner_radius > 0)
  - [x] Border rendering via SDF edge detection
  - [x] All primitives batch into the existing instance buffer pipeline (no separate draw calls per shape)

---

## 07.2 Text Rendering Integration (verified 2026-03-29)

Bridge between the font pipeline (Section 06) and the UI framework.

**File:** `oriterm_ui/src/text/mod.rs`

- [x] `ShapedText` — pre-shaped, ready-to-draw text
  - [x] `glyphs: Vec<ShapedGlyph>` — positioned glyphs from rustybuzz
  - [x] `width: f32` — total advance width
  - [x] `height: f32` — line height
  - [x] `baseline: f32` — baseline offset
  - [x] `ShapedGlyph` — per-glyph output: `glyph_id: u16`, `face_index: u16`, `synthetic: u8` (bold/italic flags), `x_advance: f32`, `x_offset: f32`, `y_offset: f32`
  - [x] `TextMetrics` — lightweight measurement result (no glyph data): `width: f32`, `height: f32`, `line_count: u32` — used when only layout dimensions are needed

- [x] `TextStyle` — how to render text
  - [x] `font_family: Option<String>` — override font (default: UI font)
  - [x] `size: f32` — font size in points
  - [x] `weight: FontWeight` — Regular, Bold, etc.
  - [x] `color: Color`
  - [x] `align: TextAlign` — Left, Center, Right
  - [x] `overflow: TextOverflow` — Clip, Ellipsis, Wrap

- [x] `TextMeasurer` trait (in `oriterm_ui/src/widgets/text_measurer.rs`) — decouples widgets from font stack
  - [x] `fn measure(&self, text: &str, style: &TextStyle, max_width: f32) -> TextMetrics` — returns width, height, line count
  - [x] `fn shape(&self, text: &str, style: &TextStyle, max_width: f32) -> ShapedText` — full shaping via rustybuzz
  - [x] Does NOT rasterize during measure — only measures
  - [x] Concrete implementation lives in `oriterm` crate; `MockMeasurer` (8px/char, 16px line height) used in tests
  - [x] Handles wrapping at word boundaries if `overflow == Wrap`
  - [x] Handles ellipsis truncation if `overflow == Ellipsis`

- [x] UI font vs terminal font:
  - [x] Terminal grid uses the configured monospace font
  - [x] UI elements (buttons, labels, menus) use a proportional UI font
  - [x] Default UI font: system sans-serif (Segoe UI / SF Pro / Cantarell)
  - [x] Both go through the same atlas and shaping pipeline

---

## 07.3 Layout Engine (verified 2026-03-29)

Flexbox-inspired layout system. Compute positions and sizes for all widgets before rendering.

**File:** `oriterm_ui/src/layout/mod.rs`, `oriterm_ui/src/layout/solver.rs`, `oriterm_ui/src/layout/flex.rs`, `oriterm_ui/src/layout/layout_box.rs`, `oriterm_ui/src/layout/layout_node.rs`, `oriterm_ui/src/layout/constraints.rs`, `oriterm_ui/src/layout/size_spec.rs`

- [x] `LayoutNode` — computed layout result for one widget
  - [x] `rect: Rect` — final position and size in screen coordinates
  - [x] `content_rect: Rect` — rect minus padding
  - [x] `children: Vec<LayoutNode>` — child layout results
  - [x] `widget_id: Option<WidgetId>` — for hit testing (links layout node to widget)

- [x] `LayoutConstraints` — size constraints passed from parent to child
  - [x] `min_width: f32`, `max_width: f32`
  - [x] `min_height: f32`, `max_height: f32`

- [x] `SizeSpec` enum — how a widget sizes itself (named `SizeSpec`, not `Size`, to avoid collision with geometry `Size`)
  - [x] `Fixed(f32)` — exact pixel size
  - [x] `Fill` — expand to fill available space
  - [x] `FillPortion(u32)` — proportional fill (like CSS flex-grow)
  - [x] `Hug` — shrink to content size
  - [x] Min/Max constraints — handled via `LayoutBox` fields (standard Flutter/Iced pattern)

- [x] `Insets` struct — padding and margin (named `Insets` following Chromium/Flutter convention)
  - [x] `top: f32`, `right: f32`, `bottom: f32`, `left: f32`
  - [x] `Insets::all(v)`, `Insets::vh(v, h)`, `Insets::ZERO`

- [x] Flex layout algorithm:
  - [x] `Direction` — `Row` (horizontal) or `Column` (vertical)
  - [x] `Align` — `Start`, `Center`, `End`, `Stretch` (cross-axis)
  - [x] `Justify` — `Start`, `Center`, `End`, `SpaceBetween`, `SpaceAround` (main-axis)
  - [x] `Gap` — spacing between children
  - [x] Two-pass layout:
    1. Measure pass: each child reports preferred size given constraints
    2. Arrange pass: distribute remaining space among `Fill` children
  - [x] Handle `Hug` containers that shrink-wrap their children

- [x] `LayoutBox` — layout descriptor input to the solver (pure data, no trait objects)
  - [x] `width: SizeSpec`, `height: SizeSpec` — how dimensions are determined
  - [x] `padding: Insets`, `margin: Insets` — inner/outer spacing
  - [x] `min_width`, `max_width`, `min_height`, `max_height` — constraint bounds
  - [x] `content: BoxContent` — `Leaf { intrinsic_width, intrinsic_height }` or `Flex { direction, align, justify, gap, children }`
  - [x] `widget_id: Option<WidgetId>` — for hit testing linkage
  - [x] Builder API: `leaf()`, `flex()`, `with_width()`, `with_height()`, `with_padding()`, `with_margin()`, `with_min_width()`, `with_max_width()`, `with_align()`, `with_justify()`, `with_gap()`, `with_widget_id()`

- [x] `compute_layout(root: &LayoutBox, viewport: Rect) -> LayoutNode`
  - [x] Top-down constraint propagation, bottom-up size resolution
  - [x] Cache layout results — only recompute when dirty (tracked in 07.12)

---

## 07.4 Hit Testing & Input Routing (verified 2026-03-29)

Determine which widget is under the cursor and route mouse/keyboard events.

**File:** `oriterm_ui/src/input/` (event types, hit testing, routing), `oriterm_ui/src/widget_id.rs`

- [x] `WidgetId` — unique widget identifier (`widget_id.rs`)
  - [x] Newtype wrapping `u64`, generated via `WidgetId::next()` using a global `AtomicU64` counter
  - [x] `Copy`, `Eq`, `Hash` — usable as map keys and set members
  - [x] `raw() -> u64` — accessor for the underlying value

- [x] `layout_hit_test(root: &LayoutNode, point: Point) -> Option<WidgetId>`
  - [x] Walk layout tree back-to-front (last child drawn = frontmost = tested first)
  - [x] Respect clip rects (widget outside clip is not hittable) — via `layout_hit_test_clipped`
  - [x] Return the deepest widget containing the point

- [x] `InputState` — mouse routing state machine (`input/routing.rs`)
  - [x] Tracks `hovered: Option<WidgetId>` (hot), `captured: Option<WidgetId>` (active), `cursor_pos: Option<Point>`
  - [x] `process_mouse_event(event, layout) -> SmallVec<[RouteAction; 4]>` — hit-tests, generates hover transitions, auto-capture on Down / auto-release on Up (Chromium pattern: hover suppressed during capture)
  - [x] `RouteAction` enum: `Deliver { target, event }`, `Hover { target, kind }` — application layer dispatches these to widgets
  - [x] `keyboard_target(focus: &FocusManager) -> Option<WidgetId>` — associated function (not method) returning focused widget ID
  - [x] `set_capture(id)` / `release_capture()` — explicit capture management

- [x] Mouse event routing:
  - [x] `MouseEvent` — `{ kind: MouseEventKind, pos: Point, modifiers: Modifiers }`
  - [x] `MouseEventKind` — `Down(MouseButton)`, `Up(MouseButton)`, `Move`, `Scroll(ScrollDelta)`; `HoverEvent` — `Enter`, `Leave`
  - [x] Events dispatched to the hit-tested widget via `InputState::process_mouse_event`
  - [x] Hover state tracked: `Enter`/`Leave` generated automatically on cursor movement
  - [x] Capture: widget can capture mouse on `Down`, receives all events until `Up`

- [x] Keyboard event routing:
  - [x] Events go to the focused widget — `InputState::keyboard_target(focus)` returns focused WidgetId
  - [x] Unhandled events bubble up to parent — caller responsibility (documented contract)
  - [x] `KeyEvent` — custom struct with `key: Key` + `modifiers: Modifiers` (simpler than winit's model — only keys widgets handle: Enter, Space, Escape, arrows, Home/End, Character(char), etc.)
  - [x] `Modifiers` — bitmask struct with `shift()`, `ctrl()`, `alt()`, `logo()` predicates; constants `NONE`, `SHIFT_ONLY`, `CTRL_ONLY`, `ALT_ONLY`, `LOGO_ONLY`; `union()` for combining
  - [x] `Key` enum: `Enter`, `Space`, `Backspace`, `Delete`, `Escape`, `Tab`, `Home`, `End`, `ArrowUp`, `ArrowDown`, `ArrowLeft`, `ArrowRight`, `Character(char)`
  - [x] `MouseButton` enum: `Left`, `Right`, `Middle`
  - [x] `ScrollDelta` enum: `Pixels { x, y }`, `Lines { x, y }` — trackpad vs. mouse wheel
  - [x] `InputState::process_cursor_left() -> SmallVec<[RouteAction; 4]>` — generates `Leave` for hovered widget when cursor exits window, clears cursor position

- [x] Event response:
  - [x] `EventResponse` — `Handled`, `Ignored`, `RequestPaint`, `RequestLayout`, `RequestFocus`, `RequestRedraw` (backward alias for `RequestLayout`)
  - [x] Widgets return response to indicate whether they consumed the event
  - [x] `merge()` method resolves priority: `RequestLayout` > `RequestPaint` > `RequestFocus` > `Handled` > `Ignored`

---

## 07.5 Focus & Keyboard Navigation (verified 2026-03-29)

Focus ring for keyboard-driven UI navigation.

**File:** `oriterm_ui/src/focus/mod.rs`

- [x] `FocusManager` — tracks which widget has keyboard focus
  - [x] `focused: Option<WidgetId>`
  - [x] `focus_order: Vec<WidgetId>` — tab order (built from widget tree traversal)
  - [x] `set_focus(id: WidgetId)`
  - [x] `clear_focus()`
  - [x] `focus_next()` — Tab key advances focus
  - [x] `focus_prev()` — Shift+Tab moves focus backward
  - [x] `set_focus_order(order: Vec<WidgetId>)` — rebuilds tab order after layout changes; clears focus if the focused widget is no longer in the new order. Used by the overlay system for modal focus trapping via `modal_focus_order()`
  - [x] `focus_order() -> &[WidgetId]` — returns the current focus order
  - [x] `is_focused(id: WidgetId) -> bool` — convenience predicate

- [x] Focus visual:
  - [x] Focused widget renders a focus ring (2px outline, accent color)
  - [x] Optional per-widget: `focusable: bool` via `Widget::is_focusable()`

- [x] Keyboard shortcuts:
  - [x] `Tab` / `Shift+Tab` — cycle focus (via FocusManager)
  - [x] `Enter` / `Space` — activate focused button/checkbox
  - [x] `Escape` — close overlay, unfocus (Key::Escape in Key enum)
  - [x] `Arrow keys` — navigate within lists, dropdowns

---

## 07.6 Core Widgets (verified 2026-03-29)

The basic building blocks.

**File:** `oriterm_ui/src/widgets/` — one file per widget

### Label
- [x] Static or dynamic text display
- [x] `LabelWidget { text: String, style: LabelStyle }` — `widgets/label/mod.rs`
- [x] Supports single-line, ellipsis truncation configurable via `TextOverflow`

### Button
- [x] `ButtonWidget` with `WidgetAction::Clicked` (no closures) — `widgets/button/mod.rs`
- [x] States: Default, Hover, Pressed, Disabled, Focused
- [x] Visual: rounded rect background, centered text, hover highlight, focus ring
- [x] Keyboard: activatable via Enter/Space when focused

### Checkbox
- [x] `CheckboxWidget` with `WidgetAction::Toggled` — `widgets/checkbox/mod.rs`
- [x] Visual: box with checkmark lines, label to the right
- [x] Keyboard: toggle via Space when focused

### Toggle
- [x] `ToggleWidget` with `AnimatedValue<f32>` toggle progress — `widgets/toggle/mod.rs`
- [x] Visual: sliding pill (iOS-style toggle)
- [x] Smooth thumb sliding via animation system (150ms `EaseInOut`)

### Slider
- [x] `SliderWidget` with `WidgetAction::ValueChanged` — `widgets/slider/mod.rs`
- [x] Visual: track with draggable thumb, filled portion
- [x] Keyboard: arrow keys adjust value by step, Home/End jump to min/max

### Text Input
- [x] `TextInputWidget` with `WidgetAction::TextChanged` — `widgets/text_input/mod.rs` + `widget_impl.rs` (Widget trait impl extracted to submodule)
- [x] Single-line text entry with cursor, selection, keyboard editing
- [x] Visual: bordered rect, cursor, selection highlight, placeholder
- [x] Clipboard operations deferred — emits actions for app layer

### Dropdown
- [x] `DropdownWidget` trigger button — `widgets/dropdown/mod.rs`
- [x] Visual: button showing selected item + chevron indicator
- [x] Popup list deferred to overlay system (07.8)
- [x] Arrow Up/Down cycle through items

### Separator
- [x] `SeparatorWidget` horizontal/vertical with optional label — `widgets/separator/mod.rs`

### Infrastructure
- [x] `Widget` trait with `id()`, `is_focusable()`, `layout()`, `draw()`, `handle_mouse()`, `handle_hover()`, `handle_key()`, `accept_action()`, `focusable_children()`
- [x] `WidgetAction` enum: `Clicked`, `Toggled`, `ValueChanged`, `TextChanged`, `Selected`, `OpenDropdown`, `DismissOverlay`, `MoveOverlay`, `SaveSettings`, `CancelSettings`, `WindowMinimize`, `WindowMaximize`, `WindowClose`
- [x] `WidgetResponse` with `EventResponse` + optional `WidgetAction` + `CaptureRequest` (Acquire/Release/None)
- [x] `TextMeasurer` trait for decoupled text measurement
- [x] `LayoutCtx`, `DrawCtx`, `EventCtx` context structs
  - [x] `LayoutCtx` includes `measurer`, `theme`
  - [x] `DrawCtx` includes `measurer`, `draw_list`, `bounds`, `focused_widget`, `now`, `animations_running`, `theme`, `icons: Option<&ResolvedIcons>`
  - [x] `EventCtx` includes `measurer`, `bounds`, `is_focused`, `focused_widget`, `theme`
- [x] `KeyEvent` + `Key` enum added to `input/event.rs`
- [x] `MockMeasurer` for widget tests (8px/char, 16px line height)

### Additional Composite Widgets (built on core + container primitives)
- [x] `DialogWidget` — `widgets/dialog/` — modal dialog with header, body, footer; split into `mod.rs`, `style.rs`, `rendering.rs`
  > **Near limit.** `dialog/mod.rs` is 478 lines (limit: 500). Monitor on next modification.
- [x] `MenuWidget` — `widgets/menu/` — vertical list of selectable items; split into `mod.rs`, `widget_impl.rs`
- [x] `FormLayout` / `FormRow` / `FormSection` — `widgets/form_layout/`, `form_row/`, `form_section/` — structured form layouts with collapsible sections
- [x] `SettingsPanelWidget` — `widgets/settings_panel/` — complete settings UI; has `id_override_button.rs` submodule
- [x] `StatusBadgeWidget` — `widgets/status_badge/` — small status indicator
- [x] `TabBarWidget` — `widgets/tab_bar/` — browser-style tab bar; has deep submodule structure: `widget/` (mod.rs, draw.rs, drag_draw.rs), `slide/`, `emoji/`, `hit.rs`, `colors.rs`, `layout.rs`
  > **Near limit.** `tab_bar/widget/mod.rs` (486 lines) and `tab_bar/widget/draw.rs` (478 lines). Monitor on next modification.
- [x] `WindowChromeWidget` — `widgets/window_chrome/` — frameless window title bar; submodules: `controls.rs`, `layout.rs`, `constants.rs`

---

## 07.7 Container Widgets (verified 2026-03-29)

Widgets that contain and arrange other widgets. Children stored as `Box<dyn Widget>` for heterogeneous composition.

**Files:** `oriterm_ui/src/widgets/container/`, `oriterm_ui/src/widgets/panel/`, `oriterm_ui/src/widgets/spacer/`, `oriterm_ui/src/widgets/stack/`, `oriterm_ui/src/widgets/scroll/`

### Row / Column (Container Widget)
- [x] `ContainerWidget { direction: Direction, children: Vec<Box<dyn Widget>>, gap: f32, align: Align, justify: Justify, padding: Insets, ... }`
- [x] Replaces the earlier `FlexWidget` with additional capabilities: mouse capture semantics, post-construction child management, padding, and explicit sizing via `SizeSpec`
- [x] The primary layout container — everything is nested Rows and Columns
- [x] Delegates to the flex layout algorithm (07.3)

### Scroll Container
- [x] `ScrollWidget` — `widgets/scroll/mod.rs` + `scrollbar.rs` (scrollbar style, policy, rendering extracted to submodule)
- [x] `ScrollWidget { child: Box<dyn Widget>, scroll_offset: f32, direction: ScrollDirection }`
- [x] Clips child to container bounds
- [x] Scrollbar: thin overlay scrollbar with configurable `ScrollbarStyle` (width, thumb color, track color, thumb radius, min height) and `ScrollbarPolicy` (Auto/Always/Hidden)
- [x] Scrollbar thumb drag interaction with mouse capture
- [x] Mouse wheel scrolling, trackpad smooth scroll (via `ScrollDelta::Pixels` / `ScrollDelta::Lines`)
- [x] Keyboard: ArrowUp/ArrowDown (scroll by `line_height`), Home/End (jump to top/bottom)
- [x] Add `Key::PageUp` and `Key::PageDown` variants to `input/event.rs`, then handle in `ScrollWidget::handle_key` to scroll by visible height
- [x] `ScrollDirection` enum: `Vertical`, `Horizontal`, `Both` — vertical is default
- [x] Cached child layout (`RefCell<Option<(Rect, Rc<LayoutNode>)>>`) — avoids re-measuring on every event
- [x] Child mouse capture forwarding — when child acquires capture, scroll events bypass scrollbar

### Panel
- [x] `PanelWidget { child: Box<dyn Widget>, style: PanelStyle }`
- [x] Visual container with background, border, rounded corners, shadow
- [x] Used for settings panels, dialog backgrounds, card-style layouts

### Spacer
- [x] `SpacerWidget { size: Size }` — flexible or fixed empty space
- [x] `Spacer::fill()` — pushes siblings to opposite ends

### Stack (Z-axis)
- [x] `StackWidget { children: Vec<Box<dyn Widget>> }` — children overlaid on top of each other
- [x] Used for positioning elements absolutely within a relative container
- [x] Last child is frontmost

---

## 07.8 Overlay & Modal System (verified 2026-03-29 -- infrastructure complete, consumers deferred)

Floating UI that renders above the main widget tree.

**Files:** `oriterm_ui/src/overlay/` — `overlay_id.rs`, `placement.rs`, `manager/mod.rs`, `manager/event_routing.rs`, `mod.rs`, `tests.rs`

> **File size resolved (verified 2026-03-29).** `overlay/manager/mod.rs` split to 302 lines; lifecycle methods extracted to `lifecycle.rs`, event routing to `event_routing.rs`.

- [x] `OverlayId` — unique identifier (separate ID space from `WidgetId`)
  - [x] Atomic counter, same pattern as `WidgetId`

- [x] `OverlayManager` — manages floating layers above the main content
  - [x] `overlays: Vec<Overlay>` — stack of active overlays (frontmost = last)
  - [x] `dismissing: Vec<Overlay>` — overlays being faded out (still drawn, excluded from event routing)
  - [x] `hovered_overlay: Option<usize>` — index of overlay under cursor (for Enter/Leave transitions)
  - [x] `captured_overlay: Option<usize>` — index of overlay with active mouse capture (drag in progress); all mouse events route to captured overlay regardless of cursor position
  - [x] `layout_dirty: bool` — set on push/remove/viewport change; cleared after `layout_overlays`
  - [x] `push_overlay(widget, anchor, placement, tree, animator, now) -> OverlayId` (dismiss-on-click-outside; creates `Textured` compositor layer)
  - [x] `push_modal(widget, anchor, placement, tree, animator, now) -> OverlayId` (blocks interaction below; creates dim + content compositor layers)
  - [x] `begin_dismiss(id, tree, animator, now) -> bool`, `begin_dismiss_topmost(tree, animator, now) -> Option<OverlayId>`, `clear_all(tree, animator)` — popups removed instantly, modals fade out via compositor
  - [x] `cleanup_dismissed(tree, animator)` — removes fully-faded overlays after animator tick
  - [x] `overlay_rect(id) -> Option<Rect>` — computed rect accessor
  - [x] `offset_topmost(dx, dy) -> bool` — shifts topmost overlay by delta (header drag repositioning); switches placement to `AtPoint` so subsequent `layout_overlays` calls preserve dragged position; clamps to viewport
  - [x] `accept_action_topmost(action) -> bool` — propagates an action to the topmost overlay's widget tree (e.g., updating dropdown's selected index after popup dismissal)
  - [x] `set_viewport(viewport)` — updates viewport bounds on window resize; marks layout dirty
  - [x] `is_empty() -> bool` — true if no active or dismissing overlays
  - [x] `has_modal() -> bool` — true if topmost overlay is modal
  - [x] `count() -> usize` — number of active overlays

- [x] `Placement` — where to position the overlay relative to its anchor
  - [x] `Below`, `BelowFlush` (zero gap), `Above`, `Left`, `Right` — auto-flip if insufficient space
  - [x] `Center` — centered on screen (for modals)
  - [x] `AtPoint(Point)` — positioned at absolute point (for context menus)
  - [x] `compute_overlay_rect()` — pure function with auto-flip + viewport clamping
  - [x] `ANCHOR_GAP` constant (4px spacing between anchor and overlay)

- [x] `OverlayEventResult` — event routing results
  - [x] `Delivered { overlay_id, response }` — event delivered to overlay widget
  - [x] `Dismissed(OverlayId)` — click-outside or Escape dismissed overlay
  - [x] `Blocked` — modal consumed the event
  - [x] `PassThrough` — no overlay intercepted; deliver to main tree

- [x] Frame-loop API:
  - [x] `layout_overlays(measurer, theme)` — computes content size → placement rect (skips if not dirty)
  - [x] `draw_overlay_at(draw_idx, DrawCtx, tree) -> f32` — draws single overlay, returns compositor opacity; replaces `draw_overlays`
  - [x] `draw_count() -> usize` — total overlays to draw (active + dismissing)
  - [x] `process_mouse_event(event, measurer, theme, focused_widget, tree, animator, now) -> OverlayEventResult`
  - [x] `process_key_event(event, measurer, theme, focused_widget, tree, animator, now) -> OverlayEventResult`
  - [x] `process_hover_event(point, measurer, theme, focused_widget) -> OverlayEventResult`
  - [x] `modal_focus_order() -> Option<Vec<WidgetId>>` — focus trapping (no measurer needed)

- [x] Overlay rendering:
  - [x] Overlays render after the main widget tree (on top)
  - [x] Background dimming for modals (semi-transparent black layer)
  - [x] Click-outside-to-dismiss behavior for non-modal overlays
  - [x] Escape key dismisses topmost overlay (modal or non-modal)

- [x] `WidgetAction::DismissOverlay(WidgetId)` — overlay content widgets can signal self-dismissal
- [x] `WidgetAction::MoveOverlay { delta_x, delta_y }` — overlay header drag repositioning (via `offset_topmost`)

- [x] Rich overlay content — overlays can contain any widget (Box<dyn Widget>)

- [ ] Overlay consumers (wiring deferred to their respective sections): <!-- blocked-by:21 --><!-- blocked-by:27 --><!-- blocked-by:24 --><!-- blocked-by:11 --><!-- blocked-by:16 -->
  - [ ] Context menus — right-click popup (Section 21) <!-- blocked-by:21 -->
  - [ ] Dropdown lists — popup on `OpenDropdown` action (Section 21) <!-- blocked-by:21 -->
  - [ ] Command palette — fuzzy search overlay (Section 27) <!-- blocked-by:27 -->
  - [ ] Settings panel — modal dialog (Section 21) <!-- blocked-by:21 -->
  - [ ] Tooltips — hover-triggered overlay (Section 24) <!-- blocked-by:24 -->
  - [ ] Search bar — overlay anchored to top of terminal (Section 11) <!-- blocked-by:11 -->
  - [ ] Tab hover previews — Chrome/Windows-style terminal thumbnail overlay (Section 16) <!-- blocked-by:16 -->

---

## 07.9 Animation (verified 2026-03-29)

Smooth transitions for UI state changes.

**Files:** `oriterm_ui/src/animation/mod.rs`, `oriterm_ui/src/animation/builder.rs`, `oriterm_ui/src/animation/delegate.rs`, `oriterm_ui/src/animation/group.rs`, `oriterm_ui/src/animation/sequence.rs`, `oriterm_ui/src/animation/tests.rs`

- [x] `Lerp` trait — generic linear interpolation
  - [x] Implemented for `f32`, `Color` (channel-wise), `Point<U>`, `Size<U>`, `Rect<U>`, `Transform2D`

- [x] `Easing` — timing functions
  - [x] `Linear`, `EaseIn`, `EaseOut`, `EaseInOut`
  - [x] `CubicBezier(x1, y1, x2, y2)` — Newton's method + bisection fallback

- [x] `Animation` — raw `f32` interpolation from one value to another
  - [x] `from: f32`, `to: f32`, `duration: Duration`, `easing: Easing`
  - [x] `progress(now: Instant) -> f32` — returns current eased value
  - [x] `is_finished(now: Instant) -> bool`

- [x] `AnimatedValue<T: Lerp>` — widget-embeddable wrapper
  - [x] `set(new_value, now)` — starts animation from current to new
  - [x] `set_immediate(value)` — sets without animation
  - [x] `get(now: Instant) -> T` — returns interpolated value
  - [x] `target() -> T` — returns final resting value
  - [x] `is_animating(now) -> bool` — animation in flight
  - [x] Smooth interruption: `set` mid-animation restarts from current position

- [x] `AnimationBuilder` — fluent builder for animation parameters (`builder.rs`)
- [x] `AnimationDelegate` / `AnimatableProperty` — property-based animation dispatch (`delegate.rs`); `AnimatableProperty` enum: `Opacity`, `Transform`
- [x] `AnimationGroup` / `PropertyAnimation` / `TransitionTarget` — coordinated multi-property animations (`group.rs`)
- [x] `AnimationSequence` / `AnimationStep` / `SequenceState` — chained sequential animations (`sequence.rs`)

- [x] `DrawCtx` integration — `now: Instant` + `animations_running: &Cell<bool>`
  - [x] Widgets set `animations_running` to request continued redraws

- [x] Used for:
  - [x] Toggle switch sliding (150ms `EaseInOut`)
  - [x] Button hover color transitions (100ms `EaseOut`)
  - [x] Overlay fade-in/fade-out (implemented via compositor `LayerAnimator::animate_opacity`, Section 43.10)
  - [x] Tab bar tab sliding (implemented via compositor `animate_transform`, Section 43.11)

---

## 07.10 Theming & Styling (verified 2026-03-29)

Consistent visual styling across all widgets.

**File:** `oriterm_ui/src/theme/mod.rs`

- [x] `UiTheme` — all UI colors, sizes, and spacing in one struct
  - [x] `bg_primary: Color` — main background
  - [x] `bg_secondary: Color` — panel/card background
  - [x] `bg_hover: Color` — hover highlight
  - [x] `bg_active: Color` — pressed/active state
  - [x] `fg_primary: Color` — primary text
  - [x] `fg_secondary: Color` — secondary/dimmed text
  - [x] `fg_disabled: Color` — disabled state text
  - [x] `accent: Color` — accent color (focus ring, toggle on, selection)
  - [x] `border: Color` — default border color
  - [x] `shadow: Color` — shadow color (semi-transparent black)
  - [x] `close_hover_bg: Color` — close button hover background (platform standard red)
  - [x] `close_pressed_bg: Color` — close button pressed background (darker red)
  - [x] `corner_radius: f32` — default corner radius
  - [x] `spacing: f32` — default gap between elements
  - [x] `font_size: f32` — default UI font size
  - [x] `font_size_small: f32` — small text
  - [x] `font_size_large: f32` — headings

- [x] `UiTheme::dark() -> Self` — dark theme defaults
- [x] `UiTheme::light() -> Self` — light theme defaults
- [x] Theme propagates through the widget tree (widgets inherit from parent unless overridden)
- [x] Integrates with system theme detection (auto dark/light via `resolve_ui_theme`)

---

## 07.11 Terminal Grid Widget (verified 2026-03-29 -- core widget done, preview blocked on Section 39)

The terminal grid itself is a widget within the UI framework. Uses a **hybrid approach**: layout and events go through the Widget trait, but cell rendering stays in the existing GPU prepare pipeline (no DrawList overhead for 1920+ cells/frame). The widget lives in `oriterm/src/widgets/` (binary crate) because it needs terminal types.

**Files:** `oriterm/src/widgets/terminal_grid/mod.rs`, `oriterm/src/widgets/terminal_preview/mod.rs`, `oriterm/src/gpu/prepare/mod.rs` (origin parameter)

- [x] `TerminalGridWidget` — terminal grid as a layout + event participant
  - [x] Implements `Widget` trait with `Fill × Fill` sizing (expands to fill remaining space)
  - [x] `is_focusable() → true` — claims keyboard input when focused
  - [x] `handle_key()` → `Handled` — all keys go to PTY
  - [x] `handle_mouse()` → `Handled` — all mouse events in grid area
  - [x] `draw()` stores computed bounds via `Cell` interior mutability (no DrawCommands — cells rendered by GPU prepare pipeline)
  - [x] `bounds()` accessor for app to read layout origin
  - [x] `set_cell_metrics()` / `set_grid_size()` — updated on resize
  - [x] Reports preferred size based on cell dimensions and grid size
  - [x] `set_bounds(rect)` — stores layout bounds from `compute_window_layout` results for the GPU prepare pipeline to read.
  - [ ] *(Alternative path — not planned)* Direct DrawList cell rendering: `RenderableContent` to `DrawCommand`s for backgrounds, glyphs, cursor, selection, search highlights. Would unify rendering but add DrawList overhead for 1920+ cells/frame. <!-- deferred: alternative path, not planned -->
  - [ ] *(Blocked on Section 39 image pipeline)* Offscreen texture rendering: render grid to offscreen texture at arbitrary scale for thumbnails/previews. <!-- blocked-by:39 -->

- [x] Grid origin offset in prepare pipeline
  - [x] `origin: (f32, f32)` parameter on `fill_frame_shaped()`, `prepare_frame_shaped_into()`, and related functions — pixel offset for all cell positions
  - [x] Applied in `fill_frame_shaped` (production path) and `prepare_frame_shaped` (test path)
  - [x] Derived from `TerminalGridWidget::bounds()` in `handle_redraw()` and passed to `renderer.prepare()`
  - [x] Zero-cost when `(0.0, 0.0)` — compiler optimizes the addition away

- [x] `TerminalPreviewWidget` — scaffold for scaled-down live preview (currently `#[allow(dead_code)]` — wired in tab hover preview section)
  - [x] Fixed-size layout (`320×200` default, configurable via `with_size(width, height, scale)`)
  - [x] `is_focusable() → false`
  - [x] Placeholder draw: rounded rectangle with theme background (`bg_secondary`, `CORNER_RADIUS = 6.0`)
  - [ ] Render terminal at thumbnail resolution to offscreen texture <!-- blocked-by:39 -->
  - [ ] Display in overlay on tab hover <!-- blocked-by:39 --><!-- blocked-by:16 -->
  - [ ] Re-render only when source terminal content is dirty <!-- blocked-by:39 -->
  - [ ] Apply rounded corners, subtle shadow, smooth fade-in animation <!-- blocked-by:39 -->
  - [ ] Wire to consumers: tab bar hover, taskbar window preview, window switcher <!-- blocked-by:16 --><!-- blocked-by:39 -->

- [x] Integration:
  - [x] `WindowContext` owns `TerminalGridWidget` (non-optional), created in `try_init()` and `create_window()`
  - [x] Grid widget updated on resize (cell metrics + grid size)
  - [x] Grid `origin` derived from `terminal_grid.bounds()` in `handle_redraw()` and passed to `renderer.prepare()`
  - [x] The terminal grid fills the remaining space after UI chrome
  - [x] Grid receives keyboard input when focused (which is the default state)
  - [x] Mouse events within the grid are routed to terminal mouse handling
  - [x] Wire main window layout as `Column { TabBar, TerminalGrid, StatusBar(optional) }` through the layout engine (currently positioned manually)

- [ ] Unify tab bar, context menus, settings, search overlay, terminal previews, and terminal grid through the same DrawList rendering pipeline (foundation laid — individual wiring in consuming sections) <!-- blocked-by:21 --><!-- blocked-by:27 --><!-- blocked-by:39 -->

---

## 07.12 Section Completion (verified 2026-03-29)

- [ ] All 07.1-07.11 unchecked items complete (remaining: 07.8 overlay consumers, 07.11 preview widget + layout engine wiring) <!-- blocked-by:11 --><!-- blocked-by:16 --><!-- blocked-by:21 --><!-- blocked-by:24 --><!-- blocked-by:27 --><!-- blocked-by:39 -->
- [x] Layout caching in `compute_layout` — skip recomputation when layout is not dirty (deferred from 07.3) (verified 2026-03-29)
- [x] Drawing primitives render correctly: rects, rounded rects, shadows, text, lines (verified 2026-03-29 -- 24 draw tests)
- [x] Layout engine computes correct positions for nested flex containers (verified 2026-03-29 -- 71 layout tests)
- [x] Hit testing correctly identifies the widget under the cursor (verified 2026-03-29 -- 33 input tests + 27 hit_test tests)
- [x] Focus management: Tab cycles through focusable widgets (verified 2026-03-29 -- 11 focus tests)
- [x] Core widgets render and respond to input: Button, Checkbox, Toggle, Slider, TextInput, Dropdown (verified 2026-03-29 -- all 18 widget types tested)
- [x] Overlays render above main content, dismiss on click-outside (verified 2026-03-29 -- 76 overlay tests)
- [x] Animations interpolate smoothly (no jank, no allocation per frame) (verified 2026-03-29 -- 63 animation tests)
- [x] Theme system provides consistent dark/light styling (verified 2026-03-29 -- 14 theme tests)
- [x] Terminal grid renders as a widget within the framework (verified 2026-03-29 -- 11 tests)
- [x] Tab bar renders as a widget within the framework (verified 2026-03-29 -- 155 tests)
- [x] All widgets are GPU-rendered — no native OS widgets used (verified 2026-03-29)
- [x] Performance: UI framework adds negligible overhead to frame time (verified 2026-03-29)
- [x] No platform-specific code in the widget and rendering layers (pure Rust, GPU-agnostic — no wgpu dependency in `oriterm_ui`). Platform-specific code is isolated to window management modules (`platform_linux.rs`, `platform_macos.rs`, `platform_windows/`). (verified 2026-03-29 -- Cargo.toml deps confirmed)
- [x] `cargo clippy -p oriterm_ui` — no warnings (verified 2026-03-29)
- [x] **File size compliance:** Split `overlay/manager/mod.rs` (523->302 lines, lifecycle methods extracted to `lifecycle.rs`, event routing to `event_routing.rs`). Monitor files approaching the limit on next modification: `tab_bar/widget/mod.rs` (486), `dialog/mod.rs` (478), `tab_bar/widget/draw.rs` (478), `form_section/mod.rs` (460), `platform_windows/mod.rs` (461), `compositor/layer_animator.rs` (448), `window_chrome/mod.rs` (444), `scroll/mod.rs` (443). (verified 2026-03-29)
- [x] **Test infrastructure:** 39 `tests.rs` sibling files across the crate (not 24 as previously counted). All use `MockMeasurer` (8px/char, 16px line height). No GPU or platform runtime required. Total: 1104 oriterm_ui tests + 16 oriterm widget tests = 1120 tests. (verified 2026-03-29)

> **CLAUDE.md accuracy note (2026-03-29):** CLAUDE.md describes `WindowRoot`, `InteractionManager`, `VisualStateAnimator`, `EventControllers` (HoverController, ClickController, DragController), and a propagation pipeline as the "Zero Exceptions Rule" target architecture. It also lists `oriterm_ui/src/window_root/`, `oriterm_ui/src/interaction/`, `oriterm_ui/src/pipeline/`, `oriterm_ui/src/testing/` as existing directories. **None of these exist yet.** The current widget system uses direct `handle_mouse()`/`handle_hover()`/`handle_key()` methods on the `Widget` trait with per-widget state tracking and `AnimatedValue<f32>` for hover/toggle animations. This is a functional and well-tested retained-mode system. The CLAUDE.md describes aspirational target architecture, not current state. This is a CLAUDE.md accuracy issue, not a Section 07 gap.

**Exit Criteria:** A complete, lightweight, GPU-rendered UI framework that can build settings panels, context menus, command palette, and any future UI. The terminal grid is just another widget. All rendering is consistent, cross-platform, and fast.
