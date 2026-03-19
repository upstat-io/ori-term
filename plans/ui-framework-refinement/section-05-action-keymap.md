---
section: "05"
title: "Action & Keymap System"
status: complete
reviewed: true
goal: "Separate keyboard shortcuts from event handlers via a data-driven keymap. Actions are typed enums declared by widgets. Keybindings are data (not code) that map keystrokes to actions. Dispatch routes through context-scoped focus path."
inspired_by:
  - "GPUI key_dispatch.rs — DispatchTree, KeyContext, Action trait, Keymap"
  - "GPUI actions! macro — action declaration"
depends_on: []
sections:
  - id: "05.0"
    title: "Prerequisite: Restructure Action Module"
    status: complete
  - id: "05.1"
    title: "Action Trait & Registration"
    status: complete
  - id: "05.2"
    title: "Keymap Data Structure"
    status: complete
  - id: "05.3"
    title: "KeyContext for Scope Gating"
    status: complete
  - id: "05.4"
    title: "Action Dispatch Pipeline"
    status: complete
  - id: "05.5"
    title: "Default Keybindings & Controller Migration"
    status: complete
  - id: "05.6"
    title: "Completion Checklist"
    status: complete
---

# Section 05: Action & Keymap System

**Status:** Not Started
**Goal:** A keybinding system where actions are declared as typed data, bound to keystrokes in a `Keymap`, and dispatched through the focus path with context-scoped precedence. Enables runtime rebinding, macro recording, and accessibility -- without changing how widgets handle the resulting actions.

**Context:** Currently, keyboard shortcuts are hardcoded in EventControllers:
- `KeyActivationController` hardcodes `Key::Enter | Key::Space` -> `Clicked`
- `DropdownKeyController` hardcodes arrow keys for navigation, Enter for select, Escape for close
- `MenuKeyController` hardcodes arrow keys for navigation, Enter/Space for select, Escape for dismiss (similar to dropdown but also handles Space)
- `SliderKeyController` hardcodes `ArrowLeft/ArrowDown` -> decrement, `ArrowRight/ArrowUp` -> increment, `Home/End` -> min/max
- `FocusController` hardcodes `Tab/Shift+Tab` -> focus traversal
- `TextEditController` handles cursor movement, selection, character input, Backspace/Delete, Ctrl+A (complex stateful behavior; clipboard ops deferred to app layer)

This means keybindings can't be rebound at runtime, shortcuts can't be listed/documented automatically, macro recording is impossible (no semantic action layer), and testing shortcuts requires simulating exact key combinations.

GPUI's approach: declare actions via macro -> bind to keys in Keymap -> DispatchTree routes to focused element's context -> handler receives typed action. The action layer decouples "what the user wants to do" from "which key they pressed."

**Reference implementations:**
- **GPUI** `src/key_dispatch.rs`: `DispatchTree` built during render. `KeyContext` tags gate which bindings apply. `actions!(editor, [MoveUp, Undo])` macro declares actions.

**Depends on:** None (orthogonal to rendering pipeline).

**Relationship to existing `oriterm/src/keybindings/` module:** The app layer already has a
keybinding system (`keybindings::Action`, `keybindings::KeyBinding`, `keybindings::BindingKey`)
for terminal-level actions (Copy, Paste, NewTab, SplitRight, etc.). That system uses winit key
types and routes through `App::execute_action()`. This section's `KeymapAction` system is for
**widget-level** actions within the `oriterm_ui` framework (Activate, NavigateUp, DismissOverlay,
etc.). The two systems are independent and operate at different layers:
- **`oriterm/src/keybindings/`**: app-level, winit types, terminal actions. Unchanged by this plan.
- **`oriterm_ui/src/action/keymap/`**: widget-level, `oriterm_ui::input` types, UI actions.
The app layer's `handle_dialog_keyboard()` already checks widget dispatch first, then falls back
to global keybindings. This ordering is preserved — keymap dispatch replaces the controller
dispatch step, not the global binding fallback.

---

## 05.0 Prerequisite: Restructure Action Module

**File(s):** `oriterm_ui/src/action.rs` -> `oriterm_ui/src/action/mod.rs`

