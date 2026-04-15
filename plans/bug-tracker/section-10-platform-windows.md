---
section: 10
title: "Platform Windows"
domain: "oriterm_ui/src/platform_windows/, oriterm_ui/src/window/"
status: not-started
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

- [ ] `[BUG-10-2][medium]` **notcurses-demo requires `strace -f -o /dev/null -D` workaround to launch on WSL**
  Repro: Run `notcurses-demo` directly in WSL — it fails or hangs. Workaround: `strace -f -o /dev/null -D notcurses-demo`. This is a known WSL issue where ConPTY interferes with the terminal capabilities that notcurses probes at startup. The `strace` wrapper changes the child's PTY/pipe topology enough to bypass the issue.
  Subsystem: `oriterm_mux/src/pty/`, ConPTY layer (Windows/WSL)
  Impact: ori_term should provide a better UX for WSL users than requiring external workarounds. If ori_term's own ConPTY handling or future raw-pipe bypass can eliminate this class of issue, WSL users get a first-class experience.
  Found: 2026-04-14 | Source: manual
  Note: Roadmap section 52 (Native PTY Layer) and section 53 (Raw Pipe Bypass for VT-Native Shells) directly target ConPTY avoidance for WSL. Section 53's raw pipe transport — bypassing ConPTY entirely for VT-native children — may resolve this class of issue by eliminating ConPTY's VT mangling from the path. Verify once section 53 is implemented.

## Resolved Bugs

(none yet)
