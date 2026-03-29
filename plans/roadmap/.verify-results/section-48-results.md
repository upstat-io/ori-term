# Section 48 Verification: Native OS Scrollbars

## Status: NOT STARTED (for terminal scrollbar -- but significant UI scrollbar infrastructure exists)

### Evidence Search

**No terminal-level scrollbar exists.** Searching for `scrollbar` in `oriterm/src/` and `oriterm/src/config/` yields zero matches. There is no `scrollbar` config field in `BehaviorConfig`, `Config`, or any appearance-related config. No scrollbar is rendered over the terminal grid.

**However, a complete UI-widget-level scrollbar already exists** in `oriterm_ui/src/widgets/scroll/`:

1. **`ScrollWidget`** (`oriterm_ui/src/widgets/scroll/mod.rs`, 443 lines): Full scroll container widget implementing `Widget` trait. Has vertical/horizontal/both-axis scrolling, mouse wheel handling, keyboard navigation (PageUp/Down, Home/End, arrow keys), child capture for drag passthrough.

2. **`scrollbar.rs`** (`oriterm_ui/src/widgets/scroll/scrollbar.rs`, 152 lines): Complete scrollbar rendering and interaction for the ScrollWidget. Includes:
   - Track and thumb rendering with `RectStyle`
   - Thumb drag interaction (mouse down capture, proportional scrolling on move, release)
   - Track click-to-jump (click outside thumb scrolls proportionally)
   - Hover state tracking (track expand on hover, color changes)
   - `ScrollbarPolicy` enum: `Auto`, `Always`, `Hidden`
   - `ScrollbarStyle` struct: `width`, `thumb_color`, `track_color`, `thumb_radius`, `min_thumb_height`
   - `ScrollbarState`: `dragging`, `drag_start_y`, `drag_start_offset`, `track_hovered`

3. **`ScrollbarPolicy`** enum and **`ScrollbarStyle`** struct are already production code used by the settings panel and menu widgets.

### What Exists vs. What the Plan Needs

| Feature | UI ScrollWidget (exists) | Terminal Scrollbar (planned) |
|---------|-------------------------|------------------------------|
| Scrollbar model | pixel-based `scroll_offset` | row-based `display_offset` from grid |
| Content metric | child widget natural height | `scrollback.len() + visible_lines` |
| Rendering | `DrawCtx` / `draw_list` | GPU overlay in terminal render path |
| Input handling | Via Widget `handle_mouse` | Intercept before mouse reporting |
| Fade animation | None (always visible when shown) | 1.5s delay, 0.3s fade-out |
| Thumb sizing | `view_height / content_height` ratio | Same ratio, row-based |
| Track hover expand | Yes (1.5x width) | Yes (8px to 12px) |
| Config | `ScrollbarPolicy` enum | New `[appearance] scrollbar` config |

**Key insight**: The terminal scrollbar is a fundamentally different control from the UI widget scrollbar. The UI `ScrollWidget` manages pixel-space scrolling of widget children. The terminal scrollbar manages `display_offset` (row-space scrolling of terminal scrollback). They share concepts (thumb drag, track click, hover expand) but operate on different data models.

### Reuse Potential

The existing `ScrollbarStyle` (color, width, radius, min_thumb_height) and `ScrollbarState` (dragging, drag_start_y/offset, track_hovered) structs could potentially be reused or adapted. The rendering code pattern (track + thumb rects with `RectStyle`) is directly applicable. However, the terminal scrollbar must:
1. Be rendered by the GPU renderer (not the UI draw list pipeline)
2. Consume `display_offset` / `scrollback.len()` instead of pixel offsets
3. Intercept mouse events before PTY mouse reporting
4. Support fade animation (the existing scrollbar has no fade)

### TODOs/FIXMEs

No TODOs or FIXMEs found related to scrollbars in either `oriterm/src/` or `oriterm_ui/src/widgets/scroll/`.

### Gap Analysis

1. **Missing fade animation**: The plan requires fade-in/fade-out animation (show on scroll, fade after 1.5s inactivity). The existing scrollbar has no animation. The `oriterm_ui` animation system (VisualStateAnimator, RenderScheduler) exists and could drive this, but the terminal scrollbar sits in `oriterm` (GPU layer), not in the widget tree. This needs either: (a) a standalone animation timer in the terminal render path, or (b) integrating the terminal scrollbar as a widget overlay via WindowRoot. The plan doesn't specify which approach.

2. **Rendering layer ambiguity**: The plan mentions both `oriterm/src/gpu/window_renderer/` and `oriterm_ui/src/widgets/scroll/scrollbar.rs` as possible file locations. These are fundamentally different rendering approaches (GPU quads vs. widget draw list). The plan should commit to one. Given that the scrollbar must overlay the terminal grid without consuming columns and must intercept before mouse reporting, the GPU renderer approach is more natural.

3. **Missing mouse reporting guard**: The plan mentions "Scrollbar clicks are NOT forwarded to applications via mouse reporting" but doesn't detail how this is wired. Mouse events currently flow through `oriterm/src/app/mouse_input.rs` and then optionally to `mouse_report`. The scrollbar hit test must happen before the mouse reporting check. This ordering is important but not specified.

4. **Config placement**: The plan adds `[appearance] scrollbar` but there is no `AppearanceConfig` section in the current config. The current config has `window`, `terminal`, `behavior`, `colors`, `font`, `bell`, `pane`. The scrollbar config could go in `window` (visual appearance of the window) or a new `appearance` section. The plan should clarify.

5. **Platform-native styling**: The plan title says "Native OS Scrollbars" and the goal mentions "matching the host OS look and feel" but the implementation is a custom-drawn overlay, not an actual platform-native scrollbar. This is fine (Ghostty does the same) but the title is slightly misleading. The appearance parameters (width, colors) are hardcoded in the plan rather than queried from the OS theme.

6. **Good**: The plan correctly identifies that the scrollbar must be an overlay (not consuming grid columns).

7. **Good**: The three-mode config (overlay/always/never) matches Ghostty's approach.

### Infrastructure from Other Sections

- **Section 05 (Window + GPU)**: Complete. Provides the GPU rendering infrastructure.
- **Section 07 (UI Framework)**: Complete. Provides `ScrollWidget` and `ScrollbarStyle` which could be referenced.
- **Section 10 (Mouse Input)**: Complete. Provides the mouse input pipeline that needs scrollbar interception.
- The existing `ScrollWidget` infrastructure provides a pattern reference but is not directly reusable for the terminal scrollbar.

### Verdict

**CONFIRMED NOT STARTED** for the terminal-level scrollbar. The UI-level `ScrollWidget` with scrollbar is complete but serves a different purpose (widget scrolling vs. terminal scrollback). The plan is reasonable but should:
- Commit to a rendering approach (GPU overlay vs. widget overlay)
- Clarify config section placement (no `AppearanceConfig` exists currently)
- Specify mouse event ordering relative to mouse reporting
- Acknowledge that `ScrollbarStyle` and `ScrollbarState` from the existing scroll widget provide a reusable pattern