`action.rs` is currently a single 78-line file containing the `WidgetAction` enum. Must be converted to a directory module before adding submodules.

Expected structure after restructure:
- `action/mod.rs`: ~80 lines (WidgetAction enum, re-exports)
- `action/keymap_action/mod.rs`: ~80 lines (KeymapAction trait + actions! macro)
- `action/keymap_action/tests.rs`: tests for trait, macro expansion, action name()
- `action/keymap/mod.rs`: ~150 lines (Keymap struct, KeyBinding, Keystroke, lookup)
- `action/keymap/tests.rs`: tests for lookup, context precedence, rebind, defaults
- `action/context.rs`: ~60 lines (KeyContext, context stack building — no tests needed, mostly data plumbing)

All source files well under 500-line limit. Test files are exempt from the limit.

- [x] Convert `action.rs` to `action/mod.rs` (move existing `WidgetAction` enum into the directory module)
- [x] Add `pub use` re-exports in `action/mod.rs` for all new submodule types:
  ```rust
  mod keymap_action;
  mod keymap;
  mod context;

  pub use keymap_action::{KeymapAction, actions};
  pub use keymap::{Keymap, KeyBinding, Keystroke};
  pub use context::KeyContext;
  ```
  Note: `keymap_action` and `keymap` are directory modules (each has a sibling `tests.rs`).
  `context` can remain a flat file (no tests expected). `lib.rs` already has `pub mod action;` — no change needed there.
- [x] Verify all imports (`crate::action::WidgetAction`) still resolve — grep for `crate::action` and `oriterm_ui::action` across both `oriterm_ui` and `oriterm` crates
- [x] `./build-all.sh` && `./clippy-all.sh` && `./test-all.sh` pass after restructure

---

## 05.1 Action Trait & Registration

**File(s):** `oriterm_ui/src/action/keymap_action/mod.rs` (+ `tests.rs` sibling)

- [x] Define `KeymapAction` trait:
  ```rust
  pub trait KeymapAction: std::any::Any + std::fmt::Debug {
      fn name(&self) -> &'static str;
      fn boxed_clone(&self) -> Box<dyn KeymapAction>;
  }
  ```
  **Why `boxed_clone()`:** `Keymap` stores `Box<dyn KeymapAction>` in each `KeyBinding`.
  When a binding matches, the action must be cloned to pass to the widget's action handler
  (the keymap retains ownership of its binding list). `boxed_clone()` enables cloning
  trait objects without requiring `Clone` on `dyn KeymapAction`.

- [x] Macro for action declaration:
  ```rust
  /// Declares keymappable actions.
  ///
  /// Usage: `actions!(settings, [ResetDefaults, NavigateUp, NavigateDown]);`
  /// Expands to unit structs implementing `KeymapAction` with `name()` returning
  /// `"settings::ResetDefaults"` etc.
  macro_rules! actions { ... }
  ```

- [x] Declare the core widget actions needed for controller migration:
  ```rust
  actions!(widget, [
      Activate,           // Enter/Space -> Clicked (replaces KeyActivationController)
      NavigateUp,         // ArrowUp (dropdown/menu navigation)
      NavigateDown,       // ArrowDown (dropdown/menu navigation)
      Confirm,            // Enter (dropdown/menu confirm selection)
      Dismiss,            // Escape (close overlay/dialog)
      FocusNext,          // Tab
      FocusPrev,          // Shift+Tab
      IncrementValue,     // ArrowRight/ArrowUp (slider)
      DecrementValue,     // ArrowLeft/ArrowDown (slider)
      ValueToMin,         // Home (slider)
      ValueToMax,         // End (slider)
  ]);
  ```

**WARNING: `FocusNext`/`FocusPrev` are not `WidgetAction` variants.** They need to set
`ControllerRequests::FOCUS_NEXT`/`FOCUS_PREV` flags on the dispatch result, not emit a
`WidgetAction`. The keymap dispatch path must handle these specially: when a keymap lookup
returns `FocusNext` or `FocusPrev`, set the appropriate `ControllerRequests` flag on the
result instead of calling `handle_keymap_action()`. Use `KeymapAction::name()` to distinguish:
`"widget::FocusNext"` and `"widget::FocusPrev"` are framework-level, everything else
is widget-level.

