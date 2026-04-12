# Catalog Coverage Check Fixtures

Read-only fixture files used by `catalog::tests` in
`crates/oriterm_test_support/src/catalog/tests.rs`.

These files are NOT part of any `#[test]` module on their own — they
are shared fixture inputs for the sibling tests in the `catalog`
module. They exist primarily as golden inputs documented in
`plans/spec-conformance/section-01-catalog-bootstrap.md §01.3.b`.

Most active test cases build their catalog inputs inline via the
`row_markdown` + `make_fixture_catalog` helpers in `tests.rs` —
those helpers give each test case its own `TempDir` so no two
tests share mutable state. The files here serve as reference
samples for manual inspection and for the deliberate-injection
walkthroughs in `§01.3.d`:

- `catalog_golden.md` — one well-formed row of every column
  status. A baseline for comparing new fixtures.
- `catalog_stale_symbol.md` — a row with `TermHandler::nonexistent_method`;
  paired with a walkthrough where the check would fail against a
  real `syn` AST walk.
- `catalog_duplicate_id.md` — two rows sharing the same `ID`; the
  `check_rejects_duplicate_row_id` sibling test uses an inline copy.
- `catalog_line_number_primary.md` — the banned
  `file.rs:91 → Symbol` form.
- `catalog_wezterm_spec_source.md` — the banned
  `Spec source: wezterm ...` form (Phase 2 Finding J).
- `catalog_verified_status.md` — a row with `Verification: verified`,
  used to walk through `--bootstrap-mode` rejection.
