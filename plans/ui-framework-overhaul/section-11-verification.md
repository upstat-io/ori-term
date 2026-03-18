---
section: "11"
title: "Verification"
status: not-started
goal: "Comprehensive verification that the framework and settings panel work correctly with no regressions"
depends_on: ["10"]
reviewed: true
sections:
  - id: "11.1"
    title: "Test Matrix"
    status: not-started
  - id: "11.2"
    title: "Visual Verification"
    status: not-started
  - id: "11.3a"
    title: "Performance Validation (Automatable)"
    status: not-started
  - id: "11.3b"
    title: "Performance Validation (Manual)"
    status: not-started
  - id: "11.4"
    title: "Cross-Platform"
    status: not-started
  - id: "11.5"
    title: "Documentation"
    status: not-started
  - id: "11.6"
    title: "Completion Checklist & Sync Point Verification"
    status: not-started
---

# Section 11: Verification

**Status:** Not Started
**Goal:** All framework components and the settings panel are thoroughly tested, visually
verified, performance-validated, and documented. No regressions in terminal functionality.

**Depends on:** Section 10 (Settings Panel — everything must be built).

**Implementation order:**
1. **11.4 Cross-Platform** — verify builds pass on all targets first (if it doesn't compile,
   nothing else matters).
2. **11.1 Test Matrix** — write any missing tests, verify all pass (`./test-all.sh`).
3. **11.5 Documentation** — audit doc comments and file sizes (can parallel with 11.1).
4. **11.3a Automatable Performance Tests** — add any missing performance unit tests.
5. **11.2 Visual Verification** + **11.3b Manual Performance** — requires running binary.
6. **11.6 Completion Checklist** — final sign-off after all above pass.

---

## 11.1 Test Matrix

**Test placement rule:** Every test listed below goes into the module's existing sibling
`tests.rs` file (e.g., interaction tests in `interaction/tests.rs`, controller tests in
`controllers/tests.rs` or `controllers/<name>/tests.rs`, widget tests in
`widgets/<name>/tests.rs`). Do not create a new centralized test file. Tests that span
multiple modules (integration tests) go in the higher-level module's `tests.rs` or in
a `tests/` integration test directory.

**Implementation order:** Verify `oriterm_ui` tests first (Interaction State through Layout),
then `oriterm` crate tests (Widget Pipeline, Settings Panel, migrated widgets in oriterm).
This respects the library-before-binary dependency ordering from impl-hygiene.md.

- [ ] **Interaction State** (tests in `interaction/tests.rs`):
  - Hot path computation with nested widgets
  - Hot path update on pointer move (enter/leave lifecycle events)
  - Active state capture and release
  - Focus request and transfer
  - focus_within propagation to ancestors
  - Disabled widget skipped in hot tracking
  - `deregister_widget` on active/focused widget clears state and emits lifecycle events
  - `get_state` returns safe default (all-false) for unregistered widget IDs

- [ ] **Sense & Hit Testing** (tests in `sense/tests.rs` and `input/tests.rs`):
  - `Sense::none()` widgets skipped in hit test
  - interact_radius extends hit area for small widgets
  - `HitTestBehavior::Opaque` blocks behind
  - `HitTestBehavior::DeferToChild` passes through
  - `HitTestBehavior::Translucent` includes both parent and child in path
  - Disabled widget (`disabled: true` on `LayoutNode`) skipped in hit test
  - Clipping container (`clip: true`) prevents hit testing children outside clip bounds
  - `WidgetHitTestResult` path is root-to-leaf order (matches `update_hot_path`)

- [ ] **Event Propagation** (tests in `input/dispatch/tests.rs`):
  - Capture phase: parent intercepts before child
  - Bubble phase: child handles before parent
  - `set_handled()` stops propagation
  - Active widget captures mouse events
  - Keyboard events route to focused widget
  - `Scroll` events use normal hit testing even during active capture
  - Cursor-left clears all hot state and emits `HotChanged(false)` for all
  - `HotChanged` lifecycle events delivered before `MouseMove` dispatch
  - Keyboard routes through `focus_ancestor_path` (capture/bubble through ancestors)