- [x] `./build-all.sh` && `./clippy-all.sh` pass

---

## 05.2 Keymap Data Structure

**File(s):** `oriterm_ui/src/action/keymap/mod.rs` (+ `tests.rs` sibling)

- [x] Define `Keymap`, `KeyBinding`, and `Keystroke`:
  ```rust
  pub struct Keymap {
      bindings: Vec<KeyBinding>,
  }

  pub struct KeyBinding {
      pub keystroke: Keystroke,
      pub action: Box<dyn KeymapAction>,
      pub context: Option<&'static str>,  // e.g., "Settings", "Dialog"
  }

  pub struct Keystroke {
      pub key: Key,
      pub modifiers: Modifiers,
  }
  ```
  **Note:** `Key` and `Modifiers` already exist at `oriterm_ui::input` (re-exported from the private `input::event` module).
  `KeyEvent` also exists with the same `{key, modifiers}` shape but only derives `Eq`, not `Hash`.
  `Key` and `Modifiers` both derive `Hash`, so adding `Hash` to `KeyEvent` is trivial.
  Consider reusing `KeyEvent` directly (after adding `Hash`), or define `Keystroke` as a thin wrapper
  that adds `Hash` for keymap lookup.

- [x] If reusing `KeyEvent` as `Keystroke`: add `Hash` derive to `KeyEvent` in
  `oriterm_ui/src/input/event.rs` (line ~156: `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`
  -> `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]`). If defining `Keystroke` separately,
  ensure it derives `Eq + Hash` and has a `From<KeyEvent>` impl for conversion convenience.

- [x] Implement `Keymap::lookup()`:
  ```rust
  impl Keymap {
      /// Finds the best-matching action for a keystroke given a context stack.
      ///
      /// Returns `None` if no binding matches. When multiple bindings match
      /// the same keystroke, context-scoped bindings win over `context: None`.
      /// Among context-scoped bindings, deeper context (later in stack) wins.
      pub fn lookup(
          &self,
          keystroke: &Keystroke,
          context_stack: &[&'static str],
      ) -> Option<Box<dyn KeymapAction>> {
          // ... returns boxed_clone() of the best match
      }
  }
  ```

- [x] Hardcode default keybindings in Rust code (no config file yet):
  ```rust
  impl Keymap {
      pub fn defaults() -> Self {
          Self {
              bindings: vec![
                  // Activate is scoped to "Button" context — NOT context:None.
                  // context:None would intercept Enter/Space before TextEditController
                  // can handle them in text inputs. Every widget that wants Enter/Space
                  // activation must return a key_context().
                  KeyBinding::new(Keystroke::new(Key::Enter, Modifiers::NONE), Activate, Some("Button")),
                  KeyBinding::new(Keystroke::new(Key::Space, Modifiers::NONE), Activate, Some("Button")),
                  // ... etc
              ],
          }
      }
  }
  ```
  **WARNING: `Activate` must NOT use `context: None`.** Enter and Space are also
  character keys consumed by `TextEditController` (Enter for newlines, Space for
  literal spaces). If bound globally, the keymap would intercept them before
  the text edit controller can handle them (keymap runs first). Instead, scope
  `Activate` to a context like `"Button"` and have `ButtonWidget`, `ToggleWidget`,
  `CheckboxWidget` return `key_context() -> Some("Button")`. This means
  `key_context()` overrides are needed on MORE widgets than initially listed in
  section 05.3.
- [ ] **[follow-up, not this plan]** TOML config loading for keybindings -- requires keystroke
  parsing, action name registry, config file discovery (XDG/Library/AppData), and
  cross-platform path handling. Tracked separately.
- [x] Implement `Keymap::rebind()` for runtime user overrides (merge with defaults)
- [x] `./build-all.sh` && `./clippy-all.sh` pass

---

## 05.3 KeyContext for Scope Gating

**File(s):** `oriterm_ui/src/action/context.rs`, `oriterm_ui/src/widgets/mod.rs`

