---
section: 13
title: Configuration & Keybindings
status: complete
reviewed: true
last_verified: "2026-03-29"
tier: 3
goal: TOML configuration with file watching and hot reload, user-configurable keybindings with defaults
sections:
  - id: "13.1"
    title: Config Structs
    status: complete
  - id: "13.2"
    title: Config I/O
    status: complete
  - id: "13.3"
    title: Config File Watcher
    status: complete
  - id: "13.4"
    title: Config Hot Reload
    status: complete
  - id: "13.5"
    title: Keybinding System
    status: complete
  - id: "13.6"
    title: Default Keybindings
    status: complete
  - id: "13.7"
    title: Keybinding Config Parsing
    status: complete
  - id: "13.8"
    title: CLI Subcommands
    status: complete
  - id: "13.9"
    title: Shell Completion Scripts
    status: complete
  - id: "13.10"
    title: Section Completion
    status: complete
---

# Section 13: Configuration & Keybindings

**Status:** Complete
**Goal:** TOML-based configuration file with typed structs, file system watching for hot reload, and a user-configurable keybinding system with sensible defaults.

**Crate:** `oriterm` (binary)
**Dependencies:** `serde`, `toml`, `notify`
**Reference:** `_old/src/config/` (mod.rs, io.rs, monitor.rs, tests.rs), `_old/src/keybindings/` (mod.rs, defaults.rs, parse.rs, tests.rs)

**Prerequisite:** Section 04 (window + GPU — need running app to apply config changes)

---

## 13.1 Config Structs

Top-level config and per-section structs. All fields have defaults via `#[serde(default)]`.

**File:** `oriterm/src/config/mod.rs`

**Reference:** `_old/src/config/mod.rs`

- [x] `Config` struct (top-level) (verified 2026-03-29)
  - [x] `#[derive(Debug, Clone, Default, Serialize, Deserialize)]` (verified 2026-03-29)
  - [x] `#[serde(default)]` (verified 2026-03-29)
  - [x] Fields: (verified 2026-03-29)
    - `font: FontConfig`
    - `terminal: TerminalConfig`
    - `colors: ColorConfig`
    - `window: WindowConfig`
    - `behavior: BehaviorConfig`
    - `bell: BellConfig`
    - `keybind: Vec<KeybindConfig>` — user keybinding overrides
- [x] `FontConfig` struct <!-- unblocks:6.15 --><!-- unblocks:6.20 --> (verified 2026-03-29)
  - [x] Fields: (verified 2026-03-29)
    - `size: f32` — point size (default: from render::FONT_SIZE)
    - `family: Option<String>` — primary font family name
    - `weight: u16` — CSS font weight 100-900 (default: 400)
    - `tab_bar_font_weight: Option<u16>` — tab bar text weight (default: 600 via effective method)
    - `tab_bar_font_family: Option<String>` — tab bar font family (default: same as primary)
    - `features: Vec<String>` — OpenType features (default: `["calt", "liga"]`)
    - `fallback: Vec<FallbackFontConfig>` — ordered fallback font list
  - [x] `effective_weight(&self) -> u16` — clamped to [100, 900] (verified 2026-03-29)
  - [x] `effective_bold_weight(&self) -> u16` — `min(900, weight + 300)` (CSS "bolder") (verified 2026-03-29)
  - [x] `effective_tab_bar_weight(&self) -> u16` — clamped, defaults to 600 (verified 2026-03-29)
- [x] `FallbackFontConfig` struct (verified 2026-03-29)
  - [x] Fields: (verified 2026-03-29)
    - `family: String` — font family name or absolute path
    - `features: Option<Vec<String>>` — per-fallback OpenType feature overrides
    - `size_offset: Option<f32>` — point size adjustment relative to primary
- [x] `TerminalConfig` struct (verified 2026-03-29)
  - [x] Fields: (verified 2026-03-29)
    - `shell: Option<String>` — override shell (default: system shell)
    - `scrollback: usize` — scrollback lines (default: 10_000)
    - `cursor_style: String` — "block", "bar"/"beam", "underline" (default: "block")
    - `cursor_blink: bool` — enable cursor blinking (default: true)
    - `cursor_blink_interval_ms: u64` — blink interval (default: 530)
