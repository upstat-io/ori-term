---
plan: "window-management"
title: "Window Management System: Exhaustive Implementation Plan"
status: in-progress
supersedes: []
references:
  - "plans/roadmap/section-24-visual-polish.md"
  - "~/projects/reference_repos/chromium_ui/ui/aura/"
  - "~/projects/reference_repos/console_repos/wezterm/wezterm-gui/src/termwindow/"
---

# Window Management System: Exhaustive Implementation Plan

## Mission

Build a proper OS-native window management system for ori_term where every window — main terminal windows, settings dialogs, confirmation dialogs, tear-off windows — is a real OS window with native shadows, proper platform treatment, and the ability to move independently outside parent bounds. Replace the current hack where dialogs are overlays trapped inside the main window with a unified window manager that handles all window kinds through a single abstraction, backed by platform-native window ownership on Windows, macOS, and Linux.

## Architecture

```
                          ┌─────────────────────────────────────┐
                          │           App (singleton)           │
                          │                                     │
                          │  ┌─────────────────────────────┐    │
                          │  │      WindowManager          │    │
                          │  │                             │    │
                          │  │  registry: HashMap<         │    │
                          │  │    WinitId, ManagedWindow>   │    │
                          │  │                             │    │
                          │  │  hierarchy: parent→children │    │
                          │  │  focused_id: Option<WinitId>│    │
                          │  └──────────┬──────────────────┘    │
                          │             │                       │
              ┌───────────┼─────────────┼───────────────┐       │
              │           │             │               │       │
     ┌────────▼───┐  ┌────▼─────┐  ┌───▼──────┐               │
     │ Main       │  │ Dialog   │  │ TearOff  │               │
     │ Window     │  │ Window   │  │ Window   │               │
     │            │  │          │  │          │               │
     │ OS Window  │  │ OS Window│  │ OS Window│               │
     │ + Grid     │  │ + UI     │  │ + Grid   │               │
     │ + TabBar   │  │ + Form   │  │ + TabBar │               │
     │ + Chrome   │  │          │  │ + Chrome │               │
     │ + Overlays │  │          │  │          │               │
     └────────────┘  └──────────┘  └──────────┘               │
              │           │             │                       │
              │     ┌─────▼─────────────▼──────────────────┐    │
              │     │  Per-Window: WindowRenderer           │    │
              │     │    surface, atlases, instance bufs    │    │
              └─────┤                                      │    │
                    │  Shared: GpuState + GpuPipelines      │    │
                    └──────────────────────────────────────┘    │
                          │                                     │
                          │  ┌─────────────────────────────┐    │
                          │  │  Platform Native Layer      │    │
                          │  │                             │    │
                          │  │  Windows: HWND ownership,   │    │
                          │  │    DWM shadows, WS_EX_*     │    │
                          │  │  macOS: NSWindow child,     │    │
                          │  │    hasShadow, levels        │    │
                          │  │  Linux: transient_for,      │    │
                          │  │    window type hints        │    │
                          │  └─────────────────────────────┘    │
                          └─────────────────────────────────────┘
```

## Design Principles

**1. Every window is an OS window.** No more fake floating panels trapped inside a parent window. Dialogs, settings, confirmations — all get their own OS window handle with native shadows, decorations (or custom chrome), and the ability to be moved independently. This is how Chrome, VS Code, and every serious desktop app works. The current overlay-based settings dialog is the motivating pain point: it can't leave the main window's bounds, has no OS shadow, and doesn't feel native.

**2. Unified window lifecycle through WindowManager.** One code path for creating, tracking, focusing, and destroying all window kinds. Main terminal windows, dialog windows, and tear-off windows all flow through the same `WindowManager` registry. This prevents the current situation where tear-off windows are ad-hoc one-offs with their own creation path disconnected from the rest of the system. (Lightweight popups like context menus and dropdowns remain as in-window overlays via `OverlayManager` — only heavy modals become real OS windows.)

