---
section: "03"
title: "Opacity-Aware Subpixel Disable"
status: complete
reviewed: true
goal: "Automatically disable subpixel rendering when terminal opacity < 1.0"
depends_on: []
sections:
  - id: "03.1"
    title: "Wire for_display into Config Resolution"
    status: complete
  - id: "03.2"
    title: "Runtime Opacity Changes"
    status: complete
  - id: "03.3"
    title: "Completion Checklist"
    status: complete
---

# Section 03: Opacity-Aware Subpixel Disable

**Status:** Not Started
**Goal:** When the terminal's background opacity is less than 1.0 (transparent/
glass/acrylic), subpixel rendering is automatically disabled and grayscale alpha
rendering is used instead. This prevents color fringing artifacts that occur when
per-channel LCD compositing assumes an opaque background that doesn't exist.

**Context:** `SubpixelMode::for_display(scale_factor, opacity)` already exists
(in `oriterm/src/font/mod.rs:208`) and correctly returns `SubpixelMode::None`
when `opacity < 1.0`. However, it is marked with `#[allow(dead_code)]`
(font/mod.rs:204–206) — the actual config resolution in
`resolve_subpixel_mode()` (`oriterm/src/app/config_reload.rs:390`) only calls
`SubpixelMode::from_scale_factor()`, ignoring opacity entirely.

This is a latent bug that becomes visible once Section 02 activates bg-hint
compositing: transparent windows would pass a bg color with alpha < 1.0 to the
shader, but `push_glyph_with_bg` hardcodes `bg.a = 1.0`, creating a mismatch
between the assumed and actual background.

**Depends on:** None (can be done in parallel with Section 01).

---

## 03.1 Wire for_display into Config Resolution

**File(s):** `oriterm/src/app/config_reload.rs`

- [x] Update `resolve_subpixel_mode()` to accept an `opacity: f64` parameter:
  ```rust
  pub(crate) fn resolve_subpixel_mode(
      config: &FontConfig,
      scale_factor: f64,
      opacity: f64,
  ) -> SubpixelMode {
      match config.subpixel_mode.as_deref() {
          Some("rgb") => SubpixelMode::Rgb,
          Some("bgr") => SubpixelMode::Bgr,
          Some("none") => SubpixelMode::None,
          Some(other) => {
              log::warn!("config: unknown subpixel_mode {other:?}, using auto-detection");
              SubpixelMode::for_display(scale_factor, opacity)
          }
          None => SubpixelMode::for_display(scale_factor, opacity),
      }
  }
  ```

- [x] Update all call sites of `resolve_subpixel_mode()` to pass the current
  opacity value. There are 5 callers:
  1. `oriterm/src/app/config_reload.rs:122` — config reload path
  2. `oriterm/src/app/init/mod.rs:92` — initial window creation
  3. `oriterm/src/app/window_management.rs:189` — new window creation
  4. `oriterm/src/app/mod.rs:323` — DPI scale change handling
  5. `oriterm/src/app/dialog_management.rs:344` — dialog window creation
  Each caller needs access to the current window opacity
  (`self.config.window.effective_opacity()` or equivalent). Note: dialog
  windows (caller 5) may use a different opacity or always be opaque —
  verify the correct opacity source for that path.

- [x] Remove the `#[allow(dead_code, ...)]` attribute from
  `SubpixelMode::for_display()` at `oriterm/src/font/mod.rs:204–206`
  since it is now used.

- [x] When the user explicitly requests "rgb" or "bgr" subpixel mode via config,
  respect that even with opacity < 1.0 — explicit config overrides auto-detection.
  Log a warning: `"config: subpixel rendering with transparent background may cause color fringing"`.

- [x] Update all existing `resolve_subpixel_mode` tests in
  `oriterm/src/config/tests.rs:1456–1518` to pass an `opacity` argument.
  There are 5 tests that must be updated:
  1. `resolve_subpixel_mode_config_override_rgb` (line 1456) — pass `1.0`
  2. `resolve_subpixel_mode_config_override_bgr` (line 1466) — pass `1.0`
  3. `resolve_subpixel_mode_config_override_none` (line 1476) — pass `1.0`
  4. `resolve_subpixel_mode_auto_detection` (line 1486) — pass `1.0`
  5. `resolve_subpixel_mode_unknown_value_falls_back` (line 1505) — pass `1.0`
  These tests should preserve their existing behavior (opacity 1.0 doesn't
  change auto-detection). Add new tests for the opacity parameter separately
  (see Section 05.1).

---

## 03.2 Runtime Opacity Changes

**File(s):** Callers of `resolve_subpixel_mode()`

When the user changes opacity at runtime (e.g., via settings panel or config
reload), the subpixel mode must be recalculated. This may require:

- [x] Verify that config reload already recalculates subpixel mode. The config
  reload path calls `resolve_subpixel_mode()` with the new opacity, and the
  font collection is rebuilt when opacity crosses the 1.0 threshold (detected
  via `opacity_affects_subpixel` check in `apply_font_changes`).

- [x] If subpixel mode changes (e.g., from `Rgb` to `None` because opacity
  dropped below 1.0), the glyph atlas is invalidated because `apply_font_changes`
  rebuilds the font collection with the new glyph format. The
  `FontCollection::new()` call uses the updated format, and
  `renderer.replace_font_collection()` clears the atlas.

- [x] Test scenario: start with opacity 1.0 (subpixel enabled), change to
  opacity 0.8 (subpixel should auto-disable), change back to 1.0 (subpixel
  should re-enable). Verified via `resolve_subpixel_mode_transparent_disables_auto`
  test and `opacity_affects_subpixel` threshold detection in `apply_font_changes`.

---

### Cleanup

- [x] **[BLOAT]** `config_reload.rs` — Extracted `apply_color_overrides` and `build_palette_from_config` into `config_reload/color_config.rs`. Module converted to directory (`config_reload/mod.rs` at 429 lines + `color_config.rs` at 91 lines).
- [x] **[STYLE]** `config/tests.rs` — Removed decorative `// -----------` banners around `resolve_subpixel_mode` tests. Replaced with plain `// Section name` comments.

---

## 03.3 Completion Checklist

- [x] `resolve_subpixel_mode()` uses `SubpixelMode::for_display(scale, opacity)`
- [x] `SubpixelMode::for_display()` no longer marked `dead_code`
- [x] All 5 call sites pass opacity parameter
- [x] All 5 existing `resolve_subpixel_mode` tests updated for 3-arg signature
- [x] Explicit config override ("rgb"/"bgr") works with opacity < 1.0 (with warning)
- [x] Opacity change at runtime recalculates subpixel mode
- [x] Atlas invalidation on subpixel mode change works correctly
- [x] `./clippy-all.sh` green
- [x] `./build-all.sh` green
- [x] `./test-all.sh` green

**Exit Criteria:** Setting terminal opacity to 0.8 automatically disables
subpixel rendering. Returning to opacity 1.0 re-enables it. No color fringing
visible with transparent backgrounds. Explicit "rgb"/"bgr" config override
forces subpixel on regardless of opacity (power user escape hatch).
