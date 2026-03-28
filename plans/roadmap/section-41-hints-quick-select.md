---
section: 41
title: Hints + Quick Select
status: not-started
reviewed: false
third_party_review:
  status: none
  updated: null
tier: 3
goal: "Regex-based labeled text selection for URLs, paths, git hashes, IPs, and custom patterns. Keyboard-driven quick copy without mouse."
sections:
  - id: "41.1"
    title: Pattern Registry
    status: not-started
  - id: "41.2"
    title: Label Assignment + Rendering
    status: not-started
  - id: "41.3"
    title: Hint Actions
    status: not-started
  - id: "41.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "41.4"
    title: Section Completion
    status: not-started
---

# Section 41: Hints + Quick Select

**Status:** Not Started
**Goal:** Detect text patterns (URLs, file paths, git hashes, IP addresses, etc.) in the visible terminal and overlay keyboard-selectable labels on each match. Type the label to copy, open, or act on the matched text. Combines Alacritty's hints system with WezTerm's quick select for the best of both.

**Crate:** `oriterm` (pattern matching, overlay rendering, input dispatch)
**Dependencies:** Section 14 (URL detection — shared regex infrastructure), Section 08 (keyboard dispatch), Section 06 (text rendering for labels)
**Prerequisite:** Sections 08, 14 complete.

**Reference:**
- Alacritty hints: `alacritty/src/config/ui_config.rs` (hint config), `alacritty/src/display/hint.rs` (label assignment + rendering)
- WezTerm QuickSelect: `wezterm-gui/src/termwindow/quickselect.rs` (pattern matching, label rendering)
- Kitty hints kitten

**Why this matters:** Copying a URL or git hash from terminal output currently requires: (1) reach for mouse, (2) carefully select text, (3) copy. With hints, it's: (1) press hotkey, (2) type 1-2 characters. This is dramatically faster for the most common terminal interaction pattern.

---

## 41.1 Pattern Registry

Configurable regex patterns that define what text to highlight in hints mode.

**File:** `oriterm/src/hints/patterns.rs`

- [ ] `HintPattern` struct:
  - [ ] `name: String` — human-readable name (e.g., "url", "path", "hash")
  - [ ] `regex: Regex` — compiled regex pattern
  - [ ] `action: HintAction` — what happens when selected
  - [ ] `binding: Option<KeyBinding>` — optional dedicated keybinding to show only this pattern
- [ ] `HintAction` enum:
  - [ ] `Copy` — copy matched text to clipboard
  - [ ] `Open` — open in system handler (browser for URLs, editor for paths)
  - [ ] `CopyAndPaste` — copy to clipboard AND paste into terminal
  - [ ] `Select` — create a selection over the match (for further action)
- [ ] Built-in patterns (always available):
  - [ ] **URL**: `(?:https?|ftp|file)://[^\s<>\[\]'"]+` — HTTP(S), FTP, file URLs
  - [ ] **File path**: `(?:/[\w.-]+)+` and `(?:[A-Z]:\\[\w\\.-]+)+` — Unix and Windows paths
  - [ ] **Git hash**: `\b[0-9a-f]{7,40}\b` — short and full SHA hashes
  - [ ] **IP address**: `\b(?:\d{1,3}\.){3}\d{1,3}\b` — IPv4 addresses
  - [ ] **Email**: `[\w.+-]+@[\w.-]+\.\w{2,}` — email addresses
  - [ ] **Number**: `\b\d+\b` — bare numbers (useful for PIDs, ports, line numbers)
- [ ] User-configurable patterns via TOML:
  ```toml
  [[hints.pattern]]
  name = "jira"
  regex = "[A-Z]+-\\d+"
  action = "open"
  url_template = "https://jira.example.com/browse/{0}"

  [[hints.pattern]]
  name = "docker"
  regex = "[0-9a-f]{12}"
  action = "copy"
  ```
- [ ] `url_template` — optional URL template for `Open` action on non-URL patterns
  - [ ] `{0}` replaced with matched text
  - [ ] Enables opening JIRA tickets, GitHub issues, etc. from pattern matches
- [ ] Pattern compilation at config load time (fail fast on invalid regex)
- [ ] **Tests:**
  - [ ] URL pattern matches `https://example.com`
  - [ ] Path pattern matches `/home/user/file.txt` and `C:\Users\file.txt`
  - [ ] Git hash pattern matches 7-40 char hex strings
  - [ ] IP pattern matches `192.168.1.1`
  - [ ] Custom pattern from config compiles and matches
  - [ ] URL template substitution works

---

## 41.2 Label Assignment + Rendering

When hints mode is activated, scan the viewport for pattern matches and assign short keyboard labels to each.

**File:** `oriterm/src/hints/labels.rs`, `oriterm/src/hints/render.rs`

**Reference:** Alacritty `alacritty/src/display/hint.rs` (label generation)