- [x] `ColorConfig` struct <!-- unblocks:3.7 --> (verified 2026-03-29)
  - [x] Fields: (verified 2026-03-29)
    - `scheme: String` — color scheme name (default: "Catppuccin Mocha")
    - `minimum_contrast: f32` — WCAG 2.0 contrast ratio 1.0-21.0 (default: 1.0 = off)
    - `alpha_blending: AlphaBlending` — text alpha blending mode
    - `foreground: Option<String>` — override fg color "#RRGGBB"
    - `background: Option<String>` — override bg color "#RRGGBB"
    - `cursor: Option<String>` — override cursor color "#RRGGBB"
    - `selection_foreground: Option<String>` — override selection fg
    - `selection_background: Option<String>` — override selection bg
    - `ansi: HashMap<String, String>` — override ANSI colors 0-7 by index
    - `bright: HashMap<String, String>` — override bright colors 8-15 by index
  - [x] `effective_minimum_contrast(&self) -> f32` — clamped to [1.0, 21.0] (verified 2026-03-29)
- [x] `AlphaBlending` enum — `Linear`, `LinearCorrected` (default) (verified 2026-03-29)
- [x] `WindowConfig` struct <!-- unblocks:3.6 --> (verified 2026-03-29)
  - [x] Fields: (verified 2026-03-29)
    - `columns: usize` — initial terminal columns (default: 120)
    - `rows: usize` — initial terminal rows (default: 30)
    - `opacity: f32` — window opacity 0.0-1.0 (default: 1.0)
    - `tab_bar_opacity: Option<f32>` — independent tab bar opacity (falls back to opacity)
    - `blur: bool` — enable backdrop blur (default: true)
    - `decorations: Decorations` — window decoration mode (default: `None` for frameless CSD)
    - `resize_increments: bool` — snap resize to cell boundaries (default: false)
  - [x] `effective_opacity(&self) -> f32` — clamped to [0.0, 1.0] (verified 2026-03-29)
  - [x] `effective_tab_bar_opacity(&self) -> f32` — clamped, falls back to opacity when None (verified 2026-03-29)
- [x] `Decorations` enum (verified 2026-03-29)
  - [x] `Full` — OS-native title bar and borders (verified 2026-03-29)
  - [x] `None` — frameless window with custom CSD (default) (verified 2026-03-29)
  - [x] On Windows/Linux: maps to `with_decorations(bool)` in winit (verified 2026-03-29)
  - [x] macOS extends with `Transparent` (transparent titlebar) and `Buttonless` (hide traffic lights) via winit macOS extensions (verified 2026-03-29)
  - [x] **Ref:** Alacritty `config/window.rs:183-189`, winit `WindowAttributes::with_decorations`
- [x] `BehaviorConfig` struct <!-- unblocks:9.6 --> (verified 2026-03-29)
  - [x] Fields: (verified 2026-03-29)
    - `copy_on_select: bool` — auto-copy on selection release (default: true)
    - `bold_is_bright: bool` — bold text uses bright colors (default: true)
    - `shell_integration: bool` — enable shell integration injection (default: true)
- [x] `BellConfig` struct (verified 2026-03-29)
  - [x] Fields: (verified 2026-03-29)
    - `animation: String` — "ease_out", "linear", "none" (default: "ease_out")
    - `duration_ms: u16` — flash duration, 0 = disabled (default: 150)
    - `color: Option<String>` — flash color "#RRGGBB" (default: white)
  - [x] `is_enabled(&self) -> bool` — `duration_ms > 0 && animation != "none"` (verified 2026-03-29)

---

## 13.2 Config I/O

Path resolution, loading, saving, and cursor style parsing.

**File:** `oriterm/src/config/io.rs`

**Reference:** `_old/src/config/io.rs`

- [x] `config_dir() -> PathBuf` (verified 2026-03-29)
  - [x] Windows: `%APPDATA%/ori_term` (verified 2026-03-29)
  - [x] Linux: `$XDG_CONFIG_HOME/ori_term` or `~/.config/ori_term` (verified 2026-03-29)
  - [x] Fallback: `./ori_term` (verified 2026-03-29)
