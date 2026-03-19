---
section: "09"
title: "Architectural Boundary Enforcement"
status: not-started
reviewed: false
goal: "Establish permanent guardrails that prevent pure UI logic from accumulating in oriterm, ensure oriterm stays a thin shell, and codify the crate responsibility boundaries."
depends_on: ["07", "08"]
sections:
  - id: "09.1"
    title: "Crate Responsibility Rules"
    status: not-started
  - id: "09.2"
    title: "Architectural Tests"
    status: not-started
  - id: "09.3"
    title: "Documentation Updates"
    status: not-started
  - id: "09.4"
    title: "Completion Checklist"
    status: not-started
---

# Section 09: Architectural Boundary Enforcement

**Status:** Not Started
**Goal:** After Sections 07-08 establish the correct architecture (WindowRoot in oriterm_ui, pure logic migrated), this section adds permanent guardrails that prevent regression. Without enforcement, new code will gradually accumulate in `oriterm` that should be in `oriterm_ui` — the same drift that created the current problem.

**Context:** The boundary between `oriterm_ui` (UI framework) and `oriterm` (application shell) has drifted over time. Pure UI logic (cursor blink, hit testing, drag states, menu state) accumulated in `oriterm` because that's where the code was being written during feature development. Sections 07-08 fix the current state; this section prevents future drift.

**Depends on:** Sections 07 and 08 (boundaries must be correct before enforcing them).

---

## 09.1 Crate Responsibility Rules

**File(s):** `.claude/rules/crate-boundaries.md` (NEW)

Codify what each crate owns and what it must NOT contain. These rules become part of the hygiene review process.

- [ ] Create `.claude/rules/crate-boundaries.md` with clear ownership rules:

  **`oriterm_ui` (UI framework) owns:**
  - Widget trait and all widget implementations
  - WindowRoot (per-window composition unit)
  - InteractionManager, FocusManager, OverlayManager
  - Layout engine, hit testing, event propagation
  - Controllers (hover, click, drag, focus, key activation)
  - Animation engine (VisualStateAnimator, RenderScheduler, CursorBlink)
  - Compositor (LayerTree, LayerAnimator)
  - Scene caching, invalidation tracking
  - Pure interaction utilities (resize geometry, cursor hiding, and drag state machines where `oriterm_mux` dependencies allow)
  - Action types and dispatch infrastructure
  - Theme types (UiTheme, color tokens)
  - Test harness (WidgetTestHarness wrapping WindowRoot)
  - Pipeline orchestration (layout → prepaint → paint → dispatch)

  **`oriterm_ui` must NOT contain:**
  - GPU types (wgpu::Device, wgpu::Surface, shader pipelines)
  - Platform window management types (winit::Window instances, TermWindow). Note: `oriterm_ui` already depends on `winit` for keyboard types but must not own `winit::Window` instances or manage window lifecycle.
  - Terminal types (Grid, Cell, PTY, VTE, Selection beyond basic geometry)
  - Mux types (PaneId, MuxBackend, domain management)
  - Font rasterization (swash, skrifa, glyph atlas)
  - Configuration (Config struct, TOML parsing, file watching)

  **`oriterm` (application shell) owns:**
  - winit event loop and window lifecycle
  - GPU initialization and rendering (wgpu, shader pipelines)
  - Window ↔ WindowRoot mapping (`HashMap<WindowId, WindowContext>`)
  - Terminal-specific interactions (selection, mouse reporting, PTY encoding)
  - Session model (tabs, split trees, floating panes, navigation)
  - Configuration loading and hot-reload
  - Clipboard integration
  - Mux integration (pane CRUD, event pump)
  - Platform chrome (title bar, resize handles)
  - Font pipeline (rasterization, atlas, shaping cache)

  **`oriterm` must NOT contain:**
  - Widget definitions (use `oriterm_ui::widgets`)
  - Pure interaction logic (use `oriterm_ui::interaction`)
  - Framework state management (use `WindowRoot`)
  - Pipeline orchestration (use `WindowRoot` methods)
  - Duplicate type definitions of anything in `oriterm_ui`

- [ ] Add a litmus test to the rules file:
  > **Litmus test:** Can this code be tested in a `#[test]` without a GPU, display server, or terminal? If yes → it belongs in `oriterm_ui`. If no → it belongs in `oriterm`.

---

## 09.2 Architectural Tests

**File(s):** `oriterm/tests/architecture.rs` (NEW integration test)

Compile-time and runtime checks that enforce crate boundaries.

- [ ] Add `oriterm_ui = { path = "../oriterm_ui", features = ["testing"] }` to `oriterm/Cargo.toml` `[dev-dependencies]` to enable the `testing` module in integration tests.

- [ ] Create `oriterm/tests/architecture.rs` with boundary validation tests:

  ```rust
  //! Architectural boundary tests.
  //!
  //! These tests verify that the crate responsibility boundaries are maintained.
  //! If a test fails, it means code has drifted into the wrong crate.

  /// WindowRoot must be constructable without GPU or platform dependencies.
  #[test]
  fn window_root_is_headless() {
      use oriterm_ui::widgets::button::ButtonWidget;
      use oriterm_ui::window_root::WindowRoot;

      // This compiles and runs = WindowRoot has no GPU/platform deps
      let _root = WindowRoot::new(ButtonWidget::new("test"));
  }

  /// WidgetTestHarness wraps WindowRoot (not raw fields).
  #[test]
  fn harness_uses_window_root() {
      use oriterm_ui::testing::WidgetTestHarness;
      use oriterm_ui::widgets::button::ButtonWidget;

      let harness = WidgetTestHarness::new(ButtonWidget::new("test"));
      // Verify we can access the WindowRoot through the harness
      let _root = harness.root();
  }
  ```