- [ ] Label alphabet: configurable (default: `asdfjkl;ghqwertyuiopzxcvbnm`)
  - [ ] Single-character labels for first N matches (where N = alphabet length)
  - [ ] Two-character labels for overflow (e.g., `aa`, `as`, `ad`, ...)
  - [ ] Prioritize single-char labels for matches closer to cursor
- [ ] Viewport scanning:
  - [ ] On hints mode activation: scan all visible rows for pattern matches
  - [ ] Run each enabled pattern's regex across each logical line
  - [ ] Collect all matches with their grid positions (row, start_col, end_col)
  - [ ] Sort by distance from cursor (nearest first — gets shortest labels)
  - [ ] Deduplicate overlapping matches (longest match wins)
- [ ] Label rendering:
  - [ ] Overlay label text at the start of each match
  - [ ] Label style: bold, contrasting color (e.g., yellow on black or theme-aware)
  - [ ] Dim non-matching terminal content (reduce opacity or desaturate)
  - [ ] Label replaces the first 1-2 characters of the match visually
  - [ ] If match is shorter than label: extend label past match end
- [ ] Progressive filtering:
  - [ ] As user types label characters, filter visible hints
  - [ ] Non-matching hints disappear immediately
  - [ ] Matching hints update: consumed characters removed from label display
  - [ ] When only one match remains: auto-trigger action
  - [ ] When label fully typed: trigger action for that match
- [ ] Hints mode exit:
  - [ ] `Escape` — cancel hints mode, no action
  - [ ] Label completed — action triggered, exit hints mode
  - [ ] Any non-label key — cancel hints mode
- [ ] Config:
  ```toml
  [hints]
  alphabet = "asdfjkl;ghqwertyuiopzxcvbnm"
  show_all = false  # true = show all patterns; false = show keybinding-specific
  ```
- [ ] **Tests:**
  - [ ] Label generation: 26 matches get single-char labels
  - [ ] Label generation: 27+ matches get two-char labels for overflow
  - [ ] Progressive filtering: typing 'a' hides hints not starting with 'a'
  - [ ] Auto-trigger: last remaining hint triggers action
  - [ ] Escape cancels without action

---

## 41.3 Hint Actions

Execute the configured action when a hint is selected.

**File:** `oriterm/src/hints/actions.rs`

- [ ] `Copy` action:
  - [ ] Copy matched text to system clipboard
  - [ ] Brief visual feedback: flash the match (highlight for 200ms)
- [ ] `Open` action:
  - [ ] URLs: open in system browser (reuse Section 14 URL opening)
  - [ ] File paths: open in system default handler (or configured editor)
  - [ ] Custom `url_template`: substitute match text and open result
  - [ ] Validate scheme before opening (security: no `javascript:`)
- [ ] `CopyAndPaste` action:
  - [ ] Copy to clipboard AND send matched text to PTY
  - [ ] Useful for: `cd <path>`, `git show <hash>`, etc.
  - [ ] Respects bracketed paste mode
- [ ] `Select` action:
  - [ ] Create a selection over the matched text region
  - [ ] Selection persists after hints mode exits
  - [ ] User can then copy, extend, or dismiss
- [ ] Keybinding integration:
  - [ ] Default: `Ctrl+Shift+H` — activate hints mode (all patterns)
  - [ ] Per-pattern bindings: `Ctrl+Shift+U` for URLs only, etc.
  - [ ] When per-pattern binding used: only that pattern's matches shown
- [ ] **Tests:**
  - [ ] Copy action places text on clipboard
  - [ ] Open action launches URL in browser
  - [ ] CopyAndPaste writes to PTY
  - [ ] Select action creates selection over match
  - [ ] Per-pattern keybinding filters to one pattern type

---

## 41.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 41.4 Section Completion

- [ ] All 41.1–41.3 items complete
- [ ] Built-in patterns detect URLs, paths, git hashes, IPs, emails
- [ ] Custom patterns configurable via TOML
- [ ] Labels assigned by proximity to cursor (nearest = shortest label)
- [ ] Progressive filtering narrows hints as user types
- [ ] Copy, Open, CopyAndPaste, Select actions all work
- [ ] URL template substitution enables custom integrations (JIRA, GitHub, etc.)
- [ ] Hints mode visually dims non-matching content
- [ ] `Escape` cancels cleanly, label completion triggers action
- [ ] `cargo clippy --target x86_64-pc-windows-gnu` — no warnings
- [ ] `cargo test` — all hint tests pass

- [ ] `/tpr-review` passed — independent Codex review found no critical or major issues (or all findings triaged)

**Exit Criteria:** Users can press a hotkey, see labeled matches for URLs/hashes/paths, type 1-2 characters, and immediately copy or open the matched text. Dramatically faster than mouse selection for the most common terminal copy patterns.