- [x] `config_path() -> PathBuf` — `config_dir().join("config.toml")` (verified 2026-03-29)
- [x] `state_path() -> PathBuf` — `config_dir().join("state.toml")` (window geometry persistence) (verified 2026-03-29)
- [x] `WindowState` struct — `{ x: i32, y: i32, width: u32, height: u32 }` (verified 2026-03-29)
  - [x] `WindowState::load() -> Option<Self>` — read from state.toml, None on missing/invalid (verified 2026-03-29)
  - [x] `WindowState::save(&self)` — write to state.toml, create dir if needed (verified 2026-03-29)
- [x] `Config::load() -> Self` (verified 2026-03-29)
  - [x] Read from `config_path()` (verified 2026-03-29)
  - [x] `NotFound`: return defaults (first run) (verified 2026-03-29)
  - [x] Parse error: log warning, return defaults (verified 2026-03-29)
  - [x] Success: log path, return parsed config (verified 2026-03-29)
- [x] `Config::try_load() -> Result<Self, String>` (verified 2026-03-29)
  - [x] Preserves error distinction (file missing vs parse error) (verified 2026-03-29)
  - [x] Used by hot reload: parse error keeps previous config (verified 2026-03-29)
- [x] `Config::save(&self)` — serialize to TOML, write to config_path (verified 2026-03-29)
- [x] `parse_cursor_style(s: &str) -> CursorShape` (verified 2026-03-29 — implemented as CursorStyle enum with serde, strictly better)
  - [x] "block" | "Block" -> Block (verified 2026-03-29)
  - [x] "bar" | "beam" -> Beam (verified 2026-03-29)
  - [x] "underline" -> Underline (verified 2026-03-29)
  - [x] Unknown -> Block (default) (verified 2026-03-29)
- [x] `save_toml(value, path, label)` — private helper: serialize, create dirs, write (verified 2026-03-29)
- [x] **Tests** (`oriterm/src/config/tests.rs`): (verified 2026-03-29 — 157 tests pass)
  - [x] Default config roundtrip: serialize then deserialize equals defaults (verified 2026-03-29)
  - [x] Partial TOML uses defaults for missing fields (verified 2026-03-29)
  - [x] Empty TOML gives full defaults (verified 2026-03-29)
  - [x] Cursor style parsing: all variants (verified 2026-03-29)
  - [x] Opacity clamping: values outside [0.0, 1.0] clamped (verified 2026-03-29)
  - [x] Minimum contrast clamping: values outside [1.0, 21.0] clamped (verified 2026-03-29)
  - [x] Color overrides roundtrip: foreground, background, cursor, selection (verified 2026-03-29)
  - [x] ANSI color overrides: per-index overrides, unset indices remain None (verified 2026-03-29)
  - [x] Font weight: defaults, clamping, bold derivation (verified 2026-03-29)
  - [x] Tab bar opacity: independent from window opacity, falls back when None (verified 2026-03-29)
  - [x] Alpha blending: defaults to LinearCorrected, parses from TOML (verified 2026-03-29)
  - [x] Config dir is non-empty, config path ends with .toml (verified 2026-03-29)

---

## 13.3 Config File Watcher

Watch the config file for changes and send reload events through the event loop.

**File:** `oriterm/src/config/monitor.rs`

**Reference:** `_old/src/config/monitor.rs`

- [x] `ConfigMonitor` struct (verified 2026-03-29)
  - [x] Fields: (verified 2026-03-29)
    - `shutdown_tx: mpsc::Sender<()>` — signal to stop watcher thread
    - `thread: Option<JoinHandle<()>>` — watcher thread handle
- [x] `ConfigMonitor::new(proxy: EventLoopProxy<TermEvent>) -> Option<Self>` (verified 2026-03-29 — takes `Arc<dyn Fn() + Send + Sync>` per impl-hygiene, strictly better)
  - [x] Get config file path and parent directory (verified 2026-03-29)
  - [x] If parent doesn't exist: return None (no config dir yet) (verified 2026-03-29)
  - [x] Create `notify::recommended_watcher` watching parent directory (NonRecursive) (verified 2026-03-29)
  - [x] Spawn watcher thread with name "config-watcher" (verified 2026-03-29)