- [ ] **Event Controllers** (tests in per-controller `tests.rs` files:
  `controllers/hover/tests.rs`, `controllers/click/tests.rs`, etc.):
  - HoverController: enter/leave on hot change
  - ClickController: single, double, triple click
  - ClickController: click cancelled by drag threshold
  - DragController: threshold, start/update/end
  - ScrollController: line-to-pixel conversion
  - FocusController: tab navigation order
  - KeyActivationController: Enter/Space key activation
  - ScrubController: horizontal drag value scrubbing
  - SliderKeyController: arrow key slider adjustment
  - TextEditController: keyboard text input handling
  - DropdownKeyController: arrow key dropdown navigation
  - MenuKeyController: arrow key menu navigation
  - Controller composition: Hover + Click + Focus on same widget
  - Click/Drag composition: MouseDown -> large move -> MouseUp produces DragEnd, NOT Clicked
  - DragController reset on `WidgetDisabled` mid-drag clears active capture
  - FocusController `KeyUp(Tab)` consumed (prevents bubble to parent)
  - Phase filtering: Capture-phase controller only invoked during Capture phase

- [ ] **Animation Engine** (tests in `animation/tests.rs`, `animation/property/tests.rs`,
  `animation/spring/tests.rs`, `animation/scheduler/tests.rs`):
  - AnimFrame request/delivery cycle
  - AnimProperty with Behavior auto-animates
  - AnimProperty without Behavior changes instantly
  - Transaction overrides property behavior
  - Spring physics: critically damped convergence
  - Spring physics: underdamped overshoot
  - RenderScheduler sleeps when idle
  - request_repaint_after wakes at correct time
  - `AnimProperty::set_immediate()` bypasses behavior even when behavior is set
  - `AnimProperty::tick()` is no-op for easing-based transitions (only advances springs)
  - Smooth interruption: set during active animation starts from current interpolated value
  - Transaction nesting: inner transaction overrides outer
  - `with_transaction(Transaction::instant())` during initial construction prevents animate-from-zero
  - `RenderScheduler::remove_widget()` clears all pending requests for that widget
  - `RenderScheduler::promote_deferred()` moves entries with `wake_at <= now` to paint requests
  - `compute_control_flow()` integrates `scheduler_wake` with cursor blink timing

- [ ] **Visual State Manager** (tests in `visual_state/tests.rs`,
  `visual_state/resolver/tests.rs`, `visual_state/transition/tests.rs`):
  - State resolution from interaction state
  - State transition triggers animation
  - Multiple state groups compose (CommonStates + FocusStates)
  - Default transition (100ms EaseOut)
  - Custom transition per state pair
  - Rapid state changes (Normal -> Hovered -> Pressed within 50ms) interrupt mid-flight
  - `find_transition()` exact match, then wildcard, then default fallback
  - Newly created animator returns correct initial values without calling `update()` first
  - Spring-based state transition converges after repeated `tick()` calls
  - `get_fg_color()` returns `Color::TRANSPARENT` when no group sets FgColor

- [ ] **Layout** (tests in `layout/tests.rs`, `widgets/rich_label/tests.rs`):
  - Grid layout: Fixed columns
  - Grid layout: AutoFill with various widths
  - Grid layout: wrap to multiple rows
  - RichLabel: multi-span measurement
  - RichLabel: multi-span rendering
  - Grid layout: 0 children produces height = 0
  - Grid layout: `min_width > available_width` produces 1 column
  - Grid layout: row_gap and column_gap spacing correct
  - `walk_invariant()` in `layout/tests.rs` handles both `BoxContent::Flex` and `BoxContent::Grid`
  - `LayoutNode` new fields (sense, hit_test_behavior, clip, disabled, interact_radius) propagated by solver

- [ ] **Widget Trait (Section 08)** (tests in per-widget `tests.rs` files):
  - `paint()` replaces `draw()` for all 37 widget implementations (35 in oriterm_ui + 2 in oriterm)
  - `sense()` override on every widget returns the correct value (not the default)
  - `controllers()` / `controllers_mut()` return correct controllers per widget
  - `visual_states()` / `visual_states_mut()` return animators for interactive widgets
  - `lifecycle()` receives `HotChanged`, `FocusChanged`, `ActiveChanged` events
  - `anim_frame()` advances `VisualStateAnimator` springs and requests continued frames
  - `reset_scroll()` on `ScrollWidget` zeros both offsets; default no-op on other widgets
  - `accept_action()` propagation: container -> children for `Selected`, `ValueChanged`, etc.
  - `for_each_child_mut()` visits all children (needed for widget registration and focus)
  - `on_input()` handles widget-specific input (sidebar arrow keys, number input arrows, etc.)
  - `is_focusable()` derived from `sense().has_focus()` for all widgets
  - `focusable_children()` returns correct IDs for containers with focusable children

