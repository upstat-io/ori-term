---
section: "11"
title: "Verification"
status: not-started
goal: "Comprehensive verification that the framework and settings panel work correctly with no regressions"
depends_on: ["10"]
reviewed: false
sections:
  - id: "11.1"
    title: "Test Matrix"
    status: not-started
  - id: "11.2"
    title: "Visual Verification"
    status: not-started
  - id: "11.3"
    title: "Performance Validation"
    status: not-started
  - id: "11.4"
    title: "Cross-Platform"
    status: not-started
  - id: "11.5"
    title: "Documentation"
    status: not-started
  - id: "11.6"
    title: "Completion Checklist"
    status: not-started
---

# Section 11: Verification

**Status:** Not Started
**Goal:** All framework components and the settings panel are thoroughly tested, visually
verified, performance-validated, and documented. No regressions in terminal functionality.

**Depends on:** Section 10 (Settings Panel — everything must be built).

---

## 11.1 Test Matrix

- [ ] **Interaction State:**
  - Hot path computation with nested widgets
  - Hot path update on pointer move (enter/leave lifecycle events)
  - Active state capture and release
  - Focus request and transfer
  - focus_within propagation to ancestors
  - Disabled widget skipped in hot tracking

- [ ] **Sense & Hit Testing:**
  - `Sense::none()` widgets skipped in hit test
  - interact_radius extends hit area for small widgets
  - `HitTestBehavior::Opaque` blocks behind
  - `HitTestBehavior::DeferToChild` passes through

- [ ] **Event Propagation:**
  - Capture phase: parent intercepts before child
  - Bubble phase: child handles before parent
  - `set_handled()` stops propagation
  - Active widget captures mouse events
  - Keyboard events route to focused widget

- [ ] **Event Controllers:**
  - HoverController: enter/leave on hot change
  - ClickController: single, double, triple click
  - ClickController: click cancelled by drag threshold
  - DragController: threshold, start/update/end
  - ScrollController: line-to-pixel conversion
  - FocusController: tab navigation order
  - Controller composition: Hover + Click + Focus on same widget

- [ ] **Animation Engine:**
  - AnimFrame request/delivery cycle
  - AnimProperty with Behavior auto-animates
  - AnimProperty without Behavior changes instantly
  - Transaction overrides property behavior
  - Spring physics: critically damped convergence
  - Spring physics: underdamped overshoot
  - RenderScheduler sleeps when idle
  - request_repaint_after wakes at correct time

- [ ] **Visual State Manager:**
  - State resolution from interaction state
  - State transition triggers animation
  - Multiple state groups compose (CommonStates + FocusStates)
  - Default transition (100ms EaseOut)
  - Custom transition per state pair

- [ ] **Layout:**
  - Grid layout: Fixed columns
  - Grid layout: AutoFill with various widths
  - Grid layout: wrap to multiple rows
  - RichLabel: multi-span measurement
  - RichLabel: multi-span rendering

- [ ] **Widgets:**
  - Each new widget renders without panic
  - Each new widget responds to hover/click correctly
  - Each migrated widget has no behavioral regression

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

---

## 11.3 Performance Validation

- [ ] **Idle CPU:** With settings dialog open and pointer stationary, CPU usage
  must be zero beyond cursor blink timer (~1.89 Hz). No animation loops running
  when no animations are active.
- [ ] **Hover responsiveness:** Moving pointer across setting rows should feel
  instant. Target: hover state change within 1 frame (< 16.6ms at 60fps).
- [ ] **Page switching:** Clicking a sidebar nav item should switch pages within
  1 frame. No perceptible delay.
- [ ] **Animation smoothness:** Toggle thumb slide and hover fade should be
  visually smooth at 60fps. No dropped frames during animation.
- [ ] **Memory:** Opening/closing settings dialog repeatedly should not leak memory.
  Measure RSS before and after 10 open/close cycles.
- [ ] **Frame time:** With settings dialog open and pointer moving, frame time
  should stay under 8ms (2x headroom at 60fps).

---

## 11.4 Cross-Platform

- [ ] `cargo build --target x86_64-pc-windows-gnu` succeeds
- [ ] `cargo build` (Linux host) succeeds
- [ ] `cargo build --target x86_64-apple-darwin` succeeds (if CI available)
- [ ] No platform-specific code in the new widget library (interaction/, controllers/,
  visual_state/, animation/ new files)
- [ ] No new `#[cfg(target_os)]` in oriterm_ui (existing platform code in `lib.rs`,
  `window/mod.rs`, `tab_bar/` is not affected by this plan)
- [ ] TerminalGridWidget and TerminalPreviewWidget (in `oriterm/src/widgets/`) compile and
  function correctly with the new Widget trait

---

## 11.5 Documentation

- [ ] Module-level `//!` docs on all new modules:
  `interaction/`, `controllers/`, `visual_state/`, `animation/` (new files)
- [ ] `///` docs on all public types and methods
- [ ] Update CLAUDE.md Key Paths if new module structure is significant
- [ ] Update CLAUDE.md Architecture notes if framework architecture changes
- [ ] Plan file `gui-framework-research.md` kept as reference (not deleted)
- [ ] No new source file exceeds 500 lines (code-hygiene.md rule)
- [ ] **Known risk files** that may approach 500 lines during implementation:
  - `widgets/mod.rs` (currently 361 lines, adding ~80-100 lines of context types)
  - `layout/solver.rs` (currently 428 lines, grid solving extracted to `grid_solver.rs`)
  - `animation/mod.rs` (currently 356 lines, `AnimProperty` extracted to `behavior.rs`)
  - `form_builder/mod.rs` (currently 224 lines, page builders extracted to submodules)
- [ ] All new public items have doc comments (`///`)
- [ ] All new modules have `//!` module docs
- [ ] No `unwrap()` in new library code
- [ ] No `println!` debugging — use `log` macros
- [ ] All new `#[cfg(test)] mod tests;` entries follow test-organization.md rules

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

**Exit Criteria:** All tests pass, all three platform builds succeed, settings dialog
visually matches the mockup, idle CPU is zero, and no terminal functionality regresses.
The UI framework is general-purpose and ready for future consumers beyond the settings panel.