- [x] `ConfigMonitor::watch_loop(...)` — private, runs on watcher thread (verified 2026-03-29)
  - [x] Loop on `notify_rx.recv()`: (verified 2026-03-29)
    - [x] Check shutdown signal before processing (verified 2026-03-29)
    - [x] Filter: only process events for the config file path (ignore other files in dir) (verified 2026-03-29)
    - [x] Debounce: drain events within 200ms window (editors save in multiple steps) (verified 2026-03-29)
    - [x] Check shutdown again after debounce (verified 2026-03-29)
    - [x] Send `TermEvent::ConfigReload` through event loop proxy (verified 2026-03-29)
    - [x] If proxy send fails: event loop closed, exit (verified 2026-03-29)
- [x] `ConfigMonitor::shutdown(mut self)` (verified 2026-03-29 — implemented via Drop trait, RAII pattern)
  - [x] Send shutdown signal (verified 2026-03-29)
  - [x] Join watcher thread (verified 2026-03-29)
- [x] Watcher keeps `_watcher` alive for thread lifetime (dropped on exit) (verified 2026-03-29)

---

## 13.4 Config Hot Reload

Apply config changes to the running application without restart.

**File:** `oriterm/src/app/config_reload.rs`

**Reference:** `_old/src/app/config_reload.rs`

- [x] On `TermEvent::ConfigReload`: (verified 2026-03-29)
  - [x] Call `Config::try_load()` — on error: log warning, keep previous config (verified 2026-03-29)
  - [x] Compare new config against current config (verified 2026-03-29)
  - [x] Apply deltas: (verified 2026-03-29)
    - [x] Font change (family, size, weight, features, fallback): rebuild FontCollection, clear glyph atlas, recompute cell metrics, resize all tabs/grids (verified 2026-03-29)
    - [x] Color change (scheme, overrides): rebuild palette, request redraw (verified 2026-03-29)
    - [x] Window change (opacity, blur): update window transparency/blur settings (verified 2026-03-29)
    - [x] Behavior change: update behavior flags (verified 2026-03-29)
    - [x] Bell change: update bell config (verified 2026-03-29)
    - [x] Keybinding change: rebuild merged keybinding table (verified 2026-03-29)
  - [x] Broadcast changes to ALL tabs in ALL windows (font metrics affect every grid) (verified 2026-03-29)
  - [x] Request redraw for all windows (verified 2026-03-29)

---

## 13.5 Keybinding System <!-- unblocks:8.3 -->

Map key + modifiers to application actions. Linear scan with O(1) expected-case lookup.

**File:** `oriterm/src/keybindings/mod.rs`

**Reference:** `_old/src/keybindings/mod.rs`

- [x] `BindingKey` enum — key identifier independent of modifiers (verified 2026-03-29)
  - [x] `Named(NamedKey)` — named keys (Tab, PageUp, F1, etc.) (verified 2026-03-29)
  - [x] `Character(String)` — always stored lowercase (verified 2026-03-29)
  - [x] Derive: `Debug`, `Clone`, `PartialEq`, `Eq`, `Hash` (verified 2026-03-29)
- [x] `Action` enum — what to do when a keybinding matches (verified 2026-03-29)
  - [x] Variants: `Copy`, `Paste`, `SmartCopy`, `SmartPaste`, `NewTab`, `CloseTab`, `NextTab`, `PrevTab`, `ZoomIn`, `ZoomOut`, `ZoomReset`, `ScrollPageUp`, `ScrollPageDown`, `ScrollToTop`, `ScrollToBottom`, `OpenSearch`, `ReloadConfig`, `PreviousPrompt`, `NextPrompt`, `DuplicateTab`, `MoveTabToNewWindow`, `ToggleFullscreen`, `SendText(String)`, `None` (verified 2026-03-29)
  - [x] `SmartCopy`: copy if selection exists, else fall through to PTY (Ctrl+C sends SIGINT) (verified 2026-03-29)
  - [x] `SmartPaste`: paste from clipboard (Ctrl+V without Shift) (verified 2026-03-29)
  - [x] `SendText(String)`: send literal bytes to PTY (supports escape sequences) (verified 2026-03-29)
  - [x] `None`: explicitly unbinds a default binding (verified 2026-03-29)
  - [x] Derive: `Debug`, `Clone`, `PartialEq`, `Eq` (verified 2026-03-29)
- [x] `KeyBinding` struct — `{ key: BindingKey, mods: Modifiers, action: Action }` (verified 2026-03-29)
  - [x] Derive: `Debug`, `Clone` (verified 2026-03-29)
