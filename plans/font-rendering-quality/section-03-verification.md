---
section: "03"
title: "Verification"
status: not-started
reviewed: false
goal: "Verify all bug fixes and settings work end-to-end, update bug tracker, and confirm build/test green."
depends_on: ["01", "02"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "03.1"
    title: "Test Matrix"
    status: not-started
  - id: "03.2"
    title: "Build & Verify"
    status: not-started
  - id: "03.3"
    title: "Bug Tracker Updates"
    status: not-started
  - id: "03.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "03.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 03: Verification

**Status:** Not Started
**Goal:** Full test matrix green, all bug tracker items resolved, build/clippy/test all passing.

**Depends on:** Section 01 (bug fixes), Section 02 (settings UI).

---

## 03.1 Test Matrix

- [ ] **Bug fixes (Section 01 — 13 tests total):**
  - 01.1: 3 tests in `window_renderer/tests.rs` — DPI change UI font preservation
  - 01.2: 4 tests in `atlas/tests.rs` — gutter zeroing (GPU-dependent, may skip)
  - 01.3: 6 tests in `prepare/tests.rs` — Y rounding across all emission paths

- [ ] **Config + resolution (Section 02.1 — 16 tests total):**
  - 3 updated existing tests in `config/tests.rs` for `Option<bool>` type change
  - 13 new tests: deserialization (5), resolution functions (8)

- [ ] **Renderer wiring (Section 02.2 — 10 tests total):**
  - 3 in `bind_groups/tests.rs` — filter mode storage and rebuild
  - 2 in `prepare/tests.rs` — subpixel positioning flag effect (grid path)
  - 2 in `window_renderer/tests.rs` — raster key subpixel positioning (grid + scene)
  - 1 in `scene_convert/tests.rs` — UI text subpixel positioning
  - 2 in `settings_overlay/tests.rs` — change detection for new fields

- [ ] **Settings UI (Section 02.3 — 14 tests total):**
  - 1 in `form_builder/tests.rs` — Advanced dropdowns default to Auto
  - 13 in `action_handler/tests.rs` — dropdown selection → config update

- [ ] **Migration (Section 02.4 — 4 tests total):**
  - 2 in `settings_overlay/tests.rs` — `per_page_dirty` migration correctness
  - 1 in `action_handler/tests.rs` — removed toggle regression guard
  - 1 count update in `form_builder/tests.rs`

- [ ] **Cross-cutting verification:**
  - `per_page_dirty`: Font page (2) dirty for `subpixel_mode` change, Rendering page (7) NOT dirty
  - `per_page_dirty`: Font page (2) dirty for `atlas_filtering` change, `subpixel_positioning` change
  - All new `SettingsIds` fields are distinct (verified by `settings_ids_all_distinct`)
  - All new IDs are non-placeholder (verified by `all_page_ids_are_set`)

---

## 03.2 Build & Verify

- [ ] `./build-all.sh` green (all platforms)
- [ ] `./clippy-all.sh` green (no warnings)
- [ ] `./test-all.sh` green (all tests pass)

---

## 03.3 Bug Tracker Updates

- [ ] Mark TPR-04-006 as fixed in `plans/bug-tracker/section-04-fonts.md`
- [ ] Mark TPR-04-007 as fixed (subpixel_positioning no longer dead)
- [ ] Mark TPR-04-008 as fixed
- [ ] Mark TPR-04-010 as fixed
- [ ] Mark TPR-04-011 as fixed (atlas filtering now user-configurable)
- [ ] Mark BUG-04-002 as fixed (DPI change blur — root cause was TPR-04-006)
- [ ] Update bug tracker section frontmatter: `third_party_review.status: resolved`

---

## 03.R Third Party Review Findings

- None.

---

## 03.N Completion Checklist

- [ ] All test matrix items verified (~57 total tests across sections)
- [ ] No dead code introduced: `dead_code = "deny"` lint passes (verify `default_true()` removed from `font_config.rs`, `subpixel_toggle` removed from all paths)
- [ ] No new clippy warnings: `deny(clippy::all)` + nursery passes
- [ ] `./build-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./test-all.sh` green
- [ ] Bug tracker items updated with fix dates and resolution notes
- [ ] `/tpr-review` passed clean

**Exit Criteria:** All tests pass, all builds green, all 5 TPR findings (04-006 through 04-011, excluding 04-009) and BUG-04-002 resolved in the bug tracker.
