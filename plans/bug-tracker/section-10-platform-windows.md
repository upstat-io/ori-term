---
section: 10
title: "Platform Windows"
domain: "oriterm_ui/src/platform_windows/, oriterm_ui/src/window/"
status: in-progress
---

# Section 10: Platform Windows

Bugs in Windows-specific platform integration: DWM, ConPTY, title bar, named pipes, window styles.

## Open Bugs

- [ ] `[BUG-10-1][medium]` **Window does not show Windows theme focus border (accent color)** — found by manual.
  Repro: Focus the oriterm window on Windows 11 with accent color borders enabled in Settings > Personalization > Colors > "Show accent color on title bars and window borders". The window does not display the system accent border; other apps (Explorer, Terminal, etc.) do.
  Subsystem: `oriterm_ui/src/window/mod.rs` (`apply_post_creation_style`) and `oriterm_ui/src/platform_windows/mod.rs` (`enable_snap` / `install_chrome_subclass`)
  Root cause (likely): `apply_post_creation_style` only sets `DWMWA_WINDOW_CORNER_PREFERENCE` (sharp corners). It does not set `DWMWA_BORDER_COLOR` to `DWMWA_COLOR_DEFAULT` (0xFFFFFFFF), which tells DWM to use the system accent color for the window border. Frameless windows with `WS_THICKFRAME` + `DwmExtendFrameIntoClientArea` need this attribute to opt into the theme border. The 1px DWM frame margin is already extended in `install_chrome_subclass`, so setting the border color attribute should be sufficient.
  Found: 2026-03-31 | Source: manual
  Note: Roadmap section 05c (window chrome) covers this area.

## Resolved Bugs

(none yet)
