---
section: "04"
title: "append_html_run shared iterator (F-16)"
status: not-started
reviewed: false
goal: "Extract the shared cell-iteration skeleton from `append_html_cells` and `append_cells_dual` into one canonical `append_html_run` helper parameterized by an `FnMut(char)` text callback."
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "04.1"
    title: "Extract append_html_run helper"
    status: not-started
  - id: "04.2"
    title: "Migrate append_html_cells (no-op text callback)"
    status: not-started
  - id: "04.3"
    title: "Migrate append_cells_dual (text-buffer callback)"
    status: not-started
  - id: "04.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "04.N"
    title: "Build & Verify"
    status: not-started
---

# Section 04: append_html_run shared iterator

**Goal:** `append_html_cells` and `append_cells_dual` share an identical
control-flow skeleton (col_start..=last → spacer skip → KITTY_PLACEHOLDER
skip → `CellStyle::from_cell` → span coalescing → `push_html_escaped` →
zerowidth append). The dual variant adds a parallel text-buffer write at
the top; the html-side logic is otherwise identical. This section
collapses the duplication into one shared helper parameterized by an
`FnMut(char)` text callback.

**Production code path:** `oriterm_core::selection::extract_html` (HTML-only)
and `oriterm_core::selection::extract_html_with_text` (HTML + plain-text
dual extract). Both ultimately call into the per-row append loop in
`oriterm_core/src/selection/html/mod.rs`.

**Observable change:** None at runtime — pure refactor. HTML output is
byte-identical; plain-text output is byte-identical.

**Context:** The Phase 5 hygiene report flagged this as Major DRIFT F-16
(duplicated-cell-iteration-in-html). Per `.claude/rules/impl-hygiene.md`
§Algorithmic DRY, a 2-instance shared skeleton longer than 5 lines is the
extraction trigger. The two functions span lines 192-250 and 257-323 of
`html/mod.rs` (~60 lines of skeleton each). The only meaningful
difference is a single text-buffer push that lives only in
`append_cells_dual`.

This section is independent of Sections 01-03. It can run in any order.
It depends on nothing.

**Reference implementations:**
- **WezTerm** `wezterm-term/src/screen.rs`: copy-to-clipboard and
  copy-as-html share a single `for_each_cell_in_range` iterator that
  takes an `FnMut(&Cell, usize, usize)` callback. Same pattern as
  `append_html_run`.
- **Alacritty** `alacritty_terminal/src/term/cell.rs`: selection extraction
  uses `Selection::range` to hand callers an iterator; the html and
  plain-text consumers are separate but ride the same iterator.

**Depends on:** None.

---

## 04.1 Extract append_html_run helper

**File(s):** `oriterm_core/src/selection/html/mod.rs`.

**Context:** The new helper takes an `FnMut(char)` text callback. The
HTML-only path passes a no-op closure. The dual path passes a closure
that pushes to its text buffer.

**Fix approach — 2 options:**

**(a) Single helper with FnMut callback** (recommended — most direct):

```rust
pub(super) fn append_html_run(
    html_buf: &mut String,
    row: &Row,
    col_start: usize,
    col_end: usize,
    ctx: &HtmlCtx<'_>,
    mut text_callback: impl FnMut(char),
) {
    // 1. Iterate col_start..=col_end on row.
    // 2. Skip spacers via cell.flags.is_spacer() (Section 02 dependency
    //    — if Section 02 has not landed yet, use the inline predicate
    //    and migrate when 02 lands).
    // 3. Skip KITTY_PLACEHOLDER cells.
    // 4. Compute CellStyle::from_cell, coalesce spans.
    // 5. push_html_escaped to html_buf.
    // 6. text_callback(ch) for the canonical char.
    // 7. Append zerowidth chars (each via text_callback as well).
}
```

**Why this is best:** One helper, one signature, two trivial call sites.
The closure overhead is zero for the no-op case (compiles away).

**Trade-off:** The closure abstraction is slightly less obvious than
two separate functions. Worth it for SSOT.

**(b) `extract_html` calls `extract_html_with_text` and discards text**
(alternative — simpler but allocates):

```rust
pub fn extract_html(...) -> String {
    let (html, _text) = extract_html_with_text(...);
    html
}
```

**Downside:** Always allocates the text buffer even when callers don't
need it. Test-suite wall-clock impact only, but it violates allocation
discipline from `oriterm_core` §Performance Invariants.

**Recommended path:** Option (a). The closure approach is the canonical
GoF visitor pattern and is what both reference emulators use.