- [x] Add `key_context()` method to the `Widget` trait in `oriterm_ui/src/widgets/mod.rs`:
  ```rust
  /// Returns the key context tag for this widget, if any.
  ///
  /// Used by the keymap dispatch pipeline to build a context stack from
  /// the focus path. Bindings with `context: Some("Settings")` only
  /// fire when a widget returning `Some("Settings")` is in the focus
  /// ancestor chain.
  fn key_context(&self) -> Option<&'static str> { None }
  ```
  Place after `hit_test_behavior()` and before `reset_scroll()` (same region as
  other metadata methods).

- [x] Override `key_context()` on widgets that need scoped bindings:
  - `ButtonWidget` -> `Some("Button")` (needed because Activate must be scoped, not global)
  - `ToggleWidget` -> `Some("Button")` (same Activate context as button)
  - `CheckboxWidget` -> `Some("Button")` (same Activate context as button)
  - `SettingsPanelWidget` -> `Some("Settings")`
  - `DialogWidget` -> `Some("Dialog")`
  - `DropdownWidget` (or its overlay content) -> `Some("Dropdown")`
  - `MenuWidget` -> `Some("Menu")`
  - `SliderWidget` -> `Some("Slider")`
  **Sync point:** Each widget file that overrides `key_context()` must be updated.
  Grep `fn controllers(&self)` to find all widgets with keyboard controllers —
  each is a candidate for a `key_context()` override.
  **Note:** Button/Toggle/Checkbox share `"Button"` context because they all use
  Enter/Space for activation. Text inputs do NOT get a `key_context()` override,
  so Enter/Space with `context: Some("Button")` will not match when a text input
  is focused.

- [x] Define `build_context_stack` helper in `action/context.rs`:
  ```rust
  /// Builds a context stack from a focus path using a pre-collected context map.
  ///
  /// Looks up each focus path widget ID in `context_map` and collects
  /// non-None `key_context()` values. The resulting stack is ordered root-to-leaf.
  pub fn build_context_stack(
      context_map: &HashMap<WidgetId, &'static str>,
      focus_path: &[WidgetId],
  ) -> Vec<&'static str> { ... }
  ```
  **Why a pre-built map, not a tree walk:** Building the context stack requires
  reading `key_context()` from widgets in the tree. The Widget trait only has
  `for_each_child_mut` (no immutable variant), so walking the tree immutably is
  not possible without adding a new trait method. Instead, during
  `register_widget_tree()` (which already walks `for_each_child_mut`), collect a
  `HashMap<WidgetId, &'static str>` of widgets that return `Some` from
  `key_context()`. Pass this map to `build_context_stack` at dispatch time.

- [x] During dispatch, build context stack from focus path (ancestor chain of key_context values)
- [x] Bindings with `context: Some("Dialog")` only fire when a Dialog widget is in the focus ancestor chain
- [x] `./build-all.sh` && `./clippy-all.sh` pass

---

## 05.4 Action Dispatch Pipeline

**Integration points — three dispatch paths must be updated:**

1. **Test harness:** `oriterm_ui/src/testing/harness_dispatch.rs` — `process_event()` calls
   `deliver_event_to_tree()` at line ~46. Keymap lookup must run BEFORE this call for
   keyboard events. The `WidgetTestHarness` struct (in `harness.rs`) must store a `Keymap`.

2. **App layer (dialogs):** `oriterm/src/app/dialog_context/content_actions.rs` —
   `dispatch_dialog_content_key()` calls `deliver_event_to_tree()` at line ~360. Keymap
   lookup must run BEFORE this call. The `DialogWindowContext` must store (or reference)
   a `Keymap`.

3. **App layer (main window/overlays):** Any other keyboard dispatch path in
   `oriterm/src/app/` that routes through `deliver_event_to_tree()` for widget trees.
   Grep for `deliver_event_to_tree` to find all call sites.

**Integration pattern** for each call site:

