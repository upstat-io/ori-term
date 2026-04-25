---
section: "06"
title: "Rendering & Performance Bugs"
status: in-progress
reviewed: false
goal: "Track and fix rendering performance bugs — frame time, input latency, GPU bottlenecks"
depends_on: []
third_party_review:
  status: resolved
  updated: 2026-03-31
sections:
  - id: "06.1"
    title: "Active Bugs"
    status: in-progress
  - id: "06.R"
    title: "Third Party Review Findings"
    status: complete
---

# Section 06: Rendering & Performance Bugs

**Status:** In Progress
**Goal:** Track and fix all rendering performance issues — frame time, input latency, GPU pipeline bottlenecks.

**Note:** This section is never marked complete. New bugs are appended as discovered.

---

## 06.1 Active Bugs

- [x] `[BUG-06-020][medium]` **notcurses-demo bugs 016/018/019 cluster: identify which oriterm capability-probe reply flips notcurses into the non-Z-order-respecting render path** — found by 2026-04-24 trace investigation (BUG-06-018 root-cause analysis).
  Resolved: diagnostic complete on 2026-04-25. See `plans/bug-tracker/fix-BUG-06-020.md` for the authoritative reply-stream diff (replayed via `notcurses_startup_emits_at_least_one_pty_reply` pilot test against `plans/spec-conformance/captures/notcurses-demo-intro.cap`) and verified divergence chain. Root cause: oriterm has no recognized identity channel for notcurses — XTVERSION reply uses `oriterm(` vendor prefix not in notcurses' `xtvers[]` table at `in.c:1549-1565`, XTGETTCAP not implemented (and `oriterm` is not in the TN→qterm map at `in.c:1717-1731` even if it were). Result: `qterm = TERMINAL_UNKNOWN` → `apply_term_heuristics` `default` arm omits `caps.rgb/quadrants/sextants/smulx` → cluster defects. Kitty graphics tuning is identity-independent so unaffected. Implementation routing: Section 38.4 (XTGETTCAP) is the canonical home for the partial cluster collateral fix (RGB key recovers `rgb=true` independent of TN matching); Section 39.10 owns XTSMGRAPHICS; new follow-up bugs filed for XTSMGRAPHICS visibility (BUG-06-022), XTVERSION SSOT LEAK (BUG-11-18), and notcurses-upstream petition (BUG-06-023).
  Found: 2026-04-24 | Source: BUG-06-018 trace investigation.

