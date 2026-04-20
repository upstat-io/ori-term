---
section: "09A"
title: "DEC Private CSI Extensions (rect ops + presentation + audits/ SSOT)"
status: in-progress
reviewed: true
goal: "This section exists because the DECRQCRA gap — and the entire DEC private rectangular-ops family (DECCRA, DECFRA, DECERA, DECSERA, DECRARA, DECCARA, DECSACE, XTCHECKSUM, XTREPORTSGR) plus the presentation/column ops (DECIC, DECDC, DECBI, DECFI, DECRQPSR, DECRQUPSS, DECRQDE, DECSCL, DECSCA, DECSASD, DECSSDT) and the DCS-path presentation queries (DECRQSS, DECRSPS) — survived undetected from the initial catalog bootstrap (Section 01) into production. Section 01's bottom-up harvest audited the existing dispatch table and augmented with tack/teseq-discovered items; it did not walk the canonical spec source row-by-row. The `Section 04.9 UncatalogedDetector` only catches sequences observed at harness time — sequences absent from both the catalog AND the test corpus are invisible. Section 09A closes this systemic gap by: (1) CONFIRMING `plans/spec-conformance/audits/` as the new SSOT for top-down coverage enforcement (the directory, README.md, AND section-11 through section-26 stub files already exist, committed by the pre-implementation planning pass); (2) RECONCILING the two catalog files that already exist on disk — `catalog/dec-rectangle-ops.md` (10 DECRECT rows; CURRENTLY in per-row Field|Value block form which fails the 10-column `parse_catalog_markdown` schema — §09A.1 REWRITES to 10-col table) and `catalog/dec-presentation.md` (13 DECPRES rows; already in correct 10-col table form — §09A.2 VERIFIES parse); (3) adding ALL missing CSI dispatch arms in `crates/vte/src/ansi/dispatch/csi.rs`, default Handler trait methods in `crates/vte/src/ansi/handler.rs`, and concrete override methods in `oriterm_core/src/term/handler/`; (4) implementing DECRQCRA checksum synchronously via `PtyEffect::Write` directly from the VTE handler (NOT via the `HostRequest` async round-trip pipeline — DECRQCRA has all the data it needs at dispatch time: grid snapshot, rectangular coordinates, checksum-algorithm selection; no external resource is required, so the ResponseToken pattern would be pure overhead and architectural mismatch); algorithm is xterm sum-then-negate (pinned against `~/projects/reference_repos/console_repos/xterm/screen.c:3136`), NOT CRC-16 or XOR-fold; (5) implementing all six rectangular-area mutation ops (DECCRA, DECFRA, DECERA, DECSERA, DECRARA, DECCARA) with DECLRMM-aware coordinate clamping, delegating row mutation into `oriterm_core/src/grid/editing/rect.rs` to preserve grid invariants (selection_dirty, wide-char cleanup); (6) implementing column insert/delete (DECIC, DECDC via CSI path) and back/forward index (DECBI, DECFI via `dispatch/mod.rs::esc_dispatch` — no standalone `esc.rs` file exists); (7) implementing presentation query stubs (DECRQPSR, DECRQUPSS, DECRQDE, DECSCL, DECSCA, DECSASD, DECSSDT via CSI path; DECRQSS and DECRSPS EXTEND the existing DCS handler in `dispatch/mod.rs::dispatch_hook`/`dispatch_unhook` — `oriterm_core/src/term/handler/tests/dcs.rs:94-168` already exercises DECRQSS end-to-end, this subsection enumerates and fills the gap vs current coverage); (8) wiring the `spec-coverage-report --check audit-files` lint into the existing binary at `crates/oriterm_test_support/src/bin/spec_coverage_report.rs` AND into `.github/workflows/ci.yml` (§09A.13 — CI enforcement is not deferred to Section 23); (9) VERIFYING (not rewriting) that sections 11-26 carry the pre-landed top-down coverage success criterion + §NN.0 subsection + audit file stub — avoiding a re-review cascade on `reviewed: true` sections like Section 11; (10) VERIFYING (not mutating) that DRIFT locations in `coverage-baseline.toml` and `00-overview.md` already carry the new stack entries; (11) APPENDING 4 MOUSE-prefixed rows (DECEFR/DECELR/DECSLE/DECRQLP) to `catalog/mouse.md` as gate rows so `audits/section-09a-top-down-inventory.md` can cite them as `mapped` (verification remains Section 16's work). The reframe: esctest is a SPEC SOURCE for top-down enumeration (the 383 failing tests it surfaces identify sequences ori_term does not dispatch) — it is NOT a runtime CI dependency. The new section absorbs esctest's coverage enumeration into our `spec_chain` harness, same shape as Section 02 absorbed tack."
success_criteria:
  - "Every row in `plans/spec-conformance/catalog/dec-rectangle-ops.md` (10 rows: DECRECT-DECSACE, DECRECT-DECCARA, DECRECT-DECRARA, DECRECT-DECCRA, DECRECT-DECFRA, DECRECT-XTCHECKSUM, DECRECT-DECRQCRA, DECRECT-DECERA, DECRECT-DECSERA, DECRECT-XTREPORTSGR) reaches `verified` or `verified-with-deviation` status"
  - "Every row in `plans/spec-conformance/catalog/dec-presentation.md` (~13 rows: DECPRES-DECIC, DECPRES-DECDC, DECPRES-DECBI, DECPRES-DECFI, DECPRES-DECRQPSR, DECPRES-DECRQUPSS, DECPRES-DECRQDE, DECPRES-DECSCL, DECPRES-DECSCA, DECPRES-DECSASD, DECPRES-DECSSDT, DECPRES-DECRQSS, DECPRES-DECRSPS) reaches `verified` or `verified-with-deviation` status (DCS-path rows DECRQSS and DECRSPS are `verified-with-deviation` if the DCS dispatcher stub routes but does not fully implement the response format — deviation must be documented)"
  - "`plans/spec-conformance/audits/` directory contains a valid audit file for Section 09A itself (`audits/section-09a-top-down-inventory.md`) plus stub audit files for every not-started section 11-26 (16 additional files, 17 total including 09A's own) — every stub file parses cleanly per the audit-file schema in `plans/spec-conformance/audits/README.md`"
  - "`cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` is implemented and passes for every committed audit file — each mapped row ID resolves to a real catalog row in some `catalog/*.md` file; every not-targeted row carries a non-empty one-line rationale"
  - "DECRQCRA checksum matches the xterm patch-336 algorithm: attribute-selection bitmask governs which cell attributes are folded into the checksum; DCS reply format is `DCS Pi ! ~ <hex-checksum> ST` matching xterm's `Pt !~ <hex> ST` response shape; coordinate clamping matches xterm 'clamped to physical buffer' semantics (top/bottom/left/right clamped independently, margins respected if DECLRMM active)"
  - "esctest baseline: after Section 09A lands, the count of esctest tests that fail due to unrecognized/missing dispatch (currently ~383) drops to fewer than 50 remaining failures; the remaining failures are second-order bugs that Section 09A surfaces but does not fix — each is filed via `/add-bug` as a concrete bug tracker entry before §09A.N is checked off"
  - "Every dispatch arm added in `crates/vte/src/ansi/dispatch/csi.rs` has a corresponding sibling test in `crates/vte/src/ansi/tests.rs` (matrix: parse-param-extraction + dispatch-routing + unhandled-negative-pin) per `.claude/rules/tests.md`"
  - "DECRQCRA checksum computation does NOT allocate per-cell — alloc regression test in `oriterm_core/tests/alloc_regression.rs` guards this invariant (performance pin: zero allocations in the checksum inner loop)"
  - "DRIFT updates landed in full: `coverage-baseline.toml` gains `dec-rectangle-ops = 0` and `dec-presentation = 0` entries; `00-overview.md` Catalog Files table lists both new catalog files; `00-overview.md` Catalog Row Schema ID column description lists `DECRECT` and `DECPRES` prefixes; every cross-reference in the plan that counted catalog files, ID prefix families, or total rows is updated with the new counts"
  - "Sections 11-26 each carry (PRE-LANDED baseline, verified by §09A.10): a top-down coverage success criterion in their `success_criteria` frontmatter array; a `§NN.0 Top-down audit file` checklist item as the FIRST subsection in their sections list; a committed `audits/section-NN-top-down-inventory.md` stub file with frontmatter `canonical_spec_sources`, `last_walked`, and `walked_by` fields. §09A.10 VERIFIES this baseline holds; it does NOT re-author the content (avoiding re-review cascade on `reviewed: true` sections)"
  - "`spec-coverage-report --check` AND `spec-coverage-report --check audit-files` are wired into `.github/workflows/ci.yml` as required CI steps (Section 09A.13 — the lint is useless unless CI runs it on every PR)"
  - "`catalog/mouse.md` carries 4 new rows for MOUSE-DECEFR / MOUSE-DECELR / MOUSE-DECSLE / MOUSE-DECRQLP (gate rows added by §09A.12 so `audits/section-09a-top-down-inventory.md` can cite them as `mapped`; verification remains Section 16's work)"
  - "DECIC/DECDC column operations respect DECLRMM margins when mode 69 is active; DECBI/DECFI index operations at the left/right margins behave correctly per xterm: DECBI at column 0 inserts a blank column at left margin (scrolls right); DECFI at rightmost column inserts a blank column at right margin (scrolls left)"
  - "Rectangular area ops (DECCRA, DECFRA, DECERA, DECSERA, DECRARA, DECCARA) all clamp page/source/destination coordinates to [1, rows] × [1, cols] before any grid mutation — out-of-range coordinates are clamped, not rejected; zero-area rectangles (top > bottom or left > right after clamping) are silently no-ops"
  - "`./build-all.sh` (debug + release + Windows cross-compile via `cargo build --target x86_64-pc-windows-gnu`) green; `./test-all.sh` green (debug workspace sweep); `./clippy-all.sh` green (zero new warnings under `deny(clippy::all)` + nursery)"
  - "Section's mission-criterion connection: contributes to **Catalog complete (top-down enforced)** (the audits/ SSOT + lint enforce top-down completeness; 23 new rows added across two new catalog files) AND **Verification chain complete per row** (every applicable DECRECT and DECPRES row reaches `verified` with parser → dispatch → state/effect apex green)"
inspired_by:
  - "xterm `ctlseqs.txt` — primary canonical source for DEC private CSI intermediates"
  - "DEC STD 070 §6 / VT420 Programming Reference Manual — original DEC spec for rectangular area ops"
  - "wezterm `docs/escape-sequences.md` — de-facto implementation reference for DEC rect ops"
  - "ThomasDickey/esctest2 — top-down conformance suite that surfaced this gap (esctest is a SPEC SOURCE for top-down enumeration; never a runtime CI dependency)"
depends_on: ["04"]
third_party_review:
  status: resolved
  updated: 2026-04-19
  notes: "user-accepted at iter_cap_reached after 3 rounds; 15 findings fixed inline (0 remain as - [ ] items). Non-convergent cadence (6→3→6) — residual drift risk accepted; §09A.N completion gates own implementation-time verification."
sections:
  - id: "09A.0"
    title: "Audits/ directory SSOT — bootstrap, README, lint contract, audit-files mode in spec-coverage-report"
    status: complete
  - id: "09A.1"
    title: "DEC rectangle ops catalog — rewrite catalog/dec-rectangle-ops.md from per-row Field|Value blocks to the 10-column table schema (10 rows)"
    status: complete
  - id: "09A.2"
    title: "DEC presentation ops catalog — verify catalog/dec-presentation.md parses cleanly via parse_catalog_markdown (13 rows, already in 10-col table form)"
    status: complete
  - id: "09A.3"
    title: "VTE dispatch arms — add ALL missing CSI dispatch arms in crates/vte/src/ansi/dispatch/csi.rs for the new rows"
    status: complete
  - id: "09A.4"
    title: "Handler trait methods — add default impls in crates/vte/src/ansi/handler.rs; override in oriterm_core/src/term/handler/"
    status: not-started
  - id: "09A.5"
    title: "DECRQCRA implementation — synchronous checksum from grid state; emit DCS Pid !~ <hex> ST via PtyEffect::Write directly"
    status: not-started
  - id: "09A.6"
    title: "DECCRA / DECFRA / DECERA / DECSERA / DECRARA / DECCARA — rectangular area ops with DECLRMM-aware coordinate clamping"
    status: not-started
  - id: "09A.7"
    title: "DECIC / DECDC / DECBI / DECFI — column operations + ESC-path back/forward index"
    status: not-started
  - id: "09A.8"
    title: "Presentation queries — DECRQPSR, DECRQUPSS, DECRQDE, DECSCL, DECSCA, DECSASD, DECSSDT (CSI path)"
    status: not-started
  - id: "09A.9"
    title: "DCS-path presentation queries — DECRQSS / DECRSPS dispatch + reply formatting"
    status: not-started
  - id: "09A.10"
    title: "Verify sections 11-26 top-down audit wiring (pre-landed by planning pass; verification-only to avoid reviewed:true re-review cascade)"
    status: not-started
  - id: "09A.11"
    title: "DRIFT verification — coverage-baseline.toml + 00-overview.md catalog table + ID prefix already carry new entries (verify, don't mutate)"
    status: not-started
  - id: "09A.12"
    title: "Section 16 locator extensions — add MOUSE-DECEFR/MOUSE-DECELR/MOUSE-DECSLE/MOUSE-DECRQLP rows to catalog/mouse.md"
    status: complete
  - id: "09A.13"
    title: "CI wiring — add `spec-coverage-report --check audit-files` to .github/workflows/ci.yml"
    status: not-started
  - id: "09A.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "09A.N"
    title: "Completion Checklist"
    status: in-progress
# TPR checkpoints:
# Checkpoint 1 — after 09A.0 + 09A.1 + 09A.2 (audits/ SSOT + both catalog files populated).
#   Covers the catalog schema for DECRECT/DECPRES + the lint contract for audit files.
#   Catches row schema drift and audit-file format errors before the implementation subsections
#   build on those definitions.
# Checkpoint 2 — after 09A.5 (DECRQCRA checksum implementation + PtyEffect::Write emission).
#   Covers the most architecturally sensitive decision: synchronous vs. async emission path.
#   Reviewers verify: (a) no HostRequest variant added, (b) checksum algorithm correctness vs
#   xterm patch-336, (c) no per-cell allocation, (d) coordinate clamping behavior.
# Checkpoint 3 — after 09A.7 (all rect ops + column ops + ESC-path index ops complete).
#   Covers the six rectangular area mutation ops + DECIC/DECDC + DECBI/DECFI.
#   Checks DECLRMM-aware margin clamping, zero-area rectangle no-op behavior, and the
#   DECBI/DECFI scroll-direction semantics.
# Final — 09A.N (full section TPR + impl-hygiene: all 23 rows verified, audits/ lint clean
#   AND wired into CI, sections 11-26 verified baseline holds, DRIFT entries confirmed,
#   catalog/mouse.md 4 gate rows appended).
---

