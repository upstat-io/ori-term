---
section: "07"
title: "CI & Build Bugs"
status: not-started
reviewed: false
goal: "Track and fix bugs in CI workflows, release automation, and build scripts"
depends_on: []
third_party_review:
  status: findings
  updated: 2026-03-29
sections:
  - id: "07.1"
    title: "Active Bugs"
    status: not-started
  - id: "07.R"
    title: "Third Party Review Findings"
    status: in-progress
---

# Section 07: CI & Build Bugs

**Status:** Not Started
**Goal:** Track and fix bugs in CI workflows, release automation, and build scripts.

**Note:** This section is never marked complete. New bugs are appended as discovered.

---

## 07.1 Active Bugs

- None.

---

## 07.R Third Party Review Findings

- [ ] `[BUG-07-001][high]` `scripts/bump-build.sh:61`, `.github/workflows/auto-release.yml:75` — Auto-release flow can only produce one release per UTC day. `bump-build.sh` derives the version suffix from `YYYYMMDD` only, and `auto-release.yml` exits early when that tag already exists. A second merge on the same UTC date silently skips release creation with no error or notification.

- [ ] `[BUG-07-002][high]` `.github/workflows/release.yml:27` — `release.yml` no longer validates that the pushed tag matches the workspace version. Current validation only checks "tag is on main" + `sync-version.sh --check`; it never compares `github.ref_name` to the version in `Cargo.toml`. A manually pushed tag like `v9.9.9` would pass validation and produce a release with a version that does not match the crate metadata.

- [ ] `[BUG-07-003][medium]` `.github/workflows/auto-release.yml:104`, `scripts/sync-version.sh:35` — Auto-release commit path stages `Cargo.lock` without regenerating it. `auto-release.yml` runs only `sync-version.sh` before `git add BUILD_NUMBER Cargo.toml Cargo.lock`, but `sync-version.sh` only edits the root `Cargo.toml`. The committed lockfile may be stale (still referencing the previous version string), causing a mismatch between the committed `Cargo.toml` version and the lockfile's recorded version.

---
