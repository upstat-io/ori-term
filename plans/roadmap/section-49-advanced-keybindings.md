---
section: 49
title: Advanced Keybinding System
status: not-started
reviewed: false
third_party_review:
  status: none
  updated: null
tier: 5
goal: Key tables (modal bindings), chained keybinds, catch-all keys, and key remapping — the power-user keybinding features that enable tmux-like workflows
sections:
  - id: "49.1"
    title: Key Tables
    status: not-started
  - id: "49.2"
    title: Chained Keybinds
    status: not-started
  - id: "49.3"
    title: Catch-All Key
    status: not-started
  - id: "49.4"
    title: Key Remapping
    status: not-started
  - id: "49.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "49.5"
    title: Section Completion
    status: not-started
---

# Section 49: Advanced Keybinding System

**Status:** Not Started
**Goal:** Extend the keybinding system (Section 13) with power-user features: named key tables for modal input (tmux prefix-key style), chained bindings (one key triggers multiple actions), catch-all keys (match any unbound key), and key remapping (remap modifier/key combinations at the terminal level).

**Crate:** `oriterm` (keybinding dispatch, config)
**Dependencies:** Section 13 (basic keybinding system — already complete)

**Reference:**
- Ghostty 1.3.0: key tables, chained keybinds, catch-all key, key remapping
- tmux: prefix key + command table pattern
- WezTerm: key tables (`key_tables` config), `ActivateKeyTable` action
- Zellij: modal input (Normal, Pane, Tab, Resize, Move, Search, Session, Scroll modes)

**Why this matters:** Power users want tmux-like prefix-key workflows without running tmux. Key tables let you define "press Ctrl+B, then..." sequences. Chained bindings reduce keystrokes. Key remapping helps users with non-standard keyboard layouts or preferences (e.g., swapping Ctrl and Super).

---

## 49.1 Key Tables

Named sets of keybindings that can be activated and deactivated. When a key table is active, its bindings take priority over the default table. This enables tmux-like prefix-key workflows.

**File:** `oriterm/src/keybindings/mod.rs`, `oriterm/src/keybindings/key_table.rs`

**Reference:** Ghostty key tables, WezTerm `key_tables`, Zellij modes

- [ ] `KeyTable` struct:
  - [ ] `name: String` — table identifier (e.g., `"prefix"`, `"resize"`, `"copy_mode"`)
  - [ ] `bindings: HashMap<KeyBinding, Action>` — the table's key-to-action map
  - [ ] `timeout: Option<Duration>` — auto-deactivate after timeout (e.g., 2 seconds for prefix tables)
  - [ ] `on_timeout: Action` — action to run when timeout fires (usually `PopKeyTable`)
- [ ] `KeyTableStack`:
  - [ ] Stack of active key tables (last activated = highest priority)
  - [ ] `push(table_name)` — activate a table
  - [ ] `pop()` — deactivate the topmost table
  - [ ] `clear()` — deactivate all tables (return to default bindings)
- [ ] Key dispatch priority:
  - [ ] 1. Active key table stack (top to bottom)
  - [ ] 2. Default key table (Section 13 bindings)
  - [ ] 3. Forward to PTY
- [ ] Built-in actions:
  - [ ] `PushKeyTable { name: String }` — activate a named table
  - [ ] `PopKeyTable` — deactivate the current table
- [ ] Config:
  ```toml
  [key-table.prefix]
  timeout = "2s"
  "c" = "new_tab"
  "n" = "next_tab"
  "p" = "previous_tab"
  "d" = "pop_key_table"

  [keybindings]
  "ctrl+b" = "push_key_table:prefix"
  ```
- [ ] Visual indicator: when a key table is active, show indicator in tab bar or status area (table name)
- [ ] **Tests:**
  - [ ] Pushing a key table makes its bindings active
  - [ ] Popping returns to previous table
  - [ ] Timeout auto-pops the table
  - [ ] Default bindings work when no table is active
  - [ ] Key in active table overrides default binding

---

## 49.2 Chained Keybinds

Bind multiple actions to a single key press. Actions execute sequentially.

**File:** `oriterm/src/keybindings/mod.rs`

**Reference:** Ghostty `chain` key

