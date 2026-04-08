---
section: "06"
title: "Tack Scenarios: Tools Menu"
status: in-progress
reviewed: true
needs_re_review_after: "04"
re_review_reason: "REWRITTEN by Agent 1 of /review-plan against the final Section 04/05 API and against LIVE empirical inspection of tack v1.08's tools submenu (NOT the guessed tack v6.x menu from the original draft). The pre-rewrite version had FOUR blocking defects: (1) it guessed wrong sub-menu keys for every tool (`d`/`D`/`s`/`r`/`c`/`e`/`g`/`m`/`x`) — none of them match tack v1.08; the real tools menu is `s) ANSI status reports`, `g) ANSI SGR modes`, `c) ANSI character sets`, `h) enable hex output on echo tool`, `e) echo tool`, `r) reply tool`, `p) performance testing`, `i) send reset and init`, `u) test ENQ/ACK handshake`, `d) change debug level`, `q) quit`, `?) help`. (2) It treated `s) ANSI status reports` as a single screen; in reality it's a SEQUENTIAL walker with its own sub-submenu containing DA1/DA2/DA3, multiple DSR variants, DECRQSS, DECRQPSR, mode-status. (3) It tried to test OSC queries via tack, but tack v1.08 has NO OSC query tool — and even if it did, the `PtySession` infrastructure only captures `Event::PtyWrite` and ignores `Event::ColorRequest` / `Event::ClipboardLoad`, so a responder extension to `oriterm_test_support::session` is a prerequisite. (4) Its cap-coverage claim was incomplete: ~19 modern caps (Smulx, Setulc, Sync, BD, BE, PS, PE, Se, Ss, XF, kxIN, kxOUT, Tc, RGB, Cr, Cs, Ms, hs, dsl, fsl, tsl, AX, XT) are declared in `extra/ori_term.info` but tack v1.08 has no tool to probe them. Per CLAUDE.md 'never scope down', the correct response is to EXPAND Section 06 to cover those caps via direct-VTE round-trip tests in `oriterm_core`, not to shrink mission criterion #9. The rewrite (a) adds 06.0 TOOLS_MENU_INVENTORY discovery subsection (parallel to Section 05.0), (b) adds 06.0.b STATUS_REPORTS_INVENTORY nested discovery for the `s) status reports` sub-submenu, (c) adds 06.0.c PtySession OSC-responder framework extension, (d) rewrites 06.1–06.4 against the verified tools menu, (e) adds 06.5 direct-VTE cap xcheck for the ~19 non-tack-reachable caps, (f) replaces the scan-codes / decompile-terminfo stubs with the real tack v1.08 exclusions (echo / reply / hex-output / change-debug-level / performance-testing / send-reset-init), (g) adds 06.6 Mission Criterion Traceability + 06.7 determinism/size/cross-compile verification subsection, (h) adds an Implementation Milestones split (M1 discovery + framework / M2 catalog + direct-VTE xcheck), (i) updates `cap_coverage/section_06.rs` CONTRIBUTION to reflect the two-track coverage. Agent 4 factual correction: PS/PE live in `oriterm_core/src/paste/mod.rs` (not `oriterm`), so only kxIN/kxOUT are genuinely cross-crate; the 06.5 cross-crate stubs are reduced to kxIN/kxOUT only. Section 06 MUST re-run `/review-plan` to flip `reviewed: true` after Agents 2-4 of this pass complete."
goal: "Validate every tool in tack v1.08's `t) tools` submenu AND every terminfo cap that tack v1.08 cannot reach. Tools menu coverage (empirically pinned against tack v1.08): `s) ANSI status reports` (DA1/DA2/DA3 + DSR variants + DECRQM — sequential sub-submenu walker, one scenario per sub-test), `g) ANSI SGR modes` (stable-screen 80-mode table), `c) ANSI character sets` (G0/G1/GL/GR banks, one scenario per bank), `u) test ENQ/ACK handshake` (u8/u9 round-trip). Non-tack-reachable caps (Smulx, Setulc, Sync, BD, BE, PS, PE, Se, Ss, XF, kxIN, kxOUT, Tc, RGB, Cr, Cs, Ms, hs, dsl, fsl, tsl, AX, XT) are covered by direct-VTE round-trip tests in `oriterm_core/src/term/handler/tack_cap_xcheck/` that feed synthetic escape sequences into a `Term` and assert the correct event/handler fires — each test cross-referenced to its cap declaration in `extra/ori_term.info`. PS/PE are tested entirely in `oriterm_core` because `prepare_paste` (the byte-emitting pure function) lives at `oriterm_core/src/paste/mod.rs:11-14`; only kxIN/kxOUT are genuinely cross-crate (emitted by winit focus events from `oriterm/src/app/event_loop_helpers/mod.rs:143 send_focus_event`). Framework prerequisite: Section 06 extends `oriterm_test_support::session::PtyResponder` to listen for `Event::ColorRequest` / `Event::ClipboardLoad` / `Event::ClipboardStore` in addition to `Event::PtyWrite` (the current `PtyResponder` only captures `Event::PtyWrite` and ignores the other three). The extension is in-place on `PtyResponder` itself — it does NOT introduce a new `OscResponder` type, because `Term<PtyResponder>` is the load-bearing field type on `PtySession` and introducing a new listener type would ripple into every consumer. Dual-file layout same as Section 05: const ScenarioSpec / PhaseSpec values + per-scenario parsers in `crates/oriterm_test_support/src/tack_framework/scenarios/{tools_menu_inventory,status_reports_inventory,status_reports,sgr_modes,character_sets,enq_ack}/`; test wrapper `#[test] fn`s in `oriterm_core/tests/tack/tools_menu/`. Only the `tools_menu_inventory` discovery module carries a `tools_` prefix (it scans the literal tools submenu); the nested `status_reports_inventory` has NO `tools_` prefix and lives alongside the matching `status_reports` scenario module so the discovery/scenario pair reads naturally. The scenario modules (`status_reports`, `sgr_modes`, `character_sets`, `enq_ack`) use the short name to match the Sections 04/05 convention and to keep Section 07's hard-pinned `scenarios::character_sets::TACK_TOOLS_G0_DEC_GRAPHICS` path stable."
success_criteria:
  - "`crates/oriterm_test_support/src/tack_framework/scenarios/tools_menu_inventory/{mod, tests}.rs` exists with the empirically pinned tools menu: every key + classification (`Scenario` / `ExcludedInteractive` / `DelegatedToSection` / `MenuMeta`). Mirrors the BEGIN_TESTING_INVENTORY pattern from Section 05.0. All 12 tack v1.08 tools-menu keys (`s`, `g`, `c`, `h`, `e`, `r`, `p`, `i`, `u`, `d`, `q`, `?`) are pre-classified upfront — no punted variants"
  - "`oriterm_core/tests/tack/tools_menu/tools_menu_inventory.rs` exists with `tack_tools_menu_inventory` test that captures the tools menu via insta + asserts the discovered keys match the pinned table (drift = test fail). Uses the new `scenarios::menu_inventory::assert_menu_drift` helper shared ONLY between 06.0 (tools_menu_inventory) and 06.0.b (status_reports_inventory) — Section 05's `begin_testing_inventory::assert_inventory_drift` stays unchanged to avoid destabilizing a green test during cross-section work. The ~15-line duplication is accepted under the stability-over-DRY rule for active cross-section work"
  - "`crates/oriterm_test_support/src/tack_framework/scenarios/status_reports_inventory/{mod, tests}.rs` exists with the pinned sub-submenu walk for `s) ANSI status reports` (DA1/DA2/DA3, DSR variants, DECRQM) — nested discovery, same drift-gate pattern. Module name has NO `tools_` prefix, matching the Sections 04/05 convention for scenario modules and staying symmetric with the shared helper at `scenarios::menu_inventory::`"
  - "`oriterm_core/tests/tack/tools_menu/status_reports_inventory.rs` exists with `tack_status_reports_inventory` test that walks the first N sub-tests via `n` presses, captures each screen via insta, asserts the discovered sub-test labels match the pinned inventory"
  - "`crates/oriterm_test_support/src/session/pty_responder/{mod, tests}.rs` exists (proactive split of `PtyResponder` out of `session/mod.rs`) with the existing `PtyResponder` extended IN-PLACE to listen for `Event::ColorRequest`, `Event::ClipboardLoad`, and `Event::ClipboardStore` in addition to the existing `Event::PtyWrite`. The responder synthesizes the canonical OSC response strings (via the closures those events carry) and exposes them via `take_osc_responses() -> Vec<String>` (for color/load) and `take_clipboard_stores() -> Vec<(ClipboardType, String)>` (for store). `PtySession::drain` / `drain_blocking` write OSC responses back through the PTY automatically via a private `write_osc_responses_back` helper. NO new `OscResponder` type — the extension is in-place on `PtyResponder` because `term: Term<PtyResponder>` in `PtySession` is load-bearing and a new listener type would ripple into every consumer."
  - "`oriterm_core/tests/tack/tools_menu/status_reports.rs` contains one `#[test] fn` per sub-test discovered in STATUS_REPORTS_INVENTORY (at minimum: DA1, DA2, DA3, DSR status, DSR cursor-position, DECRQM probe). Each scenario navigates `[t, s, n, n, ..., n]` to the target sub-test and asserts the captured grid contains the expected response fields via `grid_find_field`. Parser lives in `scenarios::status_reports::parse_status_reports_screen`"
  - "`oriterm_core/tests/tack/tools_menu/sgr_modes.rs` contains `tack_tools_sgr_80x24` (`ScenarioSpec`, NOT `PhaseSpec` — tack's SGR screen is STABLE, not a mid-flow scroll). Menu path `[MenuStep::new(b\"t\", \"tack/tools [q] >\"), MenuStep::new(b\"g\", \"tack/tools/sgr Enter\"), MenuStep::new(b\"\\r\", \"Mode 79\")]`. Parser asserts the 80-mode grid via `grid_has_token(\"Mode\") && grid_has_token(\"0\") && ... && grid_has_token(\"79\")` (or equivalent token sweep)"
  - "`oriterm_core/tests/tack/tools_menu/character_sets.rs` contains at minimum `tack_tools_g0_dec_graphics_80x24` (menu path `[t, c, ), 0]` — select G1 bank `)` then DEC special graphics charset `0`). Parser asserts the rendered DEC graphics characters are present (token-matching on specific chars per how oriterm_core renders SCS — Unicode U+2500-257F box drawing OR the raw ASCII line-drawing forms, whichever empirically matches)"
  - "`oriterm_core/tests/tack/tools_menu/enq_ack.rs` contains `tack_tools_enq_ack_80x24` (menu path `[t, u]`). Parser extracts three fields from the screen via `grid_find_field`: (a) `ENQ sequence from (u9):`, (b) `ACK received:`, (c) `Length of ACK` / `Expected length of ACK` / `Terminating character found in (u8):`. Assertions cross-reference `u8` and `u9` declarations in `extra/ori_term.info`"
  - "`oriterm_core/src/term/handler/tack_cap_xcheck/{mod, tests}.rs` exists as a new sibling submodule of `oriterm_core/src/term/handler/` with direct-VTE round-trip tests for every cap in the 23-entry Track B direct-VTE list that tack v1.08 cannot reach (Smulx, Setulc, Sync, BD, BE, PS, PE, Se, Ss, XF, kxIN, kxOUT, Tc, RGB, Cr, Cs, Ms, hs, dsl, fsl, tsl, AX, XT — 19 escape-sequence-emitting + 4 pure-bool markers). Each test constructs a `Term<RecordingListener>` or `Term<PtyResponder>` (the latter for OSC caps per 06.0.c), feeds the escape sequence declared in `extra/ori_term.info`, and asserts the correct event fires (or the correct mode/state toggles in `TermMode` / grid / cell template for the ones that don't fire events; or the bool cap is present via `parse_declared_caps()` for the 4 pure-bool markers). 21 of 23 Track B caps are tested in-crate in `oriterm_core` (including PS/PE — the byte-emitting `prepare_paste` pure function lives at `oriterm_core/src/paste/mod.rs:11-14`, so the test for PS/PE lives in `tack_cap_xcheck/bracketed_paste.rs` and calls the existing `prepare_paste` helper directly). Only the 2 genuinely cross-crate caps (kxIN/kxOUT) have their real tests in `oriterm/src/app/event_loop_helpers/tests.rs` with doc-only stubs in `tack_cap_xcheck/mod.rs`'s registry pointing to those tests. Parser cross-references each cap to its declaration line in `extra/ori_term.info` via `parse_declared_caps()`. Section 06's TOTAL cap coverage is 27 caps = 4 tack-reachable (Track A: u6/u7/u8/u9) + 23 direct-VTE (Track B, this bullet)"
  - "Meta-test in `tack_cap_xcheck::tests`: `tack_cap_xcheck_covers_every_non_tack_cap` iterates the 23-direct-VTE-cap list (Track B: 19 escape-sequence-emitting + 4 pure-bool markers) and asserts a test (or cross-crate stub) exists for each via a declarative table (`const NON_TACK_CAP_XCHECK_CAPS: &[&str] = &[...]` paired with `const XCHECK_TEST_FNS: &[(&str, fn())]`). If a cap is added to the list without a test, the meta-test fires with a set-diff diagnostic. Total Section 06 coverage is 27 caps = 4 tack-reachable (Track A: u6/u7/u8/u9 via 06.1 + 06.4) + 23 direct-VTE (Track B via 06.5)"
  - "Interactive exclusion stubs (doc-only, no `#[test] fn` bodies): `oriterm_core/tests/tack/tools_menu/{echo_tool,reply_tool,hex_output,change_debug_level,performance_testing,send_reset_init}.rs`. Each stub explains which tack tool it excludes and why, and cross-references the canonical owner (e.g., performance_testing → Section 05 padding scenarios, send_reset_init → begin-testing `i)` stub from Section 05.0)"
  - "Cap-coverage extension: `crates/oriterm_test_support/src/tack_framework/cap_coverage/section_06.rs::CONTRIBUTION.covered` is populated with ALL 27 caps Section 06 exercises — u6/u7/u8/u9 (via tack status reports + ENQ/ACK) plus the 23 non-tack caps (via direct-VTE xcheck, with cross-crate stubs for PS/PE/kxIN/kxOUT). The matching entries are REMOVED from `CONTRIBUTION.exempt` (which should be empty post-Section-06). The stale-exemption negative pin in Section 05.5's `tack_cap_coverage_matrix` fires if a cap appears in BOTH lists"
  - "Mission Criterion Traceability table at the top of the section body maps the tools-menu + direct-VTE coverage back to mission criterion 'Tack tool scenarios cover EVERY automatable tools menu screen in tack v1.08: ...' (by TEXT, never by number). The table shows the two-track approach (tack-reachable vs direct-VTE) so a reader can trace from criterion to subsection"
  - "All scenarios skip cleanly when `tack`/`tic` are unavailable via `ScenarioRunner::available()`. Direct-VTE xcheck tests in 06.5 run UNCONDITIONALLY (no tack dependency — they test `Term` directly)"
  - "All scenarios pass deterministically (10 consecutive reruns clean)"
  - "Debug AND release parity: every test in 06.0 / 06.0.b / 06.0.c / 06.1 / 06.2 / 06.3 / 06.4 / 06.5 passes in BOTH `cargo test` (debug) AND `cargo test --release`. Release-only failures are timing bugs fixed in this section"
  - "Cross-compile gate: `cargo build --target x86_64-pc-windows-gnu -p oriterm_core --tests` AND `cargo build --target x86_64-pc-windows-gnu -p oriterm_test_support --tests` succeed"
  - "Failing-test-first TDD discipline enforced end-to-end: every test in 06.0 / 06.0.b / 06.0.c / 06.1 / 06.2 / 06.3 / 06.4 / 06.5 is written as a failing test BEFORE its implementation lands (mirroring Section 05's TDD ordering rule)"
  - "Sentinel detection inherited from Section 05.0.b: any const value in `scenarios::{tools_menu_inventory,status_reports_inventory,status_reports,sgr_modes,character_sets,enq_ack}` that uses `unverified_menu_key()` or `unverified_anchor()` panics via `assert_no_unverified_sentinels` BEFORE PTY spawn. Until 06.0's discovery output pins the real keys, the 06.1–06.4 consts SHOULD carry sentinels (not guesses) so `cargo test` panics loudly with a referral to 06.0"
  - "`timeout 150 cargo test -p oriterm_core --test tack -- tools_menu` passes (entire tools_menu submodule)"
  - "`timeout 150 cargo test -p oriterm_core -- term::handler::tack_cap_xcheck` passes (direct-VTE xcheck submodule)"
  - "Final `/tpr-review` at 06.N comes back clean — mid-section TPR checkpoints are in addition to, not in place of, the mandatory final pass"
  - "Final `/impl-hygiene-review last commit` at 06.N comes back clean"
  - "Satisfies mission criterion: 'Tack tool scenarios cover EVERY automatable tools menu screen in tack v1.08: ANSI status reports (DA1/DA2/DA3, DSR, DECRQM), SGR mode table (modes 0-79), character set banks (G0/G1/GL/GR), ENQ/ACK handshake (u8/u9). Interactive-only tools (echo, reply, change debug level) have in-code exclusion stubs. For the ~19 modern caps that tack v1.08 cannot reach ... Section 06 provides direct VTE round-trip tests in oriterm_core ...'"
inspired_by:
  - "ori_term Section 04 framework (plans/tack-conformance/section-04-scenario-framework.md)"
  - "ori_term Section 05 catalog pattern (plans/tack-conformance/section-05-test-menu-scenarios.md — 05.0 inventory discovery, 05.1 phase capture, 05.5 cap-coverage matrix)"
  - "ori_term vttest menu6 (oriterm_core/tests/vttest/menu6.rs:walk_menu6_subscreens — DA/DSR response structural assertions, cross-validation target)"
  - "ncurses tack v1.08 source (tools menu items — empirically verified against live tack, NOT assumed)"
  - "ori_term VTE handler tests (oriterm_core/src/term/handler/tests.rs — RecordingListener pattern for asserting events fire correctly; reused by 06.5 direct-VTE xcheck)"
depends_on: ["04", "05"]
depends_on_contract:
  - section: "05"
    contract: "Section 06 consumes Section 05's framework extensions (PhaseSpec + run_phase[_at], the tack_version_supported() gate, the BEGIN_TESTING_INVENTORY discovery pattern generalized into a shared scenarios::menu_inventory helper, and the cap_coverage_matrix CapCoverageContribution extension contract). Section 06 owns `cap_coverage/section_06.rs` and must move ALL 27 exempt entries to `covered` as scenarios and direct-VTE xcheck tests land: 4 tack-reachable caps (u6/u7/u8/u9) via 06.1 status reports + 06.4 ENQ/ACK; AND 23 direct-VTE caps (Smulx/Setulc/Sync/BD/BE/PS/PE/Se/Ss/XF/kxIN/kxOUT/Tc/RGB/Cr/Cs/Ms/hs/dsl/fsl/tsl/AX/XT) via 06.5 direct-VTE xcheck. 27 = 4 + 23; the mission criterion uses the informal '~19' phrasing because only the escape-sequence-emitting caps require direct-VTE round-trip tests — the 4 bool markers (XF/Tc/AX/XT) are validated by terminfo parsing instead. Section 06 MUST land AFTER Section 05's M1 milestone (framework extensions + inventory pattern) but can start before Section 05's M2 milestone (cap-coverage matrix completion) — the frontmatter ordering reflects the safer 'wait for full 05' default, and the 05 cap-coverage matrix is already in place with section_06.exempt populated. The strict prerequisite is 05.0.b's sentinel-detection helper (`assert_no_unverified_sentinels`) which Section 06 inherits for its own const placeholders."
  - section: "07"
    contract: "Section 07 (GPU goldens) references the const path `scenarios::character_sets::TACK_TOOLS_G0_DEC_GRAPHICS` as its `tack_character_sets` golden source (see `section-07-gpu-golden-images.md` 07.4). Section 06 MUST land this const under `crates/oriterm_test_support/src/tack_framework/scenarios/character_sets/mod.rs` — NOT under a `tools_character_sets` or `tools_*` module path. Rationale: Sections 04/05 use single-word scenario module names (`color`, `modes`, `cursor_movement`, `graphic_rendition`) without `test_` or `tools_` prefixes. The `tools_menu_inventory` discovery module keeps its `tools_` prefix because it scans the tools submenu specifically; `status_reports_inventory` (the nested sub-submenu discovery for `s)`) does NOT keep a `tools_` prefix — it lives at `scenarios::status_reports_inventory::` alongside `scenarios::status_reports::` so the pair reads naturally. The scenario modules for tack screens that Section 07 consumes (`character_sets`, `sgr_modes`, `status_reports`, `enq_ack`) MUST use the short name so Section 07's `depends_on_contract` path holds stable and Section 06 matches the existing cross-section naming convention."
