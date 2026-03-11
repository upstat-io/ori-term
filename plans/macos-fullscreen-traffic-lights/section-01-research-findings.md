---
section: "01"
title: "Research Findings"
status: complete
goal: "Document how macOS apps handle fullscreen exit without traffic light repositioning artifacts"
reviewed: true
inspired_by:
  - "Electron WindowButtonsProxy (shell/browser/ui/cocoa/window_buttons_proxy.mm)"
  - "Electron NativeWindowMac (shell/browser/native_window_mac.mm)"
  - "Ghostty TerminalWindow.swift + Fullscreen.swift + TitlebarTabsTahoeTerminalWindow.swift + FullscreenMode+Extension.swift"
  - "WezTerm window/src/os/macos/window.rs"
  - "Alacritty alacritty/src/display/window.rs"
depends_on: []
sections:
  - id: "01.1"
    title: "The Problem"
    status: not-started
  - id: "01.2"
    title: "Reference Implementation Survey"
    status: not-started
  - id: "01.3"
    title: "Approach Comparison"
    status: not-started
  - id: "01.4"
    title: "Recommended Approach"
    status: not-started
---

# Section 01: Research Findings

**Status:** Not Started
**Goal:** Comprehensive documentation of how macOS apps with custom traffic light positioning handle fullscreen exit transitions without visible button repositioning artifacts.

**Context:** When using `NSFullSizeContentViewWindowMask` with `titlebarAppearsTransparent` and custom traffic light positioning (vertically centered in a tab bar), macOS resets the `NSTitlebarContainerView` to its default height during fullscreen exit. This causes a visible "jump" — buttons briefly appear at OS-default positions before repositioning code corrects them. The artifact is a well-known problem across the macOS app ecosystem.

---

## 01.1 The Problem

macOS fullscreen exit follows this internal sequence:

1. `NSWindowWillExitFullScreenNotification` fires
2. macOS rebuilds the `NSTitlebarContainerView` at its **default height** (~28pt standard titlebar)
3. macOS captures an **animation snapshot** of the window (including the rebuilt, default-positioned traffic lights)
4. The slide-back animation plays using the snapshot
5. `NSWindowDidExitFullScreenNotification` fires
6. The window becomes interactive with the rebuilt (default) titlebar

Any repositioning that happens **after step 3** is invisible during the animation but causes a visible jump when the animation completes (step 6). Repositioning during steps 1-2 may or may not be captured in the snapshot depending on timing.

Our current implementation (`fullscreen.rs`) attempts to reposition in steps 1, 2 (via `NSViewFrameDidChangeNotification`), and 6. The frame-change observer fires synchronously when macOS changes the container frame (step 2), and the code comment in `fullscreen.rs` (lines 122-125) claims this "eliminat[es] both the 'bump' and 'pop' artifacts." However, in practice there is a race: if the animation snapshot is captured before or during the frame-change handler, the repositioning is not reflected in the snapshot. The existing code comment is aspirational — the jump artifact is still observed.

**View hierarchy:**
```
NSThemeFrame
  └── NSTitlebarContainerView  (container — we resize this)
        └── NSTitlebarView     (superview of buttons)
              ├── NSButton (close, type 0)
              ├── NSButton (minimize, type 1)
              └── NSButton (zoom, type 2)
```

---

## 01.2 Reference Implementation Survey

### Electron (VS Code, Slack, Discord, etc.) — Hide/Show Pattern

**Files studied:**
- `shell/browser/ui/cocoa/window_buttons_proxy.h` / `.mm`
- `shell/browser/native_window_mac.mm`
- `shell/browser/ui/cocoa/electron_ns_window_delegate.mm`

**Approach:** Hide the entire `NSTitlebarContainerView` before the exit animation, reposition after animation, then show.

**Fullscreen exit sequence:**

```objc
// windowWillExitFullScreen: delegate → NotifyWindowWillLeaveFullScreen()
void NativeWindowMac::NotifyWindowWillLeaveFullScreen() {
  if (buttons_proxy_) {
    [window_ setTitleVisibility:NSWindowTitleHidden];
    // Hide the container otherwise traffic light buttons jump.
    [buttons_proxy_ setVisible:NO];  // [titleBarContainer setHidden:YES]
  }
}

// windowDidExitFullScreen: delegate → NotifyWindowLeaveFullScreen()
void NativeWindowMac::NotifyWindowLeaveFullScreen() {
  if (buttons_proxy_ && window_button_visibility_.value_or(true)) {
    [buttons_proxy_ redraw];          // reposition container + buttons
    [buttons_proxy_ setVisible:YES];  // [titleBarContainer setHidden:NO]
  }
  if (transparent() || !has_frame())
    [window_ setTitlebarAppearsTransparent:YES];
}
```