```rust
// BEFORE (current):
let result = deliver_event_to_tree(widget, &event, bounds, ...);

// AFTER (with keymap):
let result = match &event {
    InputEvent::KeyDown { key, modifiers } => {
        let keystroke = Keystroke { key: *key, modifiers: *modifiers };
        let context_stack = build_context_stack(&context_map, &focus_path);
        if let Some(action) = keymap.lookup(&keystroke, &context_stack) {
            let mut result = TreeDispatchResult::new();
            result.handled = true;
            // Framework actions: set ControllerRequests flags.
            match action.name() {
                "widget::FocusNext" => {
                    result.requests = ControllerRequests::FOCUS_NEXT;
                    result.source = focus_path.last().copied();
                }
                "widget::FocusPrev" => {
                    result.requests = ControllerRequests::FOCUS_PREV;
                    result.source = focus_path.last().copied();
                }
                _ => {
                    // Widget actions: deliver to focused widget.
                    // TODO: need mutable access to focused widget — walk tree
                    // to find it by ID, then call handle_keymap_action().
                    if let Some(widget_action) = find_and_handle_keymap_action(
                        widget, &*action, focused_id, bounds,
                    ) {
                        result.actions.push(widget_action);
                    }
                    result.source = Some(focused_id);
                }
            }
            // Track for KeyUp suppression.
            last_keymap_handled = Some(*key);
            result
        } else {
            deliver_event_to_tree(widget, &event, bounds, ...)
        }
    }
    InputEvent::KeyUp { key, .. } if last_keymap_handled == Some(*key) => {
        // Suppress KeyUp for keys the keymap already handled.
        last_keymap_handled = None;
        TreeDispatchResult::new() // handled, no action
    }
    _ => deliver_event_to_tree(widget, &event, bounds, ...),
};
```
**Complexity warning:** `find_and_handle_keymap_action()` must walk the widget tree
to find the focused widget by ID, then call `handle_keymap_action()` on it. This is
a new tree walk pattern not currently in the codebase. Consider adding a
`find_widget_mut(widget: &mut dyn Widget, id: WidgetId) -> Option<&mut dyn Widget>`
helper to `pipeline.rs` or the dispatch module. This walk is O(n) in tree size but
only runs on keymap-matched keyboard events (not per-frame).

**KeymapAction to WidgetAction conversion:** The keymap produces a `Box<dyn KeymapAction>`.
The dispatch pipeline needs a `WidgetAction` enum variant. **Use Approach B:**

Add a `Widget::handle_keymap_action()` method:
```rust
/// Handles a keymap-resolved action.
///
/// Called by the dispatch pipeline when a keystroke matched a keymap
/// binding. The widget maps the semantic `KeymapAction` to a
/// `WidgetAction` using its own state (e.g., `NavigateDown` + current
/// selected index -> `Selected { id, index: current + 1 }`).
///
/// Return `Some(action)` to emit a `WidgetAction`, or `None` to
/// suppress. Default returns `None` (widget does not handle keymap actions).
fn handle_keymap_action(
    &mut self,
    action: &dyn KeymapAction,
    _bounds: Rect,
) -> Option<WidgetAction> {
    let _ = action;
    None
}
```

This is preferred over per-context conversion functions (Approach A) because:
- Keeps widget-specific logic inside the widget (no external mapping tables).
- Widgets can use `action.name()` to match and `Any::downcast_ref()` to access typed data.
- The dispatch pipeline stays generic (no per-context plumbing).
- Pattern is identical to existing `on_action()` method.

**The method goes on the `Widget` trait** in `oriterm_ui/src/widgets/mod.rs`, placed
after `on_action()` (same region as action handling methods).

**Stateful controllers and keymap actions:** `DropdownKeyController`, `MenuKeyController`, and
`SliderKeyController` own state (selected index, current value, clickable indices). The keymap
system produces stateless actions (e.g., `NavigateDown`). When these controllers are migrated:
- The **state** (selected index, value, etc.) moves to the **widget** (which already has it
  or can easily own it — dropdowns track `selected`, sliders track `value`).
- The keymap action is delivered to the widget via `handle_keymap_action()`.
- The widget mutates its own state and emits the appropriate `WidgetAction`.
- The controller is removed entirely; its state was duplicated anyway (controllers and widgets
  both tracked `selected`/`value`).

For keyboard events:
1. Build context stack from focus path (using pre-built `HashMap<WidgetId, &'static str>`).
2. Look up keystroke in keymap with context stack.
3. If match found:
   a. **Framework actions** (`FocusNext`, `FocusPrev`): set `ControllerRequests::FOCUS_NEXT` /
      `FOCUS_PREV` on the result. Do NOT call `handle_keymap_action()`.
   b. **Widget actions** (everything else): call `widget.handle_keymap_action(&*action, bounds)`
      on the focused widget. If it returns `Some(widget_action)`, add to result actions.
      Mark result as handled. SKIP controller dispatch for this event.
