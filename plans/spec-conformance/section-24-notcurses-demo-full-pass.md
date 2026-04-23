---
section: "24"
title: "notcurses-demo FULL-PASS Milestone"
status: not-started
reviewed: false
goal: "Drive every one of the 28 notcurses-demo scenes from `not-attempted` (or `fail`) to `pass` against per-scene correctness criteria. The harness from section 21 is the test infrastructure; this section USES the harness and bisects every glitch into a catalog row addition or fix in the appropriate per-stack section."
success_criteria:
  - "Audit input committed at `plans/spec-conformance/audits/section-24-top-down-inventory.md`. The audit input is a CORPUS MANIFEST (not an external control-sequence spec — this is an integration section). Every entry in the corpus has a corresponding harness wiring + per-entry pass criterion. `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes for this audit file (integration-section variant: validates corpus completeness against harness wiring). Section 09A introduced the `audits/` SSOT; this section adapts it for integration scope per `plans/spec-conformance/audits/README.md` integration-section guidance."
  - "All 28 notcurses-demo scenes are `pass` in `plans/spec-conformance/notcurses-scene-status.md`"
  - "Per-scene correctness criteria documented for each scene — what counts as 'no visual glitch' for that specific scene's protocol mix"
  - "Glitch bisection workflow followed: when a scene fails, the bisection identifies which catalog row's behavior is incorrect → file the bug under the appropriate per-stack section as a fix → re-run the scene"
  - "**This section does NOT contain implementation work** — implementations live in the per-stack sections (sections 03-20). This section only tracks scene completion and bisects failures."
  - "Section 24 is the canary for cross-stack interactions: a scene that fails when the per-stack tests pass means there is a cross-stack interaction bug that the per-stack tests structurally cannot find. The fix lands in the appropriate stack section, and section 24 re-tests."
  - "All existing tests pass without modification"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **`notcurses-demo` runs cleanly** — this section is the gate"
inspired_by:
  - "notcurses scene matrix in `reference_notcurses_demo.md` memory"
  - "section 21 harness — replay infrastructure used by every test in this section"
