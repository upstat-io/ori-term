---
section: "02"
title: "Settings Dialog Bugs"
status: in-progress
reviewed: true
goal: "Track and fix bugs in the settings dialog layout, chrome, and interactions"
depends_on: []
third_party_review:
  status: resolved
  updated: 2026-03-30
sections:
  - id: "02.1"
    title: "Active Bugs"
    status: in-progress
  - id: "02.R"
    title: "Third Party Review Findings"
    status: complete
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

- [x] **BUG-02.6**: Search settings has wrong padding + unwanted search icon
  - **File(s)**: `oriterm_ui/src/widgets/sidebar_nav/paint.rs`, `input.rs`
  - **Root cause**: Search field had a 26px left padding to accommodate a search icon that was unwanted. The icon added visual clutter and the asymmetric padding wasted space.
  - **Found**: 2026-03-27 — manual sign-off (Section 14.4), screenshot comparison
  - **Fixed**: 2026-03-29 — Removed search icon rendering, normalized left padding from 26px to 8px (matching right padding). Updated `SEARCH_TEXT_INSET` in input.rs for click-to-cursor mapping. Cleaned up unused `IconId` import.

- [x] **BUG-02.7**: Nav item padding/spacing between elements not matching mockup
  - **File(s)**: `oriterm_ui/src/widgets/sidebar_nav/geometry.rs`, `paint.rs`
  - **Root cause**: Two issues: (1) `NAV_ITEM_CONTENT_H` was 13px (font-size) instead of ~16px (line-height for 13px font × ~1.2), making items 3px too short each. (2) Background painted over margin area, eliminating the visible 1px gap between items that CSS `margin: 1px 0` produces.
  - **Found**: 2026-03-27 — manual sign-off (Section 14.4)
  - **Fixed**: 2026-03-29 — Changed `NAV_ITEM_CONTENT_H` from 13.0 to 16.0. Added `NAV_ITEM_BG_H` constant. Background, indicator, icon, text, and modified-dot positioning all use the bg-inset rect now. Nav items are 32px total (1+7+16+7+1) with visible 1px gaps.

- [x] **BUG-02.8**: Settings dialog window not draggable (regression)
  - **File(s)**: `oriterm/src/app/dialog_management.rs`, `oriterm/src/app/dialog_context/event_handling/mouse.rs`
  - **Root cause**: Commit 81a304b removed `WindowChromeWidget` from the dialog and set `DIALOG_DRAG_CAPTION_HEIGHT` to `0.0`. This eliminated the OS-level caption region that enabled window dragging. Without it, Windows `WM_NCHITTEST` always returned `HTCLIENT` (no drag area).
  - **Found**: 2026-03-28 — manual sign-off (Section 14.4), user report
  - **Fixed**: 2026-03-28 — Set `DIALOG_DRAG_CAPTION_HEIGHT` to 48px and excluded the sidebar (200px) as an interactive rect so the search field stays clickable. The content area header (right of sidebar, top 48px) is now the drag zone. Added `try_dialog_header_drag()` for Linux/macOS `drag_window()` support.

- [ ] **BUG-02.9**: Rendering page backend dropdown doesn't show active backend
  - **Severity**: medium
  - **File(s)**: `oriterm/src/app/settings_overlay/form_builder/rendering.rs`
  - **Root cause**: The dropdown was partially fixed (platform filtering added 2026-03-29) but still shows plain "Auto" without indicating which backend it resolved to. The active-backend display was deferred.
  - **Found**: 2026-03-29 — manual | **Reopened**: 2026-03-30 per TPR-02-001

- [ ] **BUG-02.10**: Config file path link in sidebar footer missing tooltip with full path on hover
  - **File(s)**: `oriterm_ui/src/widgets/sidebar_nav/paint.rs` (footer painting), `oriterm_ui/src/widgets/sidebar_nav/mod.rs` (`config_path` field)
  - **Root cause**: The config path text is truncated via `truncate_with_ellipsis()` but no tooltip is shown on hover. Users can't see the full path to know where the config file lives. The hover highlight works (changes text color) but doesn't surface the full path.
  - **Found**: 2026-03-29 — manual, user feature request
  - **Fix**: Add tooltip rendering when `hovered_footer == Some(HoveredFooterTarget::ConfigPath)` showing `self.config_path` in full. May require tooltip infrastructure (overlay or simple painted rect near cursor).

- [ ] `[BUG-02-012][medium]` **Font family selector should be searchable with OS font enumeration** — found by manual.
  Repro: Open Settings > Font > Family dropdown. Currently a static list — should enumerate installed OS fonts and support type-to-filter.
  Subsystem: `oriterm/src/app/settings_overlay/form_builder/` (font dropdown), `oriterm/src/font/discovery/` (font enumeration)
  Found: 2026-04-01 | Source: manual — user feature request
  Note: Requires platform font enumeration (DirectWrite on Windows, fontconfig on Linux, CoreText on macOS) and a searchable/filterable dropdown widget.

- [x] **BUG-02.11**: Sidebar cursor icon is pointer over entire area — should only be pointer over interactive items
  - **File(s)**: `oriterm_ui/src/widgets/sidebar_nav/mod.rs`, `input.rs`
  - **Root cause**: `layout()` set a static `CursorIcon::Pointer` on the top-level `LayoutBox`, making cursor pointer everywhere.
  - **Found**: 2026-03-29 — manual, user report
  - **Fixed**: 2026-03-30 — Added `cursor_icon: Cell<CursorIcon>` field, dynamically set in `on_input(MouseMove)` based on hover state: Pointer when `hovered_item` or `hovered_footer` is Some, Default otherwise. `layout()` reads the Cell value for the LayoutBox cursor.

---

## 02.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- [x] `[TPR-02-001][medium]` `oriterm/src/app/settings_overlay/form_builder/rendering.rs:38` — `BUG-02.9` was marked fixed even though the second half of the bug is still present.
  Resolved: Reopened BUG-02.9 with updated scope (active-backend display). Fixed 2026-03-30.

---
