# Section 41: Hints + Quick Select -- Verification Results

**Verified:** 2026-03-29
**Status:** CONFIRMED NOT STARTED
**Reviewed:** false (unreviewed gate)

---

## 1. Code Search: Is Any Preliminary Code Present?

**No hints/quick-select code exists.** Exhaustive search results:

- `hint|quick.?select|label.*match|vimium` across all `*.rs` files: only hits for unrelated uses (font hinting `.hint(true)`, `size_hint()`, `UrgencyHints`, `bg_hint` in draw list, "layout hints" in cell flags). **Zero matches for terminal text hint/quick-select functionality.**
- No `oriterm/src/hints/` directory exists (glob returned empty).
- No `HintPattern`, `HintAction`, `HintMode`, or similar types anywhere.

**Verdict:** Truly not started. No scaffold, no stubs, no dead code.

---

## 2. TODOs/FIXMEs Related to This Section's Domain

None. No TODOs or FIXMEs reference hints, quick select, labeled selection, or pattern-based text matching.

---

## 3. Infrastructure That Partially Covers This Section

### 3a. URL Detection (DIRECTLY REUSABLE)

**Location:** `oriterm/src/url_detect/mod.rs`

This module implements regex-based URL detection on terminal grid content -- exactly the same pattern-matching approach hints mode needs:

- `URL_RE` static regex: `(?:https?|ftp|file)://[^\s<>\[\]'"]+`
- `UrlDetectCache` with logical-line scanning and caching
- `detect_urls_in_snapshot_lines()` -- scans viewport rows, runs regex, builds column-mapped segments
- `DetectedUrl` with `segments: Vec<(abs_row, start_col, end_col)>` and `url: String`
- `trim_url_trailing()` -- trailing punctuation trimming with balanced parentheses
- `extract_snapshot_row_text()` -- text extraction with column mapping (handles wide chars, zero-width)
- Logical line detection (wrapped row sequences)

This is almost exactly the scanning engine hints mode needs. The difference: URL detection uses a single hardcoded regex, while hints mode needs N configurable regex patterns. The scanning, text extraction, column mapping, and segment-building logic can be directly reused or generalized.

### 3b. URL Opening (DIRECTLY REUSABLE)

**Location:** `oriterm/src/platform/url/mod.rs`

- `open_url()` with scheme validation against `ALLOWED_SCHEMES`
- Cross-platform: `ShellExecuteW` (Windows), `xdg-open` (Linux), `open` (macOS)
- Scheme allowlist: `http://`, `https://`, `ftp://`, `file://`, `mailto:`

This is the implementation for the `HintAction::Open` action.

### 3c. Clipboard Operations (DIRECTLY REUSABLE)

**Location:** `oriterm/src/app/clipboard_ops/mod.rs`, `oriterm/src/clipboard/`

- `clipboard.store(ClipboardType::Clipboard, &text)` -- for `HintAction::Copy`
- `write_paste_to_pty()` -- for `HintAction::CopyAndPaste`
- Bracketed paste support for safe PTY writes

### 3d. Selection Model (DIRECTLY REUSABLE)

**Location:** `oriterm_core/src/selection/`

For `HintAction::Select` -- creating a selection over the matched region.

### 3e. Configuration Infrastructure

**Location:** `oriterm/src/config/`

TOML config parsing with serde exists. Custom pattern configuration (`[[hints.pattern]]`) can use the existing deserialization framework.

---

## 4. Gap Analysis

### Plan Strengths
- Clear separation of concerns: Pattern Registry, Label Assignment, Hint Actions
- Good built-in pattern set (URL, path, git hash, IP, email, number)
- URL template substitution for custom integrations (JIRA, GitHub)
- Progressive filtering (Vimium-style type-to-narrow)
- Distance-based label priority (nearest matches get shortest labels)
- Configurable alphabet

### Plan Gaps and Issues

**G1: URL Detection Overlap Not Addressed.**
The plan proposes `oriterm/src/hints/patterns.rs` with a new URL regex. The existing `oriterm/src/url_detect/mod.rs` already has a URL regex and a complete scanning infrastructure. The plan should specify whether hints mode generalizes/replaces the URL detection module, or delegates to it. Duplicating the scanning logic would be a maintenance burden.

**G2: Rendering Approach Underspecified.**
The plan says "Overlay label text at the start of each match" and "Dim non-matching terminal content." This is a significant rendering change. It needs to integrate with the existing GPU rendering pipeline:
- Label text rendering needs the font pipeline (shaping, atlas, GPU text draw)
- Dimming needs to modify cell opacity or apply a post-processing pass
- Labels overlay the terminal grid content -- this is conceptually an overlay layer (compositor)

The plan doesn't specify which GPU pipeline or rendering layer handles this. Options: (a) modify the extract/prepare phase to substitute label glyphs, (b) use the compositor overlay system (Section 43, now complete), (c) add a dedicated hints render pass.

**G3: No Line-Length Cap for Regex Matching.**
Section 45 (Security Hardening) explicitly calls out WezTerm #714 -- hyperlink regex DoS on long lines (1.5MB single-line JSON, 3.8GB memory, 100% CPU). The hints pattern registry will run N regexes against terminal content. There is no mention of a line-length cap or bounded regex execution. This is a security concern that should be built into the pattern matching from the start.

**G4: Crate Placement.**
Plan says "Crate: `oriterm`." This is correct -- hints mode needs terminal grid access, clipboard, URL opening, and GPU rendering, all of which live in `oriterm`.

**G5: Dependencies: Section 14 (URL Detection) Listed as "Not Started" but Code Exists.**
Section 14 is listed as "Not Started" in the roadmap index, but `oriterm/src/url_detect/mod.rs` with 414 lines of production code and tests already exists and is wired into cursor hover and Ctrl+click. The plan's dependency on "Section 14 complete" is technically unmet per the roadmap, but the actual code is present and functional.

**G6: No Integration with Vi Mode (Section 40).**
In Alacritty, hints mode and vi mode are separate but related -- you can activate hints from vi mode. The plan doesn't mention any interaction with Section 40. Whether this is intentional or an oversight should be clarified.

**G7: Performance Concern -- N Regexes on Every Activation.**
Running 6+ regex patterns across all visible rows on each hints activation could be slow on large viewports (e.g., 250+ row terminals). The plan should mention caching strategy or performance bounds.

---

## 5. Dependency Status

| Dependency | Roadmap Status | Actual Code Status |
|---|---|---|
| Section 14 (URL Detection) | Not Started | `url_detect/mod.rs` fully implemented, wired into hover + Ctrl+click |
| Section 08 (Keyboard Input) | Not Started | Keyboard dispatch and keybinding table exist |
| Section 06 (Text Rendering) | Complete | Font pipeline, UI text shaping, atlas all working |
