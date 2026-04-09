---
section: "21"
title: "notcurses-demo Harness + Scene Matrix + qrcode smoke"
status: not-started
reviewed: false
goal: "Build the notcurses-demo PTY recording / replay harness, the per-scene golden capture infrastructure, and the qrcode scene smoke test (the simplest scene). LANDS EARLY — does NOT wait for every Phase 3 stack to be `verified`. Section 24 (full-pass) depends on this section."
success_criteria:
  - "`crates/oriterm_test_support/src/notcurses_harness/mod.rs` exists with the notcurses-demo PTY recording + replay infrastructure"
  - "Notcurses-demo binary detected at `/usr/bin/notcurses-demo` (Linux); harness gracefully skips when not installed"
  - "Per-scene PTY capture: a scene's byte stream can be captured once via `script -c '/usr/bin/notcurses-demo -p /usr/share/notcurses' /tmp/notcurses-<scene>.cap` then replayed deterministically through ori_term"
  - "**Capture environment pinning (REQUIRED)**: every committed capture file is accompanied by a `<scene>.env.toml` sidecar that pins all capture-time inputs: `TERM` value, terminfo source + sha256 (e.g. `ori_term.info` from tack-conformance section 02 with sha256), `LC_ALL`/`LANG` locale, `notcurses-demo` binary version (`notcurses-demo --version` output), notcurses library version, `/usr/share/notcurses` media set sha256 (the media directory is the asset source), exact capture command string, capture host OS + kernel version, and ISO-8601 capture timestamp. CI verifies sidecar fields against the replay environment before running the test — a mismatch is a CI failure with a clear 'capture drift detected' message, not a test failure."
  - "Per-scene golden capture: after replay, the final ori_term grid + GPU texture is captured as a golden via the deterministic golden lane from section 05"
  - "**qrcode (q) scene smoke test passes**: the simplest scene (~40 lines, deterministic, no media, no animation) is the first scene to fully pass — drives every catalog row it touches and produces a stable golden image"
  - "Per-scene gates infrastructure: each of the 28 scenes can be marked `pass` / `fail` / `not-attempted` independently; section 24 turns each `not-attempted` to `pass` as Phase 3 stacks land"
  - "Catalog row population: any catalog row exercised by the qrcode scene that is `verified` after this section's work updates `_legacy-tack-mapping.md` style trace pointing back to the notcurses scene"
  - "**Section is `complete` when qrcode passes**, NOT when all 28 scenes pass — section 24 owns the full-pass milestone"
  - "All existing tests pass without modification"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **`notcurses-demo` runs cleanly** (delivers the harness; section 24 delivers the full-pass)"
inspired_by:
  - "ori_term existing tack scenario framework — `crates/oriterm_test_support/src/tack_framework/runner/` — pattern for PTY-driven scenario harnesses"
  - "notcurses source `~/projects/reference_repos/console_repos/notcurses/src/demo/qrcode.c` — the simplest scene (~40 lines)"
  - "notcurses-demo scene matrix in `reference_notcurses_demo.md` memory — per-scene subsystem mapping"
