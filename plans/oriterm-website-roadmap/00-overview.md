---
plan: "oriterm-website-roadmap"
title: "OriTerm Website Roadmap Pipeline: Exhaustive Implementation Plan"
status: in-progress
references:
  - "ori-lang-website/src/lib/plan-data.ts"
  - "ori-lang-website/.github/workflows/deploy.yml"
---

# OriTerm Website Roadmap Pipeline: Exhaustive Implementation Plan

## Mission

Wire the ori_term_website's interactive TUI roadmap page to parse real plan data from the ori_term repo at build time, replacing the hardcoded section data with live filesystem reads. Establish a CI pipeline so that pushes to ori_term's roadmap files automatically trigger a website rebuild and deploy.

## Architecture

```
LOCAL DEV:
  ~/projects/ori_term/plans/roadmap/section-*.md
       |
       | ../ori_term/plans/roadmap/ (relative path)
       v
  ~/projects/ori_term_website/src/lib/roadmap-data.ts
       |
       | loadRoadmapSections() at build time
       v
  ~/projects/ori_term_website/src/pages/roadmap.astro
       |
       | passes Tier[] as props
       v
  ~/projects/ori_term_website/src/components/TmuxRoadmap.svelte


CI (GitHub Actions):
  ori_term push to main (plans/roadmap/**)
       |
       | repository_dispatch event
       v
  ori_term_website deploy.yml
       |
       | checkout ori_term → symlink ../ori_term
       v
  astro build → deploy to GitHub Pages
```

## Design Principles

1. **Relative path portability.** The data loader uses `../ori_term/plans/roadmap/` which resolves correctly both locally (repos side by side in `~/projects/`) and in CI (via symlink from `$GITHUB_WORKSPACE/ori_term` to `$GITHUB_WORKSPACE/../ori_term`). This is the same pattern the ori-lang-website uses for `../ori_lang/plans/roadmap/`.

2. **Build-time only.** All filesystem reads happen in Astro's server-side frontmatter at build time. Zero runtime overhead — the Svelte component receives pre-computed data as props. No client-side data fetching.

3. **Reference implementation fidelity.** Follow the ori-lang-website's proven patterns (symlink strategy, YAML parsing approach, dispatch workflow) rather than inventing new ones.

## Section Dependency Graph

```
Section 01 (Data Loader)
    |
    v
Section 02 (Wire Component)  ← depends on Section 01
    |
    v
Section 03 (CI Dispatch)     ← independent of 01-02 (different repo)
    |
    v
Section 04 (CI Receive)      ← depends on 02 (needs build to work) + 03 (needs dispatch)
```

- Section 01 must come first — it creates the data loading infrastructure.
- Section 02 depends on 01 — it wires the loader output into the component.
- Section 03 is independent of 01-02 (it's in the ori_term repo, not the website).
- Section 04 depends on 02 (the build must work with real data) and 03 (it must receive the dispatch).

## Implementation Sequence

```
Phase 1 - Data Infrastructure
  └─ Section 01: Create roadmap-data.ts with YAML parsing and tier grouping

Phase 2 - Integration
  └─ Section 02: Wire TmuxRoadmap to accept real data via props
  Gate: `npm run build` succeeds reading from ../ori_term/plans/roadmap/

Phase 3 - CI Pipeline
  └─ Section 03: Create dispatch workflow in ori_term
  └─ Section 04: Update deploy.yml in ori_term_website to receive + build
  Gate: Push to ori_term/plans/roadmap/ triggers website rebuild
```

**Why this order:**
- Phase 1-2 are pure website changes — no CI complexity.
- Phase 3 requires Phase 2 to work (the build must succeed with real data before automating it).

## Metrics (Current State)

| Area | Files | Lines |
|------|-------|-------|
| ori_term_website (total) | ~60 | ~3,900 |
| TmuxRoadmap.svelte | 1 | ~717 |
| roadmap.astro | 1 | ~18 |
| deploy.yml | 1 | ~40 |
| ori_term/plans/roadmap/ | 53 | N/A (markdown) |

## Estimated Effort

| Section | Est. Lines | Complexity | Depends On |
|---------|-----------|------------|------------|
| 01 Data Loader | ~80 | Low | — |
| 02 Wire Component | ~-80 (net removal) | Low | 01 |
| 03 CI Dispatch | ~30 | Low | — |
| 04 CI Receive | ~20 (additions to deploy.yml) | Low | 02, 03 |
| **Total new** | **~50 net** | | |

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Data Loader | `section-01-data-loader.md` | Complete |
| 02 | Wire Component | `section-02-wire-component.md` | Complete |
| 03 | CI Dispatch | `section-03-ci-dispatch.md` | Not Started |
| 04 | CI Receive | `section-04-ci-receive.md` | Not Started |

<!-- tp-help skipped: Steps 4b, 6b, 7b, 8b — scope is clear with reference implementation to follow, user approved architecture directly -->
