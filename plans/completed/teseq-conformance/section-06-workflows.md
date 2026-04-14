---
section: "06"
title: "Complex Workflow Scenarios"
status: complete
reviewed: true
goal: "Create multi-sequence workflow scenarios testing real-world terminal interaction patterns that no existing test surface covers"
success_criteria:
  - "Scroll region + origin mode workflow validates complete interaction chain"
  - "Alt screen enter/exit with content preservation workflow passes"
  - "DECCOLM 80→132→80 transition workflow validates grid resize chain"
  - "DECSC attribute save/restore workflow validates SGR + charset preservation (deferred from 02.5)"
  - "DA handshake workflow validates query→response→continuation sequence"
  - "Shell prompt simulation workflow exercises common shell escape sequence patterns"
  - "Charset switching workflow validates G0/G1 designation + SO/SI in realistic sequence"
  - "OSC scenarios validate title (0/2), icon name (1), clipboard (52), color query (4/10/11)"
  - "Edge case scenarios: rapid mode toggles, boundary conditions, erase-with-attributes cross-cutting"
  - "Mode combination workflows run at 80x24, 97x33, and 120x40 (separate .teseq/.toml per size)"
  - "Satisfies mission criteria: multi-sequence workflow coverage, OSC coverage, and ESC workflow coverage"
inspired_by:
  - "Alacritty ref tests (alacritty_terminal/tests/ref/) — real-world recordings (tmux_git_log, vim_simple_edit)"
  - "ori_term vttest integration — multi-step menu navigation as workflow testing"
  - "Ghostty fuzz corpus (test/fuzz-libghostty/corpus/) — edge case byte sequences"
depends_on: ["01", "02", "03", "04", "05"]
third_party_review:
  status: resolved
  updated: 2026-04-06
sections:
  - id: "06.1"
    title: "Mode Combination Workflows"
    status: complete
  - id: "06.2"
    title: "Query-Response Workflows"
    status: complete
  - id: "06.3"
    title: "Real-World Pattern Workflows"
    status: complete
  - id: "06.4"
    title: "OSC Scenarios"
    status: complete
  - id: "06.5"
    title: "Edge Case Scenarios"
    status: complete
  - id: "06.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "06.N"
    title: "Completion Checklist"
    status: complete
---

# Section 06: Complex Workflow Scenarios

**Status:** Not Started
**Goal:** The highest-value scenarios — multi-sequence workflows that exercise real-world terminal interaction patterns. These test the *combination* of features across multiple escape sequence families, covering interaction patterns that no individual test surface addresses.

**Success Criteria:**

- [x] 5 mode combination workflows pass at 80x24 base size (including DECSC attribute save/restore)
- [x] Mode combination multi-size variants: 5 workflows x 3 sizes = 15 .teseq files (+ 1 companion Rust test for DECCOLM)
- [x] 2 query-response workflows validate full handshake sequences
- [x] 4 real-world pattern workflows exercise common terminal usage (including charset switching)
- [x] 4 OSC scenarios validate title, icon name, clipboard, color query
- [x] 6 edge case scenarios test boundary conditions, cross-cutting concerns, and chunked-feed resilience
- [x] 19 base .teseq scenarios + 12 multi-size .teseq variants + 3 pure-Rust tests = 34 total test functions pass
  - Base .teseq: 5 mode combo + 2 query + 4 real-world + 4 OSC + 4 edge = 19
  - Multi-size: 10 mode combo variants + 2 status_bar variants = 12
  - Pure-Rust: 2 chunked-feed + 1 DECCOLM lifecycle = 3
- [x] Satisfies mission criteria for multi-sequence workflow coverage, OSC coverage, and ESC workflow coverage

**TDD ordering:** Write each `.teseq` + `.toml` scenario first, run the test to confirm it fails (or produces a new insta snapshot), then implement any production fixes (DECSC conformance gaps) and accept the snapshot. This ensures tests are validated as meaningful before being marked green.