- [x] `KeybindConfig` struct — TOML-serializable entry (verified 2026-03-29)
  - [x] `{ key: String, mods: String, action: String }` (verified 2026-03-29)
  - [x] Derive: `Debug`, `Clone`, `Serialize`, `Deserialize` (verified 2026-03-29)
- [x] `key_to_binding_key(key: &Key) -> Option<BindingKey>` (verified 2026-03-29)
  - [x] Convert winit `Key` to `BindingKey`, normalizing characters to lowercase (verified 2026-03-29)
- [x] `find_binding(bindings: &[KeyBinding], key: &BindingKey, mods: Modifiers) -> Option<&Action>` (verified 2026-03-29)
  - [x] Linear scan: first match wins (verified 2026-03-29)
  - [x] More-specific modifier combos come first in the list (Ctrl+Shift+C before Ctrl+C) (verified 2026-03-29)

---

## 13.6 Default Keybindings

Built-in default keybindings. User bindings override these.

**File:** `oriterm/src/keybindings/defaults.rs`

**Reference:** `_old/src/keybindings/defaults.rs`

- [x] `default_bindings() -> Vec<KeyBinding>` (verified 2026-03-29)
  - [x] Ordering: more-specific modifier combos first (Ctrl+Shift before Ctrl) (verified 2026-03-29)
- [x] Default table: (verified 2026-03-29)
  - [x] `Ctrl+Shift+C` -> Copy (verified 2026-03-29)
  - [x] `Ctrl+Shift+V` -> Paste (verified 2026-03-29)
  - [x] `Ctrl+Insert` -> Copy (verified 2026-03-29)
  - [x] `Shift+Insert` -> Paste (verified 2026-03-29)
  - [x] `Ctrl+Shift+R` -> ReloadConfig (verified 2026-03-29)
  - [x] `Ctrl+Shift+F` -> OpenSearch (verified 2026-03-29)
  - [x] `Ctrl+=` / `Ctrl++` -> ZoomIn (verified 2026-03-29)
  - [x] `Ctrl+-` -> ZoomOut (verified 2026-03-29)
  - [x] `Ctrl+0` -> ZoomReset (verified 2026-03-29)
  - [x] `Ctrl+T` -> NewTab (verified 2026-03-29)
  - [x] `Ctrl+W` -> CloseTab (verified 2026-03-29)
  - [x] `Ctrl+Tab` -> NextTab (verified 2026-03-29)
  - [x] `Ctrl+Shift+Tab` -> PrevTab (verified 2026-03-29)
  - [x] `Shift+PageUp` -> ScrollPageUp (verified 2026-03-29)
  - [x] `Shift+PageDown` -> ScrollPageDown (verified 2026-03-29)
  - [x] `Shift+Home` -> ScrollToTop (verified 2026-03-29)
  - [x] `Shift+End` -> ScrollToBottom (verified 2026-03-29)
  - [x] `Ctrl+Shift+ArrowUp` -> PreviousPrompt (verified 2026-03-29)
  - [x] `Ctrl+Shift+ArrowDown` -> NextPrompt (verified 2026-03-29)
  - [x] `Alt+Enter` -> ToggleFullscreen (Windows/Linux), `Ctrl+Cmd+F` -> ToggleFullscreen (macOS) (verified 2026-03-29)
  - [x] `Ctrl+C` -> SmartCopy (must come AFTER Ctrl+Shift+C) (verified 2026-03-29)
  - [x] `Ctrl+V` -> SmartPaste (must come AFTER Ctrl+Shift+V) (verified 2026-03-29)

---

## 13.7 Keybinding Config Parsing

Parse keybinding entries from TOML and merge with defaults.

**File:** `oriterm/src/keybindings/parse.rs`

**Reference:** `_old/src/keybindings/parse.rs`

