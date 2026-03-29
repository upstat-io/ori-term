# Section 11 (Search) Verification Results

**Auditor:** verify-roadmap agent
**Date:** 2026-03-29
**Section status in plan:** complete
**Verdict:** PASS — all plan items verified with evidence

---

## Context Loaded

- `/home/eric/projects/ori_term/.claude/worktrees/verify-roadmap/CLAUDE.md` (full read)
- `.claude/rules/code-hygiene.md` (full read)
- `.claude/rules/impl-hygiene.md` (full read)
- `.claude/rules/test-organization.md` (full read)
- `.claude/rules/crate-boundaries.md` (loaded via system reminder)
- `plans/roadmap/section-11-search.md` (full read)

---

## 11.1 Search State

**File:** `oriterm_core/src/search/mod.rs` (202 lines — under 500-line limit)

### Pin-by-pin audit:

| Plan Item | Status | Evidence |
|-----------|--------|----------|
| `MatchType` enum — `None`, `Match`, `FocusedMatch` | VERIFIED | Lines 13-21: `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum MatchType { None, Match, FocusedMatch }` |
| `SearchMatch` struct with `start_row`, `start_col`, `end_row`, `end_col` | VERIFIED | Lines 24-34: All four fields present with correct types (`StableRowIndex` for rows, `usize` for cols). Derives `Debug, Clone`. |
| `SearchState` struct fields: `query`, `matches`, `focused`, `case_sensitive`, `use_regex` | VERIFIED | Lines 37-48: All five fields present with correct types. |
| `SearchState::new()` — empty defaults | VERIFIED | Lines 51-60: `query: String::new(), matches: Vec::new(), focused: 0, case_sensitive: false, use_regex: false` |
| `SearchState::next_match()` — wrapping advance | VERIFIED | Lines 100-104: `(self.focused + 1) % self.matches.len()` |
| `SearchState::prev_match()` — wrapping retreat | VERIFIED | Lines 107-115: Handles `focused == 0` case by wrapping to `len - 1`. |
| `SearchState::update_query()` — re-run search, clamp focused | VERIFIED | Lines 124-137: Clears on empty, calls `find::find_matches()`, clamps focused with `.min(len - 1)`. |
| `SearchState::focused_match()` — return Option | VERIFIED | Line 140-142: `self.matches.get(self.focused)` |
| `SearchState::cell_match_type()` — O(log n) via partition_point | VERIFIED | Lines 147-172: Uses `partition_point` for binary search, checks window of `idx-1..idx+1`, returns `FocusedMatch` for focused match index. |
| `cell_in_match()` — private helper for single/multi-row | VERIFIED | Lines 182-199: Handles `start_row == end_row` (single-row), start row, end row, and middle row cases. |
| Re-export from `lib.rs` | VERIFIED | `oriterm_core/src/lib.rs` line 33: `pub use search::{MatchType, SearchMatch, SearchState};` and line 32: `pub use search::text::extract_row_text;` |

### Tests for 11.1 (in `oriterm_core/src/search/tests.rs`):

- `cell_match_type_binary_search` — verifies O(log n) lookup returns `FocusedMatch` for matched cells, `None` outside.
- `cell_match_type_distinguishes_focused` — verifies focused vs non-focused distinction changes after `next_match()`.
- `cell_match_type_empty_matches` — verifies `None` when no matches.
- `next_match_wraps_around` — 3 matches, cycles 0->1->2->0.
- `prev_match_wraps_around` — wraps from 0->2->1.
- `focused_match_returns_correct_match` — verifies start_col for focused match.
- `update_query_clears_on_empty` — sets query, then clears with empty.
- `update_query_clamps_focused` — focused=1 clamped when matches reduce.
- `toggle_case_sensitive_reruns_search` — verifies match count changes.
- `toggle_regex_reruns_search` — verifies regex toggle activates regex mode.

**Coverage assessment:** All state operations (create, navigate, update, classify) have dedicated tests. Edge cases (empty, wrapping, clamping) are covered.

---

## 11.2 Search Algorithm

**File:** `oriterm_core/src/search/find.rs` (137 lines — under 500-line limit)

### Pin-by-pin audit:

| Plan Item | Status | Evidence |
|-----------|--------|----------|
| `find_matches()` signature — grid, query, case_sensitive, use_regex | VERIFIED | Lines 24-29: Correct signature, returns `Vec<SearchMatch>`. |
| Returns sorted by position | VERIFIED | Lines 37-59: Iterates `abs_row` from 0 to `total_lines`, appending matches row-by-row, inherently sorted. |
| Regex path — `RegexBuilder`, case_insensitive, invalid returns empty | VERIFIED | Lines 118-137: `RegexBuilder::new(pattern).case_insensitive(!case_sensitive).build()` — `else { return }` on error. Skips zero-length matches. |
| Plain text path — case-insensitive lowering, sliding search | VERIFIED | Lines 77-115: Lowercases both haystack and needle when `!case_sensitive`. Advances by char length to stay on char boundary. |
| Iteration scope: scrollback + viewport | VERIFIED | Line 37: `grid.total_lines()` iterates all rows. |
| Byte span to column mapping via `emit_match` + `byte_span_to_cols` | VERIFIED | Lines 63-74: `emit_match()` calls `byte_span_to_cols()` for coordinate translation. |

### Tests for 11.2:

- `plain_text_finds_in_two_rows` — "hello" found at cols 0-4 in both rows.
- `plain_text_case_insensitive` — "Hello HELLO hello" finds 3 matches.
- `plain_text_case_sensitive` — only exact-case match found at col 12.
- `plain_text_empty_query` — returns empty.
- `regex_digits` — `\d+` finds "123" at cols 4-6 and "456" at cols 12-14.
- `regex_invalid_returns_empty` — `[invalid` returns empty, no panic.
- `regex_case_insensitive` — "hello" matches "Hello" in regex mode.

**Coverage assessment:** All algorithm paths (plain, regex, case-sensitive/insensitive, invalid, empty) tested. No multi-row regex test since the plan explicitly defers multi-row regex.

---

## 11.3 Row Text Extraction

**File:** `oriterm_core/src/search/text.rs` (97 lines — under 500-line limit)

### Pin-by-pin audit:

| Plan Item | Status | Evidence |
|-----------|--------|----------|
| `extract_row_text()` — skip spacers, null -> space, push base char, append zerowidth | VERIFIED | Lines 19-44: Checks `WIDE_CHAR_SPACER | LEADING_WIDE_CHAR_SPACER`, replaces `\0` with `' '`, pushes `extra.zerowidth`. |
| Returns `(String, Vec<usize>)` | VERIFIED | Line 19 signature and line 43 return. |
| `byte_span_to_cols()` — byte range to inclusive column range | VERIFIED | Lines 51-67: Uses `char_index_at_byte` and `char_index_containing_byte` helpers. Returns `None` for empty/OOB spans. |
| `char_index_at_byte` — first char at or after offset | VERIFIED | Lines 70-79: Uses `char_indices().position()`. |
| `char_index_containing_byte` — char containing byte offset | VERIFIED | Lines 82-97: Walks `char_indices()` tracking last char at or before clamped offset. |
| `pub(crate)` visibility for shared use | VERIFIED | Line 51: `pub(crate) fn byte_span_to_cols(...)`. `extract_row_text` is `pub` (line 19) — plan says `pub(crate)` but it is `pub` for the `lib.rs` re-export (`pub use search::text::extract_row_text`). This is acceptable since it is re-exported at lib level for URL detection use. |

### Tests for 11.3:

- `extract_ascii_row` — "hello" produces identity col_map [0,1,2,3,4].
- `extract_null_cells_replaced_with_spaces` — null chars become spaces.
- `extract_wide_char_skips_spacers` — wide char spacer skipped, col_map jumps.
- `extract_combining_marks_share_column` — combining mark shares base char's column.
- `byte_span_ascii_identity` — span(1,4) maps to cols (1,3).
- `byte_span_multibyte_utf8` — 3-byte CJK char correctly mapped.
- `byte_span_empty_returns_none` — span(2,2) returns None.

**Coverage assessment:** ASCII, wide char, null, combining marks, multi-byte UTF-8 all tested. Good Unicode edge case coverage.

---

## 11.4 Search UI

**File:** `oriterm/src/app/search_ui.rs` (183 lines — under 500-line limit)

### Pin-by-pin audit:

| Plan Item | Status | Evidence |
|-----------|--------|----------|
| `App::open_search()` — activate search, request redraw | VERIFIED | Lines 13-22: Calls `mux.open_search(pane_id)`, sets `ctx.dirty = true`. |
| `App::close_search()` — close, clear, request redraw | VERIFIED | Lines 25-34: Calls `mux.close_search(pane_id)`, sets `ctx.dirty = true`. |
| `App::handle_search_key()` — Escape, Enter/Shift+Enter, Backspace, Character | VERIFIED | Lines 49-122: All four key types dispatched. Escape -> close. Enter -> next/prev (shift-aware). Backspace -> pop char. Character -> append. All call `scroll_to_search_match` + dirty flag. |
| `App::scroll_to_search_match()` — viewport scrolling to center match | VERIFIED | Lines 125-182: Reads focused match from snapshot, computes absolute row from StableRowIndex, checks if visible, centers viewport if not, calls `mux.scroll_display()`. |
| Search bar rendering as StatusBadge | VERIFIED | `oriterm/src/app/redraw/search_bar.rs` lines 1-71: Renders via `StatusBadge`, shows query + "N of M" format, positioned top-right with margin. |
| Match highlight in GPU pipeline | VERIFIED | `oriterm/src/gpu/prepare/mod.rs` lines 149-157: `cell_match_type()` returns `FocusedMatch` -> bright yellow bg + dark fg, `Match` -> yellow-tinted bg + original fg. Constants: `SEARCH_MATCH_BG(100,100,30)`, `SEARCH_FOCUSED_BG(200,170,40)`, `SEARCH_FOCUSED_FG(0,0,0)`. |
| `Ctrl+Shift+F` keybinding | VERIFIED | `oriterm/src/keybindings/defaults.rs` line 40: `bind(ch("f"), cs, Action::OpenSearch)` where `cs` = Ctrl+Shift. macOS: `bind(ch("f"), cmd, Action::OpenSearch)` at line 124. |
| Search state per-pane (not per-tab) | VERIFIED | `oriterm_mux/src/pane/selection.rs` lines 57-82: Search state (`Option<SearchState>`) lives on `Pane`. Open/close/active methods on `Pane`. |
| Key flow dispatched via action system | VERIFIED | `oriterm/src/app/keyboard_input/action_dispatch.rs` line 107: `Action::OpenSearch` calls `self.open_search()`. `keyboard_input/mod.rs` line 108: `is_search_active()` gates `handle_search_key()` to consume all keys during search. |

### GPU search highlight tests (in `oriterm/src/gpu/prepare/tests.rs`):

- `search_match_highlights_bg` — non-focused match cell gets yellow-tinted bg.
- `search_match_preserves_fg` — non-focused match keeps original fg.
- `search_focused_match_overrides_fg_and_bg` — focused match gets bright yellow bg + dark fg.
- `search_match_skips_block_cursor_cell` — block cursor cell skips search highlighting.
- `search_no_match_uses_default_colors` — no match = default colors.

### Integration tests (in `oriterm_mux/tests/`):

- `contract.rs::contract_search` (runs for both embedded and daemon backends) — opens search, sets query "NEEDLE", verifies matches found, closes and verifies cleared.
- `e2e.rs::test_search_lifecycle` — daemon mode: sends "NEEDLE_HAYSTACK", opens search, sets query, polls for matches, closes.
- `e2e.rs::test_search_navigation` — daemon mode: sends "AAA" x3, opens search, navigates next, verifies focused index changes.

---

## 11.5 Section Completion

### Test results:

```
oriterm_core search tests: 24 passed, 0 failed (0.01s)
oriterm search tests: 6 passed, 0 failed (0.00s)
oriterm_mux contract tests: 2 passed (embedded + daemon) (1.72s)
oriterm_mux e2e tests: 2 passed (0.27s)
Total: 34 tests, all passing.
```

---

## Hygiene Audit

### Code Hygiene

| Rule | Status | Notes |
|------|--------|-------|
| File size < 500 lines | PASS | mod.rs=202, find.rs=137, text.rs=97, search_ui.rs=183, search_bar.rs=71 |
| No `unwrap()` in library code | PASS | Zero `unwrap()` calls in search/{mod,find,text}.rs |
| No `#[allow(clippy)]` without reason | PASS | No allow/expect attributes in search code |
| No platform `#[cfg]` in search logic | PASS | Only `#[cfg(test)]` for test module |
| Module doc (`//!`) on every file | PASS | All 4 search files have module-level doc comments |
| `///` on all pub items | PASS | All pub structs, enums, methods have doc comments |
| No dead/commented-out code | PASS | Clean files |
| Import organization (std / external / internal) | PASS | Correct three-group ordering in all files |
| Sibling `tests.rs` pattern | PASS | `search/mod.rs` ends with `#[cfg(test)] mod tests;`, `search/tests.rs` is sibling |
| No module wrapper in `tests.rs` | PASS | Tests written at top level of file, no `mod tests {}` wrapper |
| **Decorative banners in tests.rs** | **FINDING** | `oriterm_core/src/search/tests.rs` uses `// ── Section ──` decorative banners (9 instances: lines 13, 42, 91, 118, 153, 181, 227, 269, 299). Code-hygiene.md explicitly bans these: "Decorative banners (`// ───`, `// ===`, `// ***`, `// ---`) — Never." Should use plain `// Section name` labels instead. |

