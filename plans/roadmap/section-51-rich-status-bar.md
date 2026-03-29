---
section: "51"
title: "Rich Status Bar — Shell, CWD, Git Branch, Foreground Process"
status: not-started
reviewed: true
tier: 5
goal: "Status bar displays live shell name, CWD path, git branch, and foreground process name from real terminal state — no hardcoded placeholders."
inspired_by:
  - "WezTerm procinfo/src/lib.rs (LocalProcessInfo, cross-platform process tree)"
  - "WezTerm wezterm-gui/src/tabbar.rs (pane:get_foreground_process_name in title formatting)"
  - "Ptyxis ptyxis-tab-monitor.c (adaptive polling, process leader kind detection)"
depends_on: ["20"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "51.1"
    title: "Shell Name from Spawn"
    status: not-started
  - id: "51.2"
    title: "CWD Display"
    status: not-started
  - id: "51.3"
    title: "Git Branch Detection"
    status: not-started
  - id: "51.4"
    title: "Foreground Process Name"
    status: not-started
  - id: "51.5"
    title: "Status Bar Layout Redesign"
    status: not-started
  - id: "51.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "51.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 51: Rich Status Bar — Shell, CWD, Git Branch, Foreground Process

**Status:** Not Started
**Goal:** The status bar displays live data from the terminal: shell name (left, accent), git branch (left, faint), CWD path (center, faint), and foreground process name. No hardcoded "shell", "UTF-8", or "xterm-256color" placeholders remain.

**Context:** The status bar widget was wired into the production window in the Main Window Brutal plan (Section 04). It currently shows hardcoded placeholders. This section replaces them with real data: shell name from pane spawn, CWD from OSC 7, git branch from `.git/HEAD`, and foreground process name from platform-specific PTY queries.

**Layout (Option A — three sections):**
```
┌──────────────────────────────────────────────────────────────────────────────┐
│ zsh  main   ~/projects/ori_term               80×24 │ UTF-8 │ xterm-256color │
└──────────────────────────────────────────────────────────────────────────────┘
 LEFT: shell + git       CENTER: CWD path                    RIGHT: metadata
 (accent)  (faint)       (faint, centered)                   (faint | accent)

Running a command (fg process replaces shell name):
│ vim  main   ~/projects/ori_term               80×24 │ UTF-8 │ xterm-256color │

No git repo:
│ cmd              C:\Users\eric\Desktop              120×30 │ UTF-8 │ xterm-256color │
```

**Reference implementations:**
- **WezTerm** `procinfo/src/lib.rs`: `LocalProcessInfo` struct with cross-platform process tree reading. Linux reads `/proc/{pid}/stat`, macOS uses `libproc`, Windows uses `CreateToolhelp32Snapshot`.
- **WezTerm** `wezterm-gui/src/tabbar.rs`: `pane:get_foreground_process_name()` used in tab title formatting.
- **Ptyxis** `ptyxis-tab-monitor.c`: Adaptive polling with backoff (100ms interactive → 10s idle) for process monitoring.

**Depends on:** Section 20 (Shell Integration — provides OSC 7 parsing, shell detection, injection scripts).

---

## 51.1 Shell Name from Spawn

**File(s):** `oriterm_mux/src/shell_integration/mod.rs`, `oriterm_mux/src/pane/mod.rs`, `oriterm_mux/src/protocol/snapshot.rs`, `oriterm/src/app/redraw/mod.rs`, `oriterm/src/app/redraw/multi_pane/mod.rs`

The shell type is already detected at spawn time by `detect_shell()` in `oriterm_mux/src/shell_integration/mod.rs`. The result is used for injection script setup but not stored. Store the detected shell name on the `Pane` and propagate it through `PaneSnapshot` to the status bar.

- [ ] Add `pub shell_name: String` field to `Pane` in `oriterm_mux/src/pane/mod.rs`. Set it during pane construction from the spawn command basename (e.g., `/usr/bin/zsh` → `"zsh"`, `C:\Windows\System32\cmd.exe` → `"cmd"`). Use `detect_shell()` result or fall back to command basename.
- [ ] Add `pub shell_name: String` field to `PaneSnapshot` in `oriterm_mux/src/protocol/snapshot.rs`. Populate from `Pane::shell_name` during snapshot refresh.
- [ ] Wire `snapshot.shell_name` into `StatusBarData::shell_name` in both redraw paths (`redraw/mod.rs` and `redraw/multi_pane/mod.rs`), replacing the hardcoded `"shell"` placeholder.
- [ ] **Test: `shell_name_from_spawn`** — Create a pane with a known command path, verify `shell_name` is the basename without extension.
- [ ] **Test: `shell_name_windows_exe_stripped`** — Verify `.exe` is stripped on Windows paths (`cmd.exe` → `cmd`).
- [ ] **Test: `shell_name_in_snapshot`** — Verify `PaneSnapshot` carries the shell name from the pane.

**Validation:** Status bar shows the actual shell name (bash, zsh, fish, cmd, pwsh) instead of "shell".

---

## 51.2 CWD Display

**File(s):** `oriterm_ui/src/widgets/status_bar/mod.rs`, `oriterm/src/app/redraw/mod.rs`, `oriterm/src/app/redraw/multi_pane/mod.rs`

The CWD is already captured via OSC 7 and stored in `PaneSnapshot.cwd`. Wire it to the status bar and add path abbreviation for long paths.

- [ ] Wire `snapshot.cwd` into a new `StatusBarData::cwd` field. Add the field to `StatusBarData` in `oriterm_ui/src/widgets/status_bar/mod.rs`.
- [ ] In both redraw paths, read `snapshot.cwd` from the mux and pass it to `StatusBarData`. When `cwd` is `None`, display nothing (empty string).
- [ ] **Path abbreviation**: Replace `$HOME` prefix with `~`. On Windows, normalize `C:\Users\eric` to `~`. Abbreviate intermediate directories for paths longer than ~40 chars (e.g., `~/p/o/ori_term` or `~/projects/.../ori_term`). Implement as a pure function `abbreviate_path(path: &str, max_chars: usize) -> String` in the status bar module.
- [ ] **Test: `abbreviate_path_home`** — `"/home/eric/projects"` → `"~/projects"`.
- [ ] **Test: `abbreviate_path_long`** — Long path abbreviates intermediate dirs.
- [ ] **Test: `abbreviate_path_short`** — Short paths pass through unchanged.
- [ ] **Test: `abbreviate_path_windows`** — `"C:\\Users\\eric\\Desktop"` → `"~\\Desktop"`.

**Validation:** Status bar shows abbreviated CWD path centered between left and right sections. Updates when the shell changes directory (OSC 7).

---

## 51.3 Git Branch Detection

**File(s):** `oriterm_mux/src/pane/mod.rs` (or new `oriterm_mux/src/pane/git.rs`), `oriterm_mux/src/protocol/snapshot.rs`, `oriterm_ui/src/widgets/status_bar/mod.rs`

When the CWD changes, check for a git repository and read the current branch name. This is a lightweight synchronous file read — `.git/HEAD` is a small file (~30 bytes).

- [ ] Add `pub git_branch: Option<String>` field to `PaneSnapshot`.
- [ ] Implement `detect_git_branch(cwd: &str) -> Option<String>`:
  - Walk up from `cwd` checking for `.git/HEAD` (supports nested repos).
  - Read `.git/HEAD`. If it starts with `ref: refs/heads/`, extract the branch name.
  - If it's a detached HEAD (raw SHA), return first 7 chars as `"abc1234"`.
  - If `.git/HEAD` doesn't exist, return `None`.
  - Cache the result keyed by CWD to avoid re-reading on every frame. Invalidate when CWD changes (OSC 7 event).
- [ ] Add `pub git_branch: Option<String>` to `StatusBarData`. Display with a branch icon (` `) in faint color next to the shell name on the left section.
- [ ] Update status bar `paint_left_items()` to render git branch after shell name when present.
- [ ] **Test: `detect_git_branch_normal`** — Create a temp dir with `.git/HEAD` containing `ref: refs/heads/main\n`, verify returns `Some("main")`.
- [ ] **Test: `detect_git_branch_detached`** — `.git/HEAD` with raw SHA, verify returns first 7 chars.
- [ ] **Test: `detect_git_branch_no_repo`** — No `.git` dir, verify returns `None`.
- [ ] **Test: `detect_git_branch_parent_dir`** — Nested subdir of a repo, verify walks up to find `.git/HEAD`.

**Validation:** Status bar shows ` main` (or current branch) next to the shell name when inside a git repository. Disappears outside git repos.

---

## 51.4 Foreground Process Name

**File(s):** New `oriterm_mux/src/pane/process_info.rs` (platform-specific), `oriterm_mux/src/pane/mod.rs`, `oriterm_mux/src/protocol/snapshot.rs`

Detect the foreground process in the PTY to show the currently running command (e.g., "vim" instead of "zsh" when editing). This is platform-specific.

- [ ] Create `oriterm_mux/src/pane/process_info.rs` with a cross-platform interface:
  ```rust
  /// Query the foreground process name for the given PTY child PID.
  /// Returns `None` if detection is unavailable or fails.
  pub fn foreground_process_name(child_pid: u32) -> Option<String>
  ```
- [ ] **Linux implementation**: `tcgetpgrp(pty_fd)` to get the foreground process group, then read `/proc/{pid}/comm` for the process name. Falls back to `/proc/{pid}/stat` comm field.
- [ ] **macOS implementation**: Similar to Linux — `tcgetpgrp()` + process info lookup via `libproc` or `/proc`-equivalent.
- [ ] **Windows implementation**: Use `CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS)` to walk the process tree from the child PID. Find the leaf process (deepest child). Extract executable basename. <!-- This is the most complex platform — may need `NtQueryInformationProcess` for console process group detection. -->
- [ ] Add `pub foreground_process: Option<String>` to `PaneSnapshot`. When present and different from `shell_name`, the status bar shows this instead of the shell name.
- [ ] **Polling strategy**: Query foreground process during snapshot refresh (already runs on PTY output). No separate polling thread — piggyback on existing snapshot refresh cadence. Cache the result and only re-query when snapshot is dirty.
- [ ] **Graceful degradation**: If platform detection fails or returns `None`, fall back to `shell_name`. Never crash or block on process detection failure.
- [ ] Wire into status bar: when `foreground_process` is `Some(name)` and `name != shell_name`, display `name` instead of `shell_name` in the left section.
- [ ] **Test: `foreground_process_fallback`** — When detection returns `None`, status bar shows shell name.
- [ ] **Test: `foreground_process_replaces_shell`** — When detection returns `"vim"`, status bar shows "vim" instead of "zsh".
- [ ] **Test: `foreground_process_same_as_shell`** — When process name matches shell name, no change in display.

**Validation:** Status bar shows "vim", "htop", "cargo", etc. when a command is running. Reverts to shell name when the command exits. Works on Linux, macOS, and Windows.

---

## 51.5 Status Bar Layout Redesign

**File(s):** `oriterm_ui/src/widgets/status_bar/mod.rs`

Redesign the status bar widget to support the three-section layout: left (shell + git), center (CWD), right (grid + encoding + term type).

- [ ] Refactor `StatusBarWidget::paint()` to use three sections:
  - **Left**: Shell/process name (accent) + git branch icon + name (faint). Items separated by gap.
  - **Center**: CWD path (faint), horizontally centered in the remaining space between left and right.
  - **Right**: Grid size (faint) + encoding (faint) + term type (accent). Items separated by gap, right-aligned.
- [ ] Compute left section width, right section width, then center the CWD in the remaining space. If CWD overflows, truncate with `…`.
- [ ] Add branch icon rendering: use `""` (U+E0A0, Powerline branch symbol) or fall back to a simple text prefix like `on ` if the icon font isn't available. Check if the UI font contains the glyph; if not, use text fallback.
- [ ] Update `StatusBarData` to include `cwd` and `git_branch` fields (if not already added in 51.2/51.3).
- [ ] **Test: `status_bar_three_section_layout`** — WidgetTestHarness test verifying left/center/right positioning.
- [ ] **Test: `status_bar_no_git_no_cwd`** — Verify graceful layout when git branch and CWD are empty.
- [ ] **Test: `status_bar_long_cwd_truncation`** — Verify CWD truncates with `…` when it would overlap left or right sections.
- [ ] Update existing golden tests in `oriterm/src/gpu/visual_regression/status_bar.rs` to reflect the new layout.
- [ ] Update composed golden tests in `oriterm/src/gpu/visual_regression/main_window.rs` if the status bar appearance changes.

**Validation:** Status bar renders with three distinct sections. All existing golden tests updated. New harness tests verify layout behavior.

---

## 51.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 51.N Completion Checklist

- [ ] Shell name populated from pane spawn (no hardcoded "shell")
- [ ] CWD displayed from OSC 7 with path abbreviation
- [ ] Git branch detected from `.git/HEAD`, displayed with icon
- [ ] Foreground process name replaces shell name when a command runs
- [ ] Three-section layout: left (shell+git), center (CWD), right (metadata)
- [ ] Cross-platform: Linux, macOS, Windows all produce correct data
- [ ] Graceful degradation: missing data shows nothing (no "unknown", no crash)
- [ ] Unit tests for all detection functions
- [ ] Golden tests updated for new layout
- [ ] `./build-all.sh` green, `./clippy-all.sh` green, `./test-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** The status bar shows real terminal data — shell name, CWD, git branch, foreground process — populated from the actual running terminal state. No hardcoded placeholders remain.
