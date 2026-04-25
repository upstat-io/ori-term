---
section: "07"
title: "oriterm Cleanup"
status: not-started
reviewed: false
goal: "Drive `cargo clippy -p oriterm --all-targets -- -D warnings` to exit 0 on host AND Windows GNU AND `--features gpu-tests` AND `--features profile`, fixing 6 native violations PLUS upgrading 10 builtin color-scheme files from bare `#![allow(clippy::unreadable_literal)]` to `#![expect(clippy::unreadable_literal, reason=\"generated color hex literals\")]`."
success_criteria:
  - "`cargo clippy -p oriterm --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` exits 0 (default)"
  - "`cargo clippy -p oriterm --all-targets --target x86_64-unknown-linux-gnu --features gpu-tests -- -D warnings` exits 0"
  - "`cargo clippy -p oriterm --all-targets --target x86_64-unknown-linux-gnu --features profile -- -D warnings` exits 0"
  - "`cargo clippy -p oriterm --all-targets --target x86_64-pc-windows-gnu -- -D warnings` exits 0"
  - "`cargo test -p oriterm` green; architecture tests, integration tests, GPU regression tests unaffected"
  - "All 10 `oriterm/src/scheme/builtin/*.rs` files use `#![expect(clippy::unreadable_literal, reason=\"generated color hex literals\")]` (matching the `oriterm_ui/src/icons/footer.rs:7` precedent), replacing the bare `#![allow]` declarations"
  - "Closes BUG-07-NNN+2 (oriterm entry from Section 01.2; supersede in Section 10)"
inspired_by:
  - "Section 02-06 cleanup pattern"
  - "oriterm_ui/src/icons/footer.rs:7 (`#![expect(clippy::unreadable_literal, reason=\"generated icon coordinates\")]` precedent)"
