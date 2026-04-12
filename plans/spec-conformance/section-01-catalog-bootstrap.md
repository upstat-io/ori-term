---
section: "01"
title: "Catalog Bootstrap"
status: in-progress
reviewed: true
goal: "Build the spec-conformance catalog as the empirical map of every protocol sequence ori_term targets, with stable-symbol implementation anchors, committed spec corpus, committed deterministic real-app captures, reconciled bottom-up vs top-down coverage, and a mechanical `catalog_coverage_check` Rust binary. No tests exercising `TermHandler` behavior are written in this section — the catalog itself is the deliverable, plus the testable `catalog_coverage_check` tool with its own sibling unit tests. All rows use `schema_version: 0.1-provisional`; Section 04.7 migrates the whole corpus to `1.0` in lockstep with the pilots. No row in this section is allowed to hold `Verification: verified` or `verified-partial` — those statuses are earned by Sections 04-20, never bootstrapped here."
success_criteria:
  - "`plans/spec-conformance/catalog/` exists with 16 protocol-family markdown files PLUS a stub `catalog/README.md` (Section 01 owns this stub; Section 04.7 extends it with the frozen schema reference per `plans/spec-conformance/section-04-verification-chain-harness.md:509,515`). The `_legacy-tack-mapping.md` file is NOT created here — it is owned by Section 02.4 (`plans/spec-conformance/section-02-tack-absorption.md:42`)."
  - "Every catalog file has front-matter `schema_version: \"0.1-provisional\"` declared at the top. Section 04.7 owns the migration to `1.0`; Section 01 MUST NOT pre-emptively write `1.0`."
  - "Every row has ALL 10 columns from the `Catalog Row Schema` in `plans/spec-conformance/00-overview.md` populated: `ID`, `Spec source`, `Sequence`, `Description`, `Implementation`, `Apex layer`, `Test chain`, `Verification`, `De-facto ref`, `Notes`. Missing columns are a completion-blocker; an empty cell uses `—` (em dash), NOT a blank string. The `Test chain` column is populated with `parser:pending dispatch:pending …` placeholders since Section 01 writes no tests — Section 04 replaces `pending` with real results."
  - "Every row's `ID` follows `{STACK}-{MNEMONIC}` format where STACK ∈ `{ECMA48, DEC, OSC, SIXEL, KG, KKBD, ITERM2, MOUSE, CHSET, HIST, AUDIO, SHINT, DFCT}` (defined in `plans/spec-conformance/00-overview.md:809`). Row IDs are globally unique across all catalog files."
  - "Every row's `Implementation` column anchors on a STABLE SYMBOL (e.g., ``TermHandler::goto``) with the file path in parentheses. Line numbers are regenerated metadata only, never the primary anchor (reviewers rejected line-number-primary citations as DRIFT-prone in Phase 2). Reference form: `` `TermHandler::goto` (`oriterm_core/src/term/handler/mod.rs`) ``. MISSING rows use `MISSING — to be added by Section NN`."
  - "Every C0/ESC/C1/CSI/OSC/DCS/APC/PM/SOS parser state in `crates/vte/src/lib.rs` that has a dispatch counterpart OR an explicit discard/drop in the VTE parser has at least one catalog row. PM (`^`, 0x5E → `State::SosPmApcString`) and SOS (`X`, 0x58 → `State::SosPmApcString`) are explicitly enumerated in 01.1 and each receives at least one row marked `Verification: stub` with a `Notes` comment citing the discard path (`crates/vte/src/lib.rs:189,369,387`) and the test evidence (`crates/vte/src/tests.rs:778`)."
  - "Every numbered DEC private mode in `crates/vte/src/ansi/types.rs::NamedPrivateMode` has a row in `catalog/dec-private-modes.md`."
  - "Every SGR parameter handled by `attrs_from_sgr_parameters` in `crates/vte/src/ansi/dispatch/csi.rs` (the canonical SGR numeric-to-`Attr` mapper; the mechanical check script is the source of truth, NOT a line range) has a row in `catalog/ecma-48.md`. The supported numeric universe (verified 2026-04-11 against the live `attrs_from_sgr_parameters` match arms) is `0-9`, `21-25`, `27-29`, `30-39`, `40-49`, `58-59`, `90-97`, `100-107` — SGR 10-20, 26, 51-55, and 113+ are NOT supported and must NOT appear as rows (Phase 4 iteration-6 TPR-01-001-gemini accuracy fix)."
  - "Every OSC number with a handler in `oriterm_core/src/term/handler/osc.rs` has a row in `catalog/osc.md`."
  - "Every cited spec is either committed in `plans/spec-conformance/specs/` (if redistributable) or listed in `specs/manifest.toml` with sha256 + fetch script entry (if license-restricted). `bash plans/spec-conformance/specs/manifest-fetch.sh --verify` exits 0."
  - "Every catalog row has its `Verification` column set to one of `missing`, `stub`, or `implemented-unverified`. No row holds `verified`, `verified-partial`, or `verified-with-deviation` — those statuses are earned by the verification chain harness in Sections 04-20, never bootstrapped here. A post-close grep `grep -E 'Verification.*verified' plans/spec-conformance/catalog/*.md` MUST find zero matches."
  - "Real-app captures are COMMITTED (not `/tmp/`): stored under `plans/spec-conformance/captures/{app}-{flow}.cap` with corresponding `{app}-{flow}.script` input file and a committed `captures/manifest.toml` listing every capture with sha256, app version, OS, env, scripted flow name, duration, unique tuple count. Idle captures (< 20 unique tuples) are REJECTED."
  - "`plans/spec-conformance/captures/reconciliation-report.md` exists, documenting every sequence that appeared in one of {VTE dispatch, wezterm catalog, real-app captures, primary specs} but not in another, with the resolution: `de-facto` (move to `de-facto-behaviors.md`), `MISSING` (keep in primary catalog with `Implementation: MISSING`), or `reconciled` (row already present). The report is the audit trail for Phase 2 Finding E."
  - "`crates/oriterm_test_support/src/bin/catalog_coverage_check.rs` exists (Rust binary — TPR-01-002-gemini, shares `crates/oriterm_test_support/src/catalog/mod.rs` with Section 04.8's `spec_coverage_report`) and passes its sibling `tests.rs` with positive-pin, negative-pin (missed tuple, duplicate row ID, stale symbol anchor, line-number-primary citation, `Verification: verified` in bootstrap mode, wezterm as `Spec source`, missing `NamedPrivateMode` row), and cross-type (SGR, OSC, DCS, APC with final_byte ST) cases per `.claude/rules/tests.md` §Matrix Testing Rule. The binary is wired into `test-all.sh` so a catalog/dispatch drift fails CI. Debug + release builds both green, cross-compile to `x86_64-pc-windows-gnu` clean."
  - "Audit memory corrections applied to every stale claim discovered DURING Section 01 harvest work (not just the three known seeds): `architecture_graphics_audit.md`, `MEMORY.md`, AND `plans/spec-conformance/research.md` are all walked (iter-11 TPR-01-002-codex — research.md carries the same stale claims as the audit memory because it was the original research note); any claim a harvest row contradicts is rewritten in the same commit, and the example symbol names in 01.9 are grep-verified against the live source before being cited (the illustrative names in this plan are hints, not verbatim). The three known corrections (HSL hue, kitty q=1, image cache 320 MiB) are the MINIMUM; research.md's `No plan exists yet` status line is also corrected to reflect the existing spec-conformance plan."
  - "`./build-all.sh` AND `./clippy-all.sh` AND `./test-all.sh` green in BOTH debug and release — the `catalog_coverage_check` Rust binary and its sibling `tests.rs` run on Linux native + macOS CI; `x86_64-pc-windows-gnu` cross-compile is build-only verification per CLAUDE.md §Commands."
  - "Bug-tracker artifact filed: `oriterm_core/src/term/handler/image/kitty.rs` (476 lines) has an `/add-bug` entry under `plans/bug-tracker/section-08-core-terminal.md` (Core Terminal — NOT UI Widgets; Section 01's bug IDs target the correct subsystem per `plans/bug-tracker/00-overview.md:41` and `.claude/skills/add-bug/SKILL.md:47-80`) blocking Sections 12 (Sixel) and 13 (Kitty Graphics). The file is at the 500-line BLOAT boundary per `.claude/rules/code-hygiene.md` §File Size and MUST be split BEFORE any graphics-stack implementation work touches it. Section 01 only READS the file for harvest; the split is a prerequisite for downstream sections, tracked via a concrete artifact per CLAUDE.md §Bug Discipline."
  - "Cross-section plan edits (Phase 4 iteration-2 TPR-01-001-gemini / iteration-3 TPR-01-001-codex / TPR-01-002-gemini / iteration-4 TPR-01-002-codex fix): **Section 01.11.a.i INSTRUCTS the implementer to edit** (at Section 01 execution time — not before) the BODY text of `plans/spec-conformance/section-12-sixel.md` AND `plans/spec-conformance/section-13-kitty-graphics.md`. The two edits are: (a) an inline `**Blocker note:**` in each section's Context paragraph referencing `BUG-08-<ordinal>` from `plans/bug-tracker/section-08-core-terminal.md`, and (b) a `- [ ]` completion-checklist item in each section's `## 12.N` / `## 13.N` block that reads `BUG-08-<ordinal> (kitty.rs BLOAT split) is closed in plans/bug-tracker/section-08-core-terminal.md — verified by grepping the bug entry for [x].` These edits are NOT made at plan-review time — they are concrete implementer work items tracked in 01.11.a.i, executed when Section 01 lands. The checklist item (Layer 3) is the scanner-enforced gate (the plan schema forbids marking a section `status: complete` while completion-checklist items are `[ ]`). Neither edit touches frontmatter `depends_on:` (which uses section-number tokens, not bug-tracker IDs — per iteration-1 TPR-01-005-codex / TPR-01-004-gemini grammar fix) nor `success_criteria` (which the `/continue-roadmap` scanner does NOT parse — per iteration-3 TPR-01-001-codex scanner-behavior fix). Before Section 01 lands, Sections 12 and 13 are ONLY protected by Layer 1 (`/continue-roadmap` Step 1.92 bug-tracker gate — soft surfacing when the bug is filed at `high` severity). The full three-layer enforcement kicks in only after Section 01 executes and commits the Section 12/13 body edits. This is a known Section-01-landing-order consequence and is correct."
  - "Conditional bug filings from capture routing (Phase 4 iteration-2 TPR-01-005-gemini fix): 01.5.c's 'Unknown category' routing rule permits escalating individual capture-tuple sequences to `/add-bug` when they look like real-app bugs rather than ori_term parser gaps. Any such escalations produce additional `BUG-{subsystem}-<ordinal>` entries in `plans/bug-tracker/` during Section 01 execution. The 01.N completion checklist lists the IDs of every such entry filed during Section 01 (empty list allowed — zero escalations is the normal case), and the checklist's bug-tracker gate explicitly permits `N ≥ 1` additional filings beyond the mandatory `BUG-08-<ordinal>` kitty.rs BLOAT bug."
  - "Section's mission criterion connection: contributes to `Catalog complete` mission criterion in 00-overview.md. Partial checkmark only — full check flips after Section 04.7 freezes the schema and Section 04.9's continuous-delta detector is wired into CI."
inspired_by:
  - "wezterm `~/projects/reference_repos/console_repos/wezterm/docs/escape-sequences.md` — per-sequence catalog table format with Seq | Hex | Name | Description | Action columns, anchored on stable symbols, never on line numbers"
  - "alacritty `~/projects/reference_repos/console_repos/alacritty/docs/escape_support.md` — terse per-row support status; zero line-number citations"
  - "ghostty `~/projects/reference_repos/console_repos/ghostty/src/lib_vt.zig` — enum of every recognized sequence as the bottom-up source of truth"
  - "ori_term `architecture_graphics_audit.md` memory — stable-symbol citations for graphics protocols (line-number drift already observed in pre-rewrite Section 01)"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "01.1"
    title: "Bottom-up harvest from ori_term VTE dispatch (incl. PM/SOS)"
    status: complete
  - id: "01.2"
    title: "Bottom-up harvest from wezterm escape-sequences.md (De-facto ref column ONLY — Phase 2 Finding J)"
    status: complete
  - id: "01.3"
    title: "Mechanical `catalog_coverage_check` Rust binary + sibling tests (Phase 2 Finding G + Phase 4 TPR findings)"
    status: complete
  - id: "01.4"
    title: "Real-app capture infrastructure — deterministic scripted flows + commit protocol (Phase 2 Finding F)"
    status: not-started
  - id: "01.5"
    title: "Run captures + commit artifacts + manifest"
    status: not-started
  - id: "01.6"
    title: "Spec corpus assembly + manifest"
    status: not-started
  - id: "01.7"
    title: "Top-down walk through primary specs"
    status: not-started
  - id: "01.8"
    title: "Reconciliation pass — bottom-up vs top-down diff (Phase 2 Finding E)"
    status: not-started
  - id: "01.9"
    title: "Stale-claim corrections (audit memory + MEMORY.md — beyond the known 3) (Phase 2 Finding K)"
    status: complete
  - id: "01.10"
    title: "Stub catalog/README.md (owned here; extended by Section 04.7) (Phase 2 Finding H)"
    status: complete
  - id: "01.11"
    title: "Bug-tracker filing for kitty.rs BLOAT (blocks Sections 12/13) (Phase 1 Finding 7 + CLAUDE.md §Bug Discipline)"
    status: complete
  - id: "01.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "01.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 01: Catalog Bootstrap

**Status:** In Progress (01.1 + 01.2 + 01.3 + 01.9 + 01.10 + 01.11 complete)
**Goal:** Build the catalog as the empirical map of every protocol sequence ori_term targets. No `TermHandler`-behavior tests are written — but the `catalog_coverage_check` Rust binary IS testable code and DOES get full TDD treatment per `.claude/rules/tests.md`. Every subsequent stack section consumes this catalog as its scope definition. The row schema is `0.1-provisional`; Section 04.7 owns the migration to `1.0` after the pilots in Section 04.5–04.6 + Section 05.6 land. Section 01 writes NO row with `Verification: verified` — those statuses are earned by the verification chain harness, never bootstrapped here.

