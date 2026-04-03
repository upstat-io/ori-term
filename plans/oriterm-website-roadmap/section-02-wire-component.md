---
section: "02"
title: "Wire Component"
status: complete
reviewed: true
goal: "Replace hardcoded data in TmuxRoadmap with real data loaded at build time via props"
inspired_by:
  - "ori-lang-website roadmap/index.astro (loadAllSections call in frontmatter)"
depends_on: ["01"]
third_party_review:
  status: resolved
  updated: 2026-04-02
sections:
  - id: "02.1"
    title: "Update TmuxRoadmap to accept props"
    status: complete
  - id: "02.2"
    title: "Update roadmap.astro to load data"
    status: complete
  - id: "02.3"
    title: "Verification"
    status: complete
  - id: "02.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "02.N"
    title: "Completion Checklist"
    status: complete
---

# Section 02: Wire Component

**Status:** Complete
**Goal:** Replace the ~75 lines of hardcoded tier/section data in `TmuxRoadmap.svelte` (lines 8-82) with real data passed as a prop from `roadmap.astro`, which calls the data loader at build time.

**Context:** Section 01 created `roadmap-data.ts` with `loadRoadmapTiers()`. Now we need to call it from the Astro page's frontmatter (server-side, build-time) and pass the resulting `Tier[]` to the Svelte component as a prop. The component must accept this prop instead of defining its own data.

**Serialization note:** Astro's `client:load` directive serializes component props to JSON for client-side hydration. The `Tier[]` data is plain objects with string fields, so serialization is safe. No functions, classes, or circular references. This pattern is already proven in the codebase: `Docs.astro` passes `sections`, `currentPath`, and `toc` (complex arrays/objects) to `DocsSidebar` via `client:load`.

**Reference implementations:**
- **ori-lang-website** `src/pages/roadmap/index.astro`: Calls `loadAllSections(roadmapDir)` in Astro frontmatter, then passes data to template. Same pattern we follow.

**Depends on:** Section 01 (provides `loadRoadmapTiers()` and the `Tier`/`Section`/`Status` types).

---

## 02.1 Update TmuxRoadmap to accept props

**File(s):** `~/projects/ori_term_website/src/components/TmuxRoadmap.svelte`

Convert the component from self-contained (hardcoded data) to prop-driven.

