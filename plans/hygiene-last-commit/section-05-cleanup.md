---
section: "05"
title: "Cleanup"
status: not-started
reviewed: false
goal: "Verify all fixes, run full build/test/lint suite, and delete this plan"
depends_on: ["01", "02", "03", "04"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "05.1"
    title: "Final Verification"
    status: not-started
  - id: "05.2"
    title: "Plan Deletion"
    status: not-started
---

# Section 05: Cleanup

**Status:** Not Started
**Goal:** Verify no behavior changes, pass all builds/tests/lints, and delete this disposable plan.

**Depends on:** Sections 01, 02, 03, 04 (all must be complete).

---

## 05.1 Final Verification

- [ ] Run `./test-all.sh` to verify no behavior changes
- [ ] Run `./clippy-all.sh` to verify no regressions
- [ ] Run `./build-all.sh` to verify cross-compilation
- [ ] Spot-check that the extracted helpers are actually called (grep for the new function names)
- [ ] Verify no file in `oriterm/src/app/` exceeds 500 lines (excluding test files)

---

## 05.2 Plan Deletion

- [ ] Delete this plan directory: `rm -rf plans/hygiene-last-commit/`