depends_on: ["04", "07"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "21.1"
    title: "Build PTY recording + replay harness"
    status: not-started
  - id: "21.2"
    title: "Build per-scene golden capture infrastructure"
    status: not-started
  - id: "21.3"
    title: "Build per-scene gates infrastructure"
    status: not-started
  - id: "21.4"
    title: "Land qrcode scene smoke test"
    status: not-started
  - id: "21.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "21.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 21: notcurses-demo Harness + Scene Matrix + qrcode smoke

**Status:** Not Started
**Goal:** Build the notcurses-demo PTY replay harness and the per-scene gates. Land the qrcode smoke test (simplest scene). This section is the SCAFFOLDING for section 24's full-pass — building it early lets section 24 incrementally add scenes as Phase 3 stacks land, without rebuilding the harness each time.

**Success Criteria:** see frontmatter.

**Context:** Per Codex Step 8B feedback, section 24 must NOT be a forever-open section. Section 21 owns the scaffolding (harness + gates + qrcode smoke); section 24 owns the full-pass gate. The scene matrix from `reference_notcurses_demo.md` memory: 28 scenes in default order `ixetunchdmbkywjgarvlsfqzo`. qrcode (q) is the simplest — ~40 lines, no media, deterministic.

**Reference implementations:** see frontmatter.

**Depends on:** Section 04 (verification chain harness exists; this section extends it for notcurses-demo replay), Section 07 (image lifecycle correct so notcurses scenes that resize don't break the harness).

---

## 21.1 Build PTY recording + replay harness (with capture environment pinning)

**File(s):** `crates/oriterm_test_support/src/notcurses_harness/mod.rs` (new), `crates/oriterm_test_support/src/notcurses_harness/replay.rs` (new), `crates/oriterm_test_support/src/notcurses_harness/env_pin.rs` (new), sibling tests

**Capture environment pinning is a hard requirement.** The notcurses-demo scenes depend on every one of: the `TERM`/`COLORTERM` env, the terminfo database content, locale (`LC_ALL`/`LANG`), the notcurses library version, the notcurses media set (images/fonts under `/usr/share/notcurses`), and the capture command exact string. If any of these drift between capture time and replay time, a passing test silently turns into a stale-baseline test. Every committed capture ships with a sidecar TOML that pins these inputs; CI verifies before replay.

- [ ] Detect notcurses-demo at `/usr/bin/notcurses-demo` (Linux). Provide a `notcurses_demo_available()` predicate that returns false on macOS / Windows where notcurses isn't typically installed.
- [ ] Define `CaptureEnvPin` struct in `notcurses_harness/env_pin.rs`:
  ```rust
  pub struct CaptureEnvPin {
      pub term: String,                     // "ori_term"
      pub colorterm: Option<String>,        // usually "truecolor"
      pub terminfo_source: String,          // path relative to workspace
      pub terminfo_sha256: String,
      pub lc_all: String,                   // "C.UTF-8" canonical
      pub lang: String,
      pub notcurses_demo_version: String,   // `notcurses-demo --version`
      pub notcurses_lib_version: String,    // `ldd` + dpkg query OR pkg-config
      pub media_dir_sha256: String,         // sha256 of sorted file list of /usr/share/notcurses
      pub capture_command: String,          // exact script -c argument
      pub host_os: String,                  // e.g. "Ubuntu 24.04"
      pub host_kernel: String,              // `uname -r`
      pub captured_at: String,              // ISO-8601
  }
  impl CaptureEnvPin {
      pub fn capture_current() -> Result<Self, CapturePinError> { /* ... */ }
      pub fn load_sidecar(capture_path: &Path) -> Result<Self, CapturePinError> { /* ... */ }
      pub fn verify_against_runtime(&self) -> Result<(), CapturePinMismatch> { /* ... */ }
  }
  ```
- [ ] Implement PTY capture: `capture_scene(scene_letter: char) -> PathBuf` runs `script -c '/usr/bin/notcurses-demo -p /usr/share/notcurses ...'` with a scene-specific argument, captures the output to a temp file, AND writes the sidecar `<scene>.env.toml` with `CaptureEnvPin::capture_current()`.
- [ ] **Important**: PTY capture is NOT done at test runtime — it's done ONCE per scene by a developer running `cargo xtask capture-notcurses-scene q` (or similar). The captured byte stream AND the sidecar are committed under `crates/oriterm_test_support/tests/data/notcurses_captures/<scene>.cap` + `<scene>.env.toml`. Test runtime only does REPLAY.
- [ ] Implement PTY replay: `replay_scene(scene_letter: char, ori_term_session: &mut SpecHarness)` reads BOTH the committed capture file AND the sidecar. Calls `CaptureEnvPin::verify_against_runtime()` before replay. On mismatch, fails with `CapturePinMismatch { field, captured, runtime }` — NOT a test failure, an infrastructure failure (exit code 2 or equivalent).
- [ ] Sibling tests in `crates/oriterm_test_support/src/notcurses_harness/tests.rs`:
  - `notcurses_demo_available_returns_true_when_installed()`
  - `replay_scene_feeds_committed_bytes_through_harness()`
  - `capture_env_pin_round_trips_toml()`
  - `verify_against_runtime_flags_term_drift()`
  - `verify_against_runtime_flags_locale_drift()`
  - `verify_against_runtime_flags_notcurses_version_drift()`
  - `verify_against_runtime_flags_media_sha_drift()`
- [ ] **Validation**: replay tests pass; tests gracefully `#[cfg_attr(not(target_os = "linux"), ignore)]` on non-Linux; drift detection fires on every pinned field mutated in isolation.

---

## 21.2 Build per-scene golden capture infrastructure

**File(s):** `crates/oriterm_test_support/src/notcurses_harness/golden.rs` (new), goldens directory

- [ ] After replay, capture the final grid + GPU texture as a golden using the deterministic lane from section 05.
- [ ] Use `headless_env_with_pinned_software_rasterizer(GoldenLaneConfig::SPEC_DEFAULT)` for reproducibility.
- [ ] Goldens stored at `crates/oriterm_test_support/tests/references/notcurses_demo/<scene>.png`
- [ ] Golden capture mode: `ORITERM_UPDATE_GOLDEN=1 cargo test --test notcurses_demo qrcode_scene_smoke` overwrites the golden
- [ ] **Validation**: golden capture works for qrcode scene; back-to-back replays produce 0-pixel diff against the captured golden.

---

## 21.3 Build per-scene gates infrastructure

**File(s):** `crates/oriterm_test_support/src/notcurses_harness/gates.rs` (new), `plans/spec-conformance/notcurses-scene-status.md` (new)

The 28 scenes are independently gated. Each scene starts at `not-attempted`; the per-scene tests in section 24 turn them to `pass` or `fail`. This section creates the tracking infrastructure but only enables one scene (qrcode).

- [ ] Define `SceneStatus { NotAttempted, Pass, Fail { reason: String } }`
- [ ] Define a const table of all 28 scenes (letters: i, x, e, t, u, n, c, h, d, m, b, k, y, w, j, g, a, r, v, l, s, f, q, z, o + intro/outro implicit if separate)
- [ ] Define a `SceneGate` API that section 24 will call: `gate_scene(letter, status)` records the status; `gate_summary()` produces a markdown report of all 28 scenes
- [ ] Plan-side tracker: `plans/spec-conformance/notcurses-scene-status.md` is a markdown file that gets updated as scenes pass. Section 24 owns updates; section 21 creates the file with all 28 scenes at `not-attempted`.
- [ ] **Validation**: gate API works; tracker file exists.

---

## 21.4 Land qrcode scene smoke test

**File(s):** `oriterm_core/tests/notcurses_demo/qrcode_scene.rs` (new), the qrcode capture file, the qrcode golden

- [ ] Capture qrcode scene's PTY output once: `script -c '/usr/bin/notcurses-demo -p /usr/share/notcurses ...' /tmp/qrcode.cap`
  - The exact invocation depends on notcurses-demo's flags for single-scene mode; verify by reading `man notcurses-demo` or running `notcurses-demo --help`
  - Commit the capture under `crates/oriterm_test_support/tests/data/notcurses_captures/q.cap`
- [ ] Spec_chain test: replay q.cap through the harness, capture the golden, assert reproducibility
- [ ] Mark scene `q` as `pass` in the gate tracker
- [ ] Update `plans/spec-conformance/notcurses-scene-status.md`
- [ ] **Validation**: qrcode test passes; back-to-back runs produce 0-pixel diff.

---

## 21.R Third Party Review Findings

- None.

---

## 21.N Completion Checklist

- [ ] Failing test matrix written FIRST
- [ ] **Matrix dimensions**: scene letter × harness operation (capture/replay/golden) — qrcode is the only fully-tested cell at this section's completion
- [ ] **Semantic pin**: qrcode scene smoke test is the regression guard for the harness
- [ ] PTY recording + replay harness exists
- [ ] Per-scene golden capture infrastructure exists
- [ ] Per-scene gates infrastructure exists
- [ ] qrcode scene smoke test passes
- [ ] Per-scene tracker `notcurses-scene-status.md` exists with all 28 scenes at `not-attempted` (except q)
- [ ] All existing tests pass
- [ ] Tests gracefully skip on platforms without notcurses-demo
- [ ] Alloc regression unchanged
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` + `index.md` updated
- [ ] `/tpr-review` passed
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** Notcurses-demo replay harness exists; qrcode smoke test passes; per-scene gates infrastructure ready for section 24 to drive the remaining 27 scenes incrementally.