- [ ] **Widgets (New — Section 09)** (tests in per-widget `tests.rs` files:
  `sidebar_nav/tests.rs`, `page_container/tests.rs`, `scheme_card/tests.rs`, etc.):
  - Each new widget renders without panic
  - Each new widget responds to hover/click correctly
  - SidebarNavWidget: layout width fixed at 200px, nav item count matches, active index tracking
  - SidebarNavWidget: ArrowUp/ArrowDown/Home/End key navigation emits `Selected`
  - PageContainerWidget: page switching via `accept_action(Selected)`, only active page laid out
  - PageContainerWidget: `accept_action(Selected)` calls `reset_scroll()` on newly-active page
  - SettingRowWidget: layout height >= 44px, hover state, child control delegation
  - SchemeCardWidget: swatch bar renders 8 colors, click emits `Selected`, selection state
  - ColorSwatchGrid: 8-column grid, click emits `Selected` with correct index
  - CodePreviewWidget: `sense()` is `Sense::none()`, produces RichLabel spans
  - CursorPickerWidget: 3 cards, click emits `Selected` with correct index (0=Block, 1=Bar, 2=Underline)
  - KeybindRow: badge renders key text, row layout with badges, hover state
  - NumberInput: min/max clamping, step increment, arrow key behavior, `ValueChanged` emission
  - SliderWidget: drag updates value, value clamping, `ValueChanged` emission

- [ ] **Widgets (Migrated — Section 08)** (tests in per-widget `tests.rs` files;
  `TerminalGridWidget` and `TerminalPreviewWidget` tests in `oriterm/src/widgets/`):
  - Each migrated widget has no behavioral regression
  - ButtonWidget: click emits `Clicked`, hover transition animates via `VisualStateAnimator`
  - ToggleWidget: click toggles state, thumb slides via `AnimProperty`, emits `Toggled`
  - DropdownWidget: click opens overlay, arrow keys navigate, Enter selects
  - CheckboxWidget: click toggles, visual check mark updates
  - SliderWidget: drag updates value, arrow keys adjust (via `SliderKeyController`)
  - TextInputWidget: keyboard input via `TextEditController`, emits `TextChanged`
  - ScrollWidget: scroll events handled, scrollbar thumb draggable, `reset_scroll()` works
  - ContainerWidget: children traversal, event dispatch delegation, grid mode support
  - WindowChromeWidget: minimize/maximize/close buttons functional
  - TabBarWidget: tab click, close button, drag reorder
  - TerminalGridWidget (oriterm crate): input via `TerminalInputController`, focus, selection
  - TerminalPreviewWidget (oriterm crate): `sense()` is `Sense::none()`, no input handling

- [ ] **Settings Panel (Section 10)** (tests in `form_builder/tests.rs`,
  `action_handler/tests.rs`, `settings_panel/tests.rs`, `oriterm/src/config/tests.rs`):
  - `build_settings_dialog()` produces sidebar + pages layout with 8 pages
  - `SettingsIds` has all 22 fixed fields + `scheme_card_ids: Vec<WidgetId>`
  - `WidgetId::placeholder()` returns `WidgetId(0)`, never matches real IDs
  - Each page builder wraps content in `ScrollWidget::vertical()` with `SizeSpec::Fill`
  - `WidgetAction::ValueChanged` routes correctly through both overlay and dialog paths
  - `WidgetAction::TextChanged` routes correctly through both overlay and dialog paths
  - `WidgetAction::ResetDefaults` resets all config fields to `Config::default()`
  - `WidgetAction::Selected` routes to sidebar page switching and scheme card selection
  - Dirty state comparison: `pending_config != original_config` after mutation
  - Window title updates to "Settings \u{2022}" when dirty, "Settings" when clean
  - Config `PartialEq` derive works correctly (including `f32` fields in `PaneConfig`)
  - All new config types (`TabBarPosition`, `GpuBackend`, `RenderingConfig`) serialize/deserialize
  - TOML round-trip tests for every new config field
  - Both `SettingsPanel::new()` (overlay) and `SettingsPanel::embedded()` (dialog) produce valid layouts
  - Footer buttons: Reset to Defaults, Cancel, Save all dispatch correctly
  - Scroll position resets to top when switching pages via `reset_scroll()`

