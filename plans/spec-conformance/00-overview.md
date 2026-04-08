---
plan: "spec-conformance"
title: "Spec Conformance: Universal Terminal Protocol Verification"
status: not-started
supersedes:
  - "plans/tack-conformance/"
references:
  - "plans/spec-conformance/research.md"
  - "plans/tack-conformance/"
  - "/home/eric/.claude/projects/-home-eric-projects-ori-term/memory/architecture_graphics_audit.md"
  - "/home/eric/.claude/projects/-home-eric-projects-ori-term/memory/reference_notcurses_demo.md"
  - "/home/eric/.claude/projects/-home-eric-projects-ori-term/memory/reference_wezterm_graphics.md"
---

# Spec Conformance: Universal Terminal Protocol Verification

## Mission

Make ori_term the most spec-complete terminal emulator ever built. Implement and conform to **every published terminal protocol specification** — historical, modern, de-facto, obscure — and *prove* conformance via end-to-end test ladders that drive each sequence as far up the pipeline as it can possibly go. Code existing is **not** "support." Verification is "support." `notcurses-demo` running cleanly through all 28 scenes is the **first major integration milestone**, not the endpoint. The endpoint is: every spec exists in the catalog, every sequence has a complete verification chain, every applicable catalog row is `verified`, and no terminal emulator on Earth matches our verified coverage.

This is an open-ended, multi-year mission. The plan is structured to accept indefinite expansion as new specs emerge or are discovered.

## Mission Success Criteria

Each criterion is concrete and testable. Together they prove the mission is complete. Every criterion traces to at least one section that delivers it.

- [ ] **Catalog complete** — Every published terminal protocol spec ori_term targets is enumerated under `plans/spec-conformance/catalog/`. No row is `MISSING` without a tracked decision. Delivered by section 01 + per-stack additions in sections 08–20.
- [ ] **Verification chain complete per row** — Every applicable catalog row reaches `verified` status (parser → dispatch → state/effect → apex test ladder, all green). Delivered by sections 08–20.
- [ ] **Coverage report green** — `cargo run -p oriterm_test_support --bin spec-coverage-report` produces 100% verified status for every in-scope stack and only ever increases. Delivered by section 04 (generator) + section 23 (CI integration).
- [ ] **`notcurses-demo` runs cleanly** — All 28 scenes pass against per-scene correctness criteria with zero visual glitches, zero tearing, zero ghosting on the canonical golden lane (Linux/x86_64 + llvmpipe). Delivered by section 24.
- [ ] **Real-app E2E milestones pass** — vim, htop, btop, tmux, aerc, helix, ncmpcpp, less, nvim all run a recorded session through ori_term and produce a snapshot identical to the golden. Delivered by section 25.
- [ ] **Cross-stack regression sweep green** — Every PR runs every stack's verification chain in CI. A row dropping from `verified` to any lower status is a build failure. Delivered by section 23.
- [ ] **Effect/State separation enforced** — The `oriterm_core::effect::Effect` type is the production interface for all boundary-crossing side effects. `Event::ClipboardLoad` and `Event::ColorRequest` closures are removed. `Term::pending_notifications` bypass is absorbed into `EffectSink::take_pending()`. Delivered by section 03.
- [ ] **Mode 2026 fully wired** — Both publication suppression AND the timeout-abort path are wired. `Processor::sync_timeout` and `stop_sync` are called from `oriterm_mux/src/pane/io_thread/mod.rs` with a documented timeout. Delivered by section 06.
- [ ] **DEC mode metadata LEAK fixed** — The 5-sync-point LEAK across `NamedPrivateMode` consumers is collapsed into a single registry table. Mode metadata becomes data; behavior stays in match arms. Delivered by section 06.
- [ ] **Image lifecycle correct under resize/reflow/scrollback/alt-screen** — Image placements survive every grid transformation correctly. Documented invariants tested on a regression matrix. Delivered by section 07.
- [ ] **Deterministic golden environment** — Canonical golden lane uses pinned software rasterizer (llvmpipe), grayscale alpha hinting, pinned cell metrics, exact-or-tiny pixel tolerance. Delivered by section 05.
- [ ] **`tack-conformance` plan absorbed and superseded** — `plans/tack-conformance/` is marked superseded with a mapping table from spec catalog row IDs to legacy tack section IDs. Existing in-flight section 06.* stays in place; new tack sections 07-09 are created directly under spec-conformance. Delivered by section 02.
- [ ] **`./test-all.sh` green** — All tests pass debug + release. No regressions in `oriterm_core/tests/alloc_regression.rs` or `oriterm/src/app/event_loop_helpers/tests.rs`.
- [ ] **`./build-all.sh` green** — Cross-platform build succeeds (x86_64-pc-windows-gnu via WSL, native Linux, native macOS).
- [ ] **`./clippy-all.sh` green** — No new clippy warnings under the project's `deny(clippy::all)` + nursery.
- [ ] **All section success criteria met** — Every section's exit criteria checked off in its own success criteria block.

## Architecture

The verification chain harness observes a sequence as it climbs the ori_term pipeline. Each rung has a test; a row is `verified` only when every applicable rung is green.

```
PTY input bytes
       │
       │ ┌──── Rung 1: Parser test ────┐
       ▼ │ asserts the bytes are       │
crates/vte/src/ansi/processor.rs       │ recognized as the correct
       │ │ sequence with correct      │
       │ │ params extracted           │
       │ └────────────────────────────┘
       │
       │ ┌──── Rung 2: Dispatch test ───┐
       ▼ │ asserts the correct          │
crates/vte/src/ansi/dispatch/*.rs       │ TermHandler method is
       │ │ invoked with correct args   │
       │ └─────────────────────────────┘
       │
       │ ┌──── Rung 3: State or Effect test ──┐
       ▼ │ orthogonal observables:             │
oriterm_core/src/term/handler/*.rs    │ State: term internal state │
       │ │ mutates correctly           │
       │ │ Effect: boundary-crossing  │
       │ │ side effect emitted        │
       │ └────────────────────────────┘
       │
       ├─── Effect transcript apex ───→ EffectSink observes
       │    (PTY writes, Host requests, UI hints, Presentation gates)
       │
       │ ┌──── Rung 4: Renderable snapshot test ──┐
       ▼ │ asserts RenderableContent fields       │
oriterm_core/src/term/renderable/mod.rs           │ are correct after the sequence
       │ │ (cells, palette, modes, images,        │
       │ │ hyperlinks, cursor, damage)            │
       │ └───────────────────────────────────────┘
       │
       │ ┌──── Rung 5: FrameInput / prepared scene test ──┐
       ▼ │ asserts FrameInput composes correctly          │
oriterm/src/gpu/frame_input/mod.rs                        │ from RenderableContent + viewport
       │ │ + font + palette                                │
       │ └────────────────────────────────────────────────┘
       │
       │ ┌──── Rung 6: GPU instance buffer test ──┐
       ▼ │ asserts instance writers contain        │
oriterm/src/gpu/prepare/emit.rs                    │ expected vertices, UVs, colors,
       │ │ z-order                                 │
       │ └────────────────────────────────────────┘
       │
       │ ┌──── Rung 7: Texture render test ──┐
       ▼ │ exercise render_frame_cached()    │
GpuRenderer::render_frame_cached()           │ on offscreen target with COPY_DST
       │ │ usage; read back pixels           │
       │ └──────────────────────────────────┘
       │
       │ ┌──── Rung 8: Golden image test (apex for visual) ──┐
       ▼ │ assert pixels match committed PNG within          │
visual_regression/<stack>/<id>.png            │ exact-or-tiny per-pixel tolerance.
                                              │ Software rasterizer (llvmpipe)
                                              │ pinned for determinism.
                                              └──────────────────────────────────┘

For non-visual sequences, the chain stops at its natural apex:
       parser → dispatch → state → State snapshot apex
                       OR → effect → Effect transcript apex
```

