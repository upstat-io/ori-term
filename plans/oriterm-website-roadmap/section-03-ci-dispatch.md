---
section: "03"
title: "CI Dispatch"
status: not-started
reviewed: false
goal: "Create a GitHub Action in ori_term that dispatches to ori_term_website on roadmap changes"
inspired_by:
  - "ori-lang-website deploy.yml repository_dispatch trigger"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "03.1"
    title: "Create dispatch workflow"
    status: not-started
  - id: "03.2"
    title: "Configure PAT secret"
    status: not-started
  - id: "03.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "03.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 03: CI Dispatch

**Status:** Not Started
**Goal:** Create `.github/workflows/notify-website.yml` in the ori_term repo that sends a `repository_dispatch` event to `ori_term_website` whenever roadmap plan files change on main.

**Context:** The website is a static build — it only reflects roadmap changes when rebuilt. This workflow automates the rebuild trigger so that updating a plan section's status in ori_term automatically propagates to the live website.

**Reference implementations:**
- **ori-lang-website** `deploy.yml`: Listens for `repository_dispatch` event `ori-lang-content-updated`. We create the sender side of this pattern.
- **peter-evans/repository-dispatch** GitHub Action: Standard action for cross-repo dispatch.

**Depends on:** None (independent of Sections 01-02, different repo).

---

## 03.1 Create dispatch workflow

**File(s):** `~/projects/ori_term/.github/workflows/notify-website.yml` (new file)

- [ ] Create the workflow file:
  ```yaml
  name: Notify Website

  on:
    push:
      branches: [main]
      paths:
        - 'plans/roadmap/**'

  jobs:
    dispatch:
      runs-on: ubuntu-latest
      steps:
        - name: Dispatch to ori_term_website
          uses: peter-evans/repository-dispatch@v3
          with:
            token: ${{ secrets.WEBSITE_DISPATCH_PAT }}
            repository: upstat-io/ori_term_website
            event-type: oriterm-roadmap-updated
  ```

- [ ] Verify the workflow YAML is valid (no syntax errors)
- [ ] The `paths` filter ensures the dispatch only fires when roadmap files change, not on every push

---

## 03.2 Configure PAT secret

This step requires manual action by the user in the GitHub UI.

- [ ] Create a fine-grained Personal Access Token (PAT) at https://github.com/settings/tokens:
  - **Token name**: `ori_term_website_dispatch`
  - **Expiration**: No expiration (or 1 year, user's choice)
  - **Repository access**: Select `upstat-io/ori_term_website` only
  - **Permissions**: Contents → Read and write (required for `repository_dispatch`)
- [ ] Add the PAT as a secret in the ori_term repo:
  - Go to `upstat-io/ori_term` → Settings → Secrets and variables → Actions
  - New repository secret: **Name** = `WEBSITE_DISPATCH_PAT`, **Value** = the PAT
- [ ] Test by pushing a minor change to any file in `plans/roadmap/` on main and checking the Actions tab in both repos

---

## 03.R Third Party Review Findings

- None.

---

## 03.N Completion Checklist

- [ ] `.github/workflows/notify-website.yml` exists in ori_term repo
- [ ] Workflow triggers only on push to main with changes in `plans/roadmap/**`
- [ ] `WEBSITE_DISPATCH_PAT` secret configured in ori_term repo
- [ ] PAT has correct permissions (Contents: Read and write on ori_term_website)
- [ ] Test dispatch: push a plan file change, verify ori_term_website Actions tab shows a triggered run
- [ ] `/tpr-review` passed

**Exit Criteria:** Pushing a change to any file in `ori_term/plans/roadmap/` on the main branch triggers a `repository_dispatch` event that appears in the ori_term_website repo's Actions tab.
