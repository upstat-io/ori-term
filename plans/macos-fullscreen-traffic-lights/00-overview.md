---
plan: "macos-fullscreen-traffic-lights"
title: "macOS Fullscreen Exit Traffic Light Repositioning Fix"
status: complete
references:
  - "plans/macos-platform-fixes/"
---

# macOS Fullscreen Exit Traffic Light Repositioning Fix

## Mission

Eliminate the visible traffic light "jump" artifact when exiting macOS native fullscreen. Currently, macOS rebuilds the `NSTitlebarContainerView` at its default height during the exit animation, causing buttons to briefly appear at OS-default positions before our centering code repositions them. The fix: adopt Electron's proven hide/show pattern to make the transition seamless.

## Architecture

```
Fullscreen Exit Timeline (current — broken):

  willExit fires
       |
       v
  center buttons ──> macOS rebuilds container at default height
       |                    |
       v                    v
  frame-change fires   animation snapshot captured (may show default positions)
       |                    |
       v                    v
  re-center buttons    exit animation plays (may show wrong positions)
       |
       v
  didExit fires ──> center buttons again (safety net)
       |
       v
  window visible with correct positions


Fullscreen Exit Timeline (fixed — Electron pattern):

  willExit fires
       |
       v
  center buttons + HIDE container ──> macOS rebuilds container (invisible)
       |                                    |
       v                                    v
  frame-change fires (re-center,     animation snapshot captured (no buttons visible)
    still hidden — invisible)              |
       |                                    v
       v                              exit animation plays (no visible buttons)
  didExit fires
       |
       v
  reposition buttons ──> SHOW container
       |
       v
  window visible with correct positions, buttons appear at correct positions
```

## Design Principles

**1. Hide before snapshot, show after reposition.** macOS captures an animation snapshot early in the fullscreen exit process. Any repositioning after the snapshot is captured creates a visual discontinuity. By hiding the entire container before the snapshot, we remove the artifact entirely. This is the same principle Electron uses across VS Code, Slack, Discord, and every Electron app with custom traffic light positions.

**2. Keep the existing safety nets.** The `NSViewFrameDidChangeNotification` observer and the `handle_did_exit_fs` centering call remain as defense-in-depth. They cost nothing when the container is hidden and provide correct positioning before the show.

## Section Dependency Graph

```
Section 01 (Research) ──> Section 02 (Implementation) ──> Section 03 (Verification)
```

Sections are strictly sequential. Research informs implementation, implementation must be verified.

## Implementation Sequence

```
Phase 0 - Research (Section 01)
  +-- Document all approaches found across reference apps
  +-- Identify the recommended approach with trade-off analysis

Phase 1 - Implementation (Section 02)
  +-- Extract set_titlebar_container_hidden helper in mod.rs (02.1)
  +-- Modify handle_will_exit_fs to hide NSTitlebarContainerView (02.2)
  +-- Modify handle_did_exit_fs to reposition then show container (02.3)
  +-- Add safety net in handle_will_enter_fs (ensure visible) (02.4)
  +-- Update stale doc comments (5 locations): module doc, reapply_traffic_lights, install_fullscreen_observers, frame-change inline comment, handle_frame_change doc (02.5)
  +-- Document multi-window assumption (02.6)
  +-- Keep existing safety nets (frame change observer, centering)
  Gate: builds clean on all platforms, clippy green, mod.rs under 500 lines

Phase 2 - Verification (Section 03)
  +-- Manual fullscreen enter/exit testing
  +-- Verify no jump, no flash, no pop
  +-- Verify traffic lights at correct positions after exit
  Gate: visual confirmation of smooth transition
```

## Estimated Effort

| Section | Est. Lines Changed | Complexity | Depends On |
|---------|-------------------|------------|------------|
| 01 Research | 0 (docs only) | Low | -- |
| 02 Implementation | ~40 | Low | 01 |
| 03 Verification | 0 (manual test) | Low | 02 |
| **Total** | **~40** | | |

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Research Findings | `section-01-research-findings.md` | Complete |
| 02 | Implementation | `section-02-implementation.md` | Complete |
| 03 | Verification | `section-03-verification.md` | Complete |
