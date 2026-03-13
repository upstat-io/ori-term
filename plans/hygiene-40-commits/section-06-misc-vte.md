---
section: "06"
title: "oriterm misc — Config/Keybindings/WindowManager/VTE"
status: complete
goal: "Split VTE ansi.rs, tighten config visibility, fix type safety, and extract inline tests"
depends_on: []
sections:
  - id: "06.1"
    title: "DRIFTs — Fix Allocation on Keystroke and Scattered Platform Code"
    status: complete
  - id: "06.2"
    title: "GAPs — Fix Type Safety, Missing Fallbacks, and Error Handling"
    status: complete
  - id: "06.3"
    title: "WASTEs — Deduplicate Platform Helpers"
    status: complete
  - id: "06.4"
    title: "EXPOSUREs — Tighten Visibility in Binary Crate"
    status: complete
  - id: "06.5"
    title: "BLOATs — Split VTE ansi.rs and Extract Inline Tests"
    status: complete
  - id: "06.6"
    title: "Completion Checklist"
    status: complete
---

# Section 06: oriterm misc — Config/Keybindings/WindowManager/VTE

**Status:** Complete
**Goal:** VTE `ansi.rs` split from 2686 lines into focused modules. VTE `lib.rs` inline tests extracted to sibling `tests.rs`. All config types `pub(crate)` in binary crate. `MoveTabToNewWindow` uses `TabId`. Key binding lookup avoids per-keystroke allocation.

**Context:** This section covers findings spread across the config, keybindings, window_manager, platform, and VTE crate. The VTE crate's `ansi.rs` at 2686 lines is the single largest hygiene violation in the project — it predates the 500-line rule and needs a systematic split. The binary crate (`oriterm`) has many `pub` types that should be `pub(crate)` since they're never consumed externally.

---

## 06.1 DRIFTs — Fix Allocation on Keystroke and Scattered Platform Code

**File(s):** `oriterm/src/keybindings/mod.rs`, `oriterm/src/main.rs`, `oriterm/src/config/mod.rs`

- [x] **Finding 14**: No change needed — `key_to_binding_key` produces a short string (~3-5 chars) from an enum discriminant. The "allocation" is a small stack-like String used as a HashMap key. Pre-hashing would require a parallel enum-to-hash registry that adds complexity without measurable benefit. The keybinding lookup is not a hot path (once per keystroke, not per frame or per cell).

- [x] **Finding 15**: No change needed — `main.rs` has only 2 small `#[cfg]` blocks (Windows DPI awareness + console attach). Extracting these into a `platform::startup` module would create a new module with two 3-line functions. The current inline approach is clearer and follows the pattern used by Alacritty's `main.rs`.

- [x] **Finding 16**: Done — Added SI unit comments (`// 320 MB (SI, not MiB)`) to memory limit constants in `config/mod.rs`.

- [x] **Finding 17**: No change needed — `image_gpu_memory_limit` is intentionally omitted from the config struct. GPU texture memory is managed by wgpu's device limits, not by application-level configuration. Adding a user-facing config for GPU memory would require wiring through the rendering pipeline with no practical benefit.

---

## 06.2 GAPs — Fix Type Safety, Missing Fallbacks, and Error Handling

**File(s):** `oriterm/src/event.rs`, `oriterm/src/platform/memory.rs`, `oriterm/src/window_manager/platform/linux.rs`

- [x] **Finding 11**: Done — Changed `MoveTabToNewWindow(usize)` to `MoveTabToNewWindow(TabId)`. Updated all construction sites (`action_dispatch.rs`, `overlay_dispatch.rs`, `tab_bar_input.rs`) and pattern matches (`event_loop.rs`, `move_ops.rs`). Context menu also updated to carry `TabId` alongside display index.

- [x] **Finding 12**: Done — Added `#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]` fallback in `platform/memory.rs` that returns `None` for RSS on unsupported platforms.

- [x] **Finding 13**: Done — Improved `intern_atom` CString construction in `window_manager/platform/linux.rs` with explicit `.expect("hardcoded atom name is valid")` instead of bare `.unwrap()`. The actual X11 connection errors already use proper `?` propagation.

- [x] **Finding 25**: Done — Changed `unreachable!()` in VTE `change_state` (Ground state) to `debug_assert!(false, "change_state called in Ground state")`. This prevents terminal crashes from malformed state machine transitions in release builds.

---

## 06.3 WASTEs — Deduplicate Platform Helpers

**File(s):** `oriterm/src/window_manager/platform/macos/mod.rs`, `oriterm/src/window_manager/mod.rs`, `oriterm/src/window_manager/hierarchy.rs`

- [x] **Finding 19**: No change needed — The three traffic light functions are intentionally different. `center_traffic_lights()` takes `&Window` with physical `caption_height` and computes logical height via scale factor. `center_and_disable_drag_raw()` takes a raw `*mut AnyObject` for use from notification handlers (where no `&Window` is available) and uses `TAB_BAR_HEIGHT` directly. `reposition_buttons_raw()` intentionally skips container resize to avoid `_syncToolbarPosition` infinite recursion on macOS 26 (Tahoe) — the comment explicitly documents this divergence. Extracting a shared helper would lose these critical behavioral differences.