**3. Platform-native ownership, not emulation.** Don't simulate window ownership in user space. Use the OS's native mechanisms: `SetWindowLongPtr` owner HWND on Windows, `addChildWindow:ordered:` on macOS, `set_transient_for` on X11/Wayland. This gives us correct z-ordering, taskbar grouping, minimize/restore behavior, and shadow rendering for free.

## Section Dependency Graph

```
  Section 1 (Core)
      │
      ├──────────────────────────┐
      │                          │
      ▼                          ▼
  Section 2 (Platform)       Section 7 (GPU)
      │                          │
      ▼                          │
  Section 3 (Main Win)           │
      │                          │
      ├──────────┬───────────────┘
      │          │
      │          ▼
      │    Section 4 (Dialogs, needs 02+03+07)
      │          │
      ▼          │
  Section 5a     │
  (Tear-Off      │
   Refactor)     │
      │          │
      ├──────────┘
      ▼
  Section 6 (Event Routing, needs 03+04+5a)
      │
      ▼
  Section 5b/5c (Cross-Platform Tear-Off) [can defer]
      │
      ▼
  Section 8 (Verification)
```

- Section 1 (Core) is the foundation — all other sections depend on it.
- Sections 2 (Platform) and 7 (GPU) are independent of each other and can be worked in parallel.
- Section 3 (Main Window Migration) requires Section 2 for platform integration.
- Section 4 (Dialogs) requires Sections 2, 3, AND 7 (platform ownership, WindowManager in App, UiOnly renderer).
- Section 5a (Tear-Off Refactor) requires Section 3 (main window must be migrated first) and Section 2 (cross-platform cursor/bounds queries).
- Section 5b/5c (Cross-Platform Tear-Off) requires Section 5a. Can be deferred without blocking Section 6.
- Section 6 (Event Routing) requires Sections 3, 4, 5a (all window kinds exist in WindowManager).
- Section 8 (Verification) requires all sections complete.