4. If no match: fall through to existing `deliver_event_to_tree` -> controller pipeline unchanged.

This means existing controllers continue to work as fallbacks for any key not in the keymap.

**Keymap ownership and storage:**

| Location | Owns `Keymap`? | How |
|---|---|---|
| `WidgetTestHarness` | YES | New field: `keymap: Keymap`. Initialized with `Keymap::defaults()` in constructor. Exposed via `harness.keymap_mut()` for test-specific rebinding. |
| `DialogWindowContext` | YES | New field: `keymap: Keymap`. Passed during dialog construction. |
| `App` (main window) | YES | New field: `ui_keymap: Keymap`. Shared reference passed to widget dispatch helpers. |

All three locations use `Keymap::defaults()` initially. The `App`-level keymap can be
mutated via `Keymap::rebind()` for runtime customization.

- [x] Add `Widget::handle_keymap_action()` method to the Widget trait in `oriterm_ui/src/widgets/mod.rs`
  (default returns `None`, place after `on_action()`)
- [x] Add `find_widget_mut()` tree walk helper to `oriterm_ui/src/pipeline/mod.rs`
  (walks `for_each_child_mut` to find a widget by ID, returns `Option<&mut dyn Widget>`)
- [x] Add `keymap: Keymap` field to `WidgetTestHarness` in `oriterm_ui/src/testing/harness.rs`
- [x] Add `keymap: Keymap` field to `DialogWindowContext` in `oriterm/src/app/dialog_context/mod.rs`
- [x] Add context map collection: extend `register_widget_tree()` in `oriterm_ui/src/pipeline/mod.rs`
  to also collect `key_context()` from each widget. Either:
  - Add an `out_context: &mut HashMap<WidgetId, &'static str>` parameter (only inserts for
    widgets where `key_context()` returns `Some`), or
  - Create a parallel `collect_key_contexts()` function with the same tree-walk pattern.
  The first approach is better (one walk, no redundancy). Store the map on `WidgetTestHarness`
  and `DialogWindowContext` alongside the existing `InteractionManager`.
  **Do NOT store on `InteractionManager`** — that struct owns interaction state (hot/active/focus),
  not keymap metadata. Keep concerns separated per module boundary discipline.
- [x] Modify `process_event()` in `harness_dispatch.rs` to intercept keyboard events before
  `deliver_event_to_tree()` and try keymap lookup first
- [x] Modify `dispatch_dialog_content_key()` in `content_actions.rs` to intercept keyboard
  events before `deliver_event_to_tree()` and try keymap lookup first
