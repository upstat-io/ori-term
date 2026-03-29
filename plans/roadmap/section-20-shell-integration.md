---
section: 20
title: Shell Integration
status: complete
reviewed: true
last_verified: "2026-03-29"
tier: 4
goal: Shell detection, injection, OSC 7/133 handling, two-parser strategy, prompt state machine
sections:
  - id: "20.1"
    title: Shell Detection
    status: complete
  - id: "20.2"
    title: Shell Injection Mechanisms
    status: complete
  - id: "20.3"
    title: Integration Scripts
    status: complete
  - id: "20.4"
    title: Version Stamping
    status: complete
  - id: "20.5"
    title: Raw Interceptor
    status: complete
  - id: "20.6"
    title: CWD Tracking
    status: complete
  - id: "20.7"
    title: Tab Title Resolution
    status: complete
  - id: "20.8"
    title: Prompt State Machine
    status: complete
  - id: "20.9"
    title: Keyboard Mode Stack Swap
    status: complete
  - id: "20.10"
    title: XTVERSION Response
    status: complete
  - id: "20.11"
    title: Notification Handling
    status: complete
  - id: "20.12"
    title: Semantic Zone Navigation
    status: complete
  - id: "20.13"
    title: Command Completion Notifications
    status: complete
  - id: "20.14"
    title: Section Completion
    status: complete
---

# Section 20: Shell Integration

**Status:** ✅ Complete
**Goal:** Detect the user's shell and inject integration scripts that enable CWD tracking, prompt markers, and notifications. Five shell injection mechanisms, each with different approaches. WSL is a special case (launcher, not shell).

**Crate:** `oriterm` (binary only — no core changes)

**Reference:** `_old/src/shell_integration.rs`, `_old/shell-integration/`, `_old/src/tab/interceptor.rs`

---

## 20.1 Shell Detection

Detect the user's shell from the program path so the correct injection mechanism can be selected.

**File:** `oriterm/src/shell_integration.rs`

**Reference:** `_old/src/shell_integration.rs`

- [x] `Shell` enum: `Bash`, `Zsh`, `Fish`, `PowerShell`, `Wsl` (verified 2026-03-29)
- [x] `detect_shell(program: &str) -> Option<Shell>` — match basename (ignoring `.exe`), handle full paths (verified 2026-03-29)

---

## 20.2 Shell Injection Mechanisms

Each shell requires a different injection strategy. WSL is a special case — it's a launcher, not a shell.

**File:** `oriterm/src/shell_integration.rs`

**Reference:** `_old/src/shell_integration.rs`

- [x] Injection mechanisms (per shell): (verified 2026-03-29)
  | Shell | Method | How |
  |-------|--------|-----|
  | Bash | `--posix` + `ENV` var | Set `ENV=path/to/oriterm.bash`, shell sources it on startup |
  | Zsh | `ZDOTDIR` redirect | Set `ZDOTDIR` to our dir with `.zshenv` that sources integration then restores original `ZDOTDIR` |
  | Fish | `XDG_DATA_DIRS` prepend | Prepend our dir so Fish finds `vendor_conf.d/oriterm-shell-integration.fish` |
  | PowerShell | `ORITERM_PS_PROFILE` env var | User's `$PROFILE` can check and source integration script |
  | WSL | `WSLENV` propagation | Simple env vars only (no path injection across WSL boundary). Users manually source scripts from their `.bashrc`/`.zshrc` |

---

## 20.3 Integration Scripts

The shell integration scripts emit OSC sequences that the terminal intercepts for CWD tracking, prompt marking, and notifications.

**File:** `shell-integration/` directory

**Reference:** `_old/shell-integration/`

- [x] Integration scripts emit: (verified 2026-03-29 -- all 4 scripts contain required OSC sequences)
  - [x] `OSC 7 ; file://hostname/path ST` — current working directory
  - [x] `OSC 133 ; A ST` — prompt start
  - [x] `OSC 133 ; B ST` — command start (user typing)
  - [x] `OSC 133 ; C ST` — output start (command executing)
  - [x] `OSC 133 ; D ST` — command complete
  - [x] `OSC 9` / `OSC 99` / `OSC 777` — notifications (iTerm2 / Kitty / rxvt-unicode)

