---
section: "03"
title: "Verification"
status: complete
reviewed: false
goal: "Verify all bug fixes and settings work end-to-end, update bug tracker, and confirm build/test green."
depends_on: ["01", "02"]
third_party_review:
  status: resolved
  updated: 2026-03-31
sections:
  - id: "03.1"
    title: "Test Matrix"
    status: complete
  - id: "03.2"
    title: "Build & Verify"
    status: complete
  - id: "03.3"
    title: "Bug Tracker Updates"
    status: complete
  - id: "03.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "03.N"
    title: "Completion Checklist"
    status: complete
---

# Section 03: Verification

**Status:** Complete
**Goal:** Full test matrix green, all bug tracker items resolved, build/clippy/test all passing.

**Depends on:** Section 01 (bug fixes), Section 02 (settings UI).

---

## 03.1 Test Matrix

- [x] **Bug fixes (Section 01 — 13 tests total):**
  - 01.1: 3 tests in `window_renderer/tests.rs` — DPI change UI font preservation
  - 01.2: 4 tests in `atlas/tests.rs` — gutter zeroing (GPU-dependent, may skip)
  - 01.3: 6 tests in `prepare/tests.rs` — Y rounding across all emission paths

- [x] **Config + resolution (Section 02.1 — 16 tests total):**
  - 3 updated existing tests in `config/tests.rs` for `Option<bool>` type change
  - 13 new tests: deserialization (5), resolution functions (8)

- [x] **Renderer wiring (Section 02.2 — 15 tests total):**
  - 5 in `bind_groups/tests.rs` — AtlasFiltering enum unit tests (from_scale_factor, to_filter_mode)
  - 3 in `bind_groups/tests.rs` — AtlasBindGroup filter mode storage and rebuild (GPU-gated)
  - 2 in `prepare/tests.rs` — subpixel positioning flag effect (grid path)
  - 2 in `window_renderer/tests.rs` — raster key subpixel positioning (grid + scene)
  - 1 in `scene_convert/tests.rs` — UI text subpixel positioning
  - 2 in `settings_overlay/tests.rs` — change detection for new fields

- [x] **Settings UI (Section 02.3 — 14 tests total):**
  - 1 in `form_builder/tests.rs` — Advanced dropdowns default to Auto
  - 13 in `action_handler/tests.rs` — dropdown selection → config update

- [x] **Migration (Section 02.4 — 4 tests total):**
  - 2 in `settings_overlay/tests.rs` — `per_page_dirty` migration correctness
  - 1 in `action_handler/tests.rs` — removed toggle regression guard
  - 1 count update in `form_builder/tests.rs`

- [x] **Cross-cutting verification:**
  - `per_page_dirty`: Font page (2) dirty for `subpixel_mode` change, Rendering page (7) NOT dirty
  - `per_page_dirty`: Font page (2) dirty for `atlas_filtering` change, `subpixel_positioning` change
  - All new `SettingsIds` fields are distinct (verified by `settings_ids_all_distinct`)
  - All new IDs are non-placeholder (verified by `all_page_ids_are_set`)

---

## 03.2 Build & Verify

- [x] `./build-all.sh` green (all platforms)
- [x] `./clippy-all.sh` green (no warnings)
- [x] `./test-all.sh` green (all tests pass)

---

## 03.3 Bug Tracker Updates

- [x] Mark TPR-04-006 as fixed in `plans/bug-tracker/section-04-fonts.md` (fixed 2026-03-30)
- [x] Mark TPR-04-007 as fixed (subpixel_positioning no longer dead — fixed 2026-03-31)
- [x] Mark TPR-04-008 as fixed (fixed 2026-03-30)
- [x] Mark TPR-04-010 as fixed (fixed 2026-03-30)
- [x] Mark TPR-04-011 as fixed (atlas filtering now user-configurable — fixed 2026-03-31)
- [x] Mark BUG-04-002 as fixed (DPI change blur — root cause was TPR-04-006, fixed 2026-03-30)
- [x] Update bug tracker section frontmatter — note: `third_party_review.status` remains `findings` because TPR-04-009 (locale-aware font fallback) is still open

---

## 03.R Third Party Review Findings

- [x] `[TPR-03-001][low]` `plans/font-rendering-quality/section-02-advanced-settings.md:38` and `plans/font-rendering-quality/section-03-verification.md:31` — the prose status lines still say `Not Started` even though the frontmatter, checklists, and index/overview files now mark Section 02 complete and Section 03 in progress. The plan metadata and the human-readable section status have drifted apart.
  Resolved: Fixed prose status lines on 2026-03-31. Section 02 now says "Complete", Section 03 says "In Progress".

---

## 03.N Completion Checklist

- [x] All test matrix items verified (~62 total tests across sections) — 557 related tests pass
- [x] No dead code introduced: `dead_code = "deny"` lint passes (verified `default_true()` removed, `subpixel_toggle` removed from all paths)
- [x] No new clippy warnings: `deny(clippy::all)` + nursery passes
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./test-all.sh` green
- [x] Bug tracker items updated with fix dates and resolution notes
- [x] `/tpr-review` passed clean — 3 findings (2 high in GPU init, 1 low prose drift), all fixed 2026-03-31

**Exit Criteria:** All tests pass, all builds green, all 5 TPR findings (04-006 through 04-011, excluding 04-009) and BUG-04-002 resolved in the bug tracker.