- [x] `merge_bindings(user: &[KeybindConfig]) -> Vec<KeyBinding>` (verified 2026-03-29)
  - [x] Start with `default_bindings()` (verified 2026-03-29)
  - [x] For each user entry: (verified 2026-03-29)
    - [x] Parse key and mods (log warning on unknown) (verified 2026-03-29)
    - [x] Parse action (log warning on unknown) (verified 2026-03-29)
    - [x] Remove any existing binding with same (key, mods) — retain filter (verified 2026-03-29)
    - [x] If action is `None`: unbind only (don't add replacement) (verified 2026-03-29)
    - [x] Otherwise: push new binding (verified 2026-03-29)
  - [x] Returns merged binding list (verified 2026-03-29)
- [x] `parse_key(s: &str) -> Option<BindingKey>` (verified 2026-03-29)
  - [x] Named keys: Tab, PageUp, PageDown, Home, End, Insert, Delete, Escape, Enter, Backspace, Space, ArrowUp, ArrowDown, ArrowLeft, ArrowRight, F1-F24 (verified 2026-03-29)
  - [x] Single characters: lowercased (verified 2026-03-29)
- [x] `parse_mods(s: &str) -> Modifiers` (verified 2026-03-29)
  - [x] Pipe-separated: "Ctrl|Shift", "Alt", "Super" (verified 2026-03-29)
  - [x] Empty string or "None": no modifiers (verified 2026-03-29)
- [x] `parse_action(s: &str) -> Option<Action>` (verified 2026-03-29)
  - [x] Direct match for each Action variant name (verified 2026-03-29)
  - [x] Special: `"SendText:..."` prefix → `Action::SendText(unescape_send_text(text))` (verified 2026-03-29)
- [x] `unescape_send_text(s: &str) -> String` — process escape sequences (verified 2026-03-29)
  - [x] `\x1b` -> ESC, `\n` -> newline, `\r` -> CR, `\t` -> tab, `\\` -> backslash (verified 2026-03-29)
  - [x] `\xHH` -> hex byte (verified 2026-03-29)
- [x] **Tests** (`oriterm/src/keybindings/tests.rs`): (verified 2026-03-29 — 59 tests pass)
  - [x] Default bindings: Ctrl+Shift+C maps to Copy (verified 2026-03-29)
  - [x] Merge: user binding overrides default (verified 2026-03-29)
  - [x] Merge: Action::None removes default binding (verified 2026-03-29)
  - [x] Parse key: named keys, single chars, unknown returns None (verified 2026-03-29)
  - [x] Parse mods: "Ctrl|Shift" -> CONTROL | SHIFT (verified 2026-03-29)
  - [x] Parse action: all variants, SendText with escapes (verified 2026-03-29)
  - [x] Unescape: `\x1b` -> '\x1b', `\n` -> '\n', `\\` -> '\\' (verified 2026-03-29)
  - [x] SmartCopy/SmartPaste resolved correctly after Ctrl+Shift variants (verified 2026-03-29)

---

## 13.8 CLI Subcommands

Utility subcommands for font discovery, keybinding reference, config validation, and theme browsing — diagnostic tools every terminal ships.

**File:** `oriterm/src/cli.rs` (clap subcommands)

**Reference:** Alacritty `alacritty msg`, Ghostty `ghostty +list-fonts`, WezTerm `wezterm ls-fonts`

- [x] `oriterm ls-fonts` — list discovered fonts with fallback chain: (verified 2026-03-29)
  - [x] Show primary font family + all 4 style variants (Regular/Bold/Italic/BoldItalic) (verified 2026-03-29)
  - [x] Show fallback chain in priority order (verified 2026-03-29)
  - [x] For each face: family name, style, file path, format (TrueType/OpenType), variable axes (verified 2026-03-29)
  - [x] `--codepoint <char>` — show which font resolves a specific character (verified 2026-03-29)
  - [x] Output: plain text, one font per line (verified 2026-03-29)
- [x] `oriterm show-keys` — dump current keybindings: (verified 2026-03-29)
  - [x] Load config, merge defaults with user overrides (verified 2026-03-29)
  - [x] Show all active bindings: `Ctrl+Shift+C -> Copy`, etc. (verified 2026-03-29)
  - [x] `--default` — show only default bindings (ignore user config) (verified 2026-03-29)
  - [x] Group by category (clipboard, tabs, navigation, etc.) (verified 2026-03-29)
- [x] `oriterm list-themes` — browse available color schemes: (verified 2026-03-29)
  - [x] List all built-in themes by name (verified 2026-03-29)
  - [x] List user-defined themes from config directory (verified 2026-03-29)
  - [x] `--preview` — show ANSI color preview for each theme (16-color palette sample) (verified 2026-03-29)
- [x] `oriterm validate-config` — check config without launching: (verified 2026-03-29)
  - [x] Parse config file, report errors with line numbers (verified 2026-03-29)
  - [x] Validate font families exist on system (verified 2026-03-29)
  - [x] Validate color values parse correctly (verified 2026-03-29)
  - [x] Validate keybinding key names and action names (verified 2026-03-29)
  - [x] Exit 0 on valid, exit 1 on errors (verified 2026-03-29)
- [x] `oriterm show-config` — dump resolved config: (verified 2026-03-29)
  - [x] Load config with all defaults filled in (verified 2026-03-29)
  - [x] Serialize to TOML and print (verified 2026-03-29)
  - [x] Shows effective config (defaults + user overrides merged) (verified 2026-03-29)
- [x] Subcommand dispatch: all subcommands run without opening a window (headless) (verified 2026-03-29)
- [x] **Tests:** (verified 2026-03-29 — 40 CLI tests pass)
  - [x] `validate-config` on valid config returns exit 0 (verified 2026-03-29)
  - [x] `validate-config` on invalid TOML returns exit 1 with error message (verified 2026-03-29)
  - [x] `show-config` output is valid TOML that can be re-parsed (verified 2026-03-29)
  - [x] `ls-fonts` includes primary font family (verified 2026-03-29)

---

## 13.9 Shell Completion Scripts

Generate shell completion scripts for bash, zsh, fish, and PowerShell.

**File:** `oriterm/src/cli.rs` (clap `generate` integration)

**Reference:** WezTerm `wezterm shell-completion`, clap `clap_complete` crate

- [x] Add `clap_complete` dependency (verified 2026-03-29)
- [x] `oriterm completions <shell>` subcommand: (verified 2026-03-29)
  - [x] `oriterm completions bash` — output bash completion script (verified 2026-03-29)
  - [x] `oriterm completions zsh` — output zsh completion script (verified 2026-03-29)
  - [x] `oriterm completions fish` — output fish completion script (verified 2026-03-29)
  - [x] `oriterm completions powershell` — output PowerShell completion script (verified 2026-03-29)
  - [x] Output to stdout (user redirects to appropriate file) (verified 2026-03-29)
- [x] Completions cover: all subcommands, `--config`, `--working-directory`, `--shell`, etc. (verified 2026-03-29)
- [x] Install instructions printed when run without redirection (verified 2026-03-29)
- [x] **Tests:** (verified 2026-03-29 — 5 completion tests pass)
  - [x] Each shell variant produces non-empty output (verified 2026-03-29)
  - [x] Output contains expected subcommand names (verified 2026-03-29)

---

## 13.10 Section Completion

- [x] All 13.1-13.9 items complete (verified 2026-03-29)
- [x] `cargo test -p oriterm` — config and keybinding tests pass (verified 2026-03-29 — 256 tests pass: 157 config + 59 keybindings + 40 CLI)
- [x] `cargo clippy -p oriterm --target x86_64-pc-windows-gnu` — no warnings (verified 2026-03-29)
- [x] Config loads from TOML file on startup (defaults if missing) (verified 2026-03-29)
- [x] Partial TOML fills in defaults for unspecified fields (verified 2026-03-29)
- [x] Invalid TOML logs warning, uses defaults (no crash) (verified 2026-03-29)
- [x] Config file watcher detects changes with 200ms debounce (verified 2026-03-29)
- [x] Hot reload applies font, color, window, behavior, bell, keybinding changes (verified 2026-03-29)
- [x] Font change triggers atlas rebuild + grid resize (verified 2026-03-29)
- [x] Default keybindings work out of the box (verified 2026-03-29)
- [x] User keybindings override defaults (verified 2026-03-29)
- [x] `Action::None` unbinds a default binding (verified 2026-03-29)
- [x] `SendText` action sends literal bytes (with escape sequences) to PTY (verified 2026-03-29)
- [x] Window state (geometry) persisted separately from user config (verified 2026-03-29)

**Exit Criteria:** Config system loads, saves, and hot-reloads without interrupting the terminal session. Keybindings are user-configurable via TOML with sensible defaults. Invalid config never crashes the app.
