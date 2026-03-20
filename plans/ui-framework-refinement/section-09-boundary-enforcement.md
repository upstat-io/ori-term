---
section: "09"
title: "Architectural Boundary Enforcement"
status: complete
reviewed: true
goal: "Establish permanent guardrails that prevent pure UI logic from accumulating in oriterm, ensure oriterm stays a thin shell, and codify the crate responsibility boundaries."
depends_on: ["07", "08"]
sections:
  - id: "09.1"
    title: "Crate Responsibility Rules"
    status: complete
  - id: "09.2"
    title: "Architectural Tests"
    status: complete
  - id: "09.3"
    title: "Documentation Updates"
    status: complete
  - id: "09.4"
    title: "Completion Checklist"
    status: complete
---

# Section 09: Architectural Boundary Enforcement

**Status:** Complete
**Goal:** After Sections 07-08 establish the correct architecture (WindowRoot in oriterm_ui, pure logic migrated), this section adds permanent guardrails that prevent regression. Without enforcement, new code will gradually accumulate in `oriterm` that should be in `oriterm_ui` — the same drift that created the current problem.

**Context:** The boundary between `oriterm_ui` (UI framework) and `oriterm` (application shell) has drifted over time. Pure UI logic (cursor blink, hit testing, drag states, menu state) accumulated in `oriterm` because that's where the code was being written during feature development. Sections 07-08 fix the current state; this section prevents future drift.

**Depends on:** Sections 07 and 08 (boundaries must be correct before enforcing them).

---

## 09.1 Crate Responsibility Rules

**File(s):** `.claude/rules/crate-boundaries.md` (NEW)

Codify what each crate owns and what it must NOT contain. These rules become part of the hygiene review process.