third_party_review:
  status: none
  updated: null
sections:
  - id: "06.0"
    title: "TOOLS_MENU_INVENTORY discovery (pin the tools submenu graph)"
    status: complete
  - id: "06.0.b"
    title: "STATUS_REPORTS_INVENTORY nested discovery (sub-submenu under s)"
    status: not-started
  - id: "06.0.c"
    title: "PtyResponder OSC-event extension (framework extension)"
    status: not-started
  - id: "06.1"
    title: "Status reports scenarios (DA/DSR/DECRQM sub-submenu walker)"
    status: not-started
  - id: "06.2"
    title: "SGR mode table scenario (stable-screen, 80 modes)"
    status: not-started
  - id: "06.3"
    title: "Character sets scenarios (G0/G1/GL/GR banks)"
    status: not-started
  - id: "06.4"
    title: "ENQ/ACK handshake scenario (u8/u9 round-trip)"
    status: not-started
  - id: "06.5.a"
    title: "RecordingListener helper promotion (structural prerequisite for 06.5)"
    status: not-started
  - id: "06.5"
    title: "Direct-VTE cap xcheck (non-tack-reachable caps)"
    status: not-started
  - id: "06.6"
    title: "Interactive exclusion stubs (echo/reply/hex/debug/perf/reset)"
    status: not-started
  - id: "06.7"
    title: "Determinism + size matrix + cross-compile verification"
    status: not-started
  - id: "06.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "06.N"
    title: "Completion Checklist (final TPR mandatory)"
    status: not-started
---

# Section 06: Tack Scenarios — Tools Menu

**Status:** Not Started — `reviewed: true` as of Agent 4's final integration pass.

**Rewrite history.** This section was rewritten in Agent 1's pass of `/review-plan` against the verified tack v1.08 tools menu and against Section 04/05's final API. The original draft had four blocking defects (enumerated in the frontmatter `re_review_reason` and detailed in the rewrite contract below) that made it structurally unimplementable. Agent 1 replaced every code sample, added three new subsections (06.0 / 06.0.b / 06.0.c) that land before any tools-menu scenario, added the 06.5 direct-VTE xcheck subsection to cover the ~19 modern caps tack v1.08 cannot reach (per the CLAUDE.md 'never scope down' rule), and renamed the interactive exclusion stubs to match reality.

**Rewrite contract (what changed and why).**