**Codebase rules reminder:**
- All `.rs` source files (excluding `tests.rs`) must stay under 500 lines. If `workflows.rs` exceeds ~400 lines, split into `workflows/mod.rs` + submodules.
- Test organization: integration test family modules live as siblings in `oriterm_core/tests/teseq/` with a `run_scenario` helper pattern (see `sgr/mod.rs`, `mode_interactions.rs` for reference).
- All tests must gracefully skip when `reseq` is unavailable. Pure-Rust chunked tests bypass this gate.
- `timeout 150` on all test commands per CLAUDE.md mandatory timeout rule.
- Run tests in both debug and release profiles: `timeout 150 cargo test -p oriterm_core --test teseq` (debug) and `timeout 150 cargo test -p oriterm_core --test teseq --release` (release). Optimizer-sensitive bugs (e.g., in VTE parser state machine across chunk boundaries) can hide in debug-only runs.

**Context:** Existing test surfaces cover isolated sequences (handler tests) and black-box vttest conformance (vttest tests). The gap is *authored multi-sequence interactions* — scenarios where you deliberately construct a sequence of operations and verify the cumulative effect. This is where real bugs hide: mode A works, mode B works, but A→B→A produces unexpected state.

**Reference implementations:**
- **Alacritty** `tests/ref/`: Real-world recordings (`tmux_git_log`, `vim_simple_edit`, `zsh_tab_completion`) capture actual terminal usage
- **ori_term** vttest menu navigation: Multi-step sequences with assertions between steps
- **Ghostty** fuzz corpus: Evolved byte sequences that found parser bugs

**Depends on:** All scenario sections (01-05) — workflows combine patterns from each.

---

## 06.1 Mode Combination Workflows

**File(s):** `oriterm_core/tests/teseq/scenarios/workflows/mode_*.teseq`, `oriterm_core/tests/teseq/workflows.rs`

**Directory setup:** Create `oriterm_core/tests/teseq/scenarios/workflows/` and `oriterm_core/tests/teseq/scenarios/osc/` directories. Create `oriterm_core/tests/teseq/workflows.rs` family module with `run_scenario` helper (path = `scenarios/workflows/{name}.teseq`, prefix = `workflows_{name}`). Register `mod workflows;` in `main.rs`.

**Implementation phases** (06.1 is the largest subsection — sequence work in this order):

1. **Scaffolding** — directory setup, `workflows.rs` module with `run_scenario`, `main.rs` registration.
2. **Mode combo base scenarios** — `mode_scroll_origin_fill`, `mode_alt_with_modes`, `mode_deccolm_full_cycle` (these exercise existing, known-working paths).
3. **DECSC scenarios** — `mode_decsc_attrs`, `mode_decsc_origin_flag` (these probe conformance gaps and may require production fixes — see DECSC fix items below).
4. **Multi-size variants** — create 97x33 and 120x40 variants for all 5 base scenarios.

**DECSC production fix items** (own the fix, do not leave as expected failures):
- [x] If `mode_decsc_attrs` reveals charset is NOT saved/restored: extend `save_cursor_position()` in `oriterm_core/src/term/handler/mod.rs` to also clone `self.charset` into a new `saved_charset: Option<CharsetState>` field on `Term`, and restore it in `restore_cursor_position()`. The `Cursor` struct (`oriterm_core/src/grid/cursor/mod.rs`) already saves SGR attributes via the template cell. Charset state lives on `Term`, not `Cursor` — so `Term` must save it separately. File path: `oriterm_core/src/term/mod.rs` (add `saved_charset` field), `oriterm_core/src/term/handler/mod.rs` (save/restore logic).
- [x] If `mode_decsc_origin_flag` reveals origin mode is NOT saved/restored: extend `save_cursor_position()` to also save `self.mode.contains(TermMode::ORIGIN)` into a new `saved_origin_mode: Option<bool>` field on `Term`, and in `restore_cursor_position()` set/clear `TermMode::ORIGIN` accordingly. File path: same as above.
- [x] After any DECSC production fix, run `timeout 150 ./test-all.sh` to verify no regressions. Add a targeted unit test in `oriterm_core/src/term/handler/tests.rs` for each fix.