The harness exposes **two orthogonal observables**, each with a typed surface, and the verification chain observes whichever apex applies to the sequence under test.

```
Effect type family (lives at oriterm_core::effect::Effect):
  ┌─ Pty(PtyEffect)            ── PTY writes (replies, queries answered, mouse encoding,
  │                                focus/keyboard encoding, image protocol replies)
  ├─ Host(HostEffect)          ── Fire-and-forget host platform calls:
  │                                Bell, DesktopNotification, TitleSet, IconNameSet, CwdSet,
  │                                CommandComplete, ChildExit, AudioRequest, PrintRequest
  ├─ HostRequest(HostRequest)  ── Typed request/response (NOT closures):
  │                                ClipboardLoad { sel, reply: ResponseToken },
  │                                ColorQuery { index, reply: ResponseToken }
  ├─ Ui(UiEffect)              ── UI hints: CursorBlinkChanged, MouseCursorDirty
  └─ Presentation(PresentationEffect) ── Sync gates:
                                   SyncBegin, SyncCommit { snapshot_seqno },
                                   SyncAbort { reason: SyncAbortReason }

State observables (already exist in RenderableContent + Term):
  RenderableContent: cells, cursor, palette, mode bits, image placements,
                     image data, hyperlinks (in cells), damage, scrollback
  Term internal:     scroll regions, charset state, attribute stack, tab stops,
                     title (cached), cwd (cached), prompt markers, keyboard mode stack
```

## Design Principles

**Three principles drive every design decision in this plan.**

### 1. Verification chain, not single tests

A sequence is verified when every layer of the pipeline that the sequence touches has a passing test. Visual sequences have long chains ending in golden images; non-visual sequences have shorter chains ending at their natural apex (PTY reply byte stream, mode flag state, clipboard mock, title callback, audio mock, etc.). **Gaps in the middle invalidate verification of higher layers.** A test that asserts pixels look right but never asserts grid state can be coincidentally green while the grid is silently wrong — a regression in grid mutation may not visibly differ in pixels but will break OTHER sequences that depend on the same grid state.

This principle motivates the two-observable design: state snapshots and effect transcripts are orthogonal, neither subsumes the other, and the test ladder observes both at every rung where they are both relevant.

### 2. Single Source of Truth for protocol metadata

The catalog is the single source of truth for what ori_term supports. The catalog determines:
- Test names (every test cites a catalog row ID)
- Coverage tracking (rows-by-status counted by automated report)
- Per-stack conformance percentage (only `verified` rows count)
- Regression detection (a row dropping status is a build failure)

This complements the existing SSOT discipline in `.claude/rules/impl-hygiene.md` (Single Source of Truth section). The catalog rows are the canonical home for "what is the spec contract for this sequence?"; consumers (test names, coverage reports, plan sections) query the catalog rather than maintaining parallel lists.

The DEC mode handling LEAK (currently 4 sync points across `crates/vte/src/ansi/types.rs:226-295`, `types.rs:175`, `oriterm_core/src/term/handler/helpers.rs:22,56`, and `modes.rs:17-102`) is an existing SSOT violation and is fixed in section 02 — mode metadata becomes a single registry table that all consumers query.

### 3. Boundary-crossing effects are first-class, not afterthoughts

The current `Event` enum mixes state changes (Title, Cwd, ChildExit), boundary crossings (PtyWrite, ClipboardStore), closure-based requests (ClipboardLoad, ColorRequest), and transport noise (Wakeup). The new `Effect` type cleanly separates these: state changes are observed via state snapshots (not effects), fire-and-forget effects use the `Effect::Host` family, request/response patterns use the `Effect::HostRequest` family with typed reply tokens (NO closures), presentation gates use `Effect::Presentation`, and transport noise stays out of the effect transcript entirely.

**Critical**: fire-and-forget effects and request/response calls are different abstractions. They share an enum top-level for routing but they have different semantics — request/response carries a `ResponseToken` that the consumer uses to deliver the reply back to the terminal, which then formats the reply via its own effect emission (`Effect::Pty(PtyEffect::Write(...))`). This is the only architectural sane way to remove the current closure pattern without losing functionality.

## Section Dependency Graph

```
Phase 0a (catalog map of territory)
  ┌─────────────────────────┐
  │ 01 catalog-bootstrap    │
  │ (catalog/ files,        │
  │  no tests)              │
  └────────────┬────────────┘
               │
               ▼
Phase 0b (plan hygiene — mechanical, no implementation)
  ┌─────────────────────────┐
  │ 02 tack-conformance     │
  │ absorption              │
  │ (supersede notice +     │
  │  mapping table)         │
  └────────────┬────────────┘
               │
               ▼
Phase 1 (foundation — five focused sections, parallelizable)
  ┌────────────────┐  ┌──────────────────┐  ┌────────────────┐
  │ 03 effect      │  │ 04 verification  │  │ 05 golden lane │
  │ boundary       │  │ chain harness    │  │ determinism    │
  │ migration      │  │ + pilots +       │  │ (llvmpipe,     │
  │ (Effect type,  │  │ schema freeze +  │  │  hinting,      │
  │  closure       │  │ coverage report  │  │  pinned cell)  │
  │  removal)      │  │ generator)       │  └────────┬───────┘
  └────────┬───────┘  └────────┬─────────┘           │
           │                   │                     │
           ├───────────────────┼─────────────────────┤
           │                   │                     │
  ┌────────▼────────┐  ┌───────▼─────────┐
  │ 06 terminal     │  │ 07 image        │
  │ mode plumbing   │  │ lifecycle       │
  │ (Mode 2026      │  │ correctness     │
  │  timeout-abort  │  │ (resize/        │
  │  + mode meta-   │  │  reflow/        │
  │  data registry) │  │  scrollback/    │
  └────────┬────────┘  │  alt-screen)    │
           │           └───────┬─────────┘
           │                   │
           └─────────┬─────────┘
                     ▼
Phase 2 (baseline)
  ┌─────────────────────────┐
  │ 08 ECMA-48 baseline     │
  │ (drives the row subset  │
  │  that tack covers + new │
  │  baseline gaps)         │
  └────────────┬────────────┘
               │
   ┌───────────┼───────────────────────────┐
   │           │                           │
   ▼           ▼                           ▼
Phase 3 (per-stack expansion — parallelizable groups)

Group A (parallel — pure data + handler stacks):
  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐
  │ 09 DEC       │  │ 10 OSC suite │  │ 11 Unicode       │
  │ private      │  │ (full        │  │ subcell glyphs   │
  │ modes        │  │ registry,    │  │ + octants        │
  │              │  │ FTCS, OSC 8, │  │                  │
  │              │  │ OSC 52, ...) │  │                  │
  └──────────────┘  └──────────────┘  └──────────────────┘

Group B (sequential — image stack):
  ┌──────────────┐
  │ 12 sixel     │
  └──────┬───────┘
         ▼
  ┌──────────────┐
  │ 13 kitty     │
  │ graphics     │
  └──────┬───────┘
         ▼
  ┌──────────────┐
  │ 14 iTerm2    │
  │ inline       │
  │ images       │
  └──────┬───────┘
         ▼
  ┌──────────────┐
  │ 15 cell-     │
  │ level alpha  │
  │ + transp     │
  └──────────────┘

Group C (sequential — input stacks):
  ┌──────────────┐
  │ 16 mouse     │
  │ protocols    │
  └──────┬───────┘
         ▼
  ┌──────────────┐
  │ 17 kitty     │
  │ keyboard     │
  └──────────────┘

Group D (parallel — independent):
  ┌──────────────┐  ┌──────────────────┐  ┌──────────────┐
  │ 18 charsets  │  │ 19 historical    │  │ 20 audio +   │
  │ + UAX policy │  │ stacks (VT52,    │  │ print        │
  │              │  │ ReGIS, Tek,      │  │              │
  │              │  │ Wyse, ADM-3A,    │  │              │
  │              │  │ ANSI.SYS)        │  │              │
  └──────────────┘  └──────────────────┘  └──────────────┘

   │                                       │
   ▼                                       ▼
Phase 4 (integration harnesses — sibling tracks, both depend on Phase 1)
  ┌────────────────────┐         ┌────────────────────┐
  │ 21 notcurses-demo  │         │ 22 real-app E2E    │
  │ harness + scene    │         │ harness            │
  │ matrix +           │         │ (PTY recording +   │
  │ qrcode smoke       │         │  replay)           │
  │ (early; lands      │         │ (early; lands      │
  │  after Phase 1)    │         │  after Phase 1)    │
  └────────┬───────────┘         └─────────┬──────────┘
           │                               │
           │                               │
           └────────────┬──────────────────┘
                        ▼
Phase 5 (continuous verification)
  ┌──────────────────────┐
  │ 23 cross-stack       │
  │ regression sweep +   │
  │ coverage report CI   │
  │ (lands after first   │
  │  ~3 stacks verified) │
  └──────────┬───────────┘
             │
             │ all stacks verified
             ▼
Phase 6 (final integration milestones — depend on every prior section)
  ┌──────────────────────┐         ┌──────────────────────┐
  │ 24 notcurses-demo    │         │ 25 real-app          │
  │ FULL-PASS milestone  │         │ FULL-PASS milestone  │
  │ (all 28 scenes pass) │         │ (vim, htop, btop,    │
  │                      │         │  tmux, aerc, helix,  │
  │                      │         │  ncmpcpp, less, nvim)│
  └──────────────────────┘         └──────────────────────┘
```

