---
section: "03"
title: "UI Framework Bugs"
status: in-progress
reviewed: true
goal: "Track and fix bugs in the UI framework (layout, interaction, focus, animation)"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "03.1"
    title: "Active Bugs"
    status: in-progress
  - id: "03.R"
    title: "Third Party Review Findings"
    status: not-started
---

# Section 03: UI Framework Bugs

**Status:** In Progress
**Goal:** Track and fix all discovered bugs in the UI framework.

**Note:** This section is never marked complete. New bugs are appended as discovered.

---

## 03.1 Active Bugs

- [x] **BUG-03.1**: Clicking outside focused input doesn't unfocus — only clicking nav works
  - **File(s)**: `oriterm_ui/src/window_root/pipeline.rs`
  - **Root cause**: The event dispatch pipeline only changed focus when a `FocusController` explicitly requested it (i.e., clicking another focusable widget). Clicking non-focusable areas (empty space, labels) produced no focus change request, leaving the previous input focused.
  - **Found**: 2026-03-27 — manual sign-off (Section 14.4)
  - **Fixed**: 2026-03-27 — Added Step 5.5 in `process_event()`: after dispatch, if the event is `MouseDown`, no overlay consumed it, no `REQUEST_FOCUS` was issued, and focus is active, clear focus via `interaction.clear_focus()`.

- [x] **BUG-03.2**: Layout solver `min_height` applies to border-box, not content-box — padding not additive like CSS
  - **File(s)**: `oriterm_ui/src/layout/solver.rs` (lines 39-54), `oriterm_ui/src/layout/layout_box.rs`
  - **Root cause**: The layout solver treats `min_height` as a constraint on the **outer rect** (border-box). When a widget has `min_height: 44px` and `padding: 10px`, the total outer height is 44px with only 24px for content. In the CSS content-box model the user expects, `min-height: 44px` should apply to the **content area**, and padding (20px total) should be added on top, yielding 64px outer height. This affects all widgets using `min_height` + padding (SettingRowWidget, FormRow, etc.) and will become critical when flow wrapping is implemented for smaller screens.
  - **Found**: 2026-03-28 — manual sign-off (Section 14.4), screenshot comparison showing cramped setting rows
  - **Fixed**: 2026-04-01 — Changed `solve()` in solver.rs to inflate `min_width`/`min_height` by padding when non-zero (content-box → border-box conversion). Updated doc comments on `LayoutBox.min_width`/`min_height` to document content-box semantics. Added 4 layout tests (`min_height_content_box_adds_padding`, `min_width_content_box_adds_padding`, `min_height_without_padding_unchanged`, `min_height_flex_content_box`). Strengthened setting_row tests to verify `content_rect.height() >= MIN_HEIGHT`. Regenerated visual regression references. Only call site affected: `setting_row/mod.rs` (min_height 44 + vh padding 10 → 64px outer, giving proper content breathing room).

---

## 03.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---