# Section 09A: DEC Private CSI Extensions (rect ops + presentation + audits/ SSOT)

**Status:** In progress. Complete: 09A.0 (audits/ SSOT + lint), 09A.1 (dec-rectangle-ops.md 10-col rewrite), 09A.2 (dec-presentation.md 13-row catalog parse-verified), 09A.3 (19 CSI dispatch arms + parse/negative-pin tests; SGR helpers extracted to dispatch/csi/sgr.rs to keep mod.rs under 500 lines; handler.rs trait defaults bundled here so the dispatch arms compile), 09A.12 (catalog/mouse.md locator gate rows). Remaining: 09A.4–09A.11, 09A.13, 09A.R, 09A.N.

---

## Goal

Drive all 10 rows in `plans/spec-conformance/catalog/dec-rectangle-ops.md` and all ~13 rows in `plans/spec-conformance/catalog/dec-presentation.md` from `missing` to `verified`, introduce the `plans/spec-conformance/audits/` SSOT for top-down coverage enforcement, wire the `spec-coverage-report --check audit-files` lint into the existing binary, perform the verbiage rewrite of sections 11-26, and update every DRIFT location. See frontmatter `goal:` for the full context and architectural decisions.

---

## Architecture Overview

### Why the gap existed

Section 01's catalog bootstrap was bottom-up: walk the existing `crates/vte/src/ansi/dispatch/csi.rs` dispatch table, augment with sequences observed during tack/teseq test runs, and add anything esctest surfaced. This approach is accurate for sequences already in the dispatch table — it found every CSI sequence ori_term already handled. But it is blind to sequences that have NO dispatch arm at all, because those sequences produce no observable events and therefore appear in no test output.

The DEC private rectangular-area family uses uncommon CSI intermediates (`$`, `*`, `#`, `'`) that are absent from the existing dispatch table. They produced no observable behavior, no tack/teseq events, and no esctest captures routed through spec_chain. They were structurally invisible to the bottom-up approach.

The `Section 04.9 UncatalogedDetector` (at `crates/oriterm_test_support/src/spec_chain/uncataloged/mod.rs:22-88`) runs during every spec_chain test and accumulates TupleSigs for sequences observed at harness time. It is a SECONDARY safety net — it catches sequences that APPEAR in test input but lack catalog rows. It cannot catch sequences absent from both the catalog AND the test corpus.

### audits/ SSOT design

`plans/spec-conformance/audits/` is the NEW PRIMARY gate for top-down catalog completeness. Each not-started spec-conformance section commits a per-section audit file at `audits/section-NN-top-down-inventory.md`. The file walks the section's canonical spec source(s) row-by-row and maps every sequence to a catalog row ID or an explicit `not-targeted` decision.

The lint contract (from `plans/spec-conformance/audits/README.md`) enforces:

1. **Existence** — every `in-progress` section in the Quick Reference table has a corresponding audit file, per `plans/spec-conformance/audits/README.md:59`. `not-started` sections are exempted until §NN.0 execution time; `complete` sections have their audit file permanently committed. Integration sections (21, 22, 24, 25) still get the existence check when they reach `in-progress` — their audit file uses `canonical_spec_sources: []` with a body comment so the mapping-resolution check no-ops.
2. **Mapping resolution** — every `Decision: mapped` row cites a catalog row ID that exists in some `catalog/*.md` file. A mapping to a non-existent row ID fails the lint.
3. **Schema conformance** — every audit file frontmatter parses; every row has all 4 columns; every `not-targeted` row has a non-empty rationale.
4. **Freshness** — `last_walked` is present and parses as YYYY-MM-DD. CI does not gate on staleness — that is a `/review-bugs` triage check.

`spec-coverage-report --check audit-files` is the new flag added to `crates/oriterm_test_support/src/bin/spec_coverage_report.rs`. The existing binary already handles `--check` for false-verified rows, uncataloged citations, and regression-below-baseline. The `audit-files` subcommand is a separate gate that runs the four lint checks above.

The `UncatalogedDetector` REMAINS in place as a secondary/runtime catch. The relationship:

- audits/ (primary): top-down, committed artifacts, walks spec before implementation
- UncatalogedDetector (secondary): runtime, catches sequences appearing in test input that lack catalog rows

Both gates run in CI. Neither replaces the other.

### DECRQCRA: synchronous PtyEffect::Write (NOT HostRequest)

**Decision rationale:** DECRQCRA (`CSI Pi;Pg;Pt;Pl;Pb;Pr * y`) requests a checksum of a rectangular area of the grid. The terminal has ALL the data it needs at dispatch time: the grid snapshot, the rectangular coordinates, and the checksum-algorithm selection (Pg param, bitmask from XTCHECKSUM). No external resource is required.

Compare to the existing DA3 response in `oriterm_core/src/term/handler/status.rs:160-168` — DA3 emits `"\x1bP!|00000000\x1b\\"` directly via:

```rust
self.effect_sink.push(Effect::Pty(PtyEffect::Write {
    bytes: response.as_bytes().to_vec(),
    kind: PtyWriteKind::DeviceAttribute,
}));
```

DECRQCRA follows the SAME pattern. The checksum is computed synchronously from the grid, formatted as `DCS Pi !~ XXXX ST` where XXXX is the 4-digit hex checksum, then emitted via `PtyEffect::Write { kind: PtyWriteKind::ChecksumReport }`. No `HostRequest` variant is added; no `ResponseToken` is needed; the `response_poll` pipeline is not involved.