**Independent (parallelizable) section groups after Phase 1 lands:**
- **Phase 1 itself** (sections 03, 04, 05, 06, 07): Foundation sections are mostly independent of each other. 03 (Effect type) is a prerequisite for 04 (harness uses Effect for transcript capture). 04 is a prerequisite for the pilots in 05/06/07 (pilots are written against the harness API). 05/06/07 can be implemented concurrently after 03+04 land.
- **Phase 2** (section 08): ECMA-48 baseline is the gate to Phase 3 stacks. Sequential.
- **Phase 3 Group A** (sections 09, 10, 11): DEC private modes, OSC suite, Unicode subcell glyphs — independent of each other and of images/mouse.
- **Phase 3 Group B** (sections 12 → 13 → 14 → 15): Image stack, **sequential** because each builds on the prior (sixel exercises image cache → kitty extends with chunked transmission and animation → iTerm2 extends OSC dispatch with image protocol → cell-level alpha is the architectural prerequisite for translucent overlays in `trans` scene).
- **Phase 3 Group C** (sections 16 → 17): Mouse protocols → kitty keyboard. Sequential because kitty keyboard reuses mouse SGR encoder scaffolding.
- **Phase 3 Group D** (sections 18, 19, 20): Charsets/UAX policy, Historical stacks, Audio+print — independent.
- **Phase 4** (sections 21, 22): notcurses-demo harness and Real-app harness are **sibling tracks**, NOT in a chain. Both depend only on Phase 1.
- **Phase 5** (section 23): Cross-stack regression sweep CI lands once Phase 3 has produced ~3 verified stacks (so the report has something meaningful to display). It runs continuously after that.
- **Phase 6** (sections 24, 25): Final full-pass milestones depend on every prior section. They are SEPARATE sections from the harness sections (21, 22) so each has its own clear completion gate, avoiding the "forever-open section" anti-pattern.

**Cross-section interactions (must be co-implemented):**

- **Section 06 + Section 09**: The mode 2026 timeout-abort fix (section 06 adds the API call site in `oriterm_mux/src/pane/io_thread/mod.rs`) and the mode 2026 verification chain rows (section 09 enumerates them with their test ladders) must land together — the timeout test in the catalog is non-executable until the API call is wired.
- **Section 06 + Section 08**: The mode metadata registry refactor (section 06 fixes the LEAK) and the ECMA-48 baseline mode rows (section 08 verifies the basic-mode subset) must land together — adding new modes in section 08 against the unfixed LEAK creates 5-sync-point drift bugs immediately.
- **Section 07 + Section 21**: Image+resize/reflow correctness and notcurses-demo harness must land together. The `keller` scene resizes during the all-7-blitters test, and any image lifecycle bug breaks the scene's correctness criterion. Section 07 lands the fix; section 21 verifies it under harness stress before section 24 expects every scene to pass.
- **Section 12 + Section 13**: Sixel and kitty graphics share `ImageCache`, `ImageTextureCache`, and the GPU image pipeline. A regression in either silently breaks the other. Cross-stack regression sweep (section 23) catches this in CI, but the per-section test ladders must include placement-survives-other-protocol scenarios.
- **Section 02 sets up the absorption that 08 references**: Section 02 adds the supersede notice and creates the mapping-table file. Section 08 populates the mapping table with the actual catalog row → tack section IDs. Sections are co-implemented in the sense that 02's empty mapping table file is filled by 08, but they are SEPARATE sections (02 is plan-hygiene only, 08 is implementation work).

## Tack Absorption Strategy (delivered by Section 02)

`plans/tack-conformance/` is in flight: sections 01-05 complete, section 06 (TOOLS_MENU_INVENTORY) just landed, sections 06.0.b/c–06.N pending, sections 07-09 not yet started.

**Mechanical absorption (Phase 0b — no file moves, no implementation):**

Section 02 delivers all of this:

1. `plans/tack-conformance/index.md` and `plans/tack-conformance/00-overview.md` get a header note: "**Superseded by `plans/spec-conformance/`**. New work continues under that plan. Existing section files (01-09) remain in place for citation stability and grepability." Update `index.md` reroute frontmatter to `status: resolved`.
2. `plans/spec-conformance/00-overview.md` (this file) lists `plans/tack-conformance/` in `supersedes` frontmatter (already done).
3. `plans/spec-conformance/catalog/_legacy-tack-mapping.md` is created — an empty mapping table that section 08 (ECMA-48 baseline) populates with catalog row IDs as it converts tack scenarios into spec verification chains:
   ```
   | Catalog row    | Legacy tack section                              | Status     |
   |---             |---                                               |---         |
   | ECMA48-CUP     | tack-conformance/section-05 (modes-am)           | covered    |
   | OSC-52         | tack-conformance/section-06 (tools/u8/u9)        | in flight  |
   ```
4. Existing tack-conformance sections continue executing under their existing section numbers. No file renames. Citation stability preserved.
5. New work that would have been tack-conformance sections 07-09 (GPU goldens, keyboard tests, final verification) is created **directly under spec-conformance** as the appropriate per-stack sections (e.g., GPU goldens for terminfo coverage become part of section 08; keyboard encoding tests become section 17; final verification becomes section 23). No section renaming or dual-identity churn.

**Why early absorption (Phase 0b) and not buried in section 08**: Section 02 is mechanical plan hygiene — supersede notices and an empty mapping-table file. It is not implementation-dependent. Doing it early removes dual-track ambiguity while tack 06.x is still moving. Burying it inside section 08 (ECMA-48 baseline) would make 08 do two jobs: protocol implementation AND plan migration. Per Codex's Step 6B feedback, those should be split.

**Why no file moves**: Physical relocation creates rename churn during architecture work, breaks `git log` traceability without `--follow`, and forces dual-identity maintenance during the migration. Leaving the files in place is mechanically simpler and preserves history.

## Verification status taxonomy

Each catalog row has exactly one of these states. Only `verified` (and `verified-with-deviation`) count toward conformance percentages.

