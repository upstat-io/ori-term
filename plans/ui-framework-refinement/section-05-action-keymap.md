---
section: "05"
title: "Action & Keymap System"
status: not-started
reviewed: false
goal: "Separate keyboard shortcuts from event handlers via a data-driven keymap. Actions are typed enums declared by widgets. Keybindings are data (not code) that map keystrokes to actions. Dispatch routes through context-scoped focus path."
inspired_by:
  - "GPUI key_dispatch.rs — DispatchTree, KeyContext, Action trait, Keymap"
  - "GPUI actions! macro — action declaration"
depends_on: []
sections:
  - id: "05.0"
    title: "Prerequisite: Restructure Action Module"
    status: not-started
  - id: "05.1"
    title: "Action Trait & Registration"
    status: not-started
  - id: "05.2"
    title: "Keymap Data Structure"
    status: not-started
  - id: "05.3"
    title: "KeyContext for Scope Gating"
    status: not-started
  - id: "05.4"
    title: "Action Dispatch Pipeline"
    status: not-started
  - id: "05.5"
    title: "Default Keybindings & Controller Migration"
    status: not-started
  - id: "05.6"
    title: "Completion Checklist"
    status: not-started
---

# Section 05: Action & Keymap System

**Status:** Not Started
**Goal:** A keybinding system where actions are declared as typed data, bound to keystrokes in a `Keymap`, and dispatched through the focus path with context-scoped precedence. Enables runtime rebinding, macro recording, and accessibility -- without changing how widgets handle the resulting actions.

**Context:** Currently, keyboard shortcuts are hardcoded in EventControllers:
- `KeyActivationController` hardcodes `Key::Enter | Key::Space` -> `Clicked`
- `DropdownKeyController` hardcodes arrow keys for navigation, Enter for select, Escape for close
- `MenuKeyController` hardcodes the same pattern as dropdown
- `SliderKeyController` hardcodes `ArrowLeft/ArrowRight` -> decrement/increment
- `FocusController` hardcodes `Tab/Shift+Tab` -> focus traversal
- `TextEditController` handles cursor movement, selection, clipboard (complex stateful behavior)

This means keybindings can't be rebound at runtime, shortcuts can't be listed/documented automatically, macro recording is impossible (no semantic action layer), and testing shortcuts requires simulating exact key combinations.

GPUI's approach: declare actions via macro -> bind to keys in Keymap -> DispatchTree routes to focused element's context -> handler receives typed action. The action layer decouples "what the user wants to do" from "which key they pressed."

**Reference implementations:**
- **GPUI** `src/key_dispatch.rs`: `DispatchTree` built during render. `KeyContext` tags gate which bindings apply. `actions!(editor, [MoveUp, Undo])` macro declares actions.

**Depends on:** None (orthogonal to rendering pipeline).

---

## 05.0 Prerequisite: Restructure Action Module

**File(s):** `oriterm_ui/src/action.rs` -> `oriterm_ui/src/action/mod.rs`

`action.rs` is currently a single 78-line file containing the `WidgetAction` enum. Must be converted to a directory module before adding submodules.

Expected structure after restructure:
- `action/mod.rs`: ~80 lines (WidgetAction enum, re-exports)
- `action/keymap_action.rs`: ~80 lines (KeymapAction trait + actions! macro)
- `action/keymap.rs`: ~150 lines (Keymap struct, KeyBinding, Keystroke, lookup)
- `action/context.rs`: ~60 lines (KeyContext, context stack building)

All files well under 500-line limit.

- [ ] Convert `action.rs` to `action/mod.rs` (move existing `WidgetAction` enum into the directory module)
- [ ] Verify all imports (`crate::action::WidgetAction`) still resolve
- [ ] `./build-all.sh` passes after restructure

---

## 05.1 Action Trait & Registration

**File(s):** `oriterm_ui/src/action/keymap_action.rs`

- [ ] Define `KeymapAction` trait:
  ```rust
  pub trait KeymapAction: std::any::Any + std::fmt::Debug {
      fn name(&self) -> &'static str;
      fn boxed_clone(&self) -> Box<dyn KeymapAction>;
  }
  ```

- [ ] Macro for action declaration:
  ```rust
  /// Declares keymappable actions.
  ///
  /// Usage: `actions!(settings, [ResetDefaults, NavigateUp, NavigateDown]);`
  macro_rules! actions { ... }
  ```

---

## 05.2 Keymap Data Structure

**File(s):** `oriterm_ui/src/action/keymap.rs`

- [ ] Define `Keymap`, `KeyBinding`, and `Keystroke`:
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
  **Note:** `Key` and `Modifiers` already exist at `oriterm_ui::input::event`.
  `KeyEvent` also exists with the same `{key, modifiers}` shape. Consider reusing
  `KeyEvent` directly (if it has `Eq + Hash`), or define `Keystroke` as a thin wrapper
  that adds `Eq + Hash` for keymap lookup.

- [ ] Hardcode default keybindings in Rust code (no config file yet):
  ```rust
  impl Keymap {
      pub fn defaults() -> Self {
          Self {
              bindings: vec![
                  KeyBinding::new(Keystroke::new(Key::Enter, Modifiers::empty()), Activate, None),
                  KeyBinding::new(Keystroke::new(Key::Space, Modifiers::empty()), Activate, None),
                  // ... etc
              ],
          }
      }
  }
  ```
