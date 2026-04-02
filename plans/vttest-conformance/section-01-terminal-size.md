---
section: "01"
title: "Terminal Size Reporting"
status: in-progress
reviewed: true
goal: "vttest sees the correct terminal size at all dimensions, not hardcoded 80x24"
inspired_by:
  - "WezTerm text_area_size_chars (wezterm/term/src/terminalstate/mod.rs)"
  - "Alacritty window_report (alacritty_terminal/src/term/mod.rs)"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "01.1"
    title: "Diagnose the 80-Column Bug"
    status: complete
  - id: "01.2"
    title: "Fix Terminal Size Reporting"
    status: complete
  - id: "01.3"
    title: "Update Golden References"
    status: complete
  - id: "01.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "01.N"
    title: "Completion Checklist"
    status: in-progress
---

# Section 01: Terminal Size Reporting

**Status:** Not Started
**Goal:** vttest receives the correct terminal dimensions via CSI 18t and uses them for all drawing operations. `vttest_border_fills_*` passes at 80x24, 97x33, and 120x40.

**Context:** vttest's first action after launch is querying the terminal size via DA (device attributes) and CSI 18t (text area size in characters). The border test (menu 1, screen 01) draws a `*`/`+` border that should fill the entire terminal. Currently, the border fills only 80 columns regardless of actual PTY size. This blocks all non-80-column testing.

**Reference implementations:**
- **WezTerm** `term/src/terminalstate/mod.rs`: responds to CSI 18t with actual grid dimensions.
- **Alacritty** `alacritty_terminal/src/term/mod.rs`: `report_text_area_size()` sends `CSI 8;lines;cols t`.

**Depends on:** None (foundation section).

---

## 01.1 Diagnose the 80-Column Bug

**File(s):** `oriterm_core/src/term/handler/status.rs`, `oriterm_core/tests/vttest.rs`

The CSI 18t handler (`status_text_area_size_chars`) exists and reports `self.grid().lines()` and `self.grid().cols()`. The question is: why does vttest still see 80 columns?

Possible root causes (ordered by likelihood):
1. The DA1 response (`\x1b[?6;4c`) does not identify oriterm as a terminal that supports CSI 18t. vttest checks the DA response to determine the terminal's class (VT100/VT220/etc.) and falls back to 80x24 if the response doesn't indicate VT200+ capabilities. The current response claims attribute 6 (ANSI color) and 4 (sixel), but doesn't include the VT200 identification prefix. Note: the comment at `status.rs:61` says "DA1: report VT220" but the response `\x1b[?6;4c` is NOT VT220 format -- the comment is wrong. vttest source `main.c` expects `CSI ? 62;...c` (or higher) to indicate VT200+ level, which enables CSI 18t queries. 62 = VT200, 63 = VT300, 64 = VT400.
2. vttest sends CSI 18t but the response format doesn't match expectations. The current implementation sends `\x1b[8;{lines};{cols}t` -- verify vttest parses this correctly (vttest expects this exact format).
3. vttest uses ioctl TIOCGWINSZ on the slave PTY fd as a fallback when CSI 18t doesn't respond. The PTY size is set at spawn time via `PtySize` -- verify this is correct.
4. The VTE parser doesn't route the specific CSI sequence vttest sends (unlikely -- CSI 18t is already handled).

- [x] Compare DA1 response (`\x1b[?6;4c` at `status.rs:65`) against what vttest expects -- confirmed: response lacked VT220+ class prefix (`62`/`63`/`64`), causing vttest to treat oriterm as VT100
- [x] Add `log::info!` to `status_text_area_size_chars` and `status_identify_terminal` to confirm whether they fire during vttest startup -- confirmed via hex tracing: vttest sends DA1 but NOT CSI 18t; vttest uses hardcoded 80x24 unless size passed as CLI arg
- [x] Verify `portable-pty` sets the correct PTY size at spawn time: open a 97x33 PTY, run `stty size` inside it, confirm it reports `33 97` -- `pty_size_is_propagated` test confirms PTY size is correct
- [x] Run vttest under `strace -e trace=ioctl,write,read` filtering for TIOCGWINSZ and CSI sequences to capture the exact size query path vttest takes -- traced via VTTEST_HEX: vttest sends DA1 + DECRQSS, no CSI 18t
- [x] Write a minimal reproduction: spawn a 97-column PTY, run `tput cols` inside it, verify it reports 97 -- verified via `pty_size_is_propagated` test
- [x] Check if the VtTestSession test harness's `drain()` loop is reliably flushing all DA/DSR responses back to the PTY before vttest proceeds -- fixed: replaced `try_recv()` with blocking `recv_timeout()` and added `wait_for()` for content-based waiting. Also discovered vttest sends DECRQSS (DCS $q "p ST) for DECSCL query that must be answered
- [x] Add unit test: `da1_response_format` -- `da1_response_indicates_vt220_class` test asserts DA1 contains VT220+ class prefix