- [x] On `KeyDown`, look up matching binding in keymap
- [x] Walk focus path, checking context stack for scope matches
- [x] Deepest matching context wins (child overrides parent)
- [x] Deliver matched action to widget's action handler
- [x] Unmatched keys fall through to existing controller dispatch unchanged
- [x] **KeyUp handling:** During coexistence, controllers still consume matching `KeyUp`
  events. After a controller is fully removed, orphaned `KeyUp` events for keymap-handled
  keys would leak to parent widgets. Add `KeyUp` suppression: if the keymap handled a
  `KeyDown` for a key, also suppress the matching `KeyUp`. Track the last keymap-handled
  key in the dispatch state.
  **Implementation note:** The `last_keymap_handled: Option<Key>` field must persist
  across `process_event()` calls (it's set on `KeyDown`, consumed on next `KeyUp`).
  In the harness: add field to `WidgetTestHarness`. In app layer: add field to
  `DialogWindowContext`. This is per-dispatch-path state, not per-frame.
- [x] `./build-all.sh` && `./clippy-all.sh` && `./test-all.sh` pass

---

## 05.5 Default Keybindings & Controller Migration

**Controller migration plan:**

| Controller | Migrate to keymap? | Rationale |
|---|---|---|
| `KeyActivationController` | YES | Simple key->action. Perfect keymap candidate. |
| `FocusController` | PARTIAL | Tab/Shift+Tab to keymap. Focus tracking logic stays as controller. |
| `DropdownKeyController` | YES | Arrow/Enter/Escape are rebindable shortcuts. |
| `MenuKeyController` | YES | Arrow/Enter/Space/Escape are rebindable shortcuts. |
| `SliderKeyController` | YES | Arrow keys and Home/End are rebindable. |
| `TextEditController` | NO | Complex stateful behavior (cursor movement, selection, character input, Backspace/Delete). Not suitable for simple key->action mapping. Stays as controller permanently. |

- [x] Define defaults for all existing keyboard-activated widgets:
  - Enter/Space -> Activate, `context: Some("Button")` (replaces `KeyActivationController`)
  - Tab/Shift+Tab -> FocusNext/FocusPrev, `context: None` (replaces `FocusController` key handling — safe with `context: None` because TextEditController does not handle Tab)
  - ArrowDown/ArrowUp -> NavigateDown/NavigateUp, `context: Some("Dropdown")` (replaces `DropdownKeyController`)
  - ArrowDown/ArrowUp -> NavigateDown/NavigateUp, `context: Some("Menu")` (replaces `MenuKeyController`)
  - Enter -> Confirm, `context: Some("Dropdown")` and `context: Some("Menu")`
  - Space -> Confirm, `context: Some("Menu")` (menu-only; dropdown does not use Space)
  - Escape -> Dismiss, `context: Some("Dropdown")` and `context: Some("Menu")` and `context: Some("Dialog")`
  - Arrow Left/Down -> DecrementValue, Arrow Right/Up -> IncrementValue, Home -> ValueToMin, End -> ValueToMax, all `context: Some("Slider")` (replaces `SliderKeyController`)

- [x] **Arrow key context scoping:** ArrowUp/ArrowDown are used by BOTH dropdown/menu
  navigation AND slider increment/decrement. These MUST be scoped to different contexts:
  - `context: Some("Dropdown")` for dropdown arrow navigation
  - `context: Some("Menu")` for menu arrow navigation
  - `context: Some("Slider")` for slider arrow adjustment
  Without context scoping, a global ArrowDown binding would conflict across all three
  widget types. This is the primary motivation for the context system.

- [x] **Coexistence during migration:** Both systems run simultaneously.
  Keymap is checked first; if no binding matches, controllers handle it.
  Controllers are removed one at a time after their bindings are in the keymap
  and integration tests pass. This is a gradual migration, not a big-bang switch.

- [x] **Migration order (with file sync points):**
  1. Add keymap infrastructure (05.0-05.4).
  2. Add default bindings for `KeyActivationController` (Enter/Space -> Activate, `context: Some("Button")`).
  3. Verify button activation works via keymap (test harness: `key_press(Key::Enter)` on focused button -> `Clicked` action).
  4. Migrate `KeyActivationController` from buttons/toggles/checkboxes.
     **For each widget:**
     a. Add `key_context() -> Some("Button")` override.
     b. Add `handle_keymap_action()` override: match `"widget::Activate"` -> return `Some(WidgetAction::Clicked(self.id()))`.
     c. Verify activation works via keymap in test harness.
     d. Remove `KeyActivationController` from `controllers()`.
     **Files to update:**
     - `oriterm_ui/src/widgets/button/mod.rs` — add `key_context()`, `handle_keymap_action()`, remove `KeyActivationController`
     - `oriterm_ui/src/widgets/toggle/mod.rs` — add `key_context()`, `handle_keymap_action()`, remove `KeyActivationController`
     - `oriterm_ui/src/widgets/checkbox/mod.rs` — add `key_context()`, `handle_keymap_action()`, remove `KeyActivationController`
     - (Grep `KeyActivationController` across `oriterm_ui/src/widgets/` to find all)
     - `./build-all.sh` && `./clippy-all.sh` && `./test-all.sh` after each widget
  5. Repeat for `DropdownKeyController`, `MenuKeyController`, `SliderKeyController`.
     **For each:** move state from controller to widget, add `key_context()` override,
     add context-scoped bindings, verify via test harness, remove controller.
     **Files to update per controller:**
     - Widget file (e.g., `dropdown/mod.rs`): remove controller from `controllers()`, add `key_context()`, add `handle_keymap_action()` override
     - Controller directory MUST be deleted (not left as dead code — `dead_code = "deny"` in `[lints.rust]`)
     - `oriterm_ui/src/controllers/mod.rs`: remove `pub mod` declaration AND `pub use` re-export
  6. Migrate Tab/Shift+Tab from `FocusController` to keymap (keep focus tracking logic in controller).
     **FocusController changes:**
     - Remove `Key::Tab` handling from `handle_event()` (only click-to-focus and lifecycle remain)
     - `FocusController` is NOT deleted — it still handles `MouseDown` -> `REQUEST_FOCUS`
       and `FocusChanged` lifecycle -> `PAINT`
     - Add `FocusNext`/`FocusPrev` keymap actions with `context: None`
  7. `TextEditController` is NOT migrated -- it stays as-is.

---

## 05.6 Completion Checklist

- [x] Tests for action submodules follow sibling `tests.rs` pattern:
  - `action/keymap_action/tests.rs` — trait, macro expansion, `name()` correctness
  - `action/keymap/tests.rs` — `lookup()`, context precedence, `rebind()`, `defaults()`
  - Each submodule's `mod.rs` ends with `#[cfg(test)] mod tests;`
- [x] `action.rs` converted to `action/mod.rs` directory module with all imports preserved
- [x] `KeymapAction` trait defined with `name()` and `boxed_clone()`
- [x] `actions!` macro for declaring actions
- [x] `Keymap` struct with `Vec<KeyBinding>`, hardcoded defaults in Rust (TOML config deferred to follow-up)
- [x] `Keystroke` type defined with `Eq + Hash` (reuse `KeyEvent` fields or thin wrapper)
- [x] `Widget::handle_keymap_action()` method on the Widget trait with default no-op
- [x] `KeyContext` scope gating on widgets via `key_context()` trait method
- [x] Actions declared as typed data, not inline key checks
- [x] Keymap binds keystrokes to actions with context gating
- [x] Dispatch routes through focus path with deepest-context-wins precedence
- [x] `find_widget_mut()` helper for walking tree to focused widget (needed by keymap dispatch)
- [x] Keymap lookup integrated BEFORE existing controller dispatch (unmatched keys fall through)
- [x] Default bindings cover all existing keyboard interactions (except TextEditController)
- [x] `KeyActivationController` migrated to keymap and removed from button/toggle/checkbox
- [x] Runtime rebinding works (change binding -> new key activates action)
- [x] `TextEditController` documented as NOT migrating to keymap (complex stateful behavior)
- [x] **Test: keymap lookup** — `Keymap::lookup()` returns correct action for exact keystroke match
- [x] **Test: keymap lookup with modifiers** — Shift+Tab matches `FocusPrev`, not `FocusNext`
- [x] **Test: context scoping** — ArrowDown in "Dropdown" context returns `NavigateDown`, not `DecrementValue`
- [x] **Test: context precedence** — deeper context overrides shallower context for same keystroke
- [x] **Test: unmatched fallthrough** — unrecognized key returns `None` from lookup, controllers still handle it
- [x] **Test: harness keymap integration** — `harness.key_press(Key::Enter)` on focused button produces `Clicked` action via keymap path
- [x] **Test: rebind** — after `keymap.rebind(Key::Enter -> SomeOtherAction)`, Enter no longer activates buttons
- [x] **Test: KeyUp suppression** — after keymap handles `KeyDown(Enter)`, the matching `KeyUp(Enter)` does not leak to parent widgets
- [x] **Test: TextEditController unaffected** — text input still works after keymap is active (character keys not in keymap fall through to controller)
- [x] **Test: coexistence** — during migration, controller-handled keys still work for controllers not yet migrated
- [x] `./build-all.sh` passes
- [x] `./clippy-all.sh` clean
- [x] `./test-all.sh` passes

**Exit Criteria:** Pressing Enter on a focused button triggers `KeymapAction::Activate`, routed through the keymap, not a hardcoded `Key::Enter` check. The binding is rebindable via `Keymap::rebind()`. `KeyActivationController` is removed from all widgets that used it. `TextEditController` continues working unchanged.