- [ ] Add `pub(super) fn append_html_run<F: FnMut(char)>(...)` in
      `oriterm_core/src/selection/html/mod.rs` containing the canonical
      cell-iteration body.
- [ ] Define the text callback as `impl FnMut(char)` (zero-cost when no-op).
- [ ] Add direct unit tests in `oriterm_core/src/selection/html/tests.rs`:
  - [ ] `append_html_run_emits_span_for_styled_cell`
  - [ ] `append_html_run_skips_spacer_cell` (semantic)
  - [ ] `append_html_run_skips_kitty_placeholder` (semantic)
  - [ ] `append_html_run_coalesces_runs_of_identical_style`
  - [ ] `append_html_run_invokes_text_callback_for_every_emitted_char`
        (semantic — confirms text-buffer parity)
  - [ ] `append_html_run_invokes_text_callback_for_zerowidth` (semantic)
  - [ ] `append_html_run_no_op_callback_does_not_affect_html_output`
        (**negative** pin — html output is identical regardless of
        callback content)

---

## 04.2 Migrate append_html_cells (no-op text callback)

**File(s):** `oriterm_core/src/selection/html/mod.rs:192-250`.

- [ ] Replace the body of `append_html_cells` with a single
      `append_html_run(html_buf, row, col_start, col_end, ctx, |_ch| {})`
      call.
- [ ] Confirm test count is unchanged in
      `oriterm_core/src/selection/html/tests.rs` and all html-only tests
      remain green. Particular attention to: span-coalescing tests,
      spacer-skip tests, kitty-placeholder-skip tests.

---

## 04.3 Migrate append_cells_dual (text-buffer callback)

**File(s):** `oriterm_core/src/selection/html/mod.rs:257-323`.

- [ ] Replace the body of `append_cells_dual` with a single
      `append_html_run(html_buf, row, col_start, col_end, ctx, |ch| text_buf.push(ch))`
      call.
- [ ] Confirm dual-extract tests remain green, including byte-for-byte
      equivalence with the prior plain-text output.
- [ ] Repo grep `rg -n "fn append_html_cells\|fn append_cells_dual" oriterm_core/src/selection/`
      should show both functions reduced to one-liners delegating to
      `append_html_run`.

---

## 04.R Third Party Review Findings

Track findings from `/tpr-review` runs against Section 04 here. Leave the
block in place even when empty so tooling has a stable anchor.

- None.

Format and rules as documented in `plans/_template/plan.md`.

---

## 04.N Build & Verify

### TDD Matrix

| Test (in `selection/html/tests.rs`) | Pin type | Lock-in target |
|---|---|---|
| `append_html_run_emits_span_for_styled_cell` | semantic | basic emit |
| `append_html_run_skips_spacer_cell` | semantic | spacer skip |
| `append_html_run_skips_kitty_placeholder` | semantic | placeholder skip |
| `append_html_run_coalesces_runs_of_identical_style` | semantic | span coalescing |
| `append_html_run_invokes_text_callback_for_every_emitted_char` | semantic | callback parity |
| `append_html_run_invokes_text_callback_for_zerowidth` | semantic | zerowidth parity |
| `append_html_run_no_op_callback_does_not_affect_html_output` | **negative** | html unchanged regardless of callback |
| Existing `extract_html` tests | regression | html-only path |
| Existing `extract_html_with_text` tests | regression | dual path |

### Completion Checklist

- [ ] `./build-all.sh` passes
- [ ] `./clippy-all.sh` passes
- [ ] `./test-all.sh` passes
- [ ] `append_html_run` exists with `FnMut(char)` text callback parameter
- [ ] `append_html_cells` reduced to a one-liner delegating to
      `append_html_run` with a no-op callback
- [ ] `append_cells_dual` reduced to a one-liner delegating to
      `append_html_run` with a text-buffer-push callback
- [ ] Test count in `oriterm_core/src/selection/html/tests.rs` is
      unchanged or higher (new helper-direct tests added; no existing
      tests removed)
- [ ] `/tpr-review` against this section returns clean (or all findings
      `[x]` resolved in 04.R)
- [ ] Repo grep confirms zero remaining duplicate cell-iteration loops
      in `selection/html/mod.rs`

**Exit Criteria:** A change to the html cell-iteration skeleton (e.g.
adding a new flag-driven skip, changing span coalescing) needs to land at
exactly one site (`append_html_run`). Section 04 is complete.