| Status                       | Meaning                                                                                                                          | Counts toward conformance? |
|---                           |---                                                                                                                               |---                         |
| `missing`                    | No implementation exists for this sequence                                                                                       | No                         |
| `stub`                       | Implementation exists but is a no-op or placeholder (e.g., parsed but flag has no effect)                                        | No                         |
| `implemented-unverified`     | Code path exists; **the E2E test ladder is incomplete or absent**. This is the DEFAULT state for everything currently "supported" per the audit memory. | No                         |
| `verified-partial`           | Test chain exists but doesn't reach the apex layer (e.g., grid state tested but no golden image for a visual sequence)           | No                         |
| `verified`                   | Full E2E test chain present, every applicable rung green, behavior matches the per-stack conformance reference                   | **Yes**                    |
| `verified-with-deviation`    | Verified to match a documented intentional deviation from spec (rare; requires reference + justification in the catalog notes)   | **Yes**                    |

**Crucial consequence**: ori_term currently has sixel, kitty graphics, iTerm2 inline images, half-blocks, quadrants, sextants, and braille all implemented per the audit memory. **Every one of those rows starts as `implemented-unverified` in the catalog.** The graphics audit becomes a list of *demotions from "support" to "implemented-unverified"*, not a list of completed work. The mission is grinding every row to `verified`.

## Authority ladder (per-stack conformance reference)

When specs disagree, the authority ladder is the tiebreaker. Per-stack default lives in this overview; per-row override (with explicit reason field) lives in the catalog.

| Stack                              | Authority ladder (highest first)                                                                |
|---                                 |---                                                                                              |
| ECMA-48 / xterm extensions         | xterm `ctlseqs.html` → ECMA-48 → DEC STD 070 → wezterm escape-sequences.md                      |
| DEC private modes                  | xterm `ctlseqs.html` → DEC technical manuals (VT220/VT320/VT420/VT520) → contour-terminal       |
| Sixel                              | DEC STD 070 → libsixel → xterm                                                                  |
| Kitty graphics protocol            | kitty source itself (`kitty/graphics/` in reference repo) — kitty IS the spec                   |
| Kitty keyboard protocol            | kitty source itself + `sw.kovidgoyal.net/kitty/keyboard-protocol/`                              |
| iTerm2 inline images / OSC 1337    | iTerm2 docs                                                                                     |
| OSC suite (general)                | xterm `ctlseqs.html` → ECMA-48 → individual application docs (iTerm2, ConEmu, mintty)           |
| OSC 8 hyperlinks                   | gist:egmontkob (canonical hyperlink spec) → wezterm                                             |
| OSC 52 clipboard                   | xterm `ctlseqs.html`                                                                            |
| Mode 2026 (synchronized output)    | contour-terminal spec (`docs/vt-extensions.md`)                                                 |
| Mode 2031 (color scheme update)    | contour-terminal spec                                                                           |
| OSC 133 (semantic prompt / FTCS)   | Final Term proposal → iTerm2 docs                                                               |
| OSC 633 (VS Code shell integration)| VS Code source                                                                                  |
| OSC 7 (CWD reporting)              | iTerm2 docs                                                                                     |
| OSC 9 / 99 / 777 (notifications)   | iTerm2 docs (9, 99) → kitty docs (777)                                                          |
| Mouse protocols                    | xterm `ctlseqs.html` (X10 + 1000 + 1002 + 1003 + 1004 + 1006 + 1015 + 1016) + URXVT docs        |
| Unicode subcell glyphs             | Unicode chart PDFs (definitive shape) + UAX                                                     |
| Unicode width / clustering / bidi  | UAX #11 (East Asian Width) + UAX #29 (Grapheme Clustering) + UAX #9 (Bidi)                      |
| ISO 2022 / NRCS                    | ISO 2022 standard → ECMA-35 → DEC technical manuals                                             |
| VT52 / VT100 / VT102               | DEC user manuals (VT52, VT100, VT102) → xterm legacy emulation                                  |
| VT220 / VT320 / VT420 / VT520      | DEC technical manuals                                                                           |
| ReGIS                              | DEC ReGIS technical manual                                                                      |
| Tektronix 4010 / 4014              | Tektronix 4014 manual                                                                           |
| Wyse 50 / 60                       | Wyse 50 user manual                                                                             |
| ADM-3A                             | ADM-3A docs                                                                                     |
| IBM PC ANSI.SYS                    | MS-DOS ANSI.SYS reference                                                                       |
| Microsoft Console VT               | Microsoft VT spec                                                                               |
| Audio (BEL, ANSI music, DECPS)     | DEC technical manual (DECPS) → ANSI.SYS docs (music) → ECMA-48 (BEL)                            |

## Implementation Sequence