- [x] **`mode_scroll_origin_fill.teseq`** — Complete scroll region + origin mode workflow:
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
  Validates: 16-line scroll region overflows, origin mode cursor stays within region, disabling origin mode returns to absolute positioning. Multi-dimensional assertions: (1) grid snapshot shows scrolled content within rows 4-19 and "After origin off" at row 0; (2) cursor position at (0, 16) after "After origin off" text; (3) `assert_mode_not_contains(TermMode::ORIGIN)` confirms origin mode is off; (4) scrollback is empty (content overflows within the scroll region, not the full grid).

- [x] **`mode_deccolm_full_cycle.teseq`** — Complete DECCOLM lifecycle:
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
  Validates: 80→132 transition clears display, resets scroll margins, and homes cursor; origin mode works at 132 columns with scroll region; 132→80 transition again clears display, resets margins, and homes cursor. **Implementation note:** The `.teseq` file captures the full sequence and validates the final state. A companion pure-Rust test (`deccolm_lifecycle_intermediate_assertions` in `workflows.rs`) feeds the same sequence in phases (pre-DECCOLM content, then DECCOLM set, then 132-col content, etc.) and asserts cursor=(0,0) and empty grid after each DECCOLM transition. This is necessary because `TeseqHarness::run()` feeds all bytes at once with no intermediate checkpoints.

- [x] **`mode_alt_with_modes.teseq`** — Alt screen with modes active:
  ```
  : Esc [ 5 ; 20 r
  : Esc [ ? 6 h
  : Esc [ 3 ; 10 H
  |Primary with modes|.
  : Esc [ ? 1049 h
  |Alt screen|.
  : Esc [ ? 1049 l
  ```
  Validates: Alt screen preserves scroll region and origin mode settings on return. Multi-dimensional assertions: (1) grid snapshot shows "Primary with modes" text restored at the correct position after returning from alt screen; (2) `assert_mode_contains(TermMode::ORIGIN)` confirms origin mode survived the alt screen roundtrip; (3) cursor position is restored to the primary screen position (not the alt screen position); (4) verify scroll region is still active by checking mode flags.

- [x] **`mode_decsc_attrs.teseq`** — DECSC saves SGR attributes and active charset (not just position):
  ```
  : Esc [ 1 ; 31 m
  : Esc ( 0
  : Esc [ 5 ; 10 H
  : Esc 7
  : Esc [ 0 m
  : Esc ( B
  : Esc [ 1 ; 1 H
  |plain text|
  : Esc 8
  |q after restore|
  ```
  Validates: DECSC saves cursor position (4, 9), SGR attributes (bold + red foreground), and active G0 charset (DEC Special Graphics). After DECRC, the cursor returns to (4, 9), bold + red fg are restored, and 'q' renders as line-drawing horizontal line (DEC Special Graphics). "after restore" is rendered in bold + red. The plain text between save and restore uses default attrs + ASCII charset — proving the restore actually restores the saved state, not just the current state. Requires cell flag inspection (bold) and color inspection (red fg) at the restored text position.
  **Implementation note — potential conformance gap:** ori_term's `save_cursor_position()` calls `grid.save_cursor()` which clones the `Cursor` struct (position + template cell with SGR flags/colors). However, the active charset state lives in `Term.charset` (a `CharsetState` struct), NOT in the `Cursor`. Per the DEC VT220 spec, DECSC should save: cursor position, character attributes (SGR), character set state (G0-G3 designations + active slot), origin mode flag, and selective erase attribute. If this scenario reveals that charset is NOT saved/restored, that is a genuine conformance bug to fix (extend `save_cursor_position()` to also save/restore `CharsetState`). SGR attributes ARE saved (confirmed by existing test `esc7_esc8_preserves_sgr_attributes`). File a bug if charset save/restore fails.
  <!-- deferred from Section 02.5 scope note: "additional saved-state dimensions require
       workflow scenarios (Section 06) to validate" -->

