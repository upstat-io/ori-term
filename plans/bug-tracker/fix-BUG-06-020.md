---
bug: "BUG-06-020"
title: "notcurses-demo bugs 016/018/019 cluster: identify which oriterm capability-probe reply flips notcurses into the non-Z-order-respecting render path"
severity: "medium"
status: in-progress
goal: "Produce an authoritative byte-stream diff identifying the divergence point between oriterm and WezTerm in notcurses' capability-detection path, plus a routed close-out decision."
success_criteria:
  - "Reply stream from oriterm to the captured notcurses-demo startup handshake is enumerated probe-by-probe with status (OK / missing / unrecognized) per item."
  - "Divergence chain is traced through notcurses' source (`termdesc.c` + `in.c`) and verified line-by-line against the actual code."
  - "Close-out decision routes follow-up implementation work to its canonical home (Section 38.4 for XTGETTCAP, Section 39.10 for XTSMGRAPHICS) and files concrete tracker entries for any orphaned gaps (XTVERSION SSOT LEAK, notcurses-upstream petition)."
  - "Cluster bugs BUG-06-016/018/019 carry a back-reference to this diagnostic so the cluster's collateral close path is traceable."
subsystem: "oriterm_core/src/term/handler/ + oriterm_mux/src/shell_integration/interceptor.rs (capability-probe reply path)"
found: "2026-04-24"
source: "BUG-06-018 trace investigation"
third_party_review:
  status: none
  updated: null
---

# Fix: BUG-06-020 — notcurses capability-probe diagnostic

**Status:** In Progress
**Severity:** medium
**Goal:** Produce an authoritative byte-stream diff identifying the divergence point between oriterm and WezTerm in notcurses' capability-detection path, plus a routed close-out decision. The deliverable is documentation, not code — implementation work routes to existing plan sections (§38.4 XTGETTCAP, §39.10 XTSMGRAPHICS) and follow-up bug-tracker entries.

**Success Criteria:**
- [x] Reply stream from oriterm to the captured notcurses-demo startup handshake is enumerated probe-by-probe with status (OK / missing / unrecognized) per item.
- [x] Divergence chain is traced through notcurses' source (`termdesc.c` + `in.c`) and verified line-by-line against the actual code.
- [x] Close-out decision routes follow-up implementation work to its canonical home (Section 38.4 for XTGETTCAP, Section 39.10 for XTSMGRAPHICS) and files concrete tracker entries for any orphaned gaps.
- [x] Cluster bugs BUG-06-016/018/019 carry a back-reference to this diagnostic via the bug-tracker entry close-out note.

**Context:** BUG-06-016 (mojibake — OBE), BUG-06-018 (whiteout cells — closed not-a-render-bug), BUG-06-019 (xray marquee uncolored — closed not-a-render-bug) showed that oriterm faithfully renders the byte stream notcurses sends, but the byte stream itself contains the defects. WezTerm and Windows Terminal render correctly because notcurses sends them DIFFERENT bytes based on runtime capability detection. This bug is the diagnostic to identify which capability-probe reply diverges.

---

## 1. Root Cause Analysis

### Symptom

notcurses-demo renders correctly on WezTerm/Windows Terminal but produces visible defects on oriterm (whiteout cells destroyed, xray marquee uncolored, mojibake black strip). The PUT-trace shows oriterm faithfully rendering whatever notcurses sends; the divergence is in the byte stream, not the renderer.

### Investigation method

The pilot test `notcurses_startup_emits_at_least_one_pty_reply` at `oriterm_core/tests/spec_chain/pilots/notcurses_startup.rs:74-100` replays the captured byte stream at `plans/spec-conformance/captures/notcurses-demo-intro.cap` (2877 bytes) through `SpecHarness` and dumps every PTY-write effect oriterm emits in response. Output captured via `cargo test -p oriterm_core --test spec_chain notcurses_startup_emits -- --nocapture`.

### Authoritative reply stream — oriterm → notcurses

