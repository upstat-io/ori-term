---
section: "01"
title: "Tab Bar Brutal Styling"
status: not-started
reviewed: true
goal: "Transform the tab bar from Chrome-style rounded tabs to flat brutal design matching mockup/main-window-brutal.html pixel-for-pixel, with golden tests proving every visual change."
inspired_by:
  - "mockups/main-window-brutal.html (.tab, .tab.active, .tab-bar CSS)"
  - "plans/completed/ui-css-framework/ (per-element styling principle)"
  - "plans/brutal-design-pass-2/ (settings dialog brutal pass pattern)"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "01.1"
    title: "Tab Bar Metrics — 36px Flat"
    status: not-started
  - id: "01.2"
    title: "Flat Tab Drawing — No Radius"
    status: not-started
  - id: "01.3"
    title: "Active Tab Accent Bar & Bottom Bleed"
    status: not-started
  - id: "01.4"
    title: "Tab Bar Bottom Border & Separators"
    status: not-started
  - id: "01.5"
    title: "Modified Indicator & Close Button Opacity"
    status: not-started
  - id: "01.6"
    title: "Tab Action & Window Control Borders"
    status: not-started
  - id: "01.7"
    title: "Golden Tests"
    status: not-started
  - id: "01.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "01.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 01: Tab Bar Brutal Styling

**Status:** Not Started
**Goal:** The tab bar renders with 36px height, flat tabs (zero radius), 2px accent bar on active tab top, 2px bottom border, 1px separators, modified dot indicator, and border-left on action buttons — all matching `mockups/main-window-brutal.html` exactly. Golden tests lock every visual property.

**Context:** The current tab bar uses a Chrome-inspired design: 46px height with 8px top margin, rounded top corners (8px radius) on the active tab, thin line separators, and no bottom border. The brutal design mockup specifies a completely different aesthetic: flat, compact, accent-barred. This section transforms the tab bar rendering without changing the underlying widget architecture (layout, hit testing, drag/drop, animation all remain intact).

**WARNING: `draw.rs` is already 523 lines (over the 500-line limit).** Adding accent bar, bottom border, bottom bleed, modified dot, and separator rewrite code will push it further over. Before making changes to `draw.rs`, first extract code to bring it under 500 lines. Recommended extraction: move `draw_separators()`, `draw_new_tab_button()`, `draw_dropdown_button()`, and the free functions (`bell_phase`, `new_tab_button_x`, `dropdown_button_x`, `draw_icon`) into a new `draw_helpers.rs` submodule in the `widget/` directory. This should remove ~120 lines from `draw.rs`, leaving room for the new brutal drawing code. Add `mod draw_helpers;` (private) to `widget/mod.rs`.

**Reference implementations:**
- **mockup** `mockups/main-window-brutal.html:81-176`: Complete CSS for `.tab`, `.tab.active`, `.tab-bar`, accent bar via `::before`, bottom bleed via `::after`, modified indicator, close button opacity
- **brutal-design-pass-2**: Prior brutal pass on settings dialog — same mockup-first, per-element-styling approach

**Depends on:** None (independent).

---

## 01.1 Tab Bar Metrics — 36px Flat

**File(s):** `oriterm_ui/src/widgets/tab_bar/constants.rs`

The mockup specifies a 36px tab bar with no top margin, 14px internal padding, and 200px max tab width. The current DEFAULT metrics are 46/8/10/80/260. Replace them to match the mockup.

- [ ] Update `TAB_BAR_HEIGHT` from `46.0` to `36.0`
- [ ] Update `TAB_TOP_MARGIN` from `8.0` to `0.0` (tabs fill the full bar height)
- [ ] Update `TAB_PADDING` from `10.0` to `14.0` (mockup uses `padding: 0 14px`)
- [ ] Update `TAB_MAX_WIDTH` from `260.0` to `200.0` (mockup `max-width: 200px`)