- [ ] `Action::Chain(Vec<Action>)` variant:
  - [ ] Contains ordered list of actions to execute
  - [ ] Actions execute sequentially in order
  - [ ] If any action fails, remaining actions still execute (best-effort)
- [ ] Config syntax:
  ```toml
  [keybindings]
  "ctrl+shift+t" = "chain:new_tab,push_key_table:prefix"
  ```
- [ ] Use cases:
  - [ ] New tab + switch to it: `chain:new_tab,next_tab`
  - [ ] Copy + exit vi mode: `chain:copy,exit_vi_mode`
  - [ ] Split + focus new pane: `chain:split_horizontal,focus_next_pane`
- [ ] **Tests:**
  - [ ] Chain of two actions: both execute in order
  - [ ] Empty chain: no-op
  - [ ] Chain with failing action: other actions still run

---

## 49.3 Catch-All Key

A special binding that matches any key not bound in the current table. Enables "passthrough" and "block all" patterns.

**File:** `oriterm/src/keybindings/mod.rs`

**Reference:** Ghostty `catch_all` special key

- [ ] `KeyBinding::CatchAll` variant:
  - [ ] Matches any key that doesn't match a specific binding in the current table
  - [ ] Can have modifiers (e.g., `catch_all+ctrl` matches any Ctrl+key not otherwise bound)
- [ ] Primary use case: key tables that consume all input
  - [ ] Example: a "locked" mode where all keys are blocked:
    ```toml
    [key-table.locked]
    "escape" = "pop_key_table"
    "catch_all" = "ignore"
    ```
  - [ ] Example: a mode that forwards all unbound keys to PTY but has some overrides
- [ ] `Action::Ignore` — consume the key event, do nothing (prevent forwarding to PTY)
- [ ] **Tests:**
  - [ ] Catch-all matches unbound keys
  - [ ] Specific bindings take priority over catch-all
  - [ ] Catch-all with `ignore` action prevents PTY forwarding

---

## 49.4 Key Remapping

Remap key/modifier combinations at the terminal level before any binding lookup or PTY forwarding. This is lower-level than keybindings — it transforms the key event itself.

**File:** `oriterm/src/keybindings/remap.rs`

**Reference:** Ghostty `key-remap` config

- [ ] `KeyRemap` struct:
  - [ ] `from: KeyCombo` — the key combination to intercept
  - [ ] `to: KeyCombo` — the key combination to substitute
- [ ] Remap applied BEFORE keybinding lookup:
  - [ ] Physical key event → remap → remapped event → keybinding dispatch → PTY
- [ ] Common use cases:
  - [ ] `ctrl = super` — swap Ctrl and Super (for macOS users on Linux)
  - [ ] `caps_lock = escape` — Caps Lock as Escape (vim users)
  - [ ] `right_alt = compose` — right Alt for compose key sequences
- [ ] Config:
  ```toml
  [[key-remap]]
  from = "ctrl"
  to = "super"
  ```
- [ ] Modifier-only remaps: remap a modifier key itself, not just key+modifier combos
- [ ] **Tests:**
  - [ ] Remapped key triggers the mapped binding
  - [ ] Original key no longer triggers its old binding
  - [ ] Remap chain: A→B, B→C (should work, not infinite loop)
  - [ ] Identity remap: no-op

---

## 49.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 49.5 Section Completion

- [ ] All 49.1–49.4 items complete
- [ ] Key tables activate/deactivate with stack semantics
- [ ] Timeout auto-deactivates key tables
- [ ] Visual indicator shows active key table name
- [ ] Chained bindings execute multiple actions sequentially
- [ ] Catch-all matches unbound keys in a table
- [ ] Key remapping transforms events before binding lookup
- [ ] Config parsing handles all new binding types
- [ ] `cargo build --target x86_64-pc-windows-gnu` — clean build
- [ ] `cargo clippy --target x86_64-pc-windows-gnu` — no warnings
- [ ] `cargo test` — all tests pass

- [ ] `/tpr-review` passed — independent Codex review found no critical or major issues (or all findings triaged)

**Exit Criteria:** Power users can configure tmux-like prefix-key workflows, chain multiple actions per binding, create modal input modes with catch-all handling, and remap keys at the terminal level. The keybinding system is now as flexible as WezTerm's or Zellij's.