- [x] **`mode_decsc_origin_flag.teseq`** — DECSC saves origin mode flag:
  ```
  : Esc [ 5 ; 20 r
  : Esc [ ? 6 h
  : Esc [ 1 ; 1 H
  : Esc 7
  : Esc [ ? 6 l
  : Esc [ 1 ; 1 H
  |absolute|
  : Esc 8
  |relative|
  ```
  Validates: DECSC saves the origin mode flag. After DECRC, origin mode is restored (re-enabled), so the cursor returns to the top of the scroll region (row 4, absolute) rather than row 0. "relative" appears within the scroll region. "absolute" appears at row 0, col 0 (written while origin mode was off).
  **Implementation note — potential conformance gap:** ori_term's `save_cursor_position()` only saves the `Cursor` struct via `grid.save_cursor()`. Origin mode is a `TermMode` flag on `Term`, not part of `Cursor`. Per VT220 spec, DECSC saves the origin mode flag. If this scenario reveals origin mode is NOT restored, that is a conformance bug to fix (extend save/restore to include origin mode flag). File a bug if origin mode save/restore fails.

- [x] Multi-size variants: all 5 mode workflows run at 80x24, 97x33, and 120x40. Each size gets a separate `.teseq` + `.toml` pair with coordinates adjusted for the terminal dimensions. This is the established pattern — see `origin_scroll_basic_97x33.teseq`/`.toml` in `scenarios/csi/modes/` for reference. Specific adjustments per scenario:
  - `mode_scroll_origin_fill`: adjust scroll region (e.g., `5;28r` for 97x33, `5;35r` for 120x40) and line count.
  - `mode_deccolm_full_cycle`: DECCOLM always resizes to 132 columns regardless of base size; adjust scroll region for taller terminals.
  - `mode_alt_with_modes`: adjust scroll region row numbers and CUP coordinates.
  - `mode_decsc_attrs`: adjust CUP coordinates for larger grids.
  - `mode_decsc_origin_flag`: adjust scroll region for taller terminals.
  That is 5 base x 2 extra sizes = 10 additional `.teseq`/`.toml` pairs, plus the 5 base = 15 total .teseq files for mode workflows.

---

## 06.2 Query-Response Workflows

**File(s):** `oriterm_core/tests/teseq/scenarios/workflows/query_*.teseq`

Multi-step query/response sequences that simulate real terminal handshakes.

- [x] **`query_da_handshake.teseq`** — Full DA negotiation:
  ```
  : Esc [ c
  : Esc [ > c
  : Esc [ = c
  ```
  Validates: All three DA responses emitted in order via `assert_pty_writes()` — raw PtyWrite bytes are the canonical oracle. Expected responses (from `oriterm_core/src/term/handler/status.rs`):
  - DA1: `"\x1b[?64;6;4c"` (VT420-class, ANSI color + sixel)
  - DA2: `"\x1b[>0;{version};1c"` where `{version}` = `crate_version_number()` (dynamic — reuse or copy `compute_da2_version()` from `csi_reports.rs:32-41` which replicates the same algorithm)
  - DA3: `"\x1bP!|00000000\x1b\\"` (unit ID, 8 zero digits)
  Optional: pipe responses through `analyze_response()` for human-readable debug output in test failure messages, but never use teseq output as assertion target.

- [x] **`query_cursor_tracking.teseq`** — DSR after each cursor movement:
  ```
  : Esc [ 5 ; 10 H
  : Esc [ 6 n
  : Esc [ 3 A
  : Esc [ 6 n
  : Esc [ 20 C
  : Esc [ 6 n
  ```
  Validates: Each DSR response encodes the correct cursor position via `assert_pty_writes()`. Three PtyWrite events with progressively updated 1-based coordinates: `\x1b[5;10R`, `\x1b[2;10R`, `\x1b[2;30R`. Raw bytes are the oracle; no teseq analysis in assertions.

- [x] **TPR checkpoint** — `/tpr-review` covering 06.1–06.2 implementation work (covered by full-section TPR)

---

## 06.3 Real-World Pattern Workflows

**File(s):** `oriterm_core/tests/teseq/scenarios/workflows/real_*.teseq`

Scenarios that mimic common terminal application patterns.

- [x] **`real_shell_prompt.teseq`** — Typical shell prompt escape sequence pattern:
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

