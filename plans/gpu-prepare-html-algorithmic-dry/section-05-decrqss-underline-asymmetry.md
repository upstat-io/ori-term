---
section: "05"
title: "DECRQSS SGR underline-variant coverage (F-10) + plan cleanup"
status: not-started
reviewed: false
goal: "Extend `build_sgr_string` to emit SGR 21 (DOUBLE_UNDERLINE), SGR 4:3/4:4/4:5 (CURLY/DOTTED/DASHED), and colored-underline. Then delete the plan directory."
depends_on: ["01", "02", "03", "04"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "05.1"
    title: "Extend build_sgr_string for DOUBLE_UNDERLINE (SGR 21)"
    status: not-started
  - id: "05.2"
    title: "Extend build_sgr_string for CURLY / DOTTED / DASHED (SGR 4:3 / 4:4 / 4:5)"
    status: not-started
  - id: "05.3"
    title: "Extend build_sgr_string for colored-underline"
    status: not-started
  - id: "05.4"
    title: "DECRQSS roundtrip tests"
    status: not-started
  - id: "05.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "05.N"
    title: "Build & Verify"
    status: not-started
  - id: "05.Z"
    title: "Final cleanup — delete plan directory"
    status: not-started
---

# Section 05: DECRQSS SGR underline-variant coverage + plan cleanup

**Goal:** Close the DECRQSS SGR query asymmetry exposed by BUG-06-014 —
the responder correctly emits the new SGR 53/73/74 codes but omits the
older underline variants (SGR 21 DOUBLE_UNDERLINE, SGR 4:3 CURLY,
SGR 4:4 DOTTED, SGR 4:5 DASHED, and colored-underline). After this
section lands, DECRQSS faithfully echoes every styled-underline
combination the terminal already renders. Then delete this plan
directory as the closing cleanup step.

**Production code path:** `build_sgr_string` in
`oriterm_core/src/term/handler/status.rs:25-90`. Invoked by the DECRQSS
handler in response to a `CSI $ q m` query. Per ECMA-48 + the kitty
underline-style extension.

**Observable change:** A program issuing DECRQSS for current SGR state
will receive a complete set of style codes including DOUBLE_UNDERLINE,
CURLY, DOTTED, DASHED, and colored-underline when those styles are
active. Today these are silently omitted.

**Context:** The Phase 5 hygiene report flagged this as Minor GAP F-10
(decrqss-sgr-asymmetry, pre-existing). Per the Broken Window Policy in
CLAUDE.md, pre-existing gaps surfaced by current work are owned by the
discovering work — "pre-existing" is diagnosis only, never justification
for ignoring. This section is the smallest scope that closes the gap
fully.

**Reference implementations:**
- **WezTerm** `wezterm-term/src/terminalstate/decrqss.rs`: emits the
  full set of underline-style codes including 4:3 / 4:4 / 4:5 and
  colored-underline (SGR 58:n).
- **Kitty** terminfo extension `terminfo/kitty.terminfo`: documents the
  CSI 4:n parameter syntax for underline styles.
- **ECMA-48** §8.3.117 (SGR): defines SGR 21 as "doubly underlined".

**Depends on:** Sections 01-04. This section runs LAST so the cleanup
step can delete the plan directory only after every preceding section is
verified.

---

## 05.1 Extend build_sgr_string for DOUBLE_UNDERLINE (SGR 21)

**File(s):** `oriterm_core/src/term/handler/status.rs:25-90`.

- [ ] Walk `build_sgr_string` and identify the underline-style branch.
      Today it likely emits only `SGR 4` for the basic underline flag.
- [ ] Add a branch: when the cell's underline-style is `Double`, emit
      `21` instead of (or in addition to) `4`.
  ```rust
  match attrs.underline_style() {
      UnderlineStyle::None => {}
      UnderlineStyle::Single => out.push_str(";4"),
      UnderlineStyle::Double => out.push_str(";21"),
      UnderlineStyle::Curly  => out.push_str(";4:3"),
      UnderlineStyle::Dotted => out.push_str(";4:4"),
      UnderlineStyle::Dashed => out.push_str(";4:5"),
  }
  ```
- [ ] Confirm `UnderlineStyle` enum exhaustively matches every variant
      the terminal already renders. If not, the gap is wider — file
      `/add-bug` and continue with the variants that DO exist.

---

## 05.2 Extend build_sgr_string for CURLY / DOTTED / DASHED (SGR 4:3 / 4:4 / 4:5)

**File(s):** `oriterm_core/src/term/handler/status.rs`.

- [ ] Per the match arm sketched in 05.1, emit `4:3`, `4:4`, `4:5` for
      Curly, Dotted, Dashed respectively.
- [ ] Confirm the colon-separated parameter syntax is preserved exactly
      (kitty-style sub-parameter — distinct from semicolon-separated
      parameters).

---

## 05.3 Extend build_sgr_string for colored-underline

**File(s):** `oriterm_core/src/term/handler/status.rs`.

**Context:** SGR 58:n encodes underline color (extension of SGR 38/48
syntax). When the cell has a non-default underline color, the responder
must emit it.

- [ ] When `attrs.underline_color()` is non-default (or non-`None`),
      append `;58:5:n` (256-color palette) or `;58:2::r:g:b` (truecolor)
      following the same encoding logic as `;38:` / `;48:` for fg/bg.
- [ ] If `build_sgr_string` already has a helper `encode_color(out, prefix, color)`,
      reuse it with prefix `"58"`. Do not duplicate the encoding.

---

## 05.4 DECRQSS roundtrip tests

**File(s):** `oriterm_core/src/term/handler/status.rs` sibling tests, OR
`oriterm_core/src/term/handler/tests.rs` if status-tests live there.

- [ ] Add a roundtrip test for each underline variant:
  - [ ] `decrqss_sgr_emits_double_underline_21`
  - [ ] `decrqss_sgr_emits_curly_underline_4_3`
  - [ ] `decrqss_sgr_emits_dotted_underline_4_4`
  - [ ] `decrqss_sgr_emits_dashed_underline_4_5`
  - [ ] `decrqss_sgr_emits_colored_underline_58_5_n` (256-color)
  - [ ] `decrqss_sgr_emits_colored_underline_58_2_rgb` (truecolor)
- [ ] Each test sets the cell attribute, calls `build_sgr_string`,
      asserts the output substring contains the expected SGR sequence.
- [ ] Add a regression pin asserting the existing BUG-06-014 codes
      (SGR 53 / 73 / 74) still appear in the output when those flags
      are set — confirms 05.x changes didn't regress BUG-06-014's
      additions.

---

## 05.R Third Party Review Findings

Track findings from `/tpr-review` runs against Section 05 here. Leave the
block in place even when empty so tooling has a stable anchor.

- None.

Format and rules as documented in `plans/_template/plan.md`.

---

## 05.N Build & Verify

### TDD Matrix

| Test | Pin type | Lock-in target |
|---|---|---|
| `decrqss_sgr_emits_double_underline_21` | semantic | SGR 21 emission |
| `decrqss_sgr_emits_curly_underline_4_3` | semantic | SGR 4:3 emission |
| `decrqss_sgr_emits_dotted_underline_4_4` | semantic | SGR 4:4 emission |
| `decrqss_sgr_emits_dashed_underline_4_5` | semantic | SGR 4:5 emission |
| `decrqss_sgr_emits_colored_underline_58_5_n` | semantic | 256-color underline |
| `decrqss_sgr_emits_colored_underline_58_2_rgb` | semantic | truecolor underline |
| `decrqss_sgr_still_emits_overline_53` | regression | BUG-06-014 SGR 53 not regressed |
| `decrqss_sgr_still_emits_superscript_73` | regression | BUG-06-014 SGR 73 not regressed |
| `decrqss_sgr_still_emits_subscript_74` | regression | BUG-06-014 SGR 74 not regressed |

### Completion Checklist

- [ ] `./build-all.sh` passes
- [ ] `./clippy-all.sh` passes
- [ ] `./test-all.sh` passes
- [ ] Every underline variant the terminal renders is also echoed by
      `build_sgr_string`
- [ ] DECRQSS roundtrip tests cover SGR 21, 4:3, 4:4, 4:5, and 58: variants
- [ ] BUG-06-014 regression pins confirm SGR 53/73/74 still emit
- [ ] `/tpr-review` against this section returns clean (or all findings
      `[x]` resolved in 05.R)

**Exit Criteria:** A program emitting `CSI $ q m` after setting
DOUBLE_UNDERLINE + curly + colored-underline receives every active SGR
code in the response. Section 05 is complete.

---

## 05.Z Final cleanup — delete plan directory

**BLOCKING — runs LAST after every section above is `[x]`.**

- [ ] Confirm every section in this plan has `status: complete` in its
      frontmatter and every checkbox in the section file is `[x]`.
- [ ] Run `timeout 150 ./build-all.sh` — green
- [ ] Run `timeout 150 ./clippy-all.sh` — green
- [ ] Run `timeout 150 ./test-all.sh` — green
- [ ] Run 10 consecutive `cargo test --release -p oriterm_core` and
      `cargo test --release -p oriterm` — all 20 pass
- [ ] Update BUG-06-014's `## Hygiene Findings` block: change the plan
      anchor from "Not Started" to "Resolved (plans/gpu-prepare-html-algorithmic-dry/
      deleted YYYY-MM-DD)". Cite the deletion commit hash.
- [ ] Commit final batch via `/commit-push`.
- [ ] **Delete this plan directory**:
      `rm -rf plans/gpu-prepare-html-algorithmic-dry/`
- [ ] Commit the deletion with
      `chore: archive completed gpu-prepare-html-algorithmic-dry cleanup plan`.
- [ ] Push.

---

**Plan-level Exit Criteria:** Every `[ ]` above is `[x]`, the entire
`plans/gpu-prepare-html-algorithmic-dry/` directory has been deleted,
and the deletion has been committed and pushed. BUG-06-014's hygiene
findings block cites the deletion commit. The slice has been
hygiene-clean across all 4 review passes (LEAK/SSOT, Algorithmic DRY,
Boundary/Flow, Surface) since the last cleanup batch landed.