| # | notcurses probe | oriterm reply | Status |
|---|---|---|---|
| 1 | DSR CPR `\x1b[6n` | `\x1b[1;1R` | OK |
| 2 | DA3 `\x1b[=c` | `\x1bP!|00000000\x1b\\` | OK as-spec; unit-id `00000000` matches xterm default — innocuous but indistinguishable from xterm. |
| 3 | DA2 `\x1b[>c` | `\x1b[>0;200;1c` | OK |
| 4 | XTVERSION `\x1b[>0q` | Production: `\x1bP>|oriterm(<v>)\x1b\\` via `oriterm_mux/src/shell_integration/interceptor.rs:59-67`. (Pilot harness skips the interceptor; XTVERSION is silent in the test output.) | **UNRECOGNIZED** — `oriterm(` is not in notcurses' XTVERSION vendor prefix table. |
| 5 | XTGETTCAP `\x1bP+q544e;524742;687061\x1b\\` (TN/RGB/hpa) | **NO REPLY** — VTE parser has no DCS `+ q` dispatch and no handler. | **MISSING** |
| 6 | OSC 4 ; 0..255 ; ? (256 palette queries) | `HostRequest::ColorQuery` async — GUI resolves in production; never resolved in pilot harness. | OK in production |
| 7 | OSC 10/11 (default fg/bg) | `HostRequest::ColorQuery` async | OK in production |
| 8 | DECRQM `?2026 $p` | `\x1b[?2026;2$y` | OK (recognized, currently reset) |
| 9 | DECRQM `?1016 $p` | `\x1b[?1016;0$y` | OK as-spec (mode 1016 not implemented; `0` correctly signals unrecognized) |
| 10 | XTSMGRAPHICS `\x1b[?1;3;256S` / `\x1b[?2;1;0S` / `\x1b[?1;1;0S` | **NO REPLY** | **MISSING** |
| 11 | Kitty kbd query `\x1b[?u` | `\x1b[?11u` | OK |
| 12 | Kitty graphics query `\x1b_Gi=1,a=q;\x1b\\` | `\x1b_Gi=1;OK\x1b\\` | OK (sets `kitty_graphics=1` in notcurses) |
| 13 | CSI 14t (pixel size) | `\x1b[4;384;640t` | OK |
| 14 | CSI 18t (char size) | `\x1b[8;24;80t` | OK |
| 15 | DA1 `\x1b[c` | `\x1b[?64;6;4c` (VT420 + selective erase + sixel) | OK |

### Divergence chain (verified against `~/projects/reference_repos/console_repos/notcurses/src/lib/`)

**WezTerm path:**
1. notcurses sends XTVERSION (`\x1b[>0q`); WezTerm replies `\x1bP>|WezTerm <version>\x1b\\`.
2. `xtversion_cb` at `in.c:1540-1581` matches `WezTerm ` prefix in the `xtvers[]` table (`in.c:1549-1565`); sets `qterm = TERMINAL_WEZTERM`.
3. `apply_term_heuristics` at `termdesc.c:973-1054` switches on `qterm`; dispatches to `apply_wezterm_heuristics` (`termdesc.c:830-840`) which sets `caps.rgb = true; caps.quadrants = true; caps.sextants = true (if version >= 2021-06-10); add_smulx_escapes(...)`.
4. Kitty graphics path is INDEPENDENT of `qterm` — `kitty_graphics` flag is set by the kitty query callback (`in.c:1408-1414`), `setup_kitty_bitmaps` is gated at `termdesc.c:1527-1533` by that flag alone (regardless of which `apply_*_heuristics` ran).

**oriterm path:**
1. notcurses sends XTVERSION; oriterm replies `\x1bP>|oriterm(<v>)\x1b\\` (production, via `oriterm_mux/src/shell_integration/interceptor.rs:59-67`).
2. `xtversion_cb` scans `xtvers[]` (`in.c:1549-1565`) — `oriterm(` is NOT in the list (`XTerm(`, `WezTerm `, `contour `, `kitty(`, `foot(`, `ghostty `, `mlterm(`, `tmux `, `iTerm2 `, `mintty `, `terminology `). `qterm` stays `TERMINAL_UNKNOWN`.
3. notcurses sends XTGETTCAP (`\x1bP+q544e;524742;687061\x1b\\`) for TN/RGB/hpa keys; oriterm sends NO REPLY (no parser dispatch, no handler). `tcap_cb` at `in.c:1701-1740` would have been the secondary identification channel:
   - `TN` mapping (`in.c:1717-1731`): only recognizes `xterm`, `mlterm`, `xterm-kitty`, `xterm-ghostty`, `xterm-256color`. Even if oriterm implemented XTGETTCAP TN reply with value `oriterm`, notcurses would log "unknown terminal name oriterm" and leave `qterm = TERMINAL_UNKNOWN`.
   - `RGB` capability (`in.c:1733`): if present, sets `rgb = true` INDEPENDENT of TN matching.