- [ ] Update `TAB_LEFT_MARGIN` from `16.0` to `0.0` (mockup: no left margin, tabs start at edge). Note: this is a module-level `pub const` in `constants.rs`, NOT a field on `TabBarMetrics`. It is consumed by `TabBarLayout::compute()` — verify that changing it doesn't break the macOS traffic light inset logic (the `left_inset` field on `TabBarWidget` adds extra margin on macOS, separate from `TAB_LEFT_MARGIN`).
- [ ] Update `TabBarMetrics::DEFAULT` to reflect all new values. Note: `TabBarMetrics::DEFAULT` is a `const` that references the module constants (`TAB_BAR_HEIGHT`, `TAB_TOP_MARGIN`, `TAB_PADDING`, `TAB_MIN_WIDTH`, `TAB_MAX_WIDTH`), so updating the module constants automatically updates `DEFAULT`.
- [ ] Update `TabBarMetrics::COMPACT` proportionally — currently `height: 34, top_margin: 4, tab_padding: 6, min_width: 64, max_width: 220`. With the new DEFAULT at `height: 36`, COMPACT should be smaller (e.g. `height: 28, top_margin: 0, tab_padding: 10, max_width: 180`). Check for call sites of `COMPACT`: `metrics_from_style()` in `oriterm/src/app/init/mod.rs` uses it when `TabBarStyle::Compact` is configured. It is used in production — do not remove.
- [ ] Add constant: `TAB_BAR_BORDER_BOTTOM: f32 = 2.0`

- [ ] Verify `CONTROLS_ZONE_WIDTH` is still correct at the new 36px height. On Windows, `CONTROL_BUTTON_WIDTH` is 46px wide (from `window_chrome/constants.rs`). The window control buttons render within `strip.h` (tab bar height minus top margin). With the new metrics (`height=36, top_margin=0`), `strip.h = 36`, so control buttons are 46px wide x 36px tall. This is correct — the mockup shows `.win-btn { width: 46px }` at full bar height. The `draw_window_controls()` method in `controls_draw.rs` uses `strip.h` for button height. On non-Windows (Linux/macOS), `CONTROL_BUTTON_DIAMETER` is 24px which fits in 36px. No changes needed to control zone width.

- [ ] Update `oriterm_ui/src/widgets/tab_bar/slide/tests.rs` lines 13 and 180: change hardcoded `46.0` in `LayerTree::new(Rect::new(0.0, 0.0, 1200.0, 46.0))` to use `TAB_BAR_HEIGHT` (import from `super::super::constants::TAB_BAR_HEIGHT`). These are the only hardcoded `46.0` values in oriterm_ui tab bar tests — all other tests use the `TAB_BAR_HEIGHT` constant and will auto-update.
- [ ] Update `oriterm_ui/src/widgets/tab_bar/hit.rs` line 115: uses `super::constants::TAB_BAR_HEIGHT` — auto-updates, no change needed. Verify only.

**Validation:** `cargo test -p oriterm_ui` passes. Tab bar layout tests reflect new metrics.

---

## 01.2 Flat Tab Drawing — No Radius

**File(s):** `oriterm_ui/src/widgets/tab_bar/widget/draw.rs`, `oriterm_ui/src/widgets/tab_bar/widget/draw_helpers.rs` (NEW)

Remove all rounded corner logic. The brutal design uses zero radius everywhere.

- [ ] **Prerequisite: Extract `draw_helpers.rs`** to bring `draw.rs` under 500 lines. Move the following from `draw.rs` into `widget/draw_helpers.rs`:
  - `draw_separators()` (~30 lines)
  - `draw_new_tab_button()` (~18 lines)
  - `draw_dropdown_button()` (~22 lines)
  - Free functions: `bell_phase()`, `new_tab_button_x()`, `dropdown_button_x()`, `draw_icon()` (~60 lines)
  - Add `mod draw_helpers;` to `widget/mod.rs` (private submodule)
  - Re-export `bell_phase` and `draw_icon` as `pub(super)` from `draw_helpers.rs` for use by `draw.rs` and `drag_draw.rs`
  - Verify `cargo test -p oriterm_ui` and `./clippy-all.sh` pass after extraction
  - `draw.rs` should drop to ~390 lines, leaving ~110 lines of headroom for new brutal code