1. **Empirical tools menu pin.** The original draft guessed sub-menu keys (`d`/`D`/`s`/`r`/`c`/`e`/`g`/`m`/`x`). Verified live reality for tack v1.08:
   ```
   Tools Menu
    s) ANSI status reports
    g) ANSI SGR modes (bold, underline, reverse)
    c) ANSI character sets
    h) enable hex output on echo tool
    e) echo tool
    r) reply tool
    p) performance testing
    i) send reset and init
    u) test ENQ/ACK handshake
    d) change debug level
    q) quit
    ?) help
   tack/tools [q] >
   ```
   The rewrite pins this via a 06.0 TOOLS_MENU_INVENTORY discovery + drift gate (mirroring Section 05.0's BEGIN_TESTING_INVENTORY pattern). Every 06.x scenario cites a row from the pinned inventory instead of inventing a key. Scenario-owned keys are `s`, `g`, `c`, `u`; interactive-exclusion keys are `h`, `e`, `r`, `p` (→ Section 05), `i` (→ Section 05 begin-testing stub), `d`; menu meta-keys are `q` and `?`.

2. **Status reports is a sub-submenu walker, not a single screen.** The original draft treated `s)` as one scenario. Verified reality: after `s)` tack displays a line like `(DA) Primary device attributes (CSI 0 c)` and waits for `n` (next) or `q` (quit), stepping through DA1, DA2, DA3, multiple DSR variants, DECRQSS, DECRQPSR, mode-status probes. Each sub-test is a STABLE screen between `n` presses. The rewrite adds a 06.0.b STATUS_REPORTS_INVENTORY nested discovery subsection that pins the sub-test sequence, then 06.1 contains one scenario per sub-test (menu path = `[t, s, n, n, ..., n]` walking to the target index).

3. **OSC query scenarios are STRUCTURALLY blocked without a framework extension.** `oriterm_core/src/term/handler/osc.rs` correctly implements OSC 10 (foreground query), OSC 11 (background query), and OSC 52 (clipboard load/store) — they fire `Event::ColorRequest` and `Event::ClipboardLoad` with a response-formatter closure. BUT `crates/oriterm_test_support/src/session/mod.rs::PtyResponder` ONLY captures `Event::PtyWrite` (see `impl EventListener for PtyResponder` lines ~109–118). Any OSC query routed through a `PtySession` would fire the callback into the void — the response never gets written back to the PTY, so tack (or any OSC-round-trip test) would hang. The rewrite adds 06.0.c: the existing `PtyResponder` is extended IN-PLACE to ALSO listen for `ColorRequest`/`ClipboardLoad`/`ClipboardStore`, invokes the response-formatter closure with a pinned test color or clipboard string, and writes the result back through `PtySession::drain`'s write loop. The type stays `PtyResponder` because `term: Term<PtyResponder>` on `PtySession` is load-bearing — introducing a new listener type would ripple into every consumer. Proactively split out of `session/mod.rs` (currently 459 lines) into `session/pty_responder/{mod, tests}.rs` BEFORE the extension lands so the growth stays under the 500-line limit. This extension is wired into `PtySession::spawn_tack` so every tack session gets it by default, and is tested independently of tack via direct `Term` feeds.

4. **Tack v1.08 has no OSC-query tool, no scan-codes tool, and no decompile-terminfo tool.** The original draft listed `g)` as "generic OSC queries" and `m)` as "scan codes" and `x)` as "decompile terminfo" — none of those exist in tack v1.08. The rewrite:
   - Moves OSC query validation to 06.5 (direct-VTE xcheck, using the 06.0.c responder extension — no tack involved).
   - Deletes the scan-codes and decompile-terminfo stubs (they don't correspond to any real tack tool).
   - Adds new exclusion stubs for the real tack v1.08 interactive tools that CANNOT be automated: `echo_tool`, `reply_tool`, `hex_output`, `change_debug_level`, `performance_testing` (→ Section 05 padding), `send_reset_init` (→ Section 05 begin-testing stub).

5. **~19 modern caps are declared in `extra/ori_term.info` but tack v1.08 has no tool to probe them.** Smulx, Setulc, Sync, BD, BE, PS, PE, Se, Ss, XF, kxIN, kxOUT, Tc, RGB, Cr, Cs, Ms, hs, dsl, fsl, tsl, AX, XT (23 entries total). These 23 caps split across two ownership buckets: **21 caps are covered entirely by `oriterm_core`** — bracketed paste mode in `oriterm_core/src/term/handler/helpers.rs:47,76`; DECSCUSR in `oriterm_core/src/term/handler/dcs.rs:18`; OSC color/clipboard in `oriterm_core/src/term/handler/osc.rs`; status line via OSC title in `oriterm_core/src/term/handler/osc.rs:22`; SGR extensions in `oriterm_core/src/term/handler/sgr.rs`; sync mode in `oriterm_core/src/term/handler/modes.rs:83`; **PS/PE bytes emitted by `oriterm_core/src/paste/mod.rs:11-14`** (the `prepare_paste` pure function that wraps pasted text with `\x1b[200~` / `\x1b[201~` — already unit-tested at `oriterm_core/src/paste/tests.rs:210,243`). The `oriterm`-side clipboard_ops path at `oriterm/src/app/clipboard_ops/mod.rs:176` merely CALLS `paste::prepare_paste`; the byte emission lives in core. Only **2 caps are genuinely cross-crate** — kxIN/kxOUT in the focus-event module `oriterm/src/app/event_loop_helpers/mod.rs:143 send_focus_event` per `extra/ori_term.info:214`, which is OUTBOUND bytes emitted from the application shell in response to winit focus events (the winit dependency keeps these in `oriterm`, not `oriterm_core`). Per CLAUDE.md "never scope down", the correct response is to EXPAND Section 06 to cover all 23 caps. The rewrite adds 06.5: a new `oriterm_core/src/term/handler/tack_cap_xcheck/` sibling submodule with one test per `oriterm_core`-owned cap (21 caps including PS/PE in the new `bracketed_paste` submodule) + cross-crate stubs pointing at `oriterm`-owned tests for kxIN/kxOUT only, each feeding a synthetic escape sequence declared in `extra/ori_term.info` directly into the canonical handler and asserting the correct event fires (or the correct mode/state toggles). A meta-test iterates the cap list and asserts every declared cap has a backing test — if a cap is added without a test, the meta-test fires.

6. **API drift.** The original code samples used `MenuStep { send, wait_for }` (no `or_wait_for`), `ScenarioSpec { id, menu_path, ready_anchor, parser }` (no `screen_id`, no `quit_path`), `outcome.id` for snapshot naming, inline scenario consts in the test target, and `grid.contains` for short markers. The rewrite uses `MenuStep::new` / full literal, `ScenarioSpec` with `screen_id`+`quit_path`, `outcome.snapshot_name()`, workspace-crate scenario consts, and `grid_has_token` / `grid_find_field`. Consistent with Section 05's M3 Codex finding fix.

**The rewrite contract is fixed:** keep the scenario INTENT (validate every tools menu tool and every non-tack cap), replace every code sample with the new API shapes, move every const + parser into the workspace crate, pin the menu via discovery BEFORE writing scenarios, add the OSC-responder framework extension, add direct-VTE xcheck for the 19 non-tack caps, and re-run `/review-plan` against this section to flip `reviewed: true`. Treat the code blocks below as authoritative pseudo-code — they describe what to build, not literally what to type (the FINAL literals are pinned by 06.0's discovery + the cap declarations in `extra/ori_term.info`).

**Goal:** Cover tack v1.08's `t) tools` submenu with structured scenarios, and cover every terminfo cap that tack cannot reach with direct-VTE round-trip tests. Tools differ from the test menu in that they INSPECT what the terminal reports (DA/DSR responses, SGR mode labels, character set bank state, ENQ/ACK handshake) instead of testing fixed protocols. Non-tack caps are validated by feeding their declared escape sequence directly into a `Term` and asserting the event/handler fires. Both tracks are glued together by the cap-coverage matrix in Section 05.5: every cap declared in `extra/ori_term.info` must be in some section's `covered` slice, and the stale-exemption negative pin fires if any cap appears in both `covered` and `exempt`.

**Layout reminder (same as Section 05):** const `ScenarioSpec` / `PhaseSpec` values and parser functions go in `crates/oriterm_test_support/src/tack_framework/scenarios/tools_*/`. Test wrapper `#[test] fn`s go in `oriterm_core/tests/tack/tools_menu/`. The two files share nothing except the import line `use oriterm_test_support::tack_framework::scenarios::{tools_status_reports, ...}::*;` in the test wrapper. Direct-VTE xcheck tests (06.5) live in `oriterm_core/src/term/handler/tack_cap_xcheck/` (sibling submodule of `handler/`, NOT in the test target) because they test `Term` directly and don't go through tack or PtySession.

**Context:** The tools menu reflects how a real human uses tack to debug a terminal: launch tack, hit `t`, pick "show DA response", look at what the terminal sent back. Each tool is a one-shot inspection — there's no test pass/fail inside tack, just a captured report. Our scenario parsers extract the report contents and assert they match what ori_term's terminfo and term handler claim to support. The direct-VTE xcheck in 06.5 is a different validation path for caps tack predates: it's the `Term` equivalent of a doctest for `extra/ori_term.info`, asserting that every declared cap's escape sequence lands in the right handler.

The cross-validation angle: vttest's `menu6` tests assert structurally against DA/DSR/DECRQM responses (`oriterm_core/tests/vttest/menu6.rs:walk_menu6_subscreens`). Section 06's status_reports scenarios should produce the SAME response strings. Section 09 verification diffs the two to catch drift between the vttest path (direct protocol) and the tack path (human-visible report).

**Reference implementations:**
- **Section 04** `plans/tack-conformance/section-04-scenario-framework.md`: framework consumed here.
- **Section 05** `plans/tack-conformance/section-05-test-menu-scenarios.md`: catalog pattern followed here (05.0 discovery + 05.1 scenarios + 05.5 cap-coverage matrix).
- **Section 05.0 `BEGIN_TESTING_INVENTORY`** `crates/oriterm_test_support/src/tack_framework/scenarios/begin_testing_inventory/mod.rs`: the discovery + drift-gate pattern Section 06.0 mirrors. `assert_inventory_drift` is the canonical drift-gate algorithm; 06.0 generalizes it into `scenarios::menu_inventory::assert_menu_drift` so both inventories share the skeleton.
- **Section 04 modes scenario** `crates/oriterm_test_support/src/tack_framework/scenarios/modes/mod.rs`: the verified MenuStep / parser pattern.
- **ori_term vttest menu6** `oriterm_core/tests/vttest/menu6.rs:walk_menu6_subscreens`: existing DA/DSR test logic — 06.1 cross-validates against the same responses.
- **ori_term VTE handler tests** `oriterm_core/src/term/handler/tests.rs`: `RecordingListener` pattern reused by 06.5 direct-VTE xcheck (see `term_with_recorder_sized` helper at line ~47).
- **`extra/ori_term.info`**: the SSOT for what caps ori_term claims; consumed by 06.5 via `parse_declared_caps` (already implemented in Section 05.5's `cap_coverage/mod.rs`).

**Depends on:** Section 04 (framework), Section 05 (inventory discovery pattern, cap-coverage matrix extension contract, framework extensions + sentinel-detection helper).

---

## Mission Criterion Traceability

This section delivers one of the plan's flat-list mission criteria. The two-track approach (tack-reachable + direct-VTE) is explicitly baked into the criterion text (see `00-overview.md`). Every subsection traces upward to the criterion; every part of the criterion traces downward to concrete subsections.

| Mission criterion (text-cited per `00-overview.md`) | Track | Owning subsections | What proves it |
|---|---|---|---|
| "Tack tool scenarios cover EVERY automatable tools menu screen in tack v1.08: ANSI status reports (DA1/DA2/DA3, DSR, DECRQM), SGR mode table (modes 0-79), character set banks (G0/G1/GL/GR), ENQ/ACK handshake (u8/u9)." | tack-reachable | 06.0 (tools menu inventory pin), 06.0.b (status reports nested inventory), 06.1 (status reports scenarios), 06.2 (SGR mode table scenario), 06.3 (character sets scenarios), 06.4 (ENQ/ACK scenario) | The 06.0 drift gate fails when any tools menu key is added/removed; the 06.0.b drift gate fails when the status reports sub-submenu drifts; every `Scenario`-classified key in `TOOLS_MENU_INVENTORY` has a `ScenarioSpec` consumer; every `Scenario`-classified sub-test in `STATUS_REPORTS_INVENTORY` has a `#[test] fn` consumer. Section 09 cross-validates against `oriterm_core/tests/vttest/menu6.rs`. |
| "Interactive-only tools (echo, reply, change debug level) have in-code exclusion stubs." | tack-reachable | 06.6 (interactive exclusion stubs) | Every `ExcludedInteractive`-classified key in `TOOLS_MENU_INVENTORY` has a doc-only stub file in `oriterm_core/tests/tack/tools_menu/`. `cargo clippy -p oriterm_core --tests` produces no warnings on the stubs. |
| "For the 23 modern caps that tack v1.08 cannot reach (Smulx, Setulc, Sync, BD, BE, PS, PE, Se, Ss, XF, kxIN, kxOUT, Tc, RGB, Cr, Cs, Ms, hs, dsl, fsl, tsl, AX, XT — 19 escape-sequence-emitting + 4 pure-bool markers), Section 06 provides direct VTE round-trip tests in `oriterm_core` (21 caps, including PS/PE because the byte-emitting `prepare_paste` lives in `oriterm_core/src/paste/mod.rs`) and in `oriterm` (2 caps: kxIN/kxOUT, emitted by winit focus events in `oriterm/src/app/event_loop_helpers/mod.rs`) that feed synthetic escape sequences and assert the correct event/handler fires or the correct bytes are written." | direct-VTE | 06.5 (direct-VTE cap xcheck) | 06.5's `NON_TACK_CAP_XCHECK_CAPS` table lists every cap; the meta-test `tack_cap_xcheck_covers_every_non_tack_cap` asserts every cap has a backing `#[test] fn` or cross-crate stub; each test cross-references the cap's declaration in `extra/ori_term.info` via `parse_declared_caps()` (Section 05.5 helper). |
| "Section 06 also extends `oriterm_test_support::session::PtyResponder` in-place with OSC event handling so the round-trip test infrastructure covers OSC 10/11/52 for both the tack-reachable AND direct-VTE paths." | framework prereq | 06.0.c (PtyResponder in-place extension + proactive split) | `crates/oriterm_test_support/src/session/pty_responder/{mod, tests}.rs` exists (proactive split out of `session/mod.rs`); `PtyResponder` now handles `ColorRequest`/`ClipboardLoad`/`ClipboardStore` alongside `PtyWrite`; `PtySession::drain` writes OSC responses back automatically; sibling tests pin OSC 10, OSC 11, OSC 52 load, OSC 52 store round-trips through `Term<PtyResponder>` directly without tack being installed (so the tests run cross-platform). NO new `OscResponder` type — the in-place extension preserves the load-bearing `Term<PtyResponder>` field type on `PtySession`. |

**Section 06 cap-coverage contribution target:** Section 06 moves ALL 27 entries from `cap_coverage/section_06.rs::CONTRIBUTION.exempt` to `CONTRIBUTION.covered`:

Track A — **4 tack-reachable caps via 06.1 and 06.4:**
- `u6`, `u7` — via 06.1 status reports (DA1 / DSR extract request/answer strings)
- `u8`, `u9` — via 06.4 ENQ/ACK (u8 = ENQ terminator, u9 = ENQ trigger sequence)

Track B — **23 direct-VTE caps via 06.5 `tack_cap_xcheck` (19 escape-sequence-emitting + 4 bool markers):**
- `Smulx`, `Setulc`, `Sync` — SGR 4:N colon-subparam, SGR 58:2::r:g:b, DECSET/DECRST 2026 (3 sequences, in `oriterm_core`)
- `BD`, `BE` — DECSET/DECRST 2004 bracketed paste on/off (2 sequences, in `oriterm_core`)
- `PS`, `PE` — paste start / end markers produced by `oriterm_core/src/paste/mod.rs::prepare_paste` (2 sequences, in `oriterm_core` — the pure `prepare_paste` function is already unit-tested at `oriterm_core/src/paste/tests.rs:210,243`; 06.5 adds an explicit tack-cap-xcheck entry that calls `prepare_paste("", true, false)` and asserts the bracketed output starts with `\x1b[200~` and ends with `\x1b[201~`, cross-referencing the cap declaration in `extra/ori_term.info:211`)
- `Se`, `Ss` — DECSCUSR reset / set with parameter (2 sequences, in `oriterm_core`)
- `kxIN`, `kxOUT` — focus-in / focus-out markers — outbound bytes produced by `oriterm/src/app/event_loop_helpers/mod.rs:143 send_focus_event` (2 sequences, CROSS-CRATE — the genuine test lives in `oriterm/src/app/event_loop_helpers/tests.rs` because emission requires a winit focus event path owned by the app shell)
- `RGB` — direct-color marker exercised via SGR 38:2::r:g:b (1 sequence, shared with `Tc` bool verification, in `oriterm_core`)
- `Cr`, `Cs` — OSC 112 cursor-color reset / OSC 12 cursor-color set (2 sequences, uses 06.0.c `PtyResponder` OSC extension, in `oriterm_core`)
- `Ms` — OSC 52 clipboard (1 sequence, uses 06.0.c OSC extension, in `oriterm_core`)
- `hs`, `dsl`, `fsl`, `tsl` — OSC 2 via `Event::Title` + status-line state transitions (4 sequences, in `oriterm_core`)
- `XF`, `Tc`, `AX`, `XT` — pure-bool markers (4 entries, no escape sequence — validated by `parse_declared_caps()` presence check, in `oriterm_core`)

27 = 4 tack-reachable + 23 direct-VTE (19 escape-sequence-emitting + 4 bool markers). The mission criterion says "~19" because only 19 caps have direct emission paths requiring VTE round-trip; the 4 bool markers are verified via terminfo parsing. The remaining exemption slice in `cap_coverage/section_06.rs::CONTRIBUTION.exempt` MUST be empty after Section 06 completes — any residual entry is a finding for 06.N's completion checklist review.

**Production-path note (BUG-11-3 interaction).** The open high-priority bug [BUG-11-3 in `plans/bug-tracker/section-11-mux.md`] documents that `oriterm_mux`'s IO-thread event proxy (`oriterm_mux/src/pane/io_thread/event_proxy/mod.rs:150`) drops `Event::ColorRequest` and does NOT invoke the response closure — so running `printf '\e]10;?\e\\'` in the live app produces no reply. Section 06's Track B tests (`Cr`/`Cs`/`Ms`) bypass the mux entirely: they construct `Term<PtyResponder>` directly and exercise the OSC handler path in `oriterm_core`. This means **Section 06 will NOT surface BUG-11-3** — Section 06's green result is NOT evidence that OSC round-trip works end-to-end in the production app. The cap-coverage matrix gate passes when the core handler is correct, which is Section 06's scope. Fixing BUG-11-3 is tracked separately in the bug tracker and is out of Section 06's scope. Do NOT widen Section 06 to add mux-integration OSC tests without filing a prerequisite plan section — that is a distinct work item. Section 06's goal is cap-coverage correctness, not mux plumbing.

---

## Implementation Milestones (M1 / M2)

Section 06 is large (~10 subsections including three new framework extensions, tools-menu catalog, direct-VTE xcheck for 23 caps, and cross-section cap-coverage cleanup). The cognitive load is too wide for a single pass — debugging the `PtyResponder` OSC extension while simultaneously authoring 23 direct-VTE tests multiplies failure surfaces. Mirror Section 05's milestone split.

### M1 — Discovery + framework extensions (06.0 / 06.0.b / 06.0.c)

**Subsections owned:** 06.0 (tools menu inventory), 06.0.b (status reports nested inventory), 06.0.c (OSC responder framework extension).

**M1 completion gate** (every item must be true before starting M2):
- `TOOLS_MENU_INVENTORY` is pinned and `tack_tools_menu_inventory` test is green (drift gate active).
- `STATUS_REPORTS_INVENTORY` is pinned and `tack_status_reports_inventory` test is green (nested drift gate active).
- The new `scenarios::menu_inventory::assert_menu_drift` helper exists and is consumed by `tools_menu_inventory` (06.0) and `status_reports_inventory` (06.0.b). Section 05's `begin_testing_inventory::assert_inventory_drift` stays unchanged — it is intentionally NOT migrated to this helper during Section 06 work. The module doc of `menu_inventory/mod.rs` cross-references Section 05's helper and documents the intentional non-consumer per Codex's midpoint review.
- `session/pty_responder/{mod, tests}.rs` exists (proactive split) with `PtyResponder` extended in-place. Unit tests pin: OSC 10 query → `ColorRequest` event → synthesized reply in `take_osc_responses()`; OSC 11 query → same; OSC 52 ‘c’ load request → `ClipboardLoad` event → synthesized base64 reply in `take_osc_responses()`; OSC 52 ‘c’ store → `ClipboardStore` event → stored tuple in `take_clipboard_stores()`; plus an SSOT regression pin that `Event::PtyWrite` still populates `take_responses()` unchanged, and a negative pin that `Event::Title` / `Event::Bell` / `Event::Wakeup` leave all queues empty. All tests run without tack being installed (they use `Term::new` directly with `PtyResponder` as the listener).
- `PtySession::spawn_tack` wires the OSC responder in by default. An integration test asserts that spawning tack and sending a forged OSC 10 query (via `send_raw`) produces a `ColorRequest` → response round-trip.
- TDD ordering honored: every M1 test written failing-first, then implementation lands.
- **File-size gate:** `wc -l crates/oriterm_test_support/src/session/mod.rs` returns ≤ 450 (post-split), `wc -l crates/oriterm_test_support/src/session/pty_responder/mod.rs` returns ≤ 200 (pre-extension the file is ~40 lines; after the extension ~150 — comfortably under 500). No source file in `crates/oriterm_test_support/src/tack_framework/scenarios/{menu_inventory,tools_menu_inventory,status_reports_inventory}/` exceeds 500 lines.
- `./build-all.sh`, `./clippy-all.sh`, `timeout 150 ./test-all.sh` all green.
- Debug AND release parity: every M1 unit test passes in BOTH `cargo test` and `cargo test --release`.
- `cargo build --target x86_64-pc-windows-gnu -p oriterm_core --tests` AND `cargo build --target x86_64-pc-windows-gnu -p oriterm_test_support --tests` succeed.
- **Recommended TPR checkpoint:** `/tpr-review` after M1. Catches drift-gate regressions, OSC-responder closure-lifetime bugs, cross-platform path issues.

**M1 explicit non-goals:** any 06.1–06.4 tools scenarios (those are M2), 06.5 direct-VTE xcheck (M2), 06.6 exclusion stubs (M2). Do NOT start ANY M2 subsection until the M1 gate is fully green.

### M2 — Catalog + direct-VTE xcheck + cleanup (06.1 – 06.7)

**Subsections owned:** 06.1 (status reports scenarios), 06.2 (SGR mode table), 06.3 (character sets), 06.4 (ENQ/ACK), 06.5.a (RecordingListener helper promotion — structural prerequisite for 06.5), 06.5 (direct-VTE cap xcheck), 06.6 (exclusion stubs), 06.7 (determinism/size/cross-compile).

**M2 parallel-track structure (MANDATORY).** M2 is explicitly structured as two parallel tracks that share zero test targets and zero framework surface. Implementers MAY run them in parallel across sessions because neither track blocks the other. Per Codex midpoint review, treating them as a single sequential pass would inflate cognitive load without any dependency justification.

**M2 Track A — tack-reachable scenarios (06.1 / 06.2 / 06.3 / 06.4)**
- Depends on: M1 (needs TOOLS_MENU_INVENTORY + STATUS_REPORTS_INVENTORY + PtyResponder OSC extension)
- Test target: `oriterm_core/tests/tack/tools_menu/`
- Framework surface: `ScenarioRunner`, `ScenarioSpec`, `PtySession::spawn_tack`, insta snapshots
- Requires tack+tic installed at test time; tests skip cleanly via `ScenarioRunner::available()` on platforms without them
- Runs Linux-primary; Windows ConPTY serializes via `CONPTY_LIFETIME_LOCK`
- M2 Track A gate: every key in `TOOLS_MENU_INVENTORY` classified as `Scenario` has a real `ScenarioSpec`; every sub-test in `STATUS_REPORTS_INVENTORY` classified as `Scenario` has a `#[test] fn`; 06.2 SGR, 06.3 character sets, 06.4 ENQ/ACK all green at 80x24; Track A caps (u6/u7/u8/u9) moved from `CONTRIBUTION.exempt` to `CONTRIBUTION.covered`

**M2 Track B — direct-VTE cap xcheck (06.5.a + 06.5)**
- Depends on: M1 (needs PtyResponder OSC extension for OSC-using caps: Cr/Cs/Ms) AND Section 05.5 (needs `parse_declared_caps()` from `cap_coverage/mod.rs` — ALREADY landed; no action needed besides citing the dependency)
- **Internal prerequisite:** 06.5.a (RecordingListener helper promotion) MUST land as the FIRST Track B commit before any Cap-by-cap work begins — see the 06.5.a sub-subsection for the full 9-step completion gate
- Test target: `oriterm_core/src/term/handler/tack_cap_xcheck/` (sibling submodule of `handler/`, INSIDE the crate not the `tests/` directory)
- Framework surface: `Term<RecordingListener>`, `Term<PtyResponder>`, the `feed` helper from `oriterm_core/src/term/handler/tests.rs`, and the `test_helpers` module introduced by 06.5.a
- Does NOT require tack — tests use `Term` directly; runs on every platform (Linux/macOS/Windows) in full parallel
- M2 Track B gate: 06.5.a's 9-step gate complete AND every cap in `NON_TACK_CAP_XCHECK_CAPS` has a backing `#[test] fn` (or cross-crate stub pointing to an `oriterm` test for kxIN/kxOUT only — PS/PE are in-crate because `prepare_paste` lives in `oriterm_core`); meta-test `tack_cap_xcheck_covers_every_non_tack_cap` green; every test cross-references its cap declaration via `assert_cap_declaration_matches` from `extra/ori_term.info`; Track B caps (19 escape-sequence-emitting + 4 bool markers) moved from `exempt` to `covered`

**Parallelism rationale.** Track A and Track B share zero test targets (`tests/tack/tools_menu/` vs `src/term/handler/tack_cap_xcheck/`), zero framework primitives beyond M1's PtyResponder extension, and zero files in the workspace. A regression in Track A's tack scenarios cannot break a Track B direct-VTE test and vice versa. Tracks may land in either order; implementers choose based on available tool state (tack installed → Track A first; no tack → Track B first, then Track A when tack is available).

**Cross-track coupling point — ONE spot.** Both tracks mutate `cap_coverage/section_06.rs::CONTRIBUTION.covered` and `CONTRIBUTION.exempt` to move their caps. These edits are lockstep with each test landing — do NOT batch all moves at the end. Each `#[test] fn` that lands comes with its matching cap move in the same commit so the stale-exemption negative pin stays green throughout M2. The ordering of Track A vs Track B edits does not matter because the two tracks touch disjoint cap sets (Track A: u6/u7/u8/u9; Track B: the other 23).

**Shared M2 completion gate** (every item must be true for BOTH tracks before invoking 06.N):
- Every `ExcludedInteractive` key in `TOOLS_MENU_INVENTORY` has a doc-only stub in `oriterm_core/tests/tack/tools_menu/` (echo_tool.rs, reply_tool.rs, hex_output.rs, change_debug_level.rs, performance_testing.rs, send_reset_init.rs). `cargo clippy -p oriterm_core --tests` clean.
- `cap_coverage/section_06.rs::CONTRIBUTION.covered` contains all 27 caps (Track A: u6/u7/u8/u9 = 4 tack-reachable + Track B: 23 direct-VTE = 19 escape-sequence-emitting + 4 pure-bool markers). The corresponding entries are REMOVED from `CONTRIBUTION.exempt`. Section 05.5's `tack_cap_coverage_matrix` test is green (the stale-exemption negative pin does not fire).
- Determinism: 10 reruns clean, `--test-threads=1` and `--test-threads=4` both pass (note: Windows `PtySession` tests are serialized by the `CONPTY_LIFETIME_LOCK` per Section 05's Windows ConPTY note; the parallelism gate is Linux/macOS-only for Track A scenarios, but Track B direct-VTE tests run fully parallel everywhere because they don't use `PtySession`).
- Debug AND release parity for every test in BOTH tracks.
- Cross-compile to `x86_64-pc-windows-gnu` succeeds for both `-p oriterm_core --tests` and `-p oriterm_test_support --tests`.
- 06.7 gate subsection complete (determinism + size matrix + cross-compile verification).

**M2 explicit non-goals:** Section 07 GPU goldens for tools-menu scenarios (out of scope — Section 07 picks `character_sets` as a single golden, not every tools scenario). Cross-crate OSC mux plumbing (BUG-11-3 is out of scope per the production-path note above).

**Mandatory final pass:** `/tpr-review` + `/impl-hygiene-review last commit` per 06.N. Findings get FIXED, not deferred.

### Why this split

M1 proves the framework extensions work (tools inventory drift gate, status reports nested inventory, PtyResponder OSC extension) BEFORE writing 20+ scenarios that depend on them. M2 authors the full catalog on top of a stable foundation. M2 is further internally split into two parallel tracks (tack-reachable + direct-VTE) because the cognitive load of debugging 06.1–06.4's tack scenarios while simultaneously authoring 06.5's 23 direct-VTE tests multiplies failure surfaces — treating them as parallel tracks contains each failure mode to its own test target.

---

## 06.0 TOOLS_MENU_INVENTORY discovery

**File(s):**
- `crates/oriterm_test_support/src/tack_framework/scenarios/tools_menu_inventory/{mod, tests}.rs` (NEW — pinned inventory with drift-gate consumer of the shared helper)
- `crates/oriterm_test_support/src/tack_framework/scenarios/menu_inventory/{mod, tests}.rs` (NEW — shared drift-gate algorithm for Section 06's two new inventories. Section 05's `begin_testing_inventory::assert_inventory_drift` stays UNCHANGED — its signature is tightly coupled to `BEGIN_TESTING_INVENTORY` and rewriting it while actively landing Section 06 would destabilize a green test. A future follow-up can consolidate if a fourth consumer appears.)
- `oriterm_core/tests/tack/tools_menu/tools_menu_inventory.rs` (NEW — `#[test] fn tack_tools_menu_inventory` that captures the tools menu via insta + asserts drift)
- `crates/oriterm_test_support/src/tack_framework/scenarios/mod.rs` (add `pub mod menu_inventory;` and `pub mod tools_menu_inventory;`)
- `oriterm_core/tests/tack/tools_menu/mod.rs` (NEW file — declares `pub mod tools_menu_inventory;` and all the other tools_menu submodules as they land)
- `oriterm_core/tests/tack/main.rs` (add `mod tools_menu;`)

**Why this is the FIRST work item.** Every other 06.x subsection cites a key from the tools menu. Section 03's smoke test captured the MAIN menu (`b/m/t/n/l/q/?`), not the tools submenu. The tools submenu has never been pinned under the pinned terminfo. Agent 1's live probe surfaced the real v1.08 tools menu (see the rewrite contract above). The discovery test PINS that reality so 06.1–06.4 can cite verified keys.

**Stability-over-DRY rationale.** Section 05.0's `begin_testing_inventory::assert_inventory_drift` is already landed and green. Its signature takes only `discovered: &BTreeSet<char>` and compares internally against the module's own `BEGIN_TESTING_INVENTORY` const — it is NOT parameterized by a pinned set or a source label. Refactoring it to the generic parameterized form during active Section 06 work would modify a passing test target in `oriterm_core/tests/tack/test_menu/begin_testing_inventory.rs` mid-flight. Per Codex's midpoint review: this is needless blast radius. The correct move is to create `scenarios::menu_inventory::assert_menu_drift` from scratch as a NEW generic parameterized helper for Section 06's two new inventories ONLY. Section 05's helper stays unchanged. A module-level NOTE in `menu_inventory/mod.rs` cross-references `begin_testing_inventory::assert_inventory_drift` and documents that it is intentionally not migrated to keep Section 05's test green during cross-section work. If a future section introduces a fourth drift-gate consumer, that work may consolidate the two helpers at that point — but NOT during Section 06's landing.

**Tasks:**

- [x] **TDD ordering (failing-first).** Write `tack_tools_menu_inventory` BEFORE creating `TOOLS_MENU_INVENTORY`. Phase A: integration test fails on unresolved import. Phase B: stub inventory module with `pub const TOOLS_MENU_INVENTORY: &[ToolsMenuKey] = &[]`, tests compile, drift-gate panics with the symmetric-difference message. Phase C: `INSTA_UPDATE=1` captures the real snapshot. Phase D: read the snapshot, fill in the inventory, re-run without `INSTA_UPDATE`, confirm green.

- [x] **Create the new shared drift-gate helper for Section 06 only.** Create `crates/oriterm_test_support/src/tack_framework/scenarios/menu_inventory/mod.rs` with:
  ```rust
  //! Shared drift-gate algorithm for Section 06's menu inventories.
  //!
  //! Consumed by `tools_menu_inventory` (06.0) and
  //! `status_reports_inventory` (06.0.b). Each caller owns its own
  //! PINNED `BTreeSet<char>` and builds the DISCOVERED set from a
  //! captured grid; this helper is the set-compare + diagnostic-diff
  //! skeleton.
  //!
  //! # Intentional non-consumer
  //!
  //! Section 05's `scenarios::begin_testing_inventory::assert_inventory_drift`
  //! implements the same skeleton against its own pinned
  //! `BEGIN_TESTING_INVENTORY` const. It is intentionally NOT migrated
  //! to this helper during Section 06 work — refactoring a green
  //! Section 05 integration test while Section 06 is actively landing
  //! is needless blast radius (Codex midpoint review, Section 06
  //! Agent 3 review pass). If a future section introduces a fourth
  //! drift-gate consumer, that work may consolidate the two helpers
  //! at that point. Until then, Section 05 keeps its own helper.

  use std::collections::BTreeSet;

  /// Compare a discovered menu-key set against a pinned inventory set
  /// and return `Err(diff_message)` on mismatch.
  ///
  /// `source_label` names the inventory in the diagnostic message
  /// (e.g. `"tools menu"`, `"status reports sub-submenu"`).
  pub fn assert_menu_drift(
      discovered: &BTreeSet<char>,
      pinned: &BTreeSet<char>,
      source_label: &str,
  ) -> Result<(), String> {
      if discovered == pinned {
          return Ok(());
      }
      let only_in_discovered: BTreeSet<&char> = discovered.difference(pinned).collect();
      let only_in_pinned: BTreeSet<&char> = pinned.difference(discovered).collect();
      Err(format!(
          "{source_label} drift detected.\n\
           Discovered: {discovered:?}\n\
           Pinned:     {pinned:?}\n\
           Only in discovered (new keys, add to inventory): {only_in_discovered:?}\n\
           Only in pinned (removed keys, drop from inventory): {only_in_pinned:?}"
      ))
  }

  #[cfg(test)]
  mod tests;
  ```
  Add a sibling `tests.rs` with pins: exact-match returns Ok, drift returns Err with the source label, empty discovered set returns Err naming the missing keys, empty pinned set returns Err naming the discovered keys.

- [x] **Do NOT touch `begin_testing_inventory::assert_inventory_drift`.** Section 05's helper stays unchanged. Section 05's integration test at `oriterm_core/tests/tack/test_menu/begin_testing_inventory.rs` is NOT modified by Section 06. The ~15-line duplication between the two helpers is documented in the module doc of `menu_inventory/mod.rs` above and is acceptable per the stability-over-DRY rationale. This also means Section 05's frontmatter does NOT need a `re_review_reason` bump — Section 06 does not touch Section 05 code.

- [x] **Create `TOOLS_MENU_INVENTORY`.** Write `crates/oriterm_test_support/src/tack_framework/scenarios/tools_menu_inventory/mod.rs`:
  ```rust
  //! Pinned classification of every key on tack's `t) tools` submenu.
  //!
  //! The discovery test in
  //! `oriterm_core/tests/tack/tools_menu/tools_menu_inventory.rs`
  //! captures the live menu via insta and asserts the discovered key
  //! set matches [`TOOLS_MENU_INVENTORY`]. Drift in either direction
  //! (new key in tack output without an inventory entry, or a removed
  //! key) fails the test.
  //!
  //! Empirically verified against tack v1.08 (2026-04-08): the tools
  //! menu exposes `s) ANSI status reports`, `g) ANSI SGR modes`,
  //! `c) ANSI character sets`, `h) enable hex output on echo tool`,
  //! `e) echo tool`, `r) reply tool`, `p) performance testing`,
  //! `i) send reset and init`, `u) test ENQ/ACK handshake`,
  //! `d) change debug level`, `q) quit`, `?) help`.

  /// One row of the tools menu inventory.
  #[derive(Copy, Clone, Debug, Eq, PartialEq)]
  pub struct ToolsMenuKey {
      pub key: char,
      pub label: &'static str,
      pub status: ToolsMenuStatus,
  }

  /// How a tools menu key is handled by the catalog.
  #[derive(Copy, Clone, Debug, Eq, PartialEq)]
  pub enum ToolsMenuStatus {
      /// Has a corresponding `ScenarioSpec` or `PhaseSpec` (once the
      /// relevant 06.x subsection lands).
      Scenario,
      /// Covered by a different section (e.g. send reset and init
      /// overlaps with Section 05's begin-testing `i)` stub).
      DelegatedToSection { section: &'static str },
      /// Cannot be automated — interactive screens that block waiting
      /// for the user to type things. MUST have a doc-only stub in
      /// `oriterm_core/tests/tack/tools_menu/`.
      ExcludedInteractive { stub_file: &'static str },
      /// Menu meta-key — not a tool, but reachable from the prompt
      /// (`q) quit`, `?) help`). The drift gate REQUIRES these be
      /// classified; they do NOT get a `ScenarioSpec` or stub file.
      MenuMeta,
  }

  /// The pinned inventory of tack v1.08's tools submenu.
  pub const TOOLS_MENU_INVENTORY: &[ToolsMenuKey] = &[
      ToolsMenuKey {
          key: 's',
          label: "ANSI status reports",
          status: ToolsMenuStatus::Scenario,
      },
      ToolsMenuKey {
          key: 'g',
          label: "ANSI SGR modes (bold, underline, reverse)",
          status: ToolsMenuStatus::Scenario,
      },
      ToolsMenuKey {
          key: 'c',
          label: "ANSI character sets",
          status: ToolsMenuStatus::Scenario,
      },
      ToolsMenuKey {
          key: 'h',
          label: "enable hex output on echo tool",
          status: ToolsMenuStatus::ExcludedInteractive { stub_file: "hex_output.rs" },
      },
      ToolsMenuKey {
          key: 'e',
          label: "echo tool",
          status: ToolsMenuStatus::ExcludedInteractive { stub_file: "echo_tool.rs" },
      },
      ToolsMenuKey {
          key: 'r',
          label: "reply tool",
          status: ToolsMenuStatus::ExcludedInteractive { stub_file: "reply_tool.rs" },
      },
      ToolsMenuKey {
          key: 'p',
          label: "performance testing",
          status: ToolsMenuStatus::DelegatedToSection { section: "05" },
      },
      ToolsMenuKey {
          key: 'i',
          label: "send reset and init",
          status: ToolsMenuStatus::DelegatedToSection { section: "05" },
      },
      ToolsMenuKey {
          key: 'u',
          label: "test ENQ/ACK handshake",
          status: ToolsMenuStatus::Scenario,
      },
      ToolsMenuKey {
          key: 'd',
          label: "change debug level",
          status: ToolsMenuStatus::ExcludedInteractive { stub_file: "change_debug_level.rs" },
      },
      ToolsMenuKey {
          key: 'q',
          label: "quit",
          status: ToolsMenuStatus::MenuMeta,
      },
      ToolsMenuKey {
          key: '?',
          label: "help",
          status: ToolsMenuStatus::MenuMeta,
      },
  ];

  #[cfg(test)]
  mod tests;
  ```
  The initial commit uses the table above; the discovery test's first run (before `TOOLS_MENU_INVENTORY` is populated) will force the implementer to look at the real captured snapshot and verify the table matches reality. All 12 keys (`s`, `g`, `c`, `h`, `e`, `r`, `p`, `i`, `u`, `d`, `q`, `?`) are pre-classified — no variant additions should be needed during implementation. If live tack v1.08 emits a key NOT in this inventory, the drift gate fires with the unknown key and the implementer adds a classified entry (per broken-window policy, never skip).

