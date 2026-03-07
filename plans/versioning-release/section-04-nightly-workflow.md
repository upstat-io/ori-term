---
section: "04"
title: "Nightly Workflow"
status: complete
goal: "Every push to main builds and publishes a rolling nightly pre-release"
inspired_by:
  - "WezTerm gen_*_continuous.yml (rolling nightly tag, daily + push trigger)"
  - "Ghostty release-tip.yml (force-push tip tag, prerelease)"
  - "Neovim nightly release (rolling nightly tag, prerelease)"
depends_on: ["03"]
sections:
  - id: "04.1"
    title: "Nightly workflow file"
    status: complete
  - id: "04.2"
    title: "Rolling release strategy"
    status: complete
  - id: "04.3"
    title: "Completion Checklist"
    status: complete
---

# Section 04: Nightly Workflow

**Status:** Complete
**Goal:** A GitHub Actions workflow that fires on every push to `main`,
builds release binaries for Linux and Windows, and publishes them to a
rolling `nightly` pre-release on GitHub.

**Context:** The existing `release.yml` only fires on `v*` tags. There is
no automated build for the `main` branch. Users who want the latest build
must compile from source. A nightly pipeline gives early adopters access to
pre-built binaries and enables automated testing of the release pipeline.

**Reference implementations:**
- **WezTerm** `gen_*_continuous.yml`: Triggers on push to main + daily
  schedule. Uploads to a `nightly` release with `--clobber`.
- **Ghostty** `release-tip.yml`: Force-pushes a `tip` tag. Deduplicates
  by checking if the commit already has a tip build.
- **Neovim**: Rolling `nightly` tag, pre-release, assets replaced each push.

**Depends on:** Section 03 (binary must report correct version with
`-nightly` channel).

**Risk note:** This workflow can only be fully tested by merging to `main`
(no dry-run for `softprops/action-gh-release`). Test iteratively: first merge
just the workflow file with the release job commented out, verify builds
succeed, then uncomment the release job in a follow-up push.

**Note:** The CI workflow (`.github/workflows/ci.yml`) also triggers on push
to `main`. The nightly workflow runs independently and concurrently with CI.
CI validates code quality; nightly produces release artifacts.

**Note:** macOS builds are intentionally omitted from the nightly workflow.
The CI runs tests on macOS, but release binaries for macOS are deferred until
the macOS platform is fully supported. Add a `build-macos` job when ready.

---

## 04.1 Nightly workflow file

**File(s):** `.github/workflows/nightly.yml`

