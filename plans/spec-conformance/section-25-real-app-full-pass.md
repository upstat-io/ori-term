---
section: "25"
title: "Real-App FULL-PASS Milestone"
status: not-started
reviewed: false
goal: "Drive every documented daily-driver real-application scenario through the harness from section 22 and verify each app's recorded session replays cleanly to a stable golden snapshot. Apps in scope: vim, neovim, helix, htop, btop, tmux, aerc, ncmpcpp, less."
success_criteria:
  - "Every app in scope has at least one recorded scenario that passes via the section 22 harness"
  - "Each scenario produces a stable text snapshot (and optionally a pixel golden) that survives back-to-back runs with 0-byte diff"
  - "Per-app scenarios committed under `crates/oriterm_test_support/tests/data/real_app_captures/<app>/`"
  - "Per-app snapshots committed under `crates/oriterm_test_support/tests/references/real_app/<app>/`"
  - "**This section does NOT contain implementation work** — implementations live in the per-stack sections (sections 03-20). This section only adds new test scenarios and bisects failures."
  - "Section 25 catches scenario-specific bugs that the per-stack tests structurally cannot find — e.g., vim's syntax highlighting exercising SGR + cursor + scrolling in a specific combination"
  - "All existing tests pass without modification"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Real-app E2E milestones pass** mission criterion"
inspired_by:
  - "section 22 harness — replay infrastructure used by every test in this section"
  - "common dev workflows — typical vim/neovim/helix usage, htop / btop monitoring, tmux multiplexing"
depends_on: ["08", "10", "11", "16", "17", "22"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "25.1"
    title: "Drive vim + neovim daily-driver scenarios to pass"
    status: not-started
  - id: "25.2"
    title: "Drive helix daily-driver scenario to pass"
    status: not-started
  - id: "25.3"
    title: "Drive htop + btop monitoring scenarios to pass"
    status: not-started
  - id: "25.4"
    title: "Drive tmux multiplexer scenario to pass"
    status: not-started
  - id: "25.5"
    title: "Drive aerc + ncmpcpp + less scenarios to pass"
    status: not-started
  - id: "25.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "25.N"
    title: "Completion Checklist"
    status: not-started
# TPR Checkpoint Placement: 25.3 (after editors + monitoring — covers .1-.3),
# 25.5 (after multiplexer + remaining apps — covers .4-.5), final in 25.N
---

# Section 25: Real-App FULL-PASS Milestone

**Status:** Not Started
**Goal:** Drive every daily-driver real-application scenario to pass via the section 22 harness. Sibling milestone to section 24.

**Success Criteria:** see frontmatter.

**Context:** Section 22 built the harness + vim simple session smoke test. This section adds scenarios for the remaining apps in scope. Each scenario is a recorded PTY session that replays deterministically through ori_term and produces a snapshot identical to its committed golden.

**Reference implementations:** see frontmatter.

**Depends on:** Sections 08, 10, 11, 16, 17 (baseline + OSC + glyphs + mouse + keyboard verified — the per-stack sections that real apps exercise), Section 22 (harness scaffolding).

---

## 25.1 Drive vim + neovim daily-driver scenarios to pass

**File(s):** `oriterm_core/tests/real_app/vim_daily_driver.rs` (new), `oriterm_core/tests/real_app/nvim_daily_driver.rs` (new), captures + snapshots

For each scenario, capture, replay, snapshot, commit, verify reproducibility.

- [ ] vim daily-driver: open a Rust file, scroll, search for a function, jump to a line, edit a few characters, undo, save, quit. Captures the typical edit/navigate/search workflow.
- [ ] neovim daily-driver: similar workflow plus treesitter syntax highlighting (which exercises SGR truecolor + glyph rendering in a stress pattern)
- [ ] Update tracker
- [ ] **Validation**: both scenarios pass; snapshots reproduce.

---

## 25.2 Drive helix daily-driver scenario to pass

- [ ] helix daily-driver: open a file, multi-cursor edit, language server hover (if helix's PTY mode supports it without an LSP), quit
- [ ] Helix's selection model is different from vim — exercises a different SGR + cursor pattern
- [ ] Update tracker

---

## 25.3 Drive htop + btop monitoring scenarios to pass

- [ ] htop scenario: launch, scroll through process list, sort by CPU, quit. Captures the per-second update pattern + fullscreen redraw.
- [ ] btop scenario: launch, observe a 10-second snapshot (deterministic via process snapshot if possible), quit. Tests fancier graphs + colors.
- [ ] Update tracker
- [ ] **TPR checkpoint** — `/tpr-review` covering 25.1-25.3

---

## 25.4 Drive tmux multiplexer scenario to pass

- [ ] tmux scenario: launch, create a new pane, split horizontally, run a command in each pane, navigate between panes, kill server. Tests tmux's heavy use of cursor positioning + scroll regions + alt screen.
- [ ] Tmux is the heaviest user of escape sequences in the corpus — if tmux scenario passes, ori_term is likely solid for most text apps
- [ ] Update tracker

---

## 25.5 Drive aerc + ncmpcpp + less scenarios to pass

- [ ] aerc scenario: launch, view inbox, open a message, navigate, quit
- [ ] ncmpcpp scenario: launch, browse library, play a track (if PTY mode supports), quit
- [ ] less scenario: pipe a large file, scroll forward + backward, search, quit
- [ ] Update tracker
- [ ] **TPR checkpoint** — `/tpr-review` covering 25.4-25.5

---

## 25.R Third Party Review Findings

- None.

---

## 25.N Completion Checklist

- [ ] Failing test matrix written FIRST: every scenario test is initially failing until the captures + snapshots are committed
- [ ] **Matrix dimensions**: app × scenario × snapshot type (text/pixel)
- [ ] **Semantic pin**: each app's daily-driver scenario is a permanent regression guard
- [ ] vim, neovim, helix, htop, btop, tmux, aerc, ncmpcpp, less all have at least one passing scenario
- [ ] All scenarios reproduce on back-to-back runs
- [ ] All existing tests pass without modification
- [ ] Alloc regression unchanged
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` mission success criteria checked off (**Real-app E2E milestones pass**)
- [ ] `index.md` section 25 status updated
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** Every daily-driver real app passes its scenario(s); the second major integration milestone is complete. ori_term is the most spec-complete and most extensively-real-app-tested terminal emulator ever built.
