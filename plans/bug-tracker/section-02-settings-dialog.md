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
  - id: "02.R"
    title: "Third Party Review Findings"
    status: not-started
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

- [x] **BUG-02.8**: Settings dialog window not draggable (regression)
  - **File(s)**: `oriterm/src/app/dialog_management.rs`, `oriterm/src/app/dialog_context/event_handling/mouse.rs`
  - **Root cause**: Commit 81a304b removed `WindowChromeWidget` from the dialog and set `DIALOG_DRAG_CAPTION_HEIGHT` to `0.0`. This eliminated the OS-level caption region that enabled window dragging. Without it, Windows `WM_NCHITTEST` always returned `HTCLIENT` (no drag area).
  - **Found**: 2026-03-28 — manual sign-off (Section 14.4), user report
  - **Fixed**: 2026-03-28 — Set `DIALOG_DRAG_CAPTION_HEIGHT` to 48px and excluded the sidebar (200px) as an interactive rect so the search field stays clickable. The content area header (right of sidebar, top 48px) is now the drag zone. Added `try_dialog_header_drag()` for Linux/macOS `drag_window()` support.

- [ ] **BUG-02.9**: Rendering page backend dropdown shows all backends on every platform + doesn't show active backend
  - **File(s)**: `oriterm/src/app/settings_overlay/form_builder/rendering.rs`
  - **Root cause**: `build_gpu_section()` hardcodes all 4 backends (Auto, Vulkan, DirectX 12, Metal) regardless of platform. Should show only valid backends per OS: Windows → Auto/Vulkan/DirectX 12, macOS → Auto/Metal, Linux → Auto/Vulkan. Additionally, the dropdown doesn't indicate which backend is currently in use (e.g. "Auto (Vulkan)" showing the resolved backend), unlike the mockup which displays the active backend.
  - **Found**: 2026-03-29 — manual, user report comparing live build to mockup
  - **Fix**: Platform-gate the dropdown items with `#[cfg()]` or runtime OS detection. Add resolved-backend display (query `wgpu::Adapter::get_info().backend` or equivalent at settings build time).

- [ ] **BUG-02.10**: Config file path link in sidebar footer missing tooltip with full path on hover
  - **File(s)**: `oriterm_ui/src/widgets/sidebar_nav/paint.rs` (footer painting), `oriterm_ui/src/widgets/sidebar_nav/mod.rs` (`config_path` field)
  - **Root cause**: The config path text is truncated via `truncate_with_ellipsis()` but no tooltip is shown on hover. Users can't see the full path to know where the config file lives. The hover highlight works (changes text color) but doesn't surface the full path.
  - **Found**: 2026-03-29 — manual, user feature request
  - **Fix**: Add tooltip rendering when `hovered_footer == Some(HoveredFooterTarget::ConfigPath)` showing `self.config_path` in full. May require tooltip infrastructure (overlay or simple painted rect near cursor).

- [ ] **BUG-02.11**: Sidebar cursor icon is pointer over entire area — should only be pointer over interactive items
  - **File(s)**: `oriterm_ui/src/widgets/sidebar_nav/mod.rs:331`
  - **Root cause**: `layout()` sets `.with_cursor_icon(CursorIcon::Pointer)` on the top-level `LayoutBox` for the whole sidebar widget. This makes the cursor a pointer everywhere — over the background, spacing, search field, version label — not just over the clickable nav items and footer links.
  - **Found**: 2026-03-29 — manual, user report
  - **Fix**: Remove the widget-level `CursorIcon::Pointer`. Instead, apply pointer cursor only to interactive sub-regions (nav items, config path link, update link) via per-region cursor handling or sub-layout boxes with their own cursor icons.

---

## 02.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---
