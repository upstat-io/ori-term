# `spec_chain::coverage` scanner + aggregator — Design Notes and Improvement Log

**Purpose of this file.** Institutional memory for the spec-conformance coverage gate that sits between the Section-10/11/12 test files and the catalog rows they verify. Captures the **design philosophy** of the citation scanner + the coverage aggregator (so future edits don't regress them), the **load-bearing invariants** (what must not change without a plan), and a **running log of drift patterns discovered in the wild**.

**Scope.** Covers:
- `crates/oriterm_test_support/src/spec_chain/coverage/scan.rs` — the citation scanner (walks test directories, extracts catalog row IDs from doc-comments and `catalog_row_id:` fields)
- `crates/oriterm_test_support/src/spec_chain/coverage/mod.rs` — the coverage aggregator (joins catalog rows with citations to produce `CoverageReport` + `has_regression` check)
- `crates/oriterm_test_support/src/bin/catalog_coverage_check.rs` — the `catalog_coverage_check check` gate consumed by `./test-all.sh`
- `crates/oriterm_test_support/src/bin/spec-coverage-report.rs` — the `spec-coverage-report` per-stack verified/partial/impl/stub/missing table

These four files are one logical tool — the scanner finds citations, the aggregator joins them against catalog rows, one binary gates CI, the other reports progress.

**When to update this file.** Any time a test author silently drops coverage because the scanner misses a citation form, any time a catalog row slips from `verified` → `implemented-unverified` without a test backing it, or any time the coverage report produces a wrong or confusing number. Add a `- [ ]` item under §6 for in-the-wild findings; add a `- [x]` when a fix lands.

---

## §1 — Core Design Philosophy (KEEP THIS)

1. **Citations are text-scanned, not type-checked.** The scanner walks `.rs` files and extracts strings by prefix match. It does not parse Rust AST, does not instantiate `SpecScenario` values, does not run tests. Adding a new citation form means teaching the scanner a new prefix; do NOT try to introspect Rust types at scan time — that reintroduces a compile dependency the scanner intentionally avoids.

2. **The accepted set of prefixes is the contract.** `// Catalog row:`, `//! Catalog row:`, `/// Catalog row:`, their plural counterparts (`Catalog rows:`), and the const field `catalog_row_id:`. Any other form is silently ignored. The module doc at the top of `scan.rs` is the human-facing list — keep it in sync with `prefixes: [&str; 6]` in `scan_file()`.

3. **Silent acceptance, not silent rejection.** When a test author writes a citation in an almost-right form (e.g., `Catalog rows` instead of `Catalog row`), the scanner must either accept it or fail loudly — never silently ignore. The 2026-04-19 plural-form fix landed because silent ignore was the prior behavior and it produced a false-missing coverage report.

4. **Aggregator is canonical for `verified`.** `CoverageReport::aggregate` reads the `Verification` column of every catalog row AND cross-checks that at least one test citation exists for each `verified` row. A row marked `verified` in the catalog with zero citations is a `false_verified` finding — the aggregator surfaces it, the CI gate rejects it. This is the primary drift-prevention mechanism between catalog text and actual test coverage.

5. **Exclude dirs prevent self-citation.** The scanner is itself a Rust file that contains citation-form strings in its tests (`//! Catalog row: OSC-52` in `tests.rs`). The `exclude_dirs` list in `catalog_coverage_check check` includes the scanner's own source directory so those test fixtures don't register as real citations. Any test fixture that writes citation-form strings via `std::fs::write` into a `tempfile::tempdir()` path is safe because tempdirs are outside the scanned tree.

6. **One citation per row-ID per file is sufficient.** The aggregator deduplicates — a test file that mentions `OSC-52-STORE` in its module doc AND in a `/// Catalog row:` comment AND in a `catalog_row_id:` field still counts as one citation. Authors can be liberal with citations without inflating the per-stack count.

---

## §2 — Load-Bearing Invariants

| Invariant | Failure mode it prevents |
|-----------|--------------------------|
| Every citation prefix is listed in both the `prefixes` array AND the module doc | Scanner accepts a form the human-facing list doesn't advertise → future authors don't discover the form and silently underciterate. Or vice-versa: the doc lists a form the scanner doesn't accept → silent false-missing. |
| Plural `Catalog rows:` emits one citation per trimmed non-empty piece | Silent false-missing when a test file covers multiple catalog rows and the author writes the natural-English plural (the 2026-04-19 `§10.2` finding that this design log was created for) |
| Scanner never parses Rust AST | Scanner stays compile-free — runs before the test tree builds, catches coverage drift even if tests fail to compile |
| `catalog_coverage_check` runs as part of `./test-all.sh` | Coverage drift cannot ship — a merged commit with a `verified` row and zero citations fails CI |
| `false_verified` + `uncataloged` are symmetric | Either direction of drift (catalog says `verified` but no test; test cites a row that doesn't exist) surfaces as a named finding, not a silent pass |
| Empty pieces in plural form are skipped | `//! Catalog rows: A,, B,` produces 2 citations (A, B), not 4 (A, empty, B, empty) — preserves SSOT semantics |

---

## §3 — File Inventory

| Path | Lines | Role |
|------|-------|------|
| `crates/oriterm_test_support/src/spec_chain/coverage/scan.rs` | ~130 | Citation scanner — `scan_test_citations()` walks directories; `scan_file()` applies prefix match; `extract_const_field_id()` handles `catalog_row_id:` |
| `crates/oriterm_test_support/src/spec_chain/coverage/mod.rs` | ~200 | Aggregator — `CoverageReport::aggregate()` joins catalog rows with citations; `has_regression()` compares against baseline |
| `crates/oriterm_test_support/src/spec_chain/coverage/tests.rs` | ~230 | Unit tests — per-prefix matrix, plural form pins, empty-piece skip, regression detection |
| `crates/oriterm_test_support/src/bin/catalog_coverage_check.rs` | ~200 | CI gate binary — `check` subcommand fails on `false_verified` + `uncataloged` drift |
| `crates/oriterm_test_support/src/bin/spec-coverage-report.rs` | ~100 | Report binary — prints per-stack verified/partial/impl/stub/missing counts |

**Recently changed (2026-04-19):** `scan.rs` now accepts the plural form `Catalog rows: A, B, C` (comma-separated, any of `//`/`//!`/`///` prefixes). Added 3 tests to `tests.rs` pinning the plural form across all three comment prefixes and the empty-piece skip behavior.

---

## §4 — Lessons from Dogfood / Production Runs

### 2026-04-19 — plural `Catalog rows:` silently dropped

**Symptom.** During §10.2 implementation (OSC 52 clipboard spec_chain tests), I wrote the module doc as `//! Catalog rows: OSC-52-STORE, OSC-52-LOAD` expecting the scanner to emit one citation per ID. Ran `cargo run --bin spec-coverage-report` — the `osc` stack showed `2 verified` instead of the expected `3` (OSC-8 + OSC-52-STORE + OSC-52-LOAD). Catalog rows were marked `verified` but had zero citations → would have become `false_verified` findings if I'd shipped as-is. I worked around it by splitting the doc into two lines (`//! Catalog row: OSC-52-STORE` + `//! Catalog row: OSC-52-LOAD`), which worked, but the natural-English plural form was silently ignored.

**Root cause.** `scan.rs:77-90` used three exact prefixes:
```
"// Catalog row: "
"//! Catalog row: "
"/// Catalog row: "
```
No prefix matched `"//! Catalog rows: "` (plural + s), so the line was silently skipped by the "Pattern 1: comment citation" branch. The `extract_const_field_id()` path at line 94 also did not match. Net: zero citations emitted from a line that clearly intended to cite two rows.

This is exactly the "silent acceptance, not silent rejection" failure mode from §1 Design Philosophy item 3 — the scanner had neither accepted the almost-right form nor rejected it with a clear error. A future test author with the same natural-English instinct would have the same silent-miss.

**Fix.** Two-part:
1. Extended `scan.rs` `prefixes` array from 3 to 6 — singular and plural forms for each of `//`, `//!`, `///`. Replaced the `rest.trim()` singleton push with `rest.split(',').for_each(trim + skip-empty + push)`. The singular form still works because a line with no comma is a one-piece split.
2. Added 3 regression tests in `tests.rs`: `scan_test_citations_finds_plural_form` (OSC-52-STORE + OSC-52-LOAD end-to-end), `scan_test_citations_plural_form_all_comment_prefixes` (matrix across `//`, `//!`, `///`), `scan_test_citations_plural_form_skips_empty_pieces` (defensive pin for trailing/double commas).
3. Updated the module doc at the top of `scan.rs` to advertise the plural form with a sentence explaining why it exists (silent-miss prevention).

**Verification.** After the fix, edited `oriterm_core/tests/spec_chain/osc/clipboard.rs` BACK to `//! Catalog rows: OSC-52-STORE, OSC-52-LOAD` (single line, plural form). Ran `cargo run --bin catalog_coverage_check -- check` → `scanned 16 files, 317 rows / catalog_coverage_check: OK`. Ran `cargo run --bin spec-coverage-report` → `osc 3 0 16 2 21 42` (3 verified, matching expectation: OSC-8 + OSC-52-STORE + OSC-52-LOAD). Plural form now works end-to-end.

**Why this matters beyond §10.2.** Multi-row test files are common in the spec-conformance suite — §10.3 will cover OSC 9 / OSC 99 / OSC 777 in one module, §10.6 will cover OSC 104 / 110 / 111 / 112 in one module, §10.9 will cover 11+ rows. Every one of those modules is a chance for the plural-form silent-miss bug to recur. Fixing once at the scanner level prevents N future incidents.

**Prior art surfaced during the fix.** The `extract_const_field_id()` function uses an anchored `starts_with("catalog_row_id:")` pattern that handles the const field case cleanly — that path is unaffected. The `walk_dir_recursive` + `exclude` machinery is robust; no new data plumbing needed. The fix is ~15 lines of code + ~70 lines of tests.

**Design lesson.** When adding a prefix-based scanner, think about the natural-English variants of the prefix up front. `row` vs `rows` is the obvious one for English count-agreement; future scanners might hit `path` vs `paths`, `file` vs `files`, `id` vs `ids`. The cost of supporting both at creation time is a comma-split loop. The cost of discovering the silent-miss after production use is a §4 entry like this one.

### 2026-04-21 — em-dash inside citation tail caused silent drop, no diagnostic surface

**Symptom.** During §13.1 (kitty graphics verification matrix) implementation, I wrote `/// Catalog row: \`KG-ACTION-TRANSMIT\` — \`a=t\`.` as the per-test citation form — embedding a short action-glyph reference after an em-dash for readability. `./test-all.sh` produced 11 `FALSE VERIFIED` findings (every row I had just flipped to `verified`). The scanner was picking up the citation prefix, matching `KG-ACTION-TRANSMIT`, but then silently dropping it because post-normalization the final text `KG-ACTION-TRANSMIT\` — \`a=t` contained whitespace + em-dash + backticks, which fail the `is_ascii_alphanumeric() || '-' || '_'` validator at `normalize_row_id:213-218`. No diagnostic; I had to read `scan.rs::normalize_row_id` to understand why the matches weren't landing. One round of format iteration to land on the working `\`ID\` (qualifier).` form.

**Root cause.** The normalizer at `scan.rs::normalize_row_id` has 5 sequential reductions (`trim` → `. ` split → ` (` split → trailing-period trim → backtick trim). If the final reduced text still contains whitespace or non-identifier chars (em-dashes, nested backticks, stray periods), it's silently dropped — no log, no error, no way to tell from outside the function that a citation was present but unacceptable. Every failed citation becomes invisible at the `spec-coverage-report` surface.

**Fix.** Added a new `--explain <file>` flag to `spec-coverage-report` (wired into `scan.rs::explain_file` and exported through `coverage/mod.rs`) that walks ONE file's citation lines and prints per-piece normalizer trace: each step's intermediate text, the outcome (ACCEPTED + resulting ID, or DROPPED with specific reason), and a hint pointing at common causes (em-dashes, nested backticks, prose in tail). Also documented the diagnostic in `.claude/rules/tests.md` §Test Hygiene item 7 so future authors hit the rule before the scanner.

**Verification.** Ran `cargo run -p oriterm_test_support --bin spec-coverage-report -- --explain oriterm_core/tests/spec_chain/kitty/actions.rs` — output shows per-piece steps for every citation line, distinguishes accepted from dropped, and reveals exactly why `piece 2: "d=a smoke — per-specifier coverage is §13.0.5's delete/tests.rs matrix)."` fails (whitespace + em-dash after normalization). A future author hitting the same bug runs this command instead of reading source.

**Why this matters beyond §13.1.** Every future kitty subsection (§13.2 through §13.6) adds tests that cite catalog rows; §14 will add iterm2 tests; other sections pile on. Each new test author is a candidate for the em-dash silent-miss. A single `--explain` diagnostic costs ~100 lines of code and saves every future author one round of source-reading. The rule entry in `tests.md` item 7 is the up-front hint; `--explain` is the debugger for when the hint is missed.

**Design lesson.** A silent-drop path is not acceptable when the drop decision is non-trivial. If the normalizer has to reject prose-contaminated input (which it does — catalog IDs must be bare identifiers), the operator needs a way to see WHAT was dropped and WHY. The reactive fix was to add the diagnostic; the proactive version would have been to emit a stderr warning at every drop in normal mode. Chose reactive (`--explain`) because normal-mode stderr warnings would be noisy on legitimate dropped pieces (the catalog-row comment prefix is used by many false-positive paragraphs in the spec_chain source). A dedicated diagnostic flag keeps normal runs quiet while still providing the debug surface.

---

## §5 — Regressions To Watch For

Pre-edit sanity check for future changes to `scan.rs` or `coverage/mod.rs`:

- [ ] Silent-miss from a new almost-right prefix variant (e.g., `Catalog-row:` without space, or `catalog row:` lowercase). If a reviewer writes it, the scanner should either accept it OR fail loudly — never silently drop.
- [ ] `false_verified` not surfaced because citation dedup collapsed the only real citation with a self-citation from a tempdir-escaped test fixture.
- [ ] Plural form emitting citations with leading/trailing whitespace (`A ,  B, C `) — the `.trim()` on each piece should prevent this. Re-confirm when refactoring the split loop.
- [ ] `catalog_coverage_check check` missing a scan directory after a new crate is added — the `test_dirs` argument list is maintained in the binary, not auto-discovered.
- [ ] `spec-coverage-report` per-stack count dropping without a corresponding catalog row status change (indicates a citation that used to parse no longer does).

---

## §6 — Improvement Log

### Open items

- [ ] [p2] **spec_chain `EffectExpectation` field-level matching.** Current `observe_effect()` in `crates/oriterm_test_support/src/spec_chain/observers/effect.rs` matches effects by top-level family (`"Host"`, `"HostRequest"`, `"Pty"`, `"Ui"`, `"Presentation"`) + optional sub-variant name (currently only `PtyWriteKind` sub-names). It cannot assert on variant fields like `HostEffect::ClipboardStore { selection: Clipboard, data: "Hello" }` — §10.2 worked around this by iterating `outcome.effects_emitted` directly via local `expect_clipboard_store()` / `expect_clipboard_load()` helpers in `oriterm_core/tests/spec_chain/osc/clipboard.rs`. When §10.3 lands a similar pattern for `HostEffect::DesktopNotification { source, title, body }`, this becomes a 2nd concrete consumer and the abstraction should be drawn. Candidate designs: (a) extend `EffectExpectation` with a predicate closure, (b) introduce a `HostEffectExpectation` enum with per-variant fields, (c) keep local helpers in each test module (SSOT = each catalog family owns its own helper). Decide at §10.3 kickoff, not earlier — avoid premature abstraction.  _Source: §10.2 retrospective 2026-04-19._

- [ ] [p3] **Consider accepting `Catalog-row:` (hyphen) and `CatalogRow:` (camelCase) variants?** The silent-miss fix only covers the `row` vs `rows` count-agreement variant. Other plausible typos exist. Deferred because no real case has surfaced yet — speculative work per §No Premature Abstraction.

### Recently closed

- [x] **Add `--explain <file>` diagnostic flag to `spec-coverage-report`** (2026-04-21, pending commit). Walks one file's citation lines and prints per-piece normalizer trace (step-by-step intermediate values + accepted-or-dropped outcome + drop-reason hint). Also added `.claude/rules/tests.md` §Test Hygiene item 7 pointing authors at the diagnostic when `FALSE VERIFIED` surfaces on a test file with `/// Catalog row:` citations. Verified by running `--explain` on `oriterm_core/tests/spec_chain/kitty/actions.rs` — correctly identifies accepted citations AND the em-dash-contaminated piece that would otherwise silently drop. _Source: §13.1 retrospective 2026-04-21._

- [x] **Accept plural `Catalog rows:` form** (2026-04-19, pending commit). Matrix-pinned across all three comment prefixes. Verified end-to-end by reverting §10.2 clipboard.rs doc to the plural form and confirming `catalog_coverage_check: OK`. _Source: §10.2 retrospective 2026-04-19._

---

## §7 — How To Use This File In Future Sessions

**When opening this file:** you are probably investigating a silent coverage drift, a scanner that dropped a citation you expected it to catch, or a `catalog_coverage_check` gate that fired unexpectedly.

**Start with §5 Regressions To Watch For.** If your symptom matches any of those lines, the regression has recurred — the fix is in §4's history. Bump the `- [ ]` line to `- [x]` after re-landing the fix.

**If your symptom is new:** check §1 Design Philosophy first. If your proposed fix violates a numbered item in §1, the fix is wrong — find a different approach that preserves the philosophy. Then write the fix, add a new `- [x]` entry under §6 "Recently closed" with date + commit sha + one-line description, and add a new §4 dated entry if the finding is substantive (more than a typo fix).

**When adding a new citation prefix form:** update BOTH `scan.rs` module doc AND the `prefixes` array AND `tests.rs`. Invariant table row 1 covers this sync.

**When adding a new effect-matching capability to `observers/effect.rs`:** close the open item in §6 above. Decide the abstraction shape based on the concrete consumer count (2+ required, per §No Premature Abstraction).