- [x] **Create the integration test `oriterm_core/tests/tack/tools_menu/tools_menu_inventory.rs`:**
  ```rust
  //! Discovery test: spawns tack, navigates to the tools submenu,
  //! captures the screen via insta, and asserts every key shown
  //! matches `TOOLS_MENU_INVENTORY`.

  use std::collections::BTreeSet;

  use oriterm_test_support::tack_framework::scenarios::menu_inventory::assert_menu_drift;
  use oriterm_test_support::tack_framework::scenarios::tools_menu_inventory::{
      ToolsMenuKey, TOOLS_MENU_INVENTORY,
  };
  use oriterm_test_support::tack_framework::{MenuStep, ScenarioRunner, ScenarioSpec};

  /// Snapshot-only scenario that lands on the tools menu. The anchor
  /// is the tools-menu prompt produced after sending `t` from the
  /// main menu (empirically verified: `tack/tools [q] >`).
  const TACK_TOOLS_MENU: ScenarioSpec = ScenarioSpec::snapshot_only(
      "tack_tools_menu",
      "tack_tools_menu",
      &[MenuStep::new(b"t", "tack/tools [q] >")],
      "tack/tools [q] >",
  );

  #[test]
  fn tack_tools_menu_inventory() {
      if !ScenarioRunner::available() {
          eprintln!("tack/tic unavailable, skipping");
          return;
      }
      let outcome = ScenarioRunner::run(&TACK_TOOLS_MENU);
      insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);

      let discovered: BTreeSet<char> = collect_menu_keys(&outcome.grid_text);
      let pinned: BTreeSet<char> =
          TOOLS_MENU_INVENTORY.iter().map(|k| k.key).collect();
      if let Err(msg) = assert_menu_drift(&discovered, &pinned, "tools menu") {
          panic!("{msg}\nGrid:\n{}", outcome.grid_text);
      }
  }

  /// Scan a captured grid for `<key>) <label>` entries. Same algorithm
  /// as `begin_testing_inventory.rs::collect_menu_keys` — if this is
  /// the THIRD consumer (06.0.b is the second), extract to a shared
  /// helper per algorithmic-DRY.
  fn collect_menu_keys(grid: &str) -> BTreeSet<char> {
      grid.lines()
          .filter_map(|line| {
              let trimmed = line.trim_start();
              let mut chars = trimmed.chars();
              let key = chars.next()?;
              if chars.next() == Some(')')
                  && (key.is_ascii_alphabetic()
                      || key.is_ascii_digit()
                      || "/?".contains(key))
              {
                  Some(key.to_ascii_lowercase())
              } else {
                  None
              }
          })
          .collect()
  }
  ```

- [x] **Scoped `collect_menu_keys` extraction for Section 06 only.** Section 06's two new inventories (tools_menu_inventory + status_reports_inventory) both need the `<key>) <label>` scanner. Extract `collect_menu_keys` to `scenarios::menu_inventory::collect_menu_keys` alongside `assert_menu_drift` — consumed ONLY by the two Section 06 inventories. Section 05's `begin_testing_inventory.rs::collect_menu_keys` (if it exists as a private helper in the integration test) stays unchanged — DO NOT update Section 05's integration test to call the new canonical. Rationale: Codex's midpoint review flagged refactoring Section 05 mid-Section-06-landing as needless blast radius. The ~15 lines of duplication between Section 05's and Section 06's scanners are accepted under the stability-over-DRY rule. The `menu_inventory/mod.rs` module doc cross-references Section 05's helper and documents the intentional non-consumer.

- [x] **Semantic pin: drift gate cannot be silently disabled.** Add a unit test in `scenarios::tools_menu_inventory::tests` that constructs a synthetic `discovered` with an extra key not in `TOOLS_MENU_INVENTORY`, calls `assert_menu_drift`, and asserts the error message names both sets. Mirror the pattern from `begin_testing_inventory::tests::begin_testing_inventory_drift_gate_pin`. Add a positive pin too (`pinned_inventory_is_non_empty` — catches regression to the empty-array start state).

- [x] **Capture the snapshot via `INSTA_UPDATE=1 timeout 150 cargo test -p oriterm_core --test tack -- tools_menu::tools_menu_inventory`.** Read the captured grid at `oriterm_core/tests/tack/tools_menu/snapshots/tack__tools_menu__tools_menu_inventory__tack_tools_menu_80x24.snap`. Confirm every ToolsMenuKey entry matches the real menu. `q` and `?` are pre-classified as `MenuMeta`; if any OTHER key surfaces that isn't in the pinned inventory, the drift gate fires with a diagnostic naming the new key — add a classified entry and re-run (broken-window policy: never skip an unknown key).

- [x] **Debug + release parity.** Run the discovery test in BOTH profiles: `timeout 150 cargo test -p oriterm_core --test tack -- tools_menu::tools_menu_inventory` and `timeout 150 cargo test -p oriterm_core --test tack --release -- tools_menu::tools_menu_inventory`. Any release-only failure is a timing bug — fix in 06.0, never defer.

- [x] **Output of 06.0:** the tools menu discovery test passes, `TOOLS_MENU_INVENTORY` is the SSOT for keys used by 06.1–06.4, every later subsection cites a row from the inventory instead of inventing a key, and `scenarios::menu_inventory::{assert_menu_drift, collect_menu_keys}` is the drift-gate + key-scanner home for Section 06's two new inventories. Section 05's helpers remain unchanged; a future refactor may consolidate if a fourth consumer appears.

- [x] **[STYLE] Update `scenarios/mod.rs` module doc to match reality.** Doc bullet for Section 06 rewritten to enumerate the final submodules (`menu_inventory` shared helper + `tools_menu_inventory` landed in 06.0; `status_reports_inventory`, `status_reports`, `sgr_modes`, `character_sets`, `enq_ack` land in their respective later subsections). `pub mod menu_inventory;` and `pub mod tools_menu_inventory;` declarations added in 06.0. The remaining `pub mod` declarations are added by their owning subsections (one `pub mod` line per module, landed with its body) to keep commits bisectable. `cargo check -p oriterm_test_support` clean.

---

## 06.0.b STATUS_REPORTS_INVENTORY nested discovery

**File(s):**
- `crates/oriterm_test_support/src/tack_framework/scenarios/status_reports_inventory/{mod, tests}.rs` (NEW — pinned nested inventory for the `s) ANSI status reports` sub-submenu walker)
- `oriterm_core/tests/tack/tools_menu/status_reports_inventory.rs` (NEW — `#[test] fn tack_status_reports_inventory` that walks the sub-submenu)
- `crates/oriterm_test_support/src/tack_framework/scenarios/mod.rs` (add `pub mod status_reports_inventory;`)
- `oriterm_core/tests/tack/tools_menu/mod.rs` (add `pub mod status_reports_inventory;`)

**Why nested discovery.** `s) ANSI status reports` is NOT a single screen. Verified behavior: after `s)`, tack displays a line like `(DA) Primary device attributes (CSI 0 c)` and waits for `n` (next sub-test) or `q` (quit the sub-submenu). The sub-submenu walks DA1, DA2, DA3, multiple DSR variants, DECRQSS, DECRQPSR, and mode-status probes. Each sub-test is a STABLE screen between `n` presses. 06.1 needs ONE scenario per sub-test; 06.0.b pins the sub-test sequence so 06.1 can walk to the right one.

**Discovery mechanism.** Unlike the flat inventories in 05.0 and 06.0 (which capture a single screen and scan for `<key>) <label>` rows), the status reports sub-submenu is sequential. The discovery test walks it by sending `n` repeatedly, captures each screen via insta (one snapshot per sub-test — naming: `tack_status_reports_01_da1_80x24.snap`, `tack_status_reports_02_da2_80x24.snap`, ...), and assembles a `DISCOVERED_SEQUENCE: Vec<String>` of the first-line text of each captured screen. The pinned `STATUS_REPORTS_INVENTORY` is a `&[StatusReportsSubTest]` listing the expected sub-test names in order. Drift-gate comparison is sequence equality, NOT set equality — order matters because 06.1 uses sub-test INDEX to build its menu path `[t, s, n, n, ..., n]`.

**Tasks:**

- [ ] **TDD ordering (failing-first).** Write `tack_status_reports_inventory` BEFORE creating `STATUS_REPORTS_INVENTORY`. Same pattern as 06.0: stub empty array, drift gate panics, read the captured snapshots, fill in the table.

