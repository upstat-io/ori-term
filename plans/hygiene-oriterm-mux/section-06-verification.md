---
section: "06"
title: "Verification"
status: complete
goal: "All hygiene fixes pass build, clippy, test, and format checks with zero regressions"
depends_on: ["01", "02", "03", "04", "05"]
sections:
  - id: "06.1"
    title: "Full Build Verification"
    status: complete
  - id: "06.2"
    title: "Test Suite Verification"
    status: complete
  - id: "06.3"
    title: "Cross-Crate Impact Check"
    status: complete
  - id: "06.4"
    title: "Completion Checklist"
    status: complete
---

# Section 06: Verification

**Status:** Complete
**Goal:** All 36 hygiene findings are resolved. The entire workspace builds, passes clippy, passes all tests, and formats correctly. No regressions in any crate.

**Context:** Hygiene fixes are individually low-risk, but in aggregate they touch many files across the crate. This section runs the full verification gauntlet to ensure nothing was missed or broken.

**Depends on:** Sections 01-05 (all prior sections must be complete).

---

## 06.1 Full Build Verification

- [x] `./build-all.sh` passes (all targets, including cross-compilation to `x86_64-pc-windows-gnu`)
- [x] `./clippy-all.sh` passes (no new warnings, no regressions)
- [x] `./fmt-all.sh` passes (or run `cargo fmt` and verify no changes)

---

## 06.2 Test Suite Verification

- [x] `./test-all.sh` passes (all crates in workspace)
- [x] `cargo test -p oriterm_mux` passes specifically (the primary crate under modification)
- [x] `cargo test -p oriterm` passes (catches cross-crate breakage from visibility changes and notification renames)

---

## 06.3 Cross-Crate Impact Check

Several findings affect types consumed by `oriterm` (the GUI crate):

- [x] `MuxNotification::PaneTitleChanged` renamed to `PaneMetadataChanged` — verified `oriterm` consumers updated
- [x] `MuxNotification::PaneClosed` now carries `exit_code` — verified `oriterm` match arms destructure correctly
- [x] `MuxNotification` import paths normalized — verified `oriterm` uses root re-export
- [x] No `oriterm` code imports types that were narrowed to `pub(crate)` in `oriterm_mux`

---

## 06.4 Completion Checklist

- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./test-all.sh` green
- [x] `./fmt-all.sh` green (no formatting changes)
- [x] All 36 findings addressed (cross-check against overview)
- [x] No `TODO` or `FIXME` comments introduced without actionable context

**Exit Criteria:** Running `./build-all.sh && ./clippy-all.sh && ./test-all.sh` produces zero errors and zero new warnings. All 36 findings from the hygiene review are resolved.