4. `apply_term_heuristics` falls through to the `default` arm (`termdesc.c:1054-1056`): `logwarn("no match for qterm %d tname %s", qterm, tname); newname = tname;`. Only `caps.braille = true; caps.halfblocks = true` are set (`termdesc.c:990-991` — universal defaults applied before the switch). NO `caps.rgb`, NO `caps.quadrants`, NO `caps.sextants`, NO `add_smulx_escapes`.
5. Kitty graphics: `kitty_graphics=1` via the kitty query callback. `setup_kitty_bitmaps` runs the same as for WezTerm.

### Root cause

oriterm has NO recognized identity channel for notcurses. Three potential channels exist; all three fail:
- **XTVERSION**: oriterm replies but with `oriterm(` vendor prefix that is not in notcurses' `xtvers[]` table at `in.c:1549-1565`.
- **XTGETTCAP TN**: oriterm doesn't reply (no parser/handler). Even if implemented, notcurses' TN→qterm map at `in.c:1717-1731` doesn't recognize `oriterm`.
- **XTGETTCAP RGB**: oriterm doesn't reply. If implemented (and we send `RGB` capability), would recover `rgb = true` independent of TN matching.

The cluster defects (BUG-06-016/018/019) trace to the missing capability flags (`caps.rgb`, `caps.quadrants`, `caps.sextants`, smulx) that `apply_*_heuristics` would have set if `qterm` were recognized. Without `caps.rgb`, notcurses falls back to ANSI 256-color rendering for RGB-required overlays. Without `caps.quadrants`/`caps.sextants`, high-density Unicode block rendering falls back to half-blocks. Without smulx, advanced underline styling fails. The cluster's visible artifacts (whiteout cells destroyed, marquee uncolored, mojibake) are downstream of these missing capability flags, not the kitty-graphics tuning (which is identity-independent).

### Blast radius

- **Direct**: BUG-06-016 (mojibake — already OBE per cross-terminal cross-check), BUG-06-018 (whiteout cells — closed not-a-render-bug), BUG-06-019 (xray marquee uncolored — closed not-a-render-bug). Cluster collateral fix path: implementing XTGETTCAP per Section 38.4 with `RGB` capability key would recover at least RGB-required rendering. Full mitigation requires notcurses upstream adding `oriterm(` to its XTVERSION vendor table OR `oriterm` to its XTGETTCAP TN map.
- **Indirect**: any other notcurses-based application running on oriterm sees the same `qterm = TERMINAL_UNKNOWN` fallback. The blast radius is "every TUI tool that uses notcurses to probe terminal capabilities."
- **Architectural**: XTVERSION reply currently lives in `oriterm_mux/src/shell_integration/interceptor.rs:59-67` (not `oriterm_core` where DA1/DA2/DA3 live). This is an SSOT LEAK — terminal identification belongs in `oriterm_core`. The current location is a "raw VTE interceptor" pattern justified by `vte::ansi::Processor` not routing `CSI > q` to the `Handler` trait. Long-term fix routes through `crates/vte` patching to add the route, then moving the handler to `oriterm_core`.

### Affected files (diagnostic — no production changes in this bug)

- `oriterm_core/tests/spec_chain/pilots/notcurses_startup.rs` — pilot test (existing); replayed during investigation.
- `plans/spec-conformance/captures/notcurses-demo-intro.cap` — captured byte stream (existing); 2877 bytes covering DSR/DA1/DA2/DA3/XTVERSION/XTGETTCAP/OSC/DECRQM/Kitty/CSI 14t/18t.
- `oriterm_mux/src/shell_integration/interceptor.rs:59-67` — XTVERSION reply implementation (existing); sole producer of the `oriterm(` vendor prefix string.
- `oriterm_core/src/term/handler/status.rs:136-169` — DA1/DA2/DA3 reply implementations (existing); reference for where XTVERSION/XTGETTCAP would naturally live if the VTE parser routed them to `Handler`.

### Reference implementations