- [x] **`real_clear_and_redraw.teseq`** — Application clears screen and redraws:
  ```
  |Old content line 1|.
  |Old content line 2|.
  : Esc [ 2 J
  : Esc [ 1 ; 1 H
  |New content line 1|.
  |New content line 2|.
  ```
  Validates: ED 2 clears, CUP homes, new content replaces old.

- [x] **`real_charset_switching.teseq`** — G0/G1 charset designation and locking shift in realistic sequence:
  ```
  : Esc ( 0
  |lqqqqk|
  : Esc ( B
  . CR/^M LF/^J
  : Esc ) 0
  . SO/^N
  |x|
  : Esc ( B
  |Text |
  . SI/^O
  |x|
  . CR/^M LF/^J
  : Esc ( 0
  |mqqqqj|
  : Esc ( B
  ```
  Validates: Draws a simple box border using DEC Special Graphics for lines and corners, with "Text" in ASCII inside. Tests G0 designation (`ESC ( 0` / `ESC ( B`), G1 designation (`ESC ) 0`), and locking shifts (SO shifts to G1, SI shifts back to G0). Grid snapshot shows line-drawing characters for box border with ASCII text content. This is a realistic pattern used by TUI applications (ncurses, dialog, etc.). **Semantic pin:** This test ONLY passes if G0/G1 designation, SO/SI locking shifts, and charset restoration all work correctly in combination. A bug in any one (e.g., SO not switching to G1, or G1 not designated as DEC Special Graphics) would produce ASCII instead of line-drawing characters in the grid snapshot, failing the golden comparison.

- [x] **`real_status_bar.teseq`** — Application draws a status bar at bottom:
  ```
  : Esc [ 24 ; 1 H
  : Esc [ 7 m
  | Status: OK                                                                     |
  : Esc [ 0 m
  : Esc [ 1 ; 1 H
  |Main content area|
  ```
  Validates: Cursor positioning to last row, inverse attribute for status bar, return to content area. **Multi-size note:** Row 24 is hard-coded for 80x24. The 97x33 variant must use `CUP 33;1`, and the 120x40 variant must use `CUP 40;1`. Each size gets its own `.teseq` + `.toml` pair with adjusted coordinates and padding width, following the established pattern (see `origin_scroll_basic_97x33.teseq` for reference).

---

## 06.4 OSC Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/osc/*.teseq`, `oriterm_core/tests/teseq/osc.rs`

Dedicated OSC scenarios covering title (OSC 0/2), icon name (OSC 1), clipboard (OSC 52), and color queries (OSC 4/10/11). OSC 7 (CWD) is tested at the mux layer, not here (see note below).

- [x] **`osc_title.teseq`** — Set window title via OSC 0 and OSC 2:
  ```
  : Esc ]
  |0;My Terminal Title|
  : Esc \
  : Esc ]
  |2;Window Title Only|
  : Esc \
  ```
  Note: OSC text content MUST be in `|...|` delimiters (teseq text lines), not on `: Esc` control lines. The `: Esc` lines strip spaces between tokens.
  Assert: OSC 0 emits BOTH `RecordedEvent::Title("My Terminal Title")` AND `RecordedEvent::IconName("My Terminal Title")` (per VTE dispatch at `crates/vte/src/ansi/dispatch/osc.rs:53-55`: OSC 0 calls `set_title` then `set_icon_name`). OSC 2 emits only `RecordedEvent::Title("Window Title Only")`. The event stream will also contain `Wakeup` events; assertions should filter for Title/IconName variants or use insta snapshot (which captures the full event list including Wakeup).

- [x] **`osc_icon_name.teseq`** — Set icon name via OSC 1:
  ```
  : Esc ]
  |1;My Icon Name|
  : Esc \
  ```
  Assert: `RecordedEvent::IconName("My Icon Name")`.

  **OSC 7 (CWD) is NOT tested here.** OSC 7 is handled by `RawInterceptor` in `oriterm_mux`, not by `Term<T>`. The VTE trait method `set_working_directory` is a default no-op on `Term<T>` — the teseq harness feeds bytes only through `vte::ansi::Processor`, so `Event::Cwd` will never be emitted. CWD is already tested at the mux layer (`oriterm_mux/src/shell_integration/tests.rs::interceptor_osc7_sets_cwd`). Implementing `set_working_directory` on `Term<T>` solely for this test would be a workaround that duplicates the mux's CWD responsibility, violating the crate boundary contract.