```
Phase 0a — Catalog map of territory
  └─ 01 Catalog Bootstrap: Build catalog/ files. No tests written.
                          Phase 1 inventory: scrape wezterm
                          escape-sequences.md, ori_term VTE dispatch
                          tables, real-app captures, notcurses source.
                          Phase 2 walk: read primary specs to fill gaps.
                          Provisional schema in 00-overview.md is the
                          starting template; section 04 freezes the
                          schema after the pilots prove what's needed.

Phase 0b — Plan hygiene (mechanical, no implementation)
  └─ 02 Tack-conformance absorption: Add supersede notices to
       plans/tack-conformance/{index.md, 00-overview.md}. Create empty
       catalog/_legacy-tack-mapping.md mapping table file (filled by 08).
       No file moves. No code changes.

Phase 1 — Foundation (5 narrow, focused sections, parallel after 03+04)
  ├─ 03 Effect Boundary Migration:
  │     - oriterm_core::effect::{Effect, EffectSink} (production type)
  │     - Family enum: Pty / Host / HostRequest / Ui / Presentation
  │     - LegacyEventSink adapter for one-phase migration
  │     - Remove ClipboardLoad / ColorRequest closures (event/mod.rs:46,50)
  │     - Replace with HostRequest::ClipboardLoad / ColorQuery + ResponseToken
  │     - Absorb Term::pending_notifications via EffectSink::take_pending()
  │  Gate: closures removed; all current Event consumers route through
  │        Effect; pending_notifications no longer drained outside Effect
  │
  ├─ 04 Verification Chain Harness + Pilots + Coverage Report:
  │     - SpecHarness API (extends TeseqHarness with effect transcript
  │       capture, snapshot_seqno tracking, presentation gate observation,
  │       per-rung test runner)
  │     - First visual pilot: minimal sixel scenario through every
  │       applicable rung
  │     - First non-visual pilot: DA1 query through effect transcript apex
  │     - Schema freeze: catalog row template based on what pilots needed
  │     - Coverage report generator binary (oriterm_test_support)
  │     - BLOAT splits AS TOUCHED:
  │         oriterm/src/gpu/prepare/mod.rs (504) split when extending
  │         oriterm/src/gpu/prepare/dirty_skip/mod.rs (506) split when extending
  │  Gate: pilots green; harness API stable; catalog schema frozen;
  │        all subsequent stack sections can be written against a stable target
  │
  ├─ 05 Golden Lane Determinism:
  │     - headless_env_with_pinned_software_rasterizer() — llvmpipe forced
  │     - HintingMode::Full no longer hardcoded; goldens default to
  │       grayscale alpha
  │     - Pinned cell metrics for golden tests
  │     - Exact-or-tiny per-pixel tolerance as primary gate
  │     - SSIM / ΔE relegated to diagnostic-only
  │     - render_frame_cached() integration for the cached production path
  │  Gate: golden tests reproducible across runs and machines on the
  │        canonical lane
  │
  ├─ 06 Terminal Mode Plumbing (control plane):
  │     - Mode 2026 timeout-abort wiring (call sync_timeout/stop_sync
  │       in oriterm_mux/src/pane/io_thread/mod.rs)
  │     - Mode metadata registry: data-only consolidation of the 5-sync-
  │       point LEAK across NamedPrivateMode + PrivateMode::new +
  │       named_private_mode_number/_flag + apply_decset/_decrst
  │     - Behavior stays in match arms (per Codex Q4 pushback)
  │  Gate: timeout-abort tests pass; adding a new mode requires touching
  │        exactly one registry entry, not 5
  │
  └─ 07 Image Lifecycle Correctness (graphics state):
        - image_cache resize/reflow handler (currently MISSING)
        - Image placement survives: scrollback eviction, grid resize,
          reflow on column change, alt-screen toggle, ED/EL erase
        - Regression matrix for every image protocol × every grid
          transformation
     Gate: image+resize tests pass; placements deterministic across
           every grid mutation

Phase 2 — Baseline
  └─ 08 ECMA-48 Baseline:
       Drives the row subset of catalog/{ecma-48,xterm-ctlseqs,
       dec-private-modes,osc}.md that the existing tack-conformance work
       already covers. Subset ownership — NOT whole-file ownership;
       sections 09 (DEC modes) and 10 (OSC suite) own the rest.
       Adds new baseline rows for gaps tack didn't cover: DECLRMM grid
       enforcement, REP edge cases, 8-bit C1 controls.
       Populates catalog/_legacy-tack-mapping.md (created by section 02).
  Gate: every row in catalog/{ecma-48,xterm-ctlseqs}.md that tack covers
        is `verified`; new baseline rows (DECLRMM, 8-bit C1, REP edge
        cases) are `verified`.

Phase 3 — Per-stack expansion (parallelizable groups)

  Group A (parallel — pure data + handler stacks):
    ├─ 09 DEC Private Modes (the rest of catalog/dec-private-modes.md
    │     beyond the basic mode subset 08 covered, plus catalog/mode-2026.md)
    ├─ 10 OSC Suite (the rest of catalog/osc.md beyond the minimal
    │     subset 08 covered, plus catalog/shell-integration.md — palette
    │     hyperlinks, clipboard, title, OSC 7, OSC 9/99/777, OSC 133,
    │     OSC 633, OSC 1337 minimal)
    └─ 11 Unicode Subcell Glyphs + octants (catalog/unicode-subcell.md)

  Group B (sequential — image stack):
    ├─ 12 Sixel (catalog/sixel.md)
    ├─ 13 Kitty Graphics (catalog/kitty-graphics.md)
    ├─ 14 iTerm2 Inline Images (catalog/iterm2.md, image rows)
    └─ 15 Cell-Level Alpha + Transparency (catalog/iterm2.md ALPHA rows
          + catalog/de-facto-behaviors.md plane composition rows)

  Group C (sequential — input stacks):
    ├─ 16 Mouse Protocols (catalog/mouse.md, every numbered protocol)
    └─ 17 Kitty Keyboard Protocol (catalog/kitty-keyboard.md, all 5
          disambiguation modes + encoding to PTY)

  Group D (parallel — independent of stacks):
    ├─ 18 Charsets + UAX Policy (catalog/charsets.md — NRCS, ISO 2022
    │     multibyte sets, UAX #9/#11/#29 + emoji ZWJ + variation selectors)
    ├─ 19 Historical Stacks (catalog/historical.md — VT52, ReGIS,
    │     Tek 4010/4014, Wyse 50/60, ADM-3A, IBM PC ANSI.SYS,
    │     Microsoft Console VT)
    └─ 20 Audio + Print (catalog/audio-print.md — BEL, ANSI music CSI M,
          DECPS, visual bell, print screen, auto print, file transfer
          detection)

Phase 4 — Integration harnesses (sibling tracks; both depend on Phase 1)
  ├─ 21 notcurses-demo Harness + Scene Matrix + qrcode smoke:
  │     - PTY recording / replay infrastructure for notcurses-demo
  │     - Per-scene golden capture infrastructure
  │     - qrcode scene smoke test (simplest scene, ~40 LoC)
  │     - LANDS EARLY: as soon as Phase 1 + a few stacks are in flight
  │     - Per-scene gates added incrementally; section is "done" when
  │       qrcode passes (NOT all 28 scenes — that's section 24)
  │
  └─ 22 Real-App E2E Harness:
        - PTY recording / replay infrastructure for real applications
        - Snapshot capture pipeline (recorded PTY trace → ori_term replay
          → snapshot golden → diff)
        - First app smoke test (vim simple session)
        - LANDS EARLY: as soon as Phase 1 is solid
        - Section is "done" when one app smoke test passes (NOT all
          apps — that's section 25)

Phase 5 — Continuous verification (lands once Phase 3 has ~3 verified stacks)
  └─ 23 Cross-stack regression sweep + coverage report CI:
       - GitHub Actions workflow (.github/workflows/spec-conformance.yml)
       - Runs every stack's verification chain on every PR
       - Per-stack test binaries to stay under the 150s test cap
       - Coverage report fails CI on any regression (verified → lower)
       - Per-platform apex matrix where the apex is OS-dependent
         (clipboard, audio, focus, kitty file/shm transports, title,
          shell integration)
  Gate: CI green on every PR; coverage report only ever increases.

Phase 6 — Final integration milestones (depend on every prior section)
  ├─ 24 notcurses-demo FULL-PASS Milestone:
  │     - All 28 scenes pass against per-scene correctness criteria
  │     - Bisects every glitch into a catalog row addition or fix in
  │       the appropriate per-stack section (NOT in this section)
  │     - Section starts when section 21's harness is live and Phase 3's
  │       image+glyph stacks are verified
  │     - This section ONLY tracks scene-by-scene completion; bug fixes
  │       belong to the per-stack sections
  │
  └─ 25 Real-App FULL-PASS Milestone:
        - vim, neovim, helix, htop, btop, tmux, aerc, ncmpcpp, less
        - Each app has a recorded daily-driver scenario; the scenario's
          captured byte stream replays cleanly through ori_term and
          produces a snapshot identical to its golden
        - Section starts when section 22's harness is live and Phase 3
          stacks are verified for the apps' protocol surface
```

**Why this order:**
- **Phase 0a** (catalog) is the map; without it, no section has scope.
- **Phase 0b** (tack absorption) is mechanical plan hygiene that removes dual-track ambiguity early. Doing it inside section 08 would mix plan migration with protocol implementation.
- **Phase 1** (foundation, 5 narrow sections) is split per Codex's Step 6B feedback. Each section has a single concern (Effect type, harness, golden determinism, mode plumbing, image lifecycle). 03 and 04 are sequential prerequisites; 05/06/07 can be parallel after 04. The previous "section 02 with 11 subsections" was too wide for review.
- **Phase 2** (ECMA-48 baseline) is the gate for Phase 3. Every per-stack section depends on baseline correctness — sixel needs SGR, kitty graphics needs OSC parsing, mouse needs CSI parsing. Without baseline verified, every Phase 3 section would fight through baseline bugs.
- **Phase 3** groups are parallelizable. Image group is sequential because they share the image cache + GPU image pipeline. Input group is sequential because keyboard reuses mouse encoder scaffolding.
- **Phase 4** is integration **harnesses**, NOT integration milestones. Sections 21/22 are early scaffolding sections that land as soon as Phase 1 + a few stacks are ready. They are SIBLING tracks (notcurses-demo and real-app are not in a chain — they probe different things).
- **Phase 5** (cross-stack CI) lands once Phase 3 has produced ~3 verified stacks so the report has something meaningful.
- **Phase 6** (final full-pass milestones) is the canary. notcurses-demo full-pass and real-app full-pass are SEPARATE sections from their harness sections — each has its own clear "complete" gate, avoiding the forever-open-section anti-pattern.

**Known failing tests (expected until plan completion):**

