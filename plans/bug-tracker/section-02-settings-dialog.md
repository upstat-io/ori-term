---
section: "02"
title: "Settings Dialog Bugs"
status: in-progress
reviewed: true
goal: "Track and fix bugs in the settings dialog layout, chrome, and interactions"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "02.1"
    title: "Active Bugs"
    status: in-progress
---

# Section 02: Settings Dialog Bugs

**Status:** In Progress
**Goal:** Track and fix all discovered bugs in the settings dialog layout, chrome, and interactions.

**Note:** This section is never marked complete. New bugs are appended as discovered.

---

## 02.1 Active Bugs

- [x] **BUG-02.1**: Dialog has title bar — mockup has none, drag by header area instead
  - **File(s)**: `oriterm/src/window_manager/platform/windows.rs`
  - **Root cause**: `set_window_type` added `WS_EX_TOOLWINDOW` extended style to dialog windows via `SetWindowLongPtrW`. On Windows, `WS_EX_TOOLWINDOW` creates a tool window with a smaller title bar — re-introducing a caption even though `with_decorations(false)` was set. The purpose was to suppress taskbar buttons, but owned windows already don't get taskbar buttons.
  - **Found**: 2026-03-27 — manual sign-off (Section 14.4), screenshot comparison
  - **Fixed**: 2026-03-27 — Removed `WS_EX_TOOLWINDOW` from `set_window_type`. The owner relationship (`GWLP_HWNDPARENT`) already prevents taskbar buttons for owned dialog windows.

- [x] **BUG-02.2**: Vertical spacing and paddings off throughout vs mockup
  - **File(s)**: Multiple widget `paint()` methods across `oriterm_ui/src/widgets/`
  - **Root cause**: Widgets passed `child_node.content_rect` (which subtracts padding) instead of `child_node.rect` (the full layout rect) when constructing child DrawCtx bounds. This caused double-padding in 9 widgets: container, dialog/rendering, form_layout, form_row, form_section, panel, setting_row, settings_footer, settings_panel.
  - **Found**: 2026-03-27 — manual sign-off (Section 14.4), screenshot comparison
  - **Fixed**: 2026-03-27 — Changed all 9 widgets from `content_rect` to `rect` for child bounds. All other spacing constants (page padding 28px, section gap 28px, setting row 44px min-height, row padding 10px/14px) already match the mockup.

- [x] **BUG-02.3**: Sidebar active highlight bg color wrong — should use opacity
  - **File(s)**: `oriterm_ui/src/widgets/sidebar_nav/mod.rs`
  - **Root cause**: N/A — not a bug. Code correctly uses `accent_bg_strong` (rgba with alpha 0.14) from the theme, matching the mockup's `var(--accent-bg-strong): rgba(109, 155, 224, 0.14)`.
  - **Found**: 2026-03-27 — manual sign-off (Section 14.4)
  - **Resolved**: 2026-03-27 — closed as not-a-bug after code investigation

- [x] **BUG-02.4**: Bottom padding below config file path in navbar wrong
  - **File(s)**: `oriterm_ui/src/widgets/sidebar_nav/paint.rs`
  - **Root cause**: Footer was positioned at `bounds.bottom() - FOOTER_PADDING_Y` (8px from edge), but the sidebar also has 16px outer padding that wasn't being accounted for. The mockup has both `.sidebar { padding: 16px 0 }` and `.sidebar-footer { padding: 8px 16px }`, totaling 24px from the window edge.
  - **Found**: 2026-03-27 — manual sign-off (Section 14.4)
  - **Fixed**: 2026-03-27 — Changed footer Y start to `bounds.bottom() - SIDEBAR_PADDING_Y - FOOTER_PADDING_Y` (16 + 8 = 24px from bottom edge).

- [x] **BUG-02.5**: Section description gap slightly off vs mockup
  - **File(s)**: `oriterm/src/app/settings_overlay/form_builder/shared/mod.rs`
  - **Root cause**: A `4px` spacer between section title row and description label created more vertical space than the mockup, which uses `margin-top: -8px` (negative margin) to pull the description tight against the title.
  - **Found**: 2026-03-27 — manual sign-off (Section 14.4)
  - **Fixed**: 2026-03-27 — Removed the 4px spacer so the description immediately follows the title row. The 12px spacer after the description is retained.

- [ ] **BUG-02.6**: Search settings has wrong padding + unwanted search icon
  - **File(s)**: `oriterm_ui/src/widgets/sidebar_nav/paint.rs`
  - **Root cause**: TBD — user reports the search input does not match the mockup visually. Needs side-by-side comparison on the live build to identify specific discrepancies.
  - **Found**: 2026-03-27 — manual sign-off (Section 14.4), screenshot comparison
  - **Fix**: Needs live visual comparison to pinpoint the issue

- [ ] **BUG-02.7**: Nav item padding/spacing between elements not matching mockup
  - **File(s)**: `oriterm_ui/src/widgets/sidebar_nav/geometry.rs`, `paint.rs`
  - **Root cause**: TBD — user reports gap between nav items doesn't look right. Current values (1px margin, 7px padding, 13px content) match mockup CSS numerically but visual result may differ. Needs live comparison.
  - **Found**: 2026-03-27 — manual sign-off (Section 14.4)
  - **Fix**: Needs live visual comparison to identify specific spacing discrepancy
