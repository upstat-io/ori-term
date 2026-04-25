---
section: "02"
title: "oriterm_ipc Cleanup"
status: not-started
reviewed: false
goal: "Drive `cargo clippy -p oriterm_ipc --all-targets --target {host,x86_64-pc-windows-gnu} -- -D warnings` to exit 0 by fixing 8 violations (5 redundant_clone, 2 manual_assert, 1 doc_markdown), AND resolve the `oriterm_ipc/Cargo.toml [lints]` divergence (keep + document, or migrate to `workspace = true` with per-file `#[allow(unsafe_code)]`)."
success_criteria:
  - "`cargo clippy -p oriterm_ipc --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` exits 0"
  - "`cargo clippy -p oriterm_ipc --all-targets --target x86_64-pc-windows-gnu -- -D warnings` exits 0"
  - "`cargo test -p oriterm_ipc` green; no test regression introduced by lint fixes"
  - "`oriterm_ipc/Cargo.toml [lints]` decision committed: either (a) divergent block retained with a `# SSOT exemption: ...` comment explaining the necessity, OR (b) migrated to `workspace = true` with per-file `#[allow(unsafe_code)]` annotations replacing the workspace override"
  - "Closes BUG-07-NNN+1 (filed in Section 01.2) — remains `[ ]` until Section 10 marks it `[x] Superseded by:`"
  - "Connects upward to mission criteria: 'workspace clippy clean' for both targets"
inspired_by:
  - "plans/tack-conformance/section-04-scenario-framework.md (per-crate `cargo clippy -p {crate} --all-targets` checkpoint pattern, `:269,545,558,827,1026`)"