- **Mode 2026 timeout-abort tests** — Will fail until section 06 wires `Processor::sync_timeout`/`stop_sync` in `oriterm_mux/src/pane/io_thread/mod.rs`. Root cause: `sync_timeout`/`stop_sync` exist in `crates/vte/src/ansi/processor.rs` but ori_term never calls them. The vte processor will never escape the sync buffer if the application crashes mid-sync.
- **Image+resize regression tests** — Will fail until section 07 adds the resize handler to `image_cache`. Root cause: `oriterm_core/src/image/cache/mod.rs` has scrollback prune (`prune_scrollback`) and erase region (`remove_placements_in_region`) but no resize handler.
- **DECLRMM enforcement tests** — Will fail until section 08 implements grid-side enforcement. Root cause: VTE recognizes the mode (`crates/vte/src/ansi/types.rs:226+`) but `oriterm_core/src/grid/mod.rs` has no left/right margin fields.
- **NRCS designation tests** — Will fail until section 18 adds NRCS variants. Root cause: only `StandardCharset::Ascii` and `StandardCharset::SpecialCharacterAndLineDrawing` exist in `crates/vte/src/ansi/attr.rs:204-208`. All NRCS variants (DE, FR, FI, etc.) are missing.
- **Octant glyph rendering tests** — Will fail until section 11 adds the octant Canvas implementation. Root cause: `oriterm/src/gpu/builtin_glyphs/legacy_computing/` has sextants but no octants (U+1CD00–U+1CDE5, Unicode 16).
- **Cell-level alpha tests** — Will fail until section 15 adds the alpha field/flag to `Cell`. Root cause: `oriterm_core/src/cell.rs` has no alpha field. Translucent overlays (notcurses `trans` scene) cannot be modeled.
- **8-bit C1 control tests** — Will fail until section 08 adds 8-bit C1 detection in the VTE handler. Root cause: VTE only handles 7-bit ESC-prefixed C1.
- **ANSI music tests** — Will fail until section 20. Root cause: CSI M (music) and DECPS (sound) have no handlers.
- **Kitty keyboard encoding tests** — Will fail until section 17. Root cause: parsing exists but no PTY encoding side.
- **`Term::pending_notifications` migration tests** — Will fail until section 03. Root cause: the bypass channel is not yet routed through `EffectSink::take_pending()`.

Do NOT attempt to fix these tests individually. They share infrastructure dependencies that must be built bottom-up through Phases 1-2.

## Metrics (Current State)

Baseline measurements before implementation begins. Establishes the starting point so progress and regressions can be measured.

| Crate                | Production LOC | Test LOC | Total |
|---                   |---:            |---:      |---:   |
| `crates/vte`         | ~2,500         | ~600     | ~3,100|
| `oriterm_core`       | ~12,000        | ~14,000  | ~26,000|
| `oriterm_ui`         | ~9,000         | ~5,000   | ~14,000|
| `oriterm_mux`        | ~3,500         | ~2,500   | ~6,000|
| `oriterm_ipc`        | ~600           | ~200     | ~800  |
| `oriterm`            | ~16,000        | ~9,000   | ~25,000|
| `oriterm_test_support`| ~3,000        | ~1,500   | ~4,500|
| **Total**            | **~46,600**    | **~32,800** | **~79,400** |

| Test infrastructure dimension          | Current count                |
|---                                     |---                           |
| teseq scenarios                        | 176 scenarios across 7 families |
| tack scenarios                         | sections 01-05 complete + 06.0 in flight |
| visual_regression test modules         | 17 (~tests per module varies)|
| alloc_regression invariant checks      | 3 (snapshot/render/render-input) |
| rss_regression invariant checks        | 1 (sustained output) |
| WidgetTestHarness widget tests         | per-widget sibling tests.rs files |

| Catalog inventory baseline (bottom-up scan only)                       | Count            |
|---                                                                     |---:              |
| C0 controls handled                                                    | 14 of 32 + DEL   |
| C1 controls handled                                                    | 7-bit ESC only; 8-bit MISSING |
| ESC sequences with handlers                                            | ~25              |
| CSI sequences with handlers (cursor/erase/insert/scroll/SGR/modes/etc.)| ~100             |
| OSC sequences with handlers                                            | 18               |
| DCS handlers                                                           | 2 (sixel + DECRQSS) |
| APC handlers                                                           | 1 (kitty graphics) |
| DEC private modes recognized                                           | ~30              |
| Mouse protocols implemented (encoders)                                 | 4 (X10/UTF-8/SGR/URXVT) |
| Charset designation handlers                                           | 2 (ASCII + Special Graphics) |
| **Estimated catalog rows after section 01 completes**                  | **~1,500-2,500** |

**Verification status before Phase 0:** **0 verified rows.** Per the new taxonomy, every existing implementation begins as `implemented-unverified`. The audit memory inventory documents what exists; this plan turns "exists" into "verified."

## Estimated Effort

Per-section line estimates are approximate. Sections will be refined as their dependencies solidify.

| Section                                       | Est. Lines (plan + tests + impl) | Complexity | Depends On  |
|---                                            |---:                              |---         |---          |
| 01 Catalog Bootstrap                          | ~3,500 (catalog files + plan)    | Medium     | —           |
| 02 Tack-Conformance Absorption                | ~300 (mechanical hygiene)        | Low        | 01          |
| 03 Effect Boundary Migration                  | ~2,500                           | High       | 02          |
| 04 Verification Chain Harness + Pilots        | ~3,500                           | High       | 03          |
| 05 Golden Lane Determinism                    | ~1,500                           | Medium     | 04          |
| 06 Terminal Mode Plumbing (timeout + registry)| ~1,500                           | Medium     | 04          |
| 07 Image Lifecycle Correctness                | ~2,000                           | Medium     | 04          |
| 08 ECMA-48 Baseline                           | ~4,000                           | Medium     | 05, 06      |
| 09 DEC Private Modes (full)                   | ~2,000                           | Medium     | 06, 08      |
| 10 OSC Suite (full)                           | ~3,500                           | Medium     | 08          |
| 11 Unicode Subcell Glyphs (incl. octants)     | ~2,500                           | Medium     | 05, 08      |
| 12 Sixel                                      | ~3,000                           | Medium     | 05, 07, 08  |
| 13 Kitty Graphics                             | ~4,500                           | High       | 12          |
| 14 iTerm2 Inline Images                       | ~2,000                           | Low-Med    | 13          |
| 15 Cell-Level Alpha + Transparency            | ~2,500                           | High       | 14          |
| 16 Mouse Protocols                            | ~2,000                           | Low-Med    | 08          |
| 17 Kitty Keyboard Protocol                    | ~2,500                           | Medium     | 16          |
| 18 Charsets + UAX Policy                      | ~3,500                           | High       | 08          |
| 19 Historical Stacks                          | ~3,500                           | Medium     | 08          |
| 20 Audio + Print                              | ~1,500                           | Low-Med    | 08          |
| 21 notcurses-demo Harness + Scene Matrix      | ~1,500                           | Medium     | 04, 07      |
| 22 Real-App E2E Harness                       | ~1,500                           | Medium     | 04          |
| 23 Cross-Stack Regression Sweep + Coverage CI | ~1,500                           | Low-Med    | 04, three stacks verified |
| 24 notcurses-demo FULL-PASS Milestone         | ~1,500                           | High       | 21, 11, 12, 13, 15 |
| 25 Real-App FULL-PASS Milestone               | ~1,500                           | Medium     | 22, all Phase 3 stacks |
| **Total new**                                 | **~58,800**                      |            |             |
| **Total deleted**                             | **~500** (closure code, deprecated tack-conformance pointers) |  |             |

This is a multi-month plan. Sections within a parallel group can be tackled concurrently across sessions. The plan is structured to accept indefinite continuation as new specs emerge.

## Research Findings

Bugs and architectural gaps discovered during the research phase (Pass 1-4 + Codex consensus loop) that affect multiple sections. Track root causes, fix locations, and status so they don't get lost. (Renamed from "Known Bugs (Pre-existing)" to "Research Findings" per Codex Step 8B feedback — many of these are not pre-existing per se, they are gaps the new plan exposes.)

