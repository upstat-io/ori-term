# OriTerm Website Roadmap Pipeline Index

> **Maintenance Notice:** Update this index when adding/modifying sections.

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

## Keyword Clusters by Section

### Section 01: Data Loader
**File:** `section-01-data-loader.md` | **Status:** Complete

```
roadmap-data.ts, loadRoadmapSections, groupByTier, parseYamlFrontmatter
js-yaml, yaml, frontmatter, section-*.md, plans/roadmap
Tier[], Section, Status, tier, goal, status normalization
ori-lang-website, plan-data.ts, loadAllSections, reference implementation
```

---

### Section 02: Wire Component
**File:** `section-02-wire-component.md` | **Status:** Not Started

```
TmuxRoadmap.svelte, roadmap.astro, props, hardcoded data removal
$props, Astro.props, build-time data, Svelte 5 props
tiers, sections, pctDone, tierPct, statusOrder
client:load, Astro frontmatter, fs.readdirSync
```

---

### Section 03: CI Dispatch
**File:** `section-03-ci-dispatch.md` | **Status:** Not Started

```
notify-website.yml, repository_dispatch, GitHub Actions
plans/roadmap/**, push trigger, cross-repo, PAT
WEBSITE_DISPATCH_PAT, secrets, peter-evans/repository-dispatch
ori_term/.github/workflows, on.push.paths
```

---

### Section 04: CI Receive
**File:** `section-04-ci-receive.md` | **Status:** Not Started

```
deploy.yml, repository_dispatch trigger, checkout ori_term
symlink, ln -s, GITHUB_WORKSPACE, ../ori_term
actions/checkout, GitHub Pages, build, deploy
ori_term_website/.github/workflows
```

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | Data Loader | `section-01-data-loader.md` |
| 02 | Wire Component | `section-02-wire-component.md` |
| 03 | CI Dispatch | `section-03-ci-dispatch.md` |
| 04 | CI Receive | `section-04-ci-receive.md` |
