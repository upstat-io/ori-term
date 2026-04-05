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
  - "OSC scenarios validate title (0/2), icon name (1), clipboard (52), color query (4/10/11)"
  - "Edge case scenarios: rapid mode toggles, boundary conditions, erase-with-attributes cross-cutting"
  - "Workflow scenarios run at 80x24, 97x33, and 120x40"
  - "Satisfies mission criteria: multi-sequence workflow coverage and OSC coverage"
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
- [ ] 4+ OSC scenarios validate title, icon name, clipboard, color query
- [ ] 4+ edge case scenarios test boundary conditions and cross-cutting concerns
- [ ] 15+ total workflow + OSC scenarios pass
- [ ] Satisfies mission criteria for multi-sequence workflow coverage and OSC coverage

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
  : Esc ]
  |0;user@host:~|
  : Esc \
  : Esc [ 1 ; 32 m
  |user@host|
  : Esc [ 0 m
  |:|
  : Esc [ 1 ; 34 m
  |~|
  : Esc [ 0 m
  |$ |
  ```
  Note: OSC content uses `|...|` text lines, not inline on `: Esc` control lines (spaces on `: Esc` lines are stripped by reseq). OSC 7 (CWD) is intentionally omitted — it is handled by `RawInterceptor` in `oriterm_mux`, not `Term<T>`, so it would be a silent no-op here.
  Validates: OSC title set, colored prompt with bold+color attributes rendered correctly.

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

Dedicated OSC scenarios covering title (OSC 0/2), icon name (OSC 1), clipboard (OSC 52), and color queries (OSC 4/10/11). OSC 7 (CWD) is tested at the mux layer, not here (see note below).

- [ ] **`osc_title.teseq`** — Set window title via OSC 0 and OSC 2:
  ```
  : Esc ]
  |0;My Terminal Title|
  : Esc \
  : Esc ]
  |2;Window Title Only|
  : Esc \
  ```
  Note: OSC text content MUST be in `|...|` delimiters (teseq text lines), not on `: Esc` control lines. The `: Esc` lines strip spaces between tokens.
  Assert: OSC 0 emits BOTH `RecordedEvent::Title("My Terminal Title")` AND `RecordedEvent::IconName("My Terminal Title")` (per VTE dispatch: OSC 0 sets both title and icon name). OSC 2 emits only `RecordedEvent::Title("Window Title Only")`. Total: 3 events (Title, IconName, Title).

- [ ] **`osc_icon_name.teseq`** — Set icon name via OSC 1:
  ```
  : Esc ]
  |1;My Icon Name|
  : Esc \
  ```
  Assert: `RecordedEvent::IconName("My Icon Name")`.

  **OSC 7 (CWD) is NOT tested here.** OSC 7 is handled by `RawInterceptor` in `oriterm_mux`, not by `Term<T>`. The VTE trait method `set_working_directory` is a default no-op on `Term<T>` — the teseq harness feeds bytes only through `vte::ansi::Processor`, so `Event::Cwd` will never be emitted. CWD is already tested at the mux layer (`oriterm_mux/src/shell_integration/tests.rs::interceptor_osc7_sets_cwd`). Implementing `set_working_directory` on `Term<T>` solely for this test would be a workaround that duplicates the mux's CWD responsibility, violating the crate boundary contract.

- [ ] **`osc_clipboard.teseq`** — Clipboard store via OSC 52:
  ```
  : Esc ]
  |52;c;SGVsbG8=|
  : Esc \
  ```
  Assert: `RecordedEvent::ClipboardStore(Clipboard, "Hello")` (base64-decoded).

- [ ] **`osc_color_query.teseq`** — Query foreground/background/palette colors via OSC 4/10/11:
  ```
  : Esc ]
  |4;1;?|
  : Esc \
  : Esc ]
  |10;?|
  : Esc \
  : Esc ]
  |11;?|
  : Esc \
  ```
  Assert: `RecordedEvent::ColorRequest(1)` for palette index 1 (red, OSC 4), `RecordedEvent::ColorRequest(256)` for foreground (OSC 10, `NamedColor::Foreground as usize = 256`), and `RecordedEvent::ColorRequest(257)` for background (OSC 11, `NamedColor::Background as usize = 257`). The closure is stripped by `RecordedEvent`.

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

- [ ] **`edge_erase_with_attrs.teseq`** — Erase inherits cursor template background:
  ```
  |AAAAAAAAAA|
  : Esc [ 1 ; 5 H
  : Esc [ 42 m
  : Esc [ 0 K
  : Esc [ 0 m
  ```
  Validates: EL 0 (erase right) at col 4 with green background active — erased cells (cols 4-79) should have green bg from the cursor template. Cells before the cursor (cols 0-3) retain original (default) bg. Requires cell attribute inspection (Section 05 helpers). This is the cross-cutting erase+SGR test that Section 02 basic erase scenarios defer.

---

## 06.R Third Party Review Findings

- None.

---

## 06.N Completion Checklist

- [ ] Mode combination workflows: scroll+origin, DECCOLM lifecycle, alt screen+modes (3+ scenarios)
- [ ] Query-response workflows: DA handshake, cursor tracking DSR (2+ scenarios)
- [ ] Real-world pattern workflows: shell prompt, clear+redraw, status bar (3+ scenarios)
- [ ] OSC scenarios: title (0/2), icon name (1), clipboard (52), color query (4/10/11) (4+ scenarios)
- [ ] Edge case scenarios: rapid toggles, zero params, large params, erase-with-attrs (4+ scenarios)
- [ ] Workflow scenarios run at 80x24, 97x33, and 120x40
- [ ] 15+ total workflow + OSC scenarios pass
- [ ] `./build-all.sh` green, `./clippy-all.sh` green
- [ ] `timeout 150 ./test-all.sh` green — no regressions
- [ ] Plan annotation cleanup
- [ ] All TPR checkpoint findings resolved
- [ ] **Plan sync** — update plan metadata:
  - [ ] This section's frontmatter `status` → `complete`
  - [ ] `00-overview.md` Quick Reference table updated
  - [ ] `index.md` section status updated
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed

**Exit Criteria:** `timeout 150 cargo test -p oriterm_core --test teseq -- workflows` and `timeout 150 cargo test -p oriterm_core --test teseq -- osc` pass with 15+ workflow + OSC scenarios. Multi-sequence interactions, query-response handshakes, real-world patterns, OSC events, and edge cases all validated. Mode combination workflows run at 3 terminal sizes. Zero regressions.