- [x] **`osc_clipboard.teseq`** — Clipboard store via OSC 52:
  ```
  : Esc ]
  |52;c;SGVsbG8=|
  : Esc \
  ```
  Assert: `RecordedEvent::ClipboardStore(ClipboardType::Clipboard, "Hello")` (base64-decoded by `osc_clipboard_store` before event emission).

- [x] **`osc_color_query.teseq`** — Query foreground/background/palette colors via OSC 4/10/11:
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

- [x] Create `oriterm_core/tests/teseq/osc.rs` family module with `run_scenario` helper (path = `scenarios/osc/{name}.teseq`, prefix = `osc_{name}`). Pattern: copy the `run_scenario` from `sgr/mod.rs` but change the path to `scenarios/osc`.
- [x] Register `mod osc;` in `oriterm_core/tests/teseq/main.rs` under a `// Family modules (Section 06).` comment.

---

## 06.5 Edge Case Scenarios

**File(s):** `oriterm_core/tests/teseq/scenarios/workflows/edge_*.teseq`

Boundary conditions and unusual sequences.

- [x] **`edge_rapid_mode_toggle.teseq`** — Rapidly toggle origin mode:
  ```
  : Esc [ ? 6 h
  : Esc [ ? 6 l
  : Esc [ ? 6 h
  : Esc [ ? 6 l
  : Esc [ 5 ; 10 H
  |After toggles|
  ```
  Validates: Rapid mode toggling doesn't corrupt state. Cursor at correct position.

- [x] **`edge_zero_params.teseq`** — CSI with zero/missing parameters:
  ```
  : Esc [ 0 ; 0 H
  |At origin via zeros|.
  : Esc [ H
  |At origin via omit|.
  : Esc [ 0 A
  |CUU zero|
  ```
  Validates: Zero and omitted params treated as 1 (per ECMA-48).

- [x] **`edge_large_params.teseq`** — CSI with very large parameters:
  ```
  : Esc [ 99999 ; 99999 H
  |Clamped|
  : Esc [ 99999 A
  |Top|
  ```
  Validates: Large params clamped to grid boundaries without panic.

- [x] **`edge_chunked_osc`** (pure Rust, no `.teseq` file) — Adversarial chunked feed of split OSC sequence:
  The standard `TeseqHarness::run()` feeds all bytes in one `Processor::advance()` call, which doesn't exercise the VTE parser's state machine across chunk boundaries. This test is a pure Rust test function (no `.teseq` file) that manually constructs `Term<RecordedListener>` + `Processor` and splits an OSC title sequence across two `advance()` calls to verify correct reassembly. **No `reseq_available()` guard needed** — constructs bytes directly.
  ```rust
  // In workflows.rs — direct Processor usage, no TeseqHarness:
  use oriterm_core::{Term, Theme};
  use super::harness::events::RecordedListener;
  let listener = RecordedListener::new();
  let mut term = Term::new(24, 80, 0, Theme::default(), listener.clone());
  let mut proc = vte::ansi::Processor::new();
  let chunk1 = b"\x1b]0;MyT";
  let chunk2 = b"itle\x07";
  proc.advance(&mut term, chunk1);
  proc.advance(&mut term, chunk2);
  let events = listener.events();
  assert!(events.iter().any(|e| matches!(e, RecordedEvent::Title(t) if t == "MyTitle")));
  ```
  Validates: VTE parser correctly reassembles OSC payload split across PTY read boundaries.