- [ ] **Create `STATUS_REPORTS_INVENTORY`.** Write `crates/oriterm_test_support/src/tack_framework/scenarios/status_reports_inventory/mod.rs`:
  ```rust
  //! Pinned sequence of sub-tests in tack's `s) ANSI status reports`
  //! sub-submenu.
  //!
  //! Unlike the flat menu inventories, this is a SEQUENCE. Tack walks
  //! sub-tests one at a time via `n` (next). The index into this
  //! sequence is what 06.1 scenarios use to build their menu path
  //! `[t, s, n, n, ..., n]`.

  #[derive(Clone, Debug, Eq, PartialEq)]
  pub struct StatusReportsSubTest {
      /// 0-based index in tack's sub-submenu walk. `0` means "no `n`
      /// presses needed after `s`" (the first sub-test displays on
      /// entry).
      pub index: usize,
      /// Tack's header label for this sub-test, as captured in the
      /// discovery snapshot. Used by the drift gate.
      pub label: &'static str,
      /// Short mnemonic for the sub-test (`"da1"`, `"da2"`, `"da3"`,
      /// `"dsr_status"`, `"dsr_cpr"`, `"decrqm"`). Used by 06.1 to
      /// name its `#[test] fn`s and `screen_id`s.
      pub mnemonic: &'static str,
      /// The protocol cap this sub-test exercises (if any), used by
      /// the 06.1 parser to decide which response-field tokens to
      /// extract via `grid_find_field`.
      pub cap_target: Option<&'static str>,
  }

  /// Pinned sequence. The FIRST run of the discovery test will force
  /// the implementer to populate this from the captured snapshots.
  pub const STATUS_REPORTS_INVENTORY: &[StatusReportsSubTest] = &[
      // Populated after `INSTA_UPDATE=1` captures the first N sub-test
      // screens. Expected entries (empirically observed at the time
      // the rewrite was authored, to be verified during implementation):
      //   index 0: "(DA) Primary device attributes (CSI 0 c)" → da1 / u6+u7
      //   index 1: "(DA2) Secondary device attributes ..."     → da2
      //   index 2: "(DA3) Tertiary device attributes ..."      → da3
      //   index 3: "(DSR) Device status report ..."            → dsr_status
      //   index 4: "(CPR) Cursor position report ..."          → dsr_cpr
      //   index 5: "(DECRQM) Mode query ..."                   → decrqm
      //   ... (up to whatever tack v1.08 actually emits)
  ];

  #[cfg(test)]
  mod tests;
  ```

- [ ] **Create the integration test `oriterm_core/tests/tack/tools_menu/status_reports_inventory.rs`:**
  ```rust
  //! Discovery test for the `s) ANSI status reports` sub-submenu.
  //! Walks tack's sub-test sequence via `n` presses, captures each
  //! sub-test screen via insta, and asserts the discovered labels
  //! match `STATUS_REPORTS_INVENTORY`.

  use oriterm_test_support::tack_framework::scenarios::status_reports_inventory::{
      StatusReportsSubTest, STATUS_REPORTS_INVENTORY,
  };
  use oriterm_test_support::tack_framework::{MenuStep, ScenarioRunner, ScenarioSpec};

  /// Scenario that lands on the status reports sub-submenu, then
  /// captures + advances + captures + advances... up to the length of
  /// the pinned inventory. The parser records each sub-test's first
  /// line as the discovered label.
  // Authoritative implementation decision (Agent 2 review): use the
  // FRESH-SPAWN LOOP — one tack spawn per sub-test. Each iteration
  // constructs a `ScenarioSpec` with `menu_path = [t, s, n*sub.index]`
  // and runs it through the stock `ScenarioRunner::run`. No new
  // framework primitive is required. The rationale:
  //
  // 1. Fresh-spawn loop reuses the existing `ScenarioSpec` contract
  //    and `ScenarioRunner::run` — zero new framework cost.
  // 2. A single-walk primitive would require a new `PhaseSpec`-like
  //    sequential-capture type (~80 lines of framework) plus its own
  //    test surface, for one consumer.
  // 3. Runtime cost estimate: tack startup is ~500 ms, sub-test count
  //    is ~6-10. Worst case ~5 s per discovery run. The discovery
  //    test runs once per `test-all.sh` invocation, acceptable.
  // 4. If sub-test count grows past ~15 or runtime exceeds 30 s
  //    (measured, not guessed), REVISIT — at that point a single-
  //    walk primitive pays for itself. Until then, fresh-spawn.
  //
  // Windows ConPTY note: each spawn acquires CONPTY_LIFETIME_LOCK
  // per Section 05's serialization contract. Fresh-spawn increases
  // the total tack spawns on Windows by ~10 per test-all.sh run.
  // Still well within the Windows CI budget.
  #[test]
  fn tack_status_reports_inventory() {
      if !ScenarioRunner::available() {
          eprintln!("tack/tic unavailable, skipping");
          return;
      }
      // Pseudocode: for each pinned sub-test, construct a scenario
      // with menu_path = [t, s] + [n; sub.index] and ready_anchor =
      // unique prompt for that sub-test's screen. Run it, capture the
      // first line, assert it matches `sub.label`.
      for sub in STATUS_REPORTS_INVENTORY {
          let scenario = build_status_reports_sub_scenario(sub);
          let outcome = ScenarioRunner::run(&scenario);
          insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);
          let first_line = outcome.grid_text.lines().next().unwrap_or("").trim();
          assert!(
              first_line.contains(sub.label) || outcome.grid_text.contains(sub.label),
              "status reports sub-test {idx} drift: pinned label {label:?} not \
               found in captured grid:\n{grid}",
              idx = sub.index,
              label = sub.label,
              grid = outcome.grid_text,
          );
      }
  }

  fn build_status_reports_sub_scenario(
      sub: &'static StatusReportsSubTest,
  ) -> ScenarioSpec {
      // Authoritative dispatch (Agent 2 review): CONST TABLE indexed
      // by sub.index. Declare `STATUS_REPORTS_SUB_SCENARIO_TABLE:
      // &[ScenarioSpec]` as a module-level const with one entry per
      // pinned sub-test. The entry for index N has menu_path
      // `STATUS_REPORTS_SUB_MENU_PATHS[N]` where
      // STATUS_REPORTS_SUB_MENU_PATHS is a parallel const array of
      // `&'static [MenuStep]` slices, each literally spelled out
      // (e.g., `&[MenuStep::new(b"t", ...), MenuStep::new(b"s", ...)]`
      // for index 0, `&[t, s, MenuStep::new(b"n", ...)]` for index 1,
      // etc.). The arrays stay in lockstep via a compile-time
      // `const _: () = assert!(STATUS_REPORTS_INVENTORY.len() ==
      // STATUS_REPORTS_SUB_SCENARIO_TABLE.len())` assertion.
      //
      // Rationale: match arm dispatch scales poorly past ~8 cases
      // (dead-code warnings on the default arm, harder to cross-
      // reference with the inventory). Const table scales to any
      // length, keeps the inventory and dispatch in the same file,
      // and the compile-time length assertion catches drift
      // immediately. Macro generation would be equivalent but adds
      // machinery for no gain at 10 entries.
      STATUS_REPORTS_SUB_SCENARIO_TABLE[sub.index]
  }
  ```

- [ ] **Anchor strategy.** The pre-existing-anchor guard prevents reusing the same anchor across steps. After the first `s`, tack displays the first sub-test (e.g., `"(DA) Primary device attributes"`). The second step's `n` must wait for something unique to the SECOND sub-test — not just `"tack/tools/status"` which would be present on every sub-test. The ready_anchor for each sub-test is the unique portion of its label (e.g., `"Primary device attributes"` for DA1, `"Secondary device attributes"` for DA2, etc.). If tack's prompt format doesn't include enough unique text, fall back to `(DSR)` / `(DECRQM)` style paren markers which ARE unique.

- [ ] **Debug + release parity.** Run the discovery test in BOTH profiles.

- [ ] **Output of 06.0.b:** the status reports sub-submenu is pinned, 06.1 can build scenarios per sub-test using `sub.index` as the `n`-press count, and the drift gate fires if tack changes the order or adds/removes sub-tests.

---

## 06.0.c PtyResponder OSC-event extension (framework extension)

**File(s):**
- `crates/oriterm_test_support/src/session/mod.rs` (EXTEND — add `ColorRequest` / `ClipboardLoad` / `ClipboardStore` handling to the existing `PtyResponder::send_event` match in-place. Do NOT introduce a new `OscResponder` type — the load-bearing field `term: Term<PtyResponder>` at `session/mod.rs:136` forces the listener type to stay `PtyResponder`. Adding a second type would require threading a new generic through every `PtySession` consumer, which is the wrong kind of churn.)
- `crates/oriterm_test_support/src/session/tests.rs` (EXTEND or CREATE sibling — sibling unit tests pinning OSC 10, OSC 11, OSC 52 load, OSC 52 store round-trip behavior against `Term<PtyResponder>` directly, no PtySession, no tack — runs cross-platform in CI without tack installed)
- `oriterm_core/src/event/mod.rs` (REFERENCE — the `ColorRequest` / `ClipboardLoad` / `ClipboardStore` variants already exist; no changes needed to oriterm_core)

**Why this exists (Blocker 2 from Agent 1's pre-rewrite analysis).** `oriterm_core/src/term/handler/osc.rs` correctly implements:
- OSC 10/11/12 query (`osc_dynamic_color_sequence` at line ~98) — fires `Event::ColorRequest(index, Arc<dyn Fn(Rgb) -> String ...>)` with a closure that formats the canonical response escape sequence.
- OSC 52 load (`osc_clipboard_load` at line ~145) — fires `Event::ClipboardLoad(ClipboardType, Arc<dyn Fn(&str) -> String ...>)` with a closure that formats the base64 response.
- OSC 52 store (`osc_clipboard_store` at line ~114) — fires `Event::ClipboardStore(ClipboardType, String)`.

BUT the test-side `PtyResponder` (`crates/oriterm_test_support/src/session/mod.rs:109–118`) only handles `Event::PtyWrite`:
```rust
impl EventListener for PtyResponder {
    fn send_event(&self, event: Event) {
        if let Event::PtyWrite(data) = event {
            self.responses.lock().expect("PtyResponder mutex poisoned").push(data);
        }
    }
}
```
Every `ColorRequest` / `ClipboardLoad` / `ClipboardStore` event fires into the void. Any OSC query test that goes through `PtySession` would send the query, ori_term's handler would fire the callback, and the response would go nowhere — the test would hang on `wait_for` waiting for a reply that never gets written.

**Fix.** Extend `PtyResponder` in-place so its `impl EventListener for PtyResponder::send_event` handles FOUR event variants instead of one:

1. `Event::PtyWrite(data)` — existing behavior, push `data` into the `responses` queue. Unchanged.
2. `Event::ColorRequest(index, formatter)` — call `formatter(pinned_color)` with a deterministic test color (e.g., `Rgb { r: 0xab, g: 0xcd, b: 0xef }`, or if the index refers to a palette slot, a `Rgb` derived from the index). Push the resulting escape sequence into a NEW `osc_responses: Arc<Mutex<Vec<String>>>` queue.
3. `Event::ClipboardLoad(clipboard_type, formatter)` — call `formatter(pinned_clipboard_text)` with a deterministic test string. Push the resulting escape sequence into the `osc_responses` queue.
4. `Event::ClipboardStore(clipboard_type, text)` — push the `(clipboard_type, text)` tuple into a NEW `clipboard_stores: Arc<Mutex<Vec<(ClipboardType, String)>>>` side channel the caller inspects directly.

All other `Event` variants are ignored (`_ => {}`) — including `Bell`, `Title`, `IconName`, `ResetTitle`, `ResetIconName`, `Wakeup`, `ChildExit`, `Cwd`, `CommandComplete`, `MouseCursorDirty`, `CursorBlinkingChange`. None of those need round-trip responses, and 06.5 tests for `hs/dsl/fsl/tsl` (which fire `Event::Title`) query `term.title()` directly rather than the responder, so `PtyResponder` doesn't need to track them.

`PtyResponder` gains two new accessor methods alongside the existing `take_responses`:
- `take_osc_responses(&self) -> Vec<String>` — drain the OSC response queue (for DA-style write-back).
- `take_clipboard_stores(&self) -> Vec<(ClipboardType, String)>` — drain the clipboard-store side channel.

**Wiring.** `PtySession::drain` / `drain_blocking` at `session/mod.rs:258` already construct `PtyResponder::new()`. No type change — the extended `PtyResponder` is the same type with more responsibilities. `PtySession` gains a new private helper `write_osc_responses_back(&mut self)` that calls `self.term.event_listener().take_osc_responses()` and writes each entry via `self.writer.write_all(...)`. `drain` and `drain_blocking` call this after every successful event loop iteration so the OSC round-trip completes automatically, transparent to scenarios.

**SSOT rationale.** `PtyResponder` is the canonical test-side `EventListener` for `oriterm_test_support`. Adding a second type (`OscResponder` or `OscAwareResponder`) would create two listener implementations — violating SSOT and forcing every consumer of `session.term()` to know which type to expect. The in-place extension keeps `Term<PtyResponder>` as the single-source-of-truth type and grows `PtyResponder`'s responsibilities to match its single purpose ("the test-side event collector for OSC and PtyWrite round-trip").

**Cross-cutting concern: don't break Section 04/05.** The existing 198 vttest tests + 18 Section 05 tack tests rely on the DA/DSR response path. The OSC responder MUST NOT interfere with it. Verify by running `timeout 150 cargo test -p oriterm_core --test vttest` + `timeout 150 cargo test -p oriterm_core --test tack` after the extension lands — all existing tests green, no new failures. Any regression is a Section 06.0.c blocker.

**TDD ordering (mandatory).** Per the TDD rule:
1. Write `pty_responder_captures_color_request` failing test: constructs `Term<PtyResponder>` directly (no PtySession), feeds OSC 10 query (`\x1b]10;?\x07`), asserts `responder.take_osc_responses()` contains a formatted reply matching the canonical OSC 10 response format (`"\x1b]10;rgb:abab/cdcd/efef\x1b\\"` or the BEL-terminated equivalent — match whatever `oriterm_core/src/term/handler/osc.rs` actually produces).
2. Extend `PtyResponder::send_event` with the `ColorRequest` arm — step 1 passes.
3. Write `pty_responder_captures_clipboard_load` failing test — repeat for OSC 52 load (`\x1b]52;c;?\x07`), assert `take_osc_responses()` contains the base64-encoded response.
4. Extend `PtyResponder::send_event` with the `ClipboardLoad` arm — step 3 passes.
5. Write `pty_responder_captures_clipboard_store` failing test — feed OSC 52 `c;<base64>` store request, assert `take_clipboard_stores()` contains the `(Clipboard, decoded_text)` tuple.
6. Extend `PtyResponder::send_event` with the `ClipboardStore` arm — step 5 passes.
7. Write `pty_responder_still_captures_pty_write` regression pin — feeds a DA query (`\x1b[c`), asserts the PtyWrite queue from `take_responses()` still contains the DA response (the extension MUST NOT break the existing PtyWrite path — this is the SSOT preservation test).
8. Write `pty_responder_ignores_non_round_trip_events` negative pin — fires `Event::Title("test")`, `Event::Bell`, `Event::Wakeup` etc. at the responder and asserts all three queues remain empty. Locks in the "all other variants are `_ => {}`" contract so a future extension that adds a variant can't silently populate the wrong queue.
9. Write `pty_session_drain_writes_osc_responses_back` integration test: construct a `PtySession` (via `PtySession::spawn_command` with a no-op echo child, no tack), send a forged OSC 10 query via `session.send_raw`, call `session.drain_blocking` with a short deadline, inspect the PTY master input buffer to confirm the response bytes were written back through `self.writer`. This test does NOT need tack.
10. Land the `drain`/`drain_blocking` OSC response-flush wiring — step 9 passes.
11. Run debug and release — both green.
12. Run `timeout 150 cargo test -p oriterm_core --test vttest` + `timeout 150 cargo test -p oriterm_core --test tack` — existing 198 vttest snapshots and 18 Section 05 tack tests pass unchanged (no regression from the responder extension).

**Sibling test file organization — PROACTIVE SPLIT MUST HAPPEN FIRST.** Verified file size: `crates/oriterm_test_support/src/session/mod.rs` is **459 lines today** (`wc -l` at 2026-04-08). Pre-extension `PtyResponder` is roughly 30 lines of that (lines 90–118 per the source) with no sibling `tests.rs`. 06.0.c's extension grows `PtyResponder` to ~100 lines and adds 6+ direct unit tests — which would push `session/mod.rs` PAST the 500-line hard limit per `.claude/rules/impl-hygiene.md` and `.claude/rules/code-hygiene.md`. Landing the split FIRST is non-negotiable: the split lands as its own commit before any `send_event` extension code, so `session/mod.rs` is already under 500 and `session/pty_responder/mod.rs` is an empty-carve-out before any new functionality adds lines. Per `test-organization.md` the module becomes `session/pty_responder/{mod, tests}.rs` and `session/mod.rs` re-exports `pub use pty_responder::PtyResponder;` for backwards compat — no downstream consumer changes its import. The extension is private methods on `PtyResponder` (e.g., `handle_color_request`, `handle_clipboard_load`, `handle_clipboard_store`) called from the existing `EventListener::send_event` dispatch, which keeps the `send_event` body small and its match arm bodies one-line delegations.

**Tasks (strict ordering — the order is load-bearing):**

- [ ] **Step 1 (BEFORE any functional change): Proactive split of `PtyResponder` into `session/pty_responder/{mod, tests}.rs`.** Move the struct + impl out of `session/mod.rs`; add `pub use pty_responder::PtyResponder;` re-export. Verify `session/mod.rs` is ≤ 450 lines after the move and `session/pty_responder/mod.rs` is ≤ 50 lines before any extension. `./build-all.sh && ./clippy-all.sh && timeout 150 ./test-all.sh` must be green AT THIS COMMIT before proceeding — no functional change, just file re-layout. This is the load-bearing proactive-split gate: violating it means the extension lands on top of a file already at 459 lines and blows past the 500 limit in one commit.
- [ ] **Step 2: TDD failing-first ordering (11 steps above).** Every test written failing-first BEFORE its implementation lands.
- [ ] **Step 3: Extend `PtyResponder::send_event`** with the three new event arms (ColorRequest, ClipboardLoad, ClipboardStore) and the `_ => {}` catch-all negative pin. The `send_event` body stays an orchestration match: each arm delegates to a private method (`handle_color_request`, `handle_clipboard_load`, `handle_clipboard_store`) defined in the same `pty_responder/mod.rs`. This keeps `send_event` under 20 lines regardless of how complex the individual handlers grow.

- [ ] **Add `take_osc_responses()` and `take_clipboard_stores()`** accessor methods to `PtyResponder` matching the existing `take_responses()` pattern.
- [ ] **Extend `PtySession::drain` and `PtySession::drain_blocking`** with a `write_osc_responses_back()` call after each iteration. The helper drains `take_osc_responses()` and writes each via `self.writer.write_all(...)`. If the write fails, propagate the error — drain is already fallible.
- [ ] **Verify the existing vttest + Section 05 tests pass unchanged:** `timeout 150 cargo test -p oriterm_core --test vttest` and `timeout 150 cargo test -p oriterm_core --test tack -- test_menu`. Any regression is a 06.0.c blocker.
- [ ] **Add an integration test** in `oriterm_core/tests/tack/tools_menu/osc_responder_integration.rs` that spawns tack via `PtySession::spawn_tack`, sends a forged OSC 10 query via `session.send_raw(b"\x1b]10;?\x07")`, calls `drain_blocking`, and asserts the PtyResponder captured the round-tripped response. Skips if tack unavailable.
- [ ] **File-size check:** `session/mod.rs` and `session/pty_responder/mod.rs` both under 500 lines after the split.
- [ ] **Debug + release parity.**
- [ ] **Cross-compile gate:** `cargo build --target x86_64-pc-windows-gnu -p oriterm_test_support --tests` succeeds.

**Output of 06.0.c:** `PtyResponder` captures all four round-trip event variants. `PtySession::drain` writes OSC responses back through the PTY transparently. 06.5's OSC-based direct-VTE xcheck tests (Cr, Cs, Ms) can exercise `Term<PtyResponder>` directly without needing tack. Any future section that needs OSC round-trip behavior inherits it for free. No new type introduced — SSOT preserved at `PtyResponder`.

---

## 06.1 Status reports scenarios (DA/DSR/DECRQM sub-submenu walker)

**File(s):**
- `crates/oriterm_test_support/src/tack_framework/scenarios/status_reports/{mod, tests}.rs` (NEW — single-word module name `status_reports`, matching the Sections 04/05 convention. Const ScenarioSpec values + `parse_status_reports_screen` parser + sibling parser tests)
- `oriterm_core/tests/tack/tools_menu/status_reports.rs` (NEW — `#[test] fn` wrappers, one per sub-test discovered in 06.0.b)

**Depends on:** 06.0 (tools menu inventory pins `s`), 06.0.b (status reports sub-submenu inventory pins the sub-test sequence).

Tack's status reports sub-submenu walker displays DA1, DA2, DA3, multiple DSR variants, DECRQSS, DECRQPSR, and mode-status probes — one per `n` press. Each sub-test is a stable screen between `n` presses, so 06.1 uses `ScenarioSpec` (NOT `PhaseSpec`).

- [ ] **Declare const ScenarioSpecs in `scenarios::status_reports::mod.rs`** — one per sub-test in `STATUS_REPORTS_INVENTORY`. The menu_path for sub-test index `i` is `[MenuStep::new(b"t", "tack/tools [q] >"), MenuStep::new(b"s", "tack/tools/status [q] >"), ...n-press steps...]`. Each `n`-press step's anchor is the unique label of the NEXT sub-test (read from the pinned inventory). Example:
  ```rust
  pub const TACK_TOOLS_DA1: ScenarioSpec = ScenarioSpec {
      id: "tack_tools_da1",
      screen_id: "tack_tools_status_reports_da1",
      menu_path: &[
          MenuStep::new(b"t", "tack/tools [q] >"),
          MenuStep::new(b"s", "Primary device attributes"),
      ],
      ready_anchor: "Primary device attributes",
      quit_path: None,
      parser: parse_status_reports_screen,
  };

  pub const TACK_TOOLS_DA2: ScenarioSpec = ScenarioSpec {
      id: "tack_tools_da2",
      screen_id: "tack_tools_status_reports_da2",
      menu_path: &[
          MenuStep::new(b"t", "tack/tools [q] >"),
          MenuStep::new(b"s", "Primary device attributes"),
          MenuStep::new(b"n", "Secondary device attributes"),
      ],
      ready_anchor: "Secondary device attributes",
      quit_path: None,
      parser: parse_status_reports_screen,
  };
  // ... one per sub-test in STATUS_REPORTS_INVENTORY
  ```
  **Sentinel fallback.** Until 06.0.b's discovery pins the exact sub-test label strings, the `wait_for` anchors SHOULD be `unverified_anchor()` sentinels (per the Section 05.0.b sentinel detection helper). The `assert_no_unverified_sentinels` check in `prepare_and_navigate` panics BEFORE PTY spawn with a referral to 06.0.b. Once 06.0.b's implementer reads the captured snapshots and pins the labels, replace the sentinels with the real strings.

- [ ] **Declare `parse_status_reports_screen`** in the same mod.rs file. It extracts the response fields via `grid_find_field`:
  ```rust
  pub fn parse_status_reports_screen(grid: &str) -> ScreenFacts {
      let mut notes = Vec::new();
      // Tack prints the sent escape + the received response on
      // separate lines; the received response has a unique label like
      // "Received:" or a paren-prefixed label like "(DA)". The exact
      // format is pinned by the 06.0.b snapshot — read the snapshot
      // and token-match via grid_find_field.
      if let Some(response) = grid_find_field(grid, "Received:") {
          notes.push(format!("received={response}"));
      }
      // DA1 response starts with \E[? — token-match the canonical
      // marker. grid_has_token prevents false-positive matches against
      // literal English text.
      if grid_has_paren_token(grid, "DA") || grid.contains("\\E[?") {
          notes.push("da1_marker".to_string());
      }
      // DSR CPR response has the form \E[<row>;<col>R — extract row
      // and col from the field.
      if let Some(cpr) = grid_find_field(grid, "CPR:") {
          notes.push(format!("cpr={cpr}"));
      }
      ScreenFacts {
          header_text: grid.lines().next().unwrap_or("").to_string(),
          capability_labels: Vec::new(),
          notes,
      }
  }
  ```
  **Every detection uses `grid_has_token` / `grid_has_paren_token` / `grid_find_field` — NEVER blind `grid.contains` for short markers.** A one-character `contains("c")` check matches every lowercase `c` in the screen including the literal word `"color"`. The M3 fix from Section 04 is the canonical rule.

- [ ] **Sibling parser tests in `scenarios::status_reports::tests.rs`:** test synthesized grids for each sub-test (DA1 response, DA2 response, DSR status, DSR CPR, DECRQM affirmative, DECRQM negative). Include negative pins: `parse_status_reports_screen_rejects_literal_word_DA_in_english_text` feeds a grid containing "the DA response was not received" and asserts NO `da1_marker` note (proves `grid_has_paren_token` is the only path — bare `grid.contains("DA")` would false-positive).

- [ ] **Write `#[test] fn` wrappers in `oriterm_core/tests/tack/tools_menu/status_reports.rs`:**
  ```rust
  use oriterm_test_support::tack_framework::ScenarioRunner;
  use oriterm_test_support::tack_framework::scenarios::status_reports::{
      TACK_TOOLS_DA1, TACK_TOOLS_DA2, TACK_TOOLS_DA3, TACK_TOOLS_DSR,
      TACK_TOOLS_DSR_CPR, TACK_TOOLS_DECRQM,
  };

  #[test]
  fn tack_tools_da1() {
      if !ScenarioRunner::available() { return; }
      let outcome = ScenarioRunner::run(&TACK_TOOLS_DA1);
      assert!(
          outcome.parsed.notes.iter().any(|n| n == "da1_marker"),
          "expected DA1 marker in status reports DA1 screen, grid:\n{}",
          outcome.grid_text,
      );
      insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);
  }
  // ... one #[test] fn per sub-test in STATUS_REPORTS_INVENTORY
  ```

- [ ] **Cross-validation with vttest menu6 — concrete shape.** Add a consistency test `tack_tools_da1_matches_vttest_menu6` (in `oriterm_core/tests/tack/tools_menu/status_reports.rs`) that does NOT require both paths to be green simultaneously. It uses the pre-captured insta snapshots from `oriterm_core/tests/vttest/menu6/snapshots/` as the byte-reference (so the new test does not re-spawn vttest, avoiding double PTY cost). Matrix dimensions: { DA1 } × { vttest menu6 snapshot line, Section 06.1 TACK_TOOLS_DA1 outcome }. Steps: (a) load `oriterm_core/tests/vttest/menu6/snapshots/menu6__walk_menu6_subscreens__*.snap` (the existing snap file containing DA1 response bytes), (b) extract the DA response token via the same `grid_find_field("Response")` call used by menu6, (c) run `ScenarioRunner::run(&TACK_TOOLS_DA1)` and extract the DA response via `parse_status_reports_screen`, (d) assert the two response strings are byte-identical (after stripping trailing whitespace/newlines). Any divergence is a bug — file via `/add-bug` immediately. Negative pin: also assert both strings are NON-empty (guards against a silent empty-vs-empty "match"). The test skips if vttest snapshots are missing (first-run freshness case) but MUST run on every CI after the initial snapshot capture.

- [ ] **Cap coverage extension for 06.1:** `u6` + `u7` move from `cap_coverage/section_06.rs::CONTRIBUTION.exempt` → `CONTRIBUTION.covered`. Verify `tack_cap_coverage_matrix` (Section 05.5) still passes — the stale-exemption negative pin must stay green.

- [ ] **Semantic pin matrix for the sub-submenu walker.** Beyond the per-sub-test existence assertions, add three semantic pins in `oriterm_core/tests/tack/tools_menu/status_reports.rs` that ONLY pass under the new two-track architecture: (a) `tack_status_reports_walker_produces_distinct_grids_per_sub_test` — run TACK_TOOLS_DA1 and TACK_TOOLS_DA2 in sequence, assert the two `outcome.grid_text` strings are NOT equal (catches a regression where the walker falls back to the first screen for all sub-tests); (b) `tack_status_reports_da_response_is_non_empty_hex` — run TACK_TOOLS_DA1, assert the parsed `notes` contain a hex-only response token of at least 4 bytes (catches a silent "response field is empty string" regression); (c) `tack_status_reports_decrqm_fires_when_sub_test_index_reached` — run TACK_TOOLS_DECRQM, assert the parser produced a `decrqm_*` note (catches a regression where tack's DECRQM screen drifts out of the sub-test sequence without the inventory drift gate catching it — the drift gate compares labels, this pin compares response-parse output).

- [ ] **Debug + release parity. Determinism: 10 reruns per `#[test] fn`.**

- [ ] **Recommended TPR checkpoint:** `/tpr-review` after 06.1 lands. Catches parser regex regressions, ready_anchor mismatches, missed `q\n` quit on the nested sub-submenu.

---

## 06.2 SGR mode table scenario (stable-screen, 80 modes)

**File(s):**
- `crates/oriterm_test_support/src/tack_framework/scenarios/sgr_modes/{mod, tests}.rs` (NEW — single-word module name `sgr_modes`. Const ScenarioSpec + parser + sibling parser tests)
- `oriterm_core/tests/tack/tools_menu/sgr_modes.rs` (NEW — `#[test] fn tack_tools_sgr_80x24`)

**Depends on:** 06.0 (tools menu inventory pins `g`).

**Empirical reality (verified by Agent 1's live probe).** After sending `g` from the tools menu, tack prompts `tack/tools/sgr Enter =><?r [<cr>] > ` — this asks whether to include optional private-use chars (`<`, `=`, `>`, `?`, `r`) in the SGR test. Sending bare `\r` runs the default test. Tack then draws a **STABLE SCREEN** with 80 modes arranged in a 9-row grid: `Mode 0 Mode 1 Mode 2 ... Mode 9 / Mode 10 ... Mode 19 / ... / Mode 70 Mode 71 ... Mode 79` plus a "Test enter/exit attributes" header line. The screen is stable — tack waits for user input after drawing it. This means `ScenarioSpec`, NOT `PhaseSpec`.

- [ ] **Declare `TACK_TOOLS_SGR`** in `scenarios::sgr_modes::mod.rs`:
  ```rust
  pub const TACK_TOOLS_SGR: ScenarioSpec = ScenarioSpec {
      id: "tack_tools_sgr",
      screen_id: "tack_tools_sgr",
      menu_path: &[
          MenuStep::new(b"t", "tack/tools [q] >"),
          // After `g`, tack prompts for optional private-use chars.
          // Anchor on the unique SGR sub-prompt substring.
          MenuStep::new(b"g", "tack/tools/sgr"),
          // Bare \r accepts the default (no private-use chars).
          // After this, tack draws the 80-mode grid. The ready_anchor
          // is "Mode 79" — the last mode label, which only exists on
          // the rendered grid.
          MenuStep::new(b"\r", "Mode 79"),
      ],
      ready_anchor: "Mode 79",
      quit_path: None,
      parser: parse_sgr_modes_screen,
  };
  ```
  **Why `Mode 79` as the ready_anchor.** It's unique to the SGR grid (the prompt doesn't contain it, nor does any earlier menu). Per the pre-existing-anchor rule, a unique last-element anchor guarantees the grid has finished painting before `grid_text()` is captured.

- [ ] **Declare `parse_sgr_modes_screen`** — token-match on the expected `Mode N` entries:
  ```rust
  pub fn parse_sgr_modes_screen(grid: &str) -> ScreenFacts {
      // Expected: 80 mode entries "Mode 0", "Mode 1", ..., "Mode 79".
      // Count how many we actually see via grid_has_token.
      let mut found_modes: Vec<u32> = Vec::new();
      for n in 0..80 {
          let label = format!("{n}");
          // grid_has_token prevents "Mode 7" from matching "Mode 70"
          // — we're looking for the specific token "7" near "Mode".
          // More robust: scan for the two-token sequence "Mode" "N".
          if grid_has_token(grid, &label) {
              found_modes.push(n);
          }
      }
      ScreenFacts {
          header_text: grid.lines().next().unwrap_or("").to_string(),
          capability_labels: Vec::new(),
          notes: vec![format!("found_modes_count={}", found_modes.len())],
      }
  }
  ```
  **Token matching gotcha.** Numbers 7 and 70 share a 1-char prefix. `grid_has_token("7")` against a grid containing "Mode 70" returns FALSE (the right boundary is `0`, not whitespace), so the mode count is not confused by partial matches. Verify this in a sibling unit test:
  ```rust
  #[test]
  fn parse_sgr_modes_screen_does_not_confuse_7_with_70() {
      let grid = "Mode 70 Mode 71\n";
      let facts = super::parse_sgr_modes_screen(grid);
      // Must find 70 and 71, but NOT 7 or 1 as false-positives.
      assert!(facts.notes[0].contains("found_modes_count=2"));
  }
  ```

- [ ] **`#[test] fn tack_tools_sgr_80x24`** in `oriterm_core/tests/tack/tools_menu/sgr_modes.rs`:
  ```rust
  #[test]
  fn tack_tools_sgr_80x24() {
      if !ScenarioRunner::available() { return; }
      let outcome = ScenarioRunner::run(&TACK_TOOLS_SGR);
      let count_note = outcome.parsed.notes.iter()
          .find(|n| n.starts_with("found_modes_count="))
          .expect("parser must record found_modes_count");
      let count: usize = count_note
          .trim_start_matches("found_modes_count=")
          .parse()
          .expect("integer count");
      assert!(
          count >= 70,
          "expected >=70 SGR modes on tack tools SGR screen (80 expected minus some that may share tokens with other labels), got {count}\nGrid:\n{}",
          outcome.grid_text,
      );
      insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);
  }
  ```
  The `>= 70` threshold (not `== 80`) allows for some token-matching slop where a mode number happens to collide with an unrelated grid character. If the real count is consistently higher, tighten to `>= 80`.

- [ ] **Sibling parser tests** covering: full 80-mode sweep, partial grids with only low modes, partial grids with only high modes, empty grid, grid with only the header.

- [ ] **Private-use char variants.** Tack's SGR prompt (`tack/tools/sgr Enter =><?r [<cr>] >`) accepts optional private-use chars `<`, `=`, `>`, `?`, `r` before `\r` to exercise non-standard SGR sub-parameter forms. Add ONE additional scenario `TACK_TOOLS_SGR_QUESTION` that sends `b"?\r"` (include the `?` private-use sub-form) and asserts the resulting screen renders the `Mode N ?` entries. The additional scenarios for `<`, `=`, `>`, `r` are optional — the `?` variant is the minimum coverage expansion because it is the only private-use char that corresponds to a DEC-private SGR form actually declared in `extra/ori_term.info`. If 06.0 discovery surfaces evidence that `<`, `=`, `>`, `r` also map to declared caps, add scenarios for those too (per the "expand the mission, never scope down" rule).

- [ ] **Debug + release parity. Determinism: 10 reruns.**

---

## 06.3 Character sets scenarios (G0/G1/GL/GR banks)

**File(s):**
- `crates/oriterm_test_support/src/tack_framework/scenarios/character_sets/{mod, tests}.rs` (NEW — single-word module name `character_sets`, NOT `tools_character_sets`; Section 07's `depends_on_contract` for "06" hard-pins this exact const path `scenarios::character_sets::TACK_TOOLS_G0_DEC_GRAPHICS`, so the module path must match the existing Sections 04/05 naming convention: `color`, `modes`, `cursor_movement`, `graphic_rendition`, `character_sets`)
- `oriterm_core/tests/tack/tools_menu/character_sets.rs` (NEW)

**Depends on:** 06.0 (tools menu inventory pins `c`).

**Empirical reality (verified by Agent 1's live probe).** After `c`, tack prompts:
```
Enter the bank ()*+,-./ followed by the character set 0123456789:;<=>?
for private use, and @A...Z[\]^_`a...z{|}~ for standard sets.
```
This is a TWO-step interactive flow: send a bank-char (e.g., `)` for G1), then a charset-char (e.g., `0` for DEC special graphics). To reach DEC special graphics via G1: send `)` then `0`. Tack then draws a stable screen showing the character set with the GL character map rendered via the designated charset.

- [ ] **Declare `TACK_TOOLS_G0_DEC_GRAPHICS`** (the canonical case — G1 bank pointed at DEC special graphics charset):
  ```rust
  pub const TACK_TOOLS_G0_DEC_GRAPHICS: ScenarioSpec = ScenarioSpec {
      id: "tack_tools_g0_dec_graphics",
      screen_id: "tack_character_sets",  // Stable name for Section 07 golden reuse
      menu_path: &[
          MenuStep::new(b"t", "tack/tools [q] >"),
          MenuStep::new(b"c", "Enter the bank"),
          MenuStep::new(b")", "character set"),  // anchor: prompt text after bank is accepted
          MenuStep::new(b"0", "DEC"),  // anchor: DEC graphics label on the rendered screen
      ],
      ready_anchor: "DEC",
      quit_path: None,
      parser: parse_character_sets_screen,
  };
  ```
  **Anchor fragility warning.** The exact prompt text after each step is pinned by the 06.0 / 06.3 discovery snapshot. If the live probe shows a different post-`c` prompt, replace the anchor strings. The plan does NOT invent anchors — the implementer reads the snapshot first.

- [ ] **Declare `parse_character_sets_screen`** — how to assert the DEC graphics chars ARE rendered depends on how oriterm_core renders SCS (Select Character Set) sequences:
  - If oriterm_core translates DEC special graphics chars to Unicode box-drawing (U+2500–U+257F), the parser scans for `('\u{2500}'..='\u{257F}').contains(&ch)` and counts distinct chars. Assertion: `>= 4` distinct box-drawing chars (corners + edges).
  - If oriterm_core keeps the raw ASCII form (l/k/q/x/m/j/n/u/t/v/w — the DEC char map in its raw form), the parser scans for those specific ASCII chars following a pattern that proves SCS was applied, not just raw input. The exact detection depends on what `grid_text()` emits.

  **VERIFY BEFORE WRITING.** Before pinning the parser, run a throwaway test that feeds the literal SCS sequence `\x1b)0\x0eabcd\x0f` into a `Term` (via the existing `feed` helper in `oriterm_core/src/term/handler/tests.rs`) and capture `grid_text()` to see what oriterm_core emits. The parser matches whatever that output is. Do NOT guess the rendering format — test it once and pin the expected output.

- [ ] **Sibling parser tests — explicit matrix.** In `scenarios::character_sets::tests` add the following `#[test] fn`s, each feeding a synthetic grid string to `parse_character_sets_screen` and asserting the counted matches: (a) `parse_character_sets_unicode_box_drawing` — feeds a string containing `"┌─┐│└┘"` (U+250C, U+2500, U+2510, U+2502, U+2514, U+2518) and asserts count ≥ 4; (b) `parse_character_sets_raw_ascii_line_drawing` — feeds a string containing `"lqkxmj"` (the DEC ASCII line-drawing form) and asserts count ≥ 4; (c) `parse_character_sets_empty_grid` — feeds `""` and asserts count == 0; (d) `parse_character_sets_negative_non_dec_grid` — feeds `"Hello World, this has no graphics"` and asserts count == 0 (negative pin against false positives on English text); (e) `parse_character_sets_mixed_unicode_and_ascii` — feeds `"┌lqk"` and asserts count ≥ 2 (proves the parser accepts whichever form oriterm_core emits, not both simultaneously); (f) `parse_character_sets_boundary_count` — construct a grid with EXACTLY `MIN_DEC_GRAPHICS_THRESHOLD - 1` chars and assert the count equals exactly `MIN_DEC_GRAPHICS_THRESHOLD - 1` (semantic pin: the threshold is a comparison, not an off-by-one match). The `MIN_DEC_GRAPHICS_THRESHOLD` is a module-level `const` so all test sites reference the same number.

- [ ] **`#[test] fn tack_tools_g0_dec_graphics_80x24`** in the test target — assert the parser's count exceeds the minimum threshold, insta-snapshot the grid. Include a negative pin: if `outcome.grid_text.is_empty()`, panic with `"TACK_TOOLS_G0_DEC_GRAPHICS returned empty grid — tack navigation failed before render"` so the threshold comparison is not silently satisfied by an empty grid.

- [ ] **Optional additional scenarios.** If 06.0 / 06.3 discovery surfaces more banks in tack's flow (e.g., G0 DEC graphics via `(0`, GL lock via SI/SO, GR lock via LS2/LS3), add one scenario per bank. Minimum: G1 DEC graphics above. Expansion is gated on the discovery output.

- [ ] **Debug + release parity. Determinism: 10 reruns.**

---

## 06.4 ENQ/ACK handshake scenario (u8/u9 round-trip)

**File(s):**
- `crates/oriterm_test_support/src/tack_framework/scenarios/enq_ack/{mod, tests}.rs` (NEW — single-word module name `enq_ack`)
- `oriterm_core/tests/tack/tools_menu/enq_ack.rs` (NEW)

**Depends on:** 06.0 (tools menu inventory pins `u`), `extra/ori_term.info` u8/u9 declarations.

**Empirical reality.** After `u` tack prints `Testing ENQ/ACK, standby...`, sends the `u9` byte sequence to the terminal, waits for a response, and reports:
```
ENQ sequence from (u9): <literal bytes>
ACK received: <received response>
Length of ACK <N>. Expected length of ACK <M>. Terminating character found in (u8): c
```
The three key fields (`ENQ sequence`, `ACK received`, `Length`) map to u9 (the trigger), u8 (the terminator), and the total response length. Section 06.4 asserts:
1. The `ENQ sequence from (u9)` field matches the `u9` declaration in `extra/ori_term.info`.
2. The `ACK received` field matches ori_term's DA response (because ori_term uses its DA response as its ENQ/ACK reply — verify this against `oriterm_core/src/term/handler/status.rs`).
3. The `Length of ACK` matches the byte length of `ACK received`.
4. The terminator `(u8)` matches the `u8` declaration.

- [ ] **Declare `TACK_TOOLS_ENQ_ACK`:**
  ```rust
  pub const TACK_TOOLS_ENQ_ACK: ScenarioSpec = ScenarioSpec {
      id: "tack_tools_enq_ack",
      screen_id: "tack_tools_enq_ack",
      menu_path: &[
          MenuStep::new(b"t", "tack/tools [q] >"),
          MenuStep::new(b"u", "ACK received:"),
      ],
      ready_anchor: "Length of ACK",
      quit_path: None,
      parser: parse_enq_ack_screen,
  };
  ```

- [ ] **Declare `parse_enq_ack_screen`** using `grid_find_field`:
  ```rust
  pub fn parse_enq_ack_screen(grid: &str) -> ScreenFacts {
      let mut notes = Vec::new();
      if let Some(enq) = grid_find_field(grid, "(u9):") {
          notes.push(format!("enq_sequence={enq}"));
      }
      if let Some(ack) = grid_find_field(grid, "received:") {
          notes.push(format!("ack_received={ack}"));
      }
      if let Some(len) = grid_find_field(grid, "Length") {
          notes.push(format!("ack_length={len}"));
      }
      ScreenFacts {
          header_text: "ENQ/ACK test".to_string(),
          capability_labels: Vec::new(),
          notes,
      }
  }
  ```
  **Field label tuning.** The exact label strings depend on how `grid_find_field` tokenizes the screen. If `grid_find_field` with the label `"(u9):"` doesn't match because the field label contains punctuation that trips the whitespace-bounded tokenizer, fall back to line-based extraction: find the line starting with `ENQ sequence`, split on `:`, take the trailing whitespace-trimmed value. Pick whichever is empirically reliable and pin it with sibling tests.

- [ ] **Cross-reference against `extra/ori_term.info` u8/u9.** In the `#[test] fn`, load the terminfo via `parse_declared_caps()` (the Section 05.5 helper already in `cap_coverage/mod.rs`, already `pub` — NO visibility widening needed, verified against `crates/oriterm_test_support/src/tack_framework/cap_coverage/mod.rs:240`). Note: `parse_declared_caps()` returns `BTreeSet<String>` of cap NAMES only — it does NOT return cap values. The test asserts u8 and u9 are PRESENT in the declared set, AND separately loads the terminfo source via `include_str!("../../../extra/ori_term.info")` (or an embedded copy from the scenario module) and regex-extracts the u8/u9 values by grepping `u8=<value>` and `u9=<value>` lines. The regex-extracted values get compared against the parser-extracted tack output. If `parse_declared_caps` grows a value-returning variant in a future refactor (e.g. `parse_declared_caps_with_values() -> BTreeMap<String, String>`), migrate this test to use it and delete the regex extraction — the regex is a temporary step until the helper grows.

- [ ] **`#[test] fn tack_tools_enq_ack_80x24`:**
  ```rust
  #[test]
  fn tack_tools_enq_ack_80x24() {
      if !ScenarioRunner::available() { return; }
      let outcome = ScenarioRunner::run(&TACK_TOOLS_ENQ_ACK);
      let enq_note = outcome.parsed.notes.iter()
          .find(|n| n.starts_with("enq_sequence="))
          .expect("parser must record enq_sequence");
      // Cross-reference u9 cap from the pinned terminfo.
      // parse_declared_caps() returns BTreeSet<String> — names only —
      // so we assert presence, then separately extract the value via
      // regex over an embedded terminfo source until a
      // parse_declared_caps_with_values() variant exists.
      let caps = oriterm_test_support::tack_framework::cap_coverage::parse_declared_caps();
      assert!(caps.contains("u9"), "extra/ori_term.info must declare u9");
      let u9_value = extract_cap_value(TERMINFO_SRC, "u9")
          .expect("u9 value must be extractable from pinned terminfo");
      assert!(
          enq_note.contains(&u9_value),
          "ENQ sequence from tack does not match u9 declaration.\n\
           tack reported: {enq_note}\n\
           u9 declares:   {u9_value}\n\
           Grid:\n{grid}",
          grid = outcome.grid_text,
      );
      insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);
  }

  /// Extract the value of a named cap from a tic-format terminfo
  /// source. Returns None if the cap is absent or is a bool/numeric
  /// cap (no `=<value>` form). Pure function for sibling-test
  /// validation — the sibling test pins the extraction against
  /// synthetic terminfo fragments.
  fn extract_cap_value(src: &str, cap: &str) -> Option<String> {
      // Walk each comma-separated cap token. Match tokens that start
      // with `<cap>=`; return everything after the `=`.
      for line in src.lines() {
          for tok in line.trim_start().split(',') {
              let t = tok.trim();
              if let Some(rest) = t.strip_prefix(cap) {
                  if let Some(val) = rest.strip_prefix('=') {
                      return Some(val.to_string());
                  }
              }
          }
      }
      None
  }

  const TERMINFO_SRC: &str = include_str!(concat!(
      env!("CARGO_MANIFEST_DIR"),
      "/../../extra/ori_term.info",
  ));
  ```
  **API note.** `parse_declared_caps()` already exists in `crates/oriterm_test_support/src/tack_framework/cap_coverage/mod.rs:240` with `pub` visibility — NO widening needed. The function returns `BTreeSet<String>` (cap names only, no values). The `extract_cap_value` helper above does the value extraction locally for 06.4; if other sections need it, promote it to `cap_coverage/mod.rs::parse_cap_value(src, cap)` in a follow-up commit. This keeps Section 06 unblocked without speculative cross-section refactoring. SSOT preservation: the regex extraction is a single local helper, not scattered across multiple tests, so if a value-returning variant lands later there's exactly one site to migrate.

- [ ] **Cap coverage extension for 06.4:** `u8` + `u9` move from `section_06.rs::CONTRIBUTION.exempt` → `CONTRIBUTION.covered`.

- [ ] **Debug + release parity. Determinism: 10 reruns.**

---

## 06.5.a RecordingListener helper promotion (structural prerequisite for 06.5)

**File(s):**
- `oriterm_core/src/term/handler/test_helpers.rs` (NEW — shared test helper module)
- `oriterm_core/src/term/handler/mod.rs` (add `#[cfg(test)] pub(super) mod test_helpers;` near the bottom)
- `oriterm_core/src/term/handler/tests.rs` (EDIT — replace the in-file `RecordingListener` definition with `use super::test_helpers::{RecordingListener, term_with_recorder, term_with_recorder_sized};`)

**Why this is its own sub-subsection.** 06.5 lands 21 direct-VTE tests in-crate, 20 of which construct a `Term<RecordingListener>` to capture events fired by escape sequences. `oriterm_core/src/term/handler/tests.rs:17-55` currently declares `RecordingListener`, `term_with_recorder()`, and `term_with_recorder_sized()` as PRIVATE items inside a `#[cfg(test)] mod tests;` sibling file. Sibling `#[cfg(test)] mod tests;` modules do NOT cross-share visibility — `handler/tack_cap_xcheck/tests.rs` CANNOT `use super::super::tests::RecordingListener` because `tests` is not on a visible path. Any attempt to start 06.5 without first promoting these helpers fails to compile. Treating the promote as a one-line "task under 06.5" obscured its load-bearing role; it is structurally a gate, so it lives as its own sub-subsection with a full TDD plan and completion criteria.

**Scope.** Move `RecordingListener` (struct + `EventListener` impl), `term_with_recorder()`, and `term_with_recorder_sized()` from `handler/tests.rs:17-55` into a new `handler/test_helpers.rs`. Change their visibility from private (or `fn` scope) to `pub(super)` so BOTH the existing `handler/tests.rs` AND the new `handler/tack_cap_xcheck/tests.rs` can consume them via `use super::test_helpers::*`. This is a pure move — zero behavioral change.

**Tasks (strict TDD ordering):**

- [ ] **Step 1 (failing-first):** add `#[cfg(test)] pub(super) mod test_helpers;` to `handler/mod.rs`, create an empty `handler/test_helpers.rs`, and add a placeholder test file `handler/tack_cap_xcheck/mod.rs` with `#[cfg(test)] mod tests;` that imports `use super::super::test_helpers::RecordingListener;`. `cargo test -p oriterm_core` fails because `RecordingListener` doesn't exist in `test_helpers` yet.
- [ ] **Step 2 (move):** cut `RecordingListener` + its `EventListener` impl + `term_with_recorder` + `term_with_recorder_sized` out of `handler/tests.rs:17-55`, paste into `handler/test_helpers.rs`, change visibility to `pub(super)`. Update `handler/tests.rs` to add `use super::test_helpers::{RecordingListener, term_with_recorder, term_with_recorder_sized};` at the top of its import block.
- [ ] **Step 3 (SSOT regression pin):** run `cargo test -p oriterm_core -- handler::tests` — every existing test in `handler/tests.rs` must still be green. If any test fails, revert and investigate (the move should be zero-diff behaviorally).
- [ ] **Step 4 (consumer pin):** add a minimal test in `handler/tack_cap_xcheck/tests.rs` that constructs `Term<RecordingListener>` via the imported `term_with_recorder()` and asserts it compiles. This is the "can I actually import the helper from tack_cap_xcheck" compile gate.
- [ ] **Step 5 (negative pin — keep tests.rs still-green):** after the move, grep `handler/tests.rs` for any lingering use of `RecordingListener` as a bare type (not via `super::test_helpers::`). Every reference should be qualified via the import.
- [ ] **Step 6 (line-count check):** `wc -l oriterm_core/src/term/handler/tests.rs` should DECREASE by ~40 lines (not grow). `wc -l oriterm_core/src/term/handler/test_helpers.rs` should be ~50. Record both numbers in the commit message so the next reviewer can spot-verify.
- [ ] **Step 7 (debug + release):** `timeout 150 cargo test -p oriterm_core -- handler` and `timeout 150 cargo test -p oriterm_core --release -- handler` both green.
- [ ] **Step 8 (cross-compile):** `cargo build --target x86_64-pc-windows-gnu -p oriterm_core --tests` succeeds.
- [ ] **Step 9 (semantic pin for the move itself):** add `#[test] fn recording_listener_captures_title_event` in `handler/test_helpers.rs`'s sibling `tests.rs` (a new file — `test_helpers/tests.rs` if the file grows past ~100 lines, otherwise an inline `#[cfg(test)] mod tests;` at the bottom). The test feeds `\x1b]2;hello\x07` to a `Term<RecordingListener>` and asserts the recorder captured an `Event::Title("hello")`. This is a regression pin against a future refactor that accidentally breaks `RecordingListener::send_event` — if the helper stops recording events, every 06.5 test silently passes, so we need an in-isolation sanity test.

**Completion gate:** all 9 steps green. Without this gate, 06.5 Cap-by-cap tasks cannot start.

**Why 06.5 lists this as a prerequisite rather than absorbing it as a task:** `handler/tests.rs` is ~5,860 lines (verified via `wc -l oriterm_core/src/term/handler/tests.rs` before starting 06.5.a). Touching a 5k-line test file during 06.5 carries blast radius — isolating the move into a single commit with its own completion gate keeps the impact auditable. Per the "fix one widget type across all interaction states before fixing another" rule from `impl-hygiene.md::Narrow the Front`.

---

## 06.5 Direct-VTE cap xcheck (non-tack-reachable caps)

**File(s):**
- `oriterm_core/src/term/handler/tack_cap_xcheck/{mod, tests}.rs` (NEW — sibling submodule of `handler/`)
- `oriterm_core/src/term/handler/mod.rs` (add `mod tack_cap_xcheck;` at the bottom)

**Why this subsection exists.** Per CLAUDE.md "never scope down", the 19 modern caps declared in `extra/ori_term.info` that tack v1.08 cannot probe (Smulx, Setulc, Sync, BD, BE, PS, PE, Se, Ss, XF, kxIN, kxOUT, Tc, RGB, Cr, Cs, Ms, hs, dsl, fsl, tsl, AX, XT — 23 entries total; "~19" in the mission criterion because a few are pure bool markers without escape sequences) are still Section 06's work. Tack is one validation mechanism, direct-VTE is another. Both tracks terminate in the cap-coverage matrix.

**Mechanism.** Each direct-VTE test:
1. Loads the cap's declared escape sequence from `extra/ori_term.info` via `parse_declared_caps` (the Section 05.5 helper).
2. Constructs a `Term<RecordingListener>` (or `Term<PtyResponder>` for OSC-based caps that need 06.0.c's extended responder) — NO tack, NO PtySession.
3. Feeds the escape sequence via the existing `feed` helper from `oriterm_core/src/term/handler/tests.rs`.
4. Asserts the correct event fires (for caps that fire events) OR the correct mode/state toggles in `TermMode` / grid (for caps that are pure state changes) OR the output bytes match (for caps that produce terminal responses).
5. Cross-references the cap declaration line in `extra/ori_term.info` so a future terminfo edit that changes the escape sequence breaks the test BEFORE the test-side code diverges.

**Structural prerequisite:** 06.5.a (RecordingListener helper promotion) MUST land before the first Cap-by-cap task. Without it, each cap test would need to copy-paste `RecordingListener` into `tack_cap_xcheck/tests.rs` — a `LEAK:algorithmic-duplication` across ~21 test files. See 06.5.a immediately below.

**Cap-by-cap task list.**

- [ ] **`Smulx`** (kitty colon underline style — `\E[4\:%p1%dm`). Matrix of five `#[test] fn`s in `tack_cap_xcheck/sgr_extensions.rs`, one per sub-parameter:
  - `tack_cap_xcheck_smulx_off_4_0` — feed `\E[4:0m`, assert NO underline flags set in cell template (`!cell.flags.intersects(CellFlags::ALL_UNDERLINES)`).
  - `tack_cap_xcheck_smulx_straight_4_1` — feed `\E[4:1m`, assert `CellFlags::UNDERLINE` set and others clear.
  - `tack_cap_xcheck_smulx_double_4_2` — feed `\E[4:2m`, assert `CellFlags::DOUBLE_UNDERLINE` set and others clear.
  - `tack_cap_xcheck_smulx_curly_4_3` — feed `\E[4:3m`, assert `CellFlags::CURLY_UNDERLINE` set and others clear.
  - `tack_cap_xcheck_smulx_dotted_4_4` — feed `\E[4:4m`, assert `CellFlags::DOTTED_UNDERLINE` set and others clear.
  - `tack_cap_xcheck_smulx_dashed_4_5` — feed `\E[4:5m`, assert `CellFlags::DASHED_UNDERLINE` set and others clear.
  - Semantic pin: `tack_cap_xcheck_smulx_transitions_clear_previous` — feed `\E[4:3m\E[4:4m` (curly → dotted), assert ONLY `DOTTED_UNDERLINE` is set (not both — this catches the "bitflag-or instead of replace" regression).
  `CellFlags` variants verified at `oriterm_core/src/cell/mod.rs:36-42` — no extension needed. The registry table entry is a single `("Smulx", tack_cap_xcheck_smulx)` where `tack_cap_xcheck_smulx` dispatches through the matrix via a parametrized helper.

- [ ] **`Setulc`** (underline color — `\E[58:2::%p1%{65536}%/%d:...`). Feed a truecolor SGR 58 sequence (`\E[58:2::255:100:50m`), assert the cell template's `underline_color: Option<Color>` field (VERIFIED EXISTS — defined at `oriterm_core/src/cell/mod.rs:63`; setter `Cell::set_underline_color` at line 196) contains `Some(Color::Rgb(Rgb { r: 255, g: 100, b: 50 }))`. Also feed `\E[59m` (reset underline color), assert the field becomes `None`.

- [ ] **`Sync`** (synchronized output mode 2026 — `\E[?2026h` / `\E[?2026l`). Three `#[test] fn`s in `tack_cap_xcheck/sync.rs`:
  - `tack_cap_xcheck_sync_enters_sync_update` — feed DECSET 2026, assert `term.mode().contains(TermMode::SYNC_UPDATE)` (mode flag name verified against `oriterm_core/src/term/handler/modes.rs:83`).
  - `tack_cap_xcheck_sync_exits_sync_update` — pre-seed SYNC_UPDATE, feed DECRST 2026, assert cleared.
  - `tack_cap_xcheck_sync_cap_declaration_matches` — call `assert_cap_declaration_matches("Sync", b"\x1b[?2026%?%p1%{1}%-%tl%eh%;")` — note the printf-style conditional because terminfo stores the parameterized form.

- [ ] **`BD` + `BE`** (bracketed paste off/on — `\E[?2004l` / `\E[?2004h`). Four `#[test] fn`s in `tack_cap_xcheck/bracketed_paste.rs`:
  - `tack_cap_xcheck_be_enters_bracketed_paste` — feed DECSET 2004, assert `term.mode().contains(TermMode::BRACKETED_PASTE)` after the single escape.
  - `tack_cap_xcheck_bd_exits_bracketed_paste` — pre-seed BRACKETED_PASTE via DECSET 2004, feed DECRST 2004, assert the flag is CLEARED (not just "not set on a fresh term").
  - `tack_cap_xcheck_bracketed_paste_idempotent_on` — feed `\E[?2004h\E[?2004h` and assert the flag is still set exactly once (no "double-set" visible state).
  - `tack_cap_xcheck_bracketed_paste_cap_declaration_matches` — call `assert_cap_declaration_matches("BE", b"\x1b[?2004h")` and `("BD", b"\x1b[?2004l")` to pin the terminfo↔VTE mapping.
  `oriterm_core/src/term/handler/helpers.rs:47,76` implements bracketed paste — cross-reference.

- [ ] **`PS` + `PE`** (paste start / end markers — `\E[200~` / `\E[201~`). These are OUTBOUND bytes emitted by the pure function `oriterm_core::paste::prepare_paste(text, bracketed=true, filter)` at `oriterm_core/src/paste/mod.rs:11-14`. The function is a standalone pure transformation (no clipboard, no platform integration) and is ALREADY unit-tested at `oriterm_core/src/paste/tests.rs:210` (empty string → `\x1b[200~\x1b[201~`) and `:243` (`"hello"` → `\x1b[200~hello\x1b[201~`). Section 06's job is to add an explicit cap-xcheck entry that cross-references the cap declaration.

  **Not cross-crate.** Earlier drafts misidentified PS/PE as `oriterm`-owned; the `oriterm/src/app/clipboard_ops/mod.rs:176` site merely CALLS `paste::prepare_paste` — the byte emission lives in core. The 06.5 test therefore lives in-crate in `oriterm_core/src/term/handler/tack_cap_xcheck/bracketed_paste.rs` alongside the BD/BE tests (which also exercise bracketed-paste semantics).

  Concrete task: add `#[test] fn tack_cap_xcheck_ps` and `#[test] fn tack_cap_xcheck_pe` to `tack_cap_xcheck/bracketed_paste.rs` that each: (a) call `assert_cap_declaration_matches("PS", b"\x1b[200~")` / `("PE", b"\x1b[201~")` to cross-reference `extra/ori_term.info:211`, (b) call `oriterm_core::paste::prepare_paste("marker", true, false)` and assert the returned bytes are exactly `b"\x1b[200~marker\x1b[201~"`, (c) register the two test fns in the `XCHECK_TEST_FNS` registry table so the meta-test sees them. No cross-crate stub required; no new `oriterm` test. The registry table entry reads `("PS", tack_cap_xcheck_ps), ("PE", tack_cap_xcheck_pe)` as plain in-crate function pointers.

- [ ] **`Se` + `Ss`** (DECSCUSR reset / set — `\E[2 q` / `\E[%p1%d q`). Feed DECSCUSR with parameter 5 (blinking bar cursor), assert the cursor style flag reflects the change. `oriterm_core/src/term/handler/dcs.rs:18` implements DECSCUSR — cross-reference.

- [ ] **`XF`** (focus event support bool — no escape sequence, just an advertisement). The cap is a boolean marker indicating ori_term sends focus events. Test: parse `extra/ori_term.info`, assert `XF` is present as a bool cap. (This is a terminfo-parser test, not a VTE test.)

- [ ] **`kxIN` + `kxOUT`** (focus-in / focus-out markers — `\E[I` / `\E[O`). These are the ONLY genuinely cross-crate Track B caps. They are OUTBOUND bytes produced by `oriterm/src/app/event_loop_helpers/mod.rs:143 send_focus_event` in response to winit `WindowEvent::Focused` events — the winit dependency anchors emission in `oriterm`, not `oriterm_core`. The canonical paste entry point was already verified: `rg -n "\\[I" oriterm/src/app/event_loop_helpers/mod.rs` returns line 153 (`let seq: &[u8] = if focused { b"\x1b[I" } else { b"\x1b[O" };`).

  Concrete task: (a) add a sibling `#[cfg(test)] mod tests;` at the bottom of `oriterm/src/app/event_loop_helpers/mod.rs` if one doesn't already exist, with a sibling `tests.rs` file. (b) The sibling test constructs a minimal `App` (or whatever test scaffold `event_loop_helpers` already has — check for existing tests first via `Grep` for `#[test]` inside `oriterm/src/app/event_loop_helpers/`), sets `FOCUS_IN_OUT` mode on a test pane, calls `send_focus_event(true)`, and asserts the pane's writer captured `b"\x1b[I"`. Repeat with `send_focus_event(false)` for `\x1b[O`. (c) Add cross-crate stubs `("kxIN", cross_crate_stub::kxin_see_oriterm_focus)` and `("kxOUT", cross_crate_stub::kxout_see_oriterm_focus)` in `tack_cap_xcheck/mod.rs`'s registry table; each stub is a one-line `fn` that asserts `true` and carries a `#[doc]` attribute naming the real test location. (d) Coverage still attributed to Section 06 in `cap_coverage/section_06.rs::CONTRIBUTION.covered` regardless of which crate the test executes in.

- [ ] **`Tc` + `RGB`** (truecolor support bool + direct-color marker). `Tc` is a bool cap; `RGB` is a direct-color advertisement. Both are static terminfo declarations — test by parsing `extra/ori_term.info` and asserting both are present. Additionally, feed a direct-color SGR (e.g., `\E[38:2::255:100:50m`) into a `Term` and assert the cell template's fg color is set to RGB(255, 100, 50) — this validates that ori_term's SGR parser honors the direct-color sub-params.

- [ ] **`Cr` + `Cs`** (cursor color reset / set — `\E]112\x07` / `\E]12;%p1%s\x07`). Feed OSC 12 with a color arg (for set) or OSC 12 `?` (for query); for queries use `Term<PtyResponder>` from 06.0.c's extension and assert `responder.take_osc_responses()` contains a formatted response matching oriterm_core's OSC 12 reply format; for sets assert `term.cursor_color()` (or the equivalent accessor — verify in `oriterm_core/src/term/handler/osc.rs`) has the new value. OSC 112 is the reset path — assert `term.cursor_color()` returns `None` / default after it fires.

- [ ] **`Ms`** (clipboard via OSC 52 — `\E]52;%p1%s;%p2%s\x07`). Feed OSC 52 `c;<base64>` (store), assert `responder.take_clipboard_stores()` contains the `(Clipboard, decoded_text)` tuple. Feed OSC 52 `c;?` (query), assert `responder.take_osc_responses()` contains the base64-encoded response matching the pinned test clipboard string the responder was seeded with.

- [ ] **`hs` + `dsl` + `fsl` + `tsl`** (status line support + disable / finish / to-status-line). **Contract: OSC 0/2 title-backed status line.** Per `plans/tack-conformance/section-02-terminfo-provisioning.md:177`, Section 02 explicitly declared `hs`/`dsl`/`tsl`/`fsl` as title-backed via `oriterm_core/src/term/handler/osc.rs:22 osc_set_title` — this is the Alacritty convention (`alacritty+common` fragment, `alacritty.info:108`) and matches the declarations in `extra/ori_term.info:194-196` (`hs, dsl=\E]2;\007, fsl=^G, tsl=\E]2;`). The contract is: `tsl=\E]2;,` opens a title-write; any text fed between `tsl` and `fsl` becomes the title payload; `fsl=^G` terminates the title; `dsl=\E]2;\007` writes an empty title (clearing the status line). This means a direct-VTE round-trip MUST assert title-backed behavior — NOT "feature not yet implemented" (which is a banned anti-pattern per CLAUDE.md).
  **Test matrix** (four `#[test] fn`s in `tack_cap_xcheck/status_line.rs`):
  1. `tack_cap_xcheck_tsl_fsl_round_trip`: construct `Term<RecordingListener>`, feed the literal byte sequence `tsl` + `"test status line"` + `fsl` (i.e. `b"\x1b]2;test status line\x07"`), assert the recording listener captured `Event::Title("test status line")` AND `term.title() == "test status line"`. This exercises the full open+payload+close path that `tsl`+text+`fsl` builds — the `tsl` and `fsl` caps are NOT tested in isolation; they are tested as the matched pair they form in any real terminfo consumer.
  2. `tack_cap_xcheck_dsl_clears_title`: with a pre-existing title set via step 1's path, feed the `dsl` sequence `b"\x1b]2;\x07"` and assert `Event::Title("")` fires AND `term.title().is_empty()`. This pins the empty-payload clear-to-default contract.
  3. `tack_cap_xcheck_hs_bool_declared`: assert `parse_declared_caps()` contains `"hs"` — the bool marker is verified via terminfo parsing, not VTE feed (bool caps have no escape sequence).
  4. `tack_cap_xcheck_status_line_cross_reference`: call `assert_cap_declaration_matches("dsl", b"\x1b]2;\x07")` and the same for `tsl` / `fsl` to pin that the declarations in `extra/ori_term.info:196` still match the literal bytes Track B feeds. A future edit to the terminfo that changes the sequences fails this test BEFORE the event-firing assertions run.
  Cross-reference declarations: `extra/ori_term.info:194-196` for the cap declarations; `oriterm_core/src/term/handler/osc.rs:22 osc_set_title` for the `Event::Title` emission path; `plans/tack-conformance/section-02-terminfo-provisioning.md:177` for the contract declaration. Each test is written failing-first per the TDD rule.

- [ ] **`AX` + `XT`** (xterm extension markers). Bool caps with no escape sequence. Test by parsing the terminfo and asserting presence.

- [ ] **Meta-test: `tack_cap_xcheck_covers_every_non_tack_cap`.** Declare:
  ```rust
  pub const NON_TACK_CAP_XCHECK_CAPS: &[&str] = &[
      "Smulx", "Setulc", "Sync",
      "BD", "BE", "PS", "PE",
      "Se", "Ss",
      "XF", "kxIN", "kxOUT",
      "Tc", "RGB",
      "Cr", "Cs", "Ms",
      "hs", "dsl", "fsl", "tsl",
      "AX", "XT",
  ];
  ```
  Meta-test iterates this list and asserts every cap has a `#[test] fn` whose name contains the cap name (lowercase, with a `tack_cap_xcheck_` prefix). Implementation: use `std::any::type_name` or a build-time registration macro. Simpler: declare a `const XCHECK_TEST_FNS: &[(&str, fn())]` registry table, and the meta-test iterates both arrays asserting every cap in `NON_TACK_CAP_XCHECK_CAPS` has an entry in `XCHECK_TEST_FNS` (and vice versa).
  ```rust
  #[test]
  fn tack_cap_xcheck_covers_every_non_tack_cap() {
      let registered: std::collections::BTreeSet<&str> =
          XCHECK_TEST_FNS.iter().map(|(cap, _)| *cap).collect();
      let expected: std::collections::BTreeSet<&str> =
          NON_TACK_CAP_XCHECK_CAPS.iter().copied().collect();
      assert_eq!(
          registered, expected,
          "direct-VTE cap xcheck drift: a cap was added to \
           NON_TACK_CAP_XCHECK_CAPS without a backing test fn (or \
           vice versa).\n\
           Only in NON_TACK_CAP_XCHECK_CAPS: {:?}\n\
           Only in XCHECK_TEST_FNS: {:?}",
          expected.difference(&registered).collect::<Vec<_>>(),
          registered.difference(&expected).collect::<Vec<_>>(),
      );
  }
  ```

- [ ] **Cross-reference helper.** A helper `assert_cap_declaration_matches(cap_name: &str, expected: &[u8])` that reads `extra/ori_term.info` via `parse_declared_caps`, finds the cap by name, and asserts its declared escape sequence bytes match `expected`. Every direct-VTE test calls this helper before feeding — if the terminfo drifts, the cross-reference fires first with a diagnostic message naming the cap and the old vs new sequence.

- [ ] **File-size proactive split.** `tack_cap_xcheck/mod.rs` holds the type defs, registry tables, and meta-test. Implementation of the individual `#[test] fn`s goes in grouped submodules: `tack_cap_xcheck/sgr_extensions.rs` (Smulx, Setulc), `tack_cap_xcheck/sync.rs` (Sync), `tack_cap_xcheck/bracketed_paste.rs` (BD, BE, PS, PE), `tack_cap_xcheck/cursor_style.rs` (Se, Ss), `tack_cap_xcheck/focus_events.rs` (XF, kxIN, kxOUT), `tack_cap_xcheck/truecolor.rs` (Tc, RGB), `tack_cap_xcheck/osc_color.rs` (Cr, Cs), `tack_cap_xcheck/osc_clipboard.rs` (Ms), `tack_cap_xcheck/status_line.rs` (hs, dsl, fsl, tsl), `tack_cap_xcheck/xterm_markers.rs` (AX, XT). Each submodule stays under 150 lines. `mod.rs` is the dispatch hub with the registry + meta-test only.

- [ ] **Cap coverage extension for 06.5:** all 23 Track B caps move from `section_06.rs::CONTRIBUTION.exempt` → `CONTRIBUTION.covered` in lockstep with each test landing. The stale-exemption negative pin catches any missed cleanup.

- [ ] **[DRIFT] Fix stale `osc_queries scenario` comments in `cap_coverage/section_06.rs:41-151`.** Every exempt entry in the pre-Section-06 file cites `"Section 06 osc_queries scenario"` as its deferral target — but Section 06's final design has NO `osc_queries` scenario module. The scenarios are `status_reports`, `sgr_modes`, `character_sets`, `enq_ack`, and the direct-VTE tests live in `tack_cap_xcheck/`. As each cap moves from `exempt` to `covered`, the stale "osc_queries scenario" text must be replaced with the actual owner (e.g., `Cr` → `"Section 06.5 tack_cap_xcheck osc_color (OSC 12/112 cursor color)"`; `hs/dsl/fsl/tsl` → `"Section 06.5 tack_cap_xcheck status_line (OSC 0/2 title-backed status line per Section 02:177)"`). For any cap that stays exempt (none should remain after 06.5 lands), rewrite its comment to name a real subsequent section, not `osc_queries`. Verification: `grep -n 'osc_queries' crates/oriterm_test_support/src/tack_framework/cap_coverage/section_06.rs` returns no matches after Section 06 closes.

- [ ] **Debug + release parity. Determinism: 10 reruns.** These tests run WITHOUT tack (they use `Term` directly), so they run on Windows / macOS / Linux identically — cross-platform gate is automatic.

---

## 06.6 Interactive exclusion stubs

**File(s):** `oriterm_core/tests/tack/tools_menu/{echo_tool,reply_tool,hex_output,change_debug_level,performance_testing,send_reset_init}.rs`

Per the 06.0 inventory, the following tools are interactive or duplicates and have doc-only stubs (no `#[test] fn` bodies):

- [ ] **`echo_tool.rs`**:
  ```rust
  //! Excluded: tack's `e) echo tool` is an interactive keyboard-echo
  //! probe — it reads from stdin and displays each keystroke as it's
  //! received. It cannot be automated from the PTY test harness
  //! because there is no "done" anchor and no deterministic exit
  //! condition. The related keyboard echo correctness is validated
  //! by Section 08's in-crate sibling tests at
  //! `oriterm/src/key_encoding/terminfo_xcheck.rs`.
  ```

- [ ] **`reply_tool.rs`**:
  ```rust
  //! Excluded: tack's `r) reply tool` prompts the user for a
  //! query/response pair to test. Interactive input required; cannot
  //! be automated. The DA/DSR query/response round-trip is validated
  //! instead by Section 06.1 status_reports scenarios (automated via
  //! tack's `s)` sub-submenu walker which runs the DA/DSR probes
  //! non-interactively).
  ```

