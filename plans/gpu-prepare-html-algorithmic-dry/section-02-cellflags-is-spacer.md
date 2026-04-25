---
section: "02"
title: "CellFlags::is_spacer predicate (F-15)"
status: not-started
reviewed: false
goal: "Add a single canonical `CellFlags::is_spacer()` predicate and migrate all 5 duplicate call sites across `oriterm` and `oriterm_core` to consume it."
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "02.1"
    title: "Add CellFlags::is_spacer predicate + sibling test"
    status: not-started
  - id: "02.2"
    title: "Migrate 5 call sites"
    status: not-started
  - id: "02.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "02.N"
    title: "Build & Verify"
    status: not-started
---

# Section 02: CellFlags::is_spacer predicate

**Goal:** Replace the 5 duplicated `flags.intersects(CellFlags::WIDE_CHAR_SPACER
| CellFlags::LEADING_WIDE_CHAR_SPACER)` predicates with a single canonical
`CellFlags::is_spacer()` method on the bitflags type. After this section
lands, adding a new spacer variant (e.g. emoji ZWJ continuation) requires
updating exactly one site, not five.

**Production code path:** Every cell-iteration loop that must skip
double-width-spacer cells. Specifically:
- `oriterm/src/gpu/prepare/mod.rs:243-247` (`fill_frame_shaped`)
- `oriterm/src/gpu/prepare/dirty_skip/mod.rs:240-245` (`fill_frame_incremental`)
- `oriterm/src/gpu/prepare/unshaped.rs:85-90` (test-only `fill_frame`)
- `oriterm_core/src/selection/html/mod.rs:208-213` (`append_html_cells`)
- `oriterm_core/src/selection/html/mod.rs:271-275` (`append_cells_dual`)
- `oriterm_core/src/selection/text.rs:89-94` (`append_text_cells`)

**Observable change:** None at runtime — pure refactor. Behavior is
preserved: the same cells are skipped under the same conditions.

**Context:** The Phase 5 hygiene report flagged this as Critical LEAK F-15
(duplicated-spacer-skip-predicate, 5 sites across 2 crates). Per
`.claude/rules/impl-hygiene.md` §"Same algorithm across N files = missing
abstraction", a 5-site duplication is the textbook case for a missing
predicate method. The bitflags crate makes this trivial — `is_spacer` is
one line plus a doc comment.

This section is independent of Section 01 (the per-cell emit extraction).
It can run before, after, or in parallel. If run after Section 01, three
of the five sites have already been collapsed inside `emit_cell` — but
the html and selection sites remain. Either way, all 5+ sites must be
audited.

**Reference implementations:**
- **Alacritty** `alacritty_terminal/src/grid/cell.rs`: `Cell::is_empty()`
  is a `bool`-returning predicate on the cell type itself, called
  throughout selection / iteration code. Same pattern: predicate lives
  on the type, callers consume.
- **WezTerm** `term/src/cell.rs`: `Cell::attrs().wide()` and related
  predicates are methods on the attrs struct, not inline bitwise ops.

**Depends on:** None.

---

## 02.1 Add CellFlags::is_spacer predicate + sibling test

**File(s):** `oriterm_core/src/cell/mod.rs`,
`oriterm_core/src/cell/tests.rs`.

**Context:** The predicate must be defined on `CellFlags` (the bitflags
struct) so it composes with other flag operations. A convenience
`Cell::is_spacer()` alias may be added at the same time if call sites
benefit, but the canonical definition lives on `CellFlags`.

- [ ] Add the predicate after the `bitflags!` block in `cell/mod.rs`:
  ```rust
  impl CellFlags {
      /// Returns `true` when this cell is the trailing or leading
      /// continuation half of a wide character. Cell-iteration loops
      /// (GPU emit, HTML serialization, plain-text serialization,
      /// selection extraction) skip these cells because the rendered
      /// glyph or copied text was emitted by the partner cell.
      ///
      /// Adding a new spacer variant (e.g. emoji ZWJ continuation)
      /// only requires updating this method.
      #[inline]
      #[must_use]
      pub const fn is_spacer(self) -> bool {
          self.intersects(Self::WIDE_CHAR_SPACER.union(Self::LEADING_WIDE_CHAR_SPACER))
      }
  }
  ```
- [ ] (Optional) Add `Cell::is_spacer(&self) -> bool { self.flags.is_spacer() }`
      if call sites prefer `cell.is_spacer()` over `cell.flags.is_spacer()`.
      Not load-bearing.
- [ ] Add tests in `oriterm_core/src/cell/tests.rs`:
  - [ ] `is_spacer_returns_true_for_wide_char_spacer`
  - [ ] `is_spacer_returns_true_for_leading_wide_char_spacer`
  - [ ] `is_spacer_returns_false_for_wide_char_anchor` (negative pin —
        the anchor cell is NOT a spacer; only its neighbour is)
  - [ ] `is_spacer_returns_false_for_no_flags`
  - [ ] `is_spacer_returns_false_for_other_flags` (test with BOLD,
        ITALIC, OVERLINE, SUPERSCRIPT, SUBSCRIPT — none of these are
        spacers)
  - [ ] `is_spacer_returns_true_when_both_spacer_flags_set`
        (defensive — `intersects` semantics)