---

## 20.4 Version Stamping

Prevent stale scripts from persisting after app updates by stamping a version file alongside the integration scripts.

**File:** `oriterm/src/shell_integration.rs`

**Reference:** `_old/src/shell_integration.rs`

- [x] `shell-integration/.version` file contains app version string (verified 2026-03-29)
- [x] On launch: if `.version` matches `env!("CARGO_PKG_VERSION")`, skip writing scripts
- [x] Otherwise: overwrite all shell integration scripts and update `.version`
- [x] Prevents stale scripts from persisting after app updates

---

## 20.5 Raw Interceptor

The high-level VTE processor drops sequences it doesn't recognize. A two-parser strategy catches OSC 7, OSC 133, and other custom sequences before they are lost.

**File:** `oriterm/src/shell_integration.rs`

**Reference:** `_old/src/tab/interceptor.rs`

- [x] The high-level VTE processor (`vte::ansi::Processor`) drops sequences it doesn't recognize (OSC 7, OSC 133, etc.) (verified 2026-03-29)
- [x] Solution: a raw `vte::Parser` with custom `Perform` trait impl runs on the **same bytes** before the high-level processor (verified 2026-03-29)
- [x] Raw interceptor catches: OSC 7 (CWD), OSC 133 (prompts), OSC 9/99/777 (notifications), CSI >q (XTVERSION response) (verified 2026-03-29)
- [x] Both parsers run within the same terminal lock (verified 2026-03-29)
- [x] Interceptor writes to mutable refs on TerminalState fields (no separate struct needed in rebuild — `Term<T>` handles these directly in the VTE handler)

---

## 20.6 CWD Tracking

Track the current working directory via OSC 7 so the tab bar can display it and new tabs can inherit it.

**File:** `oriterm/src/shell_integration.rs`

**Reference:** `_old/src/shell_integration.rs`

- [x] When OSC 7 received: parse `file://hostname/path`, strip prefix, store in `Term.cwd` (verified 2026-03-29 -- 10 tests including percent-encoding, Windows paths, edge cases)
- [x] Mark `title_dirty = true` (CWD change may affect tab bar title)
- [x] If no explicit title (OSC 0/2) was set: tab bar shows short path from CWD

---

## 20.7 Tab Title Resolution

Three sources for tab titles, with strict priority ordering.

**File:** `oriterm/src/shell_integration.rs`

**Reference:** `_old/src/shell_integration.rs`

- [x] Tab title resolution — three sources with priority: (verified 2026-03-29)
  1. [x] Explicit title from OSC 0/2: `has_explicit_title = true`, show `title` field
  2. [x] CWD short path: if `cwd.is_some()` and `!has_explicit_title`, show last component(s) of path
  3. [x] Fallback: static title (e.g., "Tab N")
- [x] `effective_title() -> &str` implements this priority
- [x] When OSC 7 updates CWD: clears `has_explicit_title` so CWD-based title takes over

---

## 20.8 Prompt State Machine

Track prompt lifecycle via OSC 133 sub-parameters. Prompt marking is deferred because the cursor position is updated by the high-level processor, not the raw interceptor.

**File:** `oriterm/src/shell_integration.rs`

**Reference:** `_old/src/shell_integration.rs`

- [x] `PromptState` enum: `None`, `PromptStart`, `CommandStart`, `OutputStart` (verified 2026-03-29)
- [x] Transitions on OSC 133 sub-params (A → B → C → D → None) (verified 2026-03-29)
- [x] `prompt_mark_pending: bool` — when OSC 133;A arrives, set pending. Actual grid row marking happens **after both parsers finish** (deferred), because the cursor position is updated by the high-level processor, not the raw interceptor (verified 2026-03-29 -- uses `PendingMarks` bitflags)
- [x] Prompt lines can be used for: smart selection (select full command), scroll-to-prompt navigation