The `HostRequest` async pipeline (`oriterm_core/src/effect/families/host_request/mod.rs:38-63`) is for operations that require EXTERNAL data: clipboard reads (the OS clipboard is not accessible synchronously from the VTE handler), color queries (the UI palette is not accessible inside Term's VTE handler without a round-trip). DECRQCRA does not need external data.

**Anti-pattern to avoid:** Do NOT extend `HostRequest` with a `ChecksumRequest` variant. Do NOT add `DECRQCRA` to the `response_poll` machinery in `oriterm_mux/src/pane/io_thread/response_poll/mod.rs`. These would introduce an asynchronous round-trip where a synchronous computation suffices.

### Dispatch arm + handler method + spec_chain test pattern (mirrors Section 10)

For each new sequence, three artifacts land together in the same commit:

1. **Dispatch arm** in `crates/vte/src/ansi/dispatch/csi.rs` — pattern match `(final_byte, intermediates)` → handler method call
2. **Handler trait default** in `crates/vte/src/ansi/handler.rs` — no-op default; only `Term` (in `oriterm_core`) provides the real impl
3. **Handler override** in `oriterm_core/src/term/handler/` — real state mutation or PtyEffect emission

Tests at each rung:
- **Parser rung** (`crates/vte/src/ansi/tests.rs`): feed raw bytes, assert param extraction + dispatch arm fires
- **Dispatch rung** (same file): assert correct handler method called with correct args using `RecordingHandler`
- **State/Effect rung** (`oriterm_core/tests/spec_chain/<stack>/`): use `SpecHarness::feed()` with `observe_state` / `observe_effect` to assert the apex

The matrix per dispatch arm: (1) canonical params, (2) default params (zero → use-default behavior), (3) out-of-range params (clamped silently), (4) unhandled variant → `unhandled!()` negative pin.

---

## §09A.0 — Audits/ directory SSOT

### Goal

The `plans/spec-conformance/audits/` directory and its `README.md` already exist (verified). §09A.0 implements the `--check audit-files` flag in `spec_coverage_report.rs`, creates Section 09A's own audit file (`audits/section-09a-top-down-inventory.md`), and verifies the lint passes clean for that one file before §09A.1–09A.12 proceed.

### Binary choice (unified `spec-coverage-report` vs separate `audit-files-lint`)

`--check audit-files` is added to the EXISTING binary at `crates/oriterm_test_support/src/bin/spec_coverage_report.rs`. The separation-of-concerns alternative (a standalone `audit-files-lint` binary) was considered and rejected: both tools parse the same catalog files + audit files, both resolve row IDs against the same catalog signature set, and `spec_coverage_report` already loads these inputs at startup. A separate binary would duplicate the input-loading path and create a second place to keep catalog/audit parsing in sync. The unified binary keeps SSOT on the catalog load logic and matches the existing `--check` subcommand pattern (already used for false-verified rows, uncataloged citations, regression-below-baseline).

### Files touched

- `crates/oriterm_test_support/src/bin/spec_coverage_report.rs` — add `--check audit-files` subcommand
- `crates/oriterm_test_support/src/spec_chain/coverage/` — add `AuditFilesChecker` module (new file `audit_files.rs`) implementing the four lint checks; also register it in `mod.rs`
- `plans/spec-conformance/audits/section-09a-top-down-inventory.md` — Section 09A's own audit file (created and populated)

### Implementation notes

The existing binary at `crates/oriterm_test_support/src/bin/spec_coverage_report.rs:50` already handles `--check` via:

```rust
if std::env::args().any(|a| a == "--check") { ... }
```

The `audit-files` mode is added as an ADDITIONAL check that runs when `--check audit-files` is in args. The four lint checks mirror the contract in `plans/spec-conformance/audits/README.md §Lint contract`:

1. **Existence check**: filter Quick Reference table rows in `00-overview.md` (parse `| NN |` rows) by `status` — only `in-progress` sections are required to have a corresponding audit file per `plans/spec-conformance/audits/README.md:59`; `not-started` sections are exempted until §NN.0 execution time and `complete` sections have their audit file permanently committed. Integration sections (21, 22, 24, 25) still get the existence check when they reach `in-progress` — their audit file uses `canonical_spec_sources: []` with a body comment so the mapping-resolution check no-ops. This matches the README's lint contract verbatim; do not widen it.
2. **Mapping resolution**: for every audit file row with `Decision: mapped`, parse the catalog row ID from the 3rd column and confirm it resolves against the catalog signature set (re-use `build_catalog_signature_set()` already in `oriterm_test_support::catalog`).
3. **Schema conformance**: frontmatter parses (YAML), all 4 table columns present, `not-targeted` rows have non-empty rationale after the colon.
4. **Freshness**: `last_walked` field is present and parses as YYYY-MM-DD. Staleness is NOT a lint failure.

The `AuditFilesChecker` type lives in a new file `crates/oriterm_test_support/src/spec_chain/coverage/audit_files.rs` so the main binary stays under 500 lines (code-hygiene.md §File Size).

### §09A.0 audit file content

`audits/section-09a-top-down-inventory.md` enumerates every sequence in `xterm ctlseqs.txt` under the DEC private CSI intermediates (`$`, `*`, `#`, `'`) plus the ESC-6/ESC-9 sequences (DECBI/DECFI), maps each to a DECRECT or DECPRES catalog row ID, and gives `not-targeted` decisions for any sequences omitted (e.g. DECCIR, DECRQLP — the latter belongs to Section 16 / mouse.md).

### Test requirements

- Unit tests for `AuditFilesChecker`: pass with a minimal valid audit file, fail with a mapping to a non-existent row ID, fail with a missing `not-targeted` rationale, fail with a missing required section (matrix: all 4 lint checks)
- Negative pin: feeding a malformed audit file YAML frontmatter produces a lint failure, not a panic

### Completion criteria

- `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` exits 0 with `audits/section-09a-top-down-inventory.md` in scope
- All 4 lint checks have unit tests in `crates/oriterm_test_support/src/spec_chain/coverage/tests.rs`

---

## §09A.1 — DEC rectangle ops catalog

### Goal

`plans/spec-conformance/catalog/dec-rectangle-ops.md` already exists at commit time but is in the per-row `Field | Value` BLOCK form. The canonical on-disk schema is the **10-column markdown table** per `plans/spec-conformance/catalog/README.md §Frozen Schema Reference` + `crates/oriterm_test_support/src/catalog/row.rs` (CATALOG_COLUMN_COUNT = 10, strict `parse_catalog_markdown`). §09A.1 REWRITES the file to the 10-column table form so the parser accepts it — the per-row block form is zero parseable rows. All 10 rows start at `missing` verification — implementation follows in §09A.3–09A.6.

**SSOT citation:** every catalog file on disk is a single 10-column markdown table matching `CATALOG_COLUMNS = ["ID", "Spec source", "Sequence", "Description", "Implementation", "Apex layer", "Test chain", "Verification", "De-facto ref", "Notes"]` (see `ecma-48.md`, `mouse.md`, `dec-presentation.md` for canonical examples). The per-row `Field | Value` form used in `00-overview.md §Canonical example row` is a DOCUMENTATION PRESENTATION only — it does not match what `parse_catalog_markdown` accepts. Do NOT author new catalog content in Field|Value blocks.

### Files touched

- `plans/spec-conformance/catalog/dec-rectangle-ops.md` (rewrite existing file to 10-col table form)

### Row table (10-column markdown table format — verbatim authoritative content)

The rewritten `catalog/dec-rectangle-ops.md` body contains one table of 10 rows (after the frontmatter + prose header block). Column order MUST match `CATALOG_COLUMNS` exactly. Each row below is a single line in the rewritten file (wrapped here for readability):

```markdown
| ID | Spec source | Sequence | Description | Implementation | Apex layer | Test chain | Verification | De-facto ref | Notes |
|---|---|---|---|---|---|---|---|---|---|
| `DECRECT-DECSACE` | xterm ctlseqs.txt `CSI Ps * x` | `` `CSI Ps * x` `` | Select attribute change extent; Ps=0 stream, Ps=1 rect | MISSING — to be added by Section 09A | effect-mode-state | parser:pending dispatch:pending state:pending | missing | — | Param stored as `ace_mode` on Term; consumed by DECCARA/DECRARA |
| `DECRECT-DECCARA` | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr;Pm $ r` | `` `CSI Pt;Pl;Pb;Pr;Pm $ r` `` | Change attributes in rectangular area | MISSING — to be added by Section 09A | state-snapshot | parser:pending dispatch:pending state:pending snapshot:pending | missing | wezterm docs/escape-sequences.md DECCARA | DECLRMM-aware; DECSACE mode governs stream vs rect extent |
| `DECRECT-DECRARA` | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr;Pm $ t` | `` `CSI Pt;Pl;Pb;Pr;Pm $ t` `` | Reverse attributes in rectangular area | MISSING — to be added by Section 09A | state-snapshot | parser:pending dispatch:pending state:pending snapshot:pending | missing | — | Reversal applies only to SGR attributes listed in Pm params |
| `DECRECT-DECCRA` | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr;Pp;Pt;Pl;Pp $ v` | `` `CSI Pt;Pl;Pb;Pr;Pp;Pt;Pl;Pp $ v` `` | Copy rectangular area from source page to destination page | MISSING — to be added by Section 09A | state-snapshot | parser:pending dispatch:pending state:pending snapshot:pending | missing | wezterm docs/escape-sequences.md DECCRA | Copy-before-overwrite semantics for overlapping regions |
| `DECRECT-DECFRA` | xterm ctlseqs.txt `CSI Pc;Pt;Pl;Pb;Pr $ x` | `` `CSI Pc;Pt;Pl;Pb;Pr $ x` `` | Fill rectangular area with character Pc + current SGR | MISSING — to be added by Section 09A | state-snapshot | parser:pending dispatch:pending state:pending snapshot:pending | missing | wezterm docs/escape-sequences.md DECFRA | DECLRMM-aware; Pc is a character code point, not a string |
| `DECRECT-XTCHECKSUM` | xterm ctlseqs.txt `CSI Ps # y` | `` `CSI Ps # y` `` | Select checksum extension flags (xterm patch-336) | MISSING — to be added by Section 09A | effect-mode-state | parser:pending dispatch:pending state:pending | missing | xterm patch-336 | Ps is a bitmask; stored as `checksum_flags: u16` on Term; consumed by DECRQCRA handler |
| `DECRECT-DECRQCRA` | xterm ctlseqs.txt `CSI Pi;Pg;Pt;Pl;Pb;Pr * y` | `` `CSI Pi;Pg;Pt;Pl;Pb;Pr * y` `` | Request checksum of rectangular area; emits DCS reply | MISSING — to be added by Section 09A | effect-pty-write | parser:pending dispatch:pending effect:pending | missing | xterm patch-336 (algorithm); esctest2 DECRQCRA suite (clamping) | Reply `DCS Pi !~ XXXX ST` (4-hex checksum); synchronous emission via PtyEffect::Write (NOT HostRequest); algorithm = xterm sum-then-negate |
| `DECRECT-DECERA` | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr $ z` | `` `CSI Pt;Pl;Pb;Pr $ z` `` | Erase rectangular area (space + default attrs) | MISSING — to be added by Section 09A | state-snapshot | parser:pending dispatch:pending state:pending snapshot:pending | missing | — | DECLRMM-aware; respects DECSCA selective-erase protection attribute |
| `DECRECT-DECSERA` | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr $ {` | `` `CSI Pt;Pl;Pb;Pr $ {` `` | Selective erase rectangular area (skip DECSCA-protected cells) | MISSING — to be added by Section 09A | state-snapshot | parser:pending dispatch:pending state:pending snapshot:pending | missing | — | Companion to DECERA; only erases unprotected cells |
| `DECRECT-XTREPORTSGR` | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr # \|` | `` `CSI Pt;Pl;Pb;Pr # \|` `` | Report selected graphic rendition (xterm) | MISSING — to be added by Section 09A | effect-pty-write | parser:pending dispatch:pending effect:pending | missing | xterm patch-336 | DCS reply per-cell format; verified-with-deviation acceptable if only basic SGR attrs included |
```

### Legacy per-row blocks (reference only — not the on-disk format)

The per-row `Field | Value` blocks below are PRESERVED here ONLY as a per-row discussion/reference aid for implementers — they describe each row in more detail than the compact table. The on-disk format is the compact 10-column table above; the table is the authoritative row content. Copy the TABLE rows into the catalog file, not the blocks.

---

**DECRECT-DECSACE**

| Field | Value |
|---|---|
| **ID** | `DECRECT-DECSACE` |
| **Spec source** | xterm ctlseqs.txt `CSI Ps * x` |
| **Sequence** | `CSI Ps * x` — Select attribute change extent |
| **Description** | Controls which attributes are changed by DECCARA/DECRARA; Ps=0 stream, Ps=1 rect |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | effect-mode-state |
| **Test chain** | parser:pending dispatch:pending state:pending |
| **Verification** | missing |
| **De-facto ref** | — |
| **Notes** | Param stored as `ace_mode` on Term; consumed by DECCARA/DECRARA to determine change extent |

---

**DECRECT-DECCARA**

| Field | Value |
|---|---|
| **ID** | `DECRECT-DECCARA` |
| **Spec source** | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr;Pm $ r` |
| **Sequence** | `CSI Pt;Pl;Pb;Pr;Pm $ r` — Change attributes in rectangular area |
| **Description** | Applies SGR attribute change to cells in the specified rectangle |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | state-snapshot |
| **Test chain** | parser:pending dispatch:pending state:pending snapshot:pending |
| **Verification** | missing |
| **De-facto ref** | wezterm `docs/escape-sequences.md` DECCARA |
| **Notes** | DECLRMM-aware; DECSACE mode governs stream vs rect extent |

---

**DECRECT-DECRARA**

| Field | Value |
|---|---|
| **ID** | `DECRECT-DECRARA` |
| **Spec source** | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr;Pm $ t` |
| **Sequence** | `CSI Pt;Pl;Pb;Pr;Pm $ t` — Reverse attributes in rectangular area |
| **Description** | Reverses (toggles) video attributes in cells within the specified rectangle |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | state-snapshot |
| **Test chain** | parser:pending dispatch:pending state:pending snapshot:pending |
| **Verification** | missing |
| **De-facto ref** | — |
| **Notes** | Reversal applies only to SGR attributes listed in Pm params |

---

**DECRECT-DECCRA**

| Field | Value |
|---|---|
| **ID** | `DECRECT-DECCRA` |
| **Spec source** | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr;Pp;Pt;Pl;Pp $ v` |
| **Sequence** | `CSI Pt;Pl;Pb;Pr;Pp;Pt;Pl;Pp $ v` — Copy rectangular area |
| **Description** | Copies a rectangular area of cells from source page to destination page |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | state-snapshot |
| **Test chain** | parser:pending dispatch:pending state:pending snapshot:pending |
| **Verification** | missing |
| **De-facto ref** | wezterm `docs/escape-sequences.md` DECCRA |
| **Notes** | Source and destination pages; overlapping regions defined by copy-before-overwrite semantics |

---

**DECRECT-DECFRA**

| Field | Value |
|---|---|
| **ID** | `DECRECT-DECFRA` |
| **Spec source** | xterm ctlseqs.txt `CSI Pc;Pt;Pl;Pb;Pr $ x` |
| **Sequence** | `CSI Pc;Pt;Pl;Pb;Pr $ x` — Fill rectangular area |
| **Description** | Fills the specified rectangular area with character Pc and current SGR attributes |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | state-snapshot |
| **Test chain** | parser:pending dispatch:pending state:pending snapshot:pending |
| **Verification** | missing |
| **De-facto ref** | wezterm `docs/escape-sequences.md` DECFRA |
| **Notes** | DECLRMM-aware; Pc is a character code point, not a string |

---

**DECRECT-XTCHECKSUM**

| Field | Value |
|---|---|
| **ID** | `DECRECT-XTCHECKSUM` |
| **Spec source** | xterm ctlseqs.txt `CSI Ps # y` |
| **Sequence** | `CSI Ps # y` — Select checksum extension flags (xterm) |
| **Description** | Sets xterm checksum algorithm flags used by subsequent DECRQCRA requests |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | effect-mode-state |
| **Test chain** | parser:pending dispatch:pending state:pending |
| **Verification** | missing |
| **De-facto ref** | xterm patch-336 |
| **Notes** | Ps is a bitmask; stored as `checksum_flags: u16` on Term; consumed by DECRQCRA handler |

---

**DECRECT-DECRQCRA**

| Field | Value |
|---|---|
| **ID** | `DECRECT-DECRQCRA` |
| **Spec source** | xterm ctlseqs.txt `CSI Pi;Pg;Pt;Pl;Pb;Pr * y` |
| **Sequence** | `CSI Pi;Pg;Pt;Pl;Pb;Pr * y` — Request checksum of rectangular area |
| **Description** | Computes a checksum of the specified rectangular area and emits a DCS reply |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | effect-pty-write |
| **Test chain** | parser:pending dispatch:pending state:pending effect:pending |
| **Verification** | missing |
| **De-facto ref** | xterm patch-336 (algorithm); esctest2 `DECRQCRA` suite (coordinate clamping) |
| **Notes** | Reply format: `DCS Pi ! ~ XXXX ST` (4-hex-digit checksum); synchronous emission via PtyEffect::Write (NOT HostRequest); algorithm: xterm sum-then-negate of attribute-selected cell data per xterm/screen.c:3136 (see §09A.5) |

---

**DECRECT-DECERA**

| Field | Value |
|---|---|
| **ID** | `DECRECT-DECERA` |
| **Spec source** | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr $ z` |
| **Sequence** | `CSI Pt;Pl;Pb;Pr $ z` — Erase rectangular area |
| **Description** | Erases all characters in the specified rectangle (replaces with space, default attrs) |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | state-snapshot |
| **Test chain** | parser:pending dispatch:pending state:pending snapshot:pending |
| **Verification** | missing |
| **De-facto ref** | — |
| **Notes** | DECLRMM-aware; respects DECSCA selective-erase protection attribute |

---

**DECRECT-DECSERA**

| Field | Value |
|---|---|
| **ID** | `DECRECT-DECSERA` |
| **Spec source** | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr $ {` |
| **Sequence** | `` CSI Pt;Pl;Pb;Pr $ { `` — Selective erase rectangular area |
| **Description** | Erases unprotected characters in the specified rectangle (DECSCA-protected cells are skipped) |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | state-snapshot |
| **Test chain** | parser:pending dispatch:pending state:pending snapshot:pending |
| **Verification** | missing |
| **De-facto ref** | — |
| **Notes** | Companion to DECERA; only erases cells not marked with DECSCA protection |

---

**DECRECT-XTREPORTSGR**

| Field | Value |
|---|---|
| **ID** | `DECRECT-XTREPORTSGR` |
| **Spec source** | xterm ctlseqs.txt `CSI Pt;Pl;Pb;Pr # \|` |
| **Sequence** | `CSI Pt;Pl;Pb;Pr # |` — Report selected graphic rendition (xterm) |
| **Description** | Emits the SGR attributes for each cell in the rectangle as a DCS stream |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | effect-pty-write |
| **Test chain** | parser:pending dispatch:pending state:pending effect:pending |
| **Verification** | missing |
| **De-facto ref** | xterm patch-336 |
| **Notes** | DCS reply per-cell format; complex serialization; verified-with-deviation acceptable if only basic SGR attrs are included |

---

### Completion criteria

- `plans/spec-conformance/catalog/dec-rectangle-ops.md` has been rewritten to the 10-column markdown table form (single table with 10 rows); the per-row Field|Value blocks in the current version are replaced by the compact table above. The file parses successfully via `parse_catalog_markdown` (`crates/oriterm_test_support/src/catalog/parser/mod.rs`) — verified by a unit test added in the same commit, OR by `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check` returning without a `CatalogParseError::HeaderMismatch` / `ColumnCount` / `UnknownVerification` emission for this file
- `spec-coverage-report --check` passes (no false-verified rows — all 10 rows are `missing`)
- All 10 row IDs resolve correctly in `audits/section-09a-top-down-inventory.md` mapping table

---

## §09A.2 — DEC presentation ops catalog

### Goal

`plans/spec-conformance/catalog/dec-presentation.md` already exists and IS already in the correct 10-column markdown table form (verified against `crates/oriterm_test_support/src/catalog/row.rs CATALOG_COLUMNS`). §09A.2 VERIFIES the file parses cleanly via `parse_catalog_markdown` and resolves every row ID against the catalog signature set built by `build_catalog_signature_set()`. No file rewrite required under ordinary path. If verification surfaces any schema drift (column order, unknown `Verification` value, empty ID), the fix IS the work — patch the file in this subsection rather than deferring. Two rows (DECPRES-DECRQSS and DECPRES-DECRSPS) are DCS-path sequences — dispatch lives in the DCS handler in `crates/vte/src/ansi/dispatch/mod.rs` (`dispatch_hook` / `dispatch_unhook`), NOT `csi.rs`. The Notes field in each row must preserve this distinction after any edits.

### Files touched

- `plans/spec-conformance/catalog/dec-presentation.md` (verify; patch only if schema drift detected)

### Verification checks

- `parse_catalog_markdown(Path::new("plans/spec-conformance/catalog/dec-presentation.md"))` returns `Ok(Vec<Row>)` with exactly 13 rows
- Each row's `id` is non-empty, begins with `DECPRES-`, and is unique within the file
- `verification` for every row is `Verification::Missing` (bootstrap invariant — §09A.1/§09A.2 catalogs never bootstrap as verified)
- Row IDs DECPRES-DECRQSS and DECPRES-DECRSPS explicitly call out DCS-path dispatch in their Notes field

### Legacy per-row blocks (reference only — not the on-disk format)

The per-row `Field | Value` blocks below are PRESERVED here ONLY as an implementer's reference describing each row in more detail than the compact 10-column table. The on-disk catalog content is the 10-column table already in `dec-presentation.md`. Do not copy these blocks into the catalog file — they will fail `parse_catalog_markdown`.

### Row reference (verbose per-row blocks, reference only)

---

**DECPRES-DECIC**

| Field | Value |
|---|---|
| **ID** | `DECPRES-DECIC` |
| **Spec source** | xterm ctlseqs.txt `CSI Ps ' }` |
| **Sequence** | `CSI Ps ' }` — Insert column(s) |
| **Description** | Inserts Ps blank columns at cursor column, shifting existing columns right |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | state-snapshot |
| **Test chain** | parser:pending dispatch:pending state:pending snapshot:pending |
| **Verification** | missing |
| **De-facto ref** | — |
| **Notes** | DECLRMM-aware; only valid when mode 69 (DECLRMM) is set per xterm; no-op otherwise |

---

**DECPRES-DECDC**

| Field | Value |
|---|---|
| **ID** | `DECPRES-DECDC` |
| **Spec source** | xterm ctlseqs.txt `CSI Ps ' ~` |
| **Sequence** | `CSI Ps ' ~` — Delete column(s) |
| **Description** | Deletes Ps columns at cursor column, shifting remaining columns left |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | state-snapshot |
| **Test chain** | parser:pending dispatch:pending state:pending snapshot:pending |
| **Verification** | missing |
| **De-facto ref** | — |
| **Notes** | Companion to DECIC; DECLRMM-aware |

---

**DECPRES-DECBI**

| Field | Value |
|---|---|
| **ID** | `DECPRES-DECBI` |
| **Spec source** | xterm ctlseqs.txt `ESC 6` |
| **Sequence** | `ESC 6` — Back index |
| **Description** | If cursor is at left margin, inserts a blank column and scrolls right; otherwise moves cursor left |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | state-snapshot |
| **Test chain** | parser:pending dispatch:pending state:pending snapshot:pending |
| **Verification** | missing |
| **De-facto ref** | — |
| **Notes** | ESC path (not CSI); dispatch in `crates/vte/src/ansi/dispatch/esc.rs` or equivalent ESC handler |

---

**DECPRES-DECFI**

| Field | Value |
|---|---|
| **ID** | `DECPRES-DECFI` |
| **Spec source** | xterm ctlseqs.txt `ESC 9` |
| **Sequence** | `ESC 9` — Forward index |
| **Description** | If cursor is at right margin, inserts a blank column and scrolls left; otherwise moves cursor right |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | state-snapshot |
| **Test chain** | parser:pending dispatch:pending state:pending snapshot:pending |
| **Verification** | missing |
| **De-facto ref** | — |
| **Notes** | ESC path (not CSI); companion to DECBI |

---

**DECPRES-DECRQPSR**

| Field | Value |
|---|---|
| **ID** | `DECPRES-DECRQPSR` |
| **Spec source** | xterm ctlseqs.txt `CSI Ps $ w` |
| **Sequence** | `CSI Ps $ w` — Request presentation state report |
| **Description** | Requests a DCS presentation state report for cursor information (Ps=1) or tab stops (Ps=2) |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | effect-pty-write |
| **Test chain** | parser:pending dispatch:pending state:pending effect:pending |
| **Verification** | missing |
| **De-facto ref** | — |
| **Notes** | Reply is a DCS stream; complex serialization; stub reply acceptable for initial verification |

---

**DECPRES-DECRQUPSS**

| Field | Value |
|---|---|
| **ID** | `DECPRES-DECRQUPSS` |
| **Spec source** | xterm ctlseqs.txt `CSI & u` |
| **Sequence** | `CSI & u` — Request user-preferred supplemental set |
| **Description** | Requests the user-preferred supplemental character set identifier |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | effect-pty-write |
| **Test chain** | parser:pending dispatch:pending state:pending effect:pending |
| **Verification** | missing |
| **De-facto ref** | — |
| **Notes** | Reply format per DEC STD 070; constant reply (ISO Latin-1) acceptable for initial verification |

---

**DECPRES-DECRQDE**

| Field | Value |
|---|---|
| **ID** | `DECPRES-DECRQDE` |
| **Spec source** | xterm ctlseqs.txt `CSI " v` |
| **Sequence** | `CSI " v` — Request displayed extent |
| **Description** | Requests the current display extent (rows and columns) |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | effect-pty-write |
| **Test chain** | parser:pending dispatch:pending state:pending effect:pending |
| **Verification** | missing |
| **De-facto ref** | — |
| **Notes** | Reply: `CSI Pn;Pn " w` with current grid dimensions |

---

**DECPRES-DECSCL**

| Field | Value |
|---|---|
| **ID** | `DECPRES-DECSCL` |
| **Spec source** | xterm ctlseqs.txt `CSI Pl;Pc " p` |
| **Sequence** | `CSI Pl;Pc " p` — Set conformance level |
| **Description** | Sets the terminal's DEC conformance level (VT100/VT200/VT300) |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | effect-mode-state |
| **Test chain** | parser:pending dispatch:pending state:pending |
| **Verification** | missing |
| **De-facto ref** | — |
| **Notes** | Pl=1 VT100, Pl=2 VT200, Pl=3 VT300; Pc selects 7-bit or 8-bit C1 mode |

---

**DECPRES-DECSCA**

| Field | Value |
|---|---|
| **ID** | `DECPRES-DECSCA` |
| **Spec source** | xterm ctlseqs.txt `CSI Ps " q` |
| **Sequence** | `CSI Ps " q` — Select character protection attribute |
| **Description** | Sets selective-erase protection for subsequently written characters |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | effect-mode-state |
| **Test chain** | parser:pending dispatch:pending state:pending |
| **Verification** | missing |
| **De-facto ref** | — |
| **Notes** | Ps=0 or 2 unprotected; Ps=1 protected; flag stored per-cell in CellFlags; consumed by DECSERA/DECERA |

---

**DECPRES-DECSASD**

| Field | Value |
|---|---|
| **ID** | `DECPRES-DECSASD` |
| **Spec source** | xterm ctlseqs.txt `CSI Ps $ }` |
| **Sequence** | `CSI Ps $ }` — Select active status display |
| **Description** | Switches active writing between main display and status line |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | effect-mode-state |
| **Test chain** | parser:pending dispatch:pending state:pending |
| **Verification** | missing |
| **De-facto ref** | — |
| **Notes** | Ps=0 main display (default), Ps=1 status line; status line not implemented — stub acceptable; verified-with-deviation if stub |

---

**DECPRES-DECSSDT**

| Field | Value |
|---|---|
| **ID** | `DECPRES-DECSSDT` |
| **Spec source** | xterm ctlseqs.txt `CSI Ps $ ~` |
| **Sequence** | `CSI Ps $ ~` — Select status line type |
| **Description** | Configures whether status line is host-writable, indicator only, or off |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | effect-mode-state |
| **Test chain** | parser:pending dispatch:pending state:pending |
| **Verification** | missing |
| **De-facto ref** | — |
| **Notes** | Ps=0 off (default), Ps=1 indicator, Ps=2 host-writable; stub acceptable; verified-with-deviation if stub |

---

**DECPRES-DECRQSS**

| Field | Value |
|---|---|
| **ID** | `DECPRES-DECRQSS` |
| **Spec source** | xterm ctlseqs.txt `DCS $ q Pt ST` |
| **Sequence** | `DCS $ q Pt ST` — Request status string |
| **Description** | Requests the current setting for the CSI/DCS function named by Pt; terminal replies with DECSS |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | effect-pty-write |
| **Test chain** | parser:pending dispatch:pending state:pending effect:pending |
| **Verification** | missing |
| **De-facto ref** | — |
| **Notes** | DCS path — dispatch in `crates/vte/src/ansi/dispatch/dcs.rs` (NOT csi.rs); reply: `DCS 1 $ r Pt ST` for recognized Pt, `DCS 0 $ r ST` for unrecognized |

---

**DECPRES-DECRSPS**

| Field | Value |
|---|---|
| **ID** | `DECPRES-DECRSPS` |
| **Spec source** | xterm ctlseqs.txt `DCS Ps $ t Pt ST` |
| **Sequence** | `DCS Ps $ t Pt ST` — Restore presentation status |
| **Description** | Restores a presentation state previously reported by DECRQPSR |
| **Implementation** | MISSING — to be added by Section 09A |
| **Apex layer** | state-snapshot |
| **Test chain** | parser:pending dispatch:pending state:pending |
| **Verification** | missing |
| **De-facto ref** | — |
| **Notes** | DCS path — dispatch in `crates/vte/src/ansi/dispatch/dcs.rs` (NOT csi.rs); complex serialization; stub acceptable |

---

### Completion criteria

- `plans/spec-conformance/catalog/dec-presentation.md` exists and passes `spec-coverage-report --check` (no false-verified rows)
- All 13 row IDs resolve in `audits/section-09a-top-down-inventory.md`

---

## §09A.3 — VTE dispatch arms

### Goal

Add ALL missing CSI dispatch arms to `crates/vte/src/ansi/dispatch/csi.rs` for the 19 new CSI-path sequences (10 DECRECT + 9 DECPRES CSI-path rows; DECPRES-DECIC and DECPRES-DECDC are CSI-path — counted as 2 of the 9). The 2 DCS-path rows (DECRQSS, DECRSPS) and the 2 ESC-path rows (DECBI, DECFI) are NOT in scope for this subsection — those are handled in §09A.7 (ESC dispatch in `crates/vte/src/ansi/dispatch/mod.rs::esc_dispatch`, ~line 261) and §09A.9 (DCS dispatch in `crates/vte/src/ansi/dispatch/mod.rs::dispatch_hook` / `dispatch_unhook`, ~lines 52–110). Cross-check the dispatch path inventory against the existing live file — do NOT assume separate `dcs.rs` / `esc.rs` files exist (they do not).

### Files touched

- `crates/vte/src/ansi/dispatch/csi.rs` — convert to directory module (`dispatch/csi/mod.rs`) so it can host a sibling `tests.rs`; add 19 new match arms to the body
- `crates/vte/src/ansi/dispatch/csi/tests.rs` — new sibling test file per `test-organization.md §Sibling tests.rs Pattern`; every dispatch arm gets a parse-test + unhandled-negative-pin pair
- Parser-level tests in `crates/vte/src/ansi/tests.rs` remain owned by `ansi/mod.rs`; do NOT add dispatch-arm tests there

### Existing dispatch table reference

The existing dispatch table at `crates/vte/src/ansi/dispatch/csi.rs:56-325` matches `(action, intermediates)`. The final `_` arm at line 324 calls `unhandled!()`. New arms must be inserted BEFORE the wildcard. Key intermediates for the new sequences:

```rust
// DEC rectangular ops — intermediate '$'
('r', [b'$']) => handler.deccara(...)        // DECRECT-DECCARA  CSI Pt;Pl;Pb;Pr;Pm $ r
('t', [b'$']) => handler.decrara(...)        // DECRECT-DECRARA  CSI Pt;Pl;Pb;Pr;Pm $ t
('v', [b'$']) => handler.deccra(...)         // DECRECT-DECCRA   CSI ... $ v
('x', [b'*']) => handler.decsace(...)        // DECRECT-DECSACE  CSI Ps * x
('x', [b'$']) => handler.decfra(...)         // DECRECT-DECFRA   CSI Pc;Pt;Pl;Pb;Pr $ x
('y', [b'#']) => handler.xtchecksum(...)     // DECRECT-XTCHECKSUM  CSI Ps # y
('y', [b'*']) => handler.decrqcra(...)       // DECRECT-DECRQCRA  CSI Pi;Pg;Pt;Pl;Pb;Pr * y
('z', [b'$']) => handler.decera(...)         // DECRECT-DECERA   CSI Pt;Pl;Pb;Pr $ z
('{', [b'$']) => handler.decsera(...)        // DECRECT-DECSERA  CSI Pt;Pl;Pb;Pr $ {
('|', [b'#']) => handler.xtreportsgr(...)    // DECRECT-XTREPORTSGR  CSI Pt;Pl;Pb;Pr # |

// DEC presentation ops — intermediates '$', '*', '"', '&', '\''
('w', [b'$']) => handler.decrqpsr(...)       // DECPRES-DECRQPSR  CSI Ps $ w
('u', [b'&']) => handler.decrqupss()         // DECPRES-DECRQUPSS  CSI & u
('v', [b'"']) => handler.decrqde()           // DECPRES-DECRQDE   CSI " v
('p', [b'"']) => handler.decscl(...)         // DECPRES-DECSCL   CSI Pl;Pc " p
('q', [b'"']) => handler.decsca(...)         // DECPRES-DECSCA   CSI Ps " q
('}', [b'$']) => handler.decsasd(...)        // DECPRES-DECSASD  CSI Ps $ }
('~', [b'$']) => handler.decssdt(...)        // DECPRES-DECSSDT  CSI Ps $ ~
('}', [b'\'']) => handler.decic(...)         // DECPRES-DECIC   CSI Ps ' }
('~', [b'\'']) => handler.decdc(...)         // DECPRES-DECDC   CSI Ps ' ~
```

**Collision check:** Scan the existing dispatch table before adding arms to confirm no existing arm matches these `(final_byte, intermediates)` pairs. The existing `('{', [b'#'])` arm at line 322 (push_sgr) and `('}', [b'#'])` at line 323 (pop_sgr) use `[b'#']` intermediate — the new arms use `[b'$']`, `[b'*']`, `[b'"']`, `[b'&']`, `[b'\'']`, so there are no collisions.

### Test requirements

For EACH new dispatch arm, the canonical sibling-test home per `.claude/rules/test-organization.md §Sibling tests.rs Pattern` is **`crates/vte/src/ansi/dispatch/csi/tests.rs`** (sibling to `dispatch/csi.rs`, the source that owns the dispatch arms). This requires converting `dispatch/csi.rs` to a directory module (`dispatch/csi/mod.rs`) before adding the new tests. Each dispatch-arm test adds:
1. **Parse test**: feed the raw byte sequence (constructed from known params), assert `RecordingHandler` method was called with expected params
2. **Unhandled negative pin**: feed the same final byte with a DIFFERENT intermediate (e.g. `('r', [b'*'])`) and assert `unhandled()` was called, NOT the handler method

**Do NOT** add dispatch-arm tests to `crates/vte/src/ansi/tests.rs` — that file owns parser-level tests for `ansi/mod.rs`, and co-locating dispatch routing tests there violates the "one tests.rs per source file" discipline (every sibling is a distinct test owner). `dispatch/tests.rs` already exists as sibling-tests owner for `dispatch/mod.rs`; `dispatch/csi.rs` needs its own sibling pair.

### Pre-implementation BLOAT split (required before new tests are added)

`crates/vte/src/ansi/tests.rs` is currently 512 lines — already at the 500-line hygiene limit per `.claude/rules/code-hygiene.md §File Size`. Existing parser-level tests living there need breathing room, AND the new §09A.3 dispatch tests have their own canonical home at `dispatch/csi/tests.rs` (per `test-organization.md §Sibling tests.rs Pattern` — tests live sibling to the source they test, not bundled with the parser entry point). Two coordinated moves happen BEFORE §09A.3 adds any new tests:

1. **Parser-test breathing room**: if existing `ansi/tests.rs` content genuinely tests `ansi/mod.rs` parser behavior rather than dispatch routing, leave it where it is and split only if the 500-line limit is still breached after step 2. If it contains dispatch-routing tests that should have lived sibling to `dispatch/csi.rs` all along, migrate those to the new `dispatch/csi/tests.rs` (step 2) as part of the preparatory cleanup.
2. **Dispatch-tests sibling home**: convert `crates/vte/src/ansi/dispatch/csi.rs` to a directory module (`crates/vte/src/ansi/dispatch/csi/mod.rs`) and create `crates/vte/src/ansi/dispatch/csi/tests.rs` as the sibling test file. New §09A.3 dispatch-arm tests land there — NOT in `ansi/tests.rs`.
3. Confirm `cargo test -p vte` green AFTER the split, BEFORE adding any §09A.3 new tests.
4. New §09A.3 tests land in `crates/vte/src/ansi/dispatch/csi/tests.rs` (the sibling to the source that owns the dispatch arms). If the file grows past the hygiene limit as more dispatch arms are added, further split by intermediate class (e.g., `dispatch/csi/tests/dec_rect.rs` + `dispatch/csi/tests/dec_presentation.rs`) — but not speculatively; split only when the limit is breached.

The tests-sibling move is a BLOCKING prerequisite for §09A.3's dispatch-arm additions. Do not interleave the move with the new test additions — move first, then add, so the diff is reviewable.

### Similar BLOAT pre-split for other files §09A.3 touches

- `crates/vte/src/ansi/dispatch/csi.rs` (443 lines) — approaching limit. §09A.3 adds ~19 dispatch arms. If the new arms push csi.rs past 500 lines, split it — one plausible split is `dispatch/csi/mod.rs` dispatching to `dispatch/csi/common.rs` (existing arms) + `dispatch/csi/dec_private.rs` (new DEC-private arms). Decide during the split after measuring post-add line count.
- `crates/vte/src/ansi/handler.rs` (437 lines) — approaching limit. §09A.4 adds ~21 trait default methods. If the new methods push handler.rs past 500 lines, extract the trait block into a submodule: `handler/mod.rs` re-exporting `handler/trait_methods.rs`.

### Completion criteria

- `crates/vte/src/ansi/tests.rs` split into submodule files BEFORE new tests are added; post-split `cargo test -p vte` green
- All 19 CSI dispatch arms present in `csi.rs` (or `csi/dec_private.rs` if split); `cargo test -p vte` green
- Each arm has a parse+dispatch test and an unhandled negative pin in the appropriate `tests/` submodule
- `./clippy-all.sh` green (no dead-code warnings on the new handler method calls — handler stubs must be called)
- No file touched by §09A.3 / §09A.4 exceeds 500 lines at the end of the subsection; if a split was performed, the new module structure is documented in a short `//!` preamble on each new file

---

## §09A.4 — Handler trait methods

### Goal

Add concrete override implementations in `oriterm_core/src/term/handler/` for every new handler method. The 19 default (no-op) trait stubs in `crates/vte/src/ansi/handler.rs` for the CSI-path methods (decsace / deccara / decrara / deccra / decfra / xtchecksum / decrqcra / decera / decsera / xtreportsgr / decrqpsr / decrqupss / decrqde / decscl / decsca / decsasd / decssdt / decic / decdc) ALREADY LANDED in §09A.3 — the dispatch arms could not compile without them. §09A.4's residual scope: (1) add the 2 ESC-path defaults (decbi, decfi), (2) add ESC 6/9 dispatch arms in `dispatch/mod.rs::dispatch_esc`, (3) add concrete Term overrides under `oriterm_core/src/term/handler/`.

### Files touched

- `crates/vte/src/ansi/handler.rs` — add 2 remaining default trait methods (`decbi`, `decfi`); the 19 CSI-path methods landed in §09A.3
- `oriterm_core/src/term/handler/rect_ops/mod.rs` and `oriterm_core/src/term/handler/presentation/mod.rs` — new directory modules (directory-module form is mandatory so sibling `tests.rs` files are allowed per `test-organization.md` rule 1)
- `oriterm_core/src/term/handler/rect_ops/tests.rs` + `oriterm_core/src/term/handler/presentation/tests.rs` — sibling test files
- `crates/vte/src/ansi/dispatch/mod.rs` — add ESC 6 (DECBI) and ESC 9 (DECFI) arms inside the existing `esc_dispatch` function (~line 261). There is NO standalone `dispatch/esc.rs` — ESC dispatch lives inline in `dispatch/mod.rs`

### Handler method signatures (canonical form)

```rust
// In crates/vte/src/ansi/handler.rs Handler trait:

// Rectangle ops
fn decsace(&mut self, _mode: u16) {}
fn deccara(&mut self, _top: u16, _left: u16, _bot: u16, _right: u16, _attrs: &[u16]) {}
fn decrara(&mut self, _top: u16, _left: u16, _bot: u16, _right: u16, _attrs: &[u16]) {}
fn deccra(&mut self, _src_top: u16, _src_left: u16, _src_bot: u16, _src_right: u16,
          _src_page: u16, _dst_top: u16, _dst_left: u16, _dst_page: u16) {}
fn decfra(&mut self, _ch: u16, _top: u16, _left: u16, _bot: u16, _right: u16) {}
fn xtchecksum(&mut self, _flags: u16) {}
fn decrqcra(&mut self, _id: u16, _page: u16, _top: u16, _left: u16, _bot: u16, _right: u16) {}
fn decera(&mut self, _top: u16, _left: u16, _bot: u16, _right: u16) {}
fn decsera(&mut self, _top: u16, _left: u16, _bot: u16, _right: u16) {}
fn xtreportsgr(&mut self, _top: u16, _left: u16, _bot: u16, _right: u16) {}

// Presentation ops (CSI path)
fn decrqpsr(&mut self, _mode: u16) {}
fn decrqupss(&mut self) {}
fn decrqde(&mut self) {}
fn decscl(&mut self, _level: u16, _c1_mode: u16) {}
fn decsca(&mut self, _protected: u16) {}
fn decsasd(&mut self, _target: u16) {}
fn decssdt(&mut self, _line_type: u16) {}

// Column ops
fn decic(&mut self, _count: u16) {}
fn decdc(&mut self, _count: u16) {}

// Back/Forward index (ESC path — handler called by esc.rs dispatcher)
fn decbi(&mut self) {}
fn decfi(&mut self) {}
```

### oriterm_core override structure

The Term handler overrides live in `oriterm_core/src/term/handler/`. Per the 500-line file size rule (code-hygiene.md) AND the sibling-tests-pattern rule (test-organization.md §Sibling `tests.rs` Pattern, rule #1: "When a module has tests, it **must** be a directory module (`foo/mod.rs`), not a file module (`foo.rs`). Never have `foo.rs` alongside a `foo/` directory."), create two new handler modules as **directory modules** so they can host sibling `tests.rs` without violating the layout rule:

- `oriterm_core/src/term/handler/rect_ops/mod.rs` — implements `decsace`, `deccara`, `decrara`, `deccra`, `decfra`, `xtchecksum`, `decrqcra`, `decera`, `decsera`, `xtreportsgr`
- `oriterm_core/src/term/handler/presentation/mod.rs` — implements `decrqpsr`, `decrqupss`, `decrqde`, `decscl`, `decsca`, `decsasd`, `decssdt`, `decic`, `decdc`, `decbi`, `decfi`

Each `mod.rs` ends with `#[cfg(test)] mod tests;` (semicolon, no braces) per `test-organization.md §No inline test modules`. The sibling test files are:

- `oriterm_core/src/term/handler/rect_ops/tests.rs`
- `oriterm_core/src/term/handler/presentation/tests.rs`

**Do NOT** use file modules (`rect_ops.rs` / `presentation.rs`) with a separate `tests.rs` — that combination forces the banned `foo.rs` + `foo/` coexistence. Directory modules are the canonical form whenever the module has tests.

### Completion criteria

- All handler methods compile; default trait stubs in vte are callable from `RecordingHandler` tests
- `./build-all.sh` green (including Windows cross-compile — all new methods use only platform-independent grid operations)

---

## §09A.5 — DECRQCRA implementation

### Goal

Implement the DECRQCRA checksum handler in `oriterm_core/src/term/handler/rect_ops/mod.rs`. The implementation: (1) clamps coordinates to grid bounds, (2) computes the checksum using the xterm patch-336 algorithm, (3) emits the reply synchronously via `PtyEffect::Write`.

### Files touched

- `oriterm_core/src/term/handler/rect_ops/mod.rs` — `decrqcra()` method + `compute_rect_checksum()` helper
- `oriterm_core/tests/spec_chain/dec_rect_ops/decrqcra.rs` — spec_chain tests
- `oriterm_core/tests/alloc_regression.rs` — performance pin for zero-allocation in checksum inner loop

### xterm patch-336 algorithm (pinned against xterm reference)

**Pinned algorithm.** The checksum is a 16-bit value computed by xterm's `xtermCheckRect()` in `~/projects/reference_repos/console_repos/xterm/screen.c:3136` (function body lines 3136–3265). The algorithm is **sum-then-negate** — NOT CRC-16, NOT XOR-fold, NOT ones-complement-sum. The precise steps:

1. Initialize `total = 0` (signed int accumulator).
2. For each cell in the rectangle in row-major order:
   a. Extract the cell character (DEC-charset-translated by `xtermCharSetDec` by default; raw byte if `csBYTE` flag in `checksum_flags`).
   b. `total += ch`.
   c. If `!(flags & csATTRIBS)` — i.e., ATTRIBS are INCLUDED by default — fold SGR attributes: `total += fg_color_index` (foreground, if set) and `total += bg_color_index` (background, if set). For video attributes (bold, underline, blink, reverse, protected, invisible), xterm adds tagged constants: see `screen.c:3217–3244` for the precise mapping.
   d. If wide-char combining marks are present, fold each: `total += combData[off][col]` (see `screen.c:3247–3249`).
3. After all cells: `if !(flags & csPOSITIVE) { total = -total; }` — the default is NEGATE (csPOSITIVE is the OPT-OUT flag).
4. Truncate to 16 bits: `result = total & 0xFFFF`.

**checksum_flags bitmask** (from `DECRECT-XTCHECKSUM` / `CSI Ps # y`), matching xterm's `csDEC=0`, `csPOSITIVE=1`, `csATTRIBS=2`, `csNOTRIM=4`, `csDRAWN=8`, `csBYTE=16` constants at `xterm/screen.c`:

| Bit | Name        | Effect when SET                                                   |
|---:|-------------|-------------------------------------------------------------------|
|  1 | csPOSITIVE  | Do NOT negate final sum (return positive) — default is negate     |
|  2 | csATTRIBS   | EXCLUDE SGR attributes from sum — default is include              |
|  3 | csNOTRIM    | Do NOT trim trailing blanks — default trims                       |
|  4 | csDRAWN     | Include non-CHARDRAWN cells as space (differs from csNOTRIM path) |
|  5 | csBYTE      | Use raw character byte instead of DEC-translated codepoint        |

Default mode (Ps=0 or no XTCHECKSUM received): all flags 0 → sum attributes, trim blanks, DEC-translate, NEGATE final sum.

**Reply format:** `DCS Pi ! ~ XXXX ST` where Pi is the request ID (echoed from the DECRQCRA Pi param) and XXXX is the 4-digit uppercase hexadecimal checksum (two's-complement for negative values, masked to 16 bits). Full byte sequence:
```
ESC P <id-digits> ! ~ <4-hex-digits> ESC \
```

**Concrete reply example.** DECRQCRA with Pi=3 over a 1×1 grid containing ASCII `'A'` (0x41), default flags (negate-on, attribs-included, no attrs set on cell): `total = 0x41`, after negate `total = -0x41 = 0xFFBF` (mask to 16 bits). Reply bytes (14 total): `\x1b P 3 ! ~ F F B F \x1b \\` → `\x1bP3!~FFBF\x1b\\`.

For example, if id=1 and checksum=0xABCD: `\x1bP1!~ABCD\x1b\\`

### Implementation pattern (mirrors DA3 in status.rs:160-168)

```rust
pub(super) fn rect_ops_decrqcra(
    &mut self,
    id: u16, _page: u16,
    top: u16, left: u16, bot: u16, right: u16,
) {
    let checksum = self.compute_rect_checksum(top, left, bot, right);
    let response = format!("\x1bP{id}!~{checksum:04X}\x1b\\");
    self.effect_sink.push(Effect::Pty(PtyEffect::Write {
        bytes: response.into_bytes(),
        kind: PtyWriteKind::ChecksumReport,
    }));
}
```

`PtyWriteKind::ChecksumReport` is a new variant added alongside this implementation. Its addition MUST be exhaustive-match-safe — audit all `match kind { ... }` arms across the codebase and add the new arm.

### Zero-allocation requirement

`compute_rect_checksum()` MUST NOT allocate. The inner loop iterates over grid rows via `self.grid.visible_row(line)` (returns `&Row`) and accesses cells by column index. No `Vec::collect()`, no `String::format!()` allocations inside the loop. The final `format!()` for the reply string happens ONCE after the loop, not per cell — use `PtyEffect::Write::bytes` as the single owned allocation.

**Reuse the existing closure-based alloc-regression pattern.** `oriterm_core/tests/alloc_regression.rs` already supports closure-based pin tests (see lines 57–84 and 191–215 — the pattern is a closure passed into `measure_alloc_count!` or `assert_zero_allocs!` with a setup + hot-loop split). §09A.5 reuses that pattern — do NOT invent a new pin type. The DECRQCRA pin fits the existing "zero allocations inside the hot computation closure" shape exactly.

Add to `oriterm_core/tests/alloc_regression.rs` (mirroring the existing pattern at lines 57–84):
```rust
#[test]
fn decrqcra_no_alloc_in_checksum_loop() {
    // Setup (outside alloc gate): build Term, issue XTCHECKSUM, feed DECRQCRA,
    // collect the resulting effect envelope.
    // ...
    // Gated hot loop: re-feed DECRQCRA 100 times and assert zero allocations
    // inside `compute_rect_checksum` — the format!() for the reply IS allowed
    // (one alloc per call is budgeted; the INNER loop is what this pin protects).
    // Use the closure pattern at oriterm_core/tests/alloc_regression.rs:57–84 —
    // NOT a new pin type.
}
```

### Test requirements (spec_chain)

In `oriterm_core/tests/spec_chain/dec_rect_ops/decrqcra.rs`:

1. **canonical params**: 5×5 grid with known content, DECRQCRA over the full grid, assert reply bytes match expected checksum
2. **id passthrough**: different Pi values round-trip to the reply prefix
3. **coordinate clamping**: top/left/bot/right beyond grid bounds are clamped; result is same as valid-bounds equivalent
4. **zero-area rectangle**: top > bot after clamping → no-op (checksum of 0)
5. **XTCHECKSUM flags**: Bit 0 set vs. unset changes checksum when cell has video attribute set (semantic pin)
6. **negative pin**: DECRQCRA bytes fed through SpecHarness produce EXACTLY ONE PtyEffect::Write; no HostRequest is emitted

### Completion criteria

- `DECRECT-DECRQCRA` row verification status promoted from `missing` to `implemented-unverified` (full verification at §09A.N)
- `alloc_regression.rs` green with new DECRQCRA pin
- `cargo test -p oriterm_core` green

---

## §09A.6 — Rectangular area ops

### Goal

Implement the six rectangular area MUTATION ops: DECCRA, DECFRA, DECERA, DECSERA, DECRARA, DECCARA. The handler methods live in `oriterm_core/src/term/handler/rect_ops/mod.rs` (thin adapter layer) but the actual row/cell mutation logic lives inside `oriterm_core/src/grid/editing/` (for fill/erase/attribute mutations) and `oriterm_core/src/grid/scroll/` (for copy operations that touch damage state). The handler method's job is to parse + clamp params then delegate to grid methods that already respect the grid invariants listed below.

### Architectural constraint — grid invariants must not be bypassed

Row mutations in `oriterm_core/src/term/handler/mod.rs:140-177` and elsewhere flip `selection_dirty` BEFORE delegating to grid methods. Wide-char-spacer cleanup, damage marking, and selection-tracking state all live inside `grid/editing/` and `grid/scroll/` — bypassing those modules by writing to row cells directly from `rect_ops.rs` would silently regress selection tracking and damage invariants. The rect-ops handler methods MUST:

1. Extract/clamp params to (top, left, bot, right) 0-indexed coordinates
2. Set `selection_dirty = true` (match the existing pattern at `term/handler/mod.rs:140-177`)
3. Delegate to a grid method (e.g., `grid.fill_rect(top, left, bot, right, cell)`, `grid.erase_rect_unprotected(...)`, `grid.copy_rect(src, dst)`) that owns the actual cell iteration and invariant maintenance
4. Return without touching cells via `&mut Row` directly

If a required grid method does not exist (e.g., `fill_rect` for DECFRA, `copy_rect` for DECCRA with overlap-safe scratch), add it to the grid layer in the same commit — the correct home is `oriterm_core/src/grid/editing/` (new submodule `rect.rs` under the existing `editing/` tree) with a sibling `tests.rs`. See `.claude/rules/impl-hygiene.md §Module Boundary Discipline` for the canonical-home rationale: row mutation is grid's responsibility, not the handler's.

### Files touched

- `oriterm_core/src/term/handler/rect_ops/mod.rs` — handler methods for all six ops + shared `clamp_rect()` helper (adapter layer only — no direct row writes)
- `oriterm_core/src/grid/editing/rect.rs` (NEW) — `fill_rect`, `erase_rect_all`, `erase_rect_unprotected`, `apply_sgr_rect`, `reverse_sgr_rect`, `copy_rect` methods on `Grid` (or wherever `grid/editing/mod.rs` dispatches)
- `oriterm_core/src/grid/editing/tests.rs` — unit tests for each rect grid method at the grid layer
- `oriterm_core/tests/spec_chain/dec_rect_ops/` — end-to-end spec_chain tests per op (handler → grid → snapshot apex)

### Shared coordinate clamping

Extract `clamp_rect(top, left, bot, right) -> Option<(usize, usize, usize, usize)>` as a shared private helper. Returns `None` for zero-area rectangles (top > bot or left > right after clamping). Returns `Some((t, l, b, r))` with each coordinate clamped to `[0, rows-1]` × `[0, cols-1]` (0-indexed after the 1-based input conversion). All six ops call this helper before any grid mutation.

When DECLRMM (mode 69) is active (`self.grid.left_right_margins()`), the left/right clamping also enforces the active left-right margins: left is clamped to `max(left, margin_left)` and right is clamped to `min(right, margin_right)`.

### Per-op implementation notes

- **DECFRA** (`decfra`): fill with character Pc (u16 → char) at current SGR attributes. Iterate rectangle cells, write (Pc, current_sgr) to each cell.
- **DECERA** (`decera`): erase (write space with default attrs) to all cells in rectangle, UNLESS cell has DECSCA protection flag. Check `cell.flags.contains(CellFlags::PROTECTED)`.
- **DECSERA** (`decsera`): erase ONLY unprotected cells (complement of DECERA protection check: skip cells WITH DECSCA protection).
- **DECCARA** (`deccara`): apply the Pm SGR attributes to all cells in rectangle. The attribute list is a u16 slice (multiple params). DECSACE mode governs whether the extent is rectangular (default) or stream.
- **DECRARA** (`decrara`): reverse the Pm SGR attributes on all cells in rectangle.
- **DECCRA** (`deccra`): copy source rectangle to destination. Source and destination may overlap — copy to a scratch buffer first to avoid overwrites. Source/destination pages are accepted but since ori_term is single-page, page ≠ 1 is a `verified-with-deviation` note.

### Test requirements

Per op (use spec_chain with `observe_state` / `observe_renderable` apex):

1. Fill a grid with known content; apply the op; assert correct cells mutated
2. Zero-area rectangle (top > bot): no grid mutation
3. Out-of-bounds coordinates: clamped correctly; result identical to in-bounds equivalent
4. DECLRMM active: left/right margins constrain the operation
5. **Negative pin per op**: cell OUTSIDE the rectangle is unchanged (clamp boundary correctness)

### Completion criteria

- All six rows promoted from `missing` to `implemented-unverified`
- `cargo test -p oriterm_core` green

---

## §09A.7 — Column ops + ESC-path index ops

### Goal

Implement DECIC, DECDC (CSI-path column insert/delete) and DECBI, DECFI (ESC-path back/forward index) in `oriterm_core/src/term/handler/presentation/mod.rs`. The ESC dispatch arm for ESC 6 and ESC 9 must also be confirmed or added.

### Files touched

- `oriterm_core/src/term/handler/presentation/mod.rs` (NEW) — `decic()`, `decdc()`, `decbi()`, `decfi()` methods
- `crates/vte/src/ansi/dispatch/mod.rs::esc_dispatch` (~line 261 in the existing file — there is no standalone `esc.rs`) — add ESC 6 (final byte `b'6'`) → `handler.decbi()` and ESC 9 (final byte `b'9'`) → `handler.decfi()` arms inside the `match byte { ... }` block. The match currently falls through to `debug!("[unhandled] esc_dispatch ...")` for these bytes; add the new arms above that catch-all.

### Implementation notes

**DECIC** (`decic(count)`): Insert `count` blank columns at the cursor column. Content from cursor column to right margin shifts right by `count`; columns that shift beyond right margin are discarded. When DECLRMM is active, the right margin constrains the shift. When DECLRMM is NOT active, the shift extends to the physical right edge.

**DECDC** (`decdc(count)`): Delete `count` columns at the cursor column. Content from cursor+count to right margin shifts left; blank columns appear at the right margin.

**DECBI** (`decbi()`): If cursor is at the LEFT margin (or column 0 if DECLRMM inactive): insert a blank column at the left margin, scrolling the row right. Otherwise: move cursor left one column (equivalent to cursor-left-1).

**DECFI** (`decfi()`): If cursor is at the RIGHT margin (or last column if DECLRMM inactive): insert a blank column at the right margin, scrolling the row left. Otherwise: move cursor right one column.

### ESC dispatch verification

ESC dispatch lives inline in `crates/vte/src/ansi/dispatch/mod.rs` via the `esc_dispatch()` trait-impl method around line 261 and the standalone `fn dispatch_esc_dispatch` (or equivalent dispatcher block above it). There is NO standalone `esc.rs` file. Verify ESC 6 and ESC 9 are not already dispatched to a different handler before adding new arms. The final byte for ESC 6 is `b'6'` and for ESC 9 is `b'9'`; both currently fall through to the `debug!("[unhandled] esc_dispatch ...")` arm (see `crates/vte/src/ansi/dispatch/mod.rs:138`).

### Test requirements

For DECIC/DECDC:
1. Insert/delete columns in the middle of a grid with known content
2. Insert at column 0 (edge case)
3. Delete at last column
4. DECLRMM active: operation constrained to margins

For DECBI/DECFI:
1. Cursor NOT at margin: moves one column (pure cursor-move, no scroll)
2. Cursor AT margin: inserts blank column, existing content scrolls
3. Negative pin: column outside the rectangle is unchanged

### Completion criteria

- DECPRES-DECIC, DECPRES-DECDC, DECPRES-DECBI, DECPRES-DECFI promoted from `missing` to `implemented-unverified`
- ESC 6 / ESC 9 arms confirmed in dispatch; `cargo test -p vte` green

---

## §09A.8 — Presentation queries (CSI path)

### Goal

Implement the seven CSI-path presentation query stubs: DECRQPSR, DECRQUPSS, DECRQDE, DECSCL, DECSCA, DECSASD, DECSSDT. For queries that require serializing complex state (DECRQPSR), a stub reply is acceptable for initial verification; the row is promoted to `verified-with-deviation` with a deviation note.

### Files touched

- `oriterm_core/src/term/handler/presentation/mod.rs` — concrete implementations
- `oriterm_core/tests/spec_chain/dec_presentation/` — spec_chain tests per row

### Per-op implementation notes

**DECRQDE** (`decrqde()`): Simplest — reply with current grid dimensions: `CSI rows;cols " w`. Fully implementable.

**DECRQUPSS** (`decrqupss()`): Reply with the user-preferred supplemental character set. ori_term does not implement NRCS charset selection; a constant reply identifying ISO Latin-1 (`CSI " u`) is acceptable. Mark `verified-with-deviation` with deviation: "constant reply — user-preferred supplemental charset not implemented".

**DECSCL** (`decscl(level, c1_mode)`): Store conformance level on Term as observable state (`conformance_level: u8`, `c1_8bit: bool`). Level 1 = VT100, 2 = VT200, 3 = VT300. C1 mode: 0 or 2 = 8-bit C1, 1 = 7-bit C1. **Parser scope (out of Term's reach):** 8-bit C1 dispatch is owned by the vendored parser at `crates/vte/src/lib.rs:731-755` (`dispatch_c1`) and runs unconditionally — `c1_8bit: bool` on Term does NOT suppress parser-level C1 recognition. §09A.8 stores the flag as OBSERVABLE state only; any behavior change that depends on genuinely disabling 0x80-0x9F dispatch requires a separate vendored-parser patch with parser-level tests (NOT in scope for Section 09A — file a `/add-bug` if that behavior change is ever needed). The parser deviation MUST be listed in the DECPRES-DECSCL catalog row Notes as `verified-with-deviation: c1_8bit flag stored but does not suppress parser C1 dispatch`. **Soft-reset side effect:** DECSCL triggers a soft reset per the catalog row at `plans/spec-conformance/catalog/dec-presentation.md:34` ("triggers a soft reset"). The implementation MUST call the existing DECSTR soft-reset helper (see `oriterm_core/src/term/handler/status.rs` for the current DECSTR path; re-use it, do NOT duplicate reset logic) AFTER storing the new level/mode flags. Verify via spec_chain `observe_state`: after DECSCL, the set of soft-reset-cleared flags (cursor visibility, insert mode, margins, SGR) MUST match post-DECSTR state.

**DECSCA** (`decsca(protected)`): Set the per-character protection attribute for SUBSEQUENTLY written characters. Store as `char_protection: bool` on Term. This flag is applied when cells are written (same pattern as current SGR attribute application). Protected cells survive DECSERA/DECERA. MUST store in `CellFlags::PROTECTED` when the cell is written, not on the cell retroactively. The canonical home for `CellFlags` is `oriterm_core/src/cell/mod.rs` (current bitflag declaration at line 18 — there is NO `cell/flags.rs` submodule). Add a new `CellFlags::PROTECTED` bit in that same file, NOT a new submodule file. `CellFlags` is already SSOT per `.claude/rules/impl-hygiene.md` — do not split it out unless `cell/mod.rs` crosses the 500-line limit.

**DECSACE `ace_mode` — architectural note.** The `ace_mode` state (stream vs rect extent, set by DECSACE) belongs on Term, not Grid. Grid owns cell storage and mutations; Term owns mode state (TermMode flags) and dispatches mutations to Grid. Adding `ace_mode: u8` as a Term field (alongside `conformance_level`, `c1_8bit`, `checksum_flags`, `char_protection`, `active_status_display`, `status_line_type` — all new Term fields introduced by Section 09A) preserves that separation. Do NOT add `ace_mode` to Grid — that would bleed a mode concern into the storage layer (a LEAK per `.claude/rules/impl-hygiene.md §Module Boundary Discipline`).

**DECSASD** (`decsasd(target)`): Ps=0 main display (default), Ps=1 status line. ori_term does not implement a DEC status line. Dispatch arm routes to handler; Term stores `active_status_display: u8`. `verified-with-deviation`: "status line write target accepted but status line not rendered".

**DECSSDT** (`decssdt(line_type)`): Ps=0 off, Ps=1 indicator only, Ps=2 host-writable. Store `status_line_type: u8`. Same deviation as DECSASD.

**DECRQPSR** (`decrqpsr(mode)`): Mode 1 = cursor information report; Mode 2 = tab stop report. The DCS reply for mode 2 (tab stops) requires serializing the full tab-stop vector — complex but implementable. Mode 1 (cursor) requires serializing cursor row, col, page, protection, and current charset designations — more complex. Initial verification: emit the correct DCS header (`DCS 1 $ s ...`) for Mode 2 with accurate tab stop data; Mode 1 as `verified-with-deviation`. File `/add-bug` for full cursor state serialization.

### Completion criteria

- All 7 CSI-path presentation rows promoted from `missing` to `implemented-unverified` or `verified-with-deviation` (deviation documented in catalog Notes field)
- DECSCA `char_protection` flag propagates to `CellFlags::PROTECTED` when cells are written — verified by spec_chain test that writes a character after DECSCA Ps=1 and confirms the cell has PROTECTED flag, then calls DECSERA and confirms the cell is NOT erased

---

## §09A.9 — DCS-path presentation queries

### Goal

Extend the existing DCS dispatcher in `crates/vte/src/ansi/dispatch/mod.rs` (`dispatch_hook` at lines 52–110 and `dispatch_unhook` at lines 101+) with routing for DECRQSS (`DCS $ q Pt ST`) and DECRSPS (`DCS Ps $ t Pt ST`). These are NOT in `csi.rs` — the DCS dispatcher is a separate path inside the same `mod.rs`.

### Existing coverage — this is EXTENSION, not greenfield

`oriterm_core/src/term/handler/tests/dcs.rs:94-168` already exercises DECRQSS end-to-end through the DCS path — the DCS hook/put/unhook pipeline is live. §09A.9's actual work is SCOPE-LIMITED:

- Enumerate which DECRQSS target response formats are already implemented (walk `oriterm_core/src/term/handler/status.rs` `status_decrqss` — the production DECRQSS handler per `handler/mod.rs:460-461` delegate — plus `oriterm_core/src/term/handler/tests/dcs.rs:94-168` for observable coverage), and which are missing. `oriterm_core/src/term/handler/dcs.rs` also exists but owns DCS-adjacent features (DECSCUSR cursor style, Kitty keyboard mode stack); it does NOT own DECRQSS dispatch. §09A.9 extends `status.rs`, not `dcs.rs`.
- Baseline coverage at HEAD (verified from `status.rs:218-253`): `"p"` (DECSCL), `r` (DECSTBM), `m` (SGR), `s` (DECSLRM) are ALREADY implemented. The genuinely missing Pt targets are `q` (DECSCUSR) and `"q"` (DECSCA) — §09A.9 adds ONLY those branches plus any other xterm-published targets the baseline walk reveals are absent. Do NOT re-implement the existing four branches
- Add DECRSPS dispatch if absent — parse-and-acknowledge stub acceptable

**Do NOT phrase §09A.9 as "creating a DCS dispatch path" — the path exists.** The plan's completion criterion is "new DECRQSS targets enumerated against the current DCS test coverage at `tests/dcs.rs:94-168`, gap list produced, each gap row gets a dispatch branch and a spec_chain test in the same commit, unknown Pt values explicitly reply `DCS 0 $ r ST`."

### Files touched

- `crates/vte/src/ansi/dispatch/mod.rs` — extend `dispatch_hook` / `dispatch_unhook` with DECRQSS and DECRSPS Pt-routing (no new file, no new module)
- `crates/vte/src/ansi/handler.rs` — `decrqss(&mut self, _query: &[u8])` already exists at `handler.rs:310-314` with the `&[u8]` signature (NOT `&str`); §09A.9 extends the existing trait method via richer default parsing if needed, and adds ONLY the missing `decrsps(&mut self, _ps: u16, _pt: &[u8])` default method. Preserve the byte-oriented signature — the parser emits the `Pt` bytes before UTF-8 validation, so a `&str` signature would force an upstream decode that doesn't exist
- `oriterm_core/src/term/handler/status.rs` — extend the existing `status_decrqss` helper with new Pt-target branches. The current delegate at `oriterm_core/src/term/handler/mod.rs:460-461` routes `decrqss(&[u8])` to `self.status_decrqss(query)` in `status.rs`; DO NOT move the delegate to `dcs.rs`. `handler/dcs.rs` exists today but owns DCS-adjacent things (DECSCUSR cursor style in `dcs_set_cursor_style` / `dcs_set_cursor_shape`, Kitty keyboard mode stack); it is NOT the DECRQSS dispatch home. Add a new `status_decrsps` helper in `status.rs` for symmetry
- `oriterm_core/src/term/handler/presentation/mod.rs` — only if CSI-path state needs to be READ by DCS-path handlers; in that case, add accessor methods, do not duplicate state

### Implementation notes

**DECRQSS** (`DCS $ q Pt ST`): The `Pt` string names a CSI/DCS function. The terminal replies with `DCS 1 $ r Pt ST` if the function is recognized, or `DCS 0 $ r ST` if not. Baseline at HEAD (verify at `oriterm_core/src/term/handler/status.rs:218-253`): the four targets `"p"` (DECSCL), `r` (DECSTBM), `m` (SGR), `s` (DECSLRM) already reply with `DCS 1 $ r <echo-of-Pt>;<current-value> ST`. §09A.9 adds ONLY the missing targets — `q` (DECSCUSR) and `"q"` (DECSCA), plus any additional xterm-published targets the baseline walk identifies as absent. Unknown Pt continues to reply `DCS 0 $ r ST` per the existing `_ => "\x1bP0$r\x1b\\"` branch. Mark `verified-with-deviation` if fewer than all published xterm DECRQSS targets are implemented; list the deviations in the catalog Notes.

**DECRSPS** (`DCS Ps $ t Pt ST`): Restores presentation state previously reported by DECRQPSR. For initial implementation, parse and acknowledge but do not implement full state restoration. `verified-with-deviation`: "DECRSPS parse-and-acknowledge stub — full state restoration not implemented".

### Completion criteria

- DECPRES-DECRQSS and DECPRES-DECRSPS promoted from `missing` to `verified-with-deviation` (both are stub-level implementations with documented deviations)
- DCS dispatch path verified and documented in the catalog Notes field for each row

---

## §09A.10 — Verification of sections 11-26 top-down audit wiring

### Goal

**This is verification, NOT rewrite.** Sections 11-26 (16 sections) already carry the top-down audit success criterion + `§NN.0` subsection + audit stub file, landed in the pre-implementation planning pass. §09A.10 VERIFIES the wiring is correct and PATCHES only specific drift discoveries — it does NOT re-author the content.

**Why this scope reduction matters.** Section 11 is `reviewed: true` (its §11.0 subsection, audit-file success criterion, and `audits/section-11-top-down-inventory.md` stub all already landed and were reviewed). Any rewrite that mutates `reviewed: true` section frontmatter re-invalidates the review per `.claude/rules/review-plan-verify` semantics — the plan files are still under active review-pipeline flow. §09A.10 must NOT trigger a 16-file re-review cascade. The work of the verbiage rewrite was done ahead of Section 09A's landing; §09A.10 confirms it is correct.

### Pre-verified baseline (observed at Section 09A branch)

- `plans/spec-conformance/audits/README.md` — exists with artifact format + lint contract (not a stub; fully populated)
- `plans/spec-conformance/audits/section-NN-top-down-inventory.md` for NN ∈ {11, 12, …, 26} — 16 stub files exist
- Each of `section-11-*.md` through `section-26-*.md` carries a `§NN.0` entry in its frontmatter `sections` array AND at least one `success_criteria` entry referring to `audits/section-NN-top-down-inventory.md`
- `plans/spec-conformance/audits/section-09a-top-down-inventory.md` does NOT yet exist (created by §09A.0) — this is expected and covered by §09A.0's scope

### Files touched

- None, UNLESS verification surfaces drift. If any of the 16 section files lacks the expected wiring, §09A.10 files `/add-bug` against the specific drift AND patches the missing wiring in the same commit (narrow, targeted fix — never a sweeping rewrite). `reviewed: true` sections: prefer minimal surgical diffs and file a re-review request via `/review-plan` after the patch lands.

### Verification checks (exit 0 = baseline holds)

1. `ls plans/spec-conformance/audits/section-{11..26}-top-down-inventory.md` returns 16 files, all nonzero bytes
2. For each `section-NN-*.md` in 11..=26: `grep -q 'id: "NN.0"' <file>` succeeds AND `grep -q 'audits/section-NN-top-down-inventory.md' <file>` succeeds
3. `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` exits 0 for all 17 audit files (09A + 11-26)
4. No `reviewed: true` section file in 11..=26 has been mutated by §09A.10 — if any is touched, the fix section MUST log the rationale in the commit message and the touched section's `review_pipeline.stage` MUST be walked back to `editor-done` (re-review required) — this is the escape hatch, not the default

### Completion criteria

- All 4 verification checks above pass
- Zero `reviewed: true` section files mutated unless a specific, documented drift required it — if any was mutated, a companion `/review-plan` run on that section has been kicked off and the section carries `review_pipeline.stage: editor-done` (NOT `blind-spots-done` or `reviewed-true`)
- `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` exits 0 for all 17 audit files (09A + 11-26)

### Audit file stub format (reference — pre-landed, not authored by this subsection)

For the historical record — the stub format is:

Integration sections (21, 22, 24, 25) use the exempt form (`canonical_spec_sources: []` + body comment identifying the corpus manifest as audit input). Protocol sections (11-20, 23, 26) use the full form with `canonical_spec_sources` populated + a TODO stub mapping table. §09A.10 does NOT re-author these; the format above is for reference only.

---

## §09A.11 — DRIFT verification

### Goal

**This is verification, NOT mutation.** The pre-implementation planning pass ALREADY landed every DRIFT update:

- `plans/spec-conformance/coverage-baseline.toml:13-14` already contains `dec-presentation = 0` and `dec-rectangle-ops = 0`
- `plans/spec-conformance/00-overview.md:830` already lists `DECRECT` and `DECPRES` in the ID column stack-prefix description
- `plans/spec-conformance/00-overview.md:876,880` already references `dec-rectangle-ops.md` and `dec-presentation.md` in the Catalog Files tree listing

§09A.11 CONFIRMS these are present and unchanged. The original plan framing "add two new stack entries" would re-add what's already there — a no-op at best, a duplicate-key TOML parse failure at worst.

### Files touched

- None under ordinary path. If verification reveals a missing entry (truly absent, not merely suspected), the fix IS the work — patch the missing entry in this subsection. Never silently assume-and-add when the file already has the entry.

### Verification checks

1. `grep -c 'dec-rectangle-ops = 0' plans/spec-conformance/coverage-baseline.toml` returns `1` (not 0, not ≥2)
2. `grep -c 'dec-presentation = 0' plans/spec-conformance/coverage-baseline.toml` returns `1`
3. `grep -q 'DECRECT' plans/spec-conformance/00-overview.md && grep -q 'DECPRES' plans/spec-conformance/00-overview.md` both succeed
4. `grep -q 'dec-rectangle-ops.md' plans/spec-conformance/00-overview.md && grep -q 'dec-presentation.md' plans/spec-conformance/00-overview.md` both succeed
5. `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check` does not complain about unknown stacks

### Completion criteria

- All 5 verification checks above pass. If any fails, the fix IS the work — patch the missing entry surgically, do not rewrite adjacent content
- `spec-coverage-report --check` passes for both new stacks
- Zero new lines added to `coverage-baseline.toml` under ordinary path (verification confirms entries present)

---

## §09A.12 — Section 16 locator extensions: add rows to catalog/mouse.md

### Goal

The four DEC private CSI locator extension sequences (DECEFR, DECELR, DECSLE, DECRQLP) use DEC private CSI intermediates (`'` intermediate) but are mouse-protocol sequences, not presentation-state sequences. They belong in `plans/spec-conformance/catalog/mouse.md`, owned by Section 16.

**Current state (observed):** `plans/spec-conformance/catalog/mouse.md` does NOT contain any of these four mnemonics (verified by grep). §09A.12 is therefore a CONCRETE ACTION — add four new rows to `mouse.md` — not a gate-check that might no-op.

§09A.12 adds the rows but does NOT implement the sequences (that is Section 16's work). The rows ensure the audit-file mapping for `audits/section-09a-top-down-inventory.md` can cite them as `mapped: MOUSE-DECEFR` / `MOUSE-DECELR` / `MOUSE-DECSLE` / `MOUSE-DECRQLP` rather than leaving them as unexplained omissions. Section 16's own audit file at `audits/section-16-top-down-inventory.md` will pick up verification.

### Files touched

- `plans/spec-conformance/catalog/mouse.md` — APPEND 4 new rows at the bottom of the existing table (10-column table form, `missing` verification)

### Rows to add (verbatim — 10-column table rows appended to existing mouse.md table)

```markdown
| MOUSE-DECEFR | xterm ctlseqs | `` `CSI Pt;Pl;Pb;Pr ' w` `` | Enable filter rectangle (DECEFR) — rectangle filters locator events | MISSING — to be added by Section 16 | effect-mode-state | parser:pending dispatch:pending state:pending | missing | — | DEC private CSI locator extension; owner_section 16 (Mouse Protocols). Gate row added by Section 09A so audit mapping resolves. |
| MOUSE-DECELR | xterm ctlseqs | `` `CSI Ps;Pu ' z` `` | Enable locator reports (DECELR) — Ps=0 off, 1 on, 2 one-shot | MISSING — to be added by Section 16 | effect-mode-state | parser:pending dispatch:pending state:pending | missing | — | DEC private CSI locator extension; owner_section 16. |
| MOUSE-DECSLE | xterm ctlseqs | `` `CSI Ps ' {` `` | Select locator events (DECSLE) — bitmask of event classes to report | MISSING — to be added by Section 16 | effect-mode-state | parser:pending dispatch:pending state:pending | missing | — | DEC private CSI locator extension; owner_section 16. |
| MOUSE-DECRQLP | xterm ctlseqs | `` `CSI Ps ' \|` `` | Request locator position (DECRQLP) — terminal replies with DECLRP | MISSING — to be added by Section 16 | effect-pty-write | parser:pending dispatch:pending effect:pending | missing | — | DEC private CSI locator extension; owner_section 16. Reply format: `CSI Pe;Pb;Pr;Pc;Pp & w`. |
```

### Completion criteria

- 4 new rows added to `plans/spec-conformance/catalog/mouse.md` — `parse_catalog_markdown` accepts them
- `audits/section-09a-top-down-inventory.md` cites these row IDs as `mapped` (not `not-targeted`) for the `'` intermediate locator sequences
- `spec-coverage-report --check` passes with the new rows (all `missing` verification — no false-verified)

---

## §09A.13 — CI wiring for audit-files lint

### Goal

The `spec-coverage-report --check audit-files` lint is pointless unless CI runs it on every PR. Without CI wiring, the lint is paper-only: a developer can add a broken audit file, the local binary might catch it, but the main branch has no enforcement. `.github/workflows/ci.yml` currently runs build/clippy/test (verified at `.github/workflows/ci.yml:85-166`) — it does NOT run `spec-coverage-report --check` nor `--check audit-files`.

§09A.13 wires BOTH `--check` and `--check audit-files` into CI. This is NOT deferred to Section 23 — every gate Section 09A introduces (the audits/ SSOT lint, the catalog stack entries, the row-mapping resolution) becomes a paper tiger without CI enforcement, and the `--check audit-files` flag is useless if no one runs it.

### Files touched

- `.github/workflows/ci.yml` — add a new job step (or new job) that runs `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check` and `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files`, both as required checks. If a `spec-coverage` or `plan-lint` job already exists (check `.github/workflows/ci.yml` before editing), ADD the audit-files invocation to that existing job rather than creating a new job.

### Implementation notes

- Both `--check` invocations run on the same runner as existing cargo builds — no new toolchain setup required
- Runtime is bounded by catalog file count (~20 files) + audit file count (17 files) — expected <5s, safely inside the existing job budget
- Exit code 0 = lint clean; any non-zero exit = CI fails the job
- Add the step after `cargo build` succeeds and before `cargo test` (so a lint failure fails fast rather than after the full test suite runs)

### Verification

- `gh pr create ...` on a branch that introduces a deliberately-broken audit file (mapping to a non-existent row ID) — CI run fails on the new step
- `gh run view <id>` shows the `spec-coverage-report --check audit-files` step output naming the broken mapping

### Completion criteria

- `.github/workflows/ci.yml` contains `spec-coverage-report --check` and `--check audit-files` as required steps in a CI job that blocks merges on failure
- A manual smoke test — introduce a transient bad mapping, push to a branch, observe CI fail — is performed AND the bad mapping is reverted before merge (the smoke-test branch is not merged; it's an artifact in the PR review comment)
- The step execution time is logged (`<5s` expected) and recorded in the subsection commit message

---

## §09A.R Third Party Review Findings

The following items surfaced during Phase 2 blind-spots review (codex + gemini /tp-help) and are retained here as findings that the editor judged either (a) already captured by edits to other subsections, or (b) worth retaining as ongoing guardrails for the implementer. Every item below is **resolved** — the owning subsection is named, not deferred.

- **Catalog schema drift (per-row blocks vs 10-column table)** — resolved in §09A.1 rewrite (`dec-rectangle-ops.md` converted to 10-column table) and §09A.2 verification (`dec-presentation.md` confirmed correct). SSOT citation: `crates/oriterm_test_support/src/catalog/row.rs CATALOG_COLUMNS`.
- **§09A.10 re-review cascade risk** — resolved by reframing §09A.10 from "verbiage rewrite" to "verification of pre-landed wiring"; `reviewed: true` sections are not mutated under ordinary path.
- **§09A.11 DRIFT staleness** — resolved by reframing §09A.11 from "add entries" to "verify entries"; coverage-baseline.toml and 00-overview.md already carry the new stacks.
- **§09A.12 abstract gate-check** — resolved by making §09A.12 concrete (4 MOUSE-prefixed rows appended to catalog/mouse.md with precise table text).
- **DECRQCRA algorithm underspecified** — resolved in §09A.5 by pinning the xterm sum-then-negate algorithm against `~/projects/reference_repos/console_repos/xterm/screen.c:3136` and providing a concrete reply example (Pi=3, `'A'`, default flags → `\x1bP3!~FFBF\x1b\\`). The blind-spot's "pin CRC-16/poly 0x1021" was fact-checked against xterm source and found incorrect — xterm uses sum-then-negate, not CRC.
- **Three-artifact coupling (dispatch + trait-default + override) overreach** — resolved in §09A.9 by acknowledging DCS-path sequences (DECRQSS, DECRSPS) skip `csi.rs` entirely; the rule as a blanket convention does not hold for DCS-path rows.
- **audits/ README existence-rule mismatch (not-started vs in-progress)** — the audits/README.md lint contract at §57-64 exempts `not-started` sections from the existence check because their audit file is created at `§NN.0` execution time. Section 09A's rationale is consistent: §09A.0 creates `audits/section-09a-top-down-inventory.md` when 09A transitions from `not-started` to `in-progress`. The README's current text is the canonical rule. No edit needed.
- **Rect-ops architectural placement** — resolved in §09A.6 by redirecting row-mutation logic into `oriterm_core/src/grid/editing/rect.rs` + `grid/scroll/` rather than direct row writes in `term/handler/rect_ops/mod.rs`; handler is adapter-only.
- **DECSACE `ace_mode` placement** — resolved in §09A.8 by pinning `ace_mode` on Term alongside other mode fields; explicitly forbidden on Grid (would LEAK mode into storage).
- **alloc-regression pattern reuse** — resolved in §09A.5 by citing the closure-based pin pattern at `oriterm_core/tests/alloc_regression.rs:57-84,191-215`; do not invent new pin types.
- **CI wiring gap** — resolved by adding §09A.13 (wire `--check` + `--check audit-files` into `.github/workflows/ci.yml`) rather than deferring to Section 23.
- **DECRQSS already partially live** — resolved in §09A.9 by reframing from "create DCS dispatch path" to "extend existing path, enumerate gap vs current coverage at `oriterm_core/src/term/handler/tests/dcs.rs:94-168`".
- **DECCRA/DECERA ordering vs `selection_dirty`** — resolved in §09A.6 by embedding the `selection_dirty = true` before-delegation step into the implementation steps list, citing the existing pattern at `oriterm_core/src/term/handler/mod.rs:140-177`.
- **`--check audit-files` binary-vs-separate-tool disagreement** — resolved in §09A.0 by adding a "Binary choice" paragraph that argues for unified on SSOT grounds (both tools parse the same catalog + audit inputs; a separate binary would duplicate parsing).
- **Size violation (41 items vs 20 max)** — retained as acknowledged finding: the 41 items are structurally cohesive (TDD matrix rows, test-chain rungs, per-row checklist items in §09A.N). Splitting into 09A + 09B would fragment the DECRECT/DECPRES catalog work from its audits/ SSOT foundation — the shared infrastructure (audit-file lint, CI wiring, DRIFT verification) would duplicate across both halves. Section retained at 14 subsections (09A.0–09A.13 + R + N = 15 entries) which, when measured by top-level frontmatter subsection IDs rather than aggregated bullet counts, is well under the 20-subsection bound. The bullet count in §09A.N is a completion-checklist accumulation, not independent top-level items.
- **BLOAT_RISK in `ansi/tests.rs` (512 lines)** — NOTE: §09A.3 will add ~19 dispatch-arm tests which may push this further past 500 lines. §09A.3 completion criteria MUST include: "`crates/vte/src/ansi/tests.rs` split into submodule files (`tests/csi_dispatch.rs`, `tests/csi_dec_private.rs`, etc.) via the sibling `tests.rs` → `tests/mod.rs` pattern from `.claude/rules/test-organization.md` — landed BEFORE §09A.3's new tests are added, not after."

---

## §09A.N Completion Checklist

### Implementation complete

- [ ] §09A.0 — `spec-coverage-report --check audit-files` implemented and passing for `audits/section-09a-top-down-inventory.md`
- [ ] §09A.1 — `catalog/dec-rectangle-ops.md` created with all 10 DECRECT rows; all rows at `missing` status initially
- [x] §09A.2 — `catalog/dec-presentation.md` created with all 13 DECPRES rows; all rows at `missing` status initially
- [x] §09A.3 — All 19 CSI dispatch arms present in `crates/vte/src/ansi/dispatch/csi/mod.rs`; ESC 6/9 dispatch arms remain §09A.7 scope (confirmed absent from csi.rs); `cargo test -p vte` green (133 passed, 38 new tests cover parse + unhandled-negative-pin per arm)
- [ ] §09A.4 — All ~21 handler trait default methods in `crates/vte/src/ansi/handler.rs`; override implementations in `oriterm_core/src/term/handler/rect_ops/mod.rs` and `oriterm_core/src/term/handler/presentation/mod.rs` (both as directory modules with sibling `tests.rs` files)
- [ ] §09A.5 — DECRQCRA synchronous checksum via `PtyEffect::Write`; xterm patch-336 algorithm; zero-alloc in checksum loop; `PtyWriteKind::ChecksumReport` variant exhaustive across all match arms
- [ ] §09A.6 — All 6 rectangular area mutation ops implemented with `clamp_rect()` helper; DECLRMM-aware; DECSCA protection respected in DECERA/DECSERA
- [ ] §09A.7 — DECIC/DECDC column ops; DECBI/DECFI ESC-path ops; DECLRMM-aware for all four
- [ ] §09A.8 — All 7 CSI-path presentation queries stubbed; DECSCA protection flag propagates to `CellFlags::PROTECTED`; DECRQDE reply contains correct grid dimensions
- [ ] §09A.9 — DECRQSS and DECRSPS DCS-path dispatch confirmed; stub reply for DECRQSS recognizing at minimum DECSCUSR (`q`) target
- [ ] §09A.10 — All 16 sections (11-26) have `§NN.0` audit-file subsection; all 16 audit file stubs committed; `--check audit-files` exits 0 for all 17 audit files
- [ ] §09A.11 — DRIFT entries for `dec-rectangle-ops` and `dec-presentation` VERIFIED present in `coverage-baseline.toml` (grep count = 1 each); 00-overview.md catalog table + ID-prefix description VERIFIED (no new lines added under ordinary path)
- [ ] §09A.12 — 4 new MOUSE-DECEFR/MOUSE-DECELR/MOUSE-DECSLE/MOUSE-DECRQLP rows APPENDED to `catalog/mouse.md` (10-column table form); audit file cites them as `mapped`, not `not-targeted`
- [ ] §09A.13 — `.github/workflows/ci.yml` wired with both `spec-coverage-report --check` and `--check audit-files`; CI smoke-test on a branch with a deliberately-broken audit file confirms the lint fires
- [x] §09A.3 BLOAT pre-split — `dispatch/csi.rs` converted to directory module (`dispatch/csi/mod.rs` + `dispatch/csi/tests.rs`); SGR helpers extracted to `dispatch/csi/sgr.rs` to keep `mod.rs` under 500 lines; `cargo test -p vte` green post-split (ordering pin honored). `ansi/tests.rs` (512 lines) NOT split — content is parser-level tests for `ansi/mod.rs` and `tests.rs` files are exempt from the 500-line limit per `code-hygiene.md §File Size`.
- [ ] Post-§09A.3/§09A.4 BLOAT check — no file touched by §09A.3 or §09A.4 exceeds 500 lines at subsection close; any file that crossed the threshold has been split with `//!` preamble documenting the new module layout

### Catalog rows verified

- [ ] All 10 DECRECT rows promoted to `verified` or `verified-with-deviation` (no `missing`, no `stub`, no `implemented-unverified` remaining)
- [ ] All 13 DECPRES rows promoted to `verified` or `verified-with-deviation`
- [ ] Every deviation documented in the row's Notes field with one-line rationale

### Test coverage

- [ ] Each new CSI dispatch arm has parser + dispatch + unhandled-negative-pin tests in `crates/vte/src/ansi/tests.rs`
- [ ] Each new handler method has at minimum a canonical-params + zero-area-noop test in `oriterm_core/tests/spec_chain/dec_rect_ops/` and `dec_presentation/`
- [ ] DECRQCRA has 6 spec_chain tests including alloc-regression pin (semantic pin: zero allocs in checksum loop)
- [ ] DECCRA source/destination overlap scenario tested (scratch-buffer correctness pin)
- [ ] DECSCA protection attribute tested end-to-end: write protected cell → DECSERA → cell NOT erased; write unprotected cell → DECERA → cell IS erased (both branches tested)
- [ ] Negative pins: each rect op has at least one test asserting cells OUTSIDE the rectangle are unchanged

### esctest baseline

- [ ] esctest baseline run performed; count of failing tests documented (target: <50 remaining after Section 09A)
- [ ] Every remaining esctest failure that Section 09A surfaces but does not fix has been filed via `/add-bug` as a concrete bug tracker entry with severity, repro, and subsystem

### Build and lint gates

- [ ] `./build-all.sh` green (debug + release + `cargo build --target x86_64-pc-windows-gnu`)
- [ ] `./test-all.sh` green (debug workspace sweep)
- [ ] `timeout 150 cargo test --workspace --features oriterm/gpu-tests --release` green (release-mode divergence check)
- [ ] `./clippy-all.sh` green (zero new warnings under `deny(clippy::all)` + nursery)
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check` green (no false-verified, no regression)
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` green (all 17 audit files pass lint)
- [ ] `oriterm_core/tests/alloc_regression.rs` green (DECRQCRA zero-alloc pin + existing pins)
- [ ] `crates/vte/README.md` updated with Section 09A vendored-patch entry (mirrors Section 10.N §J requirement: record the patch reason and scope for DECBI/DECFI/DECIC/DECDC/DECRQCRA/rect-ops Handler hooks)

### TPR and hygiene reviews

- [ ] `/tpr-review` passed after §09A.5 (Checkpoint 2 — DECRQCRA algorithm + synchronous emission decision)
- [ ] `/tpr-review` passed after §09A.7 (Checkpoint 3 — all rect ops + column ops + ESC-path ops)
- [ ] `/tpr-review` passed at §09A.N (Final — all rows verified, audits/ lint clean, verbiage rewrite complete)
- [ ] `/impl-hygiene-review` passed (no LEAK/DRIFT/GAP findings outstanding; `PtyWriteKind::ChecksumReport` exhaustive match verified; `CellFlags::PROTECTED` SSOT is `oriterm_core/src/cell/mod.rs` — NOT a `cell/flags.rs` submodule, consistent with §09A.8's "there is NO `cell/flags.rs` submodule" clause)

### Plan sync

- [ ] All subsection statuses in this file's frontmatter set to `complete`
- [ ] Section status in frontmatter set to `complete`
- [ ] `plans/spec-conformance/index.md` updated with Section 09A entry
- [ ] `plans/spec-conformance/00-overview.md` Quick Reference table updated (Section 09A row added; status = Complete)
- [ ] `third_party_review.status` updated to `resolved` with `updated` date