- [ ] Set `ACTIVE_TAB_RADIUS` to `0.0` in `draw.rs` line 31 (or remove the constant entirely). Currently `pub(super) const ACTIVE_TAB_RADIUS: f32 = 8.0;`
- [ ] Set `BUTTON_HOVER_RADIUS` to `0.0` in `draw.rs` line 34 (brutal: no radius on hover highlights). Currently `const BUTTON_HOVER_RADIUS: f32 = 4.0;`
- [ ] Simplify `draw_tab()` (draw.rs lines 82-91): replace the `if strip.active { RectStyle::filled(bg).with_per_corner_radius(ACTIVE_TAB_RADIUS, ACTIVE_TAB_RADIUS, 0.0, 0.0) } else { RectStyle::filled(bg) }` branch with a single `RectStyle::filled(bg)` for all tabs — active tab visual distinction comes from the accent bar (01.3), not from corner radius
- [ ] Update close button hover (draw.rs line 256): remove `.with_radius(BUTTON_HOVER_RADIUS)` from `RectStyle::filled(self.colors.button_hover_bg.with_alpha(opacity))`
- [ ] Update new-tab button hover (draw.rs line 353): remove `.with_radius(BUTTON_HOVER_RADIUS)` from `RectStyle::filled(self.colors.button_hover_bg)`
- [ ] Update dropdown button hover (draw.rs line 373): remove `.with_radius(BUTTON_HOVER_RADIUS)` from `RectStyle::filled(self.colors.button_hover_bg)`
- [ ] Update `drag_draw.rs` line 43: replace `RectStyle::filled(self.colors.active_bg).with_per_corner_radius(ACTIVE_TAB_RADIUS, ACTIVE_TAB_RADIUS, 0.0, 0.0)` with `RectStyle::filled(self.colors.active_bg)`. Also remove the `use super::draw::ACTIVE_TAB_RADIUS` import from `drag_draw.rs` line 12 (only keep the `TabStrip` and `CLOSE_ICON_INSET` imports)

**Validation:** Tab bar renders with perfectly square tabs and hover highlights. Dragged tab overlay is also flat.

---

## 01.3 Active Tab Accent Bar & Bottom Bleed

**File(s):** `oriterm_ui/src/widgets/tab_bar/widget/draw.rs`, `oriterm_ui/src/widgets/tab_bar/colors.rs`

The mockup's active tab has two visual signatures:
1. A 2px accent-colored bar at the **top edge** (`.tab.active::before`)
2. A 2px rect that **erases the bottom border** (`.tab.active::after`), making the active tab "bleed" into the content area

```css
/* From mockup */
.tab.active::before {
    content: '';
    position: absolute;
    top: -2px; left: 0; right: 0;
    height: 2px;
    background: var(--accent);
}
.tab.active::after {
    content: '';
    position: absolute;
    bottom: -2px; left: 0; right: 0;
    height: 2px;
    background: var(--bg-base);
}
```

