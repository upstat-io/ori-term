# Section 49 Verification: Advanced Keybinding System

## Status: NOT STARTED (confirmed)

### Evidence Search

**No key table, chained keybinding, catch-all, or key remap types exist** anywhere in the codebase. Searching for `KeyTable`, `key_table`, `CatchAll`, `catch_all`, `KeyRemap`, `key_remap`, `leader_key`, `chained` yields zero matches in production code (only in plans and prior-art docs).

### Existing Keybinding Infrastructure (Section 13)

The current keybinding system is complete and functional but simple:

1. **`Action` enum** (`oriterm/src/keybindings/mod.rs`): 45+ action variants covering copy/paste, tabs, zoom, scroll, search, prompt navigation, mark mode, pane operations, settings. No `Chain`, `PushKeyTable`, `PopKeyTable`, or `Ignore` variants exist.

2. **`KeyBinding` struct**: Simple `{ key: BindingKey, mods: Modifiers, action: Action }`. No table association, no timeout, no catch-all support.

3. **`BindingKey` enum**: `Named(NamedKey)` or `Character(String)`. No `CatchAll` variant.

4. **`find_binding()`**: Linear scan of `&[KeyBinding]` matching (key, mods). No table stack, no priority layers.

5. **`merge_bindings()`**: Merges user TOML overrides with defaults. `Action::None` unbinds. No support for `chain:`, `push_key_table:`, or key table config sections.

6. **`parse_action()`**: String-to-Action mapping. Currently handles `SendText:...` prefix. Would need extension for `chain:...` and `push_key_table:...`.

7. **`default_bindings()`**: 45+ default bindings with platform-specific macOS section (`#[cfg(target_os = "macos")]` adds Cmd-based bindings). Clean, extensible pattern.

8. **Config**: `keybind: Vec<KeybindConfig>` in the top-level `Config`. `KeybindConfig` has `key`, `mods`, `action` string fields.

### TODOs/FIXMEs

No TODOs or FIXMEs found in `oriterm/src/keybindings/`.

### Gap Analysis

1. **Plan structure is sound**: The four subsections (key tables, chained binds, catch-all, remapping) are well-ordered. Key tables (49.1) are the largest and most architectural change; chained binds (49.2) and catch-all (49.3) build on the table infrastructure; remapping (49.4) is independent and could be done first.

2. **Action dispatch needs refactoring**: The current `find_binding()` is a simple linear scan. Adding key table priority requires replacing this with a stack-based lookup: check active table stack (top to bottom), then default table, then forward to PTY. The plan describes this correctly but underestimates the scope -- the dispatch logic in `oriterm/src/app/keyboard_input/action_dispatch.rs` (where `find_binding` is called) needs significant rework, not just `find_binding` replacement.

3. **Visual indicator gap**: Plan says "when a key table is active, show indicator in tab bar or status area (table name)" but doesn't specify how this integrates with the existing tab bar or status bar widgets. Section 01 (Tab Bar) has a status bar widget, which could host this. The plan should reference the specific widget.

4. **Config format concern**: The proposed TOML config uses `[key-table.prefix]` sections:
   ```toml
   [key-table.prefix]
   timeout = "2s"
   "c" = "new_tab"
   ```
   This mixes metadata (`timeout`) with keybindings (`"c" = "new_tab"`) in the same table. TOML parsers can handle this (the value type differs -- string vs. string), but serde deserialization will be awkward. Consider separating:
   ```toml
   [key-table.prefix]
   timeout = "2s"
   bindings = { c = "new_tab", n = "next_tab" }
   ```

5. **Remap scope ambiguity (49.4)**: The plan says "Modifier-only remaps: remap a modifier key itself, not just key+modifier combos." Modifier-only remapping (e.g., `Caps Lock -> Escape`) typically requires OS-level keyboard hooks, not application-level key event interception. winit delivers physical/logical key events, but modifier keys arrive as `ModifiersChanged` events, not `KeyboardInput`. The plan should clarify that remapping operates on logical key events, not physical keys, and that true modifier remapping (CapsLock, etc.) requires OS-level configuration.

6. **Remap chain safety (49.4)**: The plan mentions "Remap chain: A->B, B->C (should work, not infinite loop)" but doesn't specify the implementation. Applying remaps in a single pass (not recursively) prevents infinite loops. This should be explicitly stated.

7. **Missing: interaction with mark mode**: Mark mode (Section 40, vi copy mode) will also need key tables or modal input. The plan should note this as a natural consumer of the key table infrastructure, to avoid designing mark mode's key handling separately.

8. **Missing: interaction with overlays/dialogs**: The current keybinding dispatch already has complexity around dialog windows (`is_global()` on Action). Key tables add another dimension -- should key tables be active in dialog windows? Probably not (dialogs have their own key handling), but this should be stated.

9. **Good**: The reference to Ghostty 1.3.0, WezTerm key_tables, and Zellij modal input is appropriate and covers the design space well.

10. **Good**: The `KeyTableStack` with push/pop/clear semantics is the right abstraction.

### Infrastructure from Other Sections

- **Section 13 (Config & Keybindings)**: Complete. Provides the `Action` enum, `KeyBinding` struct, `find_binding()`, `merge_bindings()`, and config parsing. All of these need extension, not replacement.
- **Section 40 (Vi Copy Mode)**: Not started. Would be a natural consumer of key tables (vi mode is a key table that captures all input until Escape).
- **Section 27 (Command Palette)**: Not started. Palette could dispatch key table actions.

### Verdict

**CONFIRMED NOT STARTED**. No key tables, chained binds, catch-all, or remapping infrastructure exists. The plan is well-structured but should address:
- Config format for key tables (separate metadata from bindings)
- Modifier-only remapping limitations (OS-level vs. application-level)
- Remap chain evaluation strategy (single-pass, not recursive)
- Interaction with mark mode and dialog/overlay key handling
- Specific widget for key table visual indicator
- The dispatch refactoring scope in `action_dispatch.rs`