**Cross-section interactions (must be co-implemented):**
- **Section 2 + Section 4**: Dialog windows need platform-native ownership to stack correctly above their parent. Without Section 2's platform layer, Section 4's dialogs would be unowned OS windows that disappear behind the parent.
- **Section 3 + Section 6**: Main window migration changes how events flow into the window. Section 6's event routing must account for the new WindowManager dispatch path.
- **Section 4 + Section 7**: Dialog windows need a GPU renderer (Section 7's UiOnly mode) to render their content. Section 4's `DialogWindowContext` stores a `WindowRenderer` that must be created using the UiOnly constructor from Section 7. These two sections should be co-implemented — at minimum, Section 7's `WindowRenderer::new_ui_only()` constructor must exist before Section 4 can create dialog windows.
- **Section 5 + Section 2**: Tear-off cross-platform support (Section 5) needs the platform abstraction from Section 2 for cursor position queries and window bounds queries that are currently Windows-only in `platform_windows`.

## Implementation Sequence

```
Phase 0 - Prerequisites
  +-- 1.1: Define WindowKind enum and ManagedWindow struct
  +-- 1.2: Define WindowManager trait surface (registry, hierarchy, lookup)
  +-- 7.1: Audit current GPU sharing (already shared device/queue/pipelines)

Phase 1 - Foundation
  +-- 1.3: Implement WindowManager core (create, track, destroy)
  +-- 2.1: Platform trait definition (NativeWindowOps)
  +-- 2.2: Windows platform implementation (HWND ownership, shadows)
  +-- 2.3: macOS platform implementation (NSWindow child, levels)
  +-- 2.4: Linux platform implementation (X11 transient, Wayland)
  Gate: WindowManager can create and track a second OS window with native ownership

Phase 2 - Main Window Migration
  +-- 3.1: Add WindowManager field to App, register initial window
  +-- 3.2: Route window creation/closure through WindowManager (dual-map)
  +-- 3.3: Update event loop to route through WindowManager
  Gate: Existing single-window behavior unchanged, managed by WindowManager

Phase 3a - Dialog Windows
  +-- 7.2: Dialog window GPU rendering (UI-only pipeline) [MUST precede 4.1]
  +-- 4.1: DialogWindow type and rendering pipeline
  +-- 4.2: Settings dialog as real OS window
  +-- 4.3: Confirmation dialogs as real OS windows
  Gate: Settings opens as a real OS window with native shadow

Phase 3b - Tear-Off Refactor (Windows-only verification)
  +-- 5.1: Migrate tear-off creation to WindowManager
  +-- 5.4: Merge detection uses WindowManager registry
  Gate: Existing Windows tear-off unchanged, now through WindowManager

Phase 3c - Cross-Platform Tear-Off [HIGH RISK — can defer]
  +-- 5.2: macOS tear-off support
  +-- 5.3: Linux tear-off support
  Gate: Tear-off works on all platforms

Phase 4 - Event Routing & Polish  [CRITICAL PATH]
  +-- 6.1: Multi-window focus management
  +-- 6.2: Keyboard/mouse routing through hierarchy
  +-- 6.3: Modal behavior (dialog blocks parent input)
  Gate: Focus, input, and modal behavior correct across all window kinds

Phase 5 - Verification
  +-- 8: Full test matrix, cross-platform validation, visual regression
```

**Why this order:**
- Phase 0-1 are pure additions — no behavioral changes to existing windows.
- Phase 2 must precede Phase 3a because dialogs need the main window already managed.
- Phase 3b (tear-off refactor) is a pure refactor verifiable on Windows alone — low risk.
- Phase 3c (cross-platform tear-off) is high risk and can be deferred without blocking Phase 4.
- Phase 4 is the critical path because incorrect event routing causes input to go to the wrong window, which is immediately user-visible. It only depends on Phase 3b (not Phase 3c), so cross-platform tear-off does not block it.

## Estimated Effort

| Section | Est. Lines | Complexity | Risk | Depends On |
|---------|-----------|------------|------|------------|
| 01 Window Manager Core | ~400 | Medium | Low | — |
| 02 Platform Native Layer | ~700 | High | Medium | 01 |
|   -> 02 Windows impl | ~250 | High | Medium | — |
|   -> 02 macOS impl | ~200 | High | Medium | — |
|   -> 02 Linux impl | ~250 | Medium-High | Medium | — |
| 03 Main Window Migration | ~300 | Medium | Low | 01, 02 |
| 04 Dialog Window System | ~600 | Medium-High | Medium | 02, 03, 07 |
| 05a Tear-Off Refactor | ~200 | Medium | Low | 02, 03 |
| 05b Cross-Platform Tear-Off | ~300 | High | **High** | 05a |
| 06 Event Routing & Focus | ~400 | High | **High** | 03, 04, 05a |
| 07 GPU Multi-Window | ~250 | Medium | Low | 01 |
| 08 Verification | ~500 (tests) | Medium | All |
| **Total new** | **~3650** | | |
| **Total deleted** | **~500** | | |

**Estimate revision notes:** Section 05 grew significantly because cross-platform tear-off requires extracting Windows-specific code from `platform_windows` into cross-platform abstractions (cursor position, window bounds, drag state). Section 04 grew because of the GPU renderer dependency and the dual-map management. Section 02 Linux grew because X11 property helpers need `XChangeProperty` and `XSendEvent` implementations.

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Window Manager Core | `section-01-core.md` | Complete |
| 02 | Platform Native Window Layer | `section-02-platform.md` | Complete |
| 03 | Main Window Migration | `section-03-main-window.md` | Complete |
| 04 | Dialog Window System | `section-04-dialogs.md` | In Progress |
| 05 | Tear-Off Window Unification | `section-05-tear-off.md` | In Progress (Phase 3b done) |
| 06 | Event Routing & Focus Management | `section-06-event-routing.md` | In Progress (06.1-06.3 done) |
| 07 | GPU Multi-Window Rendering | `section-07-gpu.md` | Complete |
| 08 | Verification & Cross-Platform Testing | `section-08-verification.md` | Not Started |