---

## 02.2 Migrate 5 call sites

**File(s):**
1. `oriterm/src/gpu/prepare/mod.rs` (or `oriterm/src/gpu/prepare/emit_cell.rs`
   if Section 01 has already landed)
2. `oriterm/src/gpu/prepare/dirty_skip/mod.rs` (or absorbed into
   `emit_cell` per Section 01)
3. `oriterm/src/gpu/prepare/unshaped.rs` (or absorbed into `emit_cell`)
4. `oriterm_core/src/selection/html/mod.rs` (TWO sites: `append_html_cells`
   and `append_cells_dual`)
5. `oriterm_core/src/selection/text.rs` (`append_text_cells`)

**Context:** The migration is mechanical. Each site replaces:

```rust
if cell.flags.intersects(CellFlags::WIDE_CHAR_SPACER | CellFlags::LEADING_WIDE_CHAR_SPACER) {
    continue;
}
```

with:

```rust
if cell.flags.is_spacer() {
    continue;
}
```

- [ ] Site 1: `oriterm/src/gpu/prepare/mod.rs` — replace the inline
      predicate at the top of the per-cell loop. (If Section 01 has
      landed, this site has already migrated into `emit_cell`'s caller —
      verify via grep.)
- [ ] Site 2: `oriterm/src/gpu/prepare/dirty_skip/mod.rs` — same
      treatment for the dirty-row branch.
- [ ] Site 3: `oriterm/src/gpu/prepare/unshaped.rs` — same treatment.
- [ ] Site 4a: `oriterm_core/src/selection/html/mod.rs` —
      `append_html_cells` per-cell loop (around line 208-213). Replace
      inline predicate.
- [ ] Site 4b: `oriterm_core/src/selection/html/mod.rs` —
      `append_cells_dual` per-cell loop (around line 271-275). Replace
      inline predicate. (Section 04 will later collapse 4a and 4b into
      one shared run-builder; this is fine — Section 04 only needs the
      already-canonical predicate.)
- [ ] Site 5: `oriterm_core/src/selection/text.rs` — `append_text_cells`
      per-cell loop (around line 89-94). Replace inline predicate.
- [ ] Repo audit: `rg -n "intersects\(.*WIDE_CHAR_SPACER" --type rust`
      should return zero hits after the migration. If it returns any,
      add the missing site to the list and migrate it before marking
      this section complete.

---

## 02.R Third Party Review Findings

Track findings from `/tpr-review` runs against Section 02 here. Leave the
block in place even when empty so tooling has a stable anchor.

- None.

When findings exist, use the format documented in
`plans/_template/plan.md`.

Rules: as documented in `plans/_template/plan.md`.

---

## 02.N Build & Verify

### TDD Matrix

| Test (in `cell/tests.rs`) | Pin type | Lock-in target |
|---|---|---|
| `is_spacer_returns_true_for_wide_char_spacer` | semantic | `WIDE_CHAR_SPACER` → true |
| `is_spacer_returns_true_for_leading_wide_char_spacer` | semantic | `LEADING_WIDE_CHAR_SPACER` → true |
| `is_spacer_returns_false_for_wide_char_anchor` | **negative** | `WIDE_CHAR` (anchor) → false |
| `is_spacer_returns_false_for_no_flags` | semantic | empty → false |
| `is_spacer_returns_false_for_other_flags` | **negative** | BOLD / ITALIC / OVERLINE / SUPERSCRIPT / SUBSCRIPT → false |
| `is_spacer_returns_true_when_both_spacer_flags_set` | semantic | union → true |

### Completion Checklist

- [ ] `./build-all.sh` passes
- [ ] `./clippy-all.sh` passes
- [ ] `./test-all.sh` passes
- [ ] `CellFlags::is_spacer()` exists, is `#[inline] #[must_use] const`,
      with a doc comment explaining the contract
- [ ] All 5+ call sites migrated to `cell.flags.is_spacer()` (or
      `cell.is_spacer()` if the convenience alias was added)
- [ ] Repo grep `rg -n "intersects\(.*WIDE_CHAR_SPACER" --type rust`
      returns zero hits
- [ ] `/tpr-review` against this section returns clean (or all findings
      `[x]` resolved in 02.R)
- [ ] No regressions in `cargo test -p oriterm_core`,
      `cargo test -p oriterm --lib gpu::prepare`,
      `cargo test -p oriterm_core --lib selection`

**Exit Criteria:** `rg -n "WIDE_CHAR_SPACER" --type rust` returns exactly
two hits — the `bitflags!` definition and the `is_spacer` implementation.
Every other former call site now uses the canonical predicate.