| Finding                                                                                                   | Root Cause                                                                                                                                                            | Fix Location | Status      |
|---                                                                                                        |---                                                                                                                                                                    |---           |---          |
| Mode 2026 timeout-abort path completely unwired                                                           | `Processor::sync_timeout` and `stop_sync` exist in `crates/vte/src/ansi/processor.rs` but ori_term never calls them. App crashing mid-sync hangs the terminal forever. | Section 06   | Not Started |
| `Term::pending_notifications` bypasses Event channel                                                      | `oriterm_core/src/term/shell_state.rs:218` exposes `drain_notifications()` outside the Event/Effect channel; raw interceptor pushes via `push_notification()`         | Section 03   | Not Started |
| `Event::ClipboardLoad` and `Event::ColorRequest` carry closures                                           | `oriterm_core/src/event/mod.rs:46,50` — `Arc<dyn Fn(&str) -> String + Send + Sync>` and `Arc<dyn Fn(Rgb) -> String + Send + Sync>`                                    | Section 03   | Not Started |
| Image cache has NO resize/reflow handler                                                                  | `oriterm_core/src/image/cache/mod.rs` has `prune_scrollback` and `remove_placements_in_region` but nothing for grid resize. Placements with out-of-bounds columns may persist after resize. | Section 07   | Not Started |
| DEC mode handling LEAK across 5 sync points                                                               | `crates/vte/src/ansi/types.rs:226-295` (NamedPrivateMode), `types.rs:175` (PrivateMode::new), `oriterm_core/src/term/handler/helpers.rs:22,56` (named_private_mode_*), `modes.rs:17-102` (apply_decset/apply_decrst). Adding a new mode requires 5 edits. | Section 06   | Not Started |
| GPU adapter NOT pinned                                                                                    | `oriterm/src/gpu/state/mod.rs:150` calls `wgpu::PowerPreference::HighPerformance` and picks any available adapter. Different GPU drivers → antialiasing differences → false-positive golden diffs. | Section 05   | Not Started |
| `HintingMode::Full` hardcoded for golden tests                                                            | `oriterm/src/gpu/visual_regression/mod.rs:87` defaults to `HintingMode::Full`. Hinting interacts with subpixel rasterization and produces variation across runs.       | Section 05   | Not Started |
| BLOAT: `oriterm/src/gpu/prepare/mod.rs` (504 lines)                                                       | Exceeds 500-line limit per `code-hygiene.md`. Section 04 will touch this file when extending the prepare phase to capture render-input observation hooks.              | Section 04   | Not Started |
| BLOAT: `oriterm/src/gpu/prepare/dirty_skip/mod.rs` (506 lines)                                            | Exceeds 500-line limit. Section 04 will touch this file when extending dirty-skip logic for verification chain capture.                                                | Section 04   | Not Started |
| DECLRMM (left/right margins) recognized but not enforced                                                  | VTE parser recognizes the mode (`crates/vte/src/ansi/types.rs:226+`) but `oriterm_core/src/grid/mod.rs` has no left/right margin fields. Mode flag toggles a no-op flag. | Section 08   | Not Started |
| 8-bit C1 controls not handled                                                                             | VTE parser only handles 7-bit ESC-prefixed C1 forms. CSI/DCS/APC with 8-bit introducers (0x9B, 0x90, 0x9F) are not detected.                                           | Section 08   | Not Started |
| Octants (U+1CD00–U+1CDE5, Unicode 16) not implemented                                                     | `oriterm/src/gpu/builtin_glyphs/legacy_computing/` has sextants but no octants. Required by notcurses `keller`/`uniblock` blitter exhaustive tests.                    | Section 11   | Not Started |
| NRCS variants not implemented                                                                             | Only `StandardCharset::Ascii` and `StandardCharset::SpecialCharacterAndLineDrawing` exist. Every NRCS variant (DE, FI, FR, FR_CA, IT, NL, NO, PT, SE, SP, SU, CH, JIS Roman, JIS Kana, KOR, etc.) is missing. | Section 18   | Not Started |
| ANSI music (CSI M) not implemented                                                                        | No handler. Required for full PC ANSI.SYS conformance.                                                                                                                 | Section 20   | Not Started |
| DECPS (DEC play sound) not implemented                                                                    | No handler. Optional DEC feature.                                                                                                                                      | Section 20   | Not Started |
| Kitty keyboard ENCODING not implemented                                                                   | Modes parsed (CSI > u, push/pop stack, all 5 disambiguation modes) but no key encoding to PTY. Apps that enable kitty keyboard get no enhanced input.                  | Section 17   | Not Started |
| modifyOtherKeys ENCODING stub                                                                             | CSI > 4 m parsed and reports as disabled. No key encoding logic.                                                                                                       | Section 17   | Not Started |
| Win32 Input mode 9001 stub                                                                                | Mode flag set but no encoding. Required for Windows ConPTY conformance.                                                                                                | Section 17   | Not Started |
| Cell-level alpha not modeled                                                                              | `oriterm_core/src/cell.rs` has no alpha field/flag. Translucent overlays in notcurses `trans` scene cannot render correctly.                                           | Section 15   | Not Started |
| HSL hue rotation in sixel decoder is CORRECT (audit was wrong)                                            | `oriterm_core/src/image/sixel/color.rs:41` does `hue - 120.0` correctly. Audit memory wrongly suspected this. **No fix needed; correcting the audit memory record.**   | Section 01 (memory update) | Verification only |
| Kitty `q=1` query response IS implemented (audit was stale)                                               | `oriterm_core/src/image/kitty/parse.rs:197` + `oriterm_core/src/term/handler/image/kitty.rs:320`. Audit memory was stale. **No fix needed.**                          | Section 01 (memory update) | Verification only |
| Image cache `default_memory_limit` is 320 MiB, not 512 MiB as audit memory claims                         | `oriterm_core/src/image/cache/mod.rs:15` (Ghostty parity). Audit memory at `architecture_graphics_audit.md` is stale on this number; not a bug.                       | Section 01 (memory update) | Documentation |

## Quick Reference

| ID | Title                                                  | File                                                 | Status      |
|----|---                                                     |---                                                   |---          |
| 01 | Catalog Bootstrap                                      | `section-01-catalog-bootstrap.md`                    | Not Started |
| 02 | Tack-Conformance Absorption (Phase 0b — plan hygiene)  | `section-02-tack-absorption.md`                      | Not Started |
| 03 | Effect Boundary Migration                              | `section-03-effect-boundary-migration.md`            | Not Started |
| 04 | Verification Chain Harness + Pilots + Coverage Report  | `section-04-verification-chain-harness.md`           | Not Started |
| 05 | Golden Lane Determinism                                | `section-05-golden-lane-determinism.md`              | Not Started |
| 06 | Terminal Mode Plumbing (Mode 2026 + metadata registry) | `section-06-terminal-mode-plumbing.md`               | Not Started |
| 07 | Image Lifecycle Correctness                            | `section-07-image-lifecycle-correctness.md`          | Not Started |
| 08 | ECMA-48 Baseline                                       | `section-08-ecma-48-baseline.md`                     | Not Started |
| 09 | DEC Private Modes (full)                               | `section-09-dec-private-modes.md`                    | Not Started |
| 10 | OSC Suite (full)                                       | `section-10-osc-suite.md`                            | Not Started |
| 11 | Unicode Subcell Glyphs (incl. octants)                 | `section-11-unicode-subcell-glyphs.md`               | Not Started |
| 12 | Sixel                                                  | `section-12-sixel.md`                                | Not Started |
| 13 | Kitty Graphics Protocol                                | `section-13-kitty-graphics.md`                       | Not Started |
| 14 | iTerm2 Inline Images                                   | `section-14-iterm2-images.md`                        | Not Started |
| 15 | Cell-Level Alpha + Transparency                        | `section-15-cell-level-alpha.md`                     | Not Started |
| 16 | Mouse Protocols                                        | `section-16-mouse-protocols.md`                      | Not Started |
| 17 | Kitty Keyboard Protocol                                | `section-17-kitty-keyboard.md`                       | Not Started |
| 18 | Charsets + UAX Policy                                  | `section-18-charsets-and-uax-policy.md`              | Not Started |
| 19 | Historical Stacks (VT52/ReGIS/Tek/Wyse/...)            | `section-19-historical-stacks.md`                    | Not Started |
| 20 | Audio + Print                                          | `section-20-audio-and-print.md`                      | Not Started |
| 21 | notcurses-demo Harness + Scene Matrix + qrcode smoke   | `section-21-notcurses-demo-harness.md`               | Not Started |
| 22 | Real-App E2E Harness                                   | `section-22-real-app-harness.md`                     | Not Started |
| 23 | Cross-Stack Regression Sweep + Coverage CI             | `section-23-cross-stack-regression-sweep.md`         | Not Started |
| 24 | notcurses-demo FULL-PASS Milestone                     | `section-24-notcurses-demo-full-pass.md`             | Not Started |
| 25 | Real-App FULL-PASS Milestone                           | `section-25-real-app-full-pass.md`                   | Not Started |