- [ ] `[BUG-06-022][medium]` **XTSMGRAPHICS (`CSI ? Pi ; Pa ; Pv S`) graphics geometry / color-register query has no reply path — notcurses queries for sixel parameters and gets nothing** — found by 2026-04-25 BUG-06-020 diagnostic.
  Repro: feed `\x1b[?1;3;256S`, `\x1b[?2;1;0S`, or `\x1b[?1;1;0S` to oriterm via the VTE handler; observe NO PTY reply emitted (verified via `notcurses_startup_emits_at_least_one_pty_reply` pilot test — XTSMGRAPHICS queries from the captured handshake produce no entries in oriterm's reply stream).
  Subsystem: `oriterm_core/src/term/handler/` capability-probe reply path. Owning plan subsection: **`plans/roadmap/section-39-image-protocols.md` §39.10 "XTSMGRAPHICS Sixel Size Negotiation"** (status: not-started). This bug-tracker entry exists for visibility into the open work; the implementation lands via §39.10. Required responses per `~/projects/reference_repos/console_repos/notcurses/src/lib/in.c:1174-1194` and `:1856-1857`: `CSI ? 1 ; 0 ; <pixel-width-or-N> S` for geometry, `CSI ? 2 ; 0 ; <count> S` for color registers.
  Found: 2026-04-25 | Source: BUG-06-020 diagnostic.
  Note: Cross-link to §39.10. Mitigates the cluster only partially — full mitigation also requires XTGETTCAP (Section 38.4) RGB capability response.

- [ ] `[BUG-06-023][medium]` **Petition notcurses upstream to recognize `oriterm(` in its XTVERSION vendor table and add `oriterm` to its XTGETTCAP TN→qterm map** — found by 2026-04-25 BUG-06-020 diagnostic.
  Repro: notcurses' `xtvers[]` table at `~/projects/reference_repos/console_repos/notcurses/src/lib/in.c:1549-1565` recognizes `XTerm(`, `WezTerm `, `kitty(`, `ghostty `, `foot(`, `mlterm(`, `tmux `, `iTerm2 `, `mintty `, `terminology `, `contour ` — but NOT `oriterm(`. Similarly the TN→qterm map at `in.c:1717-1731` recognizes `xterm`, `mlterm`, `xterm-kitty`, `xterm-ghostty`, `xterm-256color` — not `oriterm`. Without recognition through either channel, notcurses leaves `qterm = TERMINAL_UNKNOWN` for oriterm sessions and skips per-terminal capability heuristics (rgb/quadrants/sextants/smulx) → notcurses-demo cluster defects (BUG-06-016/018/019).
  Subsystem: external — notcurses upstream (`https://github.com/dankamongmen/notcurses`). Required work: contribute a PR adding `{ .prefix = "oriterm(", .suffix = ')', .term = TERMINAL_ORITERM, ... }` to `xtvers[]` AND adding a `TERMINAL_ORITERM` `apply_oriterm_heuristics` branch in `apply_term_heuristics` switch (`termdesc.c:991-1054`) that sets `caps.rgb=true, caps.quadrants=true, caps.sextants=true` (oriterm supports all three).
  Found: 2026-04-25 | Source: BUG-06-020 diagnostic.
  Note: Until upstream lands, the cluster collateral fix is partial — Section 38.4 XTGETTCAP RGB capability key recovers `rgb=true` independent of TN matching, but quadrants/sextants/smulx remain unset because they require `qterm` recognition.

- [x] `[BUG-06-019][high]` **notcurses-demo `xray` demo (`x`): scrolling marquee text at top — only the lower rows of the 10-row ASCII art are colorized; upper rows render dark/colorless** — found by manual.
  Resolved: not-a-bug in oriterm on 2026-04-24. Trace instrumentation (`debug/bug-06-019-trace` branch, `ORITERM_XRAY_TRACE=1`) on the exact repro shows the emitted byte stream notcurses sends per frame:
  1. `SGR 0 m` (full reset to default FG/BG)
  2. Write leg[0] at grid row 1 with no intervening SGR → row 1 cells get Named(Foreground) / Named(Background) — **this is spec-correct oriterm behavior per ECMA-48 §SGR**
  3. `SGR 38;2;103;183;140 m` + write leg[1] at grid row 2 (correct color)
  4. ... rows 2–10 each get per-row SGR before their writes and render correctly
  Row-by-row default-FG ratio from the trace confirms: row 1 = 89% default-FG, rows 2–10 = 0% default-FG, row 11 = 37% (video blit below the marquee).
  Root cause lives in notcurses's compositor output: `xray.c:make_slider` calls `ncplane_set_bg_alpha(n, NCALPHA_TRANSPARENT)` on the marquee plane (line 46), then per-row `ncplane_set_bg_rgb8(n, ...)`. The `set_bg_rgb8` call does not override alpha, so plane row 0's cells stay transparent in the composite. notcurses emits them as default-colored (no SGR before the chars), which oriterm faithfully renders as Named(Foreground) on Named(Background). Every conforming terminal does the same; visibility depends only on the user's default FG/BG palette. §08 CHARDRAWN plumbing and §13 kitty split are exonerated — the template carries FG/BG correctly; the arriving SGR IS `Reset`, and `sgr::apply(Attr::Reset)` at `oriterm_core/src/term/handler/sgr.rs:17-25` handles it per spec.
  Original suspect candidates (a/b/c in the entry) all ruled out by trace: template correctly reflects the last SGR at put_char_ascii time; no stale SGR; no wide-char interaction (all ASCII width=1).
  Instrumentation branch: `debug/bug-06-019-trace` (HEAD `03f2f651`) — delete after close, do not merge to dev.
  Found: 2026-04-22 | Source: manual
  Note: Sibling to BUG-06-016 (mojibake erasure — closed OBE 2026-04-22 via WezTerm/Windows Terminal cross-reference), BUG-06-017 (yield early-exit — escalated to spec-conformance §24.5), BUG-06-018 (whiteout cell destruction — still open). This bug likely shares the same "not-a-bug / notcurses-compositor-artifact" shape as BUG-06-016 — the pattern of cross-terminal symptom reproduction is the canonical test.
  Diagnostic close-out (2026-04-25 — BUG-06-020): root cause confirmed as identity-recognition failure in notcurses. The marquee uncolored symptom traces to `caps.rgb` being false (notcurses' `apply_term_heuristics` default arm doesn't set it), forcing fallback to ANSI 256-color resolution which loses the per-row truecolor SGR notcurses emits. See `plans/bug-tracker/fix-BUG-06-020.md` for the full divergence chain.

- [x] `[BUG-06-018][high]` **notcurses-demo `whiteout` demo (`w`): cells get destroyed but never recreated — worm trails leave permanent holes** — found by manual.
  Resolved: not-a-bug in oriterm's render path on 2026-04-24. Trace instrumentation (`debug/bug-06-018-trace` branch, `ORITERM_XRAY_TRACE=1`, 30k-event cap covering PUT/EL/ECH/ICH/DCH/CLWC/ED) on the exact repro shows the emitted byte stream notcurses sends destroys the mathplane itself. Key trace evidence at grid row 38 col 118 (mathplane `⎧` position):
  1. `PUT r=38 c=118 ch='⎧' w=1 fg=(212,175,55) bg=(0,0,0)` — math bracket drawn correctly
  2. `PUT r=38 c=118 ch='수' w=2 fg=(100,146,137) bg=(20,20,20)` — Korean wide-char OVERWRITES the bracket
  3. `PUT r=38 c=118 ch='m' w=1 fg=(124,31,193) bg=(20,20,20)` — random 'm' from worm activity overwrites that
  notcurses is emitting the **stdplane** contents (CJK/Cyrillic fill + worm-lightened chars) on top of the mathplane, ignoring the plane Z-order. oriterm correctly renders every byte it receives. User confirmed 2026-04-24 that WezTerm and Windows Terminal render whiteout correctly — notcurses sends those terminals a different byte stream based on runtime capability detection.
  oriterm's TERM is `xterm-256color` (same as WezTerm/Windows Terminal). The differential must be in a runtime probe reply (DA1/DA2/DA3, kitty keyboard `CSI ?u`, Mode 2026 sync output, kitty graphics probe, or similar). Identifying which probe reply flips notcurses into the buggy code path is a diagnostic task, not an oriterm render-path fix — filed as `BUG-06-020`.
  Instrumentation branch: `debug/bug-06-018-trace` (HEAD `ad62e279`) — preserved on remote for reference; do not merge to dev.
  Found: 2026-04-22 | Source: manual
  Note: Fourth notcurses-demo scene bug in the cluster (016 mojibake OBE, 017 yield escalated to §24.5, 019 xray closed 2026-04-24 as not-oriterm-bug). All four share the same shape: oriterm faithfully renders the bytes it receives; the visual defect is in notcurses's compositor output, which differs per-terminal via capability detection.
  Diagnostic close-out (2026-04-25 — BUG-06-020): the differential is oriterm's missing identity channel for notcurses. notcurses leaves `qterm = TERMINAL_UNKNOWN` for oriterm because XTVERSION vendor prefix `oriterm(` is not in notcurses' recognized list (`in.c:1549-1565`) and oriterm doesn't implement XTGETTCAP. The `default` arm of `apply_term_heuristics` (`termdesc.c:1054-1056`) omits `caps.rgb/quadrants/sextants/smulx`, so notcurses falls back to capability-degraded rendering paths that produce the visible defects. See `plans/bug-tracker/fix-BUG-06-020.md` for the full reply-stream diff and divergence chain. Cluster collateral fix routes through Section 38.4 (XTGETTCAP RGB) plus BUG-06-023 (notcurses-upstream petition).

- [ ] `[BUG-06-017][high]` **notcurses-demo `yield` demo (`y`): entire demo blips on screen then vanishes — demo exits early before running** — found by manual.
  Escalated: 2026-04-22 — merged into `plans/spec-conformance/section-24-notcurses-demo-full-pass.md` §24.5 (`yield (y)` scene) per user decision (the spec-conformance plan already owns per-scene notcurses-demo correctness gating; a bug-tracker duplicate is noise). Bug entry stays open as the tracker pointer to the §24.5 subsection; §24.5 body carries the Phase 1 findings.
  Phase 1 findings (before escalation):
  - Cross-terminal cross-check: ori_term Windows blips-and-vanishes; WezTerm Windows runs longer but not correctly; WezTerm macOS runs correctly. Confirms an ori_term-specific regression stacked on top of a Windows-notcurses issue that also hurts WezTerm-Windows. NOT OBE (unlike BUG-06-016).
  - Startup-reply hypothesis falsified: the capture at `plans/spec-conformance/captures/notcurses-demo-intro.cap` fed through `SpecHarness` produces 12 well-framed PTY replies (DA1/DA2/DA3, DSR-CPR, DECRQM 2026/1016, kitty query, CSI 14t/18t, kitty keyboard `[?0u`) + 256 async `HostRequest::ColorQuery` effects. `pilots/notcurses_startup.rs::notcurses_startup_reply_stream_*` scanners detect zero out-of-frame bytes and zero stray `q` bytes. The "malformed reply → `demo_getc` spurious keypress → `interrupt_demo()`" hypothesis is ruled out for the startup handshake.
  - Remaining hypotheses (need runtime capture, owned by §24.5): kitty `a=T` (transmit+place) reply framing during the render loop; placement cursor-move semantics in `kitty_create_placement` (linefeeds by `rows-1`, cursor-column behavior vs. kitty spec); GPU-path blit feedback; ConPTY translation artifacts on Windows.
  - Diagnostic artifact kept in-tree: `oriterm_core/tests/spec_chain/pilots/notcurses_startup.rs` — 3 tests (framing pin, stray-`q` pin, dump-to-stderr diagnostic) that stay as regression pins for any future reply-stream corruption.
  Repro: run `notcurses-demo` in ori_term on Windows; observe the `y` (yield) scene. Two threads concurrently polyfill a world map via pixel-blit. Expected: ~2-second progressive fill of world map from two thread colors. Observed: the demo flashes on screen briefly then disappears — the demo is terminating early, not flickering. Reference: `yield.c` exercises `media`, `pixel-blit` (kitty graphics on ori_term), `threading`. Requires `worldmap.png` under `/usr/share/notcurses/` — demo returns -1 if not found (`ncvisual_from_file` fails). See `yield.c:146` (`canopen_images` gate), `yield.c:155-163` (worldmap.png load gate), `yielder()` main loop `while(!*m->done && *m->filled < threshold_painted && iters < MAXITER)`.
  Subsystem: `oriterm_core/src/term/handler/image/kitty/` reply paths + placement cursor-move semantics; GPU kitty-blit path. Fix will land via spec-conformance §24.5 per-scene bisection (harness-driven) once §21 harness infrastructure is in place.
  Found: 2026-04-22 | Source: manual
  Note: Originally filed alongside BUG-06-016/018/019 as a cluster; 016 closed OBE after WezTerm cross-reference. 017 is confirmed ori_term-specific (WezTerm-macOS runs yield correctly). 018 (whiteout) and 019 (xray) remain independent per-scene bugs — 018 tracked by §24.4, 019 by §24.6; both can follow the same §24 bisection protocol.

- [x] `[BUG-06-016][high]` **notcurses-demo `mojibake` demo (`m`): a vertical column range immediately to the right of scrolling emoji planes stays BLACK — stdplane background text never re-renders onto the cells the emojis vacated** — found by manual.
  Resolved: OBE / not-a-bug on 2026-04-22. User confirmed via testing in WezTerm and Windows Terminal that the same "black strip" symptom reproduces identically — it is not an ori_term defect but cosmetic output from notcurses's greyscale-over-highcon composition. Log-based cell dump (rate-limited dump every 120 snapshots across a full demo run) proved:
  (1) zero orphaned WIDE_CHAR flags — the renderer-side wide-char bg_w bleed theory was wrong;
  (2) the cells in the "black strip" contain `ch=' '`, `fg=(189,189,189)`, `bg=(8,8,8)` — blank spaces with a near-black bg that came from `ncplane_greyscale(std)` (mojibake.c:3696) converting highcon's per-cell bg colors to grayscale. When the grayscale value falls into the motto's inter-word spaces, the cell renders as blank on a near-black bg, producing what looks like a black strip between the emoji plane's right edge and the stdplane text further right.
  Wins surfaced during the investigation (kept as real correctness fixes):
  (a) DECSCNM (private mode 5) toggle now calls `mark_all_dirty` — reverse-video changes fg/bg resolution for every cell and the GPU cached renderer was serving stale instances (commit 70ab4e54);
  (b) `delete_chars` strips cursor cell's `WIDE_CHAR`/`WIDE_CHAR_SPACER` flags after `clear_wide_char_at`, matching `insert_blank`'s existing pattern — the asymmetry could have carried a stale wide flag through the shift-left swap (70ab4e54);
  (c) `clear_wide_char_at` + `fix_wide_boundaries` call `dirty.mark_cols` on each modified sibling cell so downstream consumers that trust damage bounds don't miss the sibling (70ab4e54);
  (d) `swap_alt_clear` (DECSET 1047/1049) clears `alt_image_cache` alongside the alt grid, per alt-screen clear-on-enter semantics (2283974a).
  Reverted speculative defenses (no runtime evidence of effect, commit pending): `fix_wide_boundaries` in-band flag strip, `reset_wide_sibling` helper, renderer `wide_char_has_spacer` bg_w check at 3 sites, 4 regression tests for the helper.
  Verification path: to re-confirm the diagnosis, replay notcurses-demo in WezTerm/Windows Terminal/kitty and compare the mojibake scene against ori_term — the black strip appears identically in all of them.
  Repro: run `notcurses-demo` in ori_term on Windows; observe the `m` (mojibake) scene. The demo creates ~100 emoji-group planes (`makegroup(title, dimy+1, food_fruit, "food-fruit")`, `food_vegetable`, `food_asian`, `food_marine`, `food_sweet`, `drink`, `dishware`, `place_map`, `place_building`, ...) each containing a row of SMP emoji on a transparent plane, positioned over a stdplane that holds the "high contrast text is evaluated relative to the solved background" repeating text at gray (20,20,20) bg per `mojibake.c`. Expected: as each emoji plane scrolls up one row (`mojibake.c:3827-3838`: `ncplane_move_yx(planes[u], y - 1, x)` or `ncplane_resize` at y==2), the stdplane text should show through / be re-composited into any cells the plane vacates, so the stdplane text fills the background uniformly. Observed: **a consistent vertical column range immediately to the right of each emoji plane's content stays BLACK across multiple rows** — far-left stdplane text renders fine, far-right stdplane text renders fine, but a ~10-15-col band adjacent to the right edge of every emoji plane is empty black. Screenshot evidence: user shared `Screenshot 2026-04-21 222105_box.png` with red-box annotation highlighting the black trail column. The trail MOVES with the emoji planes as they scroll up — the column stays empty wherever an emoji plane passes through. Reference: `mojibake.c` `makegroup` + emoji planes, `xray.c:leg` → similar SMP width semantics if the SMP path is the shared root.
  Subsystem: almost certainly wide-char boundary handling where `WIDE_CHAR` / `WIDE_CHAR_SPACER` cells are left in a state that blocks subsequent narrow-char overwrite from the stdplane re-composition. Candidates refined: (a) `oriterm_core/src/grid/editing/wide_char.rs` `clear_wide_char_at` (lines 51-73) + `fix_wide_boundaries` (lines 21-45) — introduced by §09A / §08.5 work; if either is failing to clear the spacer cell after an SMP emoji scrolls away, the trail cell retains `WIDE_CHAR_SPACER` and refuses the stdplane's narrow-char write, (b) `oriterm_core/src/image/kitty/parse.rs` emoji-width classification if it diverges from notcurses's `wcwidth`, (c) interaction with `CellFlags::DRAWN` masking — the trail cells may look "undrawn" to `compute_rect_checksum` OR "empty" to a re-render gate somewhere. Fix location TBD via `/fix-bug` Phase 1 — but the localized column-range symptom strongly narrows it to wide-char boundary state persistence across the ncplane-move → stdplane-recomposite sequence.
  Found: 2026-04-22 | Source: manual
  Note: Sibling to BUG-06-017 (yield early exit), BUG-06-018 (whiteout worm destruction), BUG-06-019 (xray per-row SGR). Four-bug cluster sharing suspected root cause in wide-char / SGR / cell-template plumbing touched by §08.5 (SL/SR + ICH/DCH margin + wide-char cleanup) and §09A (CHARDRAWN). Escalated to plan on 2026-04-22 — see `plans/regressions/notcurses-demo-cluster/` (to be created via `/create-plan`).

- [ ] `[BUG-06-015][low]` **`oriterm/src/gpu/window_renderer/helpers.rs` is 549 lines, 49 over the 500-line limit** — found by §09A.N post-split file-size sweep.
  Repro: `wc -l oriterm/src/gpu/window_renderer/helpers.rs` prints `549`.
  Subsystem: `oriterm/src/gpu/window_renderer/helpers.rs`.
  Analysis: Pre-existing BLOAT. A `helpers.rs` file exceeding the cap is also a `code-hygiene.md` single-responsibility violation — helpers files should be split by concern, not grow monotonically. Natural split: identify the distinct concerns the helpers cover (cell metrics helpers, glyph cache helpers, damage helpers, viewport helpers) and extract each into its own sibling file under `window_renderer/`.

- [x] **BUG-06.1**: Noticeable input lag during key repeat — worse at smaller window widths
  - **Severity**: critical
  - **Found**: 2026-03-29 — manual, user report
  - **Resolved**: 2026-03-30 — User confirmed fixed. Likely resolved by frame budget and render pipeline improvements in recent commits.

- [x] **BUG-06.2**: Random extra text appears after resize following sustained key repeat
  - **Severity**: medium
  - **File(s)**: `oriterm_mux/src/pane/io_thread/handler.rs` (resize via IO thread)
  - **Root cause**: Race between queued key repeat events and synchronous main-thread reflow + PTY resize (SIGWINCH). Shell processes both simultaneously, producing interleaved output.
  - **Fix**: Threaded IO plan (plans/threaded-io). Resize now flows through `PaneIoCommand::Resize` to the IO thread, which serializes bytes and resize in its priority loop. Grid reflow and PTY resize happen atomically on the IO thread — no concurrent main-thread access. Coalescing ensures only the final size is applied during rapid resize.
  - **Resolved**: 2026-04-01 — threaded IO architecture eliminates the race condition.

- [x] **BUG-06.3**: Window surface not redrawn after dragging partially off-screen and back
  - **Severity**: high
  - **Found**: 2026-03-30 — manual, user report
  - **Root cause**: During a Win32 modal move loop, windows are never marked dirty (no terminal content changes). The 60 FPS timer generates `RedrawRequested` via `InvalidateRect`, but `modal_loop_render()` skips because no window is dirty. After the loop ends (`WM_EXITSIZEMOVE`), the timer is killed and no subsequent event marks the window dirty. The stale surface persists until cursor blink or mouse interaction.
  - **Fixed**: 2026-03-30 — Added `MODAL_LOOP_ENDED` atomic flag set in `WM_EXITSIZEMOVE`. `about_to_wait()` checks and clears it, marking all terminal windows dirty. Also hide terminal windows before close to prevent stale surface flash during teardown.

- [x] **BUG-06.4**: Settings dialog shows baby blue flash on open/close
  - **Severity**: medium
  - **File(s)**: `oriterm/src/app/dialog_management.rs` (dialog lifecycle), `oriterm/src/gpu/state/mod.rs` (poll_device)
  - **Root cause**: `render_dialog()` called `render_to_surface()` which submits GPU commands and presents, but did not call `device.poll()` to flush the work. The window became visible on the next tick via `show_primed_dialogs()` before the GPU had finished rendering the first frame, briefly showing uninitialized VRAM (baby blue). Terminal windows avoided this because `clear_surface()` already called `device.poll(wait_indefinitely())`.
  - **Found**: 2026-03-30 — user report. Only affects settings dialog, not terminal windows.
  - **Fixed**: 2026-03-30 — Added `GpuState::poll_device()` method and called it in `finalize_dialog()` after `render_dialog()`, matching the terminal window pattern. GPU work is now flushed synchronously before the Primed → Visible transition.

- [x] **BUG-06.5**: DX12 backend: terminal grid blank, only tab bar chrome renders
  - **Severity**: medium
  - **File(s)**: `oriterm/src/gpu/instance_writer/mod.rs` (`CLIP_UNCLIPPED`), `oriterm_ui/src/draw/scene/content_mask.rs` (`ContentMask::unclipped()`)
  - **Root cause**: `CLIP_UNCLIPPED` used `f32::NEG_INFINITY` / `f32::INFINITY` as clip rect values. In the shader, `clip_max = clip.xy + clip.zw` computed `-INF + INF = NaN`. DX12/HLSL treats NaN comparisons (`frag_pos > NaN`) as `true`, causing the clip test to discard EVERY fragment. Tab bar chrome was unaffected because UI framework widgets use finite clip rects from the layout system, not `CLIP_UNCLIPPED`. Same issue in `ContentMask::unclipped()` which used infinity for the default scene clip mask.
  - **Repro**: Set `gpu_backend = "dx12"` in `[rendering]`. NVIDIA RTX 3080, Windows, `Bgra8UnormSrgb` format.
  - **Found**: 2026-03-31 — manual, user testing.
  - **Fixed**: 2026-03-31 — Replaced infinity with large finite values (`-100_000.0, -100_000.0, 200_000.0, 200_000.0`) in both `CLIP_UNCLIPPED` and `ContentMask::unclipped()`. No NaN, all comparisons well-defined.

- [x] **BUG-06.6**: Font config changes not applied in realtime (weight, hinting, subpixel AA)
  - **Severity**: high
  - **File(s)**: `oriterm/src/gpu/window_renderer/font_config.rs` (`clear_and_recache`), `oriterm/src/app/config_reload/mod.rs` (`apply_font_changes`)
  - **Root cause**: Two stale caches: (1) `clear_and_recache()` cleared GPU atlases but not the `ShapedFrame` cache — the prepare fast path served old glyph IDs from the previous font. (2) `apply_font_changes()` rebuilt the `FontCollection` but didn't reset `last_rendered_pane` or `frame`, so the redraw path saw `content_changed=false` and skipped re-extraction.
  - **Found**: 2026-04-01 — manual, user report during settings UI testing.
  - **Fixed**: 2026-04-01 — (1) `clear_and_recache()` now resets `ShapingScratch.frame` to empty. (2) `apply_font_changes()` calls `ctx.invalidate_font_caches()` which clears `last_rendered_pane` and `frame`. Commits bfe736f and 793f52d.
  - **Needs**: Regression test that font config change invalidates shaping + frame caches.

- [ ] **BUG-06.7**: Vulkan backend: baby blue flash when opening settings dialog
  - **Severity**: low
  - **File(s)**: `oriterm/src/app/dialog_management.rs` (dialog lifecycle)
  - **Root cause**: Same symptom as BUG-06.4 (uninitialized VRAM visible before first frame). BUG-06.4 was fixed with `poll_device()` after `render_dialog()`, which works on DX12 but apparently Vulkan's poll timing differs — the GPU work may not be fully flushed before the window becomes visible.
  - **Repro**: Set `gpu_backend = "vulkan"`, open settings dialog. Brief baby blue flash before content renders.
  - **Found**: 2026-03-31 — manual, user report. DX12 (default) is not affected.

- [ ] **BUG-06.9**: Focus border around panes has asymmetric padding — left padded, right clips out of bounds
  - **Severity**: medium
  - **File(s)**: `oriterm/src/session/compute/mod.rs` (`snap_to_grid`, pane `pixel_rect` computation), `oriterm/src/app/redraw/multi_pane/mod.rs` (focus border call site)
  - **Root cause**: The pane `pixel_rect` (used by `append_focus_border`) has inconsistent insets — the left edge has proper padding (from window border or divider offset) but the right edge either extends to the window edge without border inset or the `snap_to_grid` trimming creates an asymmetric result. The focus border renders exactly at the `pixel_rect` bounds, so if the rect itself is wrong, the border clips on the right side.
  - **Repro**: Split a pane (Ctrl+Shift+D or equivalent). Observe the accent focus border on the active pane — left side has visible padding from the window edge, right side clips or has no padding.
  - **Found**: 2026-03-31 — manual, user report.
  - **Note**: Roadmap section 33 (split-nav-floating) touches this area.

- [ ] **BUG-06.10**: `rss_plateaus_under_sustained_output` is flaky on Windows — reports 3.0 MB growth instead of <2 MB ceiling
  - **Severity**: medium
  - **File(s)**: `oriterm_core/tests/rss_regression.rs:134`
  - **Repro**: `cargo test -p oriterm_core --test rss_regression rss_plateaus_under_sustained_output` — fails when run as part of the full workspace test serial pass (`cargo test --workspace -- --test-threads=1`); passes in isolation. Observed value `RSS grew 3.0 MB after 100k lines (warmup: 7.6 MB, after: 10.6 MB)` vs the 2 MB ceiling. The 1 MB excess is small enough to be Windows heap/scheduler noise from prior tests in the same process group inflating the warmup baseline lower than steady state.
  - **Found**: 2026-04-08 — surfaced during `/fix-bug` BUG-07-009 verification
  - **Source**: `/fix-bug` test-all sweep
  - **Fix**: Either (1) re-baseline the warmup measurement to be more representative (run a longer warmup loop, or take min-of-N samples), or (2) raise the ceiling to ~5 MB on Windows where heap fragmentation produces a larger noise floor than on Linux. The semantic invariant — RSS plateaus under sustained output — is correct; only the threshold is too tight.

- [ ] **BUG-06.8**: Floating pane (Ctrl+Shift+P) is completely transparent — no background fill
  - **Severity**: high
  - **File(s)**: `oriterm/src/gpu/window_renderer/multi_pane.rs` (`append_floating_decoration`), `oriterm/src/app/redraw/multi_pane/mod.rs`
  - **Root cause**: `append_floating_decoration()` only renders a drop shadow and accent border around floating panes. There is no opaque background fill — the floating pane's cell backgrounds are composited directly over the main window content beneath, making it appear fully transparent. The floating pane should use the same background settings as the main window (opacity, blur, bg color).
  - **Repro**: Press Ctrl+Shift+P to toggle a floating pane. The pane content is transparent — you can see the main terminal grid behind it.
  - **Found**: 2026-03-31 — manual, user report.

- [x] `[BUG-06-013][critical]` **GPU atlas unit tests hang indefinitely on `GpuState::new_headless()` — `cargo test -p oriterm` never completes**
  Resolved: Fixed on 2026-04-14. Added `sanitize_headless_env()` to `oriterm/src/gpu/state/headless.rs` — a `OnceLock`-guarded helper that unsets `WAYLAND_DISPLAY` and `DISPLAY` once per process before wgpu's vulkan adapter enumeration runs, preventing the blocking probe on unresponsive compositor sockets. `new_headless()` calls it as its first statement. File-level `#![allow(unsafe_code)]` is justified by the strict scope (test-only module, process-lifetime idempotent mutation) and documented in the file preamble. Verified: `cargo test -p oriterm --lib gpu::atlas` now passes 61/61 tests in 1.92s with `WAYLAND_DISPLAY=wayland-0` still exported in the shell — no more workaround needed. Before the fix this same invocation hung indefinitely.
  Repro: `timeout 120 cargo test -p oriterm --lib gpu::atlas::tests::insert_and_lookup_round_trip`. The test prints `test ... has been running for over 60 seconds` and never completes. All 49 tests under `oriterm::gpu::atlas::tests::*` (i.e. every `#[test]` fn in `oriterm/src/gpu/atlas/tests.rs`) are affected; the sibling `gpu::atlas::rect_packer::tests::*` (pure rect-packing, no GPU) complete in 0.00s. Symptom is reproducible from a clean build tree (`cargo clean -p oriterm` followed by `cargo test`). Earlier in the same session the same tests passed cleanly through lefthook's pre-commit hook at commit `310429a2` (round-9 TPR fixes) — something transitioned the environment or left stale GPU state that now blocks `pollster::block_on(instance.enumerate_adapters(...))` in `oriterm/src/gpu/state/helpers.rs:77`.
  Subsystem: `oriterm/src/gpu/state/helpers.rs` (`pick_adapter`, `request_device`), `oriterm/src/gpu/state/headless.rs` (`GpuState::new_headless`), `oriterm/src/gpu/atlas/tests.rs` (49 tests that all call `GpuState::new_headless()` as preamble)
  Impact: Critical — the pre-commit hook runs `cargo test` and blocks indefinitely, making every commit through lefthook impossible. Forces either `--no-verify` (banned by CLAUDE.md) or manual recovery. Also blocks `/tpr-review` iteration loops that rely on clean commits between iterations.
  Diagnostics run:
    - `timeout 300 cargo test -p oriterm --lib gpu::atlas::tests::insert_and_lookup_round_trip` — still hanging at `test has been running for over 60 seconds` after 5 minutes.
    - `RUST_LOG=info cargo test ... -- --nocapture` — NO log output before the hang (the `log::info!("GPU (headless): adapter=...")` line in `headless.rs:69` never fires), so the hang is BEFORE device init completes — likely in `enumerate_adapters` or `request_adapter`.
    - Process state: 31 threads, main thread in `futex_wait_queue`, CPU time <= 120 jiffies over 10 min — classic indefinite block.
    - Interactive `cargo test -p oriterm_core` passes in <70s and 1699/1699 tests green — hang is ISOLATED to `oriterm` crate's GPU-initializing tests.
    - `/dev/dxg` exists and is readable (WSL GPU device).
  Suspect causes:
    1. WSL `/dev/dxg` connection got into a bad state after a prior `kill -9` on a wgpu-using test process and now blocks adapter enumeration indefinitely (no timeout inside wgpu).
    2. Atlas tests are over-coupled: they use a real GPU adapter to test pure rect-packing / LRU logic that does not need GPU state. 49 `#[test]` fns each initialize a full wgpu `Instance` + `Adapter` + `Device` — if any one blocks, the whole suite blocks; and the tests cannot skip gracefully when adapter init HANGS (only when it ERRORS).
    3. `pollster::block_on(instance.enumerate_adapters(...))` has no timeout wrapper — any backend driver hang (vulkan via dzn/dxvk on WSL) blocks the test thread forever.
  Proposed fix directions (all needed; not alternatives):
    - **Design fix**: split `oriterm::gpu::atlas::tests` so the pure packing-logic tests (`insert_at_max_dimension_succeeds`, `insert_oversized_glyph_returns_none`, `glyphs_do_not_overlap`, `lru_eviction_*`, `insert_duplicate_returns_cached`, `insert_triggers_new_page_allocation`, etc.) use a fake `Device`/`Queue` or stub-buffer backend and DO NOT call `GpuState::new_headless()`. Only the handful of tests that truly exercise the GPU texture path (texture upload, format verification) should touch a real adapter. Expected: 40+ tests go from multi-minute blocking → sub-ms. This directly addresses the user's observation that "the GPU Atlas tests should be nearly instant".
    - **Infra fix**: wrap `GpuState::new_headless()` body in a bounded wait — `std::thread::spawn` + `join_timeout(Duration::from_secs(5))` around the adapter-enumeration + device-request — so a stuck backend driver produces `Err(GpuInitError)` instead of hanging. Callers already handle `Err` by skipping (`let Ok(gpu) = ... else { eprintln!("skipped: no GPU adapter available"); return; };` pattern), so the fix is localized.
    - **Graceful-skip reinforcement**: match the `Graceful Skip Protocol` from `.claude/rules/tests.md` — currently the adapter-absent path skips cleanly, but the adapter-HANG path does not. A timeout turns a hang into a skip, which is the correct degradation.
  Subsystem: `oriterm/src/gpu/atlas/` (test design), `oriterm/src/gpu/state/headless.rs` (adapter-init timeout), `oriterm/src/gpu/state/helpers.rs` (pollster::block_on wrap)
  Found: 2026-04-14 | Source: manual
  Note: Regression appeared mid-session on 2026-04-14 between 14:29 (tests passed via lefthook at commit 310429a2) and ~15:15 (tests hang). Related roadmap area: atlas test infrastructure may intersect with `oriterm/src/gpu/visual_regression/` dialog_helpers and pipeline_tests (also use `new_headless`). A fix here should double-check those paths aren't susceptible to the same hang.
  Root cause (confirmed 2026-04-14): `WAYLAND_DISPLAY=wayland-0` + `DISPLAY=:0` are set in the shell (WSLg defaults). wgpu's Vulkan backend enumeration discovers Wayland/X11 surface extensions, tries to connect to the display-server sockets as part of adapter init, and blocks indefinitely when the sockets are unresponsive (WSLg state drift mid-session). Reproduced via `timeout 30 env -i PATH="$PATH" HOME="$HOME" bash -c 'cargo test -p oriterm --lib gpu::atlas::tests::atlas_creation_succeeds'` → PASSES in 0.07s. Reproduced again via `unset WAYLAND_DISPLAY DISPLAY; cargo test ...` → PASSES in 0.06s. With the vars set in the normal shell → HANGS indefinitely. The fix on the wgpu side is pick_adapter/request_device timeout (proposed above). The test-side workaround until that lands: the test harness should unset `WAYLAND_DISPLAY` and `DISPLAY` at the top of `GpuState::new_headless()` when no window is being created, or the tests should set `WGPU_BACKEND=vulkan` and configure wgpu to skip surface-backend probing. The `Graceful Skip Protocol` from `.claude/rules/tests.md` already handles the adapter-absent case, but an adapter-HANG needs either a timeout OR pre-init env sanitization.

- [ ] `[BUG-06-012][high]` **notcurses-demo reports ~5 FPS drops that did not occur before Section 07 work**
  Repro: Run `notcurses-demo` in oriterm. Observe FPS counter — user confirmed 2026-04-24 that drops persist at HEAD (9c24ea01), specifically on the `xray` scene: 6 dropped frames out of 862 (~0.7% behind real-time).
  Subsystem: `oriterm_core/src/image/cache/`, `oriterm_mux/src/backend/embedded/mod.rs`, likely kitty graphics streaming path (`oriterm_core/src/image/kitty/`).
  Found: 2026-04-14 | Source: manual
  **Blocked:** pending profiling data (flamegraph) — shared-root-cause hypothesis with BUG-06-019 was disproven 2026-04-24.
  Investigation 2026-04-24 (fix-BUG-06-012 Phase 1 + fix-BUG-06-019 cross-reference):
    - Three originally-suspected §07 commits are pure correctness refactors with NO regression at code-read:
      - `4f88fafa` / `0d721104` (phantom-dirty guard across 13 methods) — identical behavior for live panes; only affects nonexistent-pane path (no live-pane regression possible).
      - `0d721104` `apply_frame()` extraction — semantically equivalent; same HashMap lookups, same Arc clone.
      - `0d721104` `intersects_viewport()` helper — pure code extraction, identical arithmetic.
    - Two post-filing commits eliminated frame-drop-class CPU hazards but did NOT fully resolve the drops:
      - `2af519c8` (IO-thread `select!` spin-on-disconnect) — fixed a CPU-spin on watcher-thread death.
      - `9c24ea01` (§13.3 scheduler `promote_deferred` sweep in `about_to_wait`) — fixed `WaitUntil(past)` spin on matured animation deadlines.
    - **Drops confined to `xray` scene at HEAD.** xray drives the kitty graphics streaming path at ~57 fps (862 frames / 15 s), with a scrolling ASCII-art marquee overlay (transparent fg/bg). xray explicitly drops frames when rendering can't keep up per `notcurses/src/demo/xray.c:6`.
    - **Shared-root-cause hypothesis with BUG-06-019: DISPROVEN.** BUG-06-019 closed 2026-04-24 as not-a-bug in oriterm — the trace evidence (branch `debug/bug-06-019-trace`) showed notcurses emits `SGR 0` before leg[0] with no follow-up SGR, which oriterm correctly renders as default FG/BG per ECMA-48. That's a notcurses-compositor artifact, unrelated to any perf path.
    - **New signal from the same trace run: `perf: SLOW k2r=17–27ms frame=1.4–1.5ms prep=~550µs gpu=~650µs`** observed consistently during rendering (`oriterm.log` in the user's install dir). The 16+ ms key-to-render latency with sub-ms frame/prep/gpu times means 15–25 ms is spent **waiting** somewhere between keypress and render completion — mux-pump, snapshot delivery, or event-loop scheduling. That's a concrete BUG-06-012 lead that does not require further scene-playback instrumentation.
    - **Blocking rationale:** root-cause pin still needs either (a) a flamegraph (`cargo flamegraph --bin oriterm --release -- <notcurses-demo x>`) to localize the 15–25 ms latency OR (b) targeted instrumentation of the mux-pump → snapshot-deliver → event-loop wake path to measure per-stage latency. Resumption paths in order of preference: (1) user produces flamegraph of xray on a fresh run; (2) add targeted perf-stats spans to the mux-pump/snapshot path similar to the existing `perf: SLOW k2r=` infrastructure; (3) synthetic microbench streaming kitty APC frames at 57 fps to `Term` directly.

- [x] `[BUG-06-011][medium]` **Settings dialog golden image mismatch — `settings_appearance_clean_96dpi`**
  Resolved: OBE on 2026-04-25. Test now passes reliably under the canonical invocation (`cargo test -p oriterm --features gpu-tests --lib gpu::visual_regression::settings_dialog::settings_appearance_clean_96dpi`) — 5/5 consecutive isolation runs and 173/173 across the full `gpu::visual_regression` suite at `--test-threads=4`. No `*_actual.png` / `*_diff.png` artifacts remain in `oriterm/tests/references/`. The bug entry's own hypothesis ("pixel-level drift from font/layout changes rather than a functional regression") matches the resolution: the committed reference (`settings_appearance_clean_96dpi.png`) and current rendering output now agree, so the cached-path `compare_with_reference` check succeeds within `PIXEL_TOLERANCE=2` / `MAX_MISMATCH_PERCENT=0.5%`. The original bug entry's repro command was missing the `--features gpu-tests` flag — without the feature the test is filtered out and the canonical workflow (`./test-all.sh`) does not enable `gpu-tests`, which is why the green run only surfaces under the explicit feature invocation.

- [x] `[BUG-06-014][medium]` **SGR 53/73/74 flags stored on cells but not rendered — no visual effect for overline, superscript, subscript**
  Resolved: fixed on 2026-04-25 — see `plans/bug-tracker/fix-BUG-06-014.md`. Wired OVERLINE/SUPERSCRIPT/SUBSCRIPT flags into the GPU prepare pipeline (`oriterm/src/gpu/prepare/decorations.rs` overline rect emission + new `super_sub_glyph_offset` helper applied at all three glyph-emit call sites with integer-pixel rounding) and the HTML clipboard exporter (`oriterm_core/src/selection/html/style.rs` extended with `overline` + `vertical_align`; `text-decoration` now scales linearly via direct-buf write across underline/strikethrough/overline; `vertical-align:super`/`sub` emits without `font-size` to preserve monospace grid + match GPU's vertical-only shift). Plan TPR + Code TPR + impl-hygiene-review (Phases 0–6) all clean. Hygiene findings F-03, F-07, F-08, F-10, F-15, F-16, F-19 (plus side-effects F-04/05/06) deferred to plan `plans/gpu-prepare-html-algorithmic-dry/` per `.claude/rules/impl-hygiene.md` §Findings Disposition.

- [ ] `[BUG-06-021][low]` **`oriterm/src/gpu/window_renderer/render.rs` `render_frame_cached` (line 77) and `render_cached` (line 321) exceed 4-level nesting cap**
  Repro: `bash .claude/skills/impl-hygiene-review/hygiene-lint.sh --scope oriterm/src/gpu/window_renderer/ --summary` reports `BLOAT: nesting-depth` findings at both sites with nesting depth 5 (limit 4). Flagged during BUG-04-004 hygiene review (2026-04-25).
  Subsystem: `oriterm/src/gpu/window_renderer/render.rs`
  Found: 2026-04-25 | Source: impl-hygiene-review (BUG-04-004 retrospective)
  Note: `.claude/rules/code-hygiene.md` §Style targets ≤4-level nesting. Remediation per impl-hygiene.md §Nesting Depth & Guard Clauses: invert+return early, extract inner matches into named helpers, collapse `if let` chains into a helper returning `Option`. Both functions are in the GPU render hot path — any refactor must preserve the existing invariants (damage-region, frame-budget, cached-path routing).

---

## 06.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- [x] `[TPR-06-001][high]` `oriterm/src/app/event_loop.rs:442`, `oriterm/src/gpu/state/helpers.rs:111` — the frame-budget gate was removed under the assumption that `PresentMode::Mailbox` always paces rendering, but the renderer explicitly falls back to `Immediate` when Mailbox is unavailable.
  Resolved: Added `GpuState::needs_frame_budget()` that returns true for `PresentMode::Immediate`. The rendering gate in `about_to_wait()` now applies the budget check only when the surface requires it (Immediate mode), while Mailbox/Fifo paths render immediately. Fixed 2026-03-30.

- [x] `[TPR-06-002][medium]` `oriterm/src/app/perf_stats.rs:305` — the new phase-breakdown instrumentation logs at `info` level even when profiling is disabled.
  Resolved: Phase breakdown logging now routes through the same `log_fn`/`self.profiling` gate as the rest of the perf output. Fixed 2026-03-30.

- [x] `[TPR-06-003][high]` `oriterm/src/app/init/mod.rs:36`, `oriterm/src/app/window_management.rs:133`, `oriterm/src/gpu/state/mod.rs:80`, `oriterm/src/gpu/state/mod.rs:104`, `oriterm_ui/src/window/mod.rs:241` — `use_compositor_surface` is decided from the requested config backend before GPU initialization, but `GpuState::new()` can still fall back from the DX12 `DirectComposition` path to plain DX12 or Vulkan when that init attempt fails. Because the window is already created with `WS_EX_NOREDIRECTIONBITMAP`, the fallback backend inherits a compositor-surface window it cannot present to correctly, which reintroduces the invisible-window failure on the exact fallback path this patch is trying to harden.
  Resolved: Fixed on 2026-03-31. Added `uses_dcomp` field to `GpuState` (set during `try_init`). `init/mod.rs` now calls `clear_compositor_surface_flag()` after GPU init if DComp wasn't actually used. `window_management.rs` uses `gpu.uses_dcomp()` instead of config-based backend check for new windows. `clear_compositor_surface_flag()` added to `oriterm_ui/src/window/mod.rs` (Win32 FFI to remove `WS_EX_NOREDIRECTIONBITMAP`).

- [x] `[TPR-06-004][high]` `oriterm_ui/src/window/mod.rs:339` — `clear_compositor_surface_flag()` clears `WS_EX_NOREDIRECTIONBITMAP` with `SetWindowLongPtrW`, but never follows up with `SetWindowPos(..., SWP_FRAMECHANGED)`. Microsoft’s `SetWindowLongPtrW` docs state that cached window data does not take effect until `SetWindowPos` is called, so the fallback path in `oriterm/src/app/init/mod.rs:70` can still leave the startup window in the compositor-surface state it was created with.
  Resolved: Fixed on 2026-03-31. Added `SetWindowPos(SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED)` after `SetWindowLongPtrW` in `clear_compositor_surface_flag()`.

- [x] `[TPR-06-005][high]` `oriterm/src/app/init/mod.rs:214`, `oriterm/src/app/window_management.rs:95` — the steady-state render path now correctly forces opacity to `1.0` on surfaces without alpha support (`handle_redraw` and `handle_redraw_multi_pane` both do this), but the pre-show `gpu.clear_surface()` path still uses the configured window opacity unconditionally. On the same Vulkan/opaque fallback path this patch is hardening, the first presented frame can therefore still be rendered with the invalid sub-1.0 opacity before the later redraw clamps it.
  Resolved: Fixed on 2026-03-31. Both `init/mod.rs` and `window_management.rs` now check `gpu.supports_transparency()` and clamp opacity to 1.0 when the surface lacks alpha support.

---
