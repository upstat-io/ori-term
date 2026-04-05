---
section: "06"
title: "Complex Workflow Scenarios"
status: not-started
reviewed: false
goal: "Create multi-sequence workflow scenarios testing real-world terminal interaction patterns that no existing test surface covers"
success_criteria:
  - "Scroll region + origin mode workflow validates complete interaction chain"
  - "Alt screen enter/exit with content preservation workflow passes"
  - "DECCOLM 80→132→80 transition workflow validates grid resize chain"
  - "DA handshake workflow validates query→response→continuation sequence"
  - "Shell prompt simulation workflow exercises common shell escape sequence patterns"
  - "Edge case scenarios: malformed sequences, rapid mode toggles, boundary conditions"
  - "Workflow scenarios run at 80x24, 97x33, and 120x40"
  - "Satisfies mission criteria: multi-sequence workflow coverage"
inspired_by:
  - "Alacritty ref tests (alacritty_terminal/tests/ref/) — real-world recordings (tmux_git_log, vim_simple_edit)"
  - "ori_term vttest integration — multi-step menu navigation as workflow testing"
  - "Ghostty fuzz corpus (test/fuzz-libghostty/corpus/) — edge case byte sequences"
depends_on: ["01", "02", "03", "04", "05"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "06.1"
    title: "Mode Combination Workflows"
    status: not-started
  - id: "06.2"
    title: "Query-Response Workflows"
    status: not-started
  - id: "06.3"
    title: "Real-World Pattern Workflows"
    status: not-started
  - id: "06.4"
    title: "OSC Scenarios"
    status: not-started
  - id: "06.5"
    title: "Edge Case Scenarios"
    status: not-started
  - id: "06.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "06.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 06: Complex Workflow Scenarios

**Status:** Not Started
**Goal:** The highest-value scenarios — multi-sequence workflows that exercise real-world terminal interaction patterns. These test the *combination* of features across multiple escape sequence families, covering interaction patterns that no individual test surface addresses.

**Success Criteria:**

- [ ] 4+ mode combination workflows pass at multiple sizes
- [ ] 2+ query-response workflows validate full handshake sequences
- [ ] 3+ real-world pattern workflows exercise common terminal usage
- [ ] 3+ edge case scenarios test boundary conditions
- [ ] 12+ total workflow scenarios pass
- [ ] Satisfies mission criteria for multi-sequence workflow coverage

**Context:** Existing test surfaces cover isolated sequences (handler tests) and black-box vttest conformance (vttest tests). The gap is *authored multi-sequence interactions* — scenarios where you deliberately construct a sequence of operations and verify the cumulative effect. This is where real bugs hide: mode A works, mode B works, but A→B→A produces unexpected state.

**Reference implementations:**
- **Alacritty** `tests/ref/`: Real-world recordings (`tmux_git_log`, `vim_simple_edit`, `zsh_tab_completion`) capture actual terminal usage
- **ori_term** vttest menu navigation: Multi-step sequences with assertions between steps
- **Ghostty** fuzz corpus: Evolved byte sequences that found parser bugs

**Depends on:** All scenario sections (01-05) — workflows combine patterns from each.

---

## 06.1 Mode Combination Workflows

**File(s):** `oriterm_core/tests/teseq/scenarios/workflows/mode_*.teseq`, `oriterm_core/tests/teseq/workflows.rs`

- [ ] **`mode_scroll_origin_fill.teseq`** — Complete scroll region + origin mode workflow:
  ```
  : Esc [ 5 ; 20 r
  : Esc [ ? 6 h
  : Esc [ 1 ; 1 H
  |Row 01|.
  |Row 02|.
  |Row 03|.
  |Row 04|.
  |Row 05|.
  |Row 06|.
  |Row 07|.
  |Row 08|.
  |Row 09|.
  |Row 10|.
  |Row 11|.
  |Row 12|.
  |Row 13|.
  |Row 14|.
  |Row 15|.
  |Row 16|.
  |Row 17|.
  |Row 18|.
  : Esc [ ? 6 l
  : Esc [ 1 ; 1 H
  |After origin off|
  ```
  Validates: 16-line scroll region overflows, origin mode cursor stays within region, disabling origin mode returns to absolute positioning.

- [ ] **`mode_deccolm_full_cycle.teseq`** — Complete DECCOLM lifecycle:
  ```
  |Original 80-col content|.
  : Esc [ ? 3 h
  |132-col: AAAAAA...(long line)...|.
  : Esc [ ? 6 h
  : Esc [ 5 ; 20 r
  |In origin mode at 132|.
  : Esc [ ? 3 l
  |Back to 80|
  ```
  `mode_deccolm_full_cycle.toml`:
  ```toml
  [setup]
  pre_feed = ["\\x1b[?40h"]
  ```
  Validates: 80→132 clears, origin mode works at 132 columns, 132→80 clears and resets.

- [ ] **`mode_alt_with_modes.teseq`** — Alt screen with modes active:
  ```
  : Esc [ 5 ; 20 r
  : Esc [ ? 6 h
  : Esc [ 3 ; 10 H
  |Primary with modes|.
  : Esc [ ? 1049 h
  |Alt screen|.
  : Esc [ ? 1049 l
  ```
  Validates: Alt screen preserves scroll region and origin mode settings on return.

- [ ] Multi-size variants: all mode workflows run at 80x24, 97x33, and 120x40.

---

## 06.2 Query-Response Workflows

**File(s):** `oriterm_core/tests/teseq/scenarios/workflows/query_*.teseq`

Multi-step query/response sequences that simulate real terminal handshakes.

- [ ] **`query_da_handshake.teseq`** — Full DA negotiation:
  ```
  : Esc [ c
  : Esc [ > c
  : Esc [ = c
  ```
  Validates: All three DA responses emitted in order. Response analysis via teseq shows correct device attribute format.

- [ ] **`query_cursor_tracking.teseq`** — DSR after each cursor movement:
  ```
  : Esc [ 5 ; 10 H
  : Esc [ 6 n
  : Esc [ 3 A
  : Esc [ 6 n
  : Esc [ 20 C
  : Esc [ 6 n
  ```
  Validates: Each DSR response encodes the correct cursor position after the preceding movement. Three PtyWrite events with progressively updated coordinates.

- [ ] **TPR checkpoint** — `/tpr-review` covering 06.1–06.2 implementation work

---

## 06.3 Real-World Pattern Workflows

**File(s):** `oriterm_core/tests/teseq/scenarios/workflows/real_*.teseq`

Scenarios that mimic common terminal application patterns.

- [ ] **`real_shell_prompt.teseq`** — Typical shell prompt escape sequence pattern:
  ```
  : Esc ] 0 ; user@host:~ Esc \
  : Esc ] 7 ; file:///home/user Esc \
  : Esc [ 1 ; 32 m
  |user@host|
  : Esc [ 0 m
  |:|
  : Esc [ 1 ; 34 m
  |~|
  : Esc [ 0 m
  |$ |
  ```
  Validates: OSC title set, OSC CWD set, colored prompt rendered correctly.

- [ ] **`real_clear_and_redraw.teseq`** — Application clears screen and redraws:
  ```
  |Old content line 1|.
  |Old content line 2|.
  : Esc [ 2 J
  : Esc [ 1 ; 1 H
  |New content line 1|.
  |New content line 2|.
  ```
  Validates: ED 2 clears, CUP homes, new content replaces old.

- [ ] **`real_status_bar.teseq`** — Application draws a status bar at bottom:
  ```
  : Esc [ 24 ; 1 H
  : Esc [ 7 m
  | Status: OK                                                                     |
  : Esc [ 0 m
  : Esc [ 1 ; 1 H
  |Main content area|
  ```
  Validates: Cursor positioning to last row, inverse attribute for status bar, return to content area.

---

## 06.4 OSC Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/osc/*.teseq`, `oriterm_core/tests/teseq/osc.rs`

Dedicated OSC scenarios covering title, CWD, clipboard, and color queries. These satisfy the mission success criteria for OSC coverage.

- [ ] **`osc_title.teseq`** — Set window title via OSC 0 and OSC 2:
  ```
  : Esc ] 0 ; My Terminal Title Esc \
  : Esc ] 2 ; Window Title Only Esc \
  ```
  Assert: `RecordedEvent::Title("My Terminal Title")`, then `RecordedEvent::Title("Window Title Only")`.

- [ ] **`osc_cwd.teseq`** — Set current working directory via OSC 7:
  ```
  : Esc ] 7 ; file:///home/user/project Esc \
  ```
  Assert: `RecordedEvent::Cwd("/home/user/project")`.

- [ ] **`osc_clipboard.teseq`** — Clipboard store via OSC 52:
  ```
  : Esc ] 52 ; c ; SGVsbG8= Esc \
  ```
  Assert: `RecordedEvent::ClipboardStore(Clipboard, "Hello")` (base64-decoded).

- [ ] **`osc_color_query.teseq`** — Query foreground/background colors via OSC 10/11:
  ```
  : Esc ] 10 ; ? Esc \
  : Esc ] 11 ; ? Esc \
  ```
  Assert: `RecordedEvent::ColorRequest` events emitted.

- [ ] Register family module `osc.rs` in `main.rs`.

---

## 06.5 Edge Case Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/workflows/edge_*.teseq`

Boundary conditions and unusual sequences.

- [ ] **`edge_rapid_mode_toggle.teseq`** — Rapidly toggle origin mode:
  ```
  : Esc [ ? 6 h
  : Esc [ ? 6 l
  : Esc [ ? 6 h
  : Esc [ ? 6 l
  : Esc [ 5 ; 10 H
  |After toggles|
  ```
  Validates: Rapid mode toggling doesn't corrupt state. Cursor at correct position.

- [ ] **`edge_zero_params.teseq`** — CSI with zero/missing parameters:
  ```
  : Esc [ 0 ; 0 H
  |At origin via zeros|.
  : Esc [ H
  |At origin via omit|.
  : Esc [ 0 A
  |CUU zero|
  ```
  Validates: Zero and omitted params treated as 1 (per ECMA-48).

- [ ] **`edge_large_params.teseq`** — CSI with very large parameters:
  ```
  : Esc [ 99999 ; 99999 H
  |Clamped|
  : Esc [ 99999 A
  |Top|
  ```
  Validates: Large params clamped to grid boundaries without panic.

---

## 06.R Third Party Review Findings

- None.

---

## 06.N Completion Checklist

- [ ] Mode combination workflows: scroll+origin, DECCOLM lifecycle, alt screen+modes (3+ scenarios)
- [ ] Query-response workflows: DA handshake, cursor tracking DSR (2+ scenarios)
- [ ] Real-world pattern workflows: shell prompt, clear+redraw, status bar (3+ scenarios)
- [ ] Edge case scenarios: rapid toggles, zero params, large params (3+ scenarios)
- [ ] Workflow scenarios run at 80x24, 97x33, and 120x40
- [ ] 12+ total workflow scenarios pass
- [ ] `./build-all.sh` green, `./clippy-all.sh` green
- [ ] `./test-all.sh` green — no regressions
- [ ] Plan annotation cleanup
- [ ] All TPR checkpoint findings resolved
- [ ] **Plan sync** — update plan metadata
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed

**Exit Criteria:** `cargo test -p oriterm_core --test teseq -- workflows` passes with 12+ workflow scenarios. Multi-sequence interactions, query-response handshakes, real-world patterns, and edge cases all validated. Mode combination workflows run at 3 terminal sizes. Zero regressions.