- [ ] Add `accent_bar: Color` field to `TabBarColors` (set from `theme.accent` in `from_theme()`)
- [ ] Add `bar_border: Color` field to `TabBarColors` (set from `theme.border` in `from_theme()`)
- [ ] Fix the existing `active_bg` mapping in `TabBarColors::from_theme()`: change `active_bg: theme.bg_primary` to `active_bg: theme.bg_secondary`. Currently `active_bg` maps to `theme.bg_primary` (#16161c = `--bg-surface`), but the mockup specifies `.tab.active { background: var(--bg-base) }` = #0e0e12 = `theme.bg_secondary`. The bar background (`bar_bg`) and active tab background (`active_bg`) are currently swapped relative to the mockup. Fixing both in 01.4 (bar_bg) and here (active_bg) corrects the swap.
- [ ] The bottom bleed rect also uses `self.colors.active_bg` (after the fix above, this is #0e0e12 = `--bg-base`, which is correct for erasing the bottom border under the active tab)
- [ ] In `draw_tab()`, when `strip.active` (add these draws inside the existing `draw_tab` method, after the `ctx.scene.push_quad(tab_rect, style)` call at draw.rs line 106, before the content drawing):
  - Draw the accent bar: `Rect::new(tab_rect.x(), ctx.bounds.y(), tab_rect.width(), 2.0)` filled with `self.colors.accent_bar`. With `TAB_TOP_MARGIN = 0`, `strip.y == ctx.bounds.y()`, so this is at the very top of the tab bar. Using `ctx.bounds.y()` is correct — `paint()` sets `y0 = ctx.bounds.y()` and `strip.y = y0 + self.metrics.top_margin`. When `top_margin=0`, they are equal.
  - Draw the bottom bleed: The bottom border (from 01.4) is drawn at `y = y0 + self.metrics.height - TAB_BAR_BORDER_BOTTOM`. To exactly overlay it under the active tab, use `Rect::new(tab_rect.x(), ctx.bounds.y() + self.metrics.height - TAB_BAR_BORDER_BOTTOM, tab_rect.width(), TAB_BAR_BORDER_BOTTOM)` filled with `self.colors.active_bg`. Note: `tab_rect` x and width come from `self.layout.tab_x(index)` and `self.layout.tab_width_at(index)` — already computed for this tab.
- [ ] **Draw order within `paint()`**: The accent bar and bleed MUST draw in the correct z-order. The current `paint()` method (draw.rs lines 492-523) draws: (1) bar background, (2) tabs via `draw_all_tabs`, (3) separators, (4) new-tab button, (5) dropdown button, (6) window controls. The bottom border (01.4) should be drawn between steps 1 and 2 (so it's behind tabs). The bottom bleed is part of the active tab draw (step 2) and paints over the border. The accent bar is also part of step 2.
- [ ] Ensure the bottom bleed draws AFTER the bottom border (01.4) so it correctly erases it under the active tab. Since the border draws before tabs (step 1.5), and the bleed draws inside draw_tab() (step 2), this ordering is automatic.

**Validation:** Active tab shows a 2px accent line at top and seamlessly connects to the content area below.

---

## 01.4 Tab Bar Bottom Border & Separators

**File(s):** `oriterm_ui/src/widgets/tab_bar/widget/draw.rs`

The mockup has a 2px border at the bottom of the tab bar and uses 1px right-side borders on tabs (not centered separator lines between tabs).

```css
/* From mockup */
.tab-bar { border-bottom: 2px solid var(--border); }
.tab { border-right: 1px solid var(--border); }
```

- [ ] In `paint()` (draw.rs), add bottom border drawing AFTER the bar background (step 1, line 502) and BEFORE `draw_all_tabs` (step 2, line 514). Insert between them:
  ```rust
  // 1.5. Bottom border.
  let border_y = y0 + self.metrics.height - TAB_BAR_BORDER_BOTTOM;
  let border_rect = Rect::new(0.0, border_y, w, TAB_BAR_BORDER_BOTTOM);
  ctx.scene.push_quad(border_rect, RectStyle::filled(self.colors.bar_border));
  ```
  This requires importing `TAB_BAR_BORDER_BOTTOM` from `super::super::constants` into draw.rs.
- [ ] Rewrite `draw_separators()` (originally draw.rs lines 315-344, after 01.2 extraction: `draw_helpers.rs`) to draw a 1px right-border on each tab instead of centered inter-tab lines:
  - Currently iterates `1..self.tabs.len()` and draws a line at `self.layout.tab_x(i)` (left edge of each tab after the first), using `SEPARATOR_INSET` from top and bottom, with `self.colors.separator` (half-opacity border).
  - The mockup uses `border-right: 1px solid var(--border)` on each `.tab` — every tab has its own right border.
  - New logic: iterate `0..self.tabs.len()` and draw a 1px line at `self.layout.tab_x(i) + self.layout.tab_width_at(i)` (right edge of each tab). Use `Point::new(right_x, strip.y)` to `Point::new(right_x, strip.y + strip.h)` — full height, no inset.
  - Suppression rules: skip the active tab's right separator. Skip the separator when its tab index matches a hovered tab (adapt the existing `TabBarHit::Tab(h) | TabBarHit::CloseTab(h)` guard). Skip adjacent to dragged tabs (adapt `drag_visual` guard). Note: the suppression indices shift since we now iterate `0..len` on the tab itself rather than `1..len` on the gap between tabs.
  - Remove `SEPARATOR_INSET` constant (draw.rs line 46) — mockup separators are full-height
- [ ] Update `TabBarColors::from_theme()` in `colors.rs` line 47: change `separator: theme.border.with_alpha(0.5)` to `separator: theme.border` (full opacity, matching mockup)
- [ ] Fix `bar_bg` mapping in `TabBarColors::from_theme()` in `colors.rs` line 40: change `bar_bg: theme.bg_secondary` to `bar_bg: theme.bg_primary`. Currently `bar_bg` maps to `theme.bg_secondary` (#0e0e12 = `--bg-base`), but the mockup specifies `.tab-bar { background: var(--bg-surface) }` = #16161c = `theme.bg_primary`. The bar background and active tab background are currently swapped relative to the mockup.
- [ ] Also fix `inactive_bg` mapping in `colors.rs` line 42: change `inactive_bg: theme.bg_secondary` to `inactive_bg: theme.bg_primary`. Inactive tabs sit on the tab bar background — their base color should match the bar, not the active tab.
- [ ] The active tab's bottom bleed (from 01.3) must draw ON TOP of this bottom border to erase it under the active tab. This happens automatically: the border draws in step 1.5 (before tabs), the bleed draws inside `draw_tab()` (step 2, inside `draw_all_tabs`).

**Validation:** Tab bar has a visible 2px bottom border, with each tab showing a 1px right separator, and the active tab cleanly interrupts the bottom border.

---

## 01.5 Modified Indicator & Close Button Opacity

**File(s):** `oriterm_ui/src/widgets/tab_bar/widget/draw.rs`, `oriterm_ui/src/widgets/tab_bar/widget/mod.rs`

The mockup shows a 6px square accent-colored dot for modified tabs, and close button visibility follows specific rules.

```css
/* From mockup */
.tab-modified { width: 6px; height: 6px; background: var(--accent); }
.tab-close { opacity: 0; }
.tab:hover .tab-close { opacity: 1; }
.tab.active .tab-close { opacity: 0.6; }
```

- [ ] Add `modified: bool` field to `TabEntry` in `oriterm_ui/src/widgets/tab_bar/widget/mod.rs` line 63 (after `bell_start`). Default to `false` in `TabEntry::new()` constructor (line 67). Add builder method:
  ```rust
  #[must_use]
  pub fn with_modified(mut self, modified: bool) -> Self {
      self.modified = modified;
      self
  }
  ```
  Verify all existing `TabEntry::new(...)` call sites compile — since the new field has a default in the constructor and `TabEntry` is always constructed via `new()`, not struct literal, this is backward compatible. Check: `tab_bar_icons.rs`, `tab_bar/tests.rs`, and production code in `oriterm/src/app/tab_management/`.
- [ ] **Wire `modified` to production**: In `oriterm/src/app/tab_management/` (or wherever `TabEntry::new(...)` is called with production data), set `.with_modified(pane.is_modified())` or equivalent. Without this, the modified dot from 01.5 will never appear in practice. If no `is_modified()` API exists on pane snapshots yet, add `modified: false` and document the gap as a follow-up (the golden tests can still verify rendering by constructing `TabEntry::new("x").with_modified(true)`).
- [ ] In `draw_tab()` (draw.rs), when `tab.modified && !strip.active && !self.is_tab_hovered(index)`:
  - Draw a 6px x 6px filled rect with `self.colors.accent_bar`
  - Position: vertically centered in the tab, right-aligned in the close button zone. Use the same x position as the close button center: `cx = tab_x + tab_width - CLOSE_BUTTON_RIGHT_PAD - CLOSE_BUTTON_WIDTH / 2.0 - 3.0` (centered in close zone), `cy = strip.y + (strip.h - 6.0) / 2.0`
  - This replaces the close button in the modified+not-hovered state
- [ ] **Add `is_tab_hovered()` helper** to `TabBarWidget` (or inline the check): returns true when `hover_hit` matches `TabBarHit::Tab(index)` or `TabBarHit::CloseTab(index)`. This determines whether to show the close button or modified dot.
- [ ] Update close button opacity logic in `draw_tab()` (draw.rs lines 112-128) to match mockup:
  - Currently: active tab always draws close at `content_opacity` (1.0), inactive tabs use `close_btn_opacity` animation value.
  - New logic:
    - Active tab (not hovered): draw close at 0.6 opacity (change line 114 from `self.draw_close_button(ctx, index, x, strip, content_opacity)` to `self.draw_close_button(ctx, index, x, strip, 0.6 * content_opacity)`)
    - Active tab (hovered): draw close at 1.0 opacity
    - Inactive tab (not hovered): opacity 0 (hidden) — already the default from animation
    - Inactive tab (hovered): opacity 1.0 — already handled by animation
    - Close button hovered on any tab: opacity 1.0 + danger color highlight (existing behavior at line 255)
  - To detect active+hovered, check `self.hover_hit` matches `TabBarHit::Tab(index)` or `TabBarHit::CloseTab(index)` when `strip.active`.
- [ ] Verify existing `close_btn_opacity` animation (AnimProperty in widget/mod.rs line 119) still works with these new base values — the animation drives inactive tab close fade; active tab now uses a fixed 0.6/1.0 value instead of animation
- [ ] When a tab is modified AND hovered, hide the modified dot and show the close button (close replaces dot) — the `is_tab_hovered` guard handles this: modified dot only shows when not hovered

- [ ] **Add unit tests in `oriterm_ui/src/widgets/tab_bar/tests.rs`**:
  - **Test: `tab_entry_with_modified_builder`** — Verify `TabEntry::new("x").with_modified(true).modified == true` and default is `false`.
  - **Test: `modified_dot_not_shown_when_hovered`** — Construct a tab bar with a modified tab, set hover on that tab, verify that `is_tab_hovered(index)` returns true (so modified dot is suppressed). This tests the suppression logic.
  - **Test: `close_opacity_active_not_hovered`** — Verify the active tab's close button draws at 0.6 opacity when not hovered. (Exact verification depends on Scene inspection or golden tests.)

**Validation:** Modified tabs show a 6px square accent dot. Close button appears on hover. Active tab always shows close at 60% opacity.

---

## 01.6 Tab Action & Window Control Borders

**File(s):** `oriterm_ui/src/widgets/tab_bar/widget/controls_draw.rs`, `oriterm_ui/src/widgets/tab_bar/widget/draw.rs`

The mockup adds `border-left: 1px solid var(--border)` on each tab action button, and window control buttons use specific hover styling.

```css
/* From mockup */
.tab-action { border-left: 1px solid var(--border); }
.win-btn.close:hover { background: var(--danger); color: var(--text-bright); }
```

- [ ] In `draw_new_tab_button()` (originally draw.rs line 347, after 01.2 extraction: `draw_helpers.rs`), add a 1px left border BEFORE the hover highlight:
  ```rust
  ctx.scene.push_line(
      Point::new(bx, strip.y),
      Point::new(bx, strip.y + strip.h),
      1.0,
      self.colors.bar_border,
  );
  ```
  Note: `self.colors.bar_border` is the new field added in 01.3. This line draws at the left edge of the new-tab button regardless of hover state.
- [ ] In `draw_dropdown_button()` (originally draw.rs line 367, after 01.2 extraction: `draw_helpers.rs`), draw the same 1px left border at `bx`
- [ ] Verify window control button hover colors. Currently `control_colors_from_theme()` (widget/mod.rs line 406) sets `close_hover_bg: theme.close_hover_bg` = `#C42B1C` (Windows platform red). The mockup specifies `.win-btn.close:hover { background: var(--danger) }` = `#c87878`. These are different colors.
  - **Decision needed**: use the mockup's softer danger red for close hover, or keep Windows platform standard.
  - Recommend: change `close_hover_bg: theme.close_hover_bg` to `close_hover_bg: theme.danger` in `control_colors_from_theme()` (line 412). This makes the close button hover consistent with the brutal design. The `close_hover_bg` field on `UiTheme` can stay for platform-standard uses (e.g., settings dialog might want different behavior).
- [ ] Verify tab action button icons (+ and split) still render at correct size within the new 36px height. The icon size is computed from `PLUS_ARM` (5.0) and `CHEVRON_HALF` (5.0) — both are absolute sizes, not relative to bar height. The centering uses `strip.y + strip.h / 2.0`, which adapts to any height. No change needed.

**Validation:** Tab action buttons have visible left borders. Close button hover uses consistent danger coloring.

---

## 01.7 Golden Tests

**File(s):** `oriterm/src/gpu/visual_regression/tab_bar_brutal.rs` (NEW), `oriterm/src/gpu/visual_regression/mod.rs`

Write golden tests proving every visual change. These tests render the tab bar through the real GPU pipeline and compare against reference PNGs.
- [ ] Create `oriterm/src/gpu/visual_regression/tab_bar_brutal.rs` module with `#![cfg(all(test, feature = "gpu-tests"))]` at the top (same gate as `tab_bar_icons.rs`)
- [ ] Add `mod tab_bar_brutal;` to `visual_regression/mod.rs` (currently at line 26, after `mod tab_bar_icons;`)
- [ ] Fix the hardcoded `46.0` in `tab_bar_icons.rs` line 89: change `bounds: Rect::new(0.0, 0.0, WIDTH as f32, 46.0)` to `bounds: Rect::new(0.0, 0.0, WIDTH as f32, tab_bar.metrics().height)`. This requires moving the bounds computation after `tab_bar` construction (already the case — `tab_bar` is constructed at line 74). This fix is needed BEFORE writing new tests, since the existing `tab_bar_emoji_golden` test uses this helper.
- [ ] Create `render_tab_bar_brutal()` helper in the new test file (or reuse the fixed `render_tab_bar()` from `tab_bar_icons.rs` — but since the tab bar icons test file uses `headless_tab_bar_env()` with emoji fonts and the brutal tests don't need emoji, prefer creating a separate helper that uses `headless_env()` from `super` plus `UiFontSizes` setup matching `headless_tab_bar_env()`).
- [ ] **Test: `tab_bar_brutal_3tabs_96dpi`** — 3 tabs (1 active, 1 modified, 1 plain), 600x36px at 96 DPI
  - Construct tabs: `TabEntry::new("zsh")`, `TabEntry::new("nvim config.toml").with_modified(true)`, `TabEntry::new("cargo build")`
  - Active index: 0
  - Verifies: flat tabs, accent bar on active, bottom border, modified dot, separators
  - Active tab should show: bg-base background, text-bright color, 2px accent bar on top, bottom bleed
  - Modified tab should show: 6px square accent dot
  - All tabs: no rounded corners
- [ ] **Test: `tab_bar_brutal_active_close_default_96dpi`** — Active tab with close at 0.6 opacity (default state, no hover). Uses `render_tab_bar` pattern with `tab_bar.set_hover_hit(TabBarHit::None)` (or leave default). Active tab's close button should render at 0.6 opacity per the new logic in 01.5.
- [ ] **Test: `tab_bar_brutal_hover_close_96dpi`** — Active tab with close hovered (1.0 opacity + danger highlight). Uses `render_tab_bar` pattern, then call `tab_bar.set_hover_hit(TabBarHit::CloseTab(0))` before painting (this is a pub method on TabBarWidget per animation.rs line 20). Close button should render at full opacity with danger color background.
- [ ] **Test: `tab_bar_brutal_actions_96dpi`** — Tab bar with new-tab and dropdown buttons visible
  - Verifies: border-left on action buttons, correct icon placement at 36px height
- [ ] **Test: `tab_bar_brutal_single_tab_96dpi`** — Single tab (no separators, active by default)
  - Edge case: verify accent bar and bottom bleed render correctly with only one tab
- [ ] Run `ORITERM_UPDATE_GOLDEN=1 cargo test -p oriterm --features gpu-tests -- tab_bar_brutal` to generate initial references
- [ ] Visually inspect all generated reference PNGs against the mockup
- [ ] Commit reference PNGs to `oriterm/tests/references/`
- [ ] **Update existing golden**: `tab_bar_emoji` golden will change due to new 36px height and new colors — regenerate with `ORITERM_UPDATE_GOLDEN=1 cargo test -p oriterm --features gpu-tests -- tab_bar_emoji` and visually inspect
- [ ] **Add unit tests in `oriterm_ui/src/widgets/tab_bar/tests.rs`** for the rendering changes:
  - **Test: `colors_from_theme_bar_bg_is_bg_primary`** — Verify `TabBarColors::from_theme(&UiTheme::dark()).bar_bg == UiTheme::dark().bg_primary` (after the fix in 01.4). This locks the color swap fix.
  - **Test: `colors_from_theme_active_bg_is_bg_secondary`** — Verify `TabBarColors::from_theme(&UiTheme::dark()).active_bg == UiTheme::dark().bg_secondary` (after the fix in 01.3). This locks the color swap fix.
  - **Test: `colors_from_theme_separator_full_opacity`** — Verify `TabBarColors::from_theme(&UiTheme::dark()).separator == UiTheme::dark().border` (no `.with_alpha(0.5)` after the fix in 01.4).
  - **Test: `colors_from_theme_has_accent_bar`** — Verify `TabBarColors::from_theme(&UiTheme::dark()).accent_bar == UiTheme::dark().accent`.
  - **Test: `colors_from_theme_has_bar_border`** — Verify `TabBarColors::from_theme(&UiTheme::dark()).bar_border == UiTheme::dark().border`.
- [ ] `/tpr-review` checkpoint

**Validation:** All `tab_bar_brutal_*` golden tests pass. Reference PNGs visually match the mockup's tab bar section.

---

## 01.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 01.N Completion Checklist

- [ ] `draw.rs` under 500 lines (helpers extracted to `draw_helpers.rs`)
- [ ] Tab bar renders at 36px height with 0 top margin and 14px padding
- [ ] Active tab: zero radius, 2px accent bar on top, bottom border bleed into content
- [ ] Inactive tabs: flat background, 1px right-border separator (full height)
- [ ] Tab bar: 2px bottom border visible across full width
- [ ] Modified indicator: 6px square accent dot visible on modified tabs
- [ ] Close button: opacity 0 by default, 1.0 on hover, 0.6 on active tab
- [ ] Tab action buttons: 1px left border visible
- [ ] Window control close hover: uses danger color (#c87878)
- [ ] All `tab_bar_brutal_*` golden tests pass
- [ ] Existing `tab_bar_emoji` golden updated and passing
- [ ] Color mapping unit tests pass (bar_bg, active_bg, separator, accent_bar, bar_border)
- [ ] Modified indicator unit tests pass (builder, suppression on hover)
- [ ] `cargo test -p oriterm_ui` green (no regressions in tab bar unit tests)
- [ ] `./build-all.sh` green, `./clippy-all.sh` green
- [ ] `/tpr-review` passed — independent review found no critical or major issues

**Exit Criteria:** The tab bar renders identically to the `.tab-bar` section of `mockups/main-window-brutal.html` at 96 DPI. All golden tests pass with committed reference PNGs. No visual regressions in existing tests.