- [ ] **`hex_output.rs`**:
  ```rust
  //! Excluded: tack's `h) enable hex output on echo tool` is a modal
  //! toggle on the echo tool. Since `e) echo tool` is excluded (see
  //! echo_tool.rs), toggling its hex mode is moot. If ori_term ever
  //! needs to validate hex-output rendering of received bytes, add
  //! a direct-VTE test in
  //! `oriterm_core/src/term/handler/tack_cap_xcheck/` (hex output is
  //! a display convention, not a terminfo cap).
  ```

- [ ] **`change_debug_level.rs`**:
  ```rust
  //! Excluded: tack's `d) change debug level` toggles tack's internal
  //! verbosity. It is a diagnostic control for tack itself, not a
  //! terminfo cap test. No ori_term behavior is exercised.
  ```

- [ ] **`performance_testing.rs`**:
  ```rust
  //! Excluded: tack's `p) performance testing` runs throughput probes
  //! (scroll speed, character rate) that are already covered by
  //! Section 05's `p) test padding and string capabilities` scenarios.
  //! See `oriterm_core/tests/tack/test_menu/padding.rs` for the
  //! canonical padding/perf coverage. No duplicate testing.
  ```

- [ ] **`send_reset_init.rs`**:
  ```rust
  //! Excluded: tack's `i) send reset and init` overlaps with the
  //! identically-named begin-testing `i) send reset and init`
  //! exclusion stub from Section 05.0's BEGIN_TESTING_INVENTORY. The
  //! canonical exclusion lives in
  //! `oriterm_core/tests/tack/test_menu/send_reset_init.rs`. This
  //! file exists only to satisfy the drift gate in
  //! `TOOLS_MENU_INVENTORY` — any test of the reset/init sequences
  //! belongs in Section 05's location.
  ```