**`setVisible:` implementation:**
```objc
- (void)setVisible:(BOOL)visible {
  NSView* titleBarContainer = [self titleBarContainer];
  if (!titleBarContainer) return;
  [titleBarContainer setHidden:!visible];
}
```

**`redraw` implementation:** Resizes container to custom height, positions each button with `setFrameOrigin:`, supports RTL layouts and custom margins/heights.

**Additional safety net:** `RedrawTrafficLights()` is called from `windowDidResize:`, `windowDidBecomeMain:`, `windowDidResignMain:`, `windowDidBecomeKey:`, `windowDidResignKey:`. It guards against fullscreen: `if (!IsFullscreen()) [buttons_proxy_ redraw]`.

**macOS 26 note:** Electron discovered that toggling button hidden state on macOS 26 (Tahoe) causes AppKit to re-layout the container and reset its frame. They added a `[self redraw]` call after every visibility change to counteract this.

**Verdict:** Production-proven across millions of users. Completely eliminates the jump artifact.

---

### Ghostty — No Special Handling for Visible Traffic Lights

**Files studied:**
- `macos/Sources/Features/Terminal/Window Styles/TerminalWindow.swift`
- `macos/Sources/Features/Terminal/Window Styles/TitlebarTabsVenturaTerminalWindow.swift`
- `macos/Sources/Features/Terminal/Window Styles/TitlebarTabsTahoeTerminalWindow.swift`
- `macos/Sources/Features/Terminal/Window Styles/TransparentTitlebarTerminalWindow.swift`
- `macos/Sources/Features/Terminal/Window Styles/HiddenTitlebarTerminalWindow.swift`
- `macos/Sources/Helpers/Fullscreen.swift`
- `macos/Sources/Ghostty/FullscreenMode+Extension.swift`

**Approach:** Ghostty does NOT reposition traffic lights to custom positions. For native fullscreen, it simply calls `toggleFullScreen()` and lets macOS handle everything. For non-native fullscreen, it removes the `.titled` style mask (which removes traffic lights entirely) and restores it on exit.

**Key insight — `titlebarContainer` during fullscreen:** When in native fullscreen, the titlebar container moves to a separate `NSToolbarFullScreenWindow`, not the main window:

```swift
var titlebarContainer: NSView? {
    if !styleMask.contains(.fullScreen) {
        return contentView?.firstViewFromRoot(withClassName: "NSTitlebarContainerView")
    }
    for window in NSApplication.shared.windows {
        guard window.className == "NSToolbarFullScreenWindow" else { continue }
        guard window.parent == self else { continue }
        return window.contentView?.firstViewFromRoot(withClassName: "NSTitlebarContainerView")
    }
    return nil
}
```

**For hidden titlebar style:** Permanently hides the entire `NSTitlebarContainerView` (discovered via `contentView?.superview` (NSThemeFrame) then `firstDescendant(withClassName:)`) and re-hides it after fullscreen exit via a `fullscreenDidExit` notification callback that calls `reapplyHiddenStyle()`.

**Verdict:** Not applicable to our use case since Ghostty doesn't reposition traffic lights to custom positions for visible-titlebar styles.

---

### WezTerm — No Fullscreen Transition Handling

**File studied:** `window/src/os/macos/window.rs`

**Approach:** WezTerm accesses the `NSTitlebarContainerView` for background color customization but does NOT implement `windowWillExitFullScreen:` or `windowDidExitFullScreen:` delegate methods. Has two fullscreen modes:

- **Native fullscreen:** `NSWindow::toggleFullScreen_()` — no button repositioning
- **Simple fullscreen:** Manual frame save/restore with `NSBorderlessWindowMask` — no titlebar at all

**Titlebar container discovery:**
```rust
fn get_titlebar_view_container(window: &StrongPtr) -> Option<WeakPtr> {
    // Traverse: contentView → superview → enumerate subviews → find "NSTitlebarContainerView"
}
const TITLEBAR_VIEW_NAME: &str = "NSTitlebarContainerView";
```

**Verdict:** Not applicable — WezTerm doesn't reposition traffic lights.

---

### Alacritty — No Custom Traffic Light Positioning

**File studied:** `alacritty/src/display/window.rs`

**Approach:** Alacritty uses winit's `with_titlebar_transparent(true)` + `with_fullsize_content_view(true)` for its "Transparent" decoration mode but does NOT reposition traffic lights at all. Fullscreen uses `set_fullscreen(Fullscreen::Borderless(None))` or `set_simple_fullscreen()`.

