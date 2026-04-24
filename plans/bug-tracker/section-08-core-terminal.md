---
section: 8
title: "Core Terminal"
domain: "VTE handler, terminal emulation, bell, escape sequences"
status: in-progress
---

# Section 08: Core Terminal

Terminal emulation behavior — VTE handler, bell, escape sequences, terminal modes.

## Open Bugs

- [ ] `[BUG-08-25][medium]` **Kitty `a=a,s=2` (run-wait) collapses with `s=3` (run) — the "wait for new frames on loop-end" semantic is not implemented** — found by §13.3 verification.
  Repro: feed a finite-loop kitty animation with `a=a,s=2` and wait for loop completion. Expected per kitty graphics-protocol.rst §Animation control: when loops reach `v=<N>`, STOP and wait for `a=f` to append more frames; a later `a=a,s=3` would then run the extended animation. Actual: `ImageCache::set_animation_action` at `oriterm_core/src/image/cache/animation.rs:247-260` treats `s=2` and `s=3` identically (both `paused = false; loops_completed = 0`), so `s=2` behaves exactly like `s=3` — the animation stops when loops are exhausted (via `AnimationState::is_finished`) but there is no mechanism to "wait for new frames" and resume automatically when they arrive.
  Subsystem: `oriterm_core/src/image/cache/animation.rs`, `oriterm_core/src/image/mod.rs` (`AnimationState`).
  Analysis: GAP — spec-defined distinct semantic not yet in the codebase. Requires `AnimationState::wait_mode: bool` (or similar) to differentiate s=2 from s=3 at the `advance_animations` consumption site. When `wait_mode=true` and `is_finished()` is true, don't `pause` — stay in a "waiting" state; when a new frame lands via `add_animation_frame`, extend `total_frames`, reset `is_finished`, and resume.
  TDD matrix: (1) `a=a,s=2` + `v=2` + wait beyond 2 loops — animation stops, current_frame stays at last frame; (2) same + later `a=f` appends frame — animation resumes and advances through the new frame; (3) `a=a,s=3` + `v=2` + wait beyond 2 loops — animation stops (same as s=2 today, but the wait-mode flag stays false so a later `a=f` does NOT resume); (4) negative pin: `s=3` after `v=2` exhaustion + `a=f` — animation does NOT resume (positive pin for s=2 behavior differentiation). Cross-platform: no platform gates — pure core logic.
  Catalog row: `KG-ANIMATE-RUN-WAIT` (currently `verified-with-deviation` citing this bug).
  Found: 2026-04-22 | Source: /continue-roadmap §13.3 verification