- [ ] Create `.github/workflows/nightly.yml`:

  ```yaml
  name: Nightly

  on:
    push:
      branches: [main]

  concurrency:
    group: nightly
    cancel-in-progress: true

  env:
    CARGO_TERM_COLOR: always
    ORITERM_CHANNEL: nightly

  permissions:
    contents: write

  jobs:
    build-linux:
      name: Build Linux (x86_64)
      runs-on: ubuntu-latest
      timeout-minutes: 20
      steps:
        - uses: actions/checkout@v4
        - uses: dtolnay/rust-toolchain@stable
        - name: Install system dependencies
          run: |
            sudo apt-get update
            sudo apt-get install -y \
              pkg-config libx11-dev libxi-dev libxcursor-dev \
              libxrandr-dev libxinerama-dev libwayland-dev \
              libxkbcommon-dev libegl-dev libvulkan-dev
        - uses: Swatinem/rust-cache@v2
          with:
            key: nightly-linux
        - name: Build
          run: cargo build --release
        - name: Strip binaries
          run: |
            strip target/release/oriterm
            strip target/release/oriterm-mux
        - name: Package
          run: |
            SHORT_SHA=$(git rev-parse --short=7 HEAD)
            DATE=$(date -u +%Y%m%d)
            tar -czvf oriterm-nightly-${DATE}-${SHORT_SHA}-linux-x86_64.tar.gz \
              -C target/release oriterm oriterm-mux
        - name: Upload artifact
          uses: actions/upload-artifact@v4
          with:
            name: oriterm-nightly-linux-x86_64
            path: oriterm-nightly-*-linux-x86_64.tar.gz

    build-windows:
      name: Build Windows (x86_64)
      runs-on: windows-2022
      timeout-minutes: 30
      # CI builds natively on Windows using MSVC (not the x86_64-pc-windows-gnu
      # cross-compile target used for local dev). MSVC produces better debuginfo
      # and avoids MinGW runtime dependencies.
      steps:
        - uses: actions/checkout@v4
        - uses: dtolnay/rust-toolchain@stable
          with:
            targets: x86_64-pc-windows-msvc
        - uses: Swatinem/rust-cache@v2
          with:
            key: nightly-windows
        - name: Build
          run: cargo build --release --target x86_64-pc-windows-msvc
        - name: Package
          shell: bash
          run: |
            SHORT_SHA=$(git rev-parse --short=7 HEAD)
            DATE=$(date -u +%Y%m%d)
            7z a oriterm-nightly-${DATE}-${SHORT_SHA}-windows-x86_64.zip \
              ./target/x86_64-pc-windows-msvc/release/oriterm.exe \
              ./target/x86_64-pc-windows-msvc/release/oriterm-mux.exe
        - name: Upload artifact
          uses: actions/upload-artifact@v4
          with:
            name: oriterm-nightly-windows-x86_64
            path: oriterm-nightly-*-windows-x86_64.zip

    release:
      name: Publish Nightly
      needs: [build-linux, build-windows]
      runs-on: ubuntu-latest
      steps:
        - uses: actions/checkout@v4

        - name: Download artifacts
          uses: actions/download-artifact@v4
          with:
            path: artifacts
            merge-multiple: true

        - name: Generate checksums
          run: |
            cd artifacts
            sha256sum oriterm-* > checksums-sha256.txt
            cat checksums-sha256.txt

        - name: Compute release metadata
          id: meta
          run: |
            SHORT_SHA=$(git rev-parse --short=7 HEAD)
            DATE=$(date -u +%Y-%m-%d)
            echo "short_sha=$SHORT_SHA" >> $GITHUB_OUTPUT
            echo "date=$DATE" >> $GITHUB_OUTPUT
            echo "name=Nightly ($DATE / $SHORT_SHA)" >> $GITHUB_OUTPUT

        # softprops/action-gh-release appends assets; delete old ones first.
        - name: Delete old nightly release assets
          run: |
            gh release view nightly --json assets --jq '.assets[].name' 2>/dev/null | while read -r asset; do
              gh release delete-asset nightly "$asset" --yes
            done
          env:
            GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          continue-on-error: true

        - name: Update nightly release
          uses: softprops/action-gh-release@v2
          with:
            tag_name: nightly
            name: ${{ steps.meta.outputs.name }}
            body: |
              Rolling nightly build from `main`.

              **Commit:** ${{ github.sha }}
              **Date:** ${{ steps.meta.outputs.date }}

              This is an automated pre-release. For stable builds, see
              [Releases](../../releases?q=NOT+nightly).
            files: artifacts/*
            prerelease: true
            make_latest: false
          env:
            GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
  ```

---

## 04.2 Rolling release strategy

The nightly uses **semver-compliant per-day tags** matching the binary version:

- Tag format: `v{cargo_version}-nightly.{YYYYMMDD}`
  (e.g., `v0.1.0-alpha.3-nightly.20260307`).
- The tag matches what the binary reports via `--version`, so any GitHub
  release maps unambiguously to a specific build.
- Multiple pushes on the same day update the same tag (the cleanup step
  deletes the existing release for that day before creating a new one).
- The release is always `prerelease: true` and `make_latest: false` so it
  never appears as the "Latest Release" on the repo page.
- Release title: `oriterm 0.1.0-alpha.3-nightly.20260307`.
- Artifact filenames include date + hash for downloaded file identification:
  `oriterm-nightly-20260307-abc1234-windows-x86_64.zip`
- `concurrency: nightly` with `cancel-in-progress: true` ensures rapid
  pushes don't stack up builds.

**Why per-day semver tags (not a rolling `nightly` tag):**
- The tag should match the version the binary reports — consistency matters.
- A bare `nightly` tag is not semver and can't be sorted or compared.
- One tag per day is a reasonable cadence — not polluting, still traceable.

---

## 04.3 Completion Checklist

- [ ] `.github/workflows/nightly.yml` exists
- [ ] Workflow triggers on push to `main` only
- [ ] Linux x86_64 binary built with `ORITERM_CHANNEL=nightly`
- [ ] Windows x86_64 binary built with `ORITERM_CHANNEL=nightly`
- [ ] Checksums generated
- [ ] Rolling `nightly` pre-release created/updated on GitHub
- [ ] Release title includes date and short SHA
- [ ] Nightly never appears as "Latest Release"
- [ ] `concurrency` prevents stacked builds
- [ ] Downloaded binary: `oriterm --version` shows `-nightly` channel
- [ ] Both platform archives include `oriterm-mux` alongside `oriterm`
- [ ] First run with no existing `nightly` release succeeds (creates release
  from scratch)
- [ ] Nightly workflow does not interfere with CI workflow (both trigger on
  push to main, run independently)

**Exit Criteria:** Push a commit to `main`, verify the nightly workflow
runs, produces both platform binaries, and the downloaded `oriterm --version`
outputs `oriterm 0.1.0-alpha.3-nightly (abc1234 2026-03-07)`.
