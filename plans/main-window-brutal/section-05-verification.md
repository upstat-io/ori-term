---
section: "05"
title: "Verification"
status: not-started
reviewed: true
goal: "Verify all visual changes match the mockup, DPI scaling works at 96 and 192 DPI, cross-platform builds succeed, and all test suites pass."
inspired_by:
  - "plans/completed/ui-css-framework/section-14-verification.md (verification section pattern)"
depends_on: ["01", "02", "03", "04"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "05.1"
    title: "Test Matrix"
    status: not-started
  - id: "05.2"
    title: "DPI Scaling Verification"
    status: not-started
  - id: "05.3"
    title: "Build & Verify"
    status: not-started
  - id: "05.4"
    title: "Documentation"
    status: not-started
  - id: "05.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "05.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 05: Verification

**Status:** Not Started
**Goal:** Complete test matrix confirms all features work. DPI scaling verified at 96 and 192 DPI. All builds green on all platforms.

**Context:** This is the final verification pass after all components are implemented and wired together. It ensures no regressions, confirms DPI scaling, and validates the complete visual output against the mockup.

**Depends on:** Sections 01-04 (all prior work complete).

---

## 05.1 Test Matrix

Build a comprehensive test matrix covering every visual feature from the mockup.

- [ ] **Tab Bar Features:**
  - Flat tabs (zero radius) — covered by `tab_bar_brutal_3tabs_96dpi`
  - Active tab accent bar — covered by `tab_bar_brutal_3tabs_96dpi`
  - Active tab bottom bleed — covered by `tab_bar_brutal_3tabs_96dpi`
  - Tab bar bottom border — covered by `tab_bar_brutal_3tabs_96dpi`
  - Tab separators (1px right border) — covered by `tab_bar_brutal_3tabs_96dpi`
  - Modified dot indicator — covered by `tab_bar_brutal_3tabs_96dpi`
  - Close button 0.6 opacity on active — covered by `tab_bar_brutal_active_close_default_96dpi`
  - Close button 1.0 opacity on hover — covered by `tab_bar_brutal_hover_close_96dpi`
  - Tab action borders — covered by `tab_bar_brutal_actions_96dpi`
  - Single tab edge case — covered by `tab_bar_brutal_single_tab_96dpi`

- [ ] **Status Bar Features:**
  - Background and top border — covered by `status_bar_full_data_96dpi`
  - Accent items (shell, term type) — covered by `status_bar_full_data_96dpi`
  - Normal items (panes, grid, encoding) — covered by `status_bar_full_data_96dpi`
  - Empty items handling — covered by `status_bar_empty_items_96dpi`
  - WidgetTestHarness tests — 6 tests in `status_bar/tests.rs`

- [ ] **Split Pane Features:**
  - Divider default color — covered by unit test `divider_color_default` (data assertion, Section 03.4)
  - Divider hover color — covered by unit test `divider_color_hovered` (data assertion, Section 03.4)
  - Focus border inset — covered by unit test `focus_border_inset` (data assertion, Section 03.4)
  - Focus border color — covered by unit test `focus_border_accent_color` (data assertion, Section 03.4)
  - Window outer border — covered by composed golden tests (Section 04.4)

- [ ] **Composed Features:**
  - Full window chrome — covered by `main_window_single_pane_96dpi`
  - Multiple tabs — covered by `main_window_3tabs_96dpi`
  - DPI scaling — covered by `main_window_192dpi`

- [ ] **Config Tests:**
  - `show_status_bar` default true — covered by `status_bar_default_true`
  - `show_status_bar` TOML parsing — covered by `status_bar_toml_false`, `status_bar_toml_missing_defaults_true`
  - Updated divider defaults — covered by updated config/tests.rs assertions
  - Updated focus border color — covered by updated config/tests.rs assertions

- [ ] **Unit Tests (data layer):**
  - Tab bar color mappings — 5 tests (bar_bg, active_bg, separator, accent_bar, bar_border)
  - TabEntry.modified builder — 1 test
  - Divider hover logic — 4 tests (default, hovered, empty list, multiple)
  - Window border rects — 2 tests (positions, scaled)
  - Focus border — 2 tests (inset, color)
  - Chrome layout — 6 new tests (status bar, border inset, combinations)

- [ ] **Edge Cases to Verify Manually:**
  - Window at minimum size (50x100px) with status bar — does layout degrade gracefully?
  - Tab bar with 20+ tabs — do separators and modified dots render correctly?
  - Rapid window resize — no flicker or stale status bar position

### 05.1.1 Discovered Gaps

| Gap | Section | Test | Severity |
|-----|---------|------|----------|
| (fill in during verification) | | | |

---

## 05.2 DPI Scaling Verification

Verify all new visuals scale correctly at high DPI.

- [ ] **Tab bar at 192 DPI (scale=2.0)**: Physical dimensions double: 72px height, 4px accent bar, 4px bottom border, 28px padding
  - Run existing 96 DPI golden tests at 192 DPI (add `_192dpi` variants or parameterize)
  - Verify accent bar is 4px physical (2.0 logical * 2.0 scale), bottom border is 4px physical
  - Verify text renders at correct physical size (font_size_small * 2.0)
  - Note: the tab bar operates in logical pixels and is scaled by `renderer.append_ui_scene_with_text(scene, scale, ...)`. The scale factor is threaded through the `CachedTextMeasurer` and scene conversion.
- [ ] **Status bar at 192 DPI**: 44px physical height (22.0 * 2.0), 4px top border (2.0 * 2.0)
  - Add `status_bar_full_data_192dpi` golden test if not already present (use `headless_dialog_env_with_dpi(192.0)`)
- [ ] **Focus border at 192 DPI**: 4px physical border (2.0 * 2.0). Section 03.2 changes `append_focus_border()` to accept `border_width: f32` with the caller passing `(2.0 * scale).round()`. Verify at 192 DPI that the focus border is 4px physical (matching divider and window border scaling).
- [ ] **Window border at 192 DPI**: The `append_window_border()` implementation (Section 03.3) receives `(2.0 * scale).round()` as border_width, so at 192 DPI it's 4px physical. Correct.
- [ ] Verify no sub-pixel artifacts or blurry text at either DPI. The `grid_origin_y()` rounding (chrome/mod.rs line 85) prevents fractional pixel origins at any scale factor.
- [ ] **125% DPI (scale=1.25)**: This is the most common fractional DPI on Windows. Verify:
  - `grid_origin_y(36.0, 1.25)` = `(36.0 * 1.25).round()` = 45.0 (integer). OK.
  - Status bar height: `(22.0 * 1.25).round()` = 28.0 (integer). OK.
  - Window border: `(2.0 * 1.25).round()` = 3.0 (integer). OK.
  - All physical pixel values must be integers. No sub-pixel seams.
- [ ] **Add `chrome_layout_fractional_dpi_with_status_bar` test** to chrome/tests.rs: Call `compute_window_layout` at 1.25x with status bar. Verify all origin values have zero fractional part.

---

## 05.3 Build & Verify

- [ ] `./build-all.sh` green (all platforms: Windows cross-compile, host)
- [ ] `./clippy-all.sh` green (no warnings)
- [ ] `./test-all.sh` green (all tests pass)
- [ ] Architecture tests pass (`cargo test -p oriterm --test architecture`)
- [ ] Visual regression suite: `cargo test -p oriterm --features gpu-tests -- visual_regression` (all golden tests)
- [ ] Widget tests: `cargo test -p oriterm_ui` (all widget + harness tests)

---

## 05.4 Documentation

- [ ] Update CLAUDE.md if new constants or paths introduced
  - New: StatusBarWidget path
  - New: status bar height constant
  - Updated: tab bar height (36px, was 46px)
- [ ] Update any `.claude/rules/*.md` files if new patterns established
- [ ] Update `plans/bug-tracker/` if any bugs were found and fixed during verification
- [ ] Mark this plan as complete in `index.md` and `00-overview.md`

---

## 05.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 05.N Completion Checklist

- [ ] Test matrix covers all features from mockup (every CSS property has a corresponding test)
- [ ] DPI scaling verified at 96 and 192 DPI
- [ ] All golden tests pass (individual + composed)
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] Documentation updated
- [ ] Plan status updated to complete
- [ ] `/tpr-review` passed clean

**Exit Criteria:** The main terminal window renders identically to `mockups/main-window-brutal.html` at both 96 and 192 DPI. All test suites pass. All builds green. No visual regressions.
