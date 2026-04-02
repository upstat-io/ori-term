---
section: "07"
title: "Verification & Metrics"
status: not-started
reviewed: true
goal: "90% vttest pass rate across menus 1-8 at all 3 sizes, full conformance audit documented"
depends_on: ["01", "02", "03", "04", "05", "06"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "07.1"
    title: "Conformance Audit"
    status: not-started
  - id: "07.2"
    title: "Performance Validation"
    status: not-started
  - id: "07.3"
    title: "Build & Verify"
    status: not-started
  - id: "07.4"
    title: "Documentation"
    status: not-started
  - id: "07.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "07.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 07: Verification & Metrics

**Status:** Not Started
**Goal:** Verify 90% vttest pass rate across menus 1-3, 5-6, 8 (excluding menu 4 DECDHL/DECDWL and menu 7 VT52 -- not implemented) at 80x24. Menus 1-2 additionally verified at 97x33 and 120x40. Document conformance state, fix remaining gaps, ensure no performance regressions.

**Context:** This is the final verification pass after all VTE fixes (sections 01-04), blink implementation (section 05), and test expansion (section 06) are complete. It audits the full conformance state, fixes any remaining gaps to hit 90%, and ensures no performance regressions from the changes.

**Depends on:** All prior sections.

---

## 07.1 Conformance Audit

Run the full vttest suite and score each menu.

- [ ] **Menu 1 (Cursor Movement):** Run all screens at 3 sizes, count pass/fail
  - Target: 6/6 screens pass (border, origin mode, wrap, control chars, leading zeros)
- [ ] **Menu 2 (Screen Features):** Run all 15 screens at 3 sizes
  - Target: 11/15 screens pass fully, 4 partial passes (DECCOLM screens have correct side effects but wrapped visual output)
- [ ] **Menu 3 (Character Sets):** Run all screens at 80x24
  - Target: All screens pass
- [ ] **Menu 4 (Double-Size Characters):** Run at 80x24
  - Target: Snapshot baseline only -- DECDHL/DECDWL are NOT implemented. Text renders at normal size. Document as known limitation.
- [ ] **Menu 5 (Keyboard):** Run automatable screens
  - Target: Key echo and auto-repeat tests pass
- [ ] **Menu 6 (Terminal Reports):** Run all screens at 80x24
  - Target: All DA/DSR/DECRQM responses correct (including DSR cursor position in DECOM mode -- fixed in Section 02)
- [ ] **Menu 7 (VT52 Mode):** Run at 80x24
  - Target: Snapshot baseline only -- VT52 compatibility mode is NOT implemented. This menu will fail entirely. Document as known limitation and exclude from pass-rate denominator.
- [ ] **Menu 8 (VT102 Features):** Run all screens at 80x24
  - Target: All ICH/DCH/IL/DL screens pass

### Scoring

```
Pass rate = (screens passing structural assertions + visual match) / total automatable screens
Target: >= 90% across menus 1-3, 5-6, 8 (exclude menu 4 DECDHL/DECDWL and menu 7 VT52 — unimplemented)

Per-menu scoring:
| Menu | Total Screens | Passing | Rate | Notes |
|------|--------------|---------|------|-------|
| 1    | 6            |         |      |       |
| 2    | 15           |         |      |       |
| 3    | ?            |         |      |       |
| 4    | ?            |         | N/A  | DECDHL/DECDWL not implemented |
| 5    | ? (automatable)|       |      |       |
| 6    | ?            |         |      |       |
| 7    | ?            |         | N/A  | VT52 mode not implemented |
| 8    | ?            |         |      |       |
| **Total (excl 4,7)** | **?** |     | **>= 90%** | |
```

- [ ] Fill in the scoring table
- [ ] For each failing screen, document: what's wrong, root cause, fix complexity
- [ ] For each failing screen with a simple fix (<50 lines of production code, no architectural changes, no new types): fix it in this section. For failures requiring >50 lines or new types: create a bug tracker entry with severity and estimated effort.
- [ ] Document remaining failures as known limitations with bug tracker entries
- [ ] If menu 4 or menu 7 features are desired in the future, create separate roadmap items
- [ ] **Prerequisite**: Section 06 must fill in the total screen counts from actual vttest runs before this section can compute a pass rate. If section 06 did not capture counts, run vttest menus manually and count screens.

---

## 07.2 Performance Validation

VTE handler changes must not regress performance invariants.

- [ ] **Zero idle CPU beyond cursor blink** -- verified by `compute_control_flow()` tests
  - The new `ColorEase` uses `next_change()` for WaitUntil scheduling -- verify no continuous wakeups
- [ ] **Zero allocations in hot render path** -- verified by alloc regression tests
  - Run: `cargo test -p oriterm_core --test alloc_regression`
  - Verify: `snapshot_extraction_zero_alloc_steady_state` passes
  - Verify: `hundred_frames_zero_alloc_after_warmup` passes
- [ ] **Stable RSS under sustained output** -- run `rss_stability_under_sustained_output`
- [ ] **No new allocations in VTE handler hot path** -- origin mode fix, scroll region changes must not add per-character allocations
- [ ] **Cursor blink idle CPU** -- during visible/hidden plateaus, event loop wakeup interval is ~530ms (not 16ms). During fade transitions (~200ms), wakeup is ~16ms. Verify via `compute_control_flow()` test assertions. The overall idle CPU must not increase: the total number of wakeups per blink cycle should be ~(200ms/16ms)*2 + 2 = ~27 wakeups, vs current 2 wakeups. This is acceptable because each wakeup is a no-op (cursor alpha update only, no full frame rebuild).
- [ ] **ColorEase update() cost** -- verify `update()` is O(1) with no allocations. It should be a simple elapsed-time computation.

---

## 07.3 Build & Verify

- [ ] `./build-all.sh` green (all platforms)
- [ ] `./clippy-all.sh` green (no warnings)
- [ ] `./test-all.sh` green (all tests pass)
- [ ] Architecture tests pass: `cargo test -p oriterm --test architecture`
- [ ] vttest text tests pass: `cargo test -p oriterm_core --test vttest`
- [ ] vttest GPU tests pass: `cargo test -p oriterm -- visual_regression::vttest`

---

## 07.4 Documentation

- [ ] Update `plans/bug-tracker/` with any new bugs discovered during audit
- [ ] Add vttest conformance summary to plan overview
- [ ] Update CLAUDE.md if new test commands introduced
- [ ] Document the vttest test infrastructure in a comment header for future maintainers
- [ ] **Cross-platform note**: vttest tests require a Unix PTY (portable-pty on Linux/macOS). On Windows (cross-compile target), these tests are skipped -- the PTY spawn will fail gracefully. Verify that `test-all.sh` handles this (the existing vttest.rs likely already skips on Windows via `portable_pty::native_pty_system()` returning an error). Document this in the vttest.rs header.
- [ ] Verify the plan's changes compile for the Windows cross-compile target: `cargo build --target x86_64-pc-windows-gnu` -- the ColorEase and DECCOLM changes are platform-independent, but verify no Unix-only imports leaked in.

---

## 07.R Third Party Review Findings

- None.

---

## 07.N Completion Checklist

- [ ] Conformance audit complete with per-menu scoring
- [ ] Pass rate >= 90% across menus 1-3, 5-6, 8 (excluding menu 4 DECDHL/DECDWL and menu 7 VT52 -- not implemented)
- [ ] All performance invariants verified
- [ ] All builds green
- [ ] All tests green
- [ ] Documentation updated
- [ ] `/tpr-review` passed

**Exit Criteria:** vttest pass rate >= 90% across menus 1-3, 5-6, 8 at 80x24 (excluding menu 4 DECDHL/DECDWL and menu 7 VT52 which are not implemented), with structural assertions automatically catching regressions. All golden images committed. No performance regressions. Fade blink visually smooth and verified by multi-frame capture.