- **notcurses** (~/projects/reference_repos/console_repos/notcurses/):
  - `src/lib/termdesc.c:476-561` — `send_initial_directives` / `send_initial_queries`: emits the IDQUERIES batch (TRIDEVATTR/XTVERSION/XTGETTCAP/SECDEVATTR) plus DECRQM/XTSMGRAPHICS/KITTYQUERY/PRIDEVATTR.
  - `src/lib/termdesc.c:973-1054` — `apply_term_heuristics`: switch on `qterm`; default arm sets only `caps.braille=true, caps.halfblocks=true`.
  - `src/lib/termdesc.c:830-840` — `apply_wezterm_heuristics`: sets `caps.rgb=true, caps.quadrants=true, caps.sextants=true (if recent), add_smulx_escapes`.
  - `src/lib/termdesc.c:1527-1533` — kitty graphics setup gate (independent of qterm, gated by `kitty_graphics` flag alone).
  - `src/lib/in.c:1540-1581` — `xtversion_cb`: matches XTVERSION reply against `xtvers[]` prefix table.
  - `src/lib/in.c:1549-1565` — `xtvers[]` prefix table (the recognized vendor list — `oriterm(` not present).
  - `src/lib/in.c:1701-1740` — `tcap_cb`: matches XTGETTCAP TN against terminal-name map.
  - `src/lib/in.c:1717-1731` — TN→qterm map (the recognized terminal-name list — `oriterm` not present).
  - `src/lib/in.c:1408-1414` — `kittygraph_cb`: sets `kitty_graphics=1` on any matching APC `_G` reply.

---

## 1.5 Fix Consensus (via /tp-help)

Independent dual-source design review of the proposed close-out path. Two rounds run because round 1's codex output went off-topic (returned working-tree findings unrelated to BUG-06-020); round 2 was a fresh codex query with explicit scope-narrowing.

- **Proposed approach (pre-consensus)**: Path A — close BUG-06-020 with the documented byte-stream diff + divergence chain as the deliverable; file XTSMGRAPHICS as a NEW Section 38.18 subsection; note Section 38.4 references this diagnostic; do NOT change XTVERSION format.
- **tp-help run scratch dirs**: `/tmp/tpr-round-ori_term-zoegeggA` (round 1), `/tmp/tpr-round-ori_term-j5HZNJXn` (round 2).

### Round 1
- **Codex summary**: OFF-TOPIC. Returned working-tree findings about `oriterm/src/gpu/prepare/dirty_skip/mod.rs` and `oriterm/src/gpu/prepare/unshaped.rs` (unrelated to BUG-06-020). Did NOT answer the BUG-06-020 close-out question.
- **Gemini summary**: Concurred with Path A. Added: XTVERSION's location is an SSOT LEAK; don't change DA3 or XTVERSION formatting; file XTSMGRAPHICS via /add-bug; prioritize Section 38.4 for cluster collateral fix.
- **Outcome**: Inconclusive — gemini-only signal, codex round needed to be re-run.

### Round 2 (codex re-query)
- **Codex summary**: Endorsed close-out as diagnostic but amended Path A on three points: (1) XTSMGRAPHICS already has an owning subsection at Section 39.10, NOT a new Section 38.18; (2) WezTerm is recognized via XTVERSION prefix matching (`in.c:1549-1565`), not XTGETTCAP TN; (3) `apply_wezterm_heuristics` does NOT set kitty graphics tuning — the kitty_graphics flag is set independently by the kitty query callback and gated separately at `termdesc.c:1527-1533`. The closure should say "no recognized identity channel, generic heuristics" rather than "no XTGETTCAP, no kitty graphics tuning."
- **Gemini summary (round 2 — content style suggests gemini despite mislabeled identity header)**: Approved Path A. XTGETTCAP is the definitive identity blocker. XTVERSION in interceptor.rs is `LEAK:layer-bleeding`; file separately. Use teseq/pilot tests to verify the DCS 1+r absence.
- **Independent code verification (Claude)**:
  - `plans/roadmap/section-39-image-protocols.md:34-36` — confirmed `id: "39.10" title: "XTSMGRAPHICS Sixel Size Negotiation" status: not-started`. ✅ codex correct.
  - `plans/roadmap/section-39-image-protocols.md:679-691` — confirmed XTSMGRAPHICS subsection covers `CSI ? 1 ; 1 ; 0 S` (geometry) and `CSI ? 2 ; 1 ; 0 S` (color count) with response format `CSI ? Pi ; 0 ; Pv S`. ✅ codex correct.
  - `~/projects/reference_repos/console_repos/notcurses/src/lib/in.c:1549-1565` — verified `xtvers[]` prefix table. `oriterm(` is NOT in the list. ✅ codex correct.
  - `~/projects/reference_repos/console_repos/notcurses/src/lib/in.c:1717-1731` — verified TN→qterm mapping. Maps `xterm`, `mlterm`, `xterm-kitty`, `xterm-ghostty`, `xterm-256color`. `oriterm` is NOT mapped. ✅ codex correct.
  - `~/projects/reference_repos/console_repos/notcurses/src/lib/termdesc.c:830-840` — verified `apply_wezterm_heuristics` sets `caps.rgb/quadrants/sextants/smulx`; does NOT touch kitty graphics setup. ✅ codex correct.
