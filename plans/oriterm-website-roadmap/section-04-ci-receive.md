---
section: "04"
title: "CI Receive"
status: in-progress
reviewed: false
goal: "Update ori_term_website deploy.yml to receive dispatches, clone ori_term, and rebuild"
inspired_by:
  - "ori-lang-website deploy.yml (checkout + symlink pattern)"
depends_on: ["02", "03"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "04.1"
    title: "Add repository_dispatch trigger"
    status: complete
  - id: "04.2"
    title: "Add ori_term checkout and symlink steps"
    status: complete
  - id: "04.3"
    title: "End-to-end verification"
    status: not-started
  - id: "04.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "04.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 04: CI Receive

**Status:** Not Started
**Goal:** Update the ori_term_website's `deploy.yml` to also trigger on `repository_dispatch`, checkout the ori_term repo so the data loader can read roadmap files, and rebuild/deploy.

**Context:** The existing `deploy.yml` already handles push-to-main and manual dispatch triggers, and deploys to GitHub Pages. We need to add: (1) the `repository_dispatch` trigger from Section 03, (2) a step to clone the ori_term repo, and (3) a symlink so the relative path `../ori_term/` resolves correctly in the GitHub Actions workspace.

**Reference implementations:**
- **ori-lang-website** `deploy.yml`: Checks out `ori_lang` into `$GITHUB_WORKSPACE/ori_lang`, then symlinks it to `$GITHUB_WORKSPACE/../ori_lang`. We follow the exact same pattern.

**Depends on:** Section 02 (build must work with real data), Section 03 (dispatch must be configured).

---

## 04.1 Add repository_dispatch trigger

**File(s):** `~/projects/ori_term_website/.github/workflows/deploy.yml`

- [x] Add `repository_dispatch` to the existing `on:` block:
  ```yaml
  on:
    push:
      branches: [main]
    repository_dispatch:
      types: [oriterm-roadmap-updated]
    workflow_dispatch:
  ```
  The `types` filter ensures only the specific event from ori_term triggers a rebuild, not arbitrary dispatches.

---

## 04.2 Add ori_term checkout and symlink steps

**File(s):** `~/projects/ori_term_website/.github/workflows/deploy.yml`

Add steps to the build job, BEFORE the `npm ci` / `npm run build` steps:

- [x] Add ori_term checkout step:
  ```yaml
  - name: Checkout ori_term (roadmap data)
    uses: actions/checkout@v4
    with:
      repository: upstat-io/ori_term
      path: ori_term
      sparse-checkout: |
        plans/roadmap
      sparse-checkout-cone-mode: false
  ```
  Uses sparse checkout to only fetch `plans/roadmap/` — no need for the entire ori_term repo (saves time and bandwidth).

- [x] Add symlink step:
  ```yaml
  - name: Symlink ori_term for relative path resolution
    run: ln -s "$GITHUB_WORKSPACE/ori_term" "$GITHUB_WORKSPACE/../ori_term"
  ```
  This makes `../ori_term/plans/roadmap/` resolve correctly from the website's working directory, matching the local dev setup.

- [x] Verify the final `deploy.yml` step order:
  1. Checkout website (existing)
  2. **Checkout ori_term** (new)
  3. **Symlink ori_term** (new)
  4. Setup Node (existing)
  5. npm ci (existing)
  6. npm run build (existing)
  7. Upload artifact (existing)
  8. Deploy (existing)

---

## 04.3 End-to-end verification

- [ ] Push the updated `deploy.yml` to ori_term_website main
- [ ] Verify the normal push-triggered build still works (checkout + symlink + build + deploy)
- [ ] Trigger a test dispatch from ori_term (push a minor change to `plans/roadmap/` on main)
- [ ] Verify ori_term_website's Actions tab shows a dispatch-triggered run
- [ ] Verify the deployed site reflects the roadmap data correctly

---

## 04.R Third Party Review Findings

- None.

---

## 04.N Completion Checklist

- [x] `deploy.yml` has `repository_dispatch` trigger with type `oriterm-roadmap-updated`
- [x] Build job checks out ori_term with sparse checkout of `plans/roadmap`
- [x] Symlink step creates `$GITHUB_WORKSPACE/../ori_term`
- [ ] Normal push-to-main builds still work
- [ ] Dispatch-triggered builds work (end-to-end from ori_term push to deployed site)
- [ ] Deployed site shows correct roadmap data
- [ ] `/tpr-review` passed

**Exit Criteria:** The full pipeline works end-to-end: push a change to `ori_term/plans/roadmap/section-XX.md` on main → ori_term's `notify-website.yml` fires → ori_term_website's `deploy.yml` receives the dispatch → checks out ori_term → builds with real roadmap data → deploys to GitHub Pages with updated content.