- [ ] **[follow-up, not this plan]** TOML config loading for keybindings -- requires keystroke
  parsing, action name registry, config file discovery (XDG/Library/AppData), and
  cross-platform path handling. Tracked separately.
- [ ] Implement `Keymap::rebind()` for runtime user overrides (merge with defaults)

---

## 05.3 KeyContext for Scope Gating

**File(s):** `oriterm_ui/src/action/context.rs`

- [ ] Widgets declare their context via a new trait method:
  ```rust
  fn key_context(&self) -> Option<&'static str> { None }
  ```

- [ ] During dispatch, build context stack from focus path (ancestor chain of key_context values)
- [ ] Bindings with `context: Some("Dialog")` only fire when a Dialog widget is in the focus ancestor chain

---

## 05.4 Action Dispatch Pipeline

Integration point: keymap lookup runs BEFORE existing controller dispatch. For keyboard events:
1. Build context stack from focus path.
2. Look up keystroke in keymap with context stack.
3. If match found: convert to `WidgetAction`, add to result, mark handled. SKIP
   controller dispatch for this event.
4. If no match: fall through to existing `dispatch_to_widget_tree` -> controller
   pipeline unchanged.

This means existing controllers continue to work as fallbacks for any key not in the keymap.

- [ ] On `KeyDown`, look up matching binding in keymap
- [ ] Walk focus path, checking context stack for scope matches
- [ ] Deepest matching context wins (child overrides parent)
- [ ] Deliver matched action to widget's action handler
- [ ] Unmatched keys fall through to existing controller dispatch unchanged

---

## 05.5 Default Keybindings & Controller Migration

**Controller migration plan:**

| Controller | Migrate to keymap? | Rationale |
|---|---|---|
| `KeyActivationController` | YES | Simple key->action. Perfect keymap candidate. |
| `FocusController` | PARTIAL | Tab/Shift+Tab to keymap. Focus tracking logic stays as controller. |
| `DropdownKeyController` | YES | Arrow/Enter/Escape are rebindable shortcuts. |
| `MenuKeyController` | YES | Same pattern as dropdown. |
| `SliderKeyController` | YES | Arrow keys are rebindable. |
| `TextEditController` | NO | Complex stateful behavior (cursor movement, selection, clipboard). Not suitable for simple key->action mapping. Stays as controller permanently. |

- [ ] Define defaults for all existing keyboard-activated widgets:
  - Enter/Space -> activate focused button (replaces `KeyActivationController`)
  - Tab/Shift+Tab -> focus traversal (replaces `FocusController` key handling)
  - Arrow keys -> dropdown/menu navigation (replaces `DropdownKeyController`, `MenuKeyController`)
  - Escape -> close dialog/dropdown (replaces hardcoded Escape handling)
  - Arrow Left/Right -> slider adjust (replaces `SliderKeyController`)

- [ ] **Coexistence during migration:** Both systems run simultaneously.
  Keymap is checked first; if no binding matches, controllers handle it.
  Controllers are removed one at a time after their bindings are in the keymap
  and integration tests pass. This is a gradual migration, not a big-bang switch.

- [ ] **Migration order:**
  1. Add keymap infrastructure (05.0-05.4).
  2. Add default bindings for `KeyActivationController` (Enter/Space -> Activate).
  3. Verify button activation works via keymap.
  4. Remove `KeyActivationController` from buttons/toggles/checkboxes.
  5. Repeat for `DropdownKeyController`, `MenuKeyController`, `SliderKeyController`.
  6. Migrate Tab/Shift+Tab from `FocusController` to keymap (keep focus tracking logic in controller).
  7. `TextEditController` is NOT migrated -- it stays as-is.

---

## 05.6 Completion Checklist

- [ ] Tests for action module follow sibling tests.rs pattern (`action/tests.rs` for keymap + context tests)
- [ ] `action.rs` converted to `action/mod.rs` directory module with all imports preserved
- [ ] `KeymapAction` trait defined with `name()` and `boxed_clone()`
- [ ] `actions!` macro for declaring actions
- [ ] `Keymap` struct with `Vec<KeyBinding>`, hardcoded defaults in Rust (TOML config deferred to follow-up)
- [ ] `Keystroke` type defined with `Eq + Hash` (reuse `KeyEvent` fields or thin wrapper)
- [ ] `KeyContext` scope gating on widgets via `key_context()` trait method
- [ ] Actions declared as typed data, not inline key checks
- [ ] Keymap binds keystrokes to actions with context gating
- [ ] Dispatch routes through focus path with deepest-context-wins precedence
- [ ] Keymap lookup integrated BEFORE existing controller dispatch (unmatched keys fall through)
- [ ] Default bindings cover all existing keyboard interactions (except TextEditController)
- [ ] `KeyActivationController` migrated to keymap and removed from button/toggle/checkbox
- [ ] Runtime rebinding works (change binding -> new key activates action)
- [ ] `TextEditController` documented as NOT migrating to keymap (complex stateful behavior)
- [ ] `./test-all.sh` passes
- [ ] `./clippy-all.sh` clean

**Exit Criteria:** Pressing Enter on a focused button triggers `KeymapAction::Activate`, routed through the keymap, not a hardcoded `Key::Enter` check. The binding is rebindable via `Keymap::rebind()`. `KeyActivationController` is removed from all widgets that used it. `TextEditController` continues working unchanged.