## Catalog Row Schema (provisional — frozen by Section 04 pilots)

This is a **strawman** catalog row template. Section 04's sixel + DA1 pilots may revise it as the harness uncovers what the apex layers actually need. **Do not lock this schema until Section 04 is complete.**

```markdown
## ECMA48-CUP

| Field                | Value                                                          |
|---                   |---                                                             |
| **ID**               | `ECMA48-CUP`                                                   |
| **Spec source**      | ECMA-48 §8.3.21                                                |
| **Sequence**         | `CSI Ps;Ps H` — Cursor Position                                |
| **Description**      | Move cursor to (row, col), 1-based                             |
| **Implementation**   | `crates/vte/src/ansi/dispatch/csi.rs:91` → `TermHandler::goto` |
| **Apex layer**       | state-snapshot                                                 |
| **Test chain**       | parser:pass dispatch:pass state:pass snapshot:pass             |
| **Verification**     | verified                                                       |
| **De-facto ref**     | xterm `charproc.c::CursorSet` (DECOM interaction tiebreaker)   |
| **Notes**            | Origin mode interaction tested in `teseq/csi_cursor.rs`        |
```

For non-visual sequences, the test chain ends at the natural apex (e.g., `effect-pty-write` for DA1 reply, `effect-clipboard` for OSC 52, `effect-host-title` for OSC 0/2). For visual sequences, the chain extends through `renderable-snapshot`, `frame-input`, `gpu-instance`, `texture-render`, `golden-image`. The pilots in section 04 confirm what fields are actually load-bearing.

## Catalog Files (created by Section 01)

```
plans/spec-conformance/catalog/
├── README.md                       — schema reference + maintenance rules
├── _legacy-tack-mapping.md         — created by section 02 (empty);
│                                     populated by section 08 with
│                                     catalog row → tack section ID mapping
├── ecma-48.md                      — every CSI/SGR/mode in the ECMA-48 surface
├── xterm-ctlseqs.md                — xterm extensions: window manipulation,
│                                     focus events, bracketed paste, DECRQM/DECRPM
├── dec-private-modes.md            — every numbered DECSET/DECRST private mode
├── osc.md                          — OSC registry: 0, 1, 2, 4, 7, 8, 9, 10, 11,
│                                     12, 22, 50, 52, 99, 104, 110, 111, 112, 133,
│                                     633, 777, 1337, plus xterm OSC range
├── sixel.md                        — DCS q + raster attrs + transparency + DECSDM
├── kitty-graphics.md               — APC _G + every key + chunked + animation +
│                                     virtual placements + unicode placeholders
├── kitty-keyboard.md               — CSI > u + 5 disambiguation modes + key reporting
├── iterm2.md                       — OSC 1337 + iTerm2 OSC suite extensions
├── mode-2026.md                    — sync output + presentation gates +
│                                     timeout-abort behavior
├── unicode-subcell.md              — half-blocks, quadrants, sextants, octants,
│                                     braille, Symbols for Legacy Computing
├── mouse.md                        — X10, 1000, 1001, 1002, 1003, 1004, 1005,
│                                     1006, 1015, 1016 (SGR pixels), locator
├── charsets.md                     — DEC special graphics, line drawing, technical,
│                                     supplemental, dingbats, all NRCS variants,
│                                     ISO 2022 multibyte, ISO 8859 family
├── audio-print.md                  — BEL, ANSI music CSI M, DECPS, visual bell DECVB,
│                                     print screen, auto print, file transfer detect
├── shell-integration.md            — OSC 7 (CWD), OSC 9/99/777 (notifications),
│                                     OSC 133 (semantic prompt / FTCS), OSC 633
│                                     (VS Code), command timing, CommandComplete
├── historical.md                   — VT52, VT100/102, VT220/320/420/520, ReGIS,
│                                     Tek 4010/4014, Wyse 50/60, ADM-3A,
│                                     IBM PC ANSI.SYS, Microsoft Console VT
└── de-facto-behaviors.md           — sequences with no published spec, where the
                                      authoritative oracle is a reference impl
                                      (cited per row in the table)
```

## Spec Corpus (created by Section 01)

```
plans/spec-conformance/specs/
├── manifest.toml                   — index of every cited spec, including
│                                     redistribution rights notes and sha256
│                                     hashes for restricted documents
├── ecma-48-snapshot.html           — IF redistributable; otherwise manifest entry
│                                     with fetch script
├── xterm-ctlseqs.html              — invisible-island.net snapshot (verify license)
├── kitty-graphics-protocol.md      — sw.kovidgoyal.net snapshot
├── kitty-keyboard-protocol.md      — sw.kovidgoyal.net snapshot
├── mode-2026-spec.md               — contour-terminal docs/vt-extensions.md snapshot
├── ftcs-osc-133.md                 — Final Term semantic prompt proposal
├── osc-8-hyperlinks.md             — gist:egmontkob spec snapshot
├── unicode-uax-9.txt               — UAX #9 (Bidi) snapshot
├── unicode-uax-11.txt              — UAX #11 (East Asian Width) snapshot
├── unicode-uax-29.txt              — UAX #29 (Grapheme Clustering) snapshot
├── unicode-symbols-legacy.pdf      — U+1FB00 + U+1CD00 chart PDFs
├── dec-std-070.pdf                 — IF redistributable; manifest entry otherwise
├── vt-series-manuals/              — DEC user manuals (verify license per file)
└── manifest-fetch.sh               — script that downloads restricted-license
                                      documents to local cache (dev workflow)
```

## How sections cite catalog rows

Every catalog row has a stable ID like `ECMA48-CUP`, `KG-T-Q-DIRECT`, `SIXEL-DCS-Q-P1`, `OSC-52`, `MOUSE-1006`. Tests cite the row ID:

```rust
// oriterm_core/tests/spec_conformance/csi/cursor.rs
#[test]
fn ecma48_cup_basic() {
    // Catalog row: ECMA48-CUP (ECMA-48 §8.3.21)
    // Apex: state snapshot
    let mut h = SpecHarness::new();
    h.feed(b"\x1b[5;10H");
    assert_eq!(h.cursor_pos(), (5, 10));
}
```

The coverage report walks the catalog files, scans the test directories for citations, and produces:

```
$ cargo run -p oriterm_test_support --bin spec-coverage-report

Stack                  Total   verified   verified-partial   implemented-unverified   stub   missing
ECMA-48                  185        185                  0                        0      0         0  ← 100%
DEC Private Modes         62         48                  4                       10      0         0  ← 77%
OSC Suite                 47         12                  6                       29      0         0  ← 25%
Sixel                     38         32                  3                        3      0         0  ← 84%
Kitty Graphics            72         18                  4                       50      0         0  ← 25%
...
```

This is the canonical "are we conformant" answer. CI fails if any percentage decreases.
