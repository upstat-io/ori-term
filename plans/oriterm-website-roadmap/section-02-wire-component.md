---
section: "02"
title: "Wire Component"
status: not-started
reviewed: false
goal: "Replace hardcoded data in TmuxRoadmap with real data loaded at build time via props"
inspired_by:
  - "ori-lang-website roadmap/index.astro (loadAllSections call in frontmatter)"
depends_on: ["01"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "02.1"
    title: "Update TmuxRoadmap to accept props"
    status: not-started
  - id: "02.2"
    title: "Update roadmap.astro to load data"
    status: not-started
  - id: "02.3"
    title: "Verification"
    status: not-started
  - id: "02.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "02.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 02: Wire Component

**Status:** Not Started
**Goal:** Replace the ~100 lines of hardcoded tier/section data in `TmuxRoadmap.svelte` with real data passed as a prop from `roadmap.astro`, which calls the data loader at build time.

**Context:** Section 01 created `roadmap-data.ts` with `loadRoadmapTiers()`. Now we need to call it from the Astro page's frontmatter (server-side, build-time) and pass the resulting `Tier[]` to the Svelte component as a prop. The component must accept this prop instead of defining its own data.

**Reference implementations:**
- **ori-lang-website** `src/pages/roadmap/index.astro`: Calls `loadAllSections(roadmapDir)` in Astro frontmatter, then passes data to template. Same pattern we follow.

**Depends on:** Section 01 (provides `loadRoadmapTiers()` and the `Tier`/`Section`/`Status` types).

---

## 02.1 Update TmuxRoadmap to accept props

**File(s):** `~/projects/ori_term_website/src/components/TmuxRoadmap.svelte`

Convert the component from self-contained (hardcoded data) to prop-driven.

- [ ] Add Svelte 5 props declaration at the top of the script:
  ```typescript
  let { tiers }: { tiers: Tier[] } = $props();
  ```
  Keep the `Tier`, `Section`, `Status` type definitions in the component (they're used throughout the template and styles).

- [ ] Remove the entire hardcoded `const tiers: Tier[] = [...]` block (~100 lines of data). The `tiers` variable now comes from the prop.

- [ ] Keep all computed values (`all`, `total`, `counts`, `pctDone`, helper functions) — they derive from `tiers` which now comes from props instead of hardcoded data. No changes needed to these.

- [ ] Verify the component still type-checks with the prop-based data by running `npm run build`.

---

## 02.2 Update roadmap.astro to load data

**File(s):** `~/projects/ori_term_website/src/pages/roadmap.astro`

Call the data loader in Astro's frontmatter and pass the result to the component.

- [ ] Update the frontmatter to import and call the loader:
  ```astro
  ---
  import Base from '../layouts/Base.astro';
  import Nav from '../components/Nav.svelte';
  import { loadRoadmapTiers } from '../lib/roadmap-data';

  const tiers = loadRoadmapTiers();
  ---

  <Base title="ROADMAP — ori-term" description="Development roadmap for ori-term rendered as an interactive tmux session.">
    <Nav client:load />
    <TmuxRoadmap tiers={tiers} client:load />
  </Base>
  ```

- [ ] Import TmuxRoadmap (already imported, just verify the prop is passed)

- [ ] Verify `npm run dev` serves the page correctly with real data
- [ ] Verify `npm run build` produces the same roadmap content as before (all 53 sections, 10 tiers)

---

## 02.3 Verification

- [ ] Compare the rendered page visually — it should look identical to the hardcoded version
- [ ] Verify section counts match: 24 complete, 12 in-progress, 12 not-started, 2 partial, 3 superseded
- [ ] Verify all tier names render correctly (especially T4M and T7A which have string IDs)
- [ ] Verify the detail screen still works (click a section, see goal text)
- [ ] Verify keyboard navigation still works (j/k, Enter, 1/2/3)

---

## 02.R Third Party Review Findings

- None.

---

## 02.N Completion Checklist

- [ ] TmuxRoadmap.svelte accepts `tiers` as a Svelte 5 prop
- [ ] No hardcoded section data remains in the component
- [ ] roadmap.astro calls `loadRoadmapTiers()` in frontmatter
- [ ] Page renders identically to the previous hardcoded version
- [ ] `npm run build` succeeds
- [ ] `npm run dev` serves the page correctly
- [ ] `/tpr-review` passed

**Exit Criteria:** The roadmap page renders using data parsed from the actual `../ori_term/plans/roadmap/section-*.md` files. Changing a section's status in a plan file and rebuilding the site reflects the change on the page.