depends_on: ["01"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "02.1"
    title: "Auto-fix sweep with manual diff review"
    status: not-started
  - id: "02.2"
    title: "Manual cleanup of remaining structural violations"
    status: not-started
  - id: "02.3"
    title: "Resolve `[lints]` divergence in oriterm_ipc/Cargo.toml"
    status: not-started
  - id: "02.4"
    title: "Cross-target verification"
    status: not-started
  - id: "02.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "02.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 02: oriterm_ipc Cleanup

**Status:** Not Started
**Goal:** First crate cleaned in dependency order. Establishes the per-crate cleanup template (auto-fix → diff review → manual cleanup → cross-target verification → exit gate) that Sections 03-07 follow. Resolves the SSOT-divergent `[lints]` block question that affects no other crate.

**Success Criteria:**
- [ ] `cargo clippy -p oriterm_ipc --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` exits 0
- [ ] `cargo clippy -p oriterm_ipc --all-targets --target x86_64-pc-windows-gnu -- -D warnings` exits 0
- [ ] `cargo test -p oriterm_ipc` green
- [ ] `oriterm_ipc/Cargo.toml [lints]` block: either retained with documented exemption OR migrated to workspace inheritance
- [ ] Connects upward: "Workspace clippy clean on host AND Windows GNU"

**Context:** oriterm_ipc has 8 clippy violations (smallest crate in the cluster). Top: `redundant_clone × 5` (`oriterm_ipc/tests/ipc_roundtrip.rs:88,122,172,283`), `manual_assert × 2` (`tests/ipc_roundtrip.rs:322,340`), `doc_markdown × 1` (`tests/ipc_roundtrip.rs:377`). Section 02 also resolves a SSOT concern from Phase 1 research: `oriterm_ipc/Cargo.toml` has a divergent `[lints]` block (NOT `workspace = true`) declared specifically to allow `unsafe_code` for the platform IPC FFI surface. This is a deliberate divergence but undocumented; either it gets a `# SSOT exemption:` comment, or it migrates to `workspace = true` with per-file `#[allow(unsafe_code)]` annotations on the FFI files (`mio` integration, raw socket/named-pipe handling).

**Reference implementations:**
- `plans/tack-conformance/section-04-scenario-framework.md:269,545,558,827,1026,1851` — per-crate `cargo clippy -p oriterm_test_support --all-targets` checkpoint pattern, the only prior workspace use of `--all-targets` per crate
- `oriterm_ipc/tests/ipc_roundtrip.rs:88-340` — test file containing 7 of 8 violations; the doc-comment violation is at `:377`

**Depends on:** Section 01 (baseline.md and BUG-07-NNN+1 entry filed; Section 02 cannot start without the bug-tracker artifact to progress against).

---

## Intelligence Reconnaissance

Queries run 2026-04-25:

- `scripts/intel-query.sh` — not present in this project; queries below used Grep / Glob / Read / cargo clippy --message-format=json instead.
- `Grep oriterm_ipc/tests/` — 1 test file (`ipc_roundtrip.rs`, ~400 lines) carries 7 of 8 violations. Violation density: ~1 per 50 LOC. Exclusively in test code.
- `Read oriterm_ipc/Cargo.toml` — confirmed `[lints]` block (NOT `workspace = true`); allows `unsafe_code` while copying every other workspace lint via local declarations. `unsafe_code` is needed for the raw fd/handle FFI in `mio` integration.
- `Grep -rn 'unsafe' oriterm_ipc/src/` — verifies `unsafe` blocks only appear in platform-FFI code paths (Unix domain sockets, Windows named pipes); not in protocol logic.

Results summary (≤500 chars) [ori]: oriterm_ipc 8 violations are 7 in test code (`ipc_roundtrip.rs`) + 1 doc-comment. `[lints]` divergence is intentional (FFI requires `unsafe_code`); migrate-vs-keep is a per-file annotation tradeoff. Mechanical lint set: M=3 (manual_assert, doc_markdown), S=5 (redundant_clone — verify each isn't reused-after).

See _(intel graph not available in this project; use Grep/Glob)_ for the full query protocol.

---

## 02.1 Auto-fix sweep with manual diff review

**File(s):** `oriterm_ipc/tests/ipc_roundtrip.rs` (auto-fix lands here)

- [ ] Snapshot pre-fix state: `git status -- oriterm_ipc/` should be clean.
- [ ] Run auto-fix: `cargo clippy --fix --all-targets -p oriterm_ipc --target x86_64-unknown-linux-gnu --allow-dirty`. The `--allow-dirty` flag accepts that the workspace already has uncommitted plan files.
- [ ] Capture the diff: `git diff -- oriterm_ipc/ > /tmp/oriterm_ipc-autofix.diff` for review.
- [ ] **Manual diff review** — read every hunk:
  - For `manual_assert` rewrites: verify `panic!` → `assert!` preserves the panic message format and is reachable on the same condition. The two cases at `:322` and `:340` both check `remaining.is_zero()`.
  - For `doc_markdown` rewrite: verify backtick-wrapping doesn't break a doc-link or cross-reference.
  - For any `redundant_clone` rewrites the auto-fix decided to apply: verify the value isn't reused after the removal point. If reused, REVERT that hunk (auto-fix is wrong here).
  - **Specifically watch**: any `manual_let_else` rewrite (Gemini round-1 concern). If found, verify drop timing of the `else` branch's diverging value matches the original `match`/`if let` semantics. None of the cataloged 8 violations are `manual_let_else`, so this should be a no-op for this crate.
- [ ] Run `cargo test -p oriterm_ipc` — all tests pass post-fix.
- [ ] Commit: `chore(oriterm_ipc): apply cargo clippy --fix for mechanical lint cleanup`.

- [ ] **Subsection close-out (02.1)**:
  - [ ] Diff review complete; commit landed
  - [ ] Status → `complete`
  - [ ] `/improve-tooling`: did the auto-fix workflow benefit from a wrapper (e.g., `diagnostics/clippy-fix.sh <crate>` that snapshots, runs --fix, captures diff)? If 6 more sections will repeat this pattern, file the wrapper as a finding now.
  - [ ] `/sync-claude`: no API/command changes from auto-fix; document negative finding.
  - [ ] Repo hygiene check.

---

## 02.2 Manual cleanup of remaining structural violations

**File(s):** `oriterm_ipc/tests/ipc_roundtrip.rs`

After auto-fix, structural lints (`redundant_clone × 5`) likely remain — `cargo clippy --fix` will not auto-apply `redundant_clone` removal when the value is potentially reused after.

- [ ] Run `cargo clippy -p oriterm_ipc --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` and capture the remaining error list.
- [ ] For each `redundant_clone` site (`tests/ipc_roundtrip.rs:88,122,172,283`):
  - Read ±10 lines around the cite.
  - Verify the cloned value is NOT reused after the clone (the lint fires because clippy detected this).
  - Apply the suggested fix: remove `.clone()`.
  - Re-run `cargo test -p oriterm_ipc` to verify no test regression.
- [ ] If any `manual_assert` or `doc_markdown` survived auto-fix (unlikely), apply manually.
- [ ] Verify exit gate: `cargo clippy -p oriterm_ipc --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` exits 0.
- [ ] Commit: `style(oriterm_ipc): remove 5 redundant_clone in test file (drop semantics preserved)`.

- [ ] **Subsection close-out (02.2)**:
  - [ ] Status → `complete`
  - [ ] `/improve-tooling`: per-site clone removal was tedious; consider a `cargo clippy --fix` retry with `--allow-no-vcs --allow-dirty` after the value-reuse check (would `cargo fix` apply now that we've validated)?
  - [ ] `/sync-claude`: no rules drift; test code only.
  - [ ] Repo hygiene check.

---

## 02.3 Resolve `[lints]` divergence in oriterm_ipc/Cargo.toml

**File(s):** `oriterm_ipc/Cargo.toml`, possibly `oriterm_ipc/src/**/*.rs` (per-file `#[allow(unsafe_code)]` annotations if option B chosen)

**Context:** `oriterm_ipc/Cargo.toml` declares `[lints]` independently of workspace to allow `unsafe_code`. This is an SSOT divergence per `impl-hygiene.md §SSOT` — every workspace lint addition has to be manually mirrored. Two options:

**Fix approach — 2 options:**

**(a) Keep divergence + document** (recommended — minimum change):

```toml
[lints]
# SSOT exemption: this crate diverges from `workspace = true` because the
# IPC transport layer requires `unsafe_code` for FFI (raw fd/handle
# interaction with `mio` and platform-native sockets/pipes). Every other
# workspace lint MUST be mirrored here when added at the workspace level.
# See plans/clippy-gate-hardening/section-02-oriterm-ipc-cleanup.md §02.3.
# ... (existing lints, copied verbatim from workspace)
```

**Why this is best:** Minimum change. The divergence is real and necessary; documenting it converts an SSOT violation into an SSOT exemption. Future readers understand WHY the divergence exists.

**Trade-off:** Workspace lint additions still require manual mirroring here. Mitigation: add a `tests/lints_sync_check.rs` that reads both `Cargo.toml` and `oriterm_ipc/Cargo.toml`, parses `[workspace.lints.clippy]` and `oriterm_ipc/Cargo.toml [lints.clippy]`, and asserts the lint sets match (modulo the explicit `unsafe_code` divergence). Section 09 meta-test could absorb this.

**(b) Migrate to `workspace = true` + per-file `#[allow(unsafe_code)]`** (alternative):

```toml
[lints]
workspace = true
```

Plus, in every `oriterm_ipc/src/**/*.rs` file containing `unsafe`, add `#![allow(unsafe_code)]` at the top of the file (or `#[allow(unsafe_code)]` per `unsafe` block).

**Downside:** Multiple `#![allow]` annotations replace one workspace-level setting. Each must also have `reason="..."` per workspace style. Spreads the exemption across many files instead of centralizing.

**Recommended path:** Option (a) — keep the divergence with a `# SSOT exemption:` comment. Defer Option (b) to a future hygiene plan if the divergence drifts (workspace adds a lint, oriterm_ipc forgets to mirror).

- [ ] Decide between (a) and (b) — recommend (a). If (b), invoke `AskUserQuestion` to confirm before editing every src file.
- [ ] Apply chosen option. If (a):
  - [ ] Edit `oriterm_ipc/Cargo.toml` to add `# SSOT exemption: ...` comment block at the top of the `[lints]` section. Verify the existing lint declarations match the workspace's `Cargo.toml [workspace.lints]` set verbatim (so the only divergence is `unsafe_code`).
- [ ] Verify `cargo clippy -p oriterm_ipc --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` still exits 0.
- [ ] Commit: `chore(oriterm_ipc): document [lints] divergence rationale (SSOT exemption for unsafe_code)`.

- [ ] **Subsection close-out (02.3)**:
  - [ ] Status → `complete`
  - [ ] `/improve-tooling`: did the manual workspace-vs-crate lint diff highlight a need for a `diagnostics/lints-sync.sh` script? Section 09 meta-test will need similar parsing logic.
  - [ ] `/sync-claude`: `impl-hygiene.md §SSOT` may benefit from a "SSOT exemption" pattern citation. Document if no drift.
  - [ ] Repo hygiene check.

---

## 02.4 Cross-target verification

**File(s):** none (verification only)

- [ ] `cargo clippy -p oriterm_ipc --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` exits 0
- [ ] `cargo clippy -p oriterm_ipc --all-targets --target x86_64-pc-windows-gnu -- -D warnings` exits 0 — Windows-specific code paths (named pipes via `mio` + `winapi`) covered.
- [ ] If Windows-specific violations surface that didn't appear on host, fix them in 02.2-02.3 style and update the bug entry's "Top violations" line to reflect cross-target totals.
- [ ] `cargo test -p oriterm_ipc` green on host. (Windows test runs are CI's responsibility — the cross-compile verifies clippy clean only.)

- [ ] **Subsection close-out (02.4)**:
  - [ ] Status → `complete`
  - [ ] `/improve-tooling`: cross-target verification — did running both targets reveal any platform-specific lint behavior worth documenting?
  - [ ] `/sync-claude`: no changes typically; test code only.
  - [ ] Repo hygiene check.

---

## 02.R Third Party Review Findings

<!-- Reserved for /tpr-review (Codex + Gemini). -->

- None.

---

## 02.N Completion Checklist

- [ ] `cargo clippy -p oriterm_ipc --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` exits 0
- [ ] `cargo clippy -p oriterm_ipc --all-targets --target x86_64-pc-windows-gnu -- -D warnings` exits 0
- [ ] `cargo test -p oriterm_ipc` green
- [ ] `cargo test --all` green (regression canary across workspace)
- [ ] `oriterm_ipc/Cargo.toml [lints]` decision documented (option (a) or (b) committed)
- [ ] `BUG-07-NNN+1` entry remains `[ ]` (closure happens in Section 10)
- [ ] **Plan sync**:
  - [ ] This section's frontmatter `status` → `complete`, all subsection statuses → `complete`
  - [ ] `00-overview.md` Quick Reference: Section 02 → `Complete`
  - [ ] `index.md` Section 02 status → `Complete`
- [ ] `/tpr-review` passed (Section 02 is small but `[lints]` divergence decision merits a TPR pass)
- [ ] `/impl-hygiene-review` passed (after TPR clean)
- [ ] `/improve-tooling` section-close sweep — verify per-subsection captures; add cross-cutting items (likely: `diagnostics/clippy-fix.sh` wrapper template that all subsequent sections reuse)
- [ ] `/sync-claude` section-close doc sync
- [ ] **Repo hygiene check** — `diagnostics/repo-hygiene.sh --check`

**Exit Criteria:** `cargo clippy -p oriterm_ipc --all-targets -- -D warnings` exits 0 on host AND `x86_64-pc-windows-gnu`; `cargo test -p oriterm_ipc` green; `oriterm_ipc/Cargo.toml [lints]` decision committed; section frontmatter and overview/index reflect complete.