- [ ] **Framework Orchestration (Section 08.1a)** (tests in
  `oriterm/src/app/widget_pipeline/tests.rs`):
  - `prepare_widget_frame()` drains lifecycle events from `InteractionManager`
  - Lifecycle events dispatched to controllers then to `widget.lifecycle()`
  - `anim_frame()` called only for widgets that requested it via `RenderScheduler`
  - `VisualStateAnimator::update()` + `tick()` called during `prepare_widget_frame()`,
    before `paint()` (rendering discipline: mutation in event phase, pure read in render phase)
  - `DispatchResult` merges handled state and requests from multiple controllers
  - OverlayManager integration: overlays participate in capture/bubble pipeline.
    Cross-module test — use mock widgets in `input/dispatch/tests.rs` or
    `overlay/manager/tests.rs`. Do not depend on `App` or platform resources.
  - Modal overlay semantics: click-outside dismiss still works with new pipeline

---

## 11.2 Visual Verification

- [ ] Launch settings dialog, verify visual match against mockup
- [ ] Verify at 100% DPI (1.0 scale factor)
- [ ] Verify at 150% DPI (1.5 scale factor)
- [ ] Verify at 200% DPI (2.0 scale factor)
- [ ] Theme colors match mockup CSS variables
- [ ] Hover transitions are smooth (no flickering, no missed leave events)
- [ ] Toggle thumb slides smoothly
- [ ] Scheme card selection highlights correctly
- [ ] Sidebar active indicator updates on click
- [ ] Scroll container clips correctly with scrollbar
- [ ] Font preview text renders with correct syntax colors
- [ ] Cursor picker shows correct cursor demos
- [ ] Both rendering modes verified: overlay (modal in terminal window) AND dialog (separate window)
- [ ] Dialog opens at 860x620, sidebar fixed at 200px width
- [ ] Page switching has no perceptible flicker (content swaps within 1 frame)
- [ ] Scroll position resets to top when switching between pages
- [ ] Colors page scrolls smoothly with scrollbar visible when SchemeCards overflow
- [ ] SettingRow hover background fades in/out (100ms EaseOut via VisualStateAnimator)
- [ ] NumberInput arrow key increment/decrement updates displayed value immediately
- [ ] KeybindRow KbdBadge has correct keycap depth visual (thicker bottom border)
- [ ] Footer separator line visible between content and buttons
- [ ] "Reset to Defaults" / "Cancel" use ghost button style, "Save" uses primary/accent style
- [ ] Dirty state indicator: window title shows bullet when settings modified
- [ ] All 9 new UiTheme tokens produce visually correct dark AND light theme colors
- [ ] RichLabel spans render at correct x-offsets with per-span colors (no overlap/gap)

---

## 11.3 Performance Validation

### 11.3a Automatable Performance Tests (unit tests in module tests.rs files)

State-based assertions, not timing-based. Safe for CI.

- [ ] **RenderScheduler idle state** (test in `animation/scheduler/tests.rs`): After all
  animations complete, `RenderScheduler::has_pending_work()` returns false. Verify no
  lingering entries from expired requests after `promote_deferred()`.
- [ ] **VisualStateAnimator cleanup** (test in `visual_state/transition/tests.rs`): After hover
  transition completes (advance time past duration), `animator.is_animating(now)` returns
  false. Verify the animator does not request continued anim frames.
- [ ] **Spring convergence** (test in `animation/spring/tests.rs`): Spring-based animations
  converge after repeated `tick()` calls. After convergence, `is_animating()` returns false
  and value equals target within epsilon.
- [ ] **Widget deregistration** (test in `interaction/tests.rs`): After `deregister_widget()`
  for all settings widgets, `InteractionManager` internal maps have zero stale entries.
  `RenderScheduler` has no pending requests for those widget IDs.
