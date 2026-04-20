---
section: "18"
title: "Charsets + UAX Policy"
status: not-started
reviewed: false
goal: "Drive every catalog row in `catalog/charsets.md` from `implemented-unverified` to `verified`, IMPLEMENT the missing NRCS variants (currently only ASCII + Special Graphics exist per Pass 1), implement ISO 2022 multibyte set switching, and verify Unicode policy compliance against UAX #11 (East Asian Width), UAX #29 (Grapheme Clustering), UAX #9 (Bidi), variation selectors VS15/VS16, and emoji ZWJ sequences."
success_criteria:
  - "Top-down spec audit committed at `plans/spec-conformance/audits/section-18-top-down-inventory.md`. Every sequence in the canonical spec source(s) for this stack (ISO 2022 charset designations; ISO 8859 family; DEC technical manuals NRCS variants; Unicode UAX #9 Bidi, #11 East Asian Width, #29 Grapheme Clustering; DEC special-graphics + line-drawing + technical + supplemental + dingbats charts) maps to a catalog row ID OR carries an explicit `not-targeted` decision with rationale. `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes for this audit file. This is enforced PER `plans/spec-conformance/audits/README.md` lint contract — added by Section 09A as the SSOT for top-down catalog coverage to prevent the bottom-up gap that hid DECRQCRA from the catalog."
  - "Every row in `catalog/charsets.md` is `verified`"
  - "**NRCS variants implemented**: every NRCS variant in scope (ANSI X3.4, BS, DE, FI, FR, FR_CA, IT, NL, NO, PT, SE, SP, SU, CH, JIS Roman, JIS Kana, KOR, ARA, GREEK, HEB, RUS, TUR) added to `crates/vte/src/ansi/attr.rs::StandardCharset` enum and dispatched correctly"
  - "ISO 2022 multibyte set switching verified: G0/G1/G2/G3 designation via `ESC ( <intermediate> <final>` sequences, locking shifts (LS2/LS3/LS1R/LS2R/LS3R), single shifts (SS2/SS3) all work for multibyte sets (JIS X 0208, GB 2312, KSC 5601)"
  - "**ISO 8859 family (parts 1-16)** verified: each ISO 8859 part is a single-byte charset ori_term accepts via the `ESC - <final>` / `ESC . <final>` / `ESC / <final>` G1/G2/G3 designation sequences. Decode tables for each part committed under `oriterm_core/src/term/charset/tables/iso_8859/`. Catalog rows for every ISO 8859 part in the authority ladder are `verified`."
  - "**UAX #11 East Asian Width verified**: CJK characters render as width 2; halfwidth/fullwidth distinctions correct; ambiguous-width policy documented per `de-facto-behaviors.md` (default narrow per most modern terminals)"
  - "**UAX #29 grapheme clustering verified**: combining marks attach to base characters; ZWJ sequences cluster correctly; extended grapheme clusters tested against the Unicode test suite (`auxiliary/GraphemeBreakTest.txt`)"
  - "**UAX #9 bidi verified**: terminal documents its bidi policy in `de-facto-behaviors.md` (most terminals are explicitly logical-order LTR with no bidi reordering — verify ori_term matches and document); if ori_term implements bidi reordering, verify against UAX #9"
  - "Variation selectors VS15 (text presentation) / VS16 (emoji presentation) verified: emoji codepoints respect the selector when measuring width and rendering"
  - "Emoji ZWJ sequences verified: family/profession/skin-tone-modifier sequences cluster as one grapheme and render as one cell (or multiple cells per the documented policy)"
  - "All existing teseq + visual_regression tests pass without modification"
  - "Cross-platform: charset designation works the same on macOS/Linux/Windows"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Verification chain complete per row**"
inspired_by:
  - "ISO 2022 standard (committed via `plans/spec-conformance/specs/manifest.toml` if redistributable, else fetched)"
  - "DEC technical manuals — NRCS variant tables for each national language"
  - "Unicode UAX #9, #11, #29 — committed under `plans/spec-conformance/specs/unicode-uax-{9,11,29}.txt`"
  - "ori_term existing `oriterm_core/src/term/charset/mod.rs` + `crates/vte/src/ansi/attr.rs:204-208` (only ASCII + Special exist per Pass 1)"