- [ ] `[BUG-08-21][medium]` **`kitty_store_from_file` reads the whole file into memory before enforcing the `max_single_image_bytes` limit** — found by §13.1 TPR round 0 (gemini F3).
  Repro: a kitty `APC _Ga=t,t=f,f=32,s=W,v=H;<base64 file path> ST` pointing at a file larger than `ImageCache::max_single_image_bytes()`. Current code at `oriterm_core/src/term/handler/image/kitty/store.rs:95-103` calls `std::fs::read(path)` first, then checks `file_data.len() > max_bytes` and rejects — the allocation peak already matches the file size.
  Subsystem: `oriterm_core/src/term/handler/image/kitty/store.rs`.
  Analysis: Local-program DoS vector (a program the user already launched can feed a huge `t=f` path and spike RSS up to that file's size before the cache-size check rejects the store). `max_single_image_bytes` prevents *caching* but not the transient read allocation. Gemini's recommended fix is to query `std::fs::metadata(path)?.len()` first and reject on size before calling `fs::read`; or use a `Take`-bounded reader to cap the allocation at `max_bytes + 1`. The fix must still preserve the `t=t` cleanup semantic (remove the source file even when oversized — the current code at `store.rs:98-102` does this).
  TDD matrix: (1) success case with a file ≤ max_bytes — stores + removes source file per t=t. (2) oversized file with t=f — ENOMEM reply + file NOT read fully + file NOT removed. (3) oversized file with t=t — ENOMEM reply + file NOT read fully + file IS removed. (4) metadata-unavailable case (broken symlink, permission denied) — EIO reply with clean error path. Cross-platform discipline: verify metadata path parity on Linux/macOS/Windows.

- [x] `[BUG-08-22][low]` **`kitty_finalize_payload` unnecessarily clones `cmd.payload` on the non-chunked path** — found by §13.1 TPR round 0 (gemini F4).
  Repro: `grep -n 'cmd.payload.clone()' oriterm_core/src/term/handler/image/kitty/mod.rs` — matches at line 84 inside `kitty_finalize_payload`. The caller `kitty_transmit` / `kitty_transmit_and_place` / `kitty_frame` in `transmit.rs` / `frame.rs` receives `cmd: KittyCommand` BY VALUE but then passes `&cmd` to `kitty_finalize_payload`, forcing the inner clone.
  Subsystem: `oriterm_core/src/term/handler/image/kitty/mod.rs`.
  Analysis: WASTE — each non-chunked kitty transmit allocates a second copy of the payload (can be megabytes). Not in the hot render path, but per-command allocation is still wasteful. Fix: either (a) change `kitty_finalize_payload` to take `KittyCommand` by value and extract `cmd.quiet` before the call in the transmit caller, or (b) swap `cmd.payload` out with `std::mem::take(&mut cmd.payload)` (requires `&mut KittyCommand`). Path (a) is cleaner — `cmd.quiet` is a `u8` that's easily extracted.
  TDD matrix: this is a performance change with no observable behavior change; the existing §13.1 per-action + per-format + per-transmission tests already pin the behavior end-to-end. Add an allocation-count regression test (via `oriterm_core::tests::alloc_counter` if one exists for cold-path cases) OR rely on the existing `rss_regression.rs` harness.
  **Fixed in §13.2 (2026-04-22)**: refactor combined paths (a) + (b). `kitty_finalize_payload` now takes `KittyCommand` by value and returns `(u32, KittyCommand)` merged command; new `KittyStoreParams::from_merged` helper uses `std::mem::take(&mut cmd.payload)` to extract the payload into storage params without cloning. Caller `kitty_transmit` / `kitty_transmit_and_place` / `kitty_frame` all migrated to consume the merged cmd. No `cmd.payload.clone()` remains in the kitty handler family (`grep -r 'cmd\.payload\.clone' oriterm_core/src/term/handler/image/kitty/` returns empty). §13.2 tests pin the behavior at the catalog-row level — no new allocation regression test was added per the original TDD-matrix recommendation (the fix is a refactor with observable equivalence; existing §13.1 + §13.2 matrix covers the per-command path).

- [ ] `[BUG-08-24][medium]` **`kitty_animate` silently drops on missing `i=` while `kitty_place` emits ENOENT on the same condition** — found by impl-hygiene-review.
  Repro: feed `\x1b_Ga=a,s=1\x1b\\` (animate stop, no `i=`, no `I=`) through `handle_apc_dispatch`. Expected: ENOENT or EINVAL reply so the client can recover. Actual: no reply emitted — `debug!("kitty animate: no image_id")` + early return at `oriterm_core/src/term/handler/image/kitty/animate.rs:20-23`. Compare `a=p,s=1` (no `i=`) which correctly emits `\x1b_Gi=0;ENOENT\x1b\\` via `kitty_place` at `place.rs:13`.
  Subsystem: `oriterm_core/src/term/handler/image/kitty/animate.rs`.
  Analysis: LEAK:swallowed-error per `.claude/rules/impl-hygiene.md`. Every other action handler (`a=t`, `a=T`, `a=p`, `a=f`) emits a reply on missing identifier; animate is the only handler that silently drops. Additionally, kitty's `a=a` supports `I=` as an alternate identifier per `graphics.c::handle_animate_command` (resolves to `newest_by_image_number`) — the current implementation ignores `I=` entirely, an additional spec-gap. Pre-existing in the kitty handler split; NOT introduced by §13.2 refactor (§13.2 scope was chunked + malformed-base64; animate's error-reply path is adjacent but out-of-subsection).
  Fix prescription: replace the silent early-return with `self.kitty_respond(&KittyReplyContext::from_cmd(cmd), "EINVAL: a=a requires i= or I= fallback"); return;`. ALSO add the `I=` fallback path: if `cmd.image_id.is_none() && cmd.image_number.is_some()`, call `self.image_cache().newest_by_image_number(n)` and use that id.
  TDD matrix: (1) positive — `kitty_animate_without_i_or_I_emits_einval_reply`: feed `a=a,s=1`, assert reply contains "EINVAL" + falls back to `i=0` sentinel. (2) negative — `kitty_animate_with_I_fallback_resolves_newest_by_number`: transmit two images with same `I=99`, then `a=a,I=99,s=1`, assert newest image's animation state is mutated (not the older one). (3) regression pin: `kitty_animate_handler_matches_place_error_reply_shape` — structural parity with `kitty_place`'s missing-identifier reply.
  Blocks: §13.3 animation verification — `plans/spec-conformance/section-13-kitty-graphics.md` §13.3's `Animation playback (a=a)` task (line 292) assumes every `a=a` arm produces an observable reply or cache mutation. Silent drop on missing `i=` is a matrix-coverage hole §13.3 can't close without this fix.
  Note: Active work in spec-conformance §13.3 (`not-started`) touches this area. Surfaced by §13.2 impl-hygiene Phase 3 (opus deep-analysis, finding #2 LEAK:swallowed-error) on 2026-04-22.
  Found: 2026-04-22 | Source: impl-hygiene-review

- [ ] `[BUG-08-23][low]` **`parse.rs:253-291` hand-rolls a base64 decoder duplicating the workspace `base64` crate** — found by §13.1 TPR round 0 (gemini F5).
  Repro: `oriterm_core/src/image/kitty/parse.rs:253` defines `fn decode_base64(data: &[u8]) -> Result<Vec<u8>, KittyError>` with its own 40-line loop, even though `base64 = "0.22"` is already at `oriterm_core/Cargo.toml:13` in `[dependencies]` and used elsewhere in the crate (e.g. `osc.rs`, `host_request/mod.rs`).
  Subsystem: `oriterm_core/src/image/kitty/parse.rs`.
  Analysis: DRIFT + WASTE per `.claude/rules/impl-hygiene.md` §Algorithmic DRY. The hand-rolled decoder does a whitespace-filter pre-pass (`filter(|&b| !b.is_ascii_whitespace()).collect::<Vec<u8>>()`) which allocates a second Vec, then loops again to decode. `base64` 0.22's `Engine::decode` handles padding + invalid bytes, and with `GeneralPurposeConfig::new().with_decode_allow_trailing_bits(true)` + custom alphabet tolerates whitespace. Alternative: `base64::engine::general_purpose::STANDARD.decode(data.iter().copied().filter(|b| !b.is_ascii_whitespace()).collect::<Vec<_>>().as_slice())` still has the pre-filter alloc but removes the decode loop. Cleanest: use `base64::read::DecoderReader` over a filtering iterator adapter — no intermediate Vec.
  TDD matrix: preserve the existing `parse::tests::parse_base64_payload_*` invariants (with padding, without padding, invalid chars rejected, whitespace tolerated). Add a round-trip pin: every existing catalog payload decodes to the same bytes before and after the swap.

- [ ] `[BUG-08-20][low]` **`oriterm_core/src/term/mod.rs` is 639 lines, 139 over the 500-line limit** — found by §09A.N post-split file-size sweep; size bumped by the §12.2 TPR round-0 addition of `Term::effective_background` (DECSCNM-aware helper for sixel `SetToBg`), then further grown by BUG-08-12 kitty-keyboard snapshot/live-bits fields (paired snapshot + per-screen live bits + associated constructor entries and field docs — ~55 lines).
  Repro: `wc -l oriterm_core/src/term/mod.rs` prints `639`.
  Subsystem: `oriterm_core/src/term/mod.rs`.
  Analysis: Pre-existing BLOAT — file exceeds the `.claude/rules/code-hygiene.md §File Size` 500-line cap. `Term` struct carries the full terminal state: mode stacks, cursor save, alt-screen swap, image-cache routing, keyboard-mode stack + paired per-screen snapshot fields + live per-screen inactive bits (BUG-08-12), color palette, charsets, C1-7bit / conformance-level fields, tab stops, selection, snapshot buffers, effect sink wiring. Natural split points: cursor save/restore + alt-screen swap helpers into `term/screen_swap.rs`; snapshot/effect plumbing into `term/effects.rs`; leave `Term::new` and the top-level field definitions in `mod.rs`. `effective_background` is a one-screen accessor that naturally stays on the top-level `impl Term` alongside `palette()` / `palette_mut()`.
  TDD matrix: no new tests required — existing `term/tests/` directory (core.rs, modes.rs, osc.rs, etc.) covers the behavior. Split must preserve every test's observable behavior.

- [x] `[BUG-08-19-b][low]` **`oriterm_mux/src/pane/io_thread/mod.rs` is 566 lines, 66 over the 500-line limit** — found by §09A.N post-split file-size sweep.
  Fixed: 2026-04-22 — Duplicate of `BUG-11-14`; resolved together as the broken-window precondition for spec-conformance §13.3. See `BUG-11-14` for the full resolution (run-loop extraction into `run_loop.rs`, select! branch unification via `IDLE_WAKE_CEILING` sentinel). `wc -l oriterm_mux/src/pane/io_thread/mod.rs` now returns 387. All mux tests green.

- [x] `[BUG-08-19][low]` **`crates/vte/src/ansi/handler.rs` is 543 lines, 43 over the 500-line limit** — found while landing §09A.9 (DCS-path DECRQSS / DECRSPS).
  Found: 2026-04-19 | Source: §09A.9 close-out file-size audit.
  Fixed: 2026-04-20 — §09A.N completion. `crates/vte/src/ansi/handler.rs` converted to directory module `crates/vte/src/ansi/handler/` with the single-trait surface preserved via `macro_rules!` items-level macros. `handler/mod.rs` (55 lines) declares `pub trait Handler` and invokes three method-group macros in the trait body: `handler_core_methods!()` (from `core_methods.rs`, 339 lines — upstream/core methods), `handler_vendored_osc_methods!()` (from `vendored_osc_methods.rs`, 123 lines — Section 10.0/10.9 vendored OSC), and `handler_dec_private_methods!()` (from `dec_private_methods.rs`, 111 lines — Section 09A DEC private rect + presentation). Every source file is under the 500-line cap. `macro_rules!` is the canonical Rust mechanism for items-level splitting of a trait body (unlike `include!` which is expression-level-only and rejected inside a trait). No API change: consumers implement exactly one `Handler` trait, no new super-traits. Verified: `cargo test -p vte` green (147 tests), `cargo test -p oriterm_core` green post-split. `crates/vte/README.md` updated with Section 09A vendored-patch entry describing the split mechanism.

- [ ] `[BUG-08-18][low]` **`oriterm_core/src/grid/resize/mod.rs` is 569 lines, 69 over the 500-line limit** — found by /impl-hygiene-review on BUG-08-17 fix.
  Repro: `wc -l oriterm_core/src/grid/resize/mod.rs` prints `569`.
  Subsystem: `oriterm_core/src/grid/resize/mod.rs`.
  Analysis: The file exceeds the `.claude/rules/code-hygiene.md §File Size` 500-line cap. Pre-existing at 564 lines; BUG-08-17 touched it (added 5 lines for DRAWN on synthesized wide-char spacers) without splitting, triggering the "touching a file already over the limit without splitting" BLOAT finding. Natural split points: reflow algorithm (`reflow_cols`, `collect_all_rows`, `apply_reflow_result`) into a `resize/reflow.rs` submodule; the top-level `Grid::resize()` + margin/scroll-region plumbing stays in `mod.rs`.
  TDD matrix: no new tests required — the existing reflow test suite at `resize/tests.rs` (3130 lines) covers the behavior; the split must preserve every test's observable behavior.

- [x] `[BUG-08-17][medium]` **`Cell` lacks a CHARDRAWN-equivalent flag — DECRQCRA (CSI * y) silently skips application-written spaces with default SGR** — found by /tpr-review round 2 on spec-conformance §09A.5 (DECRQCRA).
  Found: 2026-04-19 | Source: /tpr-review round 2 on spec-conformance §09A.5 (DECRQCRA), codex round 2 F1 — `Explicit default spaces are treated as undrawn`.
  Fixed: 2026-04-20 — Resolved across commits `372f448e` (CellFlags::DRAWN infrastructure + 7 write-site integrations + debug_assert hygiene + 24-cell test matrix), `6dd30e5a` (round-0 TPR fix: `Cell::is_empty()` masks DRAWN out via `(self.flags - CellFlags::DRAWN).is_empty()` to keep visual-empty orthogonal to write-history per the fix plan §1.5 Fix Consensus), `6c966f2b` (round-1 TPR fix: 3 row-level regression pins in `grid/row/tests.rs` holding the DRAWN-orthogonality invariant at the reflow consumer surface). Concrete byte-parity restored: "A B" on 1×3 grid → `DCS 1 ! ~ FF5D ST` (was `FF7D`). 7 cell-write sites carry DRAWN (put_char_ascii, put_char_slow main/spacer/leading-spacer, push_zerowidth, DECALN, reflow synthesized spacers); reset paths clear DRAWN via DRAWN-clear template copy (Cell::reset, Row::reset, clear_range, truncate, BCE erase, scroll eviction). 3 rounds of /tpr-review converged cleanly. Full test suite green (1894 lib + 2741 core + 582 spec_chain + 176 teseq). Fix section: `plans/bug-tracker/fix-BUG-08-17.md`.

- [ ] `[BUG-08-16][high]` **Default ANSI palette is Tango, not xterm — yellow looks orange, bright green looks lime, colors over-saturated** — found by manual.
  Repro: Run a colored `ls`, `htop`, neovim with default theme, or `for i in {0..15}; do printf "\e[3${i}m█████\e[0m "; done; echo` — yellow (color 3) renders as mustard/orange, bright green (color 10) renders as lime/light green, colors generally appear more saturated than expected when compared side-by-side with xterm, alacritty (default), wezterm (default), or Windows Terminal Campbell.
  Subsystem: `oriterm_core/src/color/palette/mod.rs:17-80` (`ANSI_COLORS` constant).
  Analysis: The `const ANSI_COLORS: [Rgb; 16]` array is labeled `// Standard xterm ANSI colors (indices 0–15)` but the values are the **Tango** color scheme from GNOME Terminal / Ubuntu (vte-based), not xterm. Concrete divergences observed:
  - Yellow (3): ours `0xc4a000` (Tango "Butter Dark" — mustard/orange); xterm default `0xcdcd00`.
  - Bright Green (10): ours `0x8ae234` (Tango "Chameleon Light" — lime); xterm default `0x00ff00`.
  - Bright Yellow (11): ours `0xfce94f` (Tango "Butter Light"); xterm default `0xffff00`.
  - Bright Red (9), Bright Blue (12), Bright Magenta (13), Bright Cyan (14), White (7), Bright Black (8), Black (0), Red (1), Green (2), Blue (4), Magenta (5), Cyan (6): similar Tango-vs-xterm drift on every entry.

  The comment claim ("Standard xterm") is wrong; the values are Tango. Users who expect xterm/VGA conventions (the broad default across Alacritty / WezTerm / Windows Terminal / iTerm2 / Ghostty / Kitty, all of which ship non-Tango defaults) see the mismatch as "yellow is orange, bright green is lime, colors are over-saturated."

  Fix options (pick one — both require alignment with `oriterm_core/src/color/palette/tests.rs`):
  1. Replace the `ANSI_COLORS` values with the xterm defaults (recommended — matches user mental model and the existing code comment).
  2. Keep Tango values but fix the comment and document the theme choice in `oriterm_core/src/color/palette/mod.rs` module docs; expose both as named themes and make xterm the default. This is the broader fix if the goal is "user-selectable palette themes."

  Secondary cause to rule out during fix: GPU shader sRGB-vs-linear blending. Surface format is `Bgra8UnormSrgb` / `Rgba8UnormSrgb` (see `oriterm/src/gpu/state/`). If the fragment shaders pass sRGB-encoded palette bytes directly into a linear-blend render pass without decoding to linear first, the result is over-saturation that the user might describe as "certain colors seem overly saturated." The palette-values fix alone should be verified visually after landing; if saturation still looks wrong, add a second fix in `oriterm/src/gpu/shaders/*.wgsl` (subpixel_fg.wgsl / color_fg.wgsl) to convert sRGB palette bytes → linear before blending, or switch the render target to the non-sRGB sibling and do gamma in shader. Cross-ref: `plans/bug-tracker/section-06-rendering-perf.md` if the secondary cause is confirmed.

  Found: 2026-04-19 | Source: manual
  Note: Palette values are exercised in `oriterm_core/src/color/palette/tests.rs` — those assertions must be updated in the same commit to stay in sync. Teseq SGR combination tests (`oriterm_core/tests/teseq/sgr/combinations.rs`) and resets tests (`oriterm_core/tests/teseq/sgr/resets.rs`) reference palette indices but not RGB values, so they should not require updates. If roadmap has a theming roadmap section, prefer Fix Option 2 so the switch to xterm-default lands alongside user-theme selection.

- [ ] `[BUG-08-1][medium]` **Audible bell not implemented — `printf '\a'` produces no sound** — found by manual.
  Repro: Run `printf '\a'` in the terminal. Expected: audible beep or system sound. Actual: silence.
  Detail: VTE handler emits `Event::Bell` correctly (`oriterm_core/src/term/handler/mod.rs:112`). App handler in `oriterm/src/app/mux_pump/mod.rs:103` triggers visual tab bar pulse via `ring_bell()` but plays no system sound. `BellConfig` (`oriterm/src/config/bell.rs`) only covers visual bell (animation, duration, color). No audible bell or OS notification exists. Roadmap section 27 plans `behavior.bell = "none" | "visual" | "notification"` but is not yet implemented.
  Subsystem: `oriterm/src/app/mux_pump/mod.rs`, `oriterm/src/config/bell.rs`
  Found: 2026-03-29 | Source: manual
  Note: Active work in roadmap section 27 (command palette) plans bell notification modes.

- [ ] `[BUG-08-4][low]` **vttest LNM key encoding not testable in headless mode** — found by vttest conformance audit.
  Repro: vttest menu 6 sub-item 2 (LineFeed/NewLine mode). vttest sets LNM, presses RETURN, expects CR+LF. Gets bare CR.
  Detail: LNM IS correctly implemented in both VTE handler (`handler/mod.rs:117`) and key encoding (`key_encoding/legacy.rs:165`). The failure is in test infrastructure: `VtTestSession` sends raw `\r` bytes to PTY, bypassing the key encoding layer. Would need VtTestSession to route through key encoding when simulating keypresses -- significant infrastructure change.
  Subsystem: `oriterm_core/tests/vttest/session.rs`
  Found: 2026-04-03 | Source: vttest conformance audit

- [ ] `[BUG-08-5][low]` **DA3 qualifier test fails in vttest menu 6** — found by vttest conformance audit.
  Repro: vttest menu 6 sub-item 6, screen 2. Shows `<13> failed` for a second DA3-related query.
  Detail: DA3 (tertiary device attributes) basic response implemented (`status.rs` responds to `CSI = c` with `DCS ! | 00000000 ST`). Screen 1 now passes. Screen 2 tests a DA3 qualifier/variant that we don't handle. Would require vttest source analysis to identify the specific query.
  Subsystem: `oriterm_core/src/term/handler/status.rs`
  Found: 2026-04-03 | Source: vttest conformance audit

- [ ] `[BUG-08-6][low]` **ENQ/Answerback not implemented** — found by vttest conformance audit.
  Repro: vttest menu 6 sub-item 1 (answerback test). No response displayed.
  Detail: ENQ (0x05) control code not handled in VTE C0 dispatcher. WezTerm implements it (defaults to empty string), Alacritty does not. Would need: (1) add ENQ to VTE C0 dispatch, (2) add handler method to Handler trait, (3) implement in Term. Low priority -- most terminals don't support configurable answerback.
  Subsystem: `crates/vte/src/ansi/dispatch/mod.rs`, `oriterm_core/src/term/handler/mod.rs`
  Found: 2026-04-03 | Source: vttest conformance audit

- [x] `[BUG-08-7][high]` **Kitty delete dispatch has 4 wrong specifier mappings** — found by tpr-review.
  Repro: Send Kitty graphics delete commands with d=a, d=c, d=p, d=r specifiers. Behavior diverges from protocol spec.
  Detail: Verified against official Kitty protocol docs (sw.kovidgoyal.net/kitty/graphics-protocol/) and WezTerm reference. Errors: (1) d=a clears ALL images+placements (spec: visible placements only); (2) d=c uses cursor column (spec: cursor position — cell intersection); (3) d=p does placement-ID deletion (spec: cell position x,y intersection); (4) d=r does cursor-position deletion (spec: image-ID range x≤id≤y). Also missing: d=q/Q (cell+z-index deletion), d=f/F (animation frame deletion).
  Subsystem: `oriterm_core/src/term/handler/image/kitty.rs`
  Found: 2026-04-05 | Source: tpr-review (TPR iteration 4)
  Fixed: 2026-04-21 — Closed by `plans/spec-conformance/section-13-kitty-graphics.md` §13.0.5. All 4 wrong arms (d=a, d=c, d=p, d=r) corrected per kitty graphics-protocol.rst: d=a now removes only visible placements; d=c uses (cursor_col, cursor_row) via `remove_by_position`; d=p uses cell (x-1, y-1) intersection; d=r uses image-id range [x, y]. Missing arms implemented: d=q/Q (cell + z-index via new `ImageCache::remove_placements_at_cell_with_z`); d=f/F (animation-frame deletion via new `ImageCache::{has_extra_animation_frames, remove_animation_frame}` + `remove_image` on uppercase drain); d=n/N (newest by image_number via new `ImageData.image_number` field + `ImageCache::newest_by_image_number`). Regression tests: `oriterm_core/src/term/handler/image/kitty/delete/tests.rs` (29 tests covering all 22 `d=` arms with semantic + negative pins + matrix completeness). Lowercase/uppercase contract pinned by `delete_case_pair_contract_lowercase_keeps_data_uppercase_frees`. All 22 per-specifier catalog rows (`KG-DELETE-{a..N}`) flipped to `verified`.

- [x] `[BUG-08-8][high]` **`kitty.rs` is 476 lines — BLOAT-adjacent; must split before Sections 12 / 13 implementation** — found by continue-roadmap.
  Repro: `wc -l oriterm_core/src/term/handler/image/kitty.rs` prints `476`.
  Detail: `oriterm_core/src/term/handler/image/kitty.rs` sits 24 lines below the 500-line hard limit defined in `.claude/rules/code-hygiene.md` §File Size (also ~26 lines above the ~450-line proactive-split threshold). Sections 12 (Sixel) and 13 (Kitty Graphics) in `plans/spec-conformance/` are explicitly blocked on splitting this file — starting their implementation work on a 476-line file guarantees a 500+ overflow as soon as new per-action code lands, which would violate the hard limit AND force a mid-section refactor at the worst time (feature work mixed with mechanical moves). This bug is orthogonal to `BUG-08-7`: that bug is about protocol-spec correctness of the delete-specifier arms (semantic); this bug is about file size / structural BLOAT (plumbing).
  Proposed fix: extract per-action handlers into submodules at `oriterm_core/src/term/handler/image/kitty/` — `transmit.rs`, `place.rs`, `delete.rs`, `animate.rs`, `query.rs`, `frame_compose.rs`. Keep `kitty/mod.rs` as the dispatch entry point that reads `KittyCommand::action` and routes. Follow the sibling `tests.rs` pattern per `.claude/rules/test-organization.md`. Aim for every split file ≤ 200 lines.
  Subsystem: `oriterm_core/src/term/handler/image/kitty.rs`
  Found: 2026-04-11 | Source: continue-roadmap
  Fixed: 2026-04-21 — Closed by commit `4d46d793` (structural split) + `plans/spec-conformance/section-13-kitty-graphics.md` §13.0.5. Split landed into `oriterm_core/src/term/handler/image/kitty/{mod,transmit,place,delete/mod,store,query,response,frame,animate}.rs`. Every split file ≤ 200 lines (`wc -l` per-file: mod.rs=130, delete/mod.rs=198, store.rs=132, place.rs=91, frame.rs=75, animate.rs=66, transmit.rs=51, response.rs=23, query.rs=13). `delete/` is a directory module per `.claude/rules/test-organization.md` since it has a sibling `delete/tests.rs` added by §13.0.5.

- [ ] `[BUG-08-13][high]` **Numpad keys produce no output — Enter, digits, operators all dead**
  Repro: With numpad active (NumLock on), press numpad Enter / digits / +,-,*,/ / ".". Nothing is sent to the PTY; shell sees no keystrokes.
  Detail: Winit reports numpad keys with `location = KeyLocation::Numpad` and in many cases `Key::Named(...)` (Enter) or `Key::Character("+")`/etc. The dispatcher in `oriterm/src/key_encoding/mod.rs:116` only special-cases numpad when `APP_KEYPAD` is set (line 128 → `legacy::encode_numpad_app`). Outside APP_KEYPAD it falls through to `legacy::encode_legacy`, which is expected to handle normal numpad output (NumpadEnter → CR, digits → their ASCII chars, operators → their ASCII chars). Suspect: `legacy::encode_legacy` does not cover the numpad `Key::Named` / `Key::Character` variants that winit emits with `KeyLocation::Numpad`, or `input.text` is `None` on these events and no fallback fires. Needs verification of which winit variant is delivered for each numpad key and a matching arm in legacy encoding (plus Kitty path at `key_encoding/kitty.rs` — per the dispatch priority in `mod.rs:106-109`, Kitty mode is checked FIRST, so the same gap likely exists there).
  Subsystem: `oriterm/src/key_encoding/legacy.rs`, `oriterm/src/key_encoding/kitty.rs`, `oriterm/src/key_encoding/mod.rs` (dispatch), `oriterm/src/app/keyboard_input/` (upstream KeyInput construction)
  Found: 2026-04-14 | Source: manual
  Note: Related to BUG-08-12 (keyboard-mode / encoding routing) but orthogonal — that bug is about Kitty mode persisting after program exit; this bug is about numpad keys producing no bytes in ANY mode.

- [ ] `[BUG-08-12][high]` **Kitty keyboard mode persists after program exit — shell renders raw CSI u sequences instead of typed characters**
  Repro: Run `notcurses-demo` (or any program that pushes Kitty keyboard protocol modes). After it exits and you return to the shell prompt, typed characters display as raw CSI u escape fragments like `0;1;100u7;1;97u` instead of the actual letters. Terminal is effectively unusable until `reset` is typed blind or the pane is killed. Other terminals (WezTerm, Ghostty) do not exhibit this.
  Detail: The `keyboard_mode_stack` in `Term` retains pushed Kitty keyboard protocol flags after the child program exits. When the shell regains control, `TermMode::KITTY_KEYBOARD_PROTOCOL` is still set, so `key_encoding/mod.rs:118` routes all keypresses through the Kitty CSI u encoder (`key_encoding/kitty.rs`). The shell (bash/zsh) doesn't understand CSI u encoding and displays the raw parameter bytes. Root cause: no mechanism resets the keyboard mode stack when a subprocess terminates without sending `CSI < u` (pop). Programs may crash, be killed via SIGKILL, or simply forget to clean up. RIS (`ESC c`) does clear the stack (`esc.rs:50-51`), but nothing triggers RIS automatically on subprocess exit. Possible fixes: (1) shell integration resets keyboard mode stack on prompt detection, (2) detect when the direct child shell re-emits its prompt and auto-pop any modes the shell didn't push, (3) app-layer heuristic that pops keyboard modes when the pane's foreground process group changes.
  Subsystem: `oriterm_core/src/term/handler/dcs.rs` (push/pop/set), `oriterm_core/src/term/mod.rs` (keyboard_mode_stack), `oriterm/src/key_encoding/mod.rs` (mode check at line 118)
  Found: 2026-04-14 | Source: manual
  Note: Active work in spec-conformance section-17 (Kitty Keyboard) and roadmap section-08 (Keyboard Input) touch this area.

- [ ] `[BUG-08-11][medium]` **`term/tests.rs` combines tests for multiple submodules (2500+ lines) — violates test-organization.md**
  Repro: `wc -l oriterm_core/src/term/tests.rs` prints ~2570. Contains tests for `mod.rs`, `alt_screen.rs`, `image_config.rs`, `snapshot.rs`, and `resize.rs`.
  Subsystem: `oriterm_core/src/term/tests.rs`
  Found: 2026-04-14 | Source: tpr-review
  Reviewer: gemini (TPR-07-001-gemini round 15 during spec-conformance Section 07 close-out)
  Proposed fix: Convert `alt_screen.rs`, `image_config.rs`, `snapshot.rs`, and `resize.rs` to directory modules. Extract their tests from `term/tests.rs` into per-module `tests.rs` files. Verify with `./test-all.sh`.

- [ ] `[BUG-08-14][medium]` **Mode 1042 (urgency hints): flag toggle works but BEL-to-window-manager-hint path is missing**
  Repro: `printf '\x1b[?1042h'` then `printf '\a'` — expected: window manager urgency hint; actual: no effect beyond existing visual bell.
  Detail: `TermMode::URGENCY_HINTS` flag toggles correctly via DECSET/DECRST and DECRQM reports correctly. But no `HostEffect::UrgencyHint` variant exists, and the BEL handler (`oriterm_core/src/term/handler/mod.rs`) does not check mode 1042 to conditionally emit a host-adapter urgency hint. Need: (1) new Effect variant for urgency hint, (2) mode-1042-gated emission in the BEL handler, (3) app-layer host-adapter wiring to platform window-manager urgency API.
  Subsystem: `oriterm_core/src/term/handler/mod.rs` (BEL handler), `oriterm_core/src/effect.rs`, `oriterm/src/app/mux_pump/mod.rs`
  Found: 2026-04-15 | Source: continue-roadmap
  Note: Active work in spec-conformance Section 09 touches mode 1042 flag verification.

- [ ] `[BUG-08-15][medium]` **Mode 1007 (alt-scroll): wheel-to-arrow app-shell apex has no owning roadmap section**
  Repro: `printf '\x1b[?1049h\x1b[?1007h'` then scroll mouse wheel — expected: arrow key sequences sent to PTY; actual: works (Tier-2 gate at `mouse_report/mod.rs:196`), but no spec-conformance section owns end-to-end verification.
  Detail: The `should_translate_wheel_to_arrows(mode, shift_held)` function (extracted by Section 09.1) correctly gates wheel-to-arrow translation. Bridge tests verify the parser→flag→decision path. But catalog row `DEC-ALT-SCROLL` stays `stub` because no roadmap section owns the full app-shell integration apex (actual mouse wheel event → pane write). Section 09 verified the core-layer flag toggle + DECRQM + the bridge cell; the remaining gap is the app-shell integration test that proves a real mouse wheel event flows through `handle_mouse_wheel()` Tier 2 into the PTY.
  Subsystem: `oriterm/src/app/mouse_report/mod.rs` (Tier-2 wheel-to-arrow), catalog `DEC-ALT-SCROLL`
  Found: 2026-04-15 | Source: continue-roadmap
  Note: Section 09.1 bridge cell covers the pure-function decision path; this bug tracks the integration apex.

- [x] `[BUG-08-10][high]` **`Term::image_cache()` / `image_cache_mut()` return the inactive cache in `ALT_SCREEN` mode — alt-mode placements leak into primary after `swap_alt` back** — found by continue-roadmap.
  Found: 2026-04-13 | Source: continue-roadmap
  Fixed: 2026-04-13 — Resolved as part of spec-conformance Section 07 round-6 TPR triage (TPR-07-001 codex+gemini agreement). The deeper issue was that `toggle_alt_common` swapped `image_cache`/`alt_image_cache` field contents but did NOT swap `grid`/`alt_grid`, leaving `Term::resize` pairing the primary grid with whichever cache happened to be in the `image_cache` field regardless of semantic ownership. Root-cause fix: remove the image-cache field swap from `toggle_alt_common` entirely. The fields now carry their semantic contents at all times — `image_cache` = primary, `alt_image_cache` = alt. `image_cache()` / `image_cache_mut()` route by `ALT_SCREEN` mode (mirroring `grid()` / `grid_mut()`) to return the active screen's cache without touching field contents. Regression test `term_resize_routes_each_grid_through_its_own_image_cache` reads the fields directly to prevent future routing inversions; `alt_image_cache_isolation_check` verifies primary/alt placements no longer leak across swaps.

- [x] `[BUG-08-9][medium]` **`Term::set_cell_dimensions` has no production caller — FixedPixels image placements never get updated cell coverage at runtime** — found by tpr-review.
  Found: 2026-04-13 | Source: tpr-review | Reviewer: codex (TPR-07-001-codex during spec-conformance Section 07 review)
  Fixed: 2026-04-13 — Resolved by spec-conformance Section 07.6. Added `PaneIoCommand::SetCellDimensions { width, height }` variant, `MuxPdu::SetCellDimensions`, `MsgType::SetCellDimensions = 0x012B`, `MuxBackend::set_cell_dimensions` trait method with EmbeddedMux + MuxClient + daemon server dispatch implementations. App-layer: 6 pane creation sites now seed metrics at pane spawn (init/create_initial_tab, init/create_handoff_tab, tab_management/new_tab_in_window, pane_ops/split_pane, pane_ops/floating/toggle_floating_pane, window_management/create/create_window), plus `sync_grid_layout` and `handle_dpi_change` broadcast to every pane in the affected window via `App::broadcast_cell_metrics_to_window`. IO-thread handler calls `Term::set_cell_dimensions` and marks grid dirty. Tests: `test_set_cell_dimensions_command_marks_dirty` (mux plumbing), `set_cell_dimensions_missing_pane_is_noop` (EmbeddedMux safety), `roundtrip_set_cell_dimensions` + zero-metrics wire roundtrip (protocol).

- [x] `[BUG-08-3][low]` **vttest.rs exceeds 500-line file size limit (956 lines)** — found by tpr-review.
  Found: 2026-04-03 | Source: tpr-review
  Fixed: 2026-04-03 — Split into `tests/vttest/` directory with per-menu modules (main.rs, session.rs, pty_size.rs, menu1-8.rs). Largest file is 239 lines. All 29 tests pass. 207 snapshots regenerated under new module paths.

- [x] `[BUG-08-2][high]` **Selection highlight cannot be dismissed — sticks after selecting text** — found by manual.
  Found: 2026-03-30 | Source: manual
  Root cause: Every left-click created a `PressAction::New(Selection)` — even single clicks without drag. `handle_release()` only cleared button flags, never the selection. No Escape handling existed.
  Fixed: 2026-03-30 — Two changes: (1) `clear_click_selection()` on mouse-up without drag clears `Char` mode selections (single click), preserving Word/Line selections from double/triple click. (2) Escape key dismisses active selection before falling through to PTY encoding.
