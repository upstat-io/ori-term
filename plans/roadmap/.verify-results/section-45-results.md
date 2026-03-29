# Section 45: Security Hardening -- Verification Results

**Verified:** 2026-03-29
**Status:** CONFIRMED NOT STARTED (for the section as a whole), but significant infrastructure already exists
**Reviewed:** false (unreviewed gate)

---

## 1. Code Search: Is Any Preliminary Code Present?

**Multiple security-relevant systems already exist in production code.** Unlike sections 40-42, this section describes hardening of existing features rather than building new ones. Detailed findings:

### Already Implemented

**OSC 52 Clipboard Handling:**
- `oriterm_core/src/term/handler/osc.rs:113-159` -- `osc_clipboard_store()` and `osc_clipboard_load()` fully implemented
- Handles `c` (clipboard), `p`/`s` (selection) selectors
- Base64 decode with error handling (invalid base64 and invalid UTF-8 gracefully logged and ignored)
- Extensive tests: 20+ test functions in `oriterm_core/src/term/handler/tests.rs` covering store, load, edge cases, multi-selector, empty payload, large payload, invalid base64, truncated base64

**BUT: No focus gating, no policy checking, no size limit.** The `osc_clipboard_store()` method unconditionally emits `Event::ClipboardStore`. There is no `is_focused` check, no `clipboard_write` policy, no `clipboard_max_size` enforcement. This is exactly the vulnerability described in WT #19051.

