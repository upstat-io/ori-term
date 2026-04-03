---
section: "01"
title: "Data Loader"
status: complete
reviewed: true
goal: "Create a TypeScript module that reads ori_term roadmap section files and returns typed Tier[] data"
inspired_by:
  - "ori-lang-website plan-data.ts (~/projects/ori-lang-website/src/lib/plan-data.ts)"
depends_on: []
third_party_review:
  status: resolved
  updated: 2026-04-02
sections:
  - id: "01.1"
    title: "Add js-yaml dependency"
    status: complete
  - id: "01.2"
    title: "Create roadmap-data.ts"
    status: complete
  - id: "01.3"
    title: "Verification"
    status: complete
  - id: "01.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "01.N"
    title: "Completion Checklist"
    status: complete
---

# Section 01: Data Loader

**Status:** Complete
**Goal:** Create `src/lib/roadmap-data.ts` in ori_term_website that reads `section-*.md` files from `../ori_term/plans/roadmap/`, parses YAML frontmatter, and returns typed `Tier[]` data matching the TmuxRoadmap component's interface.

**Context:** The TmuxRoadmap component currently has ~100 lines of hardcoded tier/section data. This data must come from the actual roadmap plan files so it stays in sync with development progress. The ori-lang-website has a reference implementation (`plan-data.ts`) that does the same thing for the ori_lang roadmap.

**Reference implementations:**
- **ori-lang-website** `src/lib/plan-data.ts`: `parseYamlFrontmatter()`, `loadAllSections()` — custom YAML parser and section file loader. We'll use `js-yaml` instead of a custom parser for robustness.

**Depends on:** None.

---

## 01.1 Add js-yaml dependency

**File(s):** `~/projects/ori_term_website/package.json`

Install `js-yaml` for proper YAML frontmatter parsing. This is a build-time dependency only — not shipped to the browser.

- [x] Run `npm install js-yaml` in ori_term_website
- [x] Run `npm install -D @types/js-yaml` for TypeScript types
- [x] Verify `npm run build` still succeeds after adding the dependency

---

## 01.2 Create roadmap-data.ts

**File(s):** `~/projects/ori_term_website/src/lib/roadmap-data.ts` (new file)

Create the data loading module. It must:
1. Read all `section-*.md` files from a configurable directory path
2. Parse YAML frontmatter using `js-yaml`
3. Extract the fields the TmuxRoadmap component needs: `section` (num), `title` (name), `status`, `tier`, `goal`
4. Group sections by tier
5. Return typed `Tier[]` array

- [x] Create `src/lib/roadmap-data.ts` with the following exports:

  ```typescript
  import fs from 'node:fs';
  import path from 'node:path';
  import yaml from 'js-yaml';

  // Types matching TmuxRoadmap.svelte's interfaces
  export type Status = 'complete' | 'in-progress' | 'not-started' | 'superseded' | 'partial';

  export interface Section {
    num: string;
    name: string;
    status: Status;
    goal: string;
  }

  export interface Tier {
    id: string;
    name: string;
    sections: Section[];
  }

  // YAML frontmatter shape from ori_term section files
  interface SectionFrontmatter {
    section: number | string;
    title: string;
    status: string;
    tier: number | string;
    goal: string;
  }
  ```

- [x] Implement `normalizeStatus()`:
  ```typescript
  function normalizeStatus(raw: string): Status {
    const s = raw.toLowerCase().replace(/_/g, '-');
    if (s === 'complete') return 'complete';
    if (s === 'in-progress') return 'in-progress';
    if (s === 'not-started') return 'not-started';
    if (s === 'superseded') return 'superseded';
    if (s === 'partially-started' || s === 'partial') return 'partial';
    return 'not-started';
  }
  ```

- [x] Implement `parseFrontmatter()` — extract YAML between `---` delimiters:
  ```typescript
  function parseFrontmatter(content: string): SectionFrontmatter | null {
    const match = content.match(/^---\n([\s\S]*?)\n---/);
    if (!match) return null;
    try {
      return yaml.load(match[1]) as SectionFrontmatter;
    } catch {
      return null;
    }
  }
  ```

- [x] Implement `loadRoadmapSections()`:
  ```typescript
  export function loadRoadmapSections(dir: string): Section[] {
    const files = fs.readdirSync(dir)
      .filter(f => f.startsWith('section-') && f.endsWith('.md'))
      .sort();

    const sections: Section[] = [];
    for (const file of files) {
      const content = fs.readFileSync(path.join(dir, file), 'utf-8');
      const fm = parseFrontmatter(content);
      if (!fm) continue;

      sections.push({
        num: String(fm.section).replace(/^0+(?=\d)/, ''),
        name: fm.title,
        status: normalizeStatus(fm.status),
        goal: fm.goal,
      });
    }
    return sections;
  }
  ```
  Note: `num` strips leading zeros (e.g., section `01` becomes `"1"`, but `"5B"` stays `"5B"`). Wait — the component currently uses `'01'`, `'5B'`, etc. Let me keep the original format. Actually, the frontmatter has `section: 1` (integer) and `section: "5B"` (string). We need to format integers with zero-padding for single digits:
  ```typescript
  num: typeof fm.section === 'number' && fm.section < 10
    ? String(fm.section).padStart(2, '0')
    : String(fm.section),
  ```