- [ ] **InteractionManager algorithmic complexity** (test in `interaction/tests.rs`):
  `update_hot_path()` with 3-5 level widget tree performs O(depth) HashMap operations,
  not O(total_widgets). Verify by asserting on the number of state entries accessed, not
  wall-clock time.
- [ ] **compute_control_flow integration** (test in `oriterm/src/app/event_loop_helpers/tests.rs`):
  When `scheduler_wake` is `None` and no animations are active, `compute_control_flow()`
  returns `ControlFlow::Wait`. When `scheduler_wake` is `Some(instant)`, returns
  `ControlFlow::WaitUntil(instant)` correctly merged with cursor blink timing.

### 11.3b Manual Performance Verification (not automatable -- requires running binary)

These require a running GUI binary and OS-level measurement tools.

- [ ] **Idle CPU:** With settings dialog open and pointer stationary, CPU usage
  must be zero beyond cursor blink timer (~1.89 Hz). No animation loops running
  when no animations are active. **Measure with `top`/Task Manager.**
- [ ] **Hover responsiveness:** Moving pointer across setting rows should feel
  instant. Target: hover state change within 1 frame (< 16.6ms at 60fps).
  **Manual visual check.**
- [ ] **Page switching:** Clicking a sidebar nav item should switch pages within
  1 frame. No perceptible delay. **Manual visual check.**
- [ ] **Animation smoothness:** Toggle thumb slide and hover fade should be
  visually smooth at 60fps. No dropped frames during animation. **Manual visual check.**
- [ ] **Memory:** Opening/closing settings dialog repeatedly should not leak memory.
  Measure RSS before and after 10 open/close cycles. **Measure with `ps` or Task Manager.**
- [ ] **Frame time:** With settings dialog open and pointer moving, frame time
  should stay under 8ms (2x headroom at 60fps). **Measure with frame time logging.**

---

## 11.4 Cross-Platform

- [ ] **Windows (primary dev target):** `cargo build --target x86_64-pc-windows-gnu` succeeds
  (debug and release)
- [ ] **Linux (host):** `cargo build` succeeds; `cargo test` passes (all unit tests)
- [ ] **macOS:** `cargo build --target x86_64-apple-darwin` succeeds (CI or cross-compile).
  If cross-compilation is not available, verify with `cargo check --target x86_64-apple-darwin`
  that no platform-conditional compilation errors exist.
- [ ] **All three scripts pass on host:** `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh`
- [ ] No platform-specific code in the new framework modules (`interaction/`, `sense/`,
  `controllers/`, `visual_state/`, `animation/` new files, `hit_test_behavior.rs`, `action.rs`)
- [ ] No new `#[cfg(target_os)]` in oriterm_ui (existing platform code in `lib.rs`,
  `window/mod.rs`, `tab_bar/` is not affected by this plan)
- [ ] TerminalGridWidget and TerminalPreviewWidget (in `oriterm/src/widgets/`) compile and
  function correctly with the new Widget trait
- [ ] No platform-specific code in `oriterm/src/app/widget_pipeline/` (new module)
- [ ] No platform-specific code in `oriterm/src/app/settings_overlay/form_builder/` page builders
- [ ] `thread_local!` in `transaction.rs` is compatible with all three platforms (verified by CI)
- [ ] `Instant` usage in controllers and animation engine is platform-compatible
  (no platform-specific clock APIs)

---

## 11.5 Documentation

- [ ] Module-level `//!` docs on all new modules:
  `interaction/`, `sense/`, `controllers/`, `visual_state/`, `animation/` (new submodules)
- [ ] `///` docs on all public types and methods
- [ ] Update CLAUDE.md Key Paths if new module structure is significant
- [ ] Update CLAUDE.md Architecture notes if framework architecture changes
- [ ] Plan file `gui-framework-research.md` kept as reference (not deleted)
- [ ] No new source file exceeds 500 lines (code-hygiene.md rule)
- [ ] **Known risk files** that may approach 500 lines during implementation
  (re-verify line counts during section 11 execution since prior sections may have
  altered them; baselines below are from sections 01-10 completion):
  - `widgets/mod.rs` (baseline 261 lines; context types already extracted to `contexts.rs` at 271 lines)
  - `layout/solver.rs` (baseline 462 lines, grid solving extracted to `grid_solver.rs`)
  - `animation/mod.rs` (baseline 388 lines, `AnimProperty` extracted to `property/` submodule)
  - `oriterm/src/app/settings_overlay/form_builder/mod.rs` (baseline 199 lines, page builders extracted to submodules)
  - `oriterm/src/app/widget_pipeline/mod.rs` (baseline 290 lines)
  - **Verification step:** Run `find oriterm_ui/src oriterm/src -name '*.rs' ! -name 'tests.rs' | xargs wc -l | awk '$1 > 500'`
    to find any file exceeding 500 lines. Fix before marking complete.