- [x] **`edge_chunked_csi`** (pure Rust, no `.teseq` file) — Adversarial chunked feed of split CSI sequence:
  Same pattern as above but splitting a CSI sequence (e.g., `\x1b[5;10H` split as `\x1b[5;` and `10H`). **No `reseq_available()` guard needed** — constructs bytes directly.
  ```rust
  // chunk1: ESC [ 5 ;    chunk2: 1 0 H
  let chunk1 = b"\x1b[5;";
  let chunk2 = b"10H";
  proc.advance(&mut term, chunk1);
  proc.advance(&mut term, chunk2);
  // Assert cursor at col=9 (0-based), line=4 (0-based) — CUP uses 1-based params
  let content = term.renderable_content();
  assert_eq!((content.cursor.column.0, content.cursor.line), (9, 4));
  ```
  Validates: CSI parameters are not lost or corrupted across chunk boundaries.

- [x] **`edge_erase_with_attrs.teseq`** — Erase inherits cursor template background:
  ```
  |AAAAAAAAAA|
  : Esc [ 1 ; 5 H
  : Esc [ 42 m
  : Esc [ 0 K
  : Esc [ 0 m
  ```
  Validates: EL 0 (erase right) at col 4 with green background active — erased cells (cols 4-79) should have green bg from the cursor template. Cells before the cursor (cols 0-3) retain original (default) bg. Use `cell_bg_at(&outcome, 0, 4)` (from `harness/assertions.rs`) to verify green bg (expected: Rgb from palette index 2, which is green). Use `cell_bg_at(&outcome, 0, 3)` to verify default bg is preserved. This is the cross-cutting erase+SGR test that Section 02 basic erase scenarios defer. The palette's green value can be obtained from `Palette::default()` for comparison.

---

## 06.R Third Party Review Findings