- [x] Create `.claude/rules/crate-boundaries.md` with YAML frontmatter (`paths: ["**/src/**", "**/Cargo.toml"]`) and clear ownership rules:

  **`oriterm_ui` (UI framework) owns:**
  - Widget trait and all widget implementations
  - WindowRoot (per-window composition unit)
  - InteractionManager, FocusManager, OverlayManager
  - Layout engine, hit testing, event propagation
  - Controllers (hover, click, drag, focus, key activation)
  - Animation engine (VisualStateAnimator, RenderScheduler, CursorBlink)
  - Compositor (LayerTree, LayerAnimator)
  - Scene caching, invalidation tracking
  - Pure interaction utilities (resize geometry, cursor hiding, mark mode motion — NOT drag state machines, which stay in oriterm per Section 08.3)
  - Action types and dispatch infrastructure
  - Theme types (UiTheme, color tokens)
  - Test harness (WidgetTestHarness wrapping WindowRoot)
  - Pipeline orchestration (layout → prepaint → paint → dispatch)

  **`oriterm_ui` depends on:** `oriterm_core` (for `Color` type reuse and terminal-related geometry). Also depends on `winit` (for `WindowConfig` and `create_window()` — window creation config, NOT lifecycle management). No other `oriterm_*` workspace crate dependencies.

  **`oriterm_ui` must NOT contain:**
  - GPU types (wgpu::Device, wgpu::Surface, shader pipelines)
  - Window lifecycle management (event handling, per-window state storage, `TermWindow`). Note: `oriterm_ui` already provides `window::create_window()` (returns `Arc<Window>`) and `WindowConfig` for config-driven window creation, but must not manage window lifecycle (event dispatch, `HashMap<WindowId, WindowContext>` storage).
  - Terminal types (Grid, Cell, PTY, VTE, Selection beyond basic geometry)
  - Mux types (PaneId, MuxBackend, domain management)
  - IPC types (oriterm_ipc transport)
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

  **`oriterm_core` (terminal emulation library) owns:**
  - Grid data structure (rows, columns, cursor, scrollback, reflow)
  - Cell representation (`Cell`, `CellFlags`, hyperlinks)
  - VTE handler (`term_handler.rs` — escape sequence processing)
  - Color palette (`Palette`, ANSI/256/TrueColor mapping)
  - Selection model (rectangular, linear, semantic)
  - Search (plain text + regex)
  - Terminal index types (`Line`, `Column`, `Cursor`)

  **`oriterm_core` must NOT contain:**
  - UI framework types (widgets, layout, interaction, hit testing)
  - GPU types (wgpu, shaders, atlas)
  - PTY/process management (that belongs in `oriterm_mux`)
  - Window or platform types (winit, platform-specific code)
  - Mux types (PaneId, DomainId, ClientId)

  **`oriterm_mux` (pane server) owns:**
  - Pane lifecycle (create, resize, close)
  - PTY I/O (read/write, event pump)
  - PaneRegistry (flat pane storage)
  - MuxBackend trait (embedded + daemon)
  - Daemon server (IPC protocol via `oriterm_ipc`)
  - Wire protocol (PDU codec)
  - ID types: `PaneId`, `DomainId`, `ClientId`

  **`oriterm_mux` must NOT contain:**
  - UI framework types (widgets, layout, interaction)
  - GPU types (wgpu, shaders, rendering)
  - Session model (tabs, windows, layouts — that is `oriterm`'s concern)
  - Window or platform types (winit)

  **`oriterm_ipc` (IPC abstraction) owns:**
  - Platform-specific IPC transport (Unix domain sockets, Windows named pipes)
  - Connection lifecycle (listen, accept, connect)
  - Mio integration for async I/O

  **`oriterm_ipc` must NOT contain:**
  - Protocol semantics (PDU types, serialization — that is `oriterm_mux/protocol`)
  - Any dependency on `oriterm_core`, `oriterm_ui`, or `oriterm`

  **`crates/vte` (vendored VTE parser):**
  - Vendored fork of the `vte` crate. Not modified directly — treat as external dependency.
  - No boundary rules beyond: do not add oriterm-specific types here.

  **Allowed dependency direction:**
  ```
  oriterm_ipc  (standalone — no oriterm_* deps)
  oriterm_core (standalone — no oriterm_* deps)
  oriterm_ui   → oriterm_core
  oriterm_mux  → oriterm_core, oriterm_ipc
  oriterm      → oriterm_core, oriterm_ui, oriterm_mux
  ```

- [x] Add a litmus test to the rules file:
  > **Litmus test:** Can this code be tested in a `#[test]` without a GPU, display server, or terminal? If yes → it belongs in `oriterm_ui`. If no → it belongs in `oriterm`.

---

## 09.2 Architectural Tests

**File(s):** `oriterm/tests/architecture.rs` (NEW integration test)

Compile-time and runtime checks that enforce crate boundaries.

> **Prerequisite:** Complete 09.1 first. The tests codify the rules defined there.

- [x] Add `oriterm_ui = { path = "../oriterm_ui", features = ["testing"] }` to `oriterm/Cargo.toml` `[dev-dependencies]` to enable the `testing` module in integration tests. Note: `oriterm_ui` is already in `[dependencies]` without the `testing` feature; Cargo merges dev-dependency features into the regular dependency when building tests/benches.

- [x] Create `oriterm/tests/architecture.rs` with boundary validation tests:

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

  /// WidgetTestHarness wraps WindowRoot and exposes it.
  #[test]
  fn harness_wraps_window_root() {
      use oriterm_ui::testing::WidgetTestHarness;
      use oriterm_ui::widgets::button::ButtonWidget;

      let harness = WidgetTestHarness::new(ButtonWidget::new("test"));
      // Verify the harness exposes WindowRoot and it has a computed layout.
      let root = harness.root();
      assert!(root.viewport().width() > 0.0, "WindowRoot must have a valid viewport");
  }
  ```

- [x] Add a test that verifies full event propagation through nested containers. This is a pseudo-code sketch — use `WidgetTestHarness` (which wraps `WindowRoot`) rather than constructing `WindowRoot` directly. The harness provides `click(widget_id)`, `mouse_move(point)`, `key_press(key, modifiers)`, `is_hot(id)`, `is_active(id)`, `is_focused(id)`, and `take_actions()`.
  ```rust
  /// Events propagate through WindowRoot via the test harness pipeline.
  #[test]
  fn event_propagation_through_window_root() {
      // Build harness wrapping a container with a nested ButtonWidget.
      // let harness = WidgetTestHarness::new(container_with_button);
      // let button_id = /* ButtonWidget's WidgetId */;
      // let actions = harness.click(button_id);
      // Assert actions contains WidgetAction::Clicked(button_id).
      // This proves the full pipeline works headlessly.
  }
  ```

- [x] Add a test that verifies overlay event routing through WindowRoot:
  ```rust
  /// Overlay events take priority over widget tree events.
  #[test]
  fn overlay_event_priority_through_window_root() {
      // Build harness with a ButtonWidget.
      // let button_id = /* ButtonWidget's WidgetId */;
      // harness.push_popup(overlay_widget, anchor_rect_covering_button);
      // let actions = harness.click(button_id);
      // Assert actions is empty (overlay consumed the click, not the button).
      // This proves overlay priority routing works headlessly.
  }
  ```

- [x] Add a test that verifies interaction state (hover/active/focus) propagation:
  ```rust
  /// InteractionManager state updates through WindowRoot.
  #[test]
  fn interaction_state_through_window_root() {
      // Build harness with a focusable ButtonWidget.
      // let button_id = /* ButtonWidget's WidgetId */;
      // harness.mouse_move(button_center_point);
      // assert!(harness.is_hot(button_id));
      // harness.mouse_down(MouseButton::Left);
      // assert!(harness.is_active(button_id));
      // harness.key_press(Key::Named(NamedKey::Tab), Modifiers::empty());
      // Assert focus moved (is_focused on next widget, not button).
  }
  ```

- [x] Add dependency boundary validation tests using section-aware Cargo.toml parsing. Extract a shared helper `dep_names(cargo_toml: &str) -> Vec<String>` that parses only `[dependencies]`, `[dev-dependencies]`, and `[build-dependencies]` sections, returning dependency crate names. This avoids repeating the parsing logic and prevents false positives from raw `contains()` on comments or package descriptions.

  ```rust
  /// Extracts dependency crate names from a Cargo.toml string.
  ///
  /// Only scans lines inside `[dependencies]`, `[dev-dependencies]`,
  /// `[build-dependencies]`, and their `[target.*.dependencies]` variants.
  /// Ignores `[package]`, `[features]`, `[lints]`, comments, etc.
  fn dep_names(cargo_toml: &str) -> Vec<String> {
      let mut names = Vec::new();
      let mut in_deps = false;
      for line in cargo_toml.lines() {
          let trimmed = line.trim();
          if trimmed.starts_with('[') {
              in_deps = trimmed.contains("dependencies");
              continue;
          }
          if in_deps && !trimmed.is_empty() && !trimmed.starts_with('#') {
              // Dependency lines look like: `crate_name = ...` or `crate_name.workspace = true`
              if let Some(name) = trimmed.split(&['=', '.'][..]).next() {
                  let name = name.trim();
                  if !name.is_empty() {
                      names.push(name.to_string());
                  }
              }
          }
      }
      names
  }
  ```

  ```rust
  /// oriterm_ui must NOT depend on GPU or font rasterization crates.
  ///
  /// GPU rendering and font rasterization belong in oriterm, not in the UI framework.
  #[test]
  fn oriterm_ui_has_no_gpu_or_font_deps() {
      let cargo_toml = std::fs::read_to_string(
          concat!(env!("CARGO_MANIFEST_DIR"), "/../oriterm_ui/Cargo.toml")
      ).unwrap();
      let deps = dep_names(&cargo_toml);
      for forbidden in &["wgpu", "tiny-skia", "swash", "skrifa", "rustybuzz"] {
          assert!(
              !deps.iter().any(|d| d == *forbidden),
              "oriterm_ui must not depend on {forbidden} (GPU/font pipeline belongs in oriterm)"
          );
      }
  }

  /// oriterm_ui must NOT depend on oriterm, oriterm_mux, or oriterm_ipc.
  #[test]
  fn oriterm_ui_has_no_upstream_deps() {
      let cargo_toml = std::fs::read_to_string(
          concat!(env!("CARGO_MANIFEST_DIR"), "/../oriterm_ui/Cargo.toml")
      ).unwrap();
      let deps = dep_names(&cargo_toml);
      for forbidden in &["oriterm", "oriterm_mux", "oriterm_ipc"] {
          assert!(
              !deps.iter().any(|d| d == *forbidden),
              "oriterm_ui must not depend on {forbidden}"
          );
      }
  }

  /// oriterm_core must NOT depend on oriterm_ui, oriterm_mux, oriterm_ipc, or oriterm.
  ///
  /// oriterm_core is the terminal emulation library — it must be usable standalone.
  #[test]
  fn oriterm_core_has_no_upstream_deps() {
      let cargo_toml = std::fs::read_to_string(
          concat!(env!("CARGO_MANIFEST_DIR"), "/../oriterm_core/Cargo.toml")
      ).unwrap();
      let deps = dep_names(&cargo_toml);
      for forbidden in &["oriterm", "oriterm_ui", "oriterm_mux", "oriterm_ipc"] {
          assert!(
              !deps.iter().any(|d| d == *forbidden),
              "oriterm_core must not depend on {forbidden}"
          );
      }
  }

  /// oriterm_mux must NOT depend on oriterm_ui or oriterm.
  ///
  /// It is a pane server that depends only on oriterm_core and oriterm_ipc.
  #[test]
  fn oriterm_mux_has_no_ui_or_app_deps() {
      let cargo_toml = std::fs::read_to_string(
          concat!(env!("CARGO_MANIFEST_DIR"), "/../oriterm_mux/Cargo.toml")
      ).unwrap();
      let deps = dep_names(&cargo_toml);
      for forbidden in &["oriterm", "oriterm_ui"] {
          assert!(
              !deps.iter().any(|d| d == *forbidden),
              "oriterm_mux must not depend on {forbidden}"
          );
      }
  }

  /// oriterm_ipc must NOT depend on any other oriterm_* crate.
  ///
  /// It is a standalone platform IPC abstraction.
  #[test]
  fn oriterm_ipc_is_standalone() {
      let cargo_toml = std::fs::read_to_string(
          concat!(env!("CARGO_MANIFEST_DIR"), "/../oriterm_ipc/Cargo.toml")
      ).unwrap();
      let deps = dep_names(&cargo_toml);
      for dep in &deps {
          assert!(
              !dep.starts_with("oriterm"),
              "oriterm_ipc must not depend on any oriterm crate (found: {dep})"
          );
      }
  }
  ```

- [x] Document in `.claude/rules/crate-boundaries.md` a "code review checklist" section:
  > When reviewing PRs that add code to `oriterm/src/app/`:
  > - Does this struct/function need GPU, platform, or terminal state? If not, it belongs in `oriterm_ui`.
  > - Does this duplicate a type already in `oriterm_ui`? If so, use the existing one.
  > - Could this be tested headlessly? If yes, move it to `oriterm_ui`.

---

## 09.3 Documentation Updates

**File(s):** `CLAUDE.md`, `.claude/rules/impl-hygiene.md`

- [x] Update `CLAUDE.md` Key Paths section to reflect the new architecture:
  - Add `oriterm_ui/src/window_root/` — WindowRoot (per-window UI composition unit)
  - Add `oriterm_ui/src/interaction/` — Pure interaction utilities (resize geometry, cursor hiding, mark mode motion)
  - Change `oriterm/src/app/` description to: "App struct, winit event loop, GPU init, input dispatch — thin shell delegating to WindowRoot"

- [x] Add a "Crate Boundaries" section to `CLAUDE.md`:
  - One-line description of each crate's responsibility (all 5: `oriterm_core`, `oriterm_ui`, `oriterm_mux`, `oriterm_ipc`, `oriterm`)
  - The allowed dependency direction diagram (same as in `crate-boundaries.md`)
  - The litmus test for where code belongs
  - Cross-reference to `.claude/rules/crate-boundaries.md` for full rules

- [x] Update `.claude/rules/impl-hygiene.md` "Module Boundary Discipline" section:
  - Add bullet: "**Crate-level boundaries**: Pure UI logic (testable without GPU/platform/terminal) belongs in `oriterm_ui`, not `oriterm`. See `.claude/rules/crate-boundaries.md` for full ownership rules and allowed dependency directions."

- [x] Update `CLAUDE.md` "UI Framework — Zero Exceptions Rule" to mention WindowRoot:
  - WindowRoot is the per-window composition unit
  - Both test harness and production windows use it
  - No framework state should be owned outside WindowRoot

---

## 09.4 Completion Checklist

- [x] `oriterm/Cargo.toml` `[dev-dependencies]` includes `oriterm_ui` with `features = ["testing"]`
- [x] `.claude/rules/crate-boundaries.md` exists with clear ownership rules for all 5 workspace crates
- [x] Litmus test documented: "Can it run in a #[test] without GPU/platform/terminal?"
- [x] Allowed dependency direction diagram documented in rules file
- [x] `oriterm/tests/architecture.rs` exists with boundary validation tests
- [x] WindowRoot headless construction test passes
- [x] Event propagation through nested containers test passes
- [x] Overlay event routing through WindowRoot test passes
- [x] Interaction state propagation through WindowRoot test passes
- [x] Crate dependency direction test passes (oriterm_ui has no oriterm/oriterm_mux/oriterm_ipc deps)
- [x] `oriterm_core` independence test passes (no oriterm_ui/oriterm_mux/oriterm_ipc deps)
- [x] `oriterm_mux` boundary test passes (no oriterm_ui/oriterm deps)
- [x] `oriterm_ipc` standalone test passes (no oriterm_* deps)
- [x] `oriterm_ui` has no GPU/font deps test passes (no wgpu/tiny-skia/swash/skrifa/rustybuzz)
- [x] `CLAUDE.md` Key Paths updated with new architecture
- [x] `CLAUDE.md` Crate Boundaries section added (covers all 5 workspace crates)
- [x] `.claude/rules/impl-hygiene.md` updated
- [x] `timeout 150 cargo test -p oriterm` passes (architecture tests)
- [x] `timeout 150 ./test-all.sh` passes (full workspace)
- [x] `./clippy-all.sh` clean
- [x] `./build-all.sh` clean

**Exit Criteria:** A developer reading `CLAUDE.md` and `.claude/rules/crate-boundaries.md` can immediately answer "where does this code belong?" for any new feature. Architectural tests in CI catch boundary violations before they merge. The dependency direction tests prevent any crate from depending on crates above it in the dependency graph: `oriterm_core` and `oriterm_ipc` are standalone foundations, `oriterm_ui` depends only on `oriterm_core`, `oriterm_mux` depends only on `oriterm_core` and `oriterm_ipc`, and `oriterm` consumes all of them. The litmus test is simple and unambiguous: testable without GPU/platform/terminal means it belongs in `oriterm_ui`.