**Paste Safety:**
- `oriterm_core/src/paste/mod.rs` -- Full paste processing pipeline:
  - `filter_paste()` -- smart quote/dash normalization
  - `normalize_line_endings()` -- CRLF/LF to CR (handles both `\r` and `\n` -- addresses WT #8601)
  - `strip_escape_chars()` -- ESC character stripping in bracketed paste (defense against escape injection)
  - `count_newlines()` -- counts both `\r` and `\n`, CRLF as one (addresses WT #8601)
  - `prepare_paste()` -- full pipeline: filter -> normalize -> strip ESC -> bracket wrap
  - `format_dropped_paths()` -- file path formatting for drag-drop
- `oriterm/src/app/clipboard_ops/mod.rs` -- Paste warning system:
  - `PasteWarning` enum: `Never`, `Always`, `Threshold(u32)`
  - Bracketed paste detection via `TermMode::BRACKETED_PASTE`
  - Confirmation dialog via `show_paste_confirmation()` sending `TermEvent::OpenConfirmation`
  - Multi-line paste warning when bracketed paste is off (Ghostty pattern)
- Tests in `oriterm_core/src/paste/tests.rs`:
  - Bracketed paste end marker (`\x1b[201~`) stripping verified
  - OSC/CSI injection defense: ESC stripped in bracketed paste
  - Newline counting for `\r`, `\n`, and `\r\n`

**BUT: Missing features per the plan:**
- No "dangerous content detection" (`sudo`, `rm -rf`, `curl | sh`)
- No homograph attack detection (mixed-script URLs)
- No C0/C1 control character filtering in paste (Ghostty 1.3.0 pattern)
- No bracket-end injection detection as a separate explicit check (ESC stripping covers the `\x1b` byte, but the plan wants explicit `\x1b[201~` sequence detection as a named check)
- `format_dropped_paths()` does NOT shell-escape paths -- only wraps spaces in double quotes, does not handle single quotes (WT #18006 vector: `John's Archive`), does not escape special shell characters

**Title Stack Depth Limit:**
- `oriterm_core/src/term/mod.rs:80` -- `TITLE_STACK_MAX_DEPTH = 4096` (matches Alacritty)
- `osc_push_title()` in `osc.rs:56-59` -- drops oldest when at capacity
- Tests verify the 4096 cap: `osc_title_stack_cap_at_4096()` in handler tests

**BUT: No title sanitization.** `osc_set_title()` passes title strings through unsanitized -- no C0/C1 character stripping. This is the same gap Alacritty has and that Windows Terminal explicitly fixed (WT #12206, #10312).

**URL Scheme Validation:**
- `oriterm/src/platform/url/mod.rs` -- `validate_scheme()` checks against `ALLOWED_SCHEMES = ["http://", "https://", "ftp://", "file://", "mailto:"]`
- Blocks disallowed schemes before `platform_open()`

**Link Click Modifier:**
- `oriterm/src/app/mouse_input.rs:364-365` -- Ctrl+click required to open URL (not bare click)
- `oriterm/src/app/cursor_hover.rs:143-158` -- `try_open_hovered_url()` only called on Ctrl+click path

**Sixel Resource Limits:**
- `oriterm_core/src/image/sixel/mod.rs:17-20`:
  - `MAX_DIMENSION = 10_000` (max width or height)
  - `MAX_PIXEL_BYTES = 100_000_000` (100MB, matches WezTerm's fix for CVE-2022-24130)
- Repeat count clamped: `count.min(MAX_DIMENSION)` at line 340
- Raster attribute validation: `w > MAX_DIMENSION || h > MAX_DIMENSION` check, then `w * h * 4 > MAX_PIXEL_BYTES` with `checked_mul` (saturating arithmetic)
- Buffer growth capped: `grow_buffer()` uses `checked_mul` and caps at `MAX_PIXEL_BYTES`
- `aborted` flag prevents further processing after limit hit

**Image Cache Limits:**
- `oriterm_core/src/image/cache/mod.rs`:
  - `DEFAULT_MEMORY_LIMIT = 320 * 1024 * 1024` (320 MiB, matches Ghostty)
  - `DEFAULT_MAX_SINGLE_IMAGE = 64 * 1024 * 1024` (64 MiB)
  - `set_memory_limit()` and `set_max_single_image()` configurable
  - LRU eviction when memory exceeded
  - Separate caches for primary and alternate screens (via `alt_image_cache` in `Term`)

---

## 2. TODOs/FIXMEs Related to This Section's Domain

No security-related TODOs in the codebase. However, the plan itself contains a known bug:

**Plan line 137:** "BUG: Paste confirmation dialog does not trigger -- even with `warn_on_paste: Always` (default) and cmd.exe (no bracketed paste mode), pasting multiline text does not open the confirmation dialog. Discovered during chrome plan verification (2026-03-10)."

This is a pre-existing bug documented in the plan rather than in the codebase.

---

## 3. Infrastructure Coverage Summary

| Plan Item | Already Implemented? | Coverage Level |
|---|---|---|
| 45.1: OSC 52 store/load | Yes (handler) | Partial -- no focus gate, no policy, no size limit |
| 45.1: Focus-gated clipboard | No | winit `WindowEvent::Focused` available, not wired to OSC 52 |
| 45.1: Clipboard policy config | No | No `[security]` config section exists |
| 45.1: Confirmation dialog | No | Dialog infrastructure exists (paste confirmation), needs OSC 52 variant |
| 45.2: Multi-line paste warning | Yes | Working -- `PasteWarning` enum, confirmation dialog |
| 45.2: `\r` and `\n` detection | Yes | `count_newlines()` handles both |
| 45.2: Line ending normalization | Yes | `normalize_line_endings()` handles CRLF, LF, CR |
| 45.2: ESC stripping in paste | Yes | `strip_escape_chars()` in bracketed paste |
| 45.2: Bracket-end injection | Partial | ESC stripping removes `\x1b` so `\x1b[201~` is broken, but no explicit named check |
| 45.2: Dangerous content detection | No | Not implemented |
| 45.2: Homograph attack detection | No | Not implemented |
| 45.2: C0/C1 filtering in paste | No | Not implemented (Ghostty 1.3.0 pattern) |
| 45.2: Input routing during UI focus | Not verified | Would need testing -- the event routing may already handle this |
| 45.3: Title stack cap | Yes | 4096, tested |
| 45.3: Title sanitization | No | Titles passed through unsanitized |
| 45.3: Title reporting (CSI 21 t) | Not verified | Need to check if implemented and whether it's gated |
| 45.3: Window manipulation restriction | Not verified | Need to check CSI t handling |
| 45.3: Sixel validation | Yes | MAX_DIMENSION, MAX_PIXEL_BYTES, checked_mul, abort flag |
| 45.3: Image storage limits | Yes | 320 MiB default, per-screen, configurable, LRU eviction |
| 45.3: Hyperlink regex line-length cap | No | `url_detect/mod.rs` runs regex on all lines without length cap |
| 45.3: Notification focus-gating | No | Not implemented |
| 45.3: Child process env hygiene | Not verified | Need to check umask/env handling in mux spawn |
| 45.3: VT parser robustness | Partial | vendored `vte` crate handles parsing, but no fuzz tests |
| 45.4: Drag-drop shell escaping | No | `format_dropped_paths()` only double-quotes spaces, no shell escaping |
| 45.4: Link click modifier | Yes | Ctrl+click required |
| 45.4: URI scheme allowlist | Yes | `ALLOWED_SCHEMES` validated before `open_url()` |

---

## 4. Gap Analysis

### Plan Strengths
- Extremely thorough reference material -- cites 15+ real CVEs and security bugs from Windows Terminal, Alacritty, WezTerm, and Ghostty with PR numbers
- Defense-in-depth layering (focus gate + policy + confirmation + content filtering)
- Correct identification of the `\r`-only multiline bypass (WT #8601)
- Correct identification of bracket-end injection (Ghostty pattern)
- Correct identification of the C1 control character gap in title sanitization (WT #10312)
- Per-screen image storage limits (Ghostty pattern)
- Homograph attack detection (Ghostty #10762)
- Comprehensive drag-drop path safety (WT #19015, #18006)

### Plan Gaps and Issues

**G1: Existing ESC Stripping Already Mitigates Bracket-End Injection.**
The plan specifies explicit `\x1b[201~` detection in paste data as a separate check. The existing `strip_escape_chars()` already removes all `\x1b` bytes from bracketed pastes, which breaks `\x1b[201~` by removing the ESC prefix. The plan should acknowledge this existing mitigation and clarify whether the explicit check is for the non-bracketed path or for defense-in-depth logging.

**G2: Drag-Drop Path Escaping Is a Real Vulnerability NOW.**
`format_dropped_paths()` in `oriterm_core/src/paste/mod.rs` only wraps spaces in double quotes. The test `format_path_with_quotes` explicitly comments: "Paths containing double quotes are not double-escaped -- they pass through. Shell interpretation is the user's responsibility." This is the exact WT #18006 vulnerability: a file named `John's Archive` would break POSIX shell quoting. This is a pre-existing security issue, not a future hardening item.

**G3: No `[security]` Config Section Exists.**
The plan proposes a `[security]` TOML config section with `clipboard_write`, `clipboard_read`, `title_report`, `allow_window_ops`, etc. No such config section exists yet. Section 13 (Configuration) is "Not Started," so adding new config sections depends on that work.

**G4: Cross-Crate Implementation.**
The plan specifies work across three crates: `oriterm_core` (policy checks in handler), `oriterm` (focus state, confirmation UI), and `oriterm_mux` (event filtering). This is architecturally correct but complex to coordinate. The plan should specify the order of changes and how policy state flows between crates.

**G5: Hyperlink Regex Line-Length Cap Is a Real Vulnerability NOW.**
The `url_detect/mod.rs` regex runs without any line-length cap. A malicious program writing a 1.5MB single-line JSON string would cause unbounded regex processing (WezTerm #714). This is a pre-existing performance/DoS issue.

**G6: Paste Confirmation Bug Documented But Not Tracked.**
The plan contains a known bug (paste confirmation dialog not triggering -- line 137) but this bug isn't tracked as a TODO in the code. It should be filed as a tracked issue.

**G7: VT Parser Fuzz Testing Scope.**
The plan mentions "Fuzz test the VT parser with: split sequences at every byte boundary." The VT parser is a vendored crate (`crates/vte`). Fuzz testing a vendored dependency requires a separate test harness. The plan should specify where these fuzz tests live and how they integrate with CI.

**G8: OSC 50 (Font Change) Not Mentioned in Code Search.**
The plan mentions denying OSC 50 (font change) by default. Need to verify whether OSC 50 is even implemented in the VTE handler -- if not, it's already effectively denied.

---

## 5. Pre-Existing Security Issues Found During Audit

These are issues that exist in the current codebase and should be addressed regardless of this section's timeline:

1. **No OSC 52 focus gating** -- background panes can write to system clipboard (WT #19051 vector)
2. **No title sanitization** -- C0/C1 control characters pass through in `osc_set_title()` (WT #12206, #10312 vector)
3. **No drag-drop shell escaping** -- `format_dropped_paths()` doesn't escape single quotes or special chars (WT #18006 vector)
4. **No URL detection line-length cap** -- unbounded regex on long lines (WezTerm #714 vector)

---

## 6. Dependency Status

| Dependency | Roadmap Status | Actual Code Status |
|---|---|---|
| Section 09 (Selection & Clipboard) | Not Started | Clipboard ops and paste processing exist |
| Section 21 (Context Menu) | In Progress | Dialog infrastructure exists for confirmation dialogs |
| Section 13 (Configuration) | Not Started | Config system exists, but no `[security]` section |