- [x] `[TPR-06-001][high]` [oriterm_core/src/term/handler/esc.rs](/home/eric/projects/ori_term/oriterm_core/src/term/handler/esc.rs#L19) / [oriterm_core/src/term/handler/mod.rs](/home/eric/projects/ori_term/oriterm_core/src/term/handler/mod.rs#L300) / [oriterm_core/src/grid/mod.rs](/home/eric/projects/ori_term/oriterm_core/src/grid/mod.rs#L197) / [oriterm_core/src/term/handler/tests.rs](/home/eric/projects/ori_term/oriterm_core/src/term/handler/tests.rs#L2730) — `RIS` clears the grid’s saved cursor but leaves the new DECSC sidecar state live, so a later `DECRC` can resurrect stale origin-mode and charset state after a full reset.
  Resolved: Fixed on 2026-04-06. Cleared `saved_charset` and `saved_origin_mode` in `esc_reset_state()`.

- [x] `[TPR-06-002][medium]` [plans/teseq-conformance/section-06-workflows.md](/home/eric/projects/ori_term/plans/teseq-conformance/section-06-workflows.md#L64) / [plans/teseq-conformance/section-06-workflows.md](/home/eric/projects/ori_term/plans/teseq-conformance/section-06-workflows.md#L460) / [plans/teseq-conformance/section-06-workflows.md](/home/eric/projects/ori_term/plans/teseq-conformance/section-06-workflows.md#L462) / [plans/teseq-conformance/section-06-workflows.md](/home/eric/projects/ori_term/plans/teseq-conformance/section-06-workflows.md#L477) / [oriterm_core/tests/teseq/workflows.rs](/home/eric/projects/ori_term/oriterm_core/tests/teseq/workflows.rs#L1) / [oriterm_core/tests/teseq/osc.rs](/home/eric/projects/ori_term/oriterm_core/tests/teseq/osc.rs#L1) — Section 06 is marked through 06.5 complete, but the required verification surface is still short of the planned 34 tests.
  Resolved: Fixed on 2026-04-06. Added `real_status_bar_97x33`, `real_status_bar_120x40`, and `deccolm_lifecycle_intermediate_assertions` tests. Total now 34.

- [x] `[TPR-06-003][low]` [oriterm_core/tests/teseq/workflows.rs](/home/eric/projects/ori_term/oriterm_core/tests/teseq/workflows.rs#L1) / [plans/teseq-conformance/section-06-workflows.md](/home/eric/projects/ori_term/plans/teseq-conformance/section-06-workflows.md#L73) / [plans/teseq-conformance/section-06-workflows.md](/home/eric/projects/ori_term/plans/teseq-conformance/section-06-workflows.md#L464) — `workflows.rs` is now 509 lines, past the repo’s hard 500-line limit and past this section’s own split threshold.
  Resolved: Fixed on 2026-04-06. Split workflows.rs into workflows/mod.rs + 4 submodules (mode.rs, query.rs, real_world.rs, edge.rs). All under 500 lines.

- [x] `[TPR-06-004][medium]` [oriterm_core/src/term/alt_screen.rs](/home/eric/projects/ori_term/oriterm_core/src/term/alt_screen.rs#L79) / [plans/teseq-conformance/section-06-workflows.md](/home/eric/projects/ori_term/plans/teseq-conformance/section-06-workflows.md#L106) — The new per-screen DECSC sidecar swap has no regression test, even though Section 06 requires targeted handler coverage after each DECSC fix.
  Resolved: Validated on 2026-04-06. Test `decsc_sidecar_isolation_across_alt_screen` already exists at `oriterm_core/tests/teseq/workflows/mode.rs:352` (added in commit 87801e87). It saves DEC Special Graphics + origin mode on primary, switches to alt, does different DECSC with ASCII + no origin, switches back, and verifies DECRC restores the primary's charset and origin mode.

---

## 06.N Completion Checklist

- [x] Mode combination workflows: scroll+origin, DECCOLM lifecycle, alt screen+modes, DECSC attrs, DECSC origin flag (5 base scenarios)
- [x] Query-response workflows: DA handshake, cursor tracking DSR (2 scenarios)
- [x] Real-world pattern workflows: shell prompt, clear+redraw, charset switching, status bar (4 scenarios)
- [x] OSC scenarios: title (0/2), icon name (1), clipboard (52), color query (4/10/11) (4 scenarios)
- [x] Edge case scenarios: rapid toggles, zero params, large params, erase-with-attrs, chunked-feed OSC, chunked-feed CSI (6 scenarios, 2 are pure-Rust)
- [x] Mode combination workflows run at 80x24, 97x33, and 120x40 (separate .teseq/.toml per size — 15 .teseq files for 5 base x 3 sizes)
- [x] `real_status_bar` multi-size variants (97x33, 120x40) with adjusted row numbers and padding width
- [x] DECSC production fixes applied if conformance gaps found (charset save/restore, origin mode save/restore) — see 06.1 fix items
- [x] DECCOLM companion pure-Rust test (`deccolm_lifecycle_intermediate_assertions`) validates intermediate state
- [x] 19 base .teseq + 12 multi-size .teseq + 3 pure-Rust = 34 total test functions pass
- [x] `workflows.rs` stays under 500 lines; if it grows past ~400 lines, split into submodules (e.g., `workflows/mod.rs` + `workflows/mode.rs` + `workflows/edge.rs`)
- [x] `./build-all.sh` green, `./clippy-all.sh` green
- [x] `timeout 150 ./test-all.sh` green — no regressions
- [x] `timeout 150 cargo test -p oriterm_core --test teseq --release` green — verify no optimizer-sensitive bugs in chunked-feed or workflow tests
- [x] Plan annotation cleanup
- [x] All TPR checkpoint findings resolved
- [x] **Plan sync** — update plan metadata:
  - [x] This section's frontmatter `status` → `complete`
  - [x] `00-overview.md` Quick Reference table updated
  - [x] `index.md` section status updated
- [x] `/tpr-review` passed (final, full-section)
- [x] `/impl-hygiene-review last commit` passed

**Exit Criteria:** `timeout 150 cargo test -p oriterm_core --test teseq -- workflows` and `timeout 150 cargo test -p oriterm_core --test teseq -- osc` pass with 34 total test functions (19 base .teseq + 12 multi-size .teseq + 3 pure-Rust) in both debug and release profiles. Multi-sequence interactions, query-response handshakes, real-world patterns (including charset switching), DECSC attribute save/restore (with production fixes if needed), OSC events, chunked-feed resilience, and edge cases all validated. Mode combination workflows run at 3 terminal sizes (separate .teseq/.toml per size). Zero regressions.