- [ ] **Verify `cargo clippy -p oriterm_core --tests` produces NO warnings on the stubs.** Rust accepts module files containing only `//!` doc comments; clippy does NOT emit dead-code warnings for empty modules. Confirm.

- [ ] **Wire all stubs into `oriterm_core/tests/tack/tools_menu/mod.rs`:**
  ```rust
  //! Tack `t) tools` submenu scenarios — see Section 06.

  pub mod change_debug_level;
  pub mod character_sets;
  pub mod echo_tool;
  pub mod enq_ack;
  pub mod hex_output;
  pub mod performance_testing;
  pub mod reply_tool;
  pub mod send_reset_init;
  pub mod sgr_modes;
  pub mod status_reports;
  pub mod status_reports_inventory;
  pub mod tools_menu_inventory;
  // osc_responder_integration lives here too if 06.0.c lands it
  // in the tools_menu test target instead of a sibling location.
  ```

---

## 06.7 Determinism + size matrix + cross-compile verification

**File(s):** No new files — this is the gate subsection that runs the verification matrix before 06.N closes.

- [ ] **Determinism: 10 consecutive runs of `timeout 150 cargo test -p oriterm_core --test tack -- tools_menu` all pass.** Any flake is a bug — file via `/add-bug` and fix immediately (never retry loop).