**Decoration modes:** `Full` (standard), `Transparent` (transparent titlebar), `Buttonless` (hidden buttons), `None` (hidden titlebar).

**Verdict:** Not applicable — Alacritty doesn't reposition traffic lights.

---

### Tauri / Community Solutions — Resize Listener (Insufficient)

Many community solutions (Tauri, Flutter macos_window_utils, etc.) reposition buttons in resize event handlers:

```rust
.on_window_event(|e| {
    if let WindowEvent::Resized(..) = e.event() {
        win.position_traffic_lights(30., 30.);
    }
})
```

The Tauri community explicitly acknowledged "artifacts on resize" remain unsolved. The resize event fires **after** the animation, so the jump is still visible.

**Verdict:** Insufficient — repositioning after resize does not prevent the animation-snapshot artifact.

---

### Custom Fullscreen Animation (Apple API)

Apple provides `customWindowsToExitFullScreen(for:)` and `window:startCustomAnimationToExitFullScreenWithDuration:` delegate methods to fully control the exit animation. This would allow pre-positioning buttons before any animation.

**Limitation:** winit owns the `NSWindowDelegate` and does not expose these methods. Implementing this requires either patching winit or interposing a custom delegate — significant complexity for marginal benefit over the hide/show approach.

**Verdict:** Correct but impractical given winit's delegate ownership.

---

### removeFromSuperview + re-add (steve228uk pattern)

Move buttons from the native hierarchy into a custom view:

```swift
override func viewDidLayout() {
    if let btn = view.window?.standardWindowButton(.closeButton) {
        btn.removeFromSuperview()
        btn.setFrameOrigin(NSPoint(x: 12, y: 28))
        view.addSubview(btn)
    }
}
```

**Problems:** Breaks Auto Layout expectations, may not survive fullscreen transitions (macOS expects buttons in standard hierarchy), can cause layout warnings.

**Verdict:** Not recommended — fragile and breaks AppKit assumptions.

---

## 01.3 Approach Comparison

| Approach | Prevents Jump | Complexity | Production-Proven | Fragility |
|----------|--------------|------------|-------------------|-----------|
| **Hide/show container (Electron)** | Yes | Low | Yes (millions of users) | Low — uses documented `setHidden:` |
| **Frame-change observer (current)** | Partial | Medium | No (still has artifacts) | Medium — race with snapshot timing |
| **Custom exit animation (Apple API)** | Yes | High | N/A (not possible with winit) | High — winit delegate conflict |
| **Resize listener (Tauri)** | No | Low | No (artifacts acknowledged) | Low |
| **removeFromSuperview** | Partial | Low | No | High — breaks Auto Layout |

---

## 01.4 Recommended Approach

**Adopt Electron's hide/show pattern.** This is the only approach that:

1. Completely eliminates the jump artifact
2. Is production-proven at massive scale
3. Has low implementation complexity (~40 lines changed)
4. Works with our existing notification-based architecture

**Implementation plan:**

1. In `handle_will_exit_fs`: After centering (current code), **also hide the titlebar container** via `[container setHidden: true]`
2. In `handle_did_exit_fs`: Center buttons (current code), then **show the container** via `[container setHidden: false]`
3. Keep the `NSViewFrameDidChangeNotification` observer as a safety net for centering while hidden
4. Keep the `CATransaction` disable-animations wrapping for non-fullscreen resize centering

**Trade-off:** During the fullscreen exit animation, the traffic lights will be invisible (the animation snapshot shows no buttons). They appear at the correct position only after the animation completes. This matches Electron's behavior and is visually clean — better than buttons jumping.

**Alternative considered:** We could try to make buttons visible during the animation by hiding + centering + showing all within `handle_will_exit_fs` (before the snapshot). This is riskier because the snapshot timing is not documented and may vary across macOS versions. The Electron team explicitly chose the hide-during-animation approach after testing alternatives.

---

## 01.5 Completion Checklist

- [ ] Seven reference implementations surveyed (Electron, Ghostty, WezTerm, Alacritty, Tauri, custom animation API, removeFromSuperview) with source file paths and code snippets where applicable
- [ ] Root cause explained: macOS captures an animation snapshot after rebuilding `NSTitlebarContainerView` at default height, and any repositioning after the snapshot creates a visible jump
- [ ] Comparison table covers five viable approaches with columns: prevents jump, complexity, production-proven, fragility
- [ ] Electron hide/show recommended with explicit justification: eliminates artifact, production-proven, low complexity, compatible with existing notification architecture

**Exit Criteria:** Section serves as a reference document. No code changes required.