- [x] Implement `groupByTier()`:
  ```typescript
  // Tier metadata — canonical tier names
  const TIER_META: Record<string, string> = {
    '0': 'CORE LIBRARY + CROSS-PLATFORM',
    '1': 'PROCESS LAYER',
    '2': 'RENDERING FOUNDATION',
    '3': 'INTERACTION',
    '4': 'CHROME',
    '4M': 'MULTIPLEXING FOUNDATION',
    '5': 'HARDENING',
    '6': 'POLISH',
    '7': 'ADVANCED',
    '7A': 'SERVER + PERSISTENCE + REMOTE',
  };

  const TIER_ORDER = ['0', '1', '2', '3', '4', '4M', '5', '6', '7', '7A'];

  export function groupByTier(sections: Section[], tiers: Record<string, string>): Tier[] {
    // ... group sections by tier, sort by TIER_ORDER
  }
  ```
  Wait — the sections don't carry tier info after loading. We need to also extract the `tier` field from frontmatter. Let me adjust:

  ```typescript
  interface SectionWithTier extends Section {
    tier: string;
  }

  export function loadRoadmapSections(dir: string): SectionWithTier[] {
    // ... same as above but also includes tier: String(fm.tier)
  }

  export function loadRoadmapTiers(dir?: string): Tier[] {
    const roadmapDir = dir ?? path.join(process.cwd(), '..', 'ori_term', 'plans', 'roadmap');
    const sections = loadRoadmapSections(roadmapDir);

    const tierMap = new Map<string, Section[]>();
    for (const s of sections) {
      const tid = String(s.tier);
      if (!tierMap.has(tid)) tierMap.set(tid, []);
      tierMap.get(tid)!.push({ num: s.num, name: s.name, status: s.status, goal: s.goal });
    }

    return TIER_ORDER
      .filter(id => tierMap.has(id))
      .map(id => ({
        id,
        name: TIER_META[id] ?? `TIER ${id}`,
        sections: tierMap.get(id)!,
      }));
  }
  ```

- [x] Export `loadRoadmapTiers` as the primary entry point
- [x] Verify the module compiles: `npx tsc --noEmit src/lib/roadmap-data.ts` (or just `npm run build`)

---

## 01.3 Verification

- [x] Write a quick smoke test: create a temporary Node script that calls `loadRoadmapTiers()` and prints the result. Verify it produces 10 tiers with the correct section counts matching the actual roadmap files.
- [x] Verify `npm run build` succeeds (Astro build doesn't break from the new module)

---

## 01.R Third Party Review Findings

- [x] `[TPR-01-001][medium]` `~/projects/ori_term_website/src/lib/roadmap-data.ts:1-3` — Section 01 marks the loader compile verification complete, but the advertised check does not pass with the current file. Running `npx tsc --noEmit src/lib/roadmap-data.ts` in `ori_term_website` fails because `node:fs`, `node:path`, and `js-yaml` are imported as default exports without `esModuleInterop`.
  Resolved: Switched to named imports (`{ readdirSync, readFileSync }`, `{ join, resolve }`, `{ load as yamlLoad }`) on 2026-04-02. Both `npx tsc --noEmit src/lib/roadmap-data.ts` and `npm run build` now pass clean.
- [x] `[TPR-01-002][low]` `plans/oriterm-website-roadmap/section-01-data-loader.md:4,14-28,33`, `plans/oriterm-website-roadmap/index.md:15-16`, `plans/oriterm-website-roadmap/00-overview.md:117-122` — Section 01 bookkeeping is internally contradictory. Frontmatter says `in-progress` and marks 01.1-01.3 complete, but the prose status line, index entry, and overview quick reference all still say `Not Started`.
  Resolved: Updated prose status, index entry, and overview quick reference to match frontmatter on 2026-04-02.

---

## 01.N Completion Checklist

- [x] `js-yaml` and `@types/js-yaml` installed in ori_term_website
- [x] `src/lib/roadmap-data.ts` exists with `loadRoadmapTiers()` export
- [x] `loadRoadmapTiers()` reads from `../ori_term/plans/roadmap/` by default
- [x] Returns 10 tiers with correct section counts (53 total sections)
- [x] Status normalization handles all 5 values: complete, in-progress, not-started, superseded, partial/partially-started
- [x] `npm run build` succeeds
- [x] `/tpr-review` passed

**Exit Criteria:** `loadRoadmapTiers()` returns a `Tier[]` array matching the shape of the currently hardcoded data in TmuxRoadmap.svelte, with all 53 sections correctly parsed from the real roadmap files.