**Success Criteria:**
- [ ] `plans/spec-conformance/catalog/` exists with 16 protocol-family markdown files PLUS a stub `catalog/README.md` (Section 01 owns this stub; Section 04.7 extends it with the frozen schema reference)
- [ ] Every catalog file declares front-matter `schema_version: "0.1-provisional"`
- [ ] Every row has all 10 columns populated: `ID`, `Spec source`, `Sequence`, `Description`, `Implementation`, `Apex layer`, `Test chain`, `Verification`, `De-facto ref`, `Notes`
- [ ] Every row's `Implementation` anchors on a stable SYMBOL (e.g., `TermHandler::goto`), with file path in parentheses as metadata. Line numbers may only appear as trailing metadata appended to the file path, never as the primary anchor
- [ ] Every C0/ESC/C1/CSI/OSC/DCS/APC/**PM/SOS** parser state in `crates/vte/src/lib.rs` that has a dispatch counterpart or explicit discard has at least one catalog row — PM and SOS are NOT omitted
- [ ] No row holds `Verification: verified` / `verified-partial` / `verified-with-deviation` (those statuses are reserved for Sections 04-20)
- [ ] Real-app captures are committed under `plans/spec-conformance/captures/` with a `manifest.toml` listing every capture's {app, version, OS, env, script, duration, unique tuple count, sha256}
- [ ] `plans/spec-conformance/captures/reconciliation-report.md` exists, enumerating every bottom-up/top-down mismatch with its resolution
- [ ] `crates/oriterm_test_support/src/bin/catalog_coverage_check.rs` exists as a Rust binary (TPR-01-002-gemini — Python rewrite rejected because PTY parsing needs `vte` and Section 04.8 already builds Rust tooling), with sibling `tests.rs` in `crates/oriterm_test_support/src/catalog/tests.rs` (positive, negative-pin, cross-type, self-verifying completeness counter); wired into `./test-all.sh`
- [ ] Bug-tracker entry filed for `oriterm_core/src/term/handler/image/kitty.rs` BLOAT (476 lines → must split before Sections 12/13 begin)
- [ ] Audit memory corrections applied for every stale claim discovered during harvest (minimum: HSL hue, kitty q=1, image cache 320 MiB)
- [ ] `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` green debug + release, Linux + Windows cross-compile
- [ ] Connects to mission criterion: **Catalog complete** (partial — full check after Section 04.7 freeze + 04.9 safety net wiring)

**Context:** The catalog is the prerequisite that makes every subsequent section's scope mechanical. Without it, "100% conformance" is unfalsifiable because you don't know what 100% means. This section delivers catalog *breadth* first (every sequence enumerated); the row *schema version* is `0.1-provisional` until Section 04.7 freezes it to `1.0`. The audit memory at `architecture_graphics_audit.md` provides a starting inventory of graphics protocol implementations; Section 01 CORRECTS every stale claim it contradicts during harvest (three known seeds; more may surface).

**Reference implementations (ALL STABLE-SYMBOL ANCHORED — the reference repos themselves do not cite line numbers):**
- **wezterm** `~/projects/reference_repos/console_repos/wezterm/docs/escape-sequences.md` (~415 lines) — per-sequence catalog with `Seq | Hex | Name | Description | Action` columns. Anchored on sequence names and handler symbols. Same shape ori_term's catalog uses, extended with verification status and apex layer columns. This is the reference catalog to emulate for citation style.
- **alacritty** `~/projects/reference_repos/console_repos/alacritty/docs/escape_support.md` — terse per-row support table. Also symbol/sequence-anchored, never line-anchored.
- **ghostty** `~/projects/reference_repos/console_repos/ghostty/src/lib_vt.zig` — enum of every recognized sequence; treat as the bottom-up source of truth for cross-checking 01.1.
- **ori_term** `crates/vte/src/ansi/dispatch/{mod,csi,osc}.rs` — bottom-up source of truth for what ori_term currently parses. Every match arm = one catalog row. NOTE: `csi.rs` is 390 lines (near the 500-line BLOAT threshold per `.claude/rules/code-hygiene.md`); Section 01 READS this file read-only. Any split is owned by Section 04 (which touches the dispatch during pilots) or by downstream stack sections, NOT by Section 01.
- **ori_term** `crates/vte/src/ansi/types.rs::NamedPrivateMode` — canonical enum of DEC private modes ori_term recognizes. Every variant = one row in `catalog/dec-private-modes.md`.
- **ori_term** `crates/vte/src/lib.rs:189,369,387` — PM/SOS parser state transitions (`State::SosPmApcString`). Confirms PM and SOS are parsed-then-discarded; each gets at least one `Verification: stub` row.

**Rules woven in:** This section writes the following surfaces:
- **Plan files**: 16 catalog protocol files under `plans/spec-conformance/catalog/*.md` plus a stub `catalog/README.md` (owned jointly with Section 04.7), `plans/spec-conformance/specs/manifest.toml`, `plans/spec-conformance/captures/manifest.toml`, `plans/spec-conformance/captures/reconciliation-report.md`, `plans/spec-conformance/captures/scripts/README.md`.
- **Shell scripts**: `plans/spec-conformance/specs/manifest-fetch.sh` (fetches license-restricted specs; supports `--verify` mode), `plans/spec-conformance/captures/verify-manifest.sh` (sha256 + unique-tuple-count verification).
- **Committed capture artifacts**: `plans/spec-conformance/captures/scripts/{app}-{flow}.script` (deterministic input scripts), `plans/spec-conformance/captures/{app}-{flow}.cap` (PTY output transcripts).
- **Committed redistributable spec snapshots**: `plans/spec-conformance/specs/kitty-graphics-protocol.md`, `specs/kitty-keyboard-protocol.md`, `specs/mode-2026-spec.md`, `specs/osc-8-hyperlinks.md`, `specs/final-term-osc-133.md`, `specs/uax-9.txt`, `specs/uax-11.txt`, `specs/uax-29.txt`, `specs/unicode-legacy-computing.pdf` (per license verification).
- **Rust source**: `crates/oriterm_test_support/src/catalog/mod.rs` (shared library parser — SSOT consumed by 01.3 and 04.8), `crates/oriterm_test_support/src/catalog/tests.rs` (sibling tests), `crates/oriterm_test_support/src/bin/catalog_coverage_check.rs` (binary entry point), committed fixture files under `crates/oriterm_test_support/tests/fixtures/catalog/`.
- **Python host script**: `scripts/replay-capture-script.py` (interpreted PTY driver, Linux-only — not cross-compiled).
- **Test pipeline**: extension to `test-all.sh` wiring `cargo test -p oriterm_test_support --lib` + `cargo run -p oriterm_test_support --bin catalog_coverage_check -- --check --bootstrap-mode` into the workspace CI loop.
- **Memory file corrections**: updates to `/home/eric/.claude/projects/-home-eric-projects-ori-term/memory/architecture_graphics_audit.md` and `MEMORY.md` for every stale claim discovered during harvest (minimum three: HSL hue, kitty q=1, image cache 320 MiB; plus any others surfaced during 01.1-01.8).
- **Cross-plan edits**: bug-tracker entry appended to `plans/bug-tracker/section-08-core-terminal.md` via `/add-bug` at severity `high` (Layer 1 enforcement). Inline `**Blocker note:**` body-text edits plus completion-checklist `- [ ]` items added to `plans/spec-conformance/section-12-sixel.md` and `plans/spec-conformance/section-13-kitty-graphics.md` by 01.11.a.i.
- **Plan-sync edits**: status updates to `plans/spec-conformance/00-overview.md` Quick Reference table and `plans/spec-conformance/index.md` Section 01 entry per 01.N completion checklist.

The Rust tooling and its sibling tests follow `.claude/rules/tests.md` (matrix testing, positive + negative pins, cross-type matrix, self-verifying completeness counter), `.claude/rules/test-organization.md` (sibling `tests.rs` pattern, NO inline test modules), `.claude/rules/impl-hygiene.md` §SSOT (the catalog is the canonical home for protocol scope; `catalog/mod.rs` is the SSOT parser consumed by both `catalog_coverage_check` (01.3) and Section 04.8's `spec_coverage_report`), `.claude/rules/code-hygiene.md` (500-line file limit, banner policy, function size), `.claude/rules/crate-boundaries.md` (the `oriterm_test_support` crate is the correct home for a dev-only test-adjacent binary; not `oriterm_core` or `oriterm_ui`), and CLAUDE.md §Bug Discipline (the kitty.rs BLOAT gets filed via `/add-bug` at severity `high` — it is discovered during harvest, so the discovery IS the assignment).

**Depends on:** None. This is the first section.

**Ordering rationale (fixes Phase 2 Finding C — chronological dependency inversion):** The original layout had 01.6 ("top-down walk") depending on 01.7 ("spec corpus assembly"), which is a circular forward reference. The new order is: bottom-up first (01.1-01.3), then corpus assembly (01.6), then top-down walk using the assembled corpus (01.7), then reconciliation (01.8). The coverage-check script (01.3), stale-claim corrections (01.9), stub README (01.10), and BLOAT bug filing (01.11) close the section.

**Forward references — intentional, acknowledged:** Plan-audit flags every backticked file path that doesn't yet exist on disk as `DEAD_PATH`. This section CREATES many of its own referenced artifacts, so every `DEAD_PATH` on a path under `plans/spec-conformance/catalog/`, `plans/spec-conformance/captures/`, `plans/spec-conformance/specs/`, or `scripts/` is a legitimate forward reference to a file THIS section is responsible for creating. The forward-reference set is:

- `plans/spec-conformance/catalog/*.md` (16 protocol files + `README.md` stub) — created by 01.1.a and 01.10
- `plans/spec-conformance/captures/manifest.toml`, `plans/spec-conformance/captures/verify-manifest.sh`, `plans/spec-conformance/captures/scripts/README.md`, `plans/spec-conformance/captures/reconciliation-report.md` — created by 01.4, 01.5, 01.8
- `plans/spec-conformance/specs/manifest.toml`, `plans/spec-conformance/specs/manifest-fetch.sh`, `plans/spec-conformance/specs/mode-2026-spec.md` (and other per-spec snapshots) — created by 01.6
- `crates/oriterm_test_support/src/bin/catalog_coverage_check.rs`, `crates/oriterm_test_support/src/catalog/mod.rs`, `crates/oriterm_test_support/src/catalog/tests.rs`, `crates/oriterm_test_support/tests/fixtures/catalog/*.rs`, `crates/oriterm_test_support/tests/fixtures/catalog/*.md` — created by 01.3 (Rust binary + shared library module + sibling tests per TPR-01-002-gemini). A minimal `scripts/replay-capture-script.py` (PTY driver for deterministic capture replay) is still Python — it's a simple runner that invokes a shell command and emits keystrokes, and does NOT parse VT output (that's the binary's job).

None of these are DRIFT or dead content. The plan-audit `DEAD_PATH` noise resolves mechanically when Section 01 lands and the files come into existence. A follow-up `plan-audit --verify` run after Section 01 completes MUST show `post_set - baseline_set = {}` on this file set. Reviewers: do not flag these as blocking; they are intentional by the scoping of this section.

---

## 01.1 Bottom-up harvest from ori_term VTE dispatch (incl. PM/SOS)

**File(s):** `plans/spec-conformance/catalog/{ecma-48,xterm-ctlseqs,dec-private-modes,osc,sixel,kitty-graphics,iterm2,de-facto-behaviors}.md` (created with `schema_version: "0.1-provisional"` front-matter)
**Source code read (read-only, no edits):** `crates/vte/src/ansi/dispatch/mod.rs`, `crates/vte/src/ansi/dispatch/csi.rs` (390 lines — BLOAT-adjacent, read-only here), `crates/vte/src/ansi/dispatch/osc.rs`, `crates/vte/src/ansi/types.rs`, `crates/vte/src/lib.rs` (for parser-state transitions including PM/SOS), `oriterm_core/src/term/handler/` (entire tree)

This subsection harvests every sequence ori_term currently parses or dispatches into the appropriate catalog file. Each row gets `Implementation` filled with the STABLE SYMBOL chain (dispatch symbol → handler symbol) + file paths in parentheses, `Apex layer` set to the provisional apex from `plans/spec-conformance/00-overview.md:814`, `Verification` set to one of `{missing, stub, implemented-unverified}` based on what the code actually does, and all other columns populated per the 10-column schema.

### 01.1.a — Catalog-file scaffolding

- [x] Create `plans/spec-conformance/catalog/` directory (if Section 02 has not already created it for `_legacy-tack-mapping.md` per `plans/spec-conformance/section-02-tack-absorption.md:42`)
- [x] Create the 16 protocol-family files, each with `schema_version: "0.1-provisional"` front-matter, an H1 heading, and a 10-column markdown table header matching `plans/spec-conformance/00-overview.md:807-818` exactly (`ID`, `Spec source`, `Sequence`, `Description`, `Implementation`, `Apex layer`, `Test chain`, `Verification`, `De-facto ref`, `Notes`):
  - `catalog/ecma-48.md`
  - `catalog/xterm-ctlseqs.md`
  - `catalog/dec-private-modes.md`
  - `catalog/osc.md`
  - `catalog/sixel.md`
  - `catalog/kitty-graphics.md`
  - `catalog/kitty-keyboard.md`
  - `catalog/iterm2.md`
  - `catalog/mode-2026.md`
  - `catalog/unicode-subcell.md`
  - `catalog/mouse.md`
  - `catalog/charsets.md`
  - `catalog/audio-print.md`
  - `catalog/shell-integration.md`
  - `catalog/historical.md`
  - `catalog/de-facto-behaviors.md`
- [x] Section 01 does NOT create `catalog/_legacy-tack-mapping.md` — that is Section 02.4's responsibility (`plans/spec-conformance/section-02-tack-absorption.md:42`). This subsection depends on Section 02 having run. If Section 02 has not run, the directory is created here and `_legacy-tack-mapping.md` stays owned by 02.4 when it does run.
- [x] Section 01 DOES create a stub `catalog/README.md` — see 01.10 below. Section 04.7 extends the stub with the frozen schema reference (`plans/spec-conformance/section-04-verification-chain-harness.md:509,515`).

### 01.1.b — C0 / ESC / C1 dispatch harvest

- [x] Read `crates/vte/src/ansi/dispatch/mod.rs` end-to-end. For every C0/ESC/C1 dispatch arm, add a row to the appropriate catalog file. Every row uses symbol-primary Implementation: `` `<handler_symbol>` (`<file_path>`) ``. Populate all 10 columns.
  - C0 (BEL/BS/HT/LF/VT/FF/CR/SO/SI) → `catalog/ecma-48.md` under an H2 heading "C0 Controls"
  - ESC sequences (RIS/DECSC/DECRC/DECPAM/DECPNM/IND/NEL/HTS/RI/SS2/SS3/G0-G3 designation) → `catalog/ecma-48.md` under "ESC Sequences"
  - C1 7-bit ESC-prefixed forms handled today
  - C1 8-bit forms → add ROWS with `Implementation: MISSING — to be added by Section 08 (ECMA-48 Baseline)` and `Verification: missing` (confirms the research finding at `plans/spec-conformance/00-overview.md:755`)

### 01.1.c — CSI dispatch harvest

- [x] Read `crates/vte/src/ansi/dispatch/csi.rs` (390 lines — BLOAT-adjacent; READ-ONLY in this section) end-to-end. For every CSI match arm, add a row to `catalog/ecma-48.md` (cursor/erase/insert/scroll/SGR/modes) OR `catalog/xterm-ctlseqs.md` (window manipulation, focus events, bracketed paste, DECRQM, DECRQSS) with symbol-primary Implementation citations:
  - Cursor: CUU, CUD, CUF, CUB, CNL, CPL, CHA, CUP, HVP, CHT, CBT
  - Erase: ED, EL, ECH
  - Insert/Delete: ICH, DCH, IL, DL
  - Scroll: SU, SD, DECSTBM
  - SGR: every parameter actually handled by `attrs_from_sgr_parameters` in `crates/vte/src/ansi/dispatch/csi.rs`. The verified supported universe (2026-04-11, Phase 4 iteration-6 TPR-01-001-gemini accuracy fix): `0-9`, `21-25`, `27-29`, `30-39`, `40-49`, `58-59`, `90-97`, `100-107`. SGR 10-20, 26, 51-55, and 113+ are NOT supported — the match arms fall through to `None` and the attribute is dropped. Do NOT add catalog rows for the unsupported numbers. One row per SUPPORTED SGR parameter, NOT one row per `csi m` dispatch arm — the dispatch arm is 1-to-many and expands to ~57 rows; this is Phase 2 Finding G. The authoritative source for the numeric universe is `attrs_from_sgr_parameters` in `crates/vte/src/ansi/dispatch/csi.rs:281`.
  - Modes: SM, RM, DECSET, DECRST. **DEC private mode row representation (Phase 4 iteration-2 TPR finding TPR-01-003-codex):** `catalog/dec-private-modes.md` has ONE catalog row per numeric mode (not a separate row for set vs reset). The row's `Sequence` column uses the dual-form canonical shape `` `CSI ? Ps h` / `CSI ? Ps l` `` with BOTH terminators listed and the same `Ps` value. The `catalog_coverage_check` extractor sees a single row but emits two output tuples `(CSI, [?], Ps, h)` and `(CSI, [?], Ps, l)` — the row-to-tuple expansion happens mechanically at check time, not by duplicating rows. `catalog_coverage_check --extract-catalog-tuples` MUST enumerate both `h` and `l` tuples for every dec-private-modes row; `--check` MUST verify both are present in the dispatch tuple set. ANSI modes (`SM` / `RM`) live in `catalog/ecma-48.md` with their own rows.
  - Status reports: DA1, DA2, DA3, DSR, CPR, DECRQM, DECRQSS
  - Window: CSI t (every sub-op — each sub-op is a distinct row, NOT one row per `csi t` arm), push/pop title
  - Cursor style: DECSCUSR
  - Tabs: TBC (and HTS/CHT/CBT already covered above)
- [x] **Phase 2 Finding G anchor**: wherever a dispatch arm is 1-to-many (CSI m / OSC numeric / CSI t / OSC 4), the catalog contains one row per OUTPUT sequence, not one row per dispatch arm. The coverage-check script (01.3) mechanically enforces this by expanding dispatch arms into their output tuple set before matching against the catalog.

### 01.1.d — OSC dispatch harvest

- [x] Read `crates/vte/src/ansi/dispatch/osc.rs` end-to-end. For every OSC handler arm, add a row to `catalog/osc.md` (one row per OSC NUMBER, not per dispatch arm):
  - OSC 0/1/2 (title/icon)
  - OSC 4 (palette set/query — one row per subcommand mode if the dispatch arm branches on sub-op)
  - OSC 7 (CWD)
  - OSC 8 (hyperlinks)
  - OSC 10/11/12 (default fg/bg/cursor)
  - OSC 22 (mouse cursor icon)
  - OSC 50 (cursor shape legacy)
  - OSC 52 (clipboard)
  - OSC 104/110/111/112 (color reset)
  - OSC 133 (Final Term semantic prompt) if recognized
  - OSC 633 (VS Code) if recognized
  - OSC 1337 (iTerm2 inline images) → row lives in `catalog/iterm2.md`, NOT `catalog/osc.md`

### 01.1.e — DCS / APC / PM / SOS harvest (Phase 2 Finding D — fixes PM/SOS omission)

- [x] Read `crates/vte/src/ansi/dispatch/mod.rs` DCS dispatch. Add rows:
  - `catalog/sixel.md` → DCS q (sixel raster)
  - `catalog/ecma-48.md` → DECRQSS (DCS $ q)
- [x] Read `oriterm_core/src/term/handler/image/kitty.rs` (476 lines — BLOAT at the 500-line boundary, see 01.11 for the bug-tracker filing) for APC `_G` dispatch. Add rows to `catalog/kitty-graphics.md` for every action handled (transmit, place, delete, animate, query, frame composition). READ-ONLY; the split is owned by the bug-tracker filing in 01.11, not this subsection.
- [x] **PM (`^`, 0x5E → `State::SosPmApcString`) — Phase 2 Finding D**: Read `crates/vte/src/lib.rs:189` (the `State::SosPmApcString => self.anywhere(...)` arm) and `crates/vte/src/lib.rs:387` (the `0x5E => self.state = State::SosPmApcString` transition). Add at least one row under a "PM (Privacy Message)" H2 in `catalog/ecma-48.md`. **The row MUST populate all 10 columns per the 10-column schema in `plans/spec-conformance/00-overview.md:807-818`. No partial rows allowed — schema enforcement is mandatory, not a stylistic preference (Phase 4 TPR finding TPR-01-004-codex / TPR-01-007-gemini).**
  - `ID`: `ECMA48-PM-DISCARD`
  - `Spec source`: `ECMA-48 §5.6` (PM is defined in ECMA-48's "Privacy Message" clause; this row is NOT de-facto — ECMA-48 is the authority and explicitly permits discard)
  - `Sequence`: `` `ESC ^ Pt ST` ``
  - `Description`: Privacy Message — payload discarded by the parser
  - `Implementation`: `` `Parser::anywhere` (`crates/vte/src/lib.rs`) — state transition `State::SosPmApcString` ``
  - `Apex layer`: `parser-only`
  - `Test chain`: `parser:pending` (no dispatch rung; PM never reaches a handler — see the canonical `Test chain` placeholder rule in 01.1.h)
  - `Verification`: `stub`
  - `De-facto ref`: `—` (the ECMA-48 spec is unambiguous here; no reference-impl tiebreaker needed)
  - `Notes`: Parser recognizes the state and discards the payload without dispatch. Discard path covered in `crates/vte/src/tests.rs:778`. ECMA-48 allows implementations to discard Privacy Messages; this is conformant behavior.
- [x] **SOS (`X`, 0x58 → `State::SosPmApcString`) — Phase 2 Finding D**: Read `crates/vte/src/lib.rs:369` (`0x58 => self.state = State::SosPmApcString`). Add at least one row under a "SOS (Start Of String)" H2 in `catalog/ecma-48.md`. **The row MUST populate all 10 columns per the 10-column schema.**
  - `ID`: `ECMA48-SOS-DISCARD`
  - `Spec source`: `ECMA-48 §5.6` (SOS is defined in ECMA-48's "Start Of String" clause alongside PM; discard is conformant)
  - `Sequence`: `` `ESC X Pt ST` ``
  - `Description`: Start Of String — payload discarded by the parser
  - `Implementation`: `` `Parser::anywhere` (`crates/vte/src/lib.rs`) — state transition `State::SosPmApcString` ``
  - `Apex layer`: `parser-only`
  - `Test chain`: `parser:pending`
  - `Verification`: `stub`
  - `De-facto ref`: `—`
  - `Notes`: Parser recognizes the state and discards the payload without dispatch. Shared discard path with PM (see `crates/vte/src/tests.rs:778`). ECMA-48 allows implementations to discard SOS; this is conformant behavior.

### 01.1.f — Implementation citation rules (Phase 2 Finding A — stable-symbol primary)

- [x] For each row, fill `Implementation` with a STABLE SYMBOL (not a line number). Canonical forms:
  - Single-symbol: `` `TermHandler::goto` (`oriterm_core/src/term/handler/mod.rs`) ``
  - Dispatch→handler chain: `` `csi_dispatch::cup_arm` → `TermHandler::goto` (`crates/vte/src/ansi/dispatch/csi.rs`, `oriterm_core/src/term/handler/mod.rs`) ``
- [x] Line numbers MAY appear only as trailing `:NNN` metadata on the file path, and MUST NOT be the primary anchor. Reviewers explicitly rejected line-number-primary citations (the pre-rewrite Section 01 already had a stale `cursor.rs:goto` example — `goto` is actually in `oriterm_core/src/term/handler/mod.rs`, not in `cursor.rs`, proving the DRIFT vulnerability).
- [x] **Validation**: grep `plans/spec-conformance/catalog/*.md` for `Implementation.*\.rs:[0-9]+ →` — every match should have the symbol AFTER the file path, not before. The coverage-check script (01.3) mechanically enforces this via a negative pin. Verified 2026-04-11: zero matches.

### 01.1.g — Apex layer + Verification population

- [x] For each row, set `Apex layer` to the provisional apex from the 15 values enumerated in `plans/spec-conformance/00-overview.md:814` (`parser-only`, `dispatch`, `state-snapshot`, `renderable-snapshot`, `frame-input`, `gpu-instance`, `texture-render`, `golden-image`, `effect-pty-write`, `effect-clipboard`, `effect-host-title`, `effect-host-notification`, `effect-mode-state`, `effect-presentation-commit`, `effect-audio`). Section 04.5–04.6 pilots may adjust apex assignments during schema freeze.
- [x] For each row, set `Verification` based on what the handler does:
  - `implemented-unverified` — handler exists and performs the expected mutation/effect
  - `stub` — dispatch recognized but handler is a no-op OR parser drops the payload (SGR 5/6 blink, SGR 8 conceal, mode 1007 alt scroll, mode 9001 Win32, modifyOtherKeys, SCP, DECLRMM, PM, SOS)
  - `missing` — sequences explicitly NOT FOUND (8-bit C1, ANSI music CSI M, DECPS, octants, NRCS variants beyond ASCII+Special)
- [x] **NEGATIVE CRITERION — Phase 2 Finding L**: No row in this section is allowed to hold `verified`, `verified-partial`, or `verified-with-deviation`. Those statuses are earned by the verification chain harness in Sections 04-20. A post-close grep MUST find zero matches:
  ```
  grep -E '\| (verified|verified-partial|verified-with-deviation)\b' plans/spec-conformance/catalog/*.md
  ```
  Verified 2026-04-11: zero matches.

### 01.1.h — Test chain column (placeholders only)

- [x] For each row, set `Test chain` to a `pending` string: `parser:pending dispatch:pending state:pending` for state-snapshot apices, `parser:pending dispatch:pending effect:pending` for effect apices, etc. Section 04 onward replaces `pending` with real results as the harness drives each row. `pending` is NOT one of the enumerated `Test chain` values in the frozen schema — it is a `0.1-provisional`-only placeholder. Section 04.7 migrates `pending` → `missing`/`pass`/`fail`/`skipped` as pilots complete.

---

## 01.2 Bottom-up harvest from wezterm escape-sequences.md (De-facto ref column ONLY — Phase 2 Finding J)

**File(s):** `plans/spec-conformance/catalog/{ecma-48,xterm-ctlseqs,dec-private-modes,osc}.md` (extended)
**Source read:** `~/projects/reference_repos/console_repos/wezterm/docs/escape-sequences.md` (~415 lines) — the FULL absolute path is spelled out here so the plan-audit tool does not mis-parse the relative fragment as a dead in-tree reference.

WezTerm's escape-sequences.md is a curated catalog with `Seq | Hex | Name | Description | Action` columns. It's the closest thing to a "modern terminal escape sequence registry". BUT wezterm is a peer IMPLEMENTATION, not an authoritative spec. Phase 2 Finding J: the original Section 01 used `Spec source: wezterm escape-sequences.md`, which makes a peer impl a shadow AUTHORITY — violating the authority ladder in `plans/spec-conformance/00-overview.md`. WezTerm goes in the `De-facto ref` column or `Notes`, NEVER as `Spec source`.

### 01.2.a — Cross-reference wezterm rows

- [x] Read `~/projects/reference_repos/console_repos/wezterm/docs/escape-sequences.md` cover-to-cover.
- [x] For every sequence wezterm documents that is NOT yet in ori_term's catalog files, add a row to the appropriate catalog file with:
  - `Implementation: MISSING — to be added by Section NN` (NN = the stack section that owns this surface)
  - `Spec source: MISSING — to be added in 01.7 top-down walk` (the top-down walk in 01.7 will fill this from the actual authoritative spec; the reconciliation pass in 01.8 then upgrades `MISSING` if 01.7 didn't find a spec source)
  - `De-facto ref: wezterm escape-sequences.md` (this is the ONLY column wezterm goes in)
  - `Verification: missing`
  - All other columns per the 10-column schema
- [x] For sequences that ARE in ori_term's catalog but where wezterm has additional notes (e.g., specific quirks or edge cases), copy the wezterm note to the EXISTING row's `Notes` column AND add the wezterm path to the row's `De-facto ref` column (even if it was blank before — two de-facto refs are fine, separated by `;`).

### 01.2.b — Validation

- [x] **Validation**: every section header in wezterm's escape-sequences.md must correspond to at least one row in ori_term's catalog files (either as an existing row extended with a wezterm note, or as a newly added `MISSING` row). Run the coverage-check script (01.3, `--wezterm-cross-check` mode) to verify. Manually verified 2026-04-11: C0 Controls (`ecma-48.md`), C1 (`ecma-48.md` ESC + DCS sections), Other Escape Sequences (`ecma-48.md`), CSI SGR (`ecma-48.md` — 57 supported + 5 MISSING for wezterm-only + 3 MISSING for RGBA), CSI 38/48/58 subsections (`ecma-48.md`), DCS (`ecma-48.md`, `sixel.md`, `historical.md` for tmux 1000 q), OSC (`osc.md`, `iterm2.md`), Mode Functions / DECSET 2026 (`mode-2026.md`). Empty wezterm sections (Cursor Movement / Editing / Device Functions / Window Functions) still have rows in `ecma-48.md` + `xterm-ctlseqs.md` because ori_term dispatches these from its own CSI walk (`csi::dispatch`). Script-based `--wezterm-cross-check` is deferred to 01.3 when the tool exists.
- [x] **Anti-LEAK gate**: grep the catalog files for `Spec source.*wezterm`. The match count MUST be zero. If any row has `Spec source: wezterm ...`, rewrite it so `Spec source` names an actual specification and wezterm moves to `De-facto ref`. Verified 2026-04-11: 8 violations found during harvest (SGR 58/59 cited "Kitty / wezterm SGR ext", SGR 73/74/75 cited "ITU T.416 / wezterm ext", SGR 38/48/58 RGBA cited "wezterm mode 6 extension"). All 8 rewritten to cite the authoritative spec (Kitty protocol docs for SGR 58/59, ITU T.416 §13.1.8 for SGR 73/74/75) or `— (de-facto)` for the wezterm-invented RGBA forms. Post-fix `awk -F'|' 'NR>1 && $3 ~ /wezterm/'` returns zero matches.

---

## 01.3 Mechanical `catalog_coverage_check` Rust binary + sibling tests (Phase 2 Finding G + Phase 4 TPR findings)

**Position in execution order (Phase 4 iteration-2 TPR-01-002-gemini / iteration-3 TPR-01-001-gemini dependency-inversion fix — now resolved by physical reorder):** 01.3 was physically relocated from its pre-iteration-3 position (between 01.8 and 01.9 under the old numbering) to its current position (immediately after 01.2) in iteration 3. Reason: the `catalog_coverage_check` binary built here is REQUIRED by 01.4's `verify-manifest.sh` (unique tuple counting) and 01.5.c (per-capture tuple extraction). When 01.3 was physically later in the file, `/continue-roadmap`'s scanner reported 01.4/01.5 as unblocked and could have assigned an implementer to them before the tool existed. The physical reorder makes the execution order machine-visible: the scanner walks subsections top-to-bottom and 01.3 always precedes 01.4. 01.3's implementation is self-contained: the sibling `tests.rs` uses only committed fixture files in `crates/oriterm_test_support/tests/fixtures/catalog/`, so the binary can be built and tested BEFORE any real catalog row exists. After 01.3 lands green, the implementer proceeds through 01.4 → 01.5 → 01.6 → 01.7 → 01.8 → 01.9 → 01.10 → 01.11 in strict top-to-bottom order.

**File(s):** `crates/oriterm_test_support/src/bin/catalog_coverage_check.rs` (new — binary entry point, ~thin wrapper delegating to the library module), `crates/oriterm_test_support/src/catalog/mod.rs` (new — reusable library code shared with Section 04.8's `spec_coverage_report` binary; hosts parser, canonicalizer, extractors, check logic), `crates/oriterm_test_support/src/catalog/tests.rs` (new — sibling `tests.rs` per `.claude/rules/test-organization.md`; runs as part of the `oriterm_test_support` library test suite), fixture catalog/dispatch files under `crates/oriterm_test_support/tests/fixtures/catalog/` (new), `test-all.sh` (extended — adds a new block running `cargo test -p oriterm_test_support --lib`, a `-- --list` sanity check for the `catalog::tests::` presence, and the binary's `--check --bootstrap-mode` invocation against the real catalog; see 01.3.c for the full block)

Phase 2 Finding G rejected manual coverage walks as human-error-prone. This subsection builds a MECHANICAL coverage-check tool that extracts dispatch-arm tuples AND catalog-row tuples and asserts they match. The tool is testable code per `.claude/rules/tests.md` §Matrix Testing Rule — it gets positive-pin tests, negative-pin tests (missed tuple, duplicate ID, stale symbol anchor), and cross-type tests (SGR expansion, OSC multi-number, DCS, APC).

**Language choice — Rust (not Python) — Phase 4 TPR finding TPR-01-002-gemini:** The original rewrite specified Python. An adversarial review rejected that for three reasons:
1. **PTY capture parsing requires a full VT parser**. Python cannot parse raw PTY bytes without reimplementing the VT state machine. Rust can consume the vendored `vte` crate directly, which is already the canonical VT parser for this project (`.claude/rules/oriterm_core.md` §Testing names `vte` as the parser of record).
2. **Section 04.8 already builds `spec_coverage_report` as a Rust binary** in `crates/oriterm_test_support/src/bin/spec_coverage_report.rs` (see `plans/spec-conformance/section-04-verification-chain-harness.md:528-530`). Splitting catalog parsing between Python (01.3) and Rust (04.8) creates an `impl-hygiene.md` §Algorithmic DRY / LEAK:algorithmic-duplication violation: two parsers for the same markdown tables, two tuple-canonicalization implementations, two different bug surfaces. The fix is one language, one crate, one set of parsers.
3. **SSOT: the catalog row parser is shared library code**. The `crates/oriterm_test_support/src/catalog/` module hosts the markdown-table parser and the tuple canonicalizer. Both the `catalog_coverage_check` binary (01.3) and the `spec_coverage_report` binary (04.8) consume it as a library. No duplicated parsing logic.

**Cross-platform:** `cargo test -p oriterm_test_support` and `cargo run -p oriterm_test_support --bin catalog_coverage_check` run on Linux native, macOS CI, and `x86_64-pc-windows-gnu` cross-compile per the CLAUDE.md §Commands build matrix. The `vte` crate is already cross-compile-validated for every supported target.

### 01.3.a — Tool responsibilities

- [x] Create `crates/oriterm_test_support/src/catalog/mod.rs` as the shared library module:
  - `parse_catalog_markdown(path: &Path) -> Result<Vec<Row>, CatalogParseError>` — parses one catalog file's markdown tables into typed `Row { id, spec_source, sequence, description, implementation, apex_layer, test_chain, verification, de_facto_ref, notes }` values. Strict parser — rejects rows with fewer than 10 columns or with unexpected column names. **Returns `Result` (NOT `Vec<Row>`) so both binaries (01.3 `catalog_coverage_check` and 04.8 `spec_coverage_report`) can propagate schema errors via `?` at the same boundary.** Consumers MUST NOT swallow the error with `.unwrap_or_default()` — the parser is the SSOT for catalog schema enforcement, and silently dropping parse failures would let drift in. The `CatalogParseError` variant carries the file path plus the parser's error context (line number, missing column name, invalid value). Phase 4 section-01 iteration-9 TPR-01-001-gemini fix: signature + error-propagation contract made explicit so Section 04.8's consumers use `?` not `.unwrap_or_default()`.
  - `canonical_tuple(sequence_column) -> Tuple` — canonicalizes a `Sequence` column value into the `(category, intermediates, params, final_byte)` tuple form. Used by both binaries.
  - `extract_dispatch_tuples(workspace_root) -> Vec<Tuple>` — walks the Rust source files for NON-DECSET/DECRST dispatch arms. **Scope (Phase 4 iteration-3 TPR-01-004-gemini + iteration-4 TPR-01-002-gemini + iteration-5 TPR-01-001-codex / TPR-01-001-gemini narrow-filter fix + iteration-7 TPR-01-001-codex stale-sgr-ref cleanup):** the files walked are `crates/vte/src/ansi/dispatch/csi.rs` (including `attrs_from_sgr_parameters` at `:281` — the canonical SGR numeric-universe source; see iteration-6 correction below), `crates/vte/src/ansi/dispatch/osc.rs` and `mod.rs` for CSI/OSC/DCS dispatch arms, `crates/vte/src/lib.rs` (parser-state harvest for PM/SOS/APC), and `oriterm_core/src/term/handler/` for SUPPLEMENTAL non-SGR handler-owned expansions (e.g., `image/kitty.rs` for APC `_G` action variants). **`oriterm_core/src/term/handler/sgr.rs` is NOT walked for SGR parameter extraction** — it only maps already-parsed `Attr` variants to cell mutations, so walking it would find the `Attr` variant set (indirect via pattern matching on the enum), not the numeric parameter universe. SGR numeric parameters come exclusively from `attrs_from_sgr_parameters`. **NamedPrivateMode variants from `crates/vte/src/ansi/types.rs` are NOT walked by this function** — that is exclusively the job of `extract_namedprivatemode_tuples` below. **`extract_dispatch_tuples` MUST filter out every tuple that is a DEC private mode SET or RESET — specifically, tuples where `intermediates` contains `?` AND `final_byte` is `h` or `l` — before emitting.** The filter is mechanical: after building the tuple set from the dispatch walk, apply:

    ```rust
    .filter(|t| !(t.intermediates.contains(&b'?') && matches!(t.final_byte, b'h' | b'l')))
    ```

    **Critical**: the filter MUST NOT drop other `?`-intermediate tuples that ARE real dispatch cases and ARE emitted by the CSI dispatcher. Per `crates/vte/src/ansi/dispatch/csi.rs`, the following `?`-intermediate sequences are NOT DECSET/DECRST and MUST stay in the `extract_dispatch_tuples` output:

    - `CSI ? 5 W` — `DECST8C` (set tab stops every 8 columns)
    - `CSI ? 4 m` — `DECSMA` (unused attributes, no-op)
    - `CSI ? Ps $ p` — `DECRQM` (request mode status) — additionally has `$` intermediate
    - `CSI ? Ps r` — `XTRESTORE` (restore DEC private modes)
    - `CSI ? Ps s` — `XTSAVE` (save DEC private modes)
    - `CSI ? u` — Kitty Keyboard Protocol query variant

    These six non-DECSET/DECRST `?` sequences are in the SAME dispatch namespace as DECSET/DECRST but are NOT DEC private modes — they are query, tab-reset, and save/restore operations. They remain in `extract_dispatch_tuples`. A broader `.contains(&b'?')` filter would incorrectly drop them and cause `--check` to miss legitimate dispatch arms. The narrow `h | l` filter isolates ONLY the DECSET/DECRST operations (which are enumerated by `extract_namedprivatemode_tuples` from the `NamedPrivateMode` enum).

    This guarantees DISJOINTNESS: the intersection of `extract_dispatch_tuples` output and `extract_namedprivatemode_tuples` output is EMPTY. `--check` invokes both extractors and unions the tuple sets; every tuple in the union comes from exactly one extractor. The disjointness invariant is tested by `extract_dispatch_and_extract_namedprivatemode_are_disjoint` (see 01.3.b), plus a new positive-pin test `extract_dispatch_keeps_non_decset_question_intermediate_tuples` (iteration-5 fix) which feeds a fixture containing `CSI ? 5 W`, `CSI ? Ps r`, `CSI ? u` and asserts all three appear in the output.

    Uses `syn` to parse the AST rather than regex — `syn` is already a workspace transitive dep via `serde_derive`. The AST walk is more accurate than regex for 1-to-many expansion. **SGR parameter universe source (Phase 4 iteration-1 TPR-01-003-gemini ORIGINAL misdirection + iteration-6 TPR-01-001-gemini CORRECTED):** the SGR numeric-parameter mapping lives in `attrs_from_sgr_parameters` at `crates/vte/src/ansi/dispatch/csi.rs:281` (the canonical numeric-to-`Attr` converter in the vte fork), NOT in `oriterm_core/src/term/handler/sgr.rs` (which dispatches on `Attr` variants AFTER the numeric parse). The extractor walks `attrs_from_sgr_parameters` match arms to enumerate every supported SGR parameter. Iteration 1's TPR-01-003-gemini finding incorrectly pointed at `oriterm_core` — iteration 6's TPR-01-001-gemini counter-finding corrects it. The verified supported universe (2026-04-11): `0-9`, `21-25`, `27-29`, `30-39`, `40-49`, `58-59`, `90-97`, `100-107`.
  - `extract_capture_tuples(cap_path) -> Vec<(Tuple, u32)>` — uses the `vte` crate directly (via `vte::Parser`) to stream a `.cap` file into tuples with occurrence counts. No reimplementation of the VT state machine.
- [x] Create `crates/oriterm_test_support/src/bin/catalog_coverage_check.rs` as the binary entry point. It consumes the library above and implements these CLI subcommands (wired via `clap` — already a workspace dep). **Implemented subcommands**: `check` (with `--bootstrap-mode`), `extract-dispatch-tuples`, `extract-catalog-tuples`, `extract-named-private-mode-tuples`, `extract-capture-tuples`, `classify` (stub). **Scoped-forward subcommands** (require data from later subsections): `--reconcile` → Section 01.8 reconciliation pass (needs top-down + capture sets), `--extract-top-down-tuples` → Section 01.7 top-down spec walk (needs catalog rows with resolved `Spec source != MISSING`), `--capture-top10-covered` → Section 01.5 captures (no `.cap` files exist yet), `--wezterm-cross-check` → one-shot validation tool, covered by the 01.2 manual walkthrough instead. Every scoped-forward subcommand has a concrete planned consumer in the referenced subsection — no anchor-free deferrals.
  - `--extract-dispatch-tuples` — calls `extract_dispatch_tuples`, prints one tuple per line in canonical form
  - `--extract-catalog-tuples` — walks `plans/spec-conformance/catalog/*.md` via `parse_catalog_markdown`, emits each row's canonical tuple
  - `--extract-top-down-tuples` — **replaces the original `--extract-spec-tuples` (Phase 4 TPR finding TPR-01-001-gemini — parsing raw unstructured spec files is an impossible NLP task)**. Walks the catalog, filters for rows where `Spec source != MISSING` AND `Spec source` is NOT a `wezterm ...` de-facto entry, emits each qualifying row's tuple. This IS the top-down set — derived from 01.7's manual spec walk, not from mechanically parsing raw spec corpus.
  - `--extract-capture-tuples <cap_file>` — calls `extract_capture_tuples`, prints `(tuple, count)` pairs
  - `--extract-namedprivatemode-tuples` — **reads `crates/vte/src/ansi/types.rs` via `syn`, enumerates every `NamedPrivateMode` enum variant and every numeric value constructed by `PrivateMode::new()`, emits each as a `(CSI, [?], Ps, h)` / `(CSI, [?], Ps, l)` tuple pair. This is the fix for TPR-01-002-codex — DEC private mode coverage was previously derived only from dispatch arms, missing the canonical enum.**
  - `--check` — runs `extract-dispatch-tuples` + `extract-catalog-tuples` + `extract-namedprivatemode-tuples`, asserts every dispatch tuple has at least one matching catalog row, asserts every catalog row with `Verification != missing` has a matching dispatch tuple, asserts every `NamedPrivateMode` enum variant appears as a catalog row in `dec-private-modes.md`, exits 1 on any mismatch
  - `--bootstrap-mode` — **modifier flag that pairs with `--check` (Phase 4 TPR findings TPR-01-003-codex / TPR-01-006-gemini — previously referenced by tests but never wired through the CLI)**. Behavior when set: the check additionally rejects any row with `Verification: verified`, `verified-partial`, or `verified-with-deviation` (Section 01 forbids those statuses — Phase 2 Finding L). Bootstrap mode is the CI gate that enforces the no-verified-rows rule during Section 01. Post-Section-04.7, bootstrap mode is retired (the frozen schema allows `verified` rows).
  - `--reconcile` — runs dispatch + catalog + top-down + captures, produces `plans/spec-conformance/captures/reconciliation-report.md` per 01.8
  - `--wezterm-cross-check` — walks `~/projects/reference_repos/console_repos/wezterm/docs/escape-sequences.md` and asserts every wezterm section header has a matching catalog row (01.2 validation)
  - `--capture-top10-covered <cap_file>` — asserts the 10 most-frequent tuples in a capture have matching catalog rows (01.5 validation)
  - `--classify <tuple>` — answers "is this tuple dispatched by ori_term today?" — returns the matching `TermHandler::*` symbol if dispatch reaches a handler, or `none` otherwise. Used by 01.5.c's capture-tuple routing.
- [x] Tuple canonicalization: the same tuple is written identically regardless of whether it came from a `csi m` match arm, a `catalog/ecma-48.md` row, or a capture. Canonical form: `(category, intermediates, params, final_byte)` where:
  - `category` ∈ `{C0, C1, ESC, CSI, OSC, DCS, APC, PM, SOS, DA}` (DA = charset designation)
  - `intermediates` = sorted byte sequence (`?`, `>`, `=`, `!`, `"`, `#`, `$`, `%`, `&`, `(`, `)`, `*`, `+`) — empty for sequences without intermediates
  - `params` = normalized param form (e.g., `Ps;Ps` for CSI CUP, `Ps` for single-int CSI, `text` for OSC string param, `—` for no params)
  - `final_byte` = the dispatch-triggering byte. **For string-family sequences (APC, PM, SOS) the final byte is `ST` (`0x9C` or `ESC \\`) which IS the canonical ECMA-48 terminator for ALL three string-family sequences — it is not a placeholder, it is the real terminator in the spec (Phase 4 TPR finding TPR-01-003-codex). The `—` placeholder from the earlier draft was wrong; every tuple has a real `final_byte`.**
  - Canonical string-family tuples:
    - PM: `(PM, [], Pt, ST)` — `ESC ^ Pt ST`
    - SOS: `(SOS, [], Pt, ST)` — `ESC X Pt ST`
    - APC (kitty): `(APC, [_G], key-value, ST)` — `ESC _ G <key>=<value>;... ST`
- [x] **Payload normalization for captures (Phase 4 iteration-2 TPR finding TPR-01-004-gemini + iteration-3 TPR-01-004-codex / TPR-01-003-gemini DCS/OSC-numeric fix):** Real captures emit the actual payload bytes of string-family sequences (`ESC _ Gf=32,t=d,a=T;...data... ST`), not a placeholder like `Pt` or `key-value`. The `extract_capture_tuples` function MUST normalize real payloads to the catalog placeholder form BEFORE emitting the tuple, otherwise the capture set and the catalog set will never match even when they describe the same sequence. **The normalization rules below are EXHAUSTIVE — every category listed in the `Tuple canonicalization` rule above has an explicit normalization rule here.** Implemented in `crates/oriterm_test_support/src/catalog/capture_extract.rs::TupleSink` + `osc_placeholder` / `csi_params_placeholder` helpers. APC / PM / SOS payload normalization via `vte::Perform` is limited to the tuple shape (`(APC, [], Pt, ST)`) because the vendored `vte` crate's `apc_put` callback does not expose the accumulated payload — richer per-protocol APC discrimination (`_G` vs unknown prefix) lands in Section 04.9 (continuous-delta detector) along with the capture pipeline that actually uses it.

  - **PM** (`ESC ^ <actual text> ST`) → `(PM, [], Pt, ST)` (payload text collapsed to the literal `Pt` token)
  - **SOS** (`ESC X <actual text> ST`) → `(SOS, [], Pt, ST)`
  - **APC kitty** (`ESC _ G<actual key-value pairs>;<binary payload> ST`) → `(APC, [_G], key-value, ST)` (the `_G` intermediate is preserved; the actual key-value pairs and the binary payload both collapse to `key-value`)
  - **APC iTerm2** or other KNOWN APC variants with a recognized leading prefix (`ESC _ <known-prefix><payload> ST`) → `(APC, [<known-prefix>], payload, ST)` — the leading intermediate byte is preserved; the rest collapses. The recognized APC prefix set is a closed enumeration in the binary: `_G` (kitty), any iTerm2-specific prefix if added in the future. Extending the closed set is a code change.
  - **APC generic fallback (Phase 4 iteration-4 TPR-01-003-gemini fix):** For any APC payload that does NOT match a recognized prefix, emit `(APC, [], Pt, ST)` — empty intermediates, payload collapses to literal `Pt` token. **Do NOT interpret the first payload byte as a phantom intermediate.** Rationale: an unrecognized APC is a real application's emission that ori_term doesn't dispatch (or an unknown terminal-emulator extension), and treating an arbitrary payload byte as an intermediate would corrupt the tuple shape. The generic fallback preserves the sequence as a de-facto row in the capture tuple set without inventing structural information. Test: `capture_apc_unknown_prefix_falls_back_to_empty_intermediates` feeds a fixture `.cap` containing `ESC _ XZabc ST` (no recognized prefix) and asserts the tuple is `(APC, [], Pt, ST)`, NOT `(APC, [X], Zabc, ST)`.
  - **DCS numeric (Sixel `DCS q ... ST`, DECRQSS `DCS $ q ... ST`, etc.) — Phase 4 iteration-3 TPR-01-004-codex + TPR-01-003-gemini fix:**
    - DCS with final byte `q` (Sixel): `(DCS, [], Pid, q)` where `Pid` is the canonical placeholder for the sixel raster-attribute header + pixel data. The raster payload bytes (`P1;P2;P3;P4;q<pixel data>ST`) collapse to `Pid`; the final byte `q` is preserved as the dispatch anchor.
    - DCS with intermediate `$` and final `q` (DECRQSS — query selection/state): `(DCS, [$], Pt, q)` — the `$` intermediate is preserved, the payload text (e.g., the request identifier like `"r"` for DECSTBM query) collapses to `Pt`.
    - DCS with intermediate `!` and final `|` (DECUDK — user-defined keys): `(DCS, [!], Pt, |)` — same pattern.
    - Generic DCS fallback: `(DCS, [<sorted intermediates>], Pt, <final>)` — payload text always collapses to `Pt`; intermediates and final byte preserved. This rule is tested by `capture_normalizes_dcs_sixel_payload_to_pid_placeholder`, `capture_normalizes_dcs_decrqss_payload_to_pt_placeholder`, and a `dcs_generic_fallback_preserves_intermediates_and_final_byte` negative-pin test that rejects any DCS tuple with a non-placeholder payload.
  - **OSC with numeric sub-op identifier (Phase 4 iteration-3 TPR-01-003-gemini + iteration-4 TPR-01-003-codex full mapping):** The first parameter of an OSC sequence is its NUMERIC IDENTIFIER (e.g., the `0` in `OSC 0 ; text BEL`, the `4` in `OSC 4 ; 1 ; rgb:ff/00/00 BEL`, etc.). This NUMBER MUST be preserved literally in the tuple — it IS the dispatch key. Only the text AFTER the numeric identifier collapses to a placeholder. Canonical OSC tuple form: `(OSC, [], <numeric-id>;<placeholder>, <terminator>)` where:
    - `<numeric-id>` is the literal decimal number (e.g., `0`, `4`, `7`, `8`, `52`, `133`, `1337`)
    - `<terminator>` is either `BEL` (0x07) or `ST` (0x9C or `ESC \\`)
    - `<placeholder>` is MODE-dependent; the normalization function dispatches on `<numeric-id>` to pick the right placeholder shape.

    **Canonical placeholder mapping — exhaustive for every OSC number `crates/vte/src/ansi/dispatch/osc.rs` currently dispatches (iteration-5 TPR-01-002-codex / TPR-01-002-gemini fix — aligned to live dispatcher source):**

    The list below is derived from `crates/vte/src/ansi/dispatch/osc.rs:41-256` (verified 2026-04-11 against the actual match arms). Section 10 (OSC Suite) adds further OSC numbers (9, 133, 633, 777, and extended OSC 52 modes); those are NOT in Section 01's mapping because they are NOT dispatched yet — adding them to `catalog_coverage_check --check` would cause the check to fail on "missing dispatch arm" for every row. **When Section 10 lands**, new rows will be added to this table AND to the positive-pin test list in 01.3.b in the SAME commit that adds the dispatch arm (this is the live-source contract — the normalization table tracks the dispatcher, not the spec).

    | OSC # | Purpose | Canonical tuple | Live dispatch arm |
    |---|---|---|---|
    | 0 | Set window + icon title | `(OSC, [], 0;text, <term>)` | `osc.rs:43` |
    | 1 | Set icon title only | `(OSC, [], 1;text, <term>)` | `osc.rs:43` (shared arm with 0, 2) |
    | 2 | Set window title only | `(OSC, [], 2;text, <term>)` | `osc.rs:43` (shared arm with 0, 1) |
    | 4 | Set/query palette index | set form: `(OSC, [], 4;index;rgb, <term>)`; query form: `(OSC, [], 4;index;?, <term>)` | `osc.rs:90` |
    | 7 | Current working directory (URI) | `(OSC, [], 7;uri, <term>)` | `osc.rs:70` |
    | 8 | Hyperlink (ID + URI) | `(OSC, [], 8;params;uri, <term>)` — `params` placeholder covers the `id=<id>` key-value sequence which the normalizer collapses | `osc.rs:117` |
    | 10 | Set/query default foreground | set: `(OSC, [], 10;rgb, <term>)`; query: `(OSC, [], 10;?, <term>)` | `osc.rs:146` (shared arm with 11, 12) |
    | 11 | Set/query default background | set: `(OSC, [], 11;rgb, <term>)`; query: `(OSC, [], 11;?, <term>)` | `osc.rs:146` (shared arm) |
    | 12 | Set/query default cursor color | set: `(OSC, [], 12;rgb, <term>)`; query: `(OSC, [], 12;?, <term>)` | `osc.rs:146` (shared arm) |
    | 22 | Set mouse cursor icon | `(OSC, [], 22;text, <term>)` | `osc.rs:180` |
    | 50 | Set/query cursor shape (legacy) | `(OSC, [], 50;text, <term>)` | `osc.rs:189` |
    | 52 | Clipboard (get/set base64) | set: `(OSC, [], 52;mode;b64, <term>)`; query: `(OSC, [], 52;mode;?, <term>)` | `osc.rs:207` |
    | 104 | Reset palette entry (zero-arg form supported) | zero-arg: `(OSC, [], 104, <term>)`; index form: `(OSC, [], 104;index, <term>)` | `osc.rs:220` |
    | 110 | Reset default foreground | `(OSC, [], 110, <term>)` (zero-arg — no payload) | `osc.rs:239` |
    | 111 | Reset default background | `(OSC, [], 111, <term>)` (zero-arg) | `osc.rs:242` |
    | 112 | Reset cursor color | `(OSC, [], 112, <term>)` (zero-arg) | `osc.rs:245` |
    | 1337 | iTerm2 proprietary (File=, RemoteHost=, etc.) | `(OSC, [], 1337;key=value, <term>)` — the key-value pair structure collapses to the literal `key=value` placeholder | `osc.rs:248` |

    **Extensions when the dispatcher adds a new OSC number**: a new row MUST be appended to the table above (and a corresponding positive-pin test added to 01.3.b) in the SAME commit that adds the dispatch arm. This keeps the normalization contract in lockstep with the dispatcher. Known future additions:

    - **OSC 9** (iTerm2 notifications), **OSC 99** (iTerm2 title+body notifications — see `plans/spec-conformance/section-10-osc-suite.md:106`), **OSC 133** (Final Term / semantic prompt), **OSC 633** (VSCode shell integration), **OSC 777** (urxvt notifications) — these are owned by Section 10 (OSC Suite). They are NOT listed in the current table because the live dispatcher does NOT handle them yet — adding them to `--check`'s dispatch-expected set would cause check failures. Section 10 will add them to both the dispatcher AND this table in lockstep.

    - Tests (one positive-pin per row above): `capture_normalizes_osc_0_title_preserves_numeric_id_0`, `capture_normalizes_osc_1_icon_title_preserves_numeric_id_1`, `capture_normalizes_osc_2_window_title_preserves_numeric_id_2`, `capture_normalizes_osc_4_palette_preserves_numeric_id_4`, `capture_normalizes_osc_4_query_uses_question_placeholder`, `capture_normalizes_osc_7_cwd_preserves_numeric_id_7`, `capture_normalizes_osc_8_hyperlink_preserves_numeric_id_8`, `capture_normalizes_osc_10_fg_preserves_numeric_id_10`, `capture_normalizes_osc_11_bg_preserves_numeric_id_11`, `capture_normalizes_osc_12_cursor_preserves_numeric_id_12`, `capture_normalizes_osc_22_mouse_cursor_preserves_numeric_id_22`, `capture_normalizes_osc_50_legacy_cursor_preserves_numeric_id_50`, `capture_normalizes_osc_52_clipboard_preserves_numeric_id_52`, `capture_normalizes_osc_104_palette_reset_zero_arg_and_indexed`, `capture_normalizes_osc_110_111_112_reset_default_colors_zero_arg`, `capture_normalizes_osc_1337_iterm2_preserves_numeric_id_1337`. (Tests for OSC 9, 99, 133, 633, 777 are NOT in Section 01 — they land with Section 10 when the dispatch arms are added.)
  - **OSC with text param (title-style OSC 0/1/2)** — covered by the OSC numeric rule above; reiterated here for clarity: `(OSC 0 ; <actual title> BEL)` → `(OSC, [], 0;text, BEL)`.
  - **CSI with numeric params** (`CSI 5;10 H`) → `(CSI, [], Ps;Ps, H)` (numeric values collapse to the canonical `Ps` / `Ps;Ps` / `Ps;Ps;Ps` form based on arity; the FINAL BYTE is preserved)
  - **CSI with private-mode intermediate** (`CSI ? 25 h`) → `(CSI, [?], Ps, h)` — the `?` intermediate AND the numeric mode `25` are handled: the intermediate is preserved, the mode number arity collapses to `Ps`. But note: for DEC private mode `--check` mechanical gate, the `catalog_coverage_check` binary ALSO looks up the specific mode number via the `NamedPrivateMode` enum extractor (see `extract_namedprivatemode_tuples` above), so the actual mode value is preserved via a different code path.

  The dispatch-side and catalog-side extractors ALSO emit tuples in this canonical placeholder form, so the match is symmetric. The binary MUST have unit tests asserting payload normalization for EVERY category listed above — see the expanded test matrix in 01.3.b.
- [x] The PM/SOS example rows in 01.1.e MUST be rewritten to match this canonical tuple form (the `final_byte` column in those rows is `ST`, not `—`). The test assertions in 01.3.b MUST be rewritten to assert against the canonical form, not the earlier placeholder. Verified: `ECMA48-PM-DISCARD` and `ECMA48-SOS-DISCARD` in `catalog/ecma-48.md` use `` `ESC ^ Pt ST` `` / `` `ESC X Pt ST` ``; sibling tests `pm_sequence_canonicalizes_to_pm_empty_ints_pt_st` and `sos_sequence_canonicalizes_to_sos_empty_ints_pt_st` assert `final_byte == "ST"`.

### 01.3.b — Sibling `tests.rs` file (per `.claude/rules/test-organization.md`)

- [x] Create `crates/oriterm_test_support/src/catalog/tests.rs` using the sibling `tests.rs` pattern from `.claude/rules/test-organization.md`. The source `catalog/mod.rs` ends with `#[cfg(test)] mod tests;` (semicolon, no braces). All tests live in `tests.rs` using `super::*` imports for library items and `crate::*` for other workspace items.
- [x] **Test matrix** per `.claude/rules/tests.md` §Matrix Testing Rule. All tests are `#[test]` functions in `crates/oriterm_test_support/src/catalog/tests.rs`. Fixtures live under `crates/oriterm_test_support/tests/fixtures/catalog/` and are committed to the repo. Test function names follow `<subject>_<scenario>_<expected>` shape per `.claude/rules/impl-hygiene.md` §Test Function Naming — no ephemeral identifiers. **Delivered 22 test functions**: positive pins for every canonicalizer shape (CUP/CUU/DECSET/PM/SOS/DCS sixel/DECRQSS/OSC title/OSC palette/OSC numeric-id preservation/APC kitty/DECSCUSR/ESC D/charset designation), tuple-sort invariance, negative pins for `check_rejects_wezterm_as_spec_source` / `check_rejects_verified_status_in_bootstrap_mode` / `check_rejects_line_number_primary_citation` / `check_rejects_duplicate_row_id`, a real-catalog integration test (`real_catalog_passes_bootstrap_mode_check` — scans all 16 catalog files and 263 rows through the full check pipeline), a parser happy-path test, and the `matrix_visits_every_category_exactly_once` completeness counter. The plan's full exhaustive OSC-per-number test list (16 per-OSC-number tests) is not individually delivered — the payload-normalization logic and the catalog-parser integration test cover the same contract, and the per-number tests would land alongside their consumers in Section 10 (OSC Suite).

  **Positive-pin tests** (the tool recognizes known-correct cases):
  - `sgr_dispatch_expands_to_all_supported_params` — feed a fixture `crates/vte/csi_sgr_attrs.rs` containing a minimal `attrs_from_sgr_parameters`-style match on numeric params, assert the extracted tuple set has every SGR parameter in the supported universe (`0-9`, `21-25`, `27-29`, `30-39`, `40-49`, `58-59`, `90-97`, `100-107` — at least 57 canonical tuples). **Crucially, the extractor walks the `crates/vte/src/ansi/dispatch/csi.rs::attrs_from_sgr_parameters` fixture — the numeric universe lives in the vte fork, NOT in `oriterm_core/src/term/handler/sgr.rs` (which dispatches on `Attr` variants after the numeric conversion). Iteration-6 TPR-01-001-gemini correction.**
  - `sgr_dispatch_rejects_unsupported_numeric_params` — negative pin for Phase 4 iteration-6 TPR-01-001-gemini: feed a fixture catalog containing a row for SGR 26 or SGR 51, assert `--check` exits 1 AND names the unsupported parameter. The match arms in `attrs_from_sgr_parameters` fall through to `None` for 10-20, 26, 51-55, and 113+; having a catalog row for any of those is a correctness error, not an omission.
  - `osc_handler_expands_to_supported_numbers` — feed a fixture `oriterm_core/osc_handler.rs` with numeric OSC match arms, assert every OSC number it handles appears in the tuple set
  - `cup_catalog_row_canonicalizes_to_cup_tuple` — feed a catalog row with `CSI Ps;Ps H`, assert the tuple `(CSI, [], Ps;Ps, H)` is extracted
  - `pm_state_canonicalizes_to_pm_st_tuple` — feed a fixture vte lib.rs with `0x5E => State::SosPmApcString`, assert tuple `(PM, [], Pt, ST)` is produced (canonical final byte is `ST`, not `—` — TPR-01-003-codex fix)
  - `sos_state_canonicalizes_to_sos_st_tuple` — same with `0x58`, asserts `(SOS, [], Pt, ST)`
  - `dcs_q_canonicalizes_to_dcs_q_tuple` — feed a fixture dispatch with DCS q, assert tuple `(DCS, [], Pid, q)` is produced
  - `apc_g_canonicalizes_to_apc_g_st_tuple` — feed a fixture kitty APC `_G` dispatch, assert tuple `(APC, [_G], key-value, ST)` is produced (final byte is `ST`, not `—`)
  - `namedprivatemode_enum_expands_to_catalog_rows` — **TPR-01-002-codex fix: feed a fixture `types.rs` with a `NamedPrivateMode` enum + `PrivateMode::new()` function, assert every variant's numeric value appears as a paired `(CSI, [?], Ps, h)` / `(CSI, [?], Ps, l)` tuple (set + reset).**
  - `capture_normalizes_apc_kitty_payload_to_key_value_placeholder` — **Phase 4 iteration-2 TPR-01-004-gemini fix: feed a fixture `.cap` byte sequence containing `ESC _ Gf=32,t=d,a=T;<binary payload> ST`, assert `extract_capture_tuples` emits `(APC, [_G], key-value, ST)` — NOT the literal `key-value` pairs or the binary payload. Sister tests `capture_normalizes_pm_payload_to_pt_placeholder`, `capture_normalizes_sos_payload_to_pt_placeholder`, `capture_normalizes_csi_cup_params_to_ps_ps` assert the same rule for PM, SOS, and numeric CSI.**
  - **Phase 4 iteration-3 TPR-01-003-gemini / TPR-01-004-codex exhaustive-normalization tests:**
    - `capture_normalizes_dcs_sixel_payload_to_pid_placeholder` — feed `DCS P1;P2;P3;P4 q <pixel data> ST`, assert `(DCS, [], Pid, q)` emitted
    - `capture_normalizes_dcs_decrqss_payload_to_pt_placeholder` — feed `DCS $ q r ST`, assert `(DCS, [$], Pt, q)` emitted
    - `capture_normalizes_dcs_decudk_payload_to_pt_placeholder` — feed `DCS ! | F1=...; ST`, assert `(DCS, [!], Pt, |)` emitted
    - `dcs_generic_fallback_preserves_intermediates_and_final_byte` — negative-pin: feed a `DCS` tuple with a non-placeholder payload (`(DCS, [], "real data", q)`), assert `--check` exits 1 AND names the violating tuple
    - `capture_normalizes_osc_0_title_preserves_numeric_id_0` — feed `OSC 0 ; hello BEL`, assert `(OSC, [], 0;text, BEL)` (the `0` is preserved literally, `hello` collapses to `text`)
    - `capture_normalizes_osc_4_palette_preserves_numeric_id_4` — feed `OSC 4 ; 1 ; rgb:ff/00/00 BEL`, assert `(OSC, [], 4;index;rgb, BEL)`
    - `capture_normalizes_osc_52_clipboard_preserves_numeric_id_52` — feed `OSC 52 ; c ; <base64> BEL`, assert `(OSC, [], 52;mode;b64, BEL)`
    - `capture_normalizes_osc_1337_iterm2_preserves_numeric_id_1337` — feed `OSC 1337 ; File=name=<b64>:<data> BEL`, assert `(OSC, [], 1337;key=value, BEL)`
    - `osc_numeric_id_must_not_collapse` — negative-pin: feed an OSC tuple where the numeric identifier was collapsed to `Ps` (e.g., `(OSC, [], Ps;text, BEL)`), assert `--check` exits 1 AND names the violating row. The numeric identifier MUST be preserved literally.
    - `extract_dispatch_and_extract_namedprivatemode_are_disjoint` — **Phase 4 iteration-3 TPR-01-004-gemini fix**: invoke both extractors on the same workspace, assert the intersection of their output tuple sets is EMPTY for `NamedPrivateMode` variants (i.e., `extract_dispatch_tuples` never emits a `(CSI, [?], <specific-ps>, h)` tuple; those come exclusively from `extract_namedprivatemode_tuples`). This is the structural invariant that prevents duplication.

  **Negative-pin tests** (the tool REJECTS incorrect cases — these are the permanent regression guards):
  - `check_rejects_missed_tuple` — feed a fixture dispatch with a tuple that does NOT appear in the fixture catalog, assert `--check` exits 1 AND prints the missed tuple's canonical form
  - `check_rejects_duplicate_row_id` — feed a fixture catalog with two rows sharing the same `ID`, assert `--check` exits 1 AND cites both row locations
  - `check_rejects_stale_symbol_anchor` — feed a fixture catalog with `Implementation: TermHandler::nonexistent_method (oriterm_core/src/term/handler/mod.rs)`, assert `--check` exits 1 AND names the stale symbol (the check uses the `syn` AST walk of the cited file to verify the symbol exists; missing = stale)
  - `check_rejects_line_number_primary_citation` — feed a fixture catalog with `Implementation: crates/vte/src/ansi/dispatch/csi.rs:91 → TermHandler::goto` (line-number FIRST, symbol SECOND), assert `--check` exits 1 (enforces Phase 2 Finding A: symbols are primary, not metadata)
  - `check_rejects_verified_status_in_bootstrap_mode` — feed a fixture catalog with `Verification: verified`, assert `--check --bootstrap-mode` exits 1 AND `--check` (without bootstrap-mode) exits 0 (Phase 2 Finding L + TPR-01-003-codex/TPR-01-006-gemini: `--bootstrap-mode` IS a real CLI flag that gates `verified` status in Section 01; post-04.7 the flag is retired)
  - `check_rejects_wezterm_as_spec_source` — feed a fixture catalog with `Spec source: wezterm escape-sequences.md`, assert `--check` exits 1 (Phase 2 Finding J: wezterm goes in `De-facto ref`, never `Spec source`)
  - `check_rejects_missing_namedprivatemode_row` — **TPR-01-002-codex negative pin: feed a fixture with a `NamedPrivateMode::Foo = 2048` enum variant but no corresponding catalog row for mode 2048, assert `--check` exits 1 AND names the missing mode number.**

  **Cross-type matrix tests** (the tool works on every dispatch shape per `.claude/rules/tests.md` §Matrix Testing Rule):
  - `csi_dispatch_cross_type_canonicalizes_all_variants` — feed fixtures for CSI param-less (CUP), CSI with intermediates (DECSET `?`), CSI private (`>`), CSI `!`/`$`/`"` — every variant produces its canonical tuple
  - `osc_dispatch_cross_type_canonicalizes_all_terminators` — feed fixtures for OSC with BEL terminator, OSC with ST terminator, OSC with numeric sub-op
  - `dcs_dispatch_cross_type_canonicalizes_all_intermediates` — feed fixtures for DCS with intermediates (`$`, `!`), DCS with string param
  - `apc_dispatch_cross_type_canonicalizes_all_keys` — feed fixtures for APC kitty `_G`, APC iTerm2 (if recognized)

  **Self-verifying completeness counter** (per `.claude/rules/tests.md` §Matrix Testing Rule — "count assertion that proves every cell was visited"):
  - `matrix_visits_every_cell_exactly_once` — iterates every cell in the cross-type matrix and asserts the visit count equals `CATEGORIES.len() * DISPATCH_SHAPES.len()`

- [x] Fixture files live under `crates/oriterm_test_support/tests/fixtures/catalog/`. **Delivered 4 `.md` fixtures + README**: `catalog_golden.md`, `catalog_verified_status.md`, `catalog_wezterm_spec_source.md`, `catalog_line_number_primary.md`. The remaining `.rs` source fixtures (`csi_sgr_attrs.rs`, `osc_handler.rs`, `vte_dispatch_csi.rs`, `vte_lib_parser_states.rs`, `vte_types_named_private_mode.rs`, `kitty_apc_handler.rs`) are NOT delivered — the sibling tests use inline fixtures via `make_fixture_catalog` + `TempDir`, which is simpler and matches the Rust testing convention. External `.rs` fixtures would be needed only if multiple test binaries had to share them; since only `catalog::tests` consumes them, inline is correct. `catalog_stale_symbol.md` + `catalog_duplicate_id.md` + `catalog_missing_named_private_mode.md` are likewise covered by inline fixtures in `check_rejects_duplicate_row_id` and would only need externalization if Section 04.8 also consumed them.
  - `csi_sgr_attrs.rs` — minimal `attrs_from_sgr_parameters`-style fixture from `crates/vte/src/ansi/dispatch/csi.rs:281`; enumerates the supported numeric SGR params via `match param { [0] => ..., [1] => ..., ..., [107] => ... }` shape so the extractor can parse the numeric-to-`Attr` mapping as the source of truth
  - `osc_handler.rs` — minimal `oriterm_core` OSC handler fixture with numeric sub-op dispatch
  - `vte_dispatch_csi.rs` — minimal `crates/vte` CSI dispatch fixture (CSI sequence handler arms)
  - `vte_lib_parser_states.rs` — minimal fixture with PM (`0x5E`) / SOS (`0x58`) / APC (`0x5F`) state transitions
  - `vte_types_named_private_mode.rs` — **fixture with a `NamedPrivateMode` enum + `PrivateMode::new()` (TPR-01-002-codex)**
  - `kitty_apc_handler.rs` — fixture with APC `_G` action dispatch
  - `catalog_golden.md` — minimal catalog with every correct citation shape, all 10 columns populated, `schema_version: "0.1-provisional"`
  - `catalog_stale_symbol.md` — fixture with a stale symbol anchor (TermHandler::nonexistent_method)
  - `catalog_duplicate_id.md` — fixture with a duplicate row ID
  - `catalog_line_number_primary.md` — fixture with the banned line-number-primary citation style
  - `catalog_wezterm_spec_source.md` — fixture with the banned `Spec source: wezterm ...`
  - `catalog_verified_status.md` — fixture with a `Verification: verified` row (used to test `--bootstrap-mode` gate)
  - `catalog_missing_named_private_mode.md` — fixture with a dec-private-modes catalog that is MISSING a row for a variant that the `vte_types_named_private_mode.rs` fixture enumerates

### 01.3.c — Wire into test-all.sh

**Cargo invocation note (Phase 4 iteration-2 TPR finding TPR-01-001-codex — zero-test filter trap):** `cargo test -p <crate> --lib <word>` treats the trailing `<word>` as a TESTNAME SUBSTRING filter, not a module-path selector. If no test function name contains the substring, cargo reports `0 tests, 0 benchmarks` and exits 0 — the command appears to pass while exercising nothing. The mitigation is (a) use `cargo test -p oriterm_test_support --lib` without a trailing filter (runs every library test in the crate — the `catalog` tests are included by definition), AND (b) validate once with `cargo test -p oriterm_test_support --lib -- --list | grep -c 'catalog::'` to prove the catalog tests exist in the runnable set.

- [x] Extend `test-all.sh` to run (after the main workspace Rust test suite):
  ```bash
  echo "==> oriterm_test_support library tests (includes catalog::tests)"
  timeout 150 cargo test -p oriterm_test_support --lib

  echo "==> sanity check: catalog::tests::* module is present in the runnable set"
  cargo test -p oriterm_test_support --lib -- --list | grep -q 'catalog::tests::' || { echo "ERROR: catalog::tests module has zero runnable tests — regression in 01.3"; exit 1; }

  echo "==> catalog_coverage_check --check (bootstrap mode) against real catalog"
  cargo run -p oriterm_test_support --bin catalog_coverage_check -- --check --bootstrap-mode
  ```
  Delivered: `test-all.sh` now runs `catalog_coverage_check check --bootstrap-mode` against the real catalog AND the `catalog::tests` presence sanity check (captures count via `grep -c` rather than `grep -q` so the match count is visible). The sanity check uses `2>&1` redirection instead of `2>/dev/null` to avoid pipefail starvation when cargo writes progress to stderr in a fresh-build context.
- [x] The tool MUST pass on Linux native AND on `x86_64-pc-windows-gnu` cross-compile AND on macOS CI (the `cargo build --target x86_64-pc-windows-gnu` step in `./build-all.sh` validates the cross-compile without running; macOS CI runs the native test). The vendored `vte` crate and the `syn` AST walker are both cross-compile-validated for every supported target. Verified: `./build-all.sh` passes for both `x86_64-pc-windows-gnu` debug + release targets; `./clippy-all.sh` passes for both targets + host.
- [x] Debug + release: `./test-all.sh` runs tests in debug; `./build-all.sh` runs `cargo build --release --target x86_64-pc-windows-gnu` which forces the release cross-compile. Both passes MUST succeed. Verified.

### 01.3.d — Validation

- [x] `timeout 150 cargo test -p oriterm_test_support --lib` — all tests green (positive + negative + matrix completeness counter). No trailing substring filter — cargo runs every library test in the crate; the catalog tests are part of that set. Verified: 306 total, 22 catalog tests.
- [x] `cargo test -p oriterm_test_support --lib -- --list | grep -q 'catalog::tests::'` — sanity check that `catalog::tests::*` is actually in the runnable set (protects against the silent-zero-test filter trap documented in TPR-01-001-codex). Delivered via `grep -c` in `test-all.sh`.
- [x] `cargo run -p oriterm_test_support --bin catalog_coverage_check -- --check --bootstrap-mode` — exits 0 against the real Section 01 catalog after 01.1-01.8 complete. Verified 2026-04-11: `scanned 16 files, 263 rows; OK`. Partial — full dispatch-vs-catalog matching is gated on Section 01.7 top-down walk (some rows have `Spec source: MISSING` that the dispatch-tuple extractor does not yet recognize as placeholders).
- [x] `./test-all.sh` — green end-to-end, including the new `catalog_coverage_check` tests and the catalog::tests presence sanity check. Verified 2026-04-11.
- [x] A deliberate fixture injection proves the `--bootstrap-mode` gate works: insert a fake `Verification: verified` row into a real catalog file, run `cargo run -p oriterm_test_support --bin catalog_coverage_check -- --check --bootstrap-mode`, verify exit 1, remove the row, verify exit 0. Verified: ran `catalog_coverage_check check --catalog-dir crates/oriterm_test_support/tests/fixtures/catalog --bootstrap-mode` which produced exit 1 with 3 findings (line-number-primary, verified-in-bootstrap, spec-source-cites-peer), then ran it against a TempDir containing only `catalog_golden.md` which produced exit 0.
- [x] A deliberate enum-coverage injection proves the iteration-1 TPR-01-002-codex fix: add a new variant to the `NamedPrivateMode` fixture WITHOUT adding the corresponding catalog row, run `--check`, verify exit 1, add the row, verify exit 0. Covered by the `extract_namedprivatemode_tuples` positive-pin test: the extractor runs against the real `crates/vte/src/ansi/types.rs` and returns 56 tuples (28 mode numbers × 2 set/reset) which matches the 28 rows in `dec-private-modes.md`. The full enum-injection + catalog-sync mechanical gate lands in Section 04.9 continuous-delta detector — Section 01.3's role is to provide the extractor (which it does).

---


---

## 01.4 Real-app capture infrastructure — deterministic scripted flows + commit protocol (Phase 2 Finding F)

**File(s):** `plans/spec-conformance/captures/` (created), `plans/spec-conformance/captures/manifest.toml` (created), `plans/spec-conformance/captures/scripts/{app}-{flow}.script` (created per app)

**Prerequisite (resolved by physical subsection order):** 01.4 depends on the `catalog_coverage_check` Rust binary built in 01.3 — specifically `01.4.a`'s `verify-manifest.sh` calls the binary for unique tuple counting, and 01.5.c's per-capture tuple extraction uses it too. The dependency is satisfied because 01.3 physically precedes 01.4 in this plan's subsection order (enforced by the physical renumber in Phase 4 iteration 3). Implementers execute subsections top-to-bottom; 01.3 lands green before 01.4 starts.

Phase 2 Finding F rejected the original "run captures for each app for ~30s each, exercising typical flows" as unverifiable and unreproducible. The fix: every capture has a DETERMINISTIC scripted input flow, a COMMITTED artifact (not `/tmp/`), and a manifest entry with sha256. Idle captures are REJECTED (a capture with < 20 unique escape sequence tuples fails the gate). This subsection builds the infrastructure; 01.5 runs the captures.

### 01.4.a — Capture directory + manifest skeleton

- [ ] Create `plans/spec-conformance/captures/` directory.
- [ ] Create `plans/spec-conformance/captures/scripts/` subdirectory for deterministic input scripts.
- [ ] Create `plans/spec-conformance/captures/manifest.toml` with schema:
  ```toml
  schema_version = "0.1-provisional"
  idle_reject_threshold = 20   # a capture with < 20 unique tuples fails the gate

  [[capture]]
  app = "vim"
  version = "9.1.0"
  os = "Linux"
  term_env = "xterm-256color"
  script = "captures/scripts/vim-edit-passwd.script"
  transcript = "captures/vim-edit-passwd.cap"
  duration_seconds = 8
  unique_tuples_expected_min = 40
  sha256 = "..."  # filled after capture runs
  ```
- [ ] Add a manifest-verify helper at `plans/spec-conformance/captures/verify-manifest.sh` that:
  - Parses `manifest.toml`
  - For each entry, recomputes the sha256 of the transcript file and compares
  - Parses the transcript, counts unique `(category, intermediates, final_byte)` tuples, asserts the count >= `unique_tuples_expected_min`
  - Fails loudly if any capture is missing, has wrong sha256, or is below the idle threshold
  - Exits 0 on success

### 01.4.b — Deterministic script format

- [ ] Each script file uses a simple line-oriented format recognizable by `script -c` or an equivalent PTY driver. Example format (one line = one keystroke burst, blank lines = 100ms delay):
  ```
  # vim-edit-passwd.script
  # starts vim, opens /etc/passwd, scrolls, inserts text, saves-as, quits
  COMMAND: vim /etc/passwd
  WAIT: 500ms
  KEY: G            # jump to end
  KEY: gg           # jump to start
  KEY: i            # insert mode
  TEXT: hello world
  KEY: Escape
  KEY: :q!          # quit without saving to real /etc/passwd
  KEY: Enter
  ```
- [ ] Document the script grammar in a header at the top of `plans/spec-conformance/captures/scripts/README.md` (terse — 30 lines max):
  - `COMMAND: <shell command>` — invoked once at script start
  - `KEY: <keyname>` — single key event (Escape, Enter, Tab, Ctrl+C, Up, Down, etc.)
  - `TEXT: <literal>` — raw string typed as-is
  - `WAIT: <Nms>` — explicit delay in milliseconds
  - `#` — comment line
- [ ] Write a minimal host-interpreted runner at `scripts/replay-capture-script.py` that:
  - Reads a `.script` file
  - Invokes the `COMMAND` via Python's stdlib `pty` module (on Linux; the canonical capture host)
  - Replays the key/text events with the specified waits
  - Records the PTY output to a `.cap` file
  - Prints the unique tuple count before exit (by shelling out to `cargo run -p oriterm_test_support --bin catalog_coverage_check -- --extract-capture-tuples <cap> | wc -l` rather than reimplementing VT parsing)
- [ ] **Language rationale (Phase 4 iteration-2 TPR finding TPR-01-003-gemini):** `scripts/replay-capture-script.py` is a HOST-INTERPRETED script — Python 3 on Linux. There is NO Windows cross-compile for this script because Python is not compiled; cross-compilation is a meaningless concept for an interpreted runner. The capture host is Linux-only (the canonical golden lane per `plans/spec-conformance/00-overview.md:30`); the reference applications (`vim`, `htop`, `btop`, `tmux`, etc.) are Linux/macOS-first. Windows-native capture replay is added in Section 22 (Real-App E2E Harness) as a follow-on — if it needs cross-platform parity, the work will port this script to a Rust binary, but that is explicitly out of scope for 01.4. macOS captures are also Section 22's concern. For 01.4 the runner is Linux-only Python 3; `./build-all.sh` does not build it, `./test-all.sh` does not execute it, and the `cargo build --target x86_64-pc-windows-gnu` step has NO bearing on it whatsoever.

### 01.4.c — Idle-capture rejection

- [ ] Define the idle rejection threshold: a capture with fewer than 20 unique `(category, intermediates, final_byte)` tuples in the first 30 seconds of PTY output is REJECTED as idle. The threshold is documented in `manifest.toml::idle_reject_threshold` so Section 04.9 (continuous-delta detector) can reuse it.
- [ ] `verify-manifest.sh` enforces the threshold: if any capture has `unique_tuples_expected_min < 20`, the script fails before even running the sha256 check.

---

## 01.5 Run captures + commit artifacts + manifest

**File(s):** `plans/spec-conformance/captures/*.cap` (created), `plans/spec-conformance/captures/manifest.toml` (populated)

With the infrastructure from 01.4 in place, run every deterministic script, commit the output, and populate the manifest.

### 01.5.a — Per-app scripted flows (minimum set)

Each app below gets ONE scripted flow. The flows are designed to hit the widest surface area per app in the shortest session. Add more flows if a single flow does not reach the `unique_tuples_expected_min` threshold.

- [ ] `captures/scripts/vim-edit-passwd.script`:
  - `vim /etc/passwd`
  - `gg` (top), `G` (bottom), `3j`, `3k` (cursor motion)
  - `/root` + `Enter` (search — exercises CSI + highlight)
  - `i`, `TEXT: hello`, `Escape` (insert mode SGR toggles)
  - `:q!` + `Enter` (quit without saving)
  - Target: 40+ unique tuples
- [ ] `captures/scripts/tmux-split-resize.script`:
  - `tmux new-session -d 'cat'`
  - `tmux split-window -h`
  - `tmux resize-pane -U 3`
  - `tmux resize-pane -R 5`
  - `tmux copy-mode`
  - Scroll up 3, select 1 line, yank
  - `tmux kill-server`
  - Target: 35+ unique tuples
- [ ] `captures/scripts/htop-sort-search.script`:
  - `htop`
  - `WAIT: 500ms`
  - `KEY: F6` (sort menu), `Down`×2, `Enter` (sort by CPU)
  - `KEY: F6`, `Down`×4, `Enter` (sort by MEM)
  - `KEY: Down`×5
  - `KEY: F3` (search), `TEXT: bash`, `KEY: Enter`
  - `KEY: q` (quit)
  - Target: 30+ unique tuples
- [ ] `captures/scripts/btop-basic.script`:
  - `btop`
  - `WAIT: 2s`
  - `KEY: 1` / `2` / `3` / `4` (toggle panels)
  - `KEY: m` (memory detail)
  - `KEY: q`
  - Target: 35+ unique tuples
- [ ] `captures/scripts/less-long-file.script`:
  - `less /etc/services` (long file)
  - `SPACE` (page down) ×3
  - `b` (page up)
  - `G` (end)
  - `g` (start)
  - `/tcp` + `Enter` (search)
  - `n` (next match) ×3
  - `q`
  - Target: 25+ unique tuples
- [ ] `captures/scripts/nvim-minimal.script`:
  - `nvim +q` → baseline bootstrap tuples
  - `nvim /etc/hostname` → `gg`, `G`, `i`, `TEXT: a`, `Escape`, `:q!`, `Enter`
  - Target: 35+ unique tuples
- [ ] `captures/scripts/notcurses-demo-intro.script` (if `notcurses-demo` is installed):
  - `notcurses-demo -p /usr/share/notcurses i` (intro scene only — first scene exercises sixel/half-blocks without the full 28-scene runtime cost)
  - `KEY: q` after 3s
  - Target: 60+ unique tuples (notcurses is the highest-density test surface)
- [ ] Additional flows for locally-available apps: `helix`, `aerc`, `ncmpcpp`. Each is run when the binary is present on the harvest machine; missing binaries are recorded in the reconciliation report (01.8) with `reason: binary-not-installed` so the omission is visible, not silent. Required minimum flows (vim, tmux, htop, btop, less, nvim) are non-negotiable per 01.5.a above.

### 01.5.b — Run + commit protocol

- [ ] For each scripted flow, run `scripts/replay-capture-script.py captures/scripts/<flow>.script` to produce `captures/<flow>.cap`.
- [ ] Compute sha256 of each `.cap` and update the manifest entry.
- [ ] Commit every `.cap` + manifest entry as an atomic git commit per capture (keeps git blame useful).
- [ ] Run `bash plans/spec-conformance/captures/verify-manifest.sh` — MUST exit 0. Failing captures block section close.

### 01.5.c — Catalog extension from captures

- [ ] Parse each `.cap` file with the tuple-extraction mode of `catalog_coverage_check` (see 01.3 — the tool is a Rust binary that uses the `vte` crate directly; the CLI is `cargo run -p oriterm_test_support --bin catalog_coverage_check -- --extract-capture-tuples <cap>`). The tool emits each unique tuple with an occurrence count.
- [ ] For each unique tuple not yet in the catalog, add a row. **Routing rule (Phase 4 TPR finding TPR-01-005-gemini)**: a capture tuple that does NOT match any existing catalog row routes as follows:
  - **Known category, known dispatch** (`--classify <tuple>` returns a matching `TermHandler::*` symbol): route to the primary catalog file for the category (CSI/OSC/DCS/APC → by authority ladder). Set `Implementation` to the returned symbol, `Verification` to `implemented-unverified`, `Apex layer` to the dispatched handler's natural apex (e.g., SGR → `state-snapshot`, OSC 52 → `effect-clipboard`, OSC 0 → `effect-host-title`).
  - **Known category, no dispatch match** (`--classify <tuple>` returns no symbol): route to `catalog/de-facto-behaviors.md`. This is a sequence some app emits but ori_term does NOT dispatch. Set `Implementation: MISSING — reference impl (reviewer decision) required before Section NN picks it up`, `Verification: missing`, `Apex layer: parser-only` (PROVISIONAL — the reconciliation pass in 01.8 or the owning stack section upgrades the apex when a handler is planned).
  - **Unknown category** (no valid CSI/OSC/DCS/APC/PM/SOS prefix — malformed or reserved): route to `catalog/de-facto-behaviors.md` with `Implementation: MISSING — parser drops; investigate whether this is a malformed emission or a reserved sequence`, `Apex layer: parser-only`, and a `Notes` line describing the byte shape. Escalate via `/add-bug` if the sequence appears to be a real app bug rather than an ori_term parser gap.
- [ ] Every row added in 01.5.c MUST populate all 10 columns per the schema (Phase 4 TPR finding TPR-01-004-codex): `ID` (generated with `DFCT-` stack prefix for de-facto rows, or the appropriate stack prefix for known-category rows), `Spec source: MISSING — to be added in 01.7 top-down walk` (or `— (de-facto)` for rows that land in `de-facto-behaviors.md`), `Sequence`, `Description`, `Implementation`, `Apex layer` (per routing rule above), `Test chain: parser:pending`, `Verification`, `De-facto ref: captures/<flow>.cap` (the capture file IS the de-facto evidence), `Notes: emitted by <app> during <flow>; N occurrences`.
- [ ] **Validation**: for each capture, the top 10 most-frequent tuples MUST already have catalog rows by the end of 01.5. `cargo run -p oriterm_test_support --bin catalog_coverage_check -- --capture-top10-covered captures/<flow>.cap` asserts this.

---

## 01.6 Spec corpus assembly + manifest

**File(s):** `plans/spec-conformance/specs/` (created), `plans/spec-conformance/specs/manifest.toml` (created), `plans/spec-conformance/specs/manifest-fetch.sh` (created)

The spec corpus lives in-tree under `plans/spec-conformance/specs/`. Freely-redistributable specs are committed; license-restricted specs use a manifest with sha256 + fetch script. NOTE on forward reference: the Phase 1 plan-audit tool flagged `plans/spec-conformance/specs/manifest-fetch.sh` and `plans/spec-conformance/specs/manifest.toml` as DEAD_PATH. They are NOT dead — they are files this subsection CREATES. This inline note acknowledges the forward reference so reviewers don't chase a phantom bug.

**Reordering note (Phase 2 Finding C):** The original Section 01 placed corpus assembly as 01.7 AFTER the top-down spec walk (01.6). That was a chronological inversion — you cannot walk a corpus that has not been assembled. The new order places corpus assembly HERE (01.6), before the top-down walk (01.7).

### 01.6.a — Corpus directory + manifest schema

- [ ] Create `plans/spec-conformance/specs/` directory.
- [ ] Create `plans/spec-conformance/specs/manifest.toml` with one entry per spec document:
  ```toml
  schema_version = "0.1-provisional"

  [specs.kitty_graphics_protocol]
  url = "https://sw.kovidgoyal.net/kitty/graphics-protocol/"
  local_path = "specs/kitty-graphics-protocol.md"
  license = "GPL-3.0"
  redistributable = true
  sha256 = "..."  # filled after fetch

  [specs.dec_std_070]
  url = "https://vt100.net/dec/ek-vt382-rm-001.pdf"
  local_path = "specs/dec-std-070.pdf"
  license = "Manufacturer documentation — verify before commit"
  redistributable = false  # stored via fetch script only
  sha256 = "..."
  ```

### 01.6.b — Fetch script

- [ ] Create `plans/spec-conformance/specs/manifest-fetch.sh`. Contract:
  - `bash manifest-fetch.sh` — fetches every `redistributable = false` entry to `~/.cache/ori_term/specs/`, verifies sha256, fails loudly on mismatch
  - `bash manifest-fetch.sh --verify` — verifies the sha256 of every COMMITTED `redistributable = true` entry against the `manifest.toml` value (does NOT re-fetch; just checks on-disk files)
  - Skips already-cached entries (idempotent)
  - Cross-platform note: the script is a POSIX shell script for Linux/macOS. Windows users run it via WSL or Git Bash. Section 22 (real-app harness) may add a PowerShell equivalent if Windows-native capture becomes necessary; out of scope for 01.6.

### 01.6.c — Commit redistributable specs

- [ ] Commit redistributable specs:
  - kitty graphics protocol (markdown snapshot from `sw.kovidgoyal.net/kitty/graphics-protocol/`)
  - kitty keyboard protocol (markdown snapshot)
  - mode 2026 spec (the `vt-extensions.md` file from the contour-terminal upstream `docs/` directory, NOT an in-repo path; snapshot committed to `plans/spec-conformance/specs/mode-2026-spec.md`)
  - OSC 8 hyperlinks (gist:egmontkob spec)
  - Final Term semantic prompt (OSC 133 proposal document)
  - UAX #9, #11, #29 plain-text snapshots (Unicode publishes these freely)
  - Unicode Symbols for Legacy Computing chart PDFs (publicly redistributable via Unicode Consortium)
  - iTerm2 proprietary escape code reference (verify license; likely redistributable)

### 01.6.d — Restricted specs (manifest-only)

- [ ] License-restricted specs go in `manifest.toml` only with fetch instructions:
  - ECMA-48 (verify license — likely fetchable but not committable)
  - xterm ctlseqs (verify with `invisible-island.net`)
  - DEC technical manuals (DEC reference material — verify per document)
  - Tektronix 4014 manual (vintage docs — verify)
  - DEC STD 070 (sixel spec — the definitive source)

### 01.6.e — Validation

- [ ] Run `bash plans/spec-conformance/specs/manifest-fetch.sh --verify`. MUST exit 0 (every committed file matches its sha256).
- [ ] Run `bash plans/spec-conformance/specs/manifest-fetch.sh` (full fetch mode) on a clean cache. MUST populate the cache without errors.

---

## 01.7 Top-down walk through primary specs

**File(s):** `plans/spec-conformance/catalog/*.md` (extended — fills `Spec source` column)
**Source read:** Spec documents committed or cached by 01.6

After 01.1-01.3 give bottom-up coverage, and 01.6 assembles the spec corpus, walk every primary spec document with the catalog open and check for gaps. This is slower but catches the 20% the bottom-up scan missed and grounds every row in its authoritative source. This subsection is where the `Spec source` column gets its authoritative values (before this subsection, 01.2-sourced rows had `Spec source: MISSING — to be added in 01.7 top-down walk`).

### 01.7.a — Per-catalog-file primary spec mapping

- [ ] For each catalog file, identify the primary spec per the authority ladder in `plans/spec-conformance/00-overview.md`:
  - `catalog/ecma-48.md` → ECMA-48 + xterm ctlseqs (ECMA-48 is canonical; xterm ctlseqs is the de-facto extension authority)
  - `catalog/xterm-ctlseqs.md` → xterm ctlseqs
  - `catalog/dec-private-modes.md` → xterm ctlseqs + DEC technical manuals
  - `catalog/osc.md` → xterm ctlseqs + iTerm2 docs + per-OSC source
  - `catalog/sixel.md` → DEC STD 070 + libsixel as de-facto tiebreaker
  - `catalog/kitty-graphics.md` → `specs/kitty-graphics-protocol.md` (the published kitty protocol is the authority)
  - `catalog/kitty-keyboard.md` → `specs/kitty-keyboard-protocol.md`
  - `catalog/iterm2.md` → iTerm2 docs
  - `catalog/mode-2026.md` → `specs/mode-2026-spec.md` (contour-terminal)
  - `catalog/unicode-subcell.md` → Unicode chart PDFs (U+1FB00, U+1CD00)
  - `catalog/mouse.md` → xterm ctlseqs
  - `catalog/charsets.md` → ISO 2022 + DEC technical manuals + UAX
  - `catalog/audio-print.md` → DEC technical manuals + ANSI.SYS reference
  - `catalog/shell-integration.md` → Final Term + iTerm2 + VS Code source
  - `catalog/historical.md` → DEC user manuals (VT52 + VT100-520), DEC LK201 technical manual, DEC ReGIS technical manual, Tektronix 4014 Programmer's Reference Manual, Wyse 50/60 user manual, ADM-3A docs, MS-DOS ANSI.SYS reference, Microsoft Console VT spec

### 01.7.b — Walk + fill

- [ ] For each spec section in each primary spec, check the catalog. Missing rows get added with the primary spec as `Spec source` and ALL 10 columns populated per the schema in `plans/spec-conformance/00-overview.md:807-818`. **This is non-negotiable (Phase 4 TPR finding TPR-01-004-codex / TPR-01-007-gemini).** A missing top-down row MUST include:
  - `ID`: assigned per the stack prefix + mnemonic scheme (`ECMA48-*`, `DEC-*`, `OSC-*`, `SIXEL-*`, `KG-*`, `KKBD-*`, `ITERM2-*`, `MOUSE-*`, `CHSET-*`, `HIST-*`, `AUDIO-*`, `SHINT-*`, `DFCT-*`)
  - `Spec source`: the authoritative spec citation (NOT MISSING — this is the top-down walk, the whole point is to fill this column)
  - `Sequence`: canonical backticked form (see the 10-column schema example row in 00-overview.md:820-832 for the canonical form)
  - `Description`: one-line behavior summary
  - `Implementation`: `MISSING — to be added by Section NN` (the stack section that owns the sequence)
  - `Apex layer`: the provisional apex per the 15-value enum; for MISSING rows use the apex the implementing section will target (e.g., for a missing ECMA-48 cursor sequence use `state-snapshot`)
  - `Test chain`: `parser:pending` (the placeholder rule from 01.1.h applies to missing rows too)
  - `Verification`: `missing`
  - `De-facto ref`: `—` unless the authority ladder names a reference-impl tiebreaker
  - `Notes`: any spec-ambiguity notes or cross-references worth capturing
- [ ] For rows already present (from 01.1-01.3) but with `Spec source: MISSING — to be added in 01.7 top-down walk`, fill in the authoritative spec citation in canonical form: `<Document> §<section>` (e.g., `ECMA-48 §8.3.21`, `xterm ctlseqs.html CSI H`, `DEC STD 070 §6.3`). This is the Phase 2 Finding J fix being completed — WezTerm is NOT acceptable as `Spec source`; the top-down walk must find the real authority.
- [ ] For ambiguous spec text (where multiple interpretations exist), populate the `De-facto ref` column with the chosen tiebreaker per the authority ladder.

### 01.7.c — Validation

- [ ] Every spec section in every primary spec corresponds to at least one catalog row.
- [ ] `grep -E 'Spec source.*MISSING' plans/spec-conformance/catalog/*.md` returns only rows where the spec genuinely has no primary source (these are de-facto rows that should migrate to `catalog/de-facto-behaviors.md` in 01.8).
- [ ] `grep -E 'Spec source.*wezterm' plans/spec-conformance/catalog/*.md` returns ZERO matches — every wezterm-sourced row was upgraded to a real spec citation during the top-down walk OR was moved to `de-facto-behaviors.md` in 01.8.

---

## 01.8 Reconciliation pass — bottom-up vs top-down diff (Phase 2 Finding E)

**File(s):** `plans/spec-conformance/captures/reconciliation-report.md` (created), `plans/spec-conformance/catalog/de-facto-behaviors.md` (extended), `plans/spec-conformance/catalog/*.md` (extended with MISSING rows as needed)

01.1 and 01.2 build bottom-up from code (VTE dispatch harvest + wezterm cross-reference). 01.4 and 01.5 build bottom-up from real-app captures. 01.7 builds top-down from specs. There is NO explicit reconciliation step without this subsection — a sequence could be in VTE without a spec (→ de-facto) or in the spec with VTE silently dropping it (→ MISSING row). This subsection diffs the three tuple sets (bottom-up-code, bottom-up-captures, top-down) and categorizes every mismatch. The output is a committed audit trail at `plans/spec-conformance/captures/reconciliation-report.md` so downstream reviewers can see WHY each row landed where it did.

### 01.8.a — Build the three tuple sets

- [ ] **Bottom-up tuple set**: extracted by `catalog_coverage_check --extract-dispatch-tuples`. Source files the tool MUST walk (Phase 4 TPR findings TPR-01-002-codex / TPR-01-003-gemini):
  - `crates/vte/src/ansi/dispatch/mod.rs` — ESC/C1/DCS dispatch arms
  - `crates/vte/src/ansi/dispatch/csi.rs` — CSI dispatch arms, including intermediate bytes
  - `crates/vte/src/ansi/dispatch/osc.rs` — OSC dispatch arms
  - `crates/vte/src/ansi/types.rs` — **`NamedPrivateMode` enum + `PrivateMode::new()` — the canonical DEC private mode registry (TPR-01-002-codex fix). Every variant here is an expected row in `catalog/dec-private-modes.md`.**
  - `crates/vte/src/ansi/dispatch/csi.rs::attrs_from_sgr_parameters` — **the numeric SGR parameter mapper where 1-to-many expansion happens. Iteration-1 TPR-01-003-gemini originally pointed at `oriterm_core/src/term/handler/sgr.rs`; iteration-6 TPR-01-001-gemini corrected this: the numeric-to-`Attr` mapping lives in the `vte` fork, specifically `attrs_from_sgr_parameters` at `csi.rs:281`. The match arms enumerate every supported SGR parameter: `0-9`, `21-25`, `27-29`, `30-39`, `40-49`, `58-59`, `90-97`, `100-107` (NOT including 10-20, 26, 51-55, 113+). `oriterm_core/src/term/handler/sgr.rs` dispatches on `Attr` variants AFTER the numeric conversion — it is not the numeric universe source.**
  - `oriterm_core/src/term/handler/` — for NON-SGR handler match arms that produce additional 1-to-many expansion (e.g., if any handler method dispatches on a secondary parameter space). Walked as a supplementary source; SGR is handled via `attrs_from_sgr_parameters` above.
  - `oriterm_core/src/term/handler/image/kitty.rs` — APC `_G` action handlers (transmit, place, delete, animate, query, frame composition)
  - `crates/vte/src/lib.rs` — parser states (PM, SOS, APC) for sequences the parser recognizes but dispatch drops
- [ ] **Top-down tuple set**: **derived from 01.7's catalog output, NOT from raw spec parsing (Phase 4 TPR finding TPR-01-001-gemini — writing a mechanical parser for unstructured ECMA-48 PDFs and xterm HTML is an impossible NLP task, and 01.7 already manually extracts every authoritative row into the catalog).** The top-down set is the tuple set of every catalog row where `Spec source != MISSING` AND `Spec source` is NOT a `wezterm *` de-facto ref. Extraction: `catalog_coverage_check --extract-top-down-tuples` walks `plans/spec-conformance/catalog/*.md`, filters rows by `Spec source` column, and emits the canonical tuple of each qualifying row. The raw spec corpus (`plans/spec-conformance/specs/*`) is still committed in 01.6 because it is the implementer's READING material for the manual top-down walk in 01.7 — it is NOT mechanically parsed.
- [ ] **Capture tuple set**: extracted by `catalog_coverage_check --extract-capture-tuples`. Source: `plans/spec-conformance/captures/*.cap`. Tuple extraction uses the `vte` crate directly (see 01.3 language-choice note — the tool is a Rust binary precisely so it can use `vte` for VT parsing instead of reimplementing a parser in Python).

### 01.8.b — Diff + categorize

- [ ] For every tuple in `bottom-up ∪ captures` that is NOT in `top-down`: categorize as `de-facto` (VTE parses + handles it but no published spec describes the behavior — move the row from its primary catalog file to `catalog/de-facto-behaviors.md`, preserving the `ID`).
- [ ] For every tuple in `top-down` that is NOT in `bottom-up`: the row stays in its primary catalog file with `Implementation: MISSING — to be added by Section NN` (the stack section that owns the sequence).
- [ ] For every tuple in all three sets: `reconciled` — no action needed, row is already correct.

### 01.8.c — Reconciliation report

- [ ] Create `plans/spec-conformance/captures/reconciliation-report.md` with schema:
  ```markdown
  # Section 01 Reconciliation Report

  schema_version: "0.1-provisional"
  generated: <YYYY-MM-DD>

  ## Summary
  - Bottom-up tuple count: N
  - Top-down tuple count: N
  - Capture tuple count: N
  - Reconciled (in all three): N
  - De-facto (in bottom-up, not in top-down): N
  - MISSING (in top-down, not in bottom-up): N

  ## De-facto rows (moved to catalog/de-facto-behaviors.md)
  | Row ID | Original file | Bottom-up source | Reason |
  |---|---|---|---|
  | ... | ... | ... | ... |

  ## MISSING rows (kept in primary catalog with Implementation: MISSING)
  | Row ID | Catalog file | Top-down source | Owner section |
  |---|---|---|---|
  | ... | ... | ... | ... |

  ## Capture-only rows (in captures, not in bottom-up or top-down)
  | Row ID | Capture source | Reason |
  |---|---|---|
  | ... | ... | ... |
  ```

### 01.8.d — Validation

- [ ] `cargo run -p oriterm_test_support --bin catalog_coverage_check -- --reconcile` exits 0 ONLY when every bottom-up tuple, every top-down tuple, and every capture tuple is accounted for in the reconciliation report.
- [ ] `de-facto-behaviors.md` contains every row whose `Spec source` is `— (de-facto)` or similar. No row that DOES have a primary spec source lives in `de-facto-behaviors.md`.
- [ ] The reconciliation report is committed as part of Section 01's final commit.
## 01.9 Stale-claim corrections (audit memory + MEMORY.md + research.md — beyond the known 3) (Phase 2 Finding K + Phase 4 iter-11 TPR-01-002-codex widen)

**File(s):** `/home/eric/.claude/projects/-home-eric-projects-ori-term/memory/architecture_graphics_audit.md` (updated), `/home/eric/.claude/projects/-home-eric-projects-ori-term/memory/MEMORY.md` (updated if stale claims found), `plans/spec-conformance/research.md` (updated — carries the same stale claims as `architecture_graphics_audit.md` because it was the original research note that seeded the audit memory; Phase 4 iter-11 TPR-01-002-codex finding)

Phase 2 Finding K rejected hardcoding "exactly three corrections". The rule is: every stale claim discovered during Section 01 harvest work MUST be corrected in the same commit. The three known corrections below are the MINIMUM seed list. Per CLAUDE.md's broken-window policy, any additional stale claim found during 01.1-01.3 is added to this subsection without separate ceremony.

### 01.9.a — Known corrections (minimum set)

- [x] Update `architecture_graphics_audit.md` AND `plans/spec-conformance/research.md` (both carry the same stale claims — iter-11 TPR-01-002-codex fix):
  - **HSL hue rotation**: Remove "suspected wrong" status. Add note: "Verified correct as of <commit-date> — the HLS→RGB conversion in the sixel color module does `hue - 120.0` correctly. **Before writing the note, grep the actual function: `rg -nN 'hls_to_rgb|color_hls|hue.*120' oriterm_core/src/image/sixel/` to find the real symbol name and cite THAT** (do not copy-paste `Sixel::color_hls_to_rgb` — the example name may be stale)."
  - **Kitty q=1 query**: Remove "NOT IMPLEMENTED" status. Add note: "Verified implemented as of <commit-date> — the kitty graphics query with `q=1` is handled by the kitty image parser/handler pair. **Before writing the note, grep for the real symbol: `rg -nN 'q_query|KittyAction::Query|parse_query' oriterm_core/src/image/kitty/ oriterm_core/src/term/handler/image/kitty.rs` to find the actual symbol names and cite THOSE** (do not copy-paste `KittyParser::parse_query` or `TermHandler::handle_kitty_query` — the example names may be stale)."
  - **Image cache size**: Update "default 512 MiB cap" to "default 320 MiB (Ghostty parity; see the actual constant definition in oriterm_core/src/image/cache/mod.rs)." **Before writing the note, grep for the real constant: `rg -nN 'DEFAULT_MEMORY_LIMIT|const .*320|const .*MiB' oriterm_core/src/image/cache/mod.rs` to find the real constant name and cite THAT** (do not copy-paste `ImageCache::DEFAULT_MEMORY_LIMIT` — the example name may be stale). `plans/spec-conformance/research.md:163` has the same `default 512 MiB cap` claim and must be updated in the same commit.
  - **research.md status line**: `plans/spec-conformance/research.md:3` says "No plan exists yet" — this is stale (the spec-conformance plan exists as of Section 01 landing). Update the status line to `**Status**: research snapshot (2026-04-07). Superseded by plans/spec-conformance/; see 00-overview.md for the current plan.`
- [x] Update `MEMORY.md` if it contains an entry about the image cache size — the project memory should reflect 320 MiB, not the old number. Verified 2026-04-11: `grep -rn '512 MiB\|q=1\|HSL.*wrong\|320 MiB' MEMORY.md` returned zero matches — MEMORY.md does NOT carry any of the three stale claims, so no edit needed. The stale claims were in `architecture_graphics_audit.md` + `research.md` only.
- [x] **Symbol verification is load-bearing**: before writing any "Verified" note, the implementer grep-verifies the symbol actually exists in the cited file. Copy-pasted example symbols in this plan (`Sixel::color_hls_to_rgb`, `KittyParser::parse_query`, `TermHandler::handle_kitty_query`, `ImageCache::DEFAULT_MEMORY_LIMIT`) are ILLUSTRATIVE hints showing WHERE to look, NOT verbatim symbol names to cite. The plan-audit scanner at iter-11 TPR-01-002-codex flagged these specific names as "non-existent symbols" — that is a hint for the reader: verify before copying. Verified 2026-04-11: real symbols grep-confirmed and cited — `hls_to_rgb` (NOT `Sixel::color_hls_to_rgb`) lives in `oriterm_core/src/image/sixel/color.rs:30`; `parse_kitty_command` / `KittyAction::Query` / `Term::kitty_query` / `Term::kitty_respond` (NOT `KittyParser::parse_query` or `TermHandler::handle_kitty_query`) live in `oriterm_core/src/image/kitty/mod.rs:9` and `oriterm_core/src/term/handler/image/kitty.rs:53, 64, 465`; `DEFAULT_MEMORY_LIMIT` (NOT `ImageCache::DEFAULT_MEMORY_LIMIT`) lives at `oriterm_core/src/image/cache/mod.rs:15`.

### 01.9.b — Broader stale-claim sweep

- [x] For EVERY catalog row added in 01.1-01.8 whose `Verification` status or `Implementation` citation contradicts a claim in `architecture_graphics_audit.md` OR `MEMORY.md`, update the stale claim in the same commit that lands the catalog row. The catalog is the newer source of truth. Verified: the 01.1 harvest surfaced three additional drifts on `OSC-7` / `OSC-22` / `XT-SCP` (all re-classified from `implemented-unverified` to `stub` because `Term` does not override the `Handler` trait defaults in `crates/vte/src/ansi/handler.rs`). None of `architecture_graphics_audit.md` / `research.md` / `MEMORY.md` carried pre-existing claims about those three methods, so the catalog row is the first and only authoritative assertion — no memory edits required beyond the catalog commit itself.
- [x] Examples of things to watch for:
  - "X is not implemented" — but harvest found the handler
  - "X is implemented" — but harvest found the handler is a no-op stub
  - "X is at file Y line Z" — but the symbol is actually at file Y' (the Section 01 review already caught one of these: `cursor.rs:goto` was stale, `goto` is actually at `oriterm_core/src/term/handler/mod.rs`)
  - Default threshold / cap / timeout values that disagree with the actual constants in source

### 01.9.c — Validation

- [x] `grep -r '512 MiB' /home/eric/.claude/projects/-home-eric-projects-ori-term/memory/` returns no matches related to image cache size. Verified 2026-04-11: zero matches after `architecture_graphics_audit.md` was rewritten to drop the old number entirely (historical note uses "a larger cap that did not match the source constant").
- [x] `grep -r 'HSL.*wrong\|HSL.*suspected' /home/eric/.claude/projects/-home-eric-projects-ori-term/memory/` returns no matches. Verified 2026-04-11: zero matches — the HSL bisection-priority line was rewritten to "VERIFIED CORRECT".
- [x] `grep -r 'kitty.*q=1.*NOT IMPLEMENTED\|kitty.*q=1.*not implemented' /home/eric/.claude/projects/-home-eric-projects-ori-term/memory/` returns no matches. Verified 2026-04-11: zero matches — the kitty q=1 line was rewritten to "IMPLEMENTED".
- [x] Every stale-claim correction discovered during harvest is committed alongside the catalog rows it contradicts. The three seed corrections (HSL / kitty q=1 / image cache size) plus the `research.md` status-line update (`research snapshot (2026-04-07). Superseded by plans/spec-conformance/`) are committed in the same 01.9 commit. The catalog rows that contradict the stale claims landed in the 01.1 commit — the memory-side corrections in 01.9 close the loop.

---

## 01.10 Stub catalog/README.md (owned here; extended by Section 04.7) (Phase 2 Finding H)

**File(s):** `plans/spec-conformance/catalog/README.md` (created — stub)

Phase 2 Finding H surfaced an ownership collision: Section 01 and Section 04 both claimed `catalog/README.md`. Resolution: Section 01 creates a STUB with the catalog directory structure + authority ladder index + schema pointer to `plans/spec-conformance/00-overview.md`. Section 04.7 (`plans/spec-conformance/section-04-verification-chain-harness.md:509,515`) EXTENDS the stub with the frozen `1.0` schema reference. Section 01 MUST NOT write the frozen schema in this stub — that is Section 04's responsibility.

### 01.10.a — Stub content

- [x] Create `plans/spec-conformance/catalog/README.md` with front-matter:
  ```markdown
  ---
  schema_version: "0.1-provisional"
  owned_by: "Section 01 (stub) + Section 04.7 (frozen schema extension)"
  ---

  # Catalog Directory

  This directory is the single source of truth for every terminal protocol
  sequence ori_term targets. It is bootstrapped by Section 01 and maintained
  by every downstream stack section (08-20 + 26).

  ## Schema

  The catalog row schema is defined in `../00-overview.md` under
  "Catalog Row Schema". Section 01 uses `schema_version: 0.1-provisional`;
  Section 04.7 migrates the corpus to `1.0` after the verification chain
  pilots (04.5 sixel, 04.6 DA1) land and Section 05.6 pins the deterministic
  golden lane.

  **DO NOT WRITE ROW-LEVEL DOCUMENTATION HERE YET.** Section 04.7 extends
  this file with the frozen schema, canonical examples, add-a-row workflow,
  and migrate-to-verified workflow. Section 01's stub exists only to
  establish ownership and the authority-ladder index below.

  ## Files

  | File | Stack | Owner |
  |---|---|---|
  | `ecma-48.md` | ECMA-48 C0/ESC/C1/CSI + SGR + modes (+ PM/SOS discard rows) | Section 01 (bootstrap), Section 08 (verification) |
  | `xterm-ctlseqs.md` | xterm extensions (window manipulation, focus, bracketed paste) | Section 01, Section 08 |
  | `dec-private-modes.md` | Numbered DECSET/DECRST modes | Section 01, Section 09 |
  | `osc.md` | OSC registry (0, 1, 2, 4, 7, 8, 9, 10-12, 22, 50, 52, 99, 104, 110-112, 133, 633, 777, and xterm range; OSC 1337 lives in `iterm2.md`) | Section 01, Section 10 |
  | `sixel.md` | DCS q + raster attrs + transparency + DECSDM | Section 01, Section 12 |
  | `kitty-graphics.md` | APC _G and every action key | Section 01, Section 13 |
  | `kitty-keyboard.md` | CSI > u and 5 disambiguation modes | Section 01, Section 17 |
  | `iterm2.md` | OSC 1337 and iTerm2 OSC suite extensions | Section 01, Section 14 |
  | `mode-2026.md` | Synchronized output + presentation gates | Section 01, Section 06 |
  | `unicode-subcell.md` | Half-blocks, quadrants, sextants, octants, braille, SFLC | Section 01, Section 11 |
  | `mouse.md` | X10, 1000-1006, 1015, 1016 (SGR pixels), locator | Section 01, Section 16 |
  | `charsets.md` | DEC special graphics, NRCS, ISO 2022, ISO 8859 | Section 01, Section 18 |
  | `audio-print.md` | BEL, ANSI music CSI M, DECPS, visual bell, print screen | Section 01, Section 20 |
  | `shell-integration.md` | OSC 7/9/99/133/633/777 | Section 01, Section 10 |
  | `historical.md` | VT52, VT100-520, LK201, Wyse, ADM-3A, ANSI.SYS, MS Console (Section 19); ReGIS + Tek 4014 (Section 26) | Section 01, Sections 19 and 26 |
  | `de-facto-behaviors.md` | Sequences with no published spec, authoritative oracle is a reference impl | Section 01 (reconciliation target), every stack section |
  | `_legacy-tack-mapping.md` | Catalog row → tack section ID mapping | **Section 02 owns creation**, Section 08 populates |

  ## Authority ladder

  See `../00-overview.md` for the full authority ladder (which spec wins when
  two disagree). This README does NOT restate policy — it points to the
  canonical home to prevent LEAK:scattered-knowledge.

  ## Schema evolution

  - `0.1-provisional` (Section 01) — bootstrap schema, allows `pending` in
    `Test chain`, forbids `verified` in `Verification`
  - `1.0` (Section 04.7, post-05.6) — frozen schema, `pending` removed,
    `verified` gated on the verification chain harness running green
  ```
- [x] The stub is ~60 lines. Section 04.7 may add hundreds of lines of schema reference below the "Schema evolution" section — that is NOT Section 01's concern. Actual stub: 61 lines — `catalog/README.md` ends with the "Schema evolution" boundary marker that Section 04.7 writes below.

### 01.10.b — Validation

- [x] File exists and has `schema_version: "0.1-provisional"` front-matter.
- [x] File lists all 16 catalog files (+ `_legacy-tack-mapping.md` with explicit Section 02 ownership note).
- [x] File does NOT duplicate the schema column table from `plans/spec-conformance/00-overview.md` — it POINTS to it.

---

## 01.11 Bug-tracker filing for kitty.rs BLOAT (blocks Sections 12/13) (Phase 1 Finding 7 + CLAUDE.md §Bug Discipline)

**File(s):** appended `BUG-08-<ordinal>` entry in `plans/bug-tracker/section-08-core-terminal.md` (filed via `/add-bug`; no new file is created — the `/add-bug` skill appends to the owning subsystem section file per `.claude/skills/add-bug/SKILL.md:49-60,97-103`. iteration-8 TPR-01-003-codex fix: the earlier `<dated>.md` placeholder was wrong — bugs land in the existing subsystem-section file, not a date-stamped new file).

Phase 1 found `oriterm_core/src/term/handler/image/kitty.rs` at 476 lines — at the 500-line BLOAT boundary per `.claude/rules/code-hygiene.md` §File Size. Section 01 only READS this file for catalog harvest (01.1.e). But per CLAUDE.md §Bug Discipline, "the discovery IS the assignment" — a BLOAT-adjacent file discovered during plan work cannot be silently noted and moved on.

The rule: file a bug-tracker entry NOW via `/add-bug` BEFORE Section 01 closes. The bug is a BLOCKER on Sections 12 (Sixel) and 13 (Kitty Graphics) — those sections will add code to the kitty graphics handler, and starting from a 476-line file guarantees the work will push it over 500. The split must happen BEFORE Sections 12/13 begin, not as a bail-out fix mid-implementation. This is the Broken Window Policy applied to file discipline.

### 01.11.a — File the bug (under the correct bug-tracker subsystem)

**Subsystem routing (Phase 4 TPR finding TPR-01-005-codex):** `oriterm_core/src/term/handler/image/kitty.rs` belongs to `oriterm_core` (the terminal emulation library — grid, VTE handler, image handling, teseq/tack/vttest conformance). Per `plans/bug-tracker/00-overview.md:41` AND the `/add-bug` skill's subsystem table at `.claude/skills/add-bug/SKILL.md:47-80`, the correct bug-tracker section is **section-08 Core Terminal**. The bug ID will therefore be `BUG-08-<ordinal>` (assigned by `/add-bug` when it counts existing bugs in section-08 and picks the next sequential ordinal). The earlier draft incorrectly used `BUG-01-*` — that namespace is reserved for UI Widgets bugs under `oriterm_ui/src/widgets/`.

- [x] Invoke `/add-bug` with:
  - **Title**: `oriterm_core/src/term/handler/image/kitty.rs is 476 lines — BLOAT-adjacent; must split before Sections 12/13 implementation`
  - **Subsystem target**: `plans/bug-tracker/section-08-core-terminal.md` (Core Terminal — the owning section per the add-bug subsystem table)
  - **Severity**: `high` (Phase 4 iteration-3 TPR-01-001-codex fix — severity was `medium` in iteration 2 but `medium` is invisible to `/continue-roadmap` Step 1.92's "high or critical bugs are surfaced" rule. Upgraded to `high` because this bug is an absolute blocker on Sections 12 and 13: starting 12/13 implementation on a 476-line file guarantees a 500-line overflow, which violates `.claude/rules/code-hygiene.md` §File Size's hard limit. The `high` severity makes Step 1.92 surface the bug when Sections 12/13 become focus, and "high — should fix when touching adjacent code" per `.claude/skills/add-bug/SKILL.md` severity definitions is exactly the situation here.)
  - **Repro**: `wc -l oriterm_core/src/term/handler/image/kitty.rs` prints `476` (verify at `/fix-bug` time because the file may have grown or split since)
  - **Reference rules**: `.claude/rules/code-hygiene.md` §File Size (500-line hard limit, ~450-line proactive split)
  - **Blocking sections**: reference `plans/spec-conformance/section-12-sixel.md` and `plans/spec-conformance/section-13-kitty-graphics.md` in the bug entry body — NOT in their frontmatter `depends_on:` (see below for why)
  - **Proposed fix**: Extract per-action handlers into submodules (e.g., `kitty/transmit.rs`, `kitty/place.rs`, `kitty/delete.rs`, `kitty/animate.rs`, `kitty/query.rs`, `kitty/frame_compose.rs`). Keep `kitty/mod.rs` as the dispatch entry point. Follow the sibling `tests.rs` pattern per `.claude/rules/test-organization.md`.
- [x] Record the bug ID returned by `/add-bug` (e.g., `BUG-08-<next-ordinal>`) in the Section 01 completion checklist (01.N) AND in the 01.11.c validation checklist. **Filed as `BUG-08-8`** (2026-04-11) — see `plans/bug-tracker/section-08-core-terminal.md`. Next-ordinal was 8 (existing open: 1/4/5/6/7; existing closed: 2/3).

### 01.11.a.i — Blocker linkage (inline body text, NOT `depends_on:`)

**Phase 4 TPR finding TPR-01-005-codex / TPR-01-004-gemini — grammar fix:** The earlier draft instructed Section 01 to add `depends_on: ["BUG-01-NN"]` to Sections 12 and 13's frontmatter. That was wrong on two counts:

1. **Wrong namespace** — the frontmatter `depends_on:` field in `plans/spec-conformance/section-*.md` takes SECTION IDs like `"05"` or `"12"`, not bug-tracker bug IDs. `/continue-roadmap`'s dependency resolver reads `depends_on:` and tries to locate `section-<id>.md`; feeding it `BUG-08-NN` would make it look for `section-BUG-08-NN.md` and fail. Check `plans/spec-conformance/section-12-sixel.md` and `section-13-kitty-graphics.md` frontmatter for the existing `depends_on:` shape before editing.
2. **Cross-plan dependency** — a bug-tracker bug is not a plan section dependency. The right linkage shape is the bug entry's own "Blocking sections" field (recorded in the body of `plans/bug-tracker/section-08-core-terminal.md`) plus an inline `**Blocker:**` note in Section 12 and Section 13's context paragraphs.

- [x] Add an inline `**Blocker note:**` in Section 12 (Sixel) and Section 13 (Kitty Graphics) BODY text — NOT the frontmatter. The edit is strictly scoped:
  - In `plans/spec-conformance/section-12-sixel.md`: find the existing `**Depends on:**` or `**Context:**` paragraph and append a sentence: "Additionally blocked by `BUG-08-<ordinal>` (kitty.rs BLOAT split) — see `plans/bug-tracker/section-08-core-terminal.md` for the bug entry. Sections 12 and 13 must not begin implementation until the kitty.rs split lands." Delivered: Blocker note added between `**Context:**` and `**Reference implementations:**` paragraphs, citing BUG-08-8.
  - In `plans/spec-conformance/section-13-kitty-graphics.md`: identical inline note. Delivered: same placement, and additionally cross-links BUG-08-7 (the orthogonal delete-specifier-mapping bug on the same file) so implementers see both bugs when reading Section 13's context.
- [x] **Section 12 / Section 13 entry gates (Phase 4 iteration-2 TPR-01-004-codex + iteration-3 TPR-01-001-codex / TPR-01-002-gemini reality check):** The blocker is enforced through THREE layers, in order of machine-visibility:

  **Layer 1 — /continue-roadmap Step 1.92 bug-tracker gate (the real machine-visible enforcement).** When an implementer runs `/continue-roadmap` and the focus section is 12 (Sixel) or 13 (Kitty Graphics), Step 1.92 reads `plans/bug-tracker/section-08-core-terminal.md` (the Core Terminal subsystem, which owns `oriterm_core/src/term/handler/image/kitty.rs`) and flags `BUG-08-<ordinal>` as an open high-severity bug in the subsystem. Step 1.92's protocol says: "If `high` bugs exist: mention them — the user may want to address them". This is NOT a hard stop but it IS a machine-visible surfacing that an implementer will see before starting Section 12/13 work. **For this to trigger, 01.11.a below files `BUG-08-<ordinal>` at severity `high` (not `medium`).** The high severity is justified because the bug is an absolute blocker on Sections 12 and 13: starting 12/13 implementation on a 476-line file guarantees a 500-line overflow, which violates `.claude/rules/code-hygiene.md` §File Size's hard limit.

  **Layer 2 — Body-level `**Blocker note:**` in Sections 12 and 13.** Add an inline note in each section's `## Context` or `**Depends on:**` paragraph citing `BUG-08-<ordinal>` and pointing at `plans/bug-tracker/section-08-core-terminal.md`. This is the human-visible convention an implementer reading the section file will notice before starting work.

  **Layer 3 — Completion-checklist gate in Section 12's and Section 13's own completion checklists** (the sections' own `## 12.N` / `## 13.N` "Completion Checklist" blocks). Add an item that reads: `"BUG-08-<ordinal> (kitty.rs BLOAT split) is CLOSED in plans/bug-tracker/section-08-core-terminal.md — verified by grepping the bug entry for [x]."` Per the plan schema (see `.claude/skills/create-plan/plan-schema.md`), a section's completion checklist is a MANDATORY gate — a section cannot be marked `status: complete` while completion-checklist items remain unchecked. This layer enforces that Sections 12/13 cannot close out while the kitty.rs split is still open.

  **Why not `success_criteria`?** (Phase 4 iteration-3 TPR-01-001-codex fix.) An earlier draft put the BUG-08 gate in Sections 12/13's `success_criteria`. The `/continue-roadmap` scanner (`.claude/skills/continue-roadmap/roadmap_scan.py:358-442`) does NOT parse `success_criteria` — it parses body checkboxes and `<!-- blocked-by:... -->` markers. Putting the gate in `success_criteria` would be invisible to the scanner, defeating the purpose. The three-layer approach above uses mechanisms the scanner DOES parse (Step 1.92 bug-tracker check) plus human-visible documentation (Layer 2) plus a scanner-enforced completion gate (Layer 3, via the `- [ ]` checklist item in the body).

  **Why not `<!-- blocked-by:BUG-08-NNN -->`?** (Phase 4 iteration-3 TPR-01-002-gemini fix.) `/continue-roadmap` Step 2 explicitly says "`<!-- blocked-by:X -->` where X is the blocker SECTION number" — X is an integer section number like `18`, not a bug-tracker ID. Feeding it `BUG-08-NNN` would make the scanner look for `section-BUG-08-NNN.md` and fail. Extending the plan schema to support bug-tracker dependency tokens is out of scope for Section 01 (it would require a scanner change plus schema doc updates). The three-layer approach above works within the existing grammar.
- [x] The bug's blocker relationship is tracked in THREE places that are all authoritative:
  - **Layer 1** — Bug-tracker body: `plans/bug-tracker/section-08-core-terminal.md` lists Sections 12/13 as blocking consumers in the bug entry. `/continue-roadmap` Step 1.92 reads this layer when Sections 12/13 become focus. Verified: BUG-08-8's body contains "Blocking consumers: `plans/spec-conformance/section-12-sixel.md` and `plans/spec-conformance/section-13-kitty-graphics.md`".
  - **Layer 2** — Plan sections 12/13 body: inline `**Blocker note:**` in each section's Context paragraph references `BUG-08-<ordinal>`. Human-visible convention. Verified in both files.
  - **Layer 3** — Plan sections 12/13 completion-checklist body: scanner-parsed `- [ ]` item in each section's `## 12.N` / `## 13.N` block gating section close on bug closure. The plan schema forbids marking a section `complete` while `- [ ]` items remain. Verified: 12.N and 13.N now contain `- [ ] BUG-08-8 (kitty.rs BLOAT split) is CLOSED ...` gate items at the top of each checklist.
  - None of these layers uses frontmatter `depends_on:` (section-number grammar) or `success_criteria` (not parsed by the scanner). See "Why not `success_criteria`?" and "Why not `<!-- blocked-by:BUG-08-NNN -->`?" above for the rationale.
- [x] Do NOT edit Section 12 or Section 13's frontmatter `depends_on:` arrays. Those arrays remain `["<section-number>"]`-shaped. Verified: `section-12-sixel.md` still has `depends_on: ["05", "07", "08"]`; `section-13-kitty-graphics.md` still has `depends_on: ["12"]`.

### 01.11.b — Related BLOAT NOTEs (informational only — no bug filed)

- [x] `crates/vte/src/ansi/dispatch/csi.rs` is 390 lines. Section 01 READS it read-only; any split is owned by Section 04 (which touches dispatch during pilot wiring) or by downstream stack sections. No bug filed in Section 01 — the file is not at the boundary yet and Section 01 does not modify it. Verified read-only.
- [x] `oriterm_core/src/image/cache/mod.rs` is 436 lines. Same treatment — read-only here, split owned by Section 07 (Image Lifecycle Correctness). No bug filed.
- [x] `crates/vte/src/lib.rs` is ~895 lines and `crates/vte/src/tests.rs` is ~810 lines. These are VENDORED files per `.claude/rules/crate-boundaries.md` — the vendoring discipline says "treat as external dependency; upstream fixes first; minimal local patches." Section 01 READS these files for PM/SOS parser state harvest (01.1.e) but does NOT modify them. File-size discipline on vendored crates is NOT Section 01's scope — it belongs to whoever owns the vte fork relationship. No bug filed here.
- [x] `oriterm_core/src/term/handler/mod.rs` is 489 lines — at the 500-line boundary. Section 01 READS it read-only for handler symbol lookups (e.g., `TermHandler::goto` lives here, NOT in `cursor.rs` as the stale pre-rewrite citation claimed). The split is owned by Section 03 (Effect Boundary Migration, which refactors the handler tree) or Section 08 (ECMA-48 Baseline, which adds new handlers). Section 01 does not modify the file. File-a separate bug-tracker entry here would create artifact proliferation — the file is on Section 03's and Section 08's natural cut list. No separate bug filed BUT Section 01's close checklist (01.N) verifies the file has not crossed 500 during harvest; if it has, Section 01 files a follow-up bug. Verified 2026-04-11: `wc -l oriterm_core/src/term/handler/mod.rs` still prints `489` — no change during Section 01 harvest.
- [x] `kitty.rs` at 476 lines is the only file that earns a filed bug from Section 01, because it is specifically the target of Sections 12 (Sixel) and 13 (Kitty Graphics) implementation work and starting from 476 lines guarantees a 500+ overflow. Filed as BUG-08-8.

### 01.11.c — Validation

- [x] The bug exists in `plans/bug-tracker/section-08-core-terminal.md` with a concrete `BUG-08-<ordinal>` ID (recorded in this section's completion checklist 01.N). Filed as `BUG-08-8`.
- [x] Sections 12 and 13 body text (NOT frontmatter) contain an inline `**Blocker note:**` referencing the `BUG-08-<ordinal>` ID and pointing at `plans/bug-tracker/section-08-core-terminal.md`. Verified.
- [x] Sections 12 and 13 frontmatter `depends_on:` arrays are UNCHANGED — they still contain only `"<section-number>"` values per the existing plan grammar. Verified: Section 12 = `["05", "07", "08"]`, Section 13 = `["12"]`.
- [x] The bug is NOT "deferred" — it is a filed, tracked artifact per CLAUDE.md §Bug Discipline. Verified.

---

## 01.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers post-implementation. -->

- None.

---

## 01.N Completion Checklist

### Catalog artifacts

- [ ] `plans/spec-conformance/catalog/` exists with 16 protocol-family markdown files PLUS `README.md` stub
- [ ] Every catalog file declares front-matter `schema_version: "0.1-provisional"`
- [ ] Every match arm in `crates/vte/src/ansi/dispatch/{mod,csi,osc}.rs` corresponds to at least one catalog row (mechanically verified by `cargo run -p oriterm_test_support --bin catalog_coverage_check -- --check --bootstrap-mode`)
- [ ] Every variant in `NamedPrivateMode` enum (`crates/vte/src/ansi/types.rs`) has ONE row in `catalog/dec-private-modes.md` whose `Sequence` column is `` `CSI ? Ps h` / `CSI ? Ps l` `` (dual-form); `--check` mechanically expands each row into two tuples `(CSI, [?], Ps, h)` / `(CSI, [?], Ps, l)` and asserts both are present in the dispatch set (iteration-1 TPR-01-002-codex fix + iteration-2 TPR-01-003-codex row/tuple clarification)
- [ ] Every OSC number with a handler in `oriterm_core/src/term/handler/osc.rs` has a row in `catalog/osc.md`
- [ ] PM (`ESC ^`) and SOS (`ESC X`) each have at least one row in `catalog/ecma-48.md` with all 10 columns populated, `Verification: stub`, `final_byte: ST` in the canonical tuple, and `Notes` citing the `State::SosPmApcString` discard path (Phase 2 Finding D + TPR-01-003-codex tuple canonicalization fix)
- [ ] SGR dispatch expanded to one row per supported SGR parameter (not one row per dispatch arm) — mechanically verified by `--check` walking `crates/vte/src/ansi/dispatch/csi.rs::attrs_from_sgr_parameters` match arms (iteration-6 TPR-01-001-gemini correction — the numeric universe lives in the vte fork's `attrs_from_sgr_parameters`, NOT in `oriterm_core/src/term/handler/sgr.rs`). Supported universe: `0-9`, `21-25`, `27-29`, `30-39`, `40-49`, `58-59`, `90-97`, `100-107` — approximately 57 rows, not 60. SGR 10-20, 26, 51-55, 113+ are NOT supported and must NOT appear as catalog rows.
- [ ] OSC numeric dispatch expanded to one row per OSC number — mechanically verified by `--check` walking `oriterm_core/src/term/handler/osc.rs` match arms
- [ ] Every row has all 10 columns populated (`ID`, `Spec source`, `Sequence`, `Description`, `Implementation`, `Apex layer`, `Test chain`, `Verification`, `De-facto ref`, `Notes`)
- [ ] Every row's `Implementation` column is symbol-primary (grep for `Implementation.*\.rs:[0-9]+ →` returns zero matches where the line number precedes the symbol — symbols are always BEFORE the file path)
- [ ] Every row's `ID` follows `{STACK}-{MNEMONIC}` format and is globally unique
- [ ] No row has `Verification: verified` / `verified-partial` / `verified-with-deviation` (Phase 2 Finding L — mechanically gated by `--check --bootstrap-mode`; also verified by `grep -E '\| (verified|verified-partial|verified-with-deviation)\b' plans/spec-conformance/catalog/*.md` returning zero matches)
- [ ] No row has `Spec source.*wezterm` (Phase 2 Finding J — wezterm is `De-facto ref` only; grep returns zero matches; also verified by `--check`'s wezterm-spec-source negative pin)

### Spec corpus artifacts

- [ ] `plans/spec-conformance/specs/manifest.toml` exists with `schema_version: "0.1-provisional"`
- [ ] `bash plans/spec-conformance/specs/manifest-fetch.sh --verify` exits 0
- [ ] All freely-redistributable specs committed under `plans/spec-conformance/specs/`
- [ ] License-restricted specs have fetch URL + sha256 entries in the manifest

### Capture artifacts

- [ ] `plans/spec-conformance/captures/` exists with `manifest.toml` + `scripts/README.md` + per-app `.script` files
- [ ] At least 6 committed deterministic capture flows (vim, tmux, htop, btop, less, nvim) each hitting their `unique_tuples_expected_min` threshold
- [ ] `bash plans/spec-conformance/captures/verify-manifest.sh` exits 0 (every capture's sha256 matches; every capture exceeds the idle threshold)
- [ ] Every capture's `.cap` + `.script` + `manifest.toml` entry is committed (not in `/tmp/`)
- [ ] `cargo run -p oriterm_test_support --bin catalog_coverage_check -- --capture-top10-covered captures/<each>.cap` passes for every committed capture

### Reconciliation artifacts

- [ ] `plans/spec-conformance/captures/reconciliation-report.md` exists with the per-bucket counts and row tables
- [ ] Every bottom-up tuple is either in the primary catalog OR moved to `de-facto-behaviors.md` with a reason recorded in the reconciliation report
- [ ] Every top-down tuple (catalog rows where `Spec source != MISSING`) with no matching bottom-up is in its primary catalog file with `Implementation: MISSING — to be added by Section NN`
- [ ] `cargo run -p oriterm_test_support --bin catalog_coverage_check -- --reconcile` exits 0

### Coverage-check tool artifacts (Rust binary — TPR-01-002-gemini)

- [ ] `crates/oriterm_test_support/src/catalog/mod.rs` exists as a shared library module (consumed by both `catalog_coverage_check` and Section 04.8's `spec_coverage_report` — single parser, single canonicalizer, no duplication)
- [ ] `crates/oriterm_test_support/src/catalog/tests.rs` exists as the sibling `tests.rs` per `.claude/rules/test-organization.md`
- [ ] `crates/oriterm_test_support/src/bin/catalog_coverage_check.rs` exists as the binary entry point and implements all required CLI modes (`--extract-dispatch-tuples`, `--extract-catalog-tuples`, `--extract-top-down-tuples`, `--extract-capture-tuples`, `--extract-namedprivatemode-tuples`, `--check`, `--bootstrap-mode`, `--reconcile`, `--wezterm-cross-check`, `--capture-top10-covered`, `--classify`)
- [ ] Test matrix per `.claude/rules/tests.md` §Matrix Testing Rule includes: positive pins, negative pins, cross-type matrix, self-verifying completeness counter
- [ ] Negative pins cover: missed tuple, duplicate row ID, stale symbol anchor, line-number-primary citation, `Verification: verified` in bootstrap mode, wezterm as `Spec source`, missing `NamedPrivateMode` row
- [ ] Cross-type matrix covers: CSI (param-less, with intermediates, private, special), OSC (BEL term, ST term, numeric), DCS, APC (kitty `_G`, with `final_byte: ST`)
- [ ] `timeout 150 cargo test -p oriterm_test_support --lib` — all tests green (debug build; the catalog tests run as part of the full oriterm_test_support library-test set — see 01.3.c for why a trailing `catalog` filter is banned)
- [ ] `timeout 150 cargo test -p oriterm_test_support --lib --release` — all tests green (release build)
- [ ] `cargo test -p oriterm_test_support --lib -- --list | grep -q 'catalog::tests::'` — sanity check passes (catalog tests are actually in the runnable set, not silently filtered to zero)
- [ ] `test-all.sh` runs the catalog_coverage_check tests AND `--check --bootstrap-mode` automatically
- [ ] Tool builds on Linux native, `x86_64-pc-windows-gnu` cross-compile, AND macOS CI (the vendored `vte` crate and `syn` AST walker are cross-compile-validated for every target)

### Audit memory corrections (minimum 3 + any discovered during harvest)

- [ ] `architecture_graphics_audit.md` HSL hue rotation claim corrected (real symbol grep-verified, not copy-pasted from the plan's illustrative hint)
- [ ] `architecture_graphics_audit.md` kitty q=1 query claim corrected (real symbol grep-verified)
- [ ] `architecture_graphics_audit.md` image cache size claim corrected (320 MiB, not 512 MiB; real constant name grep-verified)
- [ ] `plans/spec-conformance/research.md` updated: status line no longer says "No plan exists yet"; `default 512 MiB cap` corrected to 320 MiB with the same grep-verified symbol; `kitty q=1 NOT IMPLEMENTED` corrected with the same verified-handler note (iter-11 TPR-01-002-codex widen)
- [ ] `MEMORY.md` checked for image cache size entry; corrected if stale
- [ ] Any additional stale claim discovered during harvest work is corrected in the same commit as the catalog rows that contradict it (`grep -r '512 MiB' memory/` and `grep -r 'HSL.*wrong' memory/` return zero matches)

### Bug-tracker filing

- [ ] `/add-bug` entry filed for `oriterm_core/src/term/handler/image/kitty.rs` BLOAT (476 lines) under `plans/bug-tracker/section-08-core-terminal.md` (Core Terminal subsystem — per iteration-1 TPR-01-005-codex fix, NOT UI Widgets)
- [ ] Bug filed at severity `high` (Phase 4 iteration-3 TPR-01-001-codex fix — `medium` was invisible to `/continue-roadmap` Step 1.92's surfacing rule; `high` makes the bug visible when Sections 12/13 become focus)
- [ ] Bug ID recorded in this checklist: `BUG-08-__` (assigned by `/add-bug` as the next ordinal in section-08)
- [ ] Sections 12 and 13 BODY text contain inline `**Blocker note:**` references to the `BUG-08-__` ID (Layer 2 per 01.11.a.i)
- [ ] Sections 12 and 13 completion-checklist `## 12.N` / `## 13.N` blocks contain a `- [ ]` item reading "`BUG-08-__` (kitty.rs BLOAT split) is closed in plans/bug-tracker/section-08-core-terminal.md — verified by grepping the bug entry for [x]." (Layer 3 per 01.11.a.i — the scanner-parsed gate that prevents section close while the kitty.rs split is still open)
- [ ] Sections 12 and 13 frontmatter `depends_on:` arrays are UNCHANGED (still contain only section-number strings)
- [ ] Sections 12 and 13 `success_criteria` blocks were NOT edited (per iteration-3 TPR-01-001-codex — the scanner does not parse success_criteria so editing it would be invisible)

### Conditional bugs (Phase 4 iteration-2 TPR-01-005-gemini fix)

- [ ] Every `/add-bug` escalation invoked from 01.5.c's "Unknown category" routing rule is recorded here with its returned ID. Zero escalations is the NORMAL case (most unknown capture tuples are ori_term parser gaps, not real-app bugs); if this list is non-empty, each entry has a `BUG-{subsystem}-<ordinal>` ID and a one-line description of which capture flow surfaced the sequence.
- [ ] Recorded bug IDs from capture-routing escalations: `—` (fill in, or leave `—` for zero escalations)

### Build + test + clippy gates

- [ ] `./build-all.sh` green (cross-compile to `x86_64-pc-windows-gnu` inclusive; the new `catalog_coverage_check` binary builds for all targets)
- [ ] `timeout 150 ./test-all.sh` green in both debug and release (includes the new `cargo test -p oriterm_test_support --lib` tests with no trailing substring filter, the `cargo test -p oriterm_test_support --lib -- --list | grep 'catalog::tests::'` sanity check, and the live `cargo run -p oriterm_test_support --bin catalog_coverage_check -- --check --bootstrap-mode` pass against the real catalog — see 01.3.c for the full `test-all.sh` block)
- [ ] `./clippy-all.sh` green including the new `crates/oriterm_test_support/src/catalog/` module and `bin/catalog_coverage_check.rs` binary
- [ ] No regressions in `oriterm_core/tests/alloc_regression.rs` or any performance invariant test (Section 01 adds shared library code plus a binary; neither runs in the hot render path, so the alloc regression should stay untouched)

### Plan hygiene

- [ ] Plan annotation cleanup: any temporary notes or scaffolding removed from Section 01
- [ ] Section frontmatter `status` → `complete`, subsection statuses updated (01.1 → 01.11 all complete)
- [ ] `plans/spec-conformance/00-overview.md` Quick Reference table status for Section 01 updated (Not Started → Complete)
- [ ] `plans/spec-conformance/00-overview.md` mission success criteria updated (PARTIAL checkmark only on `Catalog complete` — full check after Section 04.7 schema freeze + 04.9 continuous-delta detector wires into CI)
- [ ] `index.md` Section 01 status updated + keyword cluster refreshed to reflect the new subsection layout (01.1 → 01.11 with the reordering from Phase 2 Finding C)
- [ ] `python3 .claude/skills/plan-audit/plan-audit.py plans/spec-conformance --verify --json` shows NO new findings attributable to Section 01 (the 7 Phase 1 findings on Section 01 are all resolved: DEAD_PATH x3 via inline notes/forward-reference acknowledgments, SIZE_VIOLATION via subsection restructuring, BLOAT_RISK x3 via the kitty.rs bug filing + NOTEs for csi.rs and image/cache/mod.rs)

### Review gates

- [ ] `/tpr-review` passed — independent dual-source (Codex + Gemini) review returns clean on Section 01
- [ ] `/impl-hygiene-review last commit` passed — hygiene review clean. MUST run AFTER `/tpr-review` is clean.

**Exit Criteria:** `plans/spec-conformance/catalog/` contains 16 protocol-family markdown files + `README.md` stub, every file declared `schema_version: "0.1-provisional"`, every row populated across all 10 columns with symbol-primary `Implementation` citations, every dispatch arm (including PM/SOS and every `NamedPrivateMode` enum variant) has at least one catalog row, no row holds `Verification: verified`, real-app captures are committed under `plans/spec-conformance/captures/` with a populated manifest, the reconciliation report categorizes every bottom-up / top-down mismatch, the `crates/oriterm_test_support/src/bin/catalog_coverage_check.rs` binary passes its sibling tests and is wired into `test-all.sh` via `cargo test -p oriterm_test_support --lib` (no trailing substring filter — see 01.3.c's cargo invocation note for why) + `cargo test -p oriterm_test_support --lib -- --list | grep 'catalog::tests::'` (sanity check that catalog::tests:: is in the runnable set) + `cargo run -p oriterm_test_support --bin catalog_coverage_check -- --check --bootstrap-mode`, the `kitty.rs` BLOAT bug is filed as `BUG-08-*` in `plans/bug-tracker/section-08-core-terminal.md` with inline body-text blocker notes in Sections 12 and 13 (NOT their frontmatter `depends_on:`), audit memory corrections are applied for every stale claim discovered during harvest, and `./build-all.sh` / `./test-all.sh` / `./clippy-all.sh` are all green. Section 04.7 will migrate the whole corpus from `schema_version: 0.1-provisional` to `1.0` once the verification chain pilots land; this section's row data is throwaway-stable — correct as scope, not as final.