- [x] **Finding 20**: No change needed — Both `children.clone()` calls are necessary due to Rust's borrow checker. In `hierarchy.rs:91`, the clone breaks the immutable borrow of `self.windows` so that `collect_descendants(&mut self)` can take `&mut self`. In `mod.rs:80`, the clone extracts IDs so the returned `impl Iterator` can call `self.windows.get()` in `filter_map` without holding a borrow on the parent entry. This is the standard Rust pattern for tree traversal with mutable access.

---

## 06.4 EXPOSUREs — Tighten Visibility in Binary Crate

**File(s):** `oriterm/src/config/mod.rs`, `oriterm/src/config/font_config.rs`, `oriterm/src/config/color_config.rs`, `oriterm/src/config/paste_warning.rs`, `oriterm/src/config/bell.rs`, `oriterm/src/font/collection/mod.rs`, `oriterm/src/font/mod.rs`, `oriterm/src/window_manager/types.rs`, `oriterm/src/platform/mod.rs`

- [x] **Finding 3**: Done — 8 config types changed to `pub(crate)` in `config/mod.rs`.
- [x] **Finding 4**: Done — 3 types changed to `pub(crate)` in `config/font_config.rs`.
- [x] **Finding 5**: Done — 3 types changed to `pub(crate)` in `config/color_config.rs`.
- [x] **Finding 6**: Done — 1 type changed to `pub(crate)` in `config/paste_warning.rs`.
- [x] **Finding 7**: Done — 2 types changed to `pub(crate)` in `config/bell.rs`.
- [x] **Finding 8**: Done — `RasterizedGlyph`, `FontCollection`, `FontSet`, `size_key` changed to `pub(crate)` in `font/collection/mod.rs`. Re-exports in `font/mod.rs` also changed to `pub(crate) use`. Font submodules (`collection`, `discovery`, `shaper`) changed to `pub(crate) mod`. Shaper re-exports also tightened to `pub(crate) use`.
- [x] **Finding 9**: Done — `ManagedWindow` struct and all 5 fields changed from `pub` to `pub(crate)` in `window_manager/types.rs`.
- [x] **Finding 10**: Done — All 8 `pub mod` declarations changed to `pub(crate) mod` in `platform/mod.rs`.

---

## 06.5 BLOATs — Split VTE ansi.rs and Extract Inline Tests

**File(s):** `crates/vte/src/ansi/`, `crates/vte/src/lib.rs`, `crates/vte/src/tests.rs`

- [x] **Finding 1**: Done — Split `ansi.rs` (2686 lines) into 10 files, all source files under 500 lines:
  ```
  ansi/
  ├── mod.rs             (54 lines)   — module declarations, re-exports, shared constants
  ├── colors.rs          (209 lines)  — Hyperlink, Rgb, xparse_color, parse_number
  ├── processor.rs       (283 lines)  — Processor, Performer, ProcessorState, SyncState, StdSyncHandler, Timeout
  ├── handler.rs         (297 lines)  — Handler trait (70+ methods)
  ├── types.rs           (313 lines)  — KeyboardModes, ModifyOtherKeys, CursorStyle, CursorShape, Mode, PrivateMode
  ├── attr.rs            (355 lines)  — NamedColor, Color, Attr, CharsetIndex, StandardCharset, C0
  ├── dispatch/
  │   ├── mod.rs         (189 lines)  — impl Perform for Performer skeleton
  │   ├── osc.rs         (258 lines)  — OSC dispatch
  │   └── csi.rs         (390 lines)  — CSI dispatch + SGR parsing
  └── tests.rs           (512 lines)  — all tests (exempt from 500-line limit)
  ```
  Public API fully preserved via re-exports in `mod.rs`.

- [x] **Finding 2**: Done — Extracted 812 lines of inline tests from `lib.rs` to `crates/vte/src/tests.rs`. `lib.rs` now ends with `#[cfg(test)] mod tests;`. All 62 parser tests pass.

- [x] **Finding 18**: Covered by Finding 2.

---

## 06.6 Completion Checklist

- [x] `key_to_binding_key` — assessed, no change needed (not a hot path)
- [x] Platform startup — assessed, no change needed (only 2 small `#[cfg]` blocks)
- [x] Memory limit units documented
- [x] `image_gpu_memory_limit` — assessed, intentionally omitted (GPU memory managed by wgpu)
- [x] `MoveTabToNewWindow` uses `TabId` (not `usize`)
- [x] `platform/memory.rs` has non-Windows/macOS/Linux fallback
- [x] `intern_atom` uses explicit `.expect()` with descriptive message
- [x] VTE `change_state` uses `debug_assert!` (not `unreachable!`)
- [x] macOS traffic light centering — assessed, intentionally different functions
- [x] `children.clone()` — assessed, required by borrow checker
- [x] All config types are `pub(crate)` in binary crate
- [x] `RasterizedGlyph`/`FontCollection`/`FontSet`/`size_key` are `pub(crate)`
- [x] `ManagedWindow` fields are `pub(crate)`
- [x] `platform/mod.rs` uses `pub(crate) mod`
- [x] Font submodules and re-exports tightened to `pub(crate)`
- [x] VTE `ansi.rs` split into 10 focused modules (each under 500 lines)
- [x] VTE `lib.rs` inline tests extracted to `tests.rs`
- [x] `./test-all.sh` passes
- [x] `./clippy-all.sh` clean
- [x] `./build-all.sh` succeeds

**Exit Criteria Met:** VTE `ansi.rs` split into 10 modules (7 source + 3 dispatch) each under 500 lines. All binary-crate types `pub(crate)`. `MoveTabToNewWindow` type-safe with `TabId`. All verification passes green.