depends_on: ["07", "11", "12", "13", "15", "21"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "24.0"
    title: "Audit input verification (BLOCKING) — commit audits/section-24-top-down-inventory.md (Section 21 harness corpus + per-scene goldens)"
    status: not-started
  - id: "24.1"
    title: "Document per-scene correctness criteria for all 28 scenes"
    status: not-started
  - id: "24.2"
    title: "Drive simple scenes (qrcode already pass; highcon, grid, animate, box) to pass"
    status: not-started
  - id: "24.3"
    title: "Drive medium scenes (trans, uniblock, mojibake, sliders, reel) to pass"
    status: not-started
  - id: "24.4"
    title: "Drive hard scenes (keller all-7-blitters, dragon, fission, whiteout, normal) to pass"
    status: not-started
  - id: "24.5"
    title: "Drive media-dependent scenes (chunli, eagle, jungle, luigi, view, yield, zoo) to pass"
    status: not-started
  - id: "24.6"
    title: "Drive video scenes (xray, outro) to pass"
    status: not-started
  - id: "24.7"
    title: "Drive intro scene to pass"
    status: not-started
  - id: "24.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "24.N"
    title: "Completion Checklist"
    status: not-started
# TPR Checkpoint Placement: 24.3 (after simple+medium scenes — covers .1-.3),
# 24.5 (after hard+media scenes — covers .4-.5), final in 24.N
---

# Section 24: notcurses-demo FULL-PASS Milestone

**Status:** Not Started
**Goal:** Drive every notcurses-demo scene to pass. This section is the cross-stack canary — it catches bugs that per-stack tests structurally cannot find. **No implementation work happens in this section** — all fixes go to the appropriate per-stack section.

**Success Criteria:** see frontmatter.

**Context:** Section 21 built the harness + qrcode smoke test. This section drives the remaining 27 scenes. Per the scene-complexity ranking from `reference_notcurses_demo.md` memory:
- **Simple**: qrcode (q) — DONE in section 21, highcon (h), grid (g)
- **Medium**: animate (a), box (b), trans (t), uniblock (u), mojibake (m), sliders (s), reel (r)
- **Hard**: dragon (d), fission (f), keller (k), whiteout (w), normal (n)
- **Media-dependent (DFSG-disabled)**: chunli (c), eagle (e), jungle (j), luigi (l), view (v), yield (y), zoo (z)
- **Video**: xray (x), outro (o)
- **Intro**: intro (i)

The order in this section's subsections walks complexity ascending. Scenes that fail trigger bug fixes in the appropriate per-stack section (e.g., `keller` fails → bug in section 11 unicode glyphs OR section 13 kitty graphics OR section 12 sixel — bisect → file the fix → re-run keller).

**Reference implementations:** see frontmatter.

**Depends on:** Section 07 (image lifecycle), Section 11 (unicode glyphs incl. octants — required by keller), Section 12 (sixel), Section 13 (kitty graphics), Section 15 (cell-level alpha — required by trans), Section 21 (harness scaffolding).

---

## 24.0 Audit input verification (BLOCKING — precedes all other subsections)

**Goal:** Verify the audit-input corpus manifest at `plans/spec-conformance/audits/section-24-top-down-inventory.md` is populated and that every entry has corresponding harness wiring + per-entry pass criterion.

**Integration-section scope:** This section is NOT a protocol-stack section — it does not walk a control-sequence spec source. Its "audit input" is a CORPUS MANIFEST: Section 21's harness corpus (all 28 notcurses-demo scenes) + per-scene golden images. The `audits/` SSOT introduced by Section 09A (per `plans/spec-conformance/audits/README.md`) adapts to integration sections by treating the corpus manifest as the top-down enumerator. The completeness check is: every corpus entry has harness wiring + a per-entry pass criterion.

**Why this exists:** Section 09A closed the bottom-up catalog gap that hid DECRQCRA via the per-section audit file pattern. Integration sections inherit the same enforcement shape — the audit file IS the corpus manifest, and `spec-coverage-report --check audit-files` validates that every entry has the required wiring (not catalog-row mapping, since integration sections don't add catalog rows).

**Files touched:**
- `plans/spec-conformance/audits/section-24-top-down-inventory.md` (NEW — stub created by Section 09A's §09A.10; populated by this subsection)

**Completion criteria:**

- [ ] Audit file is populated with every entry in the corpus manifest (all 28 scenes + their committed goldens).
- [ ] Every entry has a `harness_wiring` reference (file path + test name) + a `pass_criterion` description.
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes (integration-section variant — validates corpus completeness, not catalog-row mapping).
- [ ] Audit file `last_walked` and `walked_by` set.

**No other subsection in this section can begin work until §24.0 is complete.**

---

## 24.1 Document per-scene correctness criteria for all 28 scenes

**File(s):** `plans/spec-conformance/notcurses-scene-criteria.md` (new)

For each scene, document what counts as "no visual glitch":
- For static scenes (qrcode, grid, highcon): exact pixel match against committed golden
- For animated scenes (intro, animate, box bouncing, sliders): pixel match at the FINAL frame after animation completes
- For interactive scenes (none in notcurses-demo, but documented for completeness)
- For video scenes (xray, outro): the final frame matches the committed golden

- [ ] Walk every scene file in `~/projects/reference_repos/console_repos/notcurses/src/demo/<scene>.c`
- [ ] For each scene, identify what subsystems are exercised and what "correct" looks like
- [ ] Write the criteria into `notcurses-scene-criteria.md`
- [ ] **Validation**: criteria document covers all 28 scenes.

---

## 24.2 Drive simple scenes to pass

For each simple scene (highcon, grid, animate, box), run the harness, observe failures, bisect, fix in the appropriate per-stack section, re-run.

- [ ] highcon (h): high contrast text — exercises rgb + text-attributes. Likely passes after section 08 baseline lands. If not, bisect.
- [ ] grid (g): static color gradients + box drawing — exercises rgb, unicode-boxes, text-attributes. Likely passes after section 08 + section 11. If not, bisect.
- [ ] animate (a): spiral of U+2596..259F glyphs + progress bars — exercises quadrants + plane stacking. Requires section 11 octants? Verify.
- [ ] box (b): concentric double-line boxes — exercises unicode-boxes, transparency, multi-plane, media (optional), pixel-blit. Requires section 15 cell-level alpha for transparency.
- [ ] Mark each scene `pass` in the tracker as it passes
- [ ] **Validation**: 4 simple scenes pass.

---

## 24.3 Drive medium scenes to pass

- [ ] trans (t): 6 planes with different alpha-blend modes — **best transparency stress test**. Requires section 15 (cell-level alpha) to be solid.
- [ ] uniblock (u): exhaustive 7-blitter rendering — **best blitter A/B test**. Requires section 11 octants.
- [ ] mojibake (m): exhaustive emoji/unicode catalog — exercises wide-chars, unicode-boxes, rgb, scrolling. Requires section 18 (UAX policy + variation selectors + ZWJ).
- [ ] sliders (s): 12×6 sliding puzzle with smooth movement — exercises multi-plane, rgb, text-attributes, wide-chars, unicode-boxes.
- [ ] reel (r): ncReel widget with colored tablet threads — exercises multi-plane, scrolling, text-attributes.
- [ ] **TPR checkpoint** — `/tpr-review` covering 24.1-24.3 (criteria + simple + medium scenes). Catches systemic bisection issues.

---

## 24.4 Drive hard scenes to pass

- [ ] keller (k): same image rendered through every blitter — **gold for blitter correctness**. Requires section 11 octants + section 12 sixel + section 13 kitty + section 15 cell alpha all verified. The MOST adversarial scene.
- [ ] dragon (d): L-system dragon curve fractal — exercises pixel-blit (kitty/sixel) + rgb. Requires section 12/13.
- [ ] fission (f): screen partitioned into bricks that fall — exercises multi-plane, scrolling, media, greyscale. Requires section 15.
- [ ] whiteout (w): worms moving + lighting cells — exercises unicode-boxes, multi-plane, rgb, text-attributes, cell re-read + lighting.
- [ ] normal (n): mandelbrot rendered outward + plane rotations — exercises pixel-blit, rgb, multi-plane, text-attributes, media.

---

## 24.5 Drive media-dependent scenes to pass

**No deferral for missing media.** The mission success criterion says "All 28 scenes pass" — NOT "all 28 scenes pass except the ones where media is unavailable". If media files are DFSG-excluded from the packaged `/usr/share/notcurses`, the section 21 harness pins a KNOWN-GOOD media set that ori_term ships as committed test fixtures under `crates/oriterm_test_support/tests/data/notcurses_media/` (freely-licensed public-domain substitutes per each scene's needs: public-domain chunli sprite for `chunli`, public-domain eagle photo for `eagle`, etc.). The `CaptureEnvPin::media_dir_sha256` pin (from section 21) ensures the captured byte stream and the shipped media set match. Scene captures are re-made against the shipped fixtures so every scene is deterministic and NO scene is deferred.

- [ ] Acquire or create freely-licensed substitute media files for each media-dependent scene (chunli, eagle, jungle, luigi, view, yield, zoo). Commit under `crates/oriterm_test_support/tests/data/notcurses_media/<scene>/` with licensing notes.
- [ ] Re-capture each scene's PTY output against the shipped fixture media set; update the sidecar `.env.toml` with the new `media_dir_sha256`.
- [ ] For each scene, drive the replay through the harness; bisect any failure to the appropriate per-stack section.
- [ ] chunli (c), eagle (e), jungle (j), luigi (l), view (v), yield (y), zoo (z) — each drives via section 12/13 image stacks
  - **yield (y) — known symptom (filed as BUG-06-017, escalated 2026-04-22):** ori_term on Windows blips-and-vanishes (demo terminates early before rendering the world-map polyfill). WezTerm on Windows runs longer but also doesn't render correctly; WezTerm on macOS runs correctly — confirms an ori_term-specific regression stacked on a Windows-notcurses issue. Phase 1 investigation (pre-escalation) falsified the startup-reply-corruption hypothesis via `oriterm_core/tests/spec_chain/pilots/notcurses_startup.rs` (no out-of-frame bytes, no stray `q` bytes in the 12 startup PTY replies to `captures/notcurses-demo-intro.cap`). Remaining hypotheses for §24.5 bisection: kitty `a=T` transmit+place reply framing during the render loop; `kitty_create_placement` cursor-move semantics (currently `linefeed` × `rows-1` only); GPU blit feedback path; ConPTY translation of mid-demo replies. Reference exit paths in `yield.c:41` (`demo_render` fail → `display` returns −1 → `done=1`) and `hud.c:288` (`interrupt_demo` on `q`).
- [ ] Update gate tracker — every scene marked `pass` (NOT `verified-with-deviation`)
- [ ] **TPR checkpoint** — `/tpr-review` covering 24.4-24.5

---

## 24.6 Drive video scenes to pass

- [ ] xray (x): logo scrolls + video bg — exercises media, video, scrolling, multi-plane, text-attributes. Frame drops capped at 15s.
- [ ] outro (o): closing message + video stream + dual fades — exercises media, video, fades, multi-plane.
- [ ] Note: "video" frames are produced by ffmpeg internally and delivered as a sequence of pixel frames blitted via the chosen blitter. Section 12/13 already verifies the blitter; this scene is testing the per-frame replay timing.

---

## 24.7 Drive intro scene to pass

- [ ] intro (i): Wittgenstein quotes, gradient boxes, sextant fallback display, fade-out — exercises pixel-blit, rgb, text-attributes, media, fades, sextants, wide-chars, unicode-boxes. Most-comprehensive scene.

---

## 24.R Third Party Review Findings

- None.

---

## 24.N Completion Checklist

- [ ] **Criteria written FIRST (TDD equivalent for a milestone section)**: per-scene correctness criteria in `notcurses-scene-criteria.md` are authored BEFORE any scene is driven — a scene cannot be declared `pass` against an undefined criterion
- [ ] **Matrix dimensions**: scene (28 letters) × subsystem exercised (blitter / rgb / transparency / media / video / scrolling / multi-plane) × pass/fail/not-attempted gate state — the matrix is the full scene-status table in `notcurses-scene-status.md`
- [ ] **Semantic pin**: the per-scene goldens captured via Section 21's harness are the permanent regression guards; a scene that regresses from `pass` back to `fail` blocks CI via Section 23
- [ ] Per-scene correctness criteria documented for all 28 scenes
- [ ] All 28 scenes marked `pass` in `notcurses-scene-status.md`
- [ ] Every glitch bisected to a per-stack section and fixed there (NOT in this section)
- [ ] All existing tests pass without modification
- [ ] Alloc regression unchanged
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` mission success criteria checked off (`notcurses-demo runs cleanly`)
- [ ] `index.md` section 24 status updated
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** All 28 notcurses-demo scenes pass. The first major integration milestone is complete. ori_term passes notcurses-demo cleanly — no other terminal emulator on Earth can claim this.
