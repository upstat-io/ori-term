---
section: "08"
title: "Feature Matrix Verification"
status: not-started
reviewed: false
goal: "Run the full per-crate feature matrix × cross-target combo and verify every cell exits 0 with `--all-targets -- -D warnings`. Catches any feature-gated code path that the per-crate cleanups (Sections 02-07) missed because the section's auto-fix sweep only ran one feature combo."
success_criteria:
  - "Every cell of the feature matrix exits 0:"
  - "  - workspace × {host, x86_64-pc-windows-gnu} × default features"
  - "  - oriterm_core × both targets × `--no-default-features` (image-protocol disabled)"
  - "  - oriterm_ui × both targets × `--features testing`"
  - "  - oriterm × both targets × `--features gpu-tests`"
  - "  - oriterm × both targets × `--features profile`"
  - "Any newly-surfaced violations from feature combos NOT covered in Sections 02-07 are fixed inline in this section (e.g., a `--no-default-features` violation in oriterm_core that the Section 03 default-features pass missed)"
  - "`./test-all.sh` green; `./build-all.sh` green"
inspired_by:
  - "Section 02-07 per-crate verification subsections (.4 typically)"
depends_on: ["07"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "08.1"
    title: "Run the full feature matrix"
    status: not-started
  - id: "08.2"
    title: "Fix any newly-surfaced violations inline"
    status: not-started
  - id: "08.3"
    title: "Workspace-level verification"
    status: not-started
  - id: "08.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "08.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 08: Feature Matrix Verification

**Status:** Not Started
**Goal:** Defense in depth. Sections 02-07 each ran their primary feature combo. Section 08 closes the gaps by running every applicable feature × target combination, fixing anything that surfaces. This is the LAST chance to catch feature-gated violations before the gate flip in Section 09 — flipping the gate with even one feature combo dirty would break CI immediately.

**Success Criteria:** see frontmatter.

**Context:** Per Phase 1 research and Codex's round-1 advice: each crate's auto-fix sweep typically runs ONE feature combination, leaving feature-gated code paths uncovered. Specifically:
- `oriterm_core --no-default-features` (image-protocol disabled): doc-link warnings on `#[cfg(feature = "image-protocol")]` items, `dead_code` on now-unreachable items.
- `oriterm_ui --features testing`: the `oriterm_ui::testing` module is `#[cfg(feature = "testing")]`-gated and has its own internal lint surface that only fires when the feature is on. Section 05 ran with `--features testing` so this should be clean, but verify.
- `oriterm --features gpu-tests`: surfaces oriterm_ui::testing as a transitive dev-dep with feature enabled. Section 05 fixes the float_cmp; Section 08 verifies the cross-crate path.
- `oriterm --features profile`: enables `cfg(feature = "profile")` instrumentation paths. Profile-gated code is rarely exercised by tests; clippy may catch unused or dead code.

**Reference implementations:** None — this is the workspace's first feature-matrix verification pass.

**Depends on:** Section 07 (every per-crate cleanup section complete; Section 08 is the cross-cutting verification gate that gates Section 09's flip).

---

## Intelligence Reconnaissance

Queries run 2026-04-25:

- `scripts/intel-query.sh` — not present in this project; queries below used Grep / Glob / Read / cargo clippy --message-format=json instead.
- Per-crate feature inventory (Section 01.3 baseline + Pass 1 inventory): oriterm_core has `image-protocol` (default); oriterm_ui has `testing` (no default); oriterm has `gpu-tests` and `profile` (no default).
- No vendored crate features in scope (per `00-overview.md` Out of scope).

Results summary (≤500 chars) [ori]: 5 feature combos × 2 targets = 10 verification cells beyond the workspace default. Each cell runs `cargo clippy ... -- -D warnings` and is expected to exit 0 if Sections 02-07 covered their feature combos correctly. Any failure surfaces a missed cleanup site.

See _(intel graph not available in this project; use Grep/Glob)_ for the full query protocol.

---

## 08.1 Run the full feature matrix

**File(s):** none (verification only — populates `plans/clippy-gate-hardening/feature-matrix-verify.md`)

- [ ] Run all 10 cells:
  ```bash
  for target in x86_64-unknown-linux-gnu x86_64-pc-windows-gnu; do
    # Workspace default
    timeout 150 cargo clippy --workspace --all-targets --target $target -- -D warnings || echo "FAIL: workspace × default × $target"
    # oriterm_core --no-default-features
    timeout 150 cargo clippy -p oriterm_core --all-targets --target $target --no-default-features -- -D warnings || echo "FAIL: oriterm_core × no-default × $target"
    # oriterm_ui --features testing
    timeout 150 cargo clippy -p oriterm_ui --all-targets --target $target --features testing -- -D warnings || echo "FAIL: oriterm_ui × testing × $target"
    # oriterm --features gpu-tests
    timeout 150 cargo clippy -p oriterm --all-targets --target $target --features gpu-tests -- -D warnings || echo "FAIL: oriterm × gpu-tests × $target"
    # oriterm --features profile
    timeout 150 cargo clippy -p oriterm --all-targets --target $target --features profile -- -D warnings || echo "FAIL: oriterm × profile × $target"
  done
  ```
- [ ] For every "FAIL:" cell, capture the violation list and proceed to 08.2.
- [ ] If all 10 cells pass, write `plans/clippy-gate-hardening/feature-matrix-verify.md` documenting the verified combos with the run-date. Skip 08.2.

- [ ] **Subsection close-out (08.1)**: standard template.

---

## 08.2 Fix any newly-surfaced violations inline

**File(s):** TBD (depends on what 08.1 surfaces)

- [ ] For each cell that failed 08.1:
  - Identify the owning crate.
  - Determine if the violation is in the same files as that crate's per-crate cleanup section (02-07). If yes, treat as a missed cleanup; fix per the section's template (auto-fix → diff review → manual cleanup).
  - If the violation is in a feature-gated file path NOT touched by the per-crate section, document the gap; fix here; consider whether the per-crate section's exit gate should include the feature combo (update the section's success criteria retroactively if so).
- [ ] Re-run 08.1; verify all 10 cells exit 0.
- [ ] Commit: `chore(workspace): fix feature-matrix-only violations missed by per-crate cleanups`.

- [ ] **Subsection close-out (08.2)**: standard template; `/improve-tooling` retrospective: should each per-crate section's exit gate explicitly include ALL applicable feature combos (rather than just the primary)? If yes, file as a `00-overview.md` Design Principle update.

---

## 08.3 Workspace-level verification

- [ ] `cargo clippy --workspace --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` exits 0
- [ ] `cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu -- -D warnings` exits 0
- [ ] `./test-all.sh` green
- [ ] `./build-all.sh` green
- [ ] Document the verified state in `plans/clippy-gate-hardening/feature-matrix-verify.md`.

- [ ] **Subsection close-out (08.3)**: standard template.

---

## 08.R Third Party Review Findings

- None.

---

## 08.N Completion Checklist

- [ ] All 10 feature × target cells exit 0
- [ ] Workspace-level both-target runs exit 0
- [ ] `./test-all.sh` green
- [ ] `./build-all.sh` green
- [ ] `cargo test --all` green
- [ ] `feature-matrix-verify.md` committed
- [ ] **Plan sync**: section 08 status → complete in section file + 00-overview.md + index.md
- [ ] `/tpr-review` passed
- [ ] `/impl-hygiene-review` passed (after TPR clean)
- [ ] `/improve-tooling` section-close sweep
- [ ] `/sync-claude` section-close doc sync
- [ ] **Repo hygiene check**

**Exit Criteria:** Every applicable feature × target × `--all-targets` combination exits 0; workspace tests + builds green; verification document committed.
