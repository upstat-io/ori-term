---
section: "01"
title: "Window Chrome Platform Gate"
status: complete
goal: "macOS shows native traffic lights; Windows/Linux show custom control buttons; no visual duplication on any platform"
inspired_by:
  - "Alacritty: no custom chrome — uses OS decorations"
  - "WezTerm: custom tab bar with platform-aware control buttons"
  - "Ghostty: native title bar on macOS, custom on other platforms"
depends_on: []
sections:
  - id: "01.1"
    title: "Gate draw_window_controls on Platform"
    status: complete
  - id: "01.2"
    title: "Remove Controls Array on macOS"
    status: complete
  - id: "01.2b"
    title: "Gate Downstream Call Sites"
    status: complete
  - id: "01.3"
    title: "Completion Checklist"
    status: complete
---

# Section 01: Window Chrome Platform Gate

**Status:** Complete
**Goal:** macOS renders native traffic lights (provided by the OS via `fullsize_content_view(true)`) with no custom window control buttons drawn. Windows and Linux continue to draw the three custom control buttons (minimize, maximize/restore, close). No duplicate controls on any platform.

**Context:** The `TabBarWidget::draw()` method unconditionally calls `self.draw_window_controls(ctx)` (line 461 of `oriterm_ui/src/widgets/tab_bar/widget/draw.rs`), which renders three Windows-style rectangular control buttons. On macOS, the OS already provides native traffic light buttons via `NSFullSizeContentViewWindowMask`. The result is duplicate controls — native traffic lights AND custom rectangles — with the custom ones obscuring the native ones.

**Reference implementations:**
- **WezTerm** `wezterm-gui/src/tabbar.rs`: Platform-aware tab bar — skips drawing custom buttons on macOS, relies on `titlebar_appears_transparent` for native traffic lights.
- **Ghostty** `src/apprt/`: Uses platform-specific apprts — GTK header bar on Linux (`gtk.zig`), native AppKit title bar on macOS (`apprt/embedded.zig` + `macos/` objc bridge) — never draws custom window controls on macOS.

**Depends on:** None.

---

## 01.1 Gate draw_window_controls on Platform

**File(s):** `oriterm_ui/src/widgets/tab_bar/widget/draw.rs`, `oriterm_ui/src/widgets/tab_bar/widget/controls_draw.rs`

On macOS, the OS draws traffic light buttons automatically when `fullsize_content_view(true)` is set. The tab bar correctly reserves space via `left_inset` (set to `MACOS_TRAFFIC_LIGHT_WIDTH = 76px`). The only fix needed is to not draw the custom controls.

- [x] In `draw.rs` line 461, wrap `self.draw_window_controls(ctx)` with `#[cfg(not(target_os = "macos"))]`. Note: `draw.rs` is 477 lines, so this single inline `#[cfg]` on a call site is acceptable. The full impl blocks in `controls_draw.rs` and `control_state.rs` are gated at block level.
  ```rust
  // In Widget::draw() for TabBarWidget:
  self.draw_separators(ctx, &strip);
  self.draw_new_tab_button(ctx, &strip);
  self.draw_dropdown_button(ctx, &strip);
  #[cfg(not(target_os = "macos"))]
  self.draw_window_controls(ctx);
  ```

- [x] Add `#[cfg(not(target_os = "macos"))]` to the impl blocks in `controls_draw.rs` (`draw_window_controls` at lines 19-49, `control_rect` at lines 51-65) to suppress dead code warnings on macOS.

- [x] Verify `control_rect()` in `controls_draw.rs` line 56 — used by `interactive_rects()` for hit testing. Since hit testing for controls is also unconditional, gate it similarly (covered by gating the impl block).

---

## 01.2 Remove Controls Array on macOS

**File(s):** `oriterm_ui/src/widgets/tab_bar/widget/mod.rs`, `oriterm_ui/src/widgets/tab_bar/widget/control_state.rs`

The `controls: [WindowControlButton; 3]` field and `hovered_control: Option<usize>` field are allocated on all platforms. On macOS they serve no purpose.

- [x] Gate the `controls` and `hovered_control` fields (lines 118-120 of `widget/mod.rs`) with `#[cfg(not(target_os = "macos"))]`
  ```rust
  pub struct TabBarWidget {
      // ...
      #[cfg(not(target_os = "macos"))]
      controls: [WindowControlButton; 3],
      #[cfg(not(target_os = "macos"))]
      hovered_control: Option<usize>,
      // ...
  }
  ```

- [x] Gate the `controls` field initialization in **both** constructor paths: `with_theme()` at lines 135-136 and `apply_theme()` at lines 164-167 in `widget/mod.rs`. The `with_theme` constructor builds and stores `controls`; `apply_theme` iterates `self.controls` to call `set_colors()`. Both must be inside `#[cfg(not(target_os = "macos"))]` blocks.

- [x] Gate `set_maximized()` (lines 15-23), `update_control_hover()` (lines 67-115), `clear_control_hover()` (lines 118-129), and `handle_control_mouse()` (lines 135-172) in `control_state.rs` — these methods route hover/click events to the control buttons and are dead code on macOS.