- [ ] All new public items have doc comments (`///`)
- [ ] All new modules have `//!` module docs
- [ ] No `unwrap()` in new library code
- [ ] No `println!` debugging — use `log` macros
- [ ] All new `#[cfg(test)] mod tests;` entries follow test-organization.md rules
- [ ] `//!` docs on `oriterm_ui/src/hit_test_behavior.rs` (new leaf module from Section 02)
- [ ] `//!` docs on `oriterm_ui/src/action.rs` (relocated from `widgets/mod.rs` in Section 04)
- [ ] `//!` docs on `oriterm_ui/src/widgets/contexts.rs` (extracted in Section 08.0)
- [ ] `//!` docs on all new widget modules: `sidebar_nav/`, `page_container/`, `setting_row/`,
  `scheme_card/`, `color_swatch/`, `code_preview/`, `cursor_picker/`, `keybind/`,
  `number_input/` (Section 09)
- [ ] `//!` docs on all new controller modules: `text_edit/`, `dropdown_key/`, `menu_key/`,
  `key_activation/`, `scrub/`, `slider_key/` (Sections 04 and 08.1b)
- [ ] `//!` docs on `oriterm/src/app/widget_pipeline/mod.rs` (new module from Section 08.1a)
- [ ] `//!` docs on `oriterm/src/widgets/terminal_grid/input_controller.rs` (new file from Section 08.1b)
- [ ] `///` docs on `WidgetId::placeholder()` explaining it returns `WidgetId(0)` and never
  matches real IDs
- [ ] `///` docs on all `WidgetAction` variants including new ones: `DoubleClicked`,
  `TripleClicked`, `DragStart`, `DragUpdate`, `DragEnd`, `ScrollBy`, `ResetDefaults`
- [ ] `///` docs on `SettingsIds` struct and all 22 fields
- [ ] `///` docs on new config types: `TabBarPosition`, `GpuBackend`, `RenderingConfig`
- [ ] `///` docs on `ControllerCtx.bounds` noting it may be `Rect::default()` during capture
- [ ] All `from_theme()` methods on updated style types have doc comments explaining
  which UiTheme tokens they consume

---

## 11.6 Completion Checklist

- [ ] Test matrix: all categories have passing tests
- [ ] Visual verification: settings dialog matches mockup at 100% and 150% DPI
- [ ] Performance: zero idle CPU, < 8ms frame time, no memory leaks
- [ ] Cross-platform: builds on all three targets
- [ ] Documentation: all new public APIs documented
- [ ] `./test-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./build-all.sh` green
- [ ] No regressions in terminal functionality (grid rendering, scrollback,
  selection, search, tab bar, split panes)