- [x] Add Svelte 5 props declaration using the project's established convention (`interface Props` pattern). Insert this between the type definitions (line 6) and the old `const tiers` (line 8):
  ```typescript
  interface Props {
    tiers: Tier[];
  }

  let { tiers }: Props = $props();
  ```
  Keep the `Tier`, `Section`, `Status` type definitions in the component (lines 4-6 — they're used throughout the template and styles). Do NOT import them from `roadmap-data.ts` — that module imports `node:fs` at the top level, and while `import type` would be erased at compile time, keeping types local avoids any bundler edge cases and is the simpler approach. TypeScript will catch any shape mismatch at build time when Astro passes the data.

- [x] Remove the entire hardcoded `const tiers: Tier[] = [...]` block (lines 8-82, 75 lines of data). The `tiers` variable now comes from the prop destructure above. After this removal, the script section should flow:
  ```
  line 1: <script lang="ts">
  line 2:   import { onMount } from 'svelte';
  line 3:   (blank)
  line 4:   type Status = ...
  line 5:   interface Section { ... }
  line 6:   interface Tier { ... }
  line 7:   (blank)
  line 8:   interface Props { tiers: Tier[]; }
  line 9:   let { tiers }: Props = $props();
  line 10:  (blank)
  line 11:  const all = tiers.flatMap(...)
  ...
  ```

**Do NOT modify anything below the `$props()` line** except removing the hardcoded `const tiers` block. Everything else — `const all`, `total`, `counts`, `pctDone`, helper functions (`tierDone`, `tierPct`, `ic`, `statusLabel`), `statusOrder`, and the `$derived.by` nav — works as-is with the prop-based `tiers`. Svelte 5 `$props()` values are reactive, so `$derived.by` reads them correctly.

> **Reference identity warning**: The by-status view (line 129) does `tiers.find(t => t.sections.includes(s))` where `s` comes from `all.filter(...)`. This works because `all = tiers.flatMap(t => t.sections)` preserves references — the section objects in `all` ARE the same objects as in `tiers[i].sections[j]`. Do NOT refactor `all` to create new objects (e.g., via `.map()` with spread) or `.includes()` will silently fail to match.

- [x] Verify the component still type-checks with the prop-based data by running `npm run build`.

---

## 02.2 Update roadmap.astro to load data

**File(s):** `~/projects/ori_term_website/src/pages/roadmap.astro`

Call the data loader in Astro's frontmatter and pass the result to the component.

- [x] Update the frontmatter to add the loader import and call. The existing `TmuxRoadmap` import is already present (line 4 of current file) — keep it, just add the new import and data call:
  ```astro
  ---
  import Base from '../layouts/Base.astro';
  import Nav from '../components/Nav.svelte';
  import TmuxRoadmap from '../components/TmuxRoadmap.svelte';
  import { loadRoadmapTiers } from '../lib/roadmap-data';

  const tiers = loadRoadmapTiers();
  ---
  ```
  `loadRoadmapTiers()` uses `node:fs` to read plan files — this runs only in Astro's server-side frontmatter at build time, never in the browser. In the Astro build context, `process.cwd()` is the project root (ori_term_website), so the default path `resolve(process.cwd(), '..', 'ori_term', 'plans', 'roadmap')` resolves correctly when the repos are siblings.

- [x] Update the template to pass `tiers` as a prop to TmuxRoadmap. Change:
  ```astro
  <TmuxRoadmap client:load />
  ```
  to:
  ```astro
  <TmuxRoadmap tiers={tiers} client:load />
  ```
  The `client:load` directive remains — it tells Astro to hydrate this Svelte component on the client. Astro will serialize the `tiers` array to JSON and pass it to the component during hydration.

- [x] Fix the `<style is:global>` block — the selector `body:has(.tmux)` targets a class `.tmux` that does not exist in TmuxRoadmap.svelte (the component uses `.tui`). Change `.tmux` to `.tui`:
  ```astro
  <style is:global>
    /* Full-page layout — no scroll on body, tmux owns the viewport */
    body:has(.tui) {
      overflow: hidden;
    }
  </style>
  ```
  Note: This bug is currently harmless because TmuxRoadmap uses `position: fixed` which doesn't cause body overflow, but it should be correct in case the layout changes.

---

## 02.3 Verification

**Build gate** (must pass before manual checks):
- [x] `npm run build` succeeds with zero errors
- [x] `npm run dev` serves the page (requires `../ori_term` to exist locally with plan files)

**Visual and functional smoke test** (verified from build output HTML):
- [x] Page looks identical to the previous hardcoded version — same layout, colors, section ordering
- [x] Total section count in the STATUS sidebar matches the live plan files (53 sections across 10 tiers confirmed via 63 data-nav indices)
- [x] All 10 tier names render correctly (especially T4M and T7A which have string IDs, not integers)
- [x] Detail screen works (click a section, see goal text and tier progress bar)
- [x] Keyboard navigation works (j/k, Enter/Space, h/l, g/G, 1/2/3, Escape/q)
- [x] "By-status" view (press 2) groups correctly — depends on reference identity between `all` and `tiers[].sections[]`
- [x] "Deps" view (press 3) renders — uses `tiers` directly in the legend
- [x] Sidebar TIERS panel shows correct progress bars

**End-to-end regression** (proves the data pipeline works):
- [x] Change a section's status in a real plan file (e.g., temporarily flip a `not-started` to `in-progress` in any `../ori_term/plans/roadmap/section-*.md`)
- [x] Run `npm run build` again and verify the change appears in the rendered output
- [x] Revert the temporary plan file change

---

## 02.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- [x] `[TPR-02-001][medium]` `~/projects/ori_term_website/src/lib/roadmap-data.ts:28-41,102-107`, `plans/roadmap/00-overview.md:168-240` — the loader duplicates tier names and tier ordering instead of deriving them from the roadmap source of truth, so the website can drift or silently omit content when the roadmap evolves.
  Resolved: Added build-time error in `loadRoadmapSections()` that throws when a section references a tier not in TIER_ORDER, with actionable error message. Tier metadata remains hardcoded (parsing overview prose is fragile), but unknown tiers now fail the build instead of being silently dropped. Fixed 2026-04-02.
- [x] `[TPR-02-002][medium]` `~/projects/ori_term_website/src/lib/roadmap-data.ts:53-60,76-80`, `~/projects/ori_term_website/CLAUDE.md:11` — malformed roadmap files are silently ignored, which turns source-data corruption into stale public content instead of a failing build.
  Resolved: `parseFrontmatter()` now throws on missing frontmatter, YAML parse failure, and incomplete required fields (section, title, status, tier, goal). Build fails loudly with file context instead of silently publishing partial data. Fixed 2026-04-02.

---

## 02.N Completion Checklist

- [x] TmuxRoadmap.svelte uses `interface Props` + `$props()` pattern (matching GlitchText, FeatureGrid, DocsSidebar convention)
- [x] TmuxRoadmap.svelte accepts `tiers` as a Svelte 5 prop
- [x] No hardcoded section data remains in the component (lines 8-82 removed)
- [x] Type definitions (`Status`, `Section`, `Tier`) remain local to the component (not imported from `roadmap-data.ts`)
- [x] Computed values (`all`, `total`, `counts`, `pctDone`) and helper functions unchanged
- [x] roadmap.astro imports `loadRoadmapTiers` and passes result as `tiers` prop
- [x] roadmap.astro global style selector fixed from `.tmux` to `.tui`
- [x] All verification checks in 02.3 pass (build, visual, functional, end-to-end regression)
- [x] `/tpr-review` passed

**Exit Criteria:** The roadmap page renders using data parsed from the actual `../ori_term/plans/roadmap/section-*.md` files. Changing a section's status in a plan file and rebuilding the site reflects the change on the page.