---

## 01.2 Fix Terminal Size Reporting

**File(s):** Depends on diagnosis from 01.1

Based on the diagnosis, apply the fix. The most likely scenarios:

**(a) DA1 response needs VT220+ class identification** (most likely fix) -- vttest expects the DA1 response to indicate VT200+ class. The current response `\x1b[?6;4c` claims attributes 6 and 4, but doesn't indicate VT220 class (which would be `\x1b[?62;6;4c` -- note `62` = VT200 level). Change `status_identify_terminal()` at `status.rs:65` to respond with `\x1b[?62;6;4c` or `\x1b[?64;6;4c` (VT400 level, which xterm uses). Also fix the misleading comment at `status.rs:61` that says "DA1: report VT220" when the response is not actually VT220 format.

**(b) Missing CSI sequence handler** -- vttest may send a query oriterm's VTE parser doesn't dispatch. Add the handler.

**(c) PTY size correct but response timing issue** -- the `drain()` loop uses `try_recv()` which is non-blocking. If the DA1 response event is enqueued after `drain()` completes but before vttest reads, the handshake succeeds. But if vttest reads before the response is written back, vttest may not receive the reply. This is a race condition in the test infrastructure.

- [x] Implement the fix identified in 01.1 -- updated DA1 response from `\x1b[?6;4c` to `\x1b[?64;6;4c` (VT420 class). Also implemented DECRQSS handler (DCS $q ST) for DECSCL conformance level query, responding with `DCS 1$r64;1"p ST` (VT400 level, 7-bit controls). Added DECRQSS dispatch to vendored vte crate. Also pass terminal size as CLI arg to vttest (`LINESxCOLS.COLS`) since vttest hardcodes 80x24 defaults.
- [x] Add unit test: create a `Term` at 97x33, trigger CSI 18t, verify response is `\x1b[8;33;97t` -- `csi_18t_at_97x33` test
- [x] Add unit test: verify DA1 response starts with `\x1b[?62;` or `\x1b[?64;` to indicate VT200+ class -- `da1_response_indicates_vt220_class` test
- [x] Add unit test: verify DA2 response format (`\x1b[>0;{version};1c`) matches vttest expectations -- `da2_produces_secondary_device_attributes` test (pre-existing)
- [x] Add unit test: `csi_18t_at_non_80_cols` -- `csi_18t_at_non_80_cols` test creates 120x40 Term, verifies `\x1b[8;40;120t`
- [x] **Post-DA1 fix smoke test**: ran all vttest tests after DA1 change. Discovered two issues: (1) vttest sends DECRQSS for DECSCL when terminal reports VT420 — implemented DECRQSS handler, (2) vttest hardcodes 80x24 unless size passed as CLI arg — updated VtTestSession to pass `{rows}x{cols}.{cols}`. All 13 tests pass including origin mode at all sizes.
- [x] Verify the fix does NOT break any existing tests: all `oriterm_core` tests pass, clippy clean, build green

---

## 01.3 Update Golden References

**File(s):** `oriterm_core/tests/vttest.rs`, `oriterm/src/gpu/visual_regression/vttest.rs`

After the fix, the border test output changes at non-80-column sizes. Update all golden references.

- [x] Run `INSTA_UPDATE=always cargo test -p oriterm_core --test vttest` to regenerate text snapshots -- all snapshots updated for VT420 menu output (includes proper-sized screens at 97x33 and 120x40)
- [x] Run `ORITERM_UPDATE_GOLDEN=1 cargo test -p oriterm --features gpu-tests vttest_golden` to regenerate PNGs -- all 6 golden tests pass
- [x] Verify structural assertions pass: `cargo test -p oriterm_core --test vttest vttest_border_fills` -- all 3 sizes pass
- [x] Verify all 3 sizes: 80x24 (PASS), 97x33 (PASS), 120x40 (PASS)

---

## 01.R Third Party Review Findings

- None.

---

## 01.N Completion Checklist

- [x] `vttest_border_fills_80x24` passes
- [x] `vttest_border_fills_97x33` passes
- [x] `vttest_border_fills_120x40` passes
- [x] DA1 response verified against vttest expectations
- [x] CSI 18t response verified at non-80-column sizes
- [x] All text snapshots regenerated and committed
- [x] All golden PNGs regenerated and committed
- [x] `./build-all.sh` green
- [x] `./clippy-all.sh` green
- [x] `./test-all.sh` green
- [ ] `/tpr-review` passed

**Exit Criteria:** `vttest_border_fills_*` passes at all 3 terminal sizes. The vttest border screen renders identically to xterm's output — `*`/`+` border filling the entire terminal area with no gaps.