---

## 20.9 Keyboard Mode Stack Swap

When switching between primary and alt screen, the keyboard mode stack must be swapped so alt-screen apps can use different key encodings without affecting the primary shell.

**File:** `oriterm/src/shell_integration.rs`

**Reference:** `_old/src/shell_integration.rs`

- [x] `keyboard_mode_stack: Vec<KeyboardModes>` — active screen's stack (verified 2026-03-29)
- [x] `inactive_keyboard_mode_stack: Vec<KeyboardModes>` — stashed stack (verified 2026-03-29)
- [x] When switching primary ↔ alt screen (`swap_alt()`): swap the two stacks (verified 2026-03-29)
- [x] Allows alt-screen apps (vim, less) to use different key encodings without affecting the primary shell

---

## 20.10 XTVERSION Response

Respond to XTVERSION queries so that shell integration scripts and applications can detect the terminal emulator.

**File:** `oriterm/src/shell_integration.rs`

**Reference:** `_old/src/shell_integration.rs`

- [x] On CSI >q: generate `DCS > | oriterm(version build N) ST` (verified 2026-03-29)
- [x] Append to VTE response buffer for reader thread to flush outside the terminal lock (verified 2026-03-29 -- sends via `Event::PtyWrite`)

---

## 20.11 Notification Handling

Collect notifications from OSC 9/99/777 sequences and forward them to the OS notification system.

**File:** `oriterm/src/shell_integration.rs`

**Reference:** `_old/src/shell_integration.rs`

- [x] `pending_notifications: Vec<Notification>` — drained by main thread on each Wakeup (verified 2026-03-29)
- [x] `Notification { title: String, body: String }` (verified 2026-03-29)
- [x] OS notification dispatch (platform-specific, stretch goal) (verified 2026-03-29 -- Windows/Linux/macOS all implemented)

---

## 20.12 Semantic Zone Navigation

Expose prompt markers (OSC 133) as user-facing navigation features — jump between prompts, select command output.

**File:** `oriterm/src/app/prompt_nav.rs`

**Reference:** WezTerm `ScrollToPrompt`, Ghostty `scroll-to-prompt`

- [x] **Prompt line tracking:** (verified 2026-03-29)
  - [x] Store prompt positions in grid: `prompt_markers: Vec<PromptMarker>` (absolute row indices for OSC 133 A/B/C)
  - [x] Updated by the prompt state machine (Section 20.8) — when markers received, record cursor row (deferred)
  - [x] Pruned on scrollback eviction (remove markers that fell off the top)