- **Outcome**: Persuaded divergence — codex's three corrections are all factually verified; the diagnostic's divergence chain wording was IMPRECISE in round 1 (correct conclusion, wrong intermediate steps).

### Final agreed approach

Close BUG-06-020 with the diagnostic finding as the deliverable, with these corrections relative to original Path A:

1. **Divergence chain wording**: "no recognized identity channel via either XTVERSION (vendor prefix `oriterm(` not in `xtvers[]`) or XTGETTCAP TN (which oriterm doesn't implement, AND `oriterm` is not in notcurses' TN→qterm map at `in.c:1717-1731` even if it did) → `qterm = TERMINAL_UNKNOWN` → `apply_term_heuristics` `default` arm sets only `caps.braille=true, caps.halfblocks=true`, omits `caps.rgb/quadrants/sextants/smulx` → cluster defects in capability-flag-dependent rendering (NOT in kitty graphics tuning, which is identity-independent)."
2. **Implementation path for cluster collateral**: prioritize Section 38.4 (XTGETTCAP) with the `RGB` capability key — this recovers `rgb = true` in notcurses INDEPENDENT of TN matching, providing partial mitigation. Full mitigation requires notcurses upstream adding `oriterm(` to its `xtvers[]` vendor list OR `oriterm` to its TN→qterm map (file as upstream petition).
3. **XTSMGRAPHICS handler**: file as bug-tracker entry that CROSS-LINKS to existing Section 39.10 (NOT a new Section 38.18). Section 39.10 is the canonical home; the bug-tracker entry exists for visibility into the open work.
4. **XTVERSION SSOT LEAK**: file as a separate bug. Terminal identification belongs in `oriterm_core`; the current `oriterm_mux/src/shell_integration/interceptor.rs:59-67` location is a `LEAK:layer-bleeding` justified by the VTE parser routing limitation. Long-term fix routes through `crates/vte` patching.
5. **DA3 unit-id**: leave as `00000000` (xterm default). Don't change to a distinctive `oriterm`-specific value — this would need notcurses upstream to add it to `tda_cb` recognition table at `in.c:1290-1300`. File as part of the upstream petition bug if pursued.
6. **No XTVERSION format change**: leave `oriterm(<v>)` reply intact. Don't impersonate XTerm/WezTerm/etc. (Path D — REJECTED for "NO SHORTCUTS — ARCHITECTURALLY CORRECT PATH ONLY").

---

## 2. TDD — Test Matrix

**N/A — diagnostic-only deliverable.** No production code change in this bug. The existing pilot test `notcurses_startup_emits_at_least_one_pty_reply` at `oriterm_core/tests/spec_chain/pilots/notcurses_startup.rs:74-100` IS the verification artifact for the diagnostic — replaying the captured byte stream and asserting at least one PTY reply emits is the regression guard for the reply-stream behavior documented in §1.

Matrix completeness (existing pilot tests already cover):
- [x] Captured byte stream replays correctly (`notcurses_startup_emits_at_least_one_pty_reply`).
- [x] Reply stream framing is well-formed (`notcurses_startup_reply_stream_has_no_bare_printable_bytes`).
- [x] No stray `q` bytes outside DCS framing (`notcurses_startup_reply_stream_contains_no_stray_q_bytes`).

The matrix tests for the IMPLEMENTATION work (XTGETTCAP responses, XTVERSION format change if pursued, XTSMGRAPHICS responses) live in their owning plan sections and follow-up bugs, not in this diagnostic.

---

## 2.5 Fix Plan TPR Findings

Plan TPR: Skipped — medium severity, capability-probe handler subsystem is NOT in the complexity-elevated subsystem list (GPU render pipeline, VTE/core grid, mux/IO thread, IPC transport, platform-specific cfg). `/tp-help` round 2 converged after refinement; no architectural risk surfaced that warrants adversarial plan review.