depends_on: ["06"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "07.1"
    title: "Auto-fix sweep with diff review (default + gpu-tests + profile features)"
    status: not-started
  - id: "07.2"
    title: "Manual cleanup of native violations + 3 production float_cmp judgment"
    status: not-started
  - id: "07.3"
    title: "Upgrade 10 builtin color-scheme #![allow] → #![expect(reason=...)]"
    status: not-started
  - id: "07.4"
    title: "Cross-target + feature-matrix verification"
    status: not-started
  - id: "07.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "07.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 07: oriterm Cleanup

**Status:** Not Started
**Goal:** Smallest crate by violation count (6 native + 3 surfaced from oriterm_ui via dev-dep) but biggest feature surface (default, gpu-tests, profile). Also resolves the long-pre-existing tech debt of 10 builtin color-scheme files using bare `#![allow]` without `reason=`.

**Success Criteria:** see frontmatter.

**Context:** oriterm is the application shell (winit event loop, GPU init, session model, font pipeline). 6 native violations: 3 production-code float_cmp (judgment), plus doc_markdown, manual_assert, many_single_char_names. The 3 "extra" violations seen in `--features gpu-tests` are float_cmp surfaced through `oriterm_ui::testing::scene_snapshot::mod.rs:101` — those are oriterm_ui's responsibility, fixed in Section 05.

The interesting work in Section 07 is the 10 builtin color-scheme files (`oriterm/src/scheme/builtin/{catppuccin,popular,material,nature,tokyo,retro,modern,extended,extended2,mod}.rs`). Each declares bare `#![allow(clippy::unreadable_literal)]` without a `reason=` clause. CLAUDE.md `[workspace.lints.clippy]` policy and `code-hygiene.md §Style` mandate `#[expect(reason=...)]` over `#[allow]` for new annotations; the 10 pre-existing files are tech debt from before the policy. Upgrading them to `#![expect(reason="generated color hex literals")]` matches the `oriterm_ui/src/icons/footer.rs:7` precedent and converts dead annotations into ones the compiler can tell us about (`#[expect]` warns if the suppressed lint never fires).

**Reference implementations:**
- Section 02-06 cleanup pattern
- `oriterm_ui/src/icons/footer.rs:7` — file-level `#![expect(clippy::unreadable_literal, reason="generated icon coordinates")]` precedent

**Depends on:** Section 06 (oriterm_mux clean — its types are consumed in oriterm's session model and event loop).

---

## Intelligence Reconnaissance

Queries run 2026-04-25:

- `scripts/intel-query.sh` — not present in this project; queries below used Grep / Glob / Read / cargo clippy --message-format=json instead.
- `cargo clippy -p oriterm --all-targets --message-format=json` — 6 native violations: float_cmp 3 (production code), doc_markdown 1, manual_assert 1, many_single_char_names 1.
- `Grep -rn '#!\[allow(clippy::unreadable_literal' oriterm/src/` — 10 files in `oriterm/src/scheme/builtin/`: catppuccin.rs:3, popular.rs:3, material.rs:3, mod.rs:8, nature.rs:3, extended.rs:3, tokyo.rs:3, retro.rs:3, modern.rs:3, extended2.rs:4. None have `reason=` clauses.
- `Read oriterm/Cargo.toml [features]` — `default = []`, `gpu-tests = []`, `profile = []`. Three feature flags = three additional verification cells beyond default.

Results summary (≤500 chars) [ori]: 6 native violations (3 production float_cmp + 3 mechanical) + 10 file-level `#![allow]` upgrades. `--features gpu-tests` adds 3 oriterm_ui-owned float_cmp (Section 05 fixes those). Three feature-flag cells × 2 targets = 6 verification cells.

See _(intel graph not available in this project; use Grep/Glob)_ for the full query protocol.

---

## 07.1 Auto-fix sweep with diff review (default + gpu-tests + profile features)

**File(s):** `oriterm/src/**/*.rs`, `oriterm/tests/**/*.rs`, `oriterm/benches/*.rs`

- [ ] Run auto-fix in three feature combos:
  - `cargo clippy --fix --all-targets -p oriterm --target x86_64-unknown-linux-gnu --allow-dirty`
  - `cargo clippy --fix --all-targets -p oriterm --target x86_64-unknown-linux-gnu --features gpu-tests --allow-dirty`
  - `cargo clippy --fix --all-targets -p oriterm --target x86_64-unknown-linux-gnu --features profile --allow-dirty`
- [ ] Capture cumulative diff; manual review per Section 02 template.
- [ ] `cargo test -p oriterm` green; architecture tests pass.
- [ ] Commit: `chore(oriterm): apply cargo clippy --fix for ~3 mechanical lints across feature combos`.

- [ ] **Subsection close-out (07.1)**: standard template.

---

## 07.2 Manual cleanup of native violations + 3 production float_cmp judgment

**File(s):** TBD per JSON enumeration

- [ ] Enumerate remaining (~3-6 sites): `cargo clippy -p oriterm --all-targets -- -D warnings`.
- [ ] For each `float_cmp` (3 sites — production):
  - Read context.
  - Classify per Section 03.3 protocol (exact-representable vs computed).
  - Apply `#[expect(clippy::float_cmp, reason="<site-specific reason>")]` or rewrite to epsilon comparison.
- [ ] For `many_single_char_names` (1 site): rename single-char names that span scope >3 lines, OR `#[expect(reason="3D coord variables are conventional in this scope")]`.
- [ ] For surviving `doc_markdown` / `manual_assert`: apply per-site fix.
- [ ] `cargo test -p oriterm` green.
- [ ] Commit: `style(oriterm): cleanup 6 native clippy violations (3 production float_cmp + 3 mech)`.

- [ ] **Subsection close-out (07.2)**: standard template.

---

## 07.3 Upgrade 10 builtin color-scheme #![allow] → #![expect(reason=...)]

**File(s):** `oriterm/src/scheme/builtin/{catppuccin,popular,material,mod,nature,extended,tokyo,retro,modern,extended2}.rs`

For each of the 10 files:

- [ ] Read the file's header (lines 1-10).
- [ ] Replace `#![allow(clippy::unreadable_literal)]` with:
  ```rust
  #![expect(
      clippy::unreadable_literal,
      reason = "generated color hex literals — readability not improved by underscore separators"
  )]
  ```
- [ ] Verify the replacement line conforms to the file's existing rustfmt output (check tab/space, line wrap).
- [ ] Verify the `#![expect]` does NOT suddenly fire as an "unfulfilled expectation" (which would mean the lint is no longer triggered — i.e., the underlying issue was already fixed and we're suppressing a now-stale warning). If `#![expect]` fires, REMOVE the annotation entirely.
- [ ] Commit per-cluster: `style(oriterm/scheme/builtin): upgrade 10 #![allow(unreadable_literal)] → #![expect(reason=...)]`.

- [ ] **Subsection close-out (07.3)**: standard template; `/improve-tooling` retrospective: did `cargo clippy --all-targets` flag the `#![allow]` → `#![expect]` migration as cumbersome? If so, `diagnostics/migrate-allow-to-expect.sh` could be a one-shot helper.

---

## 07.4 Cross-target + feature-matrix verification

- [ ] `cargo clippy -p oriterm --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` exits 0
- [ ] `cargo clippy -p oriterm --all-targets --target x86_64-unknown-linux-gnu --features gpu-tests -- -D warnings` exits 0 (NOTE: this run depends on Section 05 oriterm_ui clean — `oriterm_ui/testing/scene_snapshot::mod.rs:101` float_cmp must be fixed in Section 05 before this verification can pass)
- [ ] `cargo clippy -p oriterm --all-targets --target x86_64-unknown-linux-gnu --features profile -- -D warnings` exits 0
- [ ] `cargo clippy -p oriterm --all-targets --target x86_64-pc-windows-gnu -- -D warnings` exits 0
- [ ] `cargo clippy -p oriterm --all-targets --target x86_64-pc-windows-gnu --features gpu-tests -- -D warnings` exits 0
- [ ] `cargo clippy -p oriterm --all-targets --target x86_64-pc-windows-gnu --features profile -- -D warnings` exits 0

- [ ] **Subsection close-out (07.4)**: standard template.

---

## 07.R Third Party Review Findings

- None.

---

## 07.N Completion Checklist

- [ ] All 6 target × feature cells exit 0 (`-D warnings`)
- [ ] `cargo test -p oriterm` green; architecture tests pass; GPU regression tests pass under `--features gpu-tests`
- [ ] `cargo test --all` green (regression canary)
- [ ] All 10 `oriterm/src/scheme/builtin/*.rs` files use `#![expect(...reason)]`
- [ ] BUG-07-NNN+2 (oriterm entry) remains `[ ]` (closure in Section 10)
- [ ] **Plan sync**: section 07 status → complete in section file + 00-overview.md + index.md
- [ ] `/tpr-review` passed
- [ ] `/impl-hygiene-review` passed (after TPR clean)
- [ ] `/improve-tooling` section-close sweep
- [ ] `/sync-claude` section-close doc sync — CLAUDE.md doesn't reference scheme/builtin specifically; no rules drift expected
- [ ] **Repo hygiene check**

**Exit Criteria:** All six oriterm target × feature cells exit 0; architecture + integration tests green; 10 color-scheme files upgraded to `#![expect(reason=...)]`; section frontmatter and overview/index reflect complete.