- [x] **Jump to prompt:** (verified 2026-03-29)
  - [x] `PreviousPrompt` action (default: `Ctrl+Shift+ArrowUp`):
    - [x] Find nearest prompt row ABOVE current viewport top (or vi cursor if in vi mode)
    - [x] Scroll viewport to center that prompt row
  - [x] `NextPrompt` action (default: `Ctrl+Shift+ArrowDown`):
    - [x] Find nearest prompt row BELOW current position
    - [x] Scroll viewport to center that prompt row
  - [x] Wrap: at first/last prompt, stop (don't wrap around)
- [x] **Select command output:** (verified 2026-03-29)
  - [x] `SelectCommandOutput` action (default: unbound, available via command palette):
    - [x] From current prompt row, find the next prompt row
    - [x] Select all rows between `OutputStart` (OSC 133;C) and next `PromptStart` (OSC 133;A)
    - [x] Creates a standard selection (Section 09 model) over the output region
  - [x] `SelectCommandInput` action:
    - [x] Select text between `CommandStart` (OSC 133;B) and `OutputStart` (OSC 133;C)
    - [x] Selects just the typed command (useful for copying commands)
- [x] **Visual prompt markers** (optional): (verified 2026-03-29 -- 2px bar, GPU tests confirm emit)
  - [x] Subtle left-margin indicator at prompt lines (thin colored bar, 2px)
  - [x] Config: `behavior.prompt_markers = true | false` (default: false)
- [x] Graceful fallback: if no shell integration / no OSC 133 data, navigation actions are no-ops
- [x] **Tests:**
  - [x] Prompt rows recorded on OSC 133;A
  - [x] PreviousPrompt scrolls to correct row
  - [x] NextPrompt scrolls forward correctly
  - [x] SelectCommandOutput creates selection over correct range
  - [x] No prompts: navigation is no-op (no crash)

---

## 20.13 Command Completion Notifications

Desktop notification when a long-running command finishes in an unfocused tab/window.

**File:** `oriterm/src/app/command_notify.rs`

**Reference:** Ghostty `notify-on-command-finish`, iTerm2 shell integration

- [x] **Command timing:** (verified 2026-03-29)
  - [x] Track command start time: when OSC 133;C (output start) received, record `Instant::now()`
  - [x] Track command end: when OSC 133;D (command complete) received, compute elapsed duration
  - [x] Store: `last_command_duration: Option<Duration>` per pane
- [x] **Notification trigger conditions:** (verified 2026-03-29)
  - [x] Command ran longer than threshold (default: 10 seconds)
  - [x] Pane is NOT focused (unfocused tab or unfocused window)
  - [x] Config: `behavior.notify_on_command_finish` enum:
    - [x] `never` — disabled
    - [x] `unfocused` — only when pane is not focused (default)
    - [x] `always` — always notify, even if focused
  - [x] Config: `behavior.notify_command_threshold_secs = 10` — minimum duration to trigger
- [x] **Notification dispatch:** (verified 2026-03-29)
  - [x] Emit `Notification` (reuse Section 20.11 notification system)
  - [x] Title: `"Command finished"` or pane title
  - [x] Body: `"<command> completed in <duration>"` (command text from shell integration if available)
  - [x] Platform-specific: Windows toast, macOS NSUserNotification, Linux D-Bus notification
- [x] **Tab bell integration:** (verified 2026-03-29)
  - [x] Optionally flash the tab bar for the completed tab (reuse bell pulse)
  - [x] Config: `behavior.notify_command_bell = true | false` (default: true)
- [x] **Tests:**
  - [x] Command < threshold: no notification
  - [x] Command >= threshold + unfocused: notification sent
  - [x] Command >= threshold + focused: no notification (in `unfocused` mode)
  - [x] `never` mode: no notifications regardless
  - [x] `always` mode: notification even when focused

---

## 20.14 Section Completion

- [x] All 20.1–20.13 items complete (verified 2026-03-29 -- 230 tests, all passing)
- [x] Shell detection identifies all five shell types correctly
- [x] Injection mechanisms set correct environment variables per shell
- [x] Integration scripts emit proper OSC 7, OSC 133, and notification sequences
- [x] Version stamping prevents stale scripts
- [x] Two-parser strategy catches all custom sequences without dropping standard VTE output
- [x] CWD tracking updates tab bar title correctly
- [x] Tab title resolution follows 3-source priority (explicit → CWD → fallback)
- [x] Prompt state machine transitions correctly through all OSC 133 sub-params with deferred marking
- [x] Keyboard mode stack swaps correctly on primary ↔ alt screen transitions
- [x] XTVERSION response is correct and flushed outside terminal lock
- [x] `cargo build -p oriterm --target x86_64-pc-windows-gnu` — clean build
- [x] `cargo clippy -p oriterm -p oriterm_core --target x86_64-pc-windows-gnu` — no warnings
- [x] `cargo test -p oriterm_core` — all tests pass

**Exit Criteria:** Shell integration works for all five shell types. CWD tracking, prompt marking, and notifications function correctly. The two-parser strategy catches all custom OSC sequences. Title resolution follows the correct priority chain. Keyboard mode stacks swap cleanly on alt screen transitions.