- [ ] Add a test that verifies full event propagation through nested containers:
  ```rust
  /// Events propagate through WindowRoot → Dialog → Panel → Button.
  #[test]
  fn event_propagation_through_window_root() {
      // Build hierarchy: WindowRoot wrapping a nested container
      // Dispatch click at button position
      // Assert button's action fires
      // This proves the full pipeline works headlessly
  }
  ```

- [ ] Add a test that verifies overlay event routing through WindowRoot:
  ```rust
  /// Overlay events take priority over widget tree events.
  #[test]
  fn overlay_event_priority_through_window_root() {
      // Build WindowRoot with a button
      // Push a popup overlay covering the button
      // Dispatch click at button position
      // Assert overlay handles it (button does NOT receive click)
      // This proves overlay routing works headlessly
  }
  ```

- [ ] Add a test that verifies interaction state (hover/active/focus) propagation:
  ```rust
  /// InteractionManager state updates through WindowRoot.
  #[test]
  fn interaction_state_through_window_root() {
      // Build WindowRoot with a focusable button
      // Dispatch mouse move over button
      // Assert button is hot
      // Dispatch mouse down
      // Assert button is active
      // Tab to next widget
      // Assert focus moved
  }
  ```

- [ ] Consider a `cargo test` wrapper script or CI check that greps `oriterm/src/app/` for patterns that indicate pure UI logic:
  - `struct.*State` definitions that have no GPU/platform/terminal fields
  - `fn.*hit_test` functions that don't reference app state
  - `enum` definitions that duplicate types in `oriterm_ui`

- [ ] Add a crate dependency direction validation test:
  ```rust
  /// oriterm_ui must NOT depend on oriterm or oriterm_mux.
  ///
  /// This test validates the crate dependency direction by checking
  /// that oriterm_ui's Cargo.toml does not list these as dependencies.
  #[test]
  fn oriterm_ui_has_no_upstream_deps() {
      let cargo_toml = std::fs::read_to_string(
          concat!(env!("CARGO_MANIFEST_DIR"), "/../oriterm_ui/Cargo.toml")
      ).unwrap();
      assert!(
          !cargo_toml.contains("oriterm_mux"),
          "oriterm_ui must not depend on oriterm_mux"
      );
      // oriterm_ui should not have a dep named exactly "oriterm"
      // (careful: "oriterm_core" and "oriterm_ui" are fine)
      for line in cargo_toml.lines() {
          let trimmed = line.trim();
          if trimmed.starts_with("oriterm") && !trimmed.starts_with("oriterm_") {
              panic!("oriterm_ui must not depend on oriterm (found: {trimmed})");
          }
      }
  }
  ```

---

## 09.3 Documentation Updates

**File(s):** `CLAUDE.md`, `.claude/rules/impl-hygiene.md`

- [ ] Update `CLAUDE.md` Key Paths section to reflect the new architecture:
  - Add `oriterm_ui/src/window_root/` — WindowRoot (per-window UI composition unit)
  - Add `oriterm_ui/src/interaction/` — Pure interaction utilities (resize, drag, cursor, mark mode)
  - Update the description of `oriterm/src/app/` to emphasize it's a thin shell

- [ ] Add a "Crate Boundaries" section to `CLAUDE.md`:
  - One-line description of each crate's responsibility
  - The litmus test for where code belongs
  - Cross-reference to `.claude/rules/crate-boundaries.md` for full rules

- [ ] Update `.claude/rules/impl-hygiene.md` to reference the new crate boundary rules:
  - Add a bullet under "Module Boundary Discipline" about crate-level boundaries
  - Reference the litmus test

- [ ] Update `CLAUDE.md` "UI Framework — Zero Exceptions Rule" to mention WindowRoot:
  - WindowRoot is the per-window composition unit
  - Both test harness and production windows use it
  - No framework state should be owned outside WindowRoot

---

## 09.4 Completion Checklist

- [ ] `oriterm/Cargo.toml` `[dev-dependencies]` includes `oriterm_ui` with `features = ["testing"]`
- [ ] `.claude/rules/crate-boundaries.md` exists with clear ownership rules
- [ ] Litmus test documented: "Can it run in a #[test] without GPU/platform/terminal?"
- [ ] `oriterm/tests/architecture.rs` exists with boundary validation tests
- [ ] WindowRoot headless construction test passes
- [ ] Event propagation through nested containers test passes
- [ ] Overlay event routing through WindowRoot test passes
- [ ] Interaction state propagation through WindowRoot test passes
- [ ] Crate dependency direction test passes (oriterm_ui has no oriterm/oriterm_mux deps)
- [ ] `CLAUDE.md` Key Paths updated with new architecture
- [ ] `CLAUDE.md` Crate Boundaries section added
- [ ] `.claude/rules/impl-hygiene.md` updated
- [ ] `timeout 150 cargo test -p oriterm` passes (architecture tests)
- [ ] `./clippy-all.sh` clean
- [ ] `./build-all.sh` clean

**Exit Criteria:** A developer reading `CLAUDE.md` and `.claude/rules/crate-boundaries.md` can immediately answer "where does this code belong?" for any new feature. Architectural tests in CI catch boundary violations before they merge. The crate dependency direction test prevents `oriterm_ui` from ever depending on `oriterm` or `oriterm_mux`. The litmus test is simple and unambiguous: testable without GPU/platform/terminal → `oriterm_ui`.