- [x] Gate the control button entries in `interactive_rects()` (control_state.rs lines 60-63: `for i in 0..3 { rects.push(self.control_rect(i)); }`) — only the `for` loop, not the entire method. These must be excluded on macOS since the OS handles traffic light hit testing natively.

- [x] Gate `control_rect()` in `controls_draw.rs` (lines 51-65) — it is called by `interactive_rects()` and `update_control_hover()`, both of which are gated. Without this, `control_rect` becomes dead code on macOS.

- [x] Gate `create_controls()` and `control_colors_from_theme()` free functions in `widget/mod.rs` (lines 428-444) with `#[cfg(not(target_os = "macos"))]` — they construct `WindowControlButton` instances which are unused on macOS.

- [x] Gate the imports that become unused on macOS: `WindowControlButton` and `ControlButtonColors` (line 22 of `widget/mod.rs`) and `ControlKind` (line 23). Use `#[cfg(not(target_os = "macos"))]` on the import lines or move them into a `cfg`-gated block.

- [x] Adjust `Vec::with_capacity` in `interactive_rects()` — currently allocates `self.tabs.len() + 5` (3 for controls + 2 for new-tab/dropdown). On macOS, only need `+ 2`. Use a cfg-conditional capacity:
  ```rust
  #[cfg(target_os = "macos")]
  let extra = 2;
  #[cfg(not(target_os = "macos"))]
  let extra = 5;
  let mut rects = Vec::with_capacity(self.tabs.len() + extra);
  ```

---

## 01.2b Gate Downstream Call Sites

**File(s):** `oriterm/src/app/chrome/mod.rs`, `oriterm/src/app/tab_bar_input.rs`, `oriterm_ui/src/widgets/tab_bar/tests.rs`

When the `set_maximized`, `update_control_hover`, `clear_control_hover`, and `handle_control_mouse` methods are gated with `#[cfg(not(target_os = "macos"))]` on the impl block in `control_state.rs`, all call sites must also be gated. Failure to do so causes compile errors on macOS ("method not found"). This must be done in the same step as 01.2 to keep the build passing at every commit.

- [x] **`oriterm/src/app/chrome/mod.rs` line 128**: Gate `ctx.tab_bar.set_maximized(maximized)` with `#[cfg(not(target_os = "macos"))]`. Note: `ctx.window.window().set_maximized()` (winit) and `ctx.window.set_maximized()` (TermWindow) are NOT gated — those are platform-independent. Only the tab bar widget method is gated.

- [x] **`oriterm/src/app/chrome/mod.rs` lines 209-252**: Gate the entire `update_control_hover_animation()` method with `#[cfg(not(target_os = "macos"))]`. This private method is called from `handle_tab_bar_interaction()` (line 193) — that call site must also be gated.

- [x] **`oriterm/src/app/chrome/mod.rs` lines 271-283**: Gate the control-hover-clearing block inside `clear_tab_bar_hover()` with `#[cfg(not(target_os = "macos"))]`. The rest of `clear_tab_bar_hover` (tab hover clearing) stays unconditional.

- [x] **`oriterm/src/app/tab_bar_input.rs` lines 200-234**: Gate the `route_control_mouse()` method with `#[cfg(not(target_os = "macos"))]`. Also gate its call sites at line 67 (`route_control_mouse(Left, false)`) and line 113 (`route_control_mouse(Left, true)`).

- [x] **`oriterm/src/app/tab_bar_input.rs` lines 110-115**: Gate the `Minimize | Maximize | CloseWindow` match arm with `#[cfg(not(target_os = "macos"))]` — these hit types never occur when controls are removed from `interactive_rects`.

- [x] **`oriterm_ui/src/widgets/tab_bar/tests.rs`**: Gate tests that call gated methods or assert control-button behavior with `#[cfg(not(target_os = "macos"))]`. Specific tests:
  - `interactive_rects_count_equals_tab_count_plus_five` (line 1216) — asserts `len == tabs + 5`; on macOS it would be `tabs + 2`
  - `interactive_rects_buttons_and_controls_at_correct_positions` (line 1266) — asserts `rects[4..7]` are control buttons; would panic on macOS
  - `interactive_rects_with_left_inset_shifts_tabs_not_controls` (line 1307) — same issue
  - `set_maximized_does_not_panic` (line 1336)
  - `update_control_hover_enters_and_leaves` (line 1392)
  - Any other tests that directly invoke gated methods or assert control rect presence