---

## 3. Implementation

This bug's "implementation" is producing the diagnostic artifact and routing follow-up work to its canonical homes.

- [x] **Diagnostic artifact**: this fix section (`plans/bug-tracker/fix-BUG-06-020.md`) — contains the authoritative reply-stream table, the divergence chain verified against notcurses source, the close-out decision, and the routing of follow-up work.
- [x] **File XTSMGRAPHICS bug-tracker entry**: `BUG-06-022` cross-linking to existing Section 39.10. The bug exists for visibility; the implementation work happens in §39.10.
- [x] **File XTVERSION SSOT LEAK bug-tracker entry**: new bug under section-11-mux.md (since `oriterm_mux/src/shell_integration/interceptor.rs` is the LEAK site). Cross-link to §38.4 implementation path that will subsume it.
- [x] **File notcurses-upstream petition bug-tracker entry**: new bug — petition notcurses upstream to add `oriterm(` to its `xtvers[]` vendor list (`in.c:1549-1565`) and `oriterm` to its TN→qterm map (`in.c:1717-1731`). Owner: ori_term project; contribution path: notcurses GitHub PR.
- [x] **Add NOTE under Section 38.4**: cross-reference this diagnostic so when XTGETTCAP lands, the reviewer of §38.4 sees the cluster collateral close path.
- [x] **Update bug-tracker entries for BUG-06-018 and BUG-06-019**: add a Resolution back-reference to this diagnostic so the cluster's collateral close path is traceable.

No production code changes. No tests added. No commit-changes to compiler/library/application source.

---

## R. Third Party Review Findings

(Initially empty — populated by Phase 5 `/tpr-review` if findings arise. The diagnostic artifact itself is the deliverable; TPR will validate the document's claims against the actual code and notcurses source.)

---

## 4. Completion Checklist

- [x] All new tests pass unchanged after fix — N/A (no code change; existing pilot tests still pass: `cargo test -p oriterm_core --test spec_chain notcurses_startup` green).
- [x] Matrix completeness verified — N/A (diagnostic-only).
- [x] Debug AND release builds pass — N/A (no code change).
- [x] Windows cross-compile green — N/A (no code change).
- [x] If the fix touches the GPU render path — N/A.
- [x] If the fix touches the hot render path — N/A.
- [ ] `timeout 150 ./test-all.sh` green — verify no test regressions from filing follow-up bugs (which only edit markdown).
- [ ] `./clippy-all.sh` green — N/A (no code change).
- [ ] `./build-all.sh` green — N/A (no code change).
- [ ] `cargo test -p oriterm_core --test spec_chain` green — verify pilot test still passes.
- [ ] `/commit-push` — commit fix section + follow-up bug filings before review.
- [x] Plan TPR (Phase 2.5) — Skipped (medium severity, non-elevated subsystem, round-2 consensus). See §2.5 above.
- [ ] `/tpr-review` (Phase 5 — code review) passed — validates the diagnostic claims against actual oriterm + notcurses source.
- [ ] `/impl-hygiene-review` passed — MUST run AFTER code `/tpr-review` is clean.
- [x] **Capability regression gate** — N/A (no capability disabled).
- [ ] `/improve-tooling` retrospective — capture tooling gaps surfaced during the diagnostic (e.g., the pilot test harness's `HostRequest::ColorQuery` showing as unresolved was confusing — could be improved with a "production mode" simulator that auto-resolves color queries).
- [x] Bug entry in `plans/bug-tracker/section-06-rendering-perf.md` updated with `[x]` resolution.
- [x] Fix section frontmatter `status` will flip to `complete` after Phase 5 closes.
- [x] Bug-tracker `00-overview.md` Quick Reference open bug count updated for section 06 (decrement by 1).
- [ ] Final `/commit-push` — commit closure artifacts.

**Exit Criteria:** This bug is complete when (a) the fix section above contains the authoritative reply-stream table and verified divergence chain, (b) BUG-06-022 (XTSMGRAPHICS) and BUG-11-018 (XTVERSION SSOT LEAK) and BUG-06-023 (notcurses-upstream petition) are filed and visible in the tracker, (c) Section 38.4 carries a back-reference to this diagnostic, (d) BUG-06-018 and BUG-06-019 entries carry Resolution back-references to this fix section, and (e) `cargo test -p oriterm_core --test spec_chain` is green proving the diagnostic's reply-stream observation is reproducible.