### Implementation Hygiene

| Rule | Status | Notes |
|------|--------|-------|
| Crate boundaries respected | PASS | Search logic in `oriterm_core`, UI integration in `oriterm`, mux plumbing in `oriterm_mux`. No circular deps. |
| No allocation in hot paths | PASS | `cell_match_type()` uses binary search, no allocation. |
| No panics on user input | PASS | Invalid regex returns empty vec, empty query handled. |
| No `cfg` in business logic | PASS | Search modules platform-independent. |
| Rendering is pure computation | PASS | `prepare/mod.rs` reads search state, outputs instances. No mutations. |
| One-way data flow | PASS | Search state flows: SearchState -> snapshot -> FrameSearch -> GPU prepare. No callbacks or reverse flow. |

### Test Organization

| Rule | Status | Notes |
|------|--------|-------|
| Sibling tests.rs | PASS | `search/tests.rs` |
| `super::` imports | PASS | Uses `super::find::find_matches`, `super::text::*`, `super::{MatchType, SearchState}` |
| No module wrapper | PASS | Direct top-level tests |
| Test helpers local | PASS | `row_from_str`, `grid_with_rows`, `sri` helpers in tests.rs |

---

## Gap Analysis

### Against section goal: "Plain text and regex search across terminal grid with search UI overlay and match navigation"

1. **Plain text search** — Fully implemented with case-sensitive/insensitive modes, sliding window algorithm. Tested.
2. **Regex search** — Fully implemented via `regex::RegexBuilder`, graceful invalid-regex handling. Tested.
3. **Search across viewport + scrollback** — `find_matches` iterates `grid.total_lines()` (all rows). Tested via contract tests with real PTY output.
4. **Search UI overlay** — `StatusBadge` rendered top-right showing query + "N of M". Keybinding wired (`Ctrl+Shift+F` / `Cmd+F`).
5. **Match navigation** — `next_match()`/`prev_match()` with wrapping. `Enter`/`Shift+Enter` dispatch. Tested including e2e.
6. **Match highlighting** — GPU pipeline classifies cells via `cell_match_type()` (O(log n) binary search), applies distinct colors for focused vs non-focused matches. 5 GPU prepare tests.
7. **Scrollback search** — Viewport auto-scrolls to center focused match when outside view. `scroll_to_search_match()` implemented.
8. **Per-pane search state** — Lives on `Pane` in `oriterm_mux`, plumbed through `MuxBackend` trait (6 methods), wire protocol `PaneSnapshot`, and snapshot serialization.
9. **Wide chars and combining marks** — `extract_row_text` handles `WIDE_CHAR_SPACER`, `LEADING_WIDE_CHAR_SPACER`, and zero-width combining chars. Tests for all three.

### Missing coverage (minor):

- No test for multi-row search matches (plan explicitly defers multi-row regex).
- No test for `scroll_to_search_match()` specifically (requires App context — GUI integration).
- No test for `LEADING_WIDE_CHAR_SPACER` specifically in text extraction (only `WIDE_CHAR_SPACER` tested, but both are checked in the same `intersects()` call).
- `extract_row_text` is `pub` but plan says `pub(crate)` — however it is intentionally re-exported at lib.rs level for URL detection, so this is a plan-vs-code discrepancy, not a bug.

### Findings:

1. **Decorative banners in tests.rs** — `oriterm_core/src/search/tests.rs` uses 9 `// ── ... ──` decorative banner comments (lines 13, 42, 91, 118, 153, 181, 227, 269, 299). Code hygiene rules ban these; should be plain `// Section name` labels.

---

## Summary

| Subsection | Pins | Verified | Tests | Test Results |
|------------|------|----------|-------|-------------|
| 11.1 Search State | 11 | 11 | 10 unit | All pass |
| 11.2 Search Algorithm | 6 | 6 | 7 unit | All pass |
| 11.3 Row Text Extraction | 6 | 6 | 7 unit | All pass |
| 11.4 Search UI | 8 | 8 | 5 GPU + 4 integration | All pass |
| **Total** | **31** | **31** | **34** | **34/34 pass** |

**Verdict: PASS** — All 31 plan items verified against source code. 34 tests across 3 crates all passing. One minor code hygiene finding (decorative banners in test file).