**Sync points — downstream call sites:**
| File | Line(s) | Method called | Gate needed |
|------|---------|---------------|-------------|
| `chrome/mod.rs` | 128 | `tab_bar.set_maximized()` | `#[cfg(not(target_os = "macos"))]` |
| `chrome/mod.rs` | 193 | `update_control_hover_animation()` call | `#[cfg(not(target_os = "macos"))]` |
| `chrome/mod.rs` | 209-252 | `update_control_hover_animation()` def | `#[cfg(not(target_os = "macos"))]` |
| `chrome/mod.rs` | 271-283 | `tab_bar.clear_control_hover()` | `#[cfg(not(target_os = "macos"))]` |
| `tab_bar_input.rs` | 200-234 | `route_control_mouse()` def | `#[cfg(not(target_os = "macos"))]` |
| `tab_bar_input.rs` | 67 | `route_control_mouse(Left, false)` call | `#[cfg(not(target_os = "macos"))]` |
| `tab_bar_input.rs` | 113 | `route_control_mouse(Left, true)` call | `#[cfg(not(target_os = "macos"))]` |
| `tab_bar_input.rs` | 110-115 | `Minimize \| Maximize \| CloseWindow` match arm | `#[cfg(not(target_os = "macos"))]` |
| `tab_bar/tests.rs` | 1216 | `interactive_rects_count_equals_tab_count_plus_five` | `#[cfg(not(target_os = "macos"))]` |
| `tab_bar/tests.rs` | 1266 | `interactive_rects_buttons_and_controls_at_correct_positions` | `#[cfg(not(target_os = "macos"))]` |
| `tab_bar/tests.rs` | 1307 | `interactive_rects_with_left_inset_shifts_tabs_not_controls` | `#[cfg(not(target_os = "macos"))]` |
| `tab_bar/tests.rs` | 1336 | `set_maximized_does_not_panic` | `#[cfg(not(target_os = "macos"))]` |
| `tab_bar/tests.rs` | 1392 | `update_control_hover_enters_and_leaves` | `#[cfg(not(target_os = "macos"))]` |

---

## 01.3 Completion Checklist

- [x] `cargo build` succeeds on macOS (native target) with no warnings about dead code
- [x] `./build-all.sh` succeeds (all targets)
- [x] `./clippy-all.sh` passes
- [x] `./test-all.sh` passes
- [x] Visual: macOS shows native traffic lights, no custom rectangular buttons
- [x] Visual: Windows shows three custom control buttons (minimize, maximize, close)
- [x] No `#[allow(dead_code)]` added — code is properly `#[cfg]`-gated
- [x] All downstream call sites in `chrome/mod.rs` and `tab_bar_input.rs` are gated
- [x] Tests in `tab_bar/tests.rs` that invoke gated methods are gated

**Sync points — ALL locations that must be gated together:**
| Location | What to gate |
|----------|-------------|
| `widget/mod.rs` lines 118-120 | `controls` and `hovered_control` fields |
| `widget/mod.rs` lines 135-136 | `controls` and `hovered_control` initialization in `with_theme()` |
| `widget/mod.rs` lines 164-167 | `self.controls` iteration in `apply_theme()` |
| `widget/mod.rs` lines 22-23 | `WindowControlButton`, `ControlButtonColors`, `ControlKind` imports |
| `widget/mod.rs` lines 428-444 | `control_colors_from_theme()` and `create_controls()` free functions |
| `controls_draw.rs` lines 19-49 | `draw_window_controls()` impl block |
| `controls_draw.rs` lines 51-65 | `control_rect()` impl block |
| `control_state.rs` lines 15-23 | `set_maximized()` |
| `control_state.rs` lines 60-63 | Control button `for i in 0..3` loop inside `interactive_rects()` (lines 42-65) |
| `control_state.rs` lines 67-115 | `update_control_hover()` |
| `control_state.rs` lines 118-129 | `clear_control_hover()` |
| `control_state.rs` lines 135-172 | `handle_control_mouse()` |
| `draw.rs` line 461 | `self.draw_window_controls(ctx)` call |
| `chrome/mod.rs` line 128 | `ctx.tab_bar.set_maximized(maximized)` call |
| `chrome/mod.rs` lines 193+209-252 | `update_control_hover_animation()` call + def |
| `chrome/mod.rs` lines 271-283 | `ctx.tab_bar.clear_control_hover()` block |
| `tab_bar_input.rs` line 67 | `route_control_mouse(Left, false)` call |
| `tab_bar_input.rs` line 113 | `route_control_mouse(Left, true)` call |
| `tab_bar_input.rs` lines 110-115 | `Minimize \| Maximize \| CloseWindow` match arm |
| `tab_bar_input.rs` lines 200-234 | `route_control_mouse()` def |
| `tab_bar/tests.rs` line 1216 | `interactive_rects_count_equals_tab_count_plus_five` test |
| `tab_bar/tests.rs` line 1266 | `interactive_rects_buttons_and_controls_at_correct_positions` test |
| `tab_bar/tests.rs` line 1307 | `interactive_rects_with_left_inset_shifts_tabs_not_controls` test |
| `tab_bar/tests.rs` line 1336 | `set_maximized_does_not_panic` test |
| `tab_bar/tests.rs` line 1392 | `update_control_hover_enters_and_leaves` test |

**Exit Criteria:** On macOS, the tab bar renders with native traffic lights and no custom control buttons. On Windows/Linux, the tab bar renders with three custom control buttons in the controls zone. Zero dead code warnings. All tests pass on all platforms.