- [ ] Performance invariants from CLAUDE.md verified:
  - Zero idle CPU beyond cursor blink (no spurious animation loops)
  - Zero allocations in hot render path (new framework types don't allocate per-frame)
  - `InteractionManager` HashMap lookups are O(1), not O(n)
  - `RenderScheduler` HashSet operations are O(1)
- [ ] **Sync point verification — all cross-section types consistent:**
  - `WidgetAction` enum: all new variants (`DoubleClicked`, `TripleClicked`, `DragStart`,
    `DragUpdate`, `DragEnd`, `ScrollBy`, `ResetDefaults`, `ValueChanged`, `TextChanged`)
    handled or wildcarded in every match site across both crates
  - `IconId` enum: all 8 new variants (`Sun`, `Palette`, `Type`, `Terminal`, `Keyboard`,
    `Window`, `Bell`, `Activity`) have SVG path definitions and are in `ALL_ICONS` test array
  - `BoxContent::Grid` handled in `solver.rs` match and `walk_invariant()` in `layout/tests.rs`
  - `Widget` trait: all 37 implementations (35 in oriterm_ui + 2 in oriterm) provide
    explicit `sense()` override (no widget relies on the `Sense::all()` default after migration)
  - `EventResponse` type: fully removed, zero remaining references (grep verification)
  - `WidgetResponse` type: fully removed, zero remaining references (grep verification)
  - `CaptureRequest` type: fully removed, zero remaining references (grep verification)
  - `ContainerInputState` type: fully removed, zero remaining references
  - `InputState` struct and `RouteAction` enum: fully removed (`routing.rs` deleted)
  - `AnimatedValue<T>`: removed or deprecated with zero active usages
  - Legacy `handle_mouse()`, `handle_hover()`, `handle_key()` fully removed from Widget trait
    (grep verification: zero references outside `_old/`)
- [ ] **Module declarations verified** (all present in `lib.rs` / parent `mod.rs`):
  - `oriterm_ui/src/lib.rs`: `interaction`, `sense`, `hit_test_behavior`, `controllers`,
    `visual_state`, `action` modules declared
  - `oriterm_ui/src/widgets/mod.rs`: `contexts`, `rich_label`, `sidebar_nav`,
    `page_container`, `setting_row`, `scheme_card`, `color_swatch`, `code_preview`,
    `cursor_picker`, `keybind`, `number_input` modules declared
  - `oriterm_ui/src/controllers/mod.rs`: `hover`, `click`, `drag`, `scroll`, `focus`,
    `text_edit`, `dropdown_key`, `menu_key`, `key_activation`, `scrub`, `slider_key`
    modules declared
  - `oriterm_ui/src/animation/mod.rs`: `anim_frame`, `behavior`, `property`, `spring`,
    `transaction`, `scheduler` submodules declared
  - `oriterm/src/widgets/terminal_grid/mod.rs`: `input_controller` module declared
- [ ] **Test file organization** verified (per test-organization.md):
  - Every directory module with tests has a sibling `tests.rs` file
  - No inline `mod tests { ... }` with braces in any source file
  - All `tests.rs` files use `super::` imports, no `mod tests { }` wrapper
- [ ] **File size compliance** — no source file (excluding `tests.rs`) exceeds 500 lines:
  - `widgets/mod.rs` under 300 lines (baseline: 261, contexts extracted to 271-line contexts.rs)
  - `widgets/contexts.rs` under 500 lines (baseline: 271)
  - `layout/solver.rs` under 500 lines (baseline: 462, grid solver extracted)
  - `container/mod.rs` under 500 lines (baseline: 391, layout_build extracted)
  - `scroll/mod.rs` under 500 lines (baseline: 441, rendering + event_handling extracted)
  - `dialog/mod.rs` under 500 lines (baseline: 343, event_handling extracted)
  - `settings_panel/mod.rs` under 500 lines (baseline: 399, event_handling extracted)
  - `tab_bar/widget/mod.rs` under 500 lines (baseline: 498 -- **WARNING: 2 lines headroom,
    do NOT add code to this file without splitting first**)
  - `content_actions.rs` under 500 lines (baseline: 458, key_conversion extracted)
  - `widget_pipeline/mod.rs` under 500 lines (baseline: 290)
  - `interaction/manager.rs` under 500 lines (baseline: 493 -- **WARNING: 7 lines headroom**)
  - Each page builder submodule under 500 lines
  - **Run the verification command from 11.5 to catch any file missed by this list**
- [ ] **No dead code**: `#[allow(dead_code)]` only on `open_settings_overlay()` (retained as
  fallback). No other new code has `allow(dead_code)`.
- [ ] **Settings overlay and dialog paths both tested**: action dispatch verified for
  `try_dispatch_settings_action()` (overlay) and `dispatch_dialog_settings_action()` (dialog)
- [ ] **Backward compatibility**: existing config TOML files without new fields load correctly
  (all new fields have `#[serde(default)]`)

**Exit Criteria:** All tests pass, all three platform builds succeed, settings dialog
visually matches the mockup, idle CPU is zero, and no terminal functionality regresses.
The UI framework is general-purpose and ready for future consumers beyond the settings panel.
All legacy event types (`EventResponse`, `WidgetResponse`, `CaptureRequest`, `InputState`,
`ContainerInputState`) are fully removed. Every new module has `//!` docs, every new public
type has `///` docs. No source file exceeds 500 lines.