- [ ] **Determinism: 10 consecutive runs of `timeout 150 cargo test -p oriterm_core -- term::handler::tack_cap_xcheck` all pass.** These tests don't use `PtySession` so they should be stable; any flake indicates non-determinism in `Term::feed` or the recording listener.

- [ ] **`--test-threads=1` and `--test-threads=4` both pass for both test groups.** Note: Windows `PtySession` tests are serialized by `CONPTY_LIFETIME_LOCK`, so the parallelism gate for 06.1–06.4 is Linux/macOS-only. 06.5 direct-VTE tests run fully parallel on all platforms because they don't use `PtySession`.

- [ ] **Cross-compile gates:**
  - `cargo build --target x86_64-pc-windows-gnu -p oriterm_core --tests` succeeds.
  - `cargo build --target x86_64-pc-windows-gnu -p oriterm_test_support --tests` succeeds.
  - `cargo build --target x86_64-pc-windows-gnu -p oriterm_core --tests --release` succeeds.

- [ ] **Debug AND release parity (whole section):** every test in 06.0 – 06.5 passes in BOTH `cargo test` and `cargo test --release`. Any release-only failure is a timing bug fixed in 06.7 — never deferred.

- [ ] **Cap-coverage matrix gate:** `timeout 150 cargo test -p oriterm_core --test tack -- cap_coverage_matrix` is green after Section 06's cap moves land. Specifically:
  - `section_06.rs::CONTRIBUTION.covered` contains all 27 entries (Track A: u6, u7, u8, u9 = 4 tack-reachable caps; plus Track B: Smulx, Setulc, Sync, BD, BE, PS, PE, Se, Ss, XF, kxIN, kxOUT, Tc, RGB, Cr, Cs, Ms, hs, dsl, fsl, tsl, AX, XT = 23 direct-VTE caps).
  - `section_06.rs::CONTRIBUTION.exempt` no longer contains any of those entries.
  - The stale-exemption negative pin does not fire.
  - Any remaining `exempt` entries in `section_06.rs` are justified by a comment explaining why they belong to a future section or why they are unreachable by Section 06.

- [ ] **File-size check:** no file in `oriterm_core/tests/tack/tools_menu/`, `oriterm_core/src/term/handler/tack_cap_xcheck/`, or `crates/oriterm_test_support/src/tack_framework/scenarios/tools_*/` exceeds 500 lines. `tack_cap_xcheck/mod.rs` proactively splits into per-cap-family submodules per 06.5's task list.

- [ ] **`./build-all.sh` green.**
- [ ] **`./clippy-all.sh` green.**
- [ ] **`timeout 150 ./test-all.sh` green.**

---

## 06.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. Mandatory final TPR at 06.N. -->

- None yet.

---

## 06.N Completion Checklist (final TPR mandatory)

- [ ] **06.0 discovery complete:** `TOOLS_MENU_INVENTORY` pinned, `tack_tools_menu_inventory` test passing, drift gate active.
- [ ] **06.0.b nested discovery complete:** `STATUS_REPORTS_INVENTORY` pinned, `tack_status_reports_inventory` test passing.
- [ ] **06.0.c framework extension complete:** `session/pty_responder/{mod, tests}.rs` exists (proactive split), `PtyResponder` extended in-place with ColorRequest/ClipboardLoad/ClipboardStore handling, `take_osc_responses()` + `take_clipboard_stores()` accessors added, `PtySession::drain`/`drain_blocking` write OSC responses back automatically, existing vttest (198) + Section 05 tack (18) tests pass unchanged.
- [ ] **New `scenarios::menu_inventory` helper landed for Section 06 only:** `scenarios::menu_inventory::{assert_menu_drift, collect_menu_keys}` is the drift-gate + key-scanner home for `tools_menu_inventory` (06.0) and `status_reports_inventory` (06.0.b). Section 05's `begin_testing_inventory::assert_inventory_drift` (and its local `collect_menu_keys`) stays unchanged — the module doc of `menu_inventory/mod.rs` cross-references Section 05's helper and documents the intentional non-consumer per Codex midpoint review. No cross-section test file in Section 05 is touched.
- [ ] **06.1 status reports:** one `#[test] fn` per sub-test in STATUS_REPORTS_INVENTORY, all passing, cross-validated against `oriterm_core/tests/vttest/menu6.rs`.
- [ ] **06.2 SGR modes:** `tack_tools_sgr_80x24` passing, parser cross-validates 80 mode labels via `grid_has_token`.
- [ ] **06.3 character sets:** `tack_tools_g0_dec_graphics_80x24` passing, parser validates the DEC special graphics rendering against the empirical oriterm_core output format.
- [ ] **06.4 ENQ/ACK:** `tack_tools_enq_ack_80x24` passing, cross-referenced against `extra/ori_term.info` u8/u9.
- [ ] **06.5.a RecordingListener helper promotion complete:** `oriterm_core/src/term/handler/test_helpers.rs` exists with `pub(super) RecordingListener`, `term_with_recorder`, `term_with_recorder_sized`. `handler/tests.rs` line count decreased by ~40. The `recording_listener_captures_title_event` semantic pin is green. `cargo test -p oriterm_core -- handler::tests` shows zero new or broken tests from the move. 06.5 Cap-by-cap tasks can start.
- [ ] **06.5 direct-VTE cap xcheck:** every cap in `NON_TACK_CAP_XCHECK_CAPS` has a backing `#[test] fn`, meta-test `tack_cap_xcheck_covers_every_non_tack_cap` passes, cross-reference helper asserts terminfo declaration matches. **Cross-crate tests landed:** `oriterm/src/app/event_loop_helpers/tests.rs` contains the kxIN/kxOUT emission tests (the ONLY genuinely cross-crate Track B caps); PS/PE tests live in-crate in `oriterm_core/src/term/handler/tack_cap_xcheck/bracketed_paste.rs` because the byte-emitting `prepare_paste` pure function is in `oriterm_core`.
- [ ] **06.6 exclusion stubs:** six stubs in place, `cargo clippy` clean.
- [ ] **06.7 determinism + cross-compile:** 10 reruns clean for both test groups, `--test-threads=1` and `--test-threads=4` both pass (Linux/macOS for PtySession-using tests; all platforms for 06.5), cross-compile gates pass for debug AND release, cap-coverage matrix green.
- [ ] **Cap-coverage extension (cross-section sync from Section 05.5).** Section 06 owns `crates/oriterm_test_support/src/tack_framework/cap_coverage/section_06.rs`. ALL 27 entries (u6/u7/u8/u9 + Smulx/Setulc/Sync + BD/BE/PS/PE + Se/Ss + XF/kxIN/kxOUT + Tc/RGB + Cr/Cs/Ms + hs/dsl/fsl/tsl + AX/XT) have moved FROM `CONTRIBUTION.exempt` INTO `CONTRIBUTION.covered`. 27 = 4 tack-reachable (Track A: 06.1 + 06.4) + 23 direct-VTE (Track B: 06.5). The doc comment at the top of `section_06.rs` reflects that Section 06 has landed and the remaining exempt entries (if any) each have a justification. `cap_coverage/section_06.rs::CONTRIBUTION.exempt` SHOULD be empty.
- [ ] **Mission criterion traceability table reflects the final subsections** and cites mission criterion by TEXT, not by number.
- [ ] **Cross-validation against vttest menu6 passes:** the DA/DSR responses captured by 06.1 match the responses asserted by `oriterm_core/tests/vttest/menu6.rs`. Section 09 verification can diff them.
- [ ] **All parsers have sibling-file unit tests** including substring-collision negative pins that prove the token-match helpers are the only detection path.
- [ ] **Debug + release parity verified** for every test in 06.0 – 06.5.
- [ ] **File-size check:** no file in the Section 06 additions exceeds 500 lines.
- [ ] **`./build-all.sh` green.**
- [ ] **`./clippy-all.sh` green.**
- [ ] **`timeout 150 ./test-all.sh` green.**
- [ ] **Plan annotation cleanup:** all `<!-- reviewed: ... -->` markers from prior review passes stripped.
- [ ] **All TPR checkpoint findings resolved (see `06.R`).**
- [ ] **Plan sync:**
  - [ ] Section frontmatter `status` → `complete`
  - [ ] `00-overview.md` Quick Reference table updated (Section 06 → Complete)
  - [ ] `00-overview.md` Mission Success Criteria checkbox for the tools-menu criterion ticked
  - [ ] `index.md` Section 06 "Status: Complete"
  - [ ] **Section 07 const-path verify:** confirm `crates/oriterm_test_support/src/tack_framework/scenarios/character_sets/mod.rs` exposes `TACK_TOOLS_G0_DEC_GRAPHICS` at exactly that module path (no `tools_character_sets` prefix) so Section 07's `depends_on_contract` import at `section-07-gpu-golden-images.md:392` (`use oriterm_test_support::tack_framework::scenarios::character_sets::TACK_TOOLS_G0_DEC_GRAPHICS;`) compiles when Section 07 lands. `cargo check -p oriterm --tests` (or the `grep` equivalent) is the verification command.
  - [ ] **Section 09 count sync:** confirm `plans/tack-conformance/section-09-verification.md` scenario counts at its verification matrix and success criteria still match the FINAL Section 06 counts (Section 09 currently claims "18 test_menu + ~12 tools_menu active + ~23 direct-VTE cap xcheck from Section 06 Track B"). If Section 06's final `#[test] fn` count differs, update Section 09 in the same commit.
  - [ ] **Section 05.5 dependency cite:** confirm `parse_declared_caps()` still lives at `crates/oriterm_test_support/src/tack_framework/cap_coverage/mod.rs:240` (already landed, Section 05 complete). No edits needed — this is a sanity check that Section 06's Track B is not citing a phantom API.
- [ ] **Final `/tpr-review` clean pass.** Per CLAUDE.md, the section cannot close without a clean final TPR — findings get FIXED, never reasoned out of. This is in addition to mid-section TPR checkpoints (after M1 and after 06.1).
- [ ] **Final `/impl-hygiene-review last commit` clean pass** (after the final TPR).

**Exit Criteria:** `timeout 150 cargo test -p oriterm_core --test tack -- tools_menu` runs every tools-menu scenario (tools_menu_inventory discovery + status_reports_inventory discovery + ~8 status_reports scenarios + sgr_modes + character_sets + enq_ack = ~12 active `#[test] fn`s, plus 6 doc-only stubs) deterministically. `timeout 150 cargo test -p oriterm_core -- term::handler::tack_cap_xcheck` runs 21 in-crate direct-VTE cap tests (Track B minus kxIN/kxOUT) + meta-test deterministically. `timeout 150 cargo test -p oriterm -- event_loop_helpers` runs the 2 cross-crate focus-event tests (kxIN/kxOUT). The cap-coverage matrix asserts all 27 Section 06 caps (4 tack-reachable + 23 direct-VTE) are covered. Section 06 closes with the entire tack conformance catalog at ~53 active test scenarios across Sections 05 + 06 (18 test_menu + ~12 tools_menu + 21 in-crate direct-VTE cap xcheck + 2 cross-crate focus-event tests).