depends_on: ["08"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "18.0"
    title: "Top-down spec audit (BLOCKING)"
    status: not-started
  - id: "18.1"
    title: "Implement NRCS variant enum + dispatch (single-byte NRCS)"
    status: not-started
  - id: "18.2"
    title: "Implement ISO 8859 family designation (parts 1-16, single-byte upper-half)"
    status: not-started
  - id: "18.3"
    title: "Implement ISO 2022 multibyte set switching (JIS X 0208, GB 2312, KSC 5601)"
    status: not-started
  - id: "18.4"
    title: "Verify UAX #11 East Asian Width"
    status: not-started
  - id: "18.5"
    title: "Verify UAX #29 Grapheme Clustering against the Unicode test suite"
    status: not-started
  - id: "18.6"
    title: "Verify UAX #9 Bidi (or document logical-order policy)"
    status: not-started
  - id: "18.7"
    title: "Verify variation selectors VS15/VS16 + emoji ZWJ sequences"
    status: not-started
  - id: "18.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "18.N"
    title: "Completion Checklist"
    status: not-started
# TPR Checkpoint Placement (reordered per Codex midpoint feedback — 18.1/18.2/18.3 all
# mutate StandardCharset and the designation dispatch; treat them as a single
# implementation wave with the TPR checkpoint AFTER all three, so the reviewer can
# assess the full single-byte + multibyte picture at once):
#   18.3 → TPR (covers .1-.3, all charset enum + designation mutation)
#   18.5 → TPR (covers .4-.5, UAX #11 + #29)
#   final TPR in 18.N
#
# Ordering rationale: 18.1 single-byte NRCS first (adds enum variants, smallest delta),
# then 18.2 ISO 8859 single-byte upper-half (same enum, same designation arms, extends
# the pattern), then 18.3 multibyte ISO 2022 (adds the `$` intermediate dispatch path
# and the decode-table pipeline — largest delta). Single-byte before multibyte minimises
# merge conflicts on StandardCharset and keeps each subsection's diff scoped.
---

# Section 18: Charsets + UAX Policy

**Status:** Not Started
**Goal:** Verify every charset catalog row, implement the missing NRCS variants and ISO 2022 multibyte support, and verify Unicode policy compliance.

**Success Criteria:** see frontmatter.

**Context:** Pass 1 confirmed that `crates/vte/src/ansi/attr.rs:204-208` only defines two charsets in the `StandardCharset` enum: `Ascii` and `SpecialCharacterAndLineDrawing`. Every NRCS variant is missing. ISO 2022 multibyte set switching (JIS X 0208, GB 2312, KSC 5601) is also missing. The charset designation handlers exist (G0/G1/G2/G3 designation, locking shifts, single shifts) but only for the two existing charsets. Unicode policy is mostly implicit through the `unicode-width` crate dependency, but UAX #11/#29/#9 compliance has never been explicitly verified. This section closes both gaps.

**Reference implementations:** see frontmatter.

**Depends on:** Section 08 (baseline charset designation handlers verified — section 08 doesn't expand charsets, so this section is the first to add new variants).

---

## 18.0 Top-down spec audit (BLOCKING — precedes all other subsections)

**Goal:** Walk the canonical spec source(s) for this stack TOP-DOWN. Every sequence the spec defines gets a row in this section's audit file at `plans/spec-conformance/audits/section-18-top-down-inventory.md`, mapped to either an existing catalog row ID or an explicit `not-targeted` decision with rationale.

**Why this exists:** Section 09A introduced the `audits/` SSOT to close the bottom-up catalog construction gap that hid DECRQCRA (and the entire DEC private rectangular-ops family) from the catalog. The original Section 01 catalog bootstrap was bottom-up — sequences absent from both the catalog AND the test corpus are invisible. The per-section audit file makes top-down coverage mechanically lintable: `spec-coverage-report --check audit-files` fails CI if any audit-file mapping does not resolve to a real catalog row.

**Canonical spec source(s):** ISO 2022 (row-by-row enumerator for G0/G1/G2/G3 designation sequences — every `ESC ( <final>`, `ESC ) <final>`, `ESC * <final>`, `ESC + <final>`, `ESC $ <final>` maps to a catalog row); ISO 8859 parts 1-16 (single-byte upper-half charsets); DEC technical manuals (NRCS variant tables for British/German/French/FrenchCanadian/Italian/Dutch/NorwegianDanish/Portuguese/Swedish/Spanish/Finnish/Swiss + JIS Roman/Katakana + DEC special-graphics/line-drawing/technical/supplemental/dingbats); Unicode UAX #9 (Bidi), UAX #11 (East Asian Width), UAX #29 (Grapheme Clustering).

**Files touched:**
- `plans/spec-conformance/audits/section-18-top-down-inventory.md` (NEW — stub created by Section 09A's §09A.10; populated by this subsection)
- `plans/spec-conformance/catalog/charsets.md` (open new rows for any designation sequences present in the spec but not yet catalogued)

**Completion criteria:**

- [ ] Audit file `plans/spec-conformance/audits/section-18-top-down-inventory.md` is populated with every sequence in the canonical spec source(s).
- [ ] Every row has a `Decision` of `mapped` (cites catalog row ID) or `not-targeted` (with rationale).
- [ ] Every `mapped` row resolves to a real catalog row.
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes for this audit file.
- [ ] Audit file `last_walked` and `walked_by` set.
- [ ] Any new catalog rows use the canonical 10-column schema.

**No other subsection in this section can begin work until §18.0 is complete.**

---

## 18.1 Implement NRCS variant enum + dispatch (single-byte NRCS)

**File(s):** `crates/vte/src/ansi/attr.rs`, `crates/vte/src/ansi/dispatch/mod.rs`, `oriterm_core/src/term/charset/mod.rs`, sibling tests

- [ ] Extend `StandardCharset` enum in `crates/vte/src/ansi/attr.rs` to include every NRCS variant in scope. Reference DEC technical manuals for the exact code points each variant remaps:
  - `BritishOrIrish` (UK NRCS) — remaps `#` to `£`
  - `German` — remaps `[`, `\`, `]`, `{`, `|`, `}` to umlauts and ß
  - `French` — remaps for French accents
  - `FrenchCanadian`
  - `Italian`
  - `Dutch`
  - `NorwegianDanish`
  - `Portuguese`
  - `Swedish`
  - `Spanish`
  - `Finnish`
  - `Swiss`
  - `JisRoman` — Japanese Romaji
  - `JisKatakana` — JIS X 0201 katakana
  - `KoreanWanseong`
- [ ] Update `crates/vte/src/ansi/dispatch/mod.rs::esc_dispatch()` charset designation arms to recognize the NRCS final bytes (`A` for British, `5` or `C` for Finnish, `K` for German, etc., per DEC manuals)
- [ ] Update `oriterm_core/src/term/charset/mod.rs::CharsetState` to apply the new NRCS variants when translating output bytes through the active charset
- [ ] **Important**: NRCS code point remapping must NOT break UTF-8 — the remapping only applies to ASCII bytes (0x21-0x7E) when the active charset is an NRCS variant. Multibyte UTF-8 sequences pass through untranslated.
- [ ] Sibling tests in `oriterm_core/src/term/charset/tests.rs`:
  - `german_nrcs_remaps_brackets_to_umlauts()`
  - `british_nrcs_remaps_hash_to_pound()`
  - `nrcs_does_not_break_utf8_passthrough()`
- [ ] Update catalog rows in `catalog/charsets.md` for each NRCS variant to `verified`.
- [ ] **Validation**: NRCS designation tests pass; existing charset tests still pass.

---

## 18.2 Implement ISO 8859 family designation (parts 1-16, single-byte upper-half)

**File(s):** `crates/vte/src/ansi/attr.rs` (StandardCharset extended), `oriterm_core/src/term/charset/tables/iso_8859/{latin1,latin2,latin3,latin4,cyrillic,arabic,greek,hebrew,latin5,latin6,thai,latin7,latin8,latin9,latin10}.rs` (new), `crates/vte/src/ansi/dispatch/mod.rs` (designation arms), spec_chain tests

Each ISO 8859 part (1-16) is a single-byte charset mapping the upper half of the code space (0xA0-0xFF) to different national character sets. Terminals may designate an ISO 8859 part to a G slot via `ESC - <final>` (G1), `ESC . <final>` (G2), or `ESC / <final>` (G3). The final byte identifies the part (e.g. `A` for ISO 8859-1, `B` for ISO 8859-2, etc. per the ISO International Register of Coded Character Sets).

- [ ] Extend `StandardCharset` enum with every ISO 8859 part variant (`Iso8859_1` through `Iso8859_16`, skipping retired parts)
- [ ] Commit the decode tables — each table maps byte 0xA0-0xFF to the corresponding Unicode codepoint. Source data: Unicode consortium `MAPPINGS/ISO8859/` directory (freely redistributable per the Unicode Terms of Use).
- [ ] Extend the designation dispatch in `crates/vte/src/ansi/dispatch/mod.rs` to recognize `ESC - A` (ISO 8859-1 to G1), `ESC - B` (ISO 8859-2 to G1), etc. The full final-byte to ISO 8859 mapping lives in the ISO International Register.
- [ ] Update `oriterm_core/src/term/charset/mod.rs` to apply the correct decode table when an ISO 8859 part is the active charset for GL or GR
- [ ] Spec_chain tests:
  - `iso_8859_1_designation_to_g1_plus_ls1_plus_byte_0xe9_decodes_to_u00e9()` (é)
  - `iso_8859_5_cyrillic_decodes_byte_0xe0_to_u0430()` (а)
  - `iso_8859_7_greek_decodes_byte_0xe1_to_u03b1()` (α)
  - `iso_8859_16_decodes_romanian_specific_characters()`
- [ ] Update catalog rows for every ISO 8859 part in `catalog/charsets.md` to `verified`
- [ ] **Validation**: every committed decode table is complete (no 0x00 holes beyond the genuinely unmapped positions); tests pass.

---

## 18.3 Implement ISO 2022 multibyte set switching

**File(s):** `crates/vte/src/ansi/attr.rs`, `crates/vte/src/ansi/dispatch/mod.rs`, `oriterm_core/src/term/charset/mod.rs`, sibling tests

ISO 2022 supports multibyte character sets via `ESC $ <intermediate> <final>` designation sequences (note the `$` intermediate, distinct from single-byte `ESC ( <final>`). The supported sets include JIS X 0208 (Japanese), GB 2312 (Chinese), KSC 5601 (Korean).

- [ ] Add multibyte variants to `StandardCharset`:
  - `JisX0208`
  - `Gb2312`
  - `Ksc5601`
- [ ] Add the `ESC $ A` (GB 2312), `ESC $ B` (JIS X 0208), `ESC $ C` (KSC 5601) designation sequence handling to the dispatch
- [ ] **Decode path (required, not deferred)**: when a multibyte set is designated to a G slot AND that slot is active (GL or GR), the terminal interprets incoming bytes as the multibyte charset and MAPS each two-byte sequence to the corresponding Unicode codepoint via a lookup table. Commit the lookup tables under `oriterm_core/src/term/charset/tables/jis_x_0208.rs`, `gb_2312.rs`, `ksc_5601.rs`. The tables are generated from the Unicode consortium's `index-jis0208.txt`, `index-gb18030-2022.txt`, `index-euc-kr.txt` — commit the generator script and the committed tables together.
- [ ] Charset state machine: the multibyte path produces a `DecodedCluster { codepoint, width }` per two-byte sequence; the terminal's character write path consumes the cluster the same way it consumes a UTF-8 cluster
- [ ] Spec_chain tests for designation tracking AND decode:
  - `esc_dollar_b_designates_jis_x_0208_to_g0()`
  - `jis_x_0208_two_byte_sequence_maps_to_correct_unicode_codepoint()`
  - `gb_2312_decode_table_spot_checks_against_index_gb18030()`
  - `ksc_5601_decode_table_spot_checks_against_index_euc_kr()`
  - `multibyte_designation_survives_ls2_ls3_shifts()`
- [ ] Update catalog rows for every ISO 2022 multibyte set to `verified` (NOT `verified-with-deviation`)
- [ ] **Validation**: designation state tracked correctly; multibyte decode tables produce expected Unicode codepoints; existing charset tests still pass.
- [ ] **TPR checkpoint** — `/tpr-review` covering 18.1-18.3 (NRCS + ISO 8859 single-byte + ISO 2022 multibyte). All three subsections mutate `StandardCharset` and the designation dispatch, so they are reviewed as a single wave. Catches charset designation interaction bugs before UAX work.

---

## 18.4 Verify UAX #11 East Asian Width

**File(s):** `oriterm_core/tests/spec_chain/charsets/uax_11.rs` (new)

- [ ] Read UAX #11 from `plans/spec-conformance/specs/unicode-uax-11.txt`
- [ ] Spec_chain tests covering each width category:
  - Narrow (N): basic Latin
  - Wide (W): CJK characters, fullwidth forms
  - Halfwidth (H): halfwidth katakana
  - Fullwidth (F): fullwidth digits, fullwidth latin
  - Ambiguous (A): characters that may be narrow or wide depending on context — document the chosen policy (default narrow is the modern convention)
  - Neutral (Na): everything else
- [ ] For each category, write a test that places the character in the grid and asserts the cell width matches expected (1 for narrow/halfwidth, 2 for wide/fullwidth, configurable for ambiguous)
- [ ] Update `catalog/charsets.md` UAX-11 row to `verified`
- [ ] **Validation**: tests pass.

---

## 18.5 Verify UAX #29 Grapheme Clustering against the Unicode test suite

**File(s):** `oriterm_core/tests/spec_chain/charsets/uax_29.rs` (new), `plans/spec-conformance/specs/unicode-grapheme-break-test.txt` (committed test data from `auxiliary/GraphemeBreakTest.txt`)

- [ ] Download `https://www.unicode.org/Public/16.0.0/ucd/auxiliary/GraphemeBreakTest.txt` and commit it under `plans/spec-conformance/specs/` (verify license — Unicode test data is freely redistributable per the Unicode Terms of Use)
- [ ] Write a parameterized test that walks every test case in the file (each line: `÷ <codepoints> ÷` where `÷` is a grapheme break and `×` is a non-break)
- [ ] For each test case, feed the codepoints through ori_term's grapheme clustering and assert the cluster boundaries match
- [ ] Update `catalog/charsets.md` UAX-29 row to `verified`
- [ ] **Validation**: every test case in `GraphemeBreakTest.txt` passes
- [ ] **TPR checkpoint** — `/tpr-review` covering 18.4-18.5 (UAX #11 + #29).

---

## 18.6 Verify UAX #9 Bidi (or document logical-order policy)

**File(s):** `oriterm_core/tests/spec_chain/charsets/uax_9.rs` (new), `plans/spec-conformance/catalog/de-facto-behaviors.md` (updated)

- [ ] Determine ori_term's bidi policy. Most terminals are explicitly logical-order LTR with NO bidi reordering — they emit the bytes in logical order and let the application handle bidi (which is the de-facto convention because reordering interacts badly with cursor positioning, selection, and copy-paste).
- [ ] If ori_term is logical-order LTR (most likely): document the policy in `de-facto-behaviors.md` and mark the UAX-9 row as `verified-with-deviation` with the explicit reference to xterm/wezterm/kitty behavior.
- [ ] If ori_term implements bidi reordering: verify against UAX #9 with test cases.
- [ ] Spec_chain test verifying the policy: feed an Arabic or Hebrew sequence and assert the cell layout matches the documented policy (logical order, no reordering).
- [ ] **Validation**: policy documented and tested; row verified-with-deviation if appropriate.

---

## 18.7 Verify variation selectors VS15/VS16 + emoji ZWJ sequences

**File(s):** `oriterm_core/tests/spec_chain/charsets/variation_selectors.rs` (new), `oriterm_core/tests/spec_chain/charsets/emoji_zwj.rs` (new)

- [ ] Spec_chain test for VS15 (text presentation): emit a base emoji + VS15, assert the cell width is 1 (text presentation) and not 2 (emoji presentation)
- [ ] Spec_chain test for VS16 (emoji presentation): emit a base emoji + VS16, assert the cell width is 2 (emoji)
- [ ] Spec_chain test for emoji ZWJ sequence: emit `👨‍👩‍👧‍👦` (family emoji = man + ZWJ + woman + ZWJ + girl + ZWJ + boy), assert the cluster is 1 grapheme and renders as 1-or-2 cells per the documented policy
- [ ] Test profession emoji (`👨‍💻` man technologist), skin tone modifiers
- [ ] Update catalog rows to `verified`
- [ ] **Validation**: variation selector + ZWJ tests pass.

---

## 18.R Third Party Review Findings

- None.

---

## 18.N Completion Checklist

- [ ] Failing test matrix written FIRST (TDD)
- [ ] **Matrix dimensions**: charset variant (NRCS × ISO 2022 multibyte × DEC special) × designation slot (G0/G1/G2/G3) × shift type (locking/single) × Unicode policy (UAX #11/#29/#9 + variation selectors + ZWJ)
- [ ] **Semantic pin**: UAX #29 grapheme test suite is the regression guard for grapheme clustering
- [ ] Every NRCS variant implemented (18.1)
- [ ] ISO 8859 family (parts 1-16) designation + decode tables implemented (18.2)
- [ ] ISO 2022 multibyte designation tracked, decode tables committed (18.3)
- [ ] UAX #11 East Asian Width verified
- [ ] UAX #29 Grapheme Clustering verified against Unicode test suite
- [ ] UAX #9 Bidi policy documented and tested
- [ ] Variation selectors VS15/VS16 verified
- [ ] Emoji ZWJ sequences verified
- [ ] All existing charset + width tests pass
- [ ] Cross-platform encoder tests pass
- [ ] Alloc regression unchanged
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` Quick Reference + mission criteria updated
- [ ] `index.md` section 18 status updated
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** Every charset catalog row is `verified`; NRCS variants and ISO 2022 multibyte designation implemented; UAX #11/#29 explicitly verified against the Unicode test suite; UAX #9 policy documented; emoji ZWJ + variation selectors verified.
