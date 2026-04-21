---
section: "13"
title: "Kitty Graphics Protocol"
status: in-progress

reviewed: true
goal: "Drive every catalog row in `catalog/kitty-graphics.md` from `implemented-unverified`/`missing`/`stub` to `verified` — full APC `_G` protocol including chunked transmission, animation, virtual placements, and unicode placeholders. This section is the canonical home for the kitty graphics verification chain AND for the unicode-placeholder rendering implementation (currently `missing` in the catalog)."
success_criteria:
  - "Top-down spec audit committed at `plans/spec-conformance/audits/section-13-top-down-inventory.md`. Every sequence in the canonical spec source(s) for this stack (sw.kovidgoyal.net/kitty/graphics-protocol/ docs (primary, kitty source is the de facto SPEC for this protocol) + kitty source `kittens/icat/icat.py` cross-reference) maps to a catalog row ID OR carries an explicit `not-targeted` decision with rationale. `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes for this audit file. This is enforced PER `plans/spec-conformance/audits/README.md` lint contract — added by Section 09A as the SSOT for top-down catalog coverage to prevent the bottom-up gap that hid DECRQCRA from the catalog."
  - "`catalog/kitty-graphics.md` is expanded in §13.0 from 9 coarse rows into per-action / per-format / per-transmission / per-specifier / per-error-code rows (KG-TRANSMIT-{DIRECT,FILE,TEMPFILE,SHARED-MEM-REJECTED}, KG-TRANSMIT-FORMAT-{24,32,100}, KG-COMPRESSION-{OZ-IGNORED,OZ-REJECTED}, KG-DELETE-{a,A,i,I,p,P,c,C,x,X,y,Y,z,Z,r,R,n,N}, KG-FRAME-{TRANSMIT,COMPOSITE-OVERWRITE,COMPOSITE-ALPHABLEND}, KG-ANIMATE-{STOP,RUN-WAIT,RUN}, KG-RESPONSE-{OK,EBADF,EBIG,EINVAL,ENOENT,ENOMEM,EIO}, KG-ACTION-COMPOSE, KG-ACTION-FALLBACK-TRANSMITANDPLACE)."
  - "Every row in `catalog/kitty-graphics.md` (after the §13.0 expansion) is `verified`, `verified-with-deviation` (for intentional spec divergences like shared-memory rejection or `o=z` unconsumed), or `not-targeted` with rationale. No `missing`, `stub`, or `implemented-unverified` rows remain."
  - "Every kitty action (a=t Transmit, a=T TransmitAndPlace, a=p Place, a=d Delete, a=f Frame, a=a Animate, a=q Query) verified via spec_chain. `KittyAction` at `oriterm_core/src/image/kitty/parse.rs:59-74` has NO `Compose` variant; `apply_key_value` at `parse.rs:189-200` silently falls back to `TransmitAndPlace` on any unrecognized `a=` value including `a=c`. §13.0 opens TWO distinct catalog rows: `KG-ACTION-COMPOSE` (status `missing` — kitty's separate compose-frame action with its own key set `r,c,w,h,x,y,X,Y,C` is not implemented) and `KG-ACTION-FALLBACK-TRANSMITANDPLACE` (status `verified-with-deviation` — the current silent fallback pins a load-bearing behavior for any unknown `a=` value). The `cell_x_offset == 1` → `CompositionMode::Overwrite` path at `kitty_animation.rs:58-62` governs in-frame blend mode on a newly-transmitted `a=f` frame; it is NOT a substitute for `a=c`."
  - "Every transmission mode (t=d Direct, t=f File, t=t TempFile, t=s SharedMemory) verified. SharedMemory is parsed at `parse.rs:207-213` but rejected with `EINVAL` at `kitty.rs:289-291`. The catalog row KG-TRANSMIT-SHARED-MEM-REJECTED captures this as `verified-with-deviation` — the rejection IS the verified behavior."
  - "Every transmission format (f=24 RGB, f=32 RGBA, f=100 PNG) verified + the format-dispatch matrix at `kitty.rs:314-345` pinned by per-format rows."
  - "Compression (o=z) is parsed at `parse.rs:215` into `cmd.compression` but NEVER consumed downstream (`kitty.rs:255-345` does not decompress). §13.0 MUST open a catalog row that either codifies the no-op as `not-targeted` (with rationale) OR this section MUST implement zlib decompression. Survivor-mode `verified` on an aggregated KG-TRANSMIT row that ignores `o=z` would hide this gap."
  - "Chunked transmission (m=1 more chunks, m=0 final) verified — feed split chunks, assert coalesced + decoded correctly. Malformed-base64-mid-chunk path verified: prior plan asserted an error reply; `parse_kitty_command` currently returns `Err(KittyError::InvalidBase64)` which `handle_kitty_graphics` drops via `warn!` at `kitty.rs:39-42` with no reply emitted. §13.2 MUST EITHER implement reply-on-parse-error at that call site OR document the silent-drop as `verified-with-deviation` against the spec."
  - "Animation (a=f TransmitFrame, composition via `cell_x_offset == 1` → `CompositionMode::Overwrite`, default → `CompositionMode::AlphaBlend`) verified — Pass 1 confirmed both modes are implemented at `kitty_animation.rs:58-62`. §13.3 MUST include timer-driven redraw wiring: `Term::advance_animations` at `oriterm_core/src/term/image_config.rs:65` has NO caller in `oriterm/` or `oriterm_mux/` production code (verified by grep). Animation verification is only meaningful once the IO thread or render loop drives `advance_animations` on each frame deadline returned by the cache."
  - "Virtual placements (U=1 unicode placeholders) verified — §13.4 is implementation + verification because the cell-level placeholder rendering is currently absent. `catalog/kitty-graphics.md` lists KG-UNICODE-PLACEHOLDER as `missing` (not `implemented-unverified`): the handler suppresses placement on `U=1` at `kitty.rs:106-108` but `U+10EEEE` placeholder cells do NOT resolve back to the stored image at render time. §13.4 owns adding the cell-level placeholder rendering path AND its reflow/ED/EL behavior per §07's carve-out that explicitly routed placeholder lifecycle to §13.4."
  - "Image protocol replies (kitty ACK/error) verified via Effect transcript apex. Emission point: `Term::kitty_respond` at `oriterm_core/src/term/handler/image/kitty.rs:466-479` pushes `Effect::Pty(PtyEffect::Write { kind: PtyWriteKind::ImageProtocolReply, bytes })` onto the `EffectSink`. Routing point: the generic `Effect::Pty(PtyEffect::Write { bytes, .. })` arm at `oriterm_mux/src/pane/io_thread/effect_router/mod.rs:87-93` forwards the reply into `MuxEvent::PtyWrite { pane_id, data: bytes }`. `response_poll` is NOT in this path — `oriterm_mux/src/pane/io_thread/response_poll/mod.rs:33-54` handles ONLY `HostRequest::{ClipboardLoad, ColorQuery}`, per blind-spot verification."
  - "Kitty graphics + image lifecycle interactions verified (depends on section 07 non-placeholder handling). Placeholder lifecycle (U=1) is OWNED by this section per §07's carve-out at `plans/spec-conformance/section-07-image-lifecycle-correctness.md:84` (reflow, ED, EL, scroll against cells carrying `U+10EEEE`)."
  - "Kitty + sixel cross-stack regression: §13.6 extends §12.5's already-green `cross_stack_handoff.rs` handshake (which proved coexistence at the `ImageCache`/`RenderableContent` snapshot level) with the deep mixed-protocol rendering regressions (overlapping placements, z-order interleaving with both protocols present, shared-eviction races under LRU pressure) that §12.5 explicitly DEFERRED-TO-DOWNSTREAM. §13.6 does NOT re-prove coexistence and does NOT duplicate §07 lifecycle."
  - "Existing teseq kitty scenarios remain green (zero-count today — `oriterm_core/tests/teseq/main.rs:57` has no kitty family module; this criterion is a non-regression baseline for any kitty teseq scenarios that land in §23.5 archival work)."
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release (workspace + `x86_64-pc-windows-gnu` cross-compile)."
  - "Section's mission criterion connection: contributes to **Verification chain complete per row** AND **Image lifecycle correct under resize/reflow/scrollback/alt-screen** (placeholder-cell reflow is part of the lifecycle invariant)."
inspired_by:
  - "kitty source itself — `~/projects/reference_repos/console_repos/kitty/kitty/graphics.py` — kitty IS the spec"
  - "sw.kovidgoyal.net/kitty/graphics-protocol/ — public protocol documentation"
  - "wezterm `term/src/terminalstate/kitty.rs` — production reference for chunked transmission, animation, frame composition"
depends_on: ["12"]
third_party_review:
  status: resolved
  updated: 2026-04-21
  notes: "§13.0.5 TPR: user-accepted at iter_cap_reached after 3 rounds; 10 in-scope findings fixed inline with 12 regression pins in `delete/tests.rs`; 2 out-of-scope findings triaged via `/verify-tpr` on 2026-04-21 and closed with concrete anchors (`TPR-13.0.5-R1-F3-gemini` → new `- [ ]` v=0 loop-count task in §13.3 with `blocked-by` marker; `TPR-13.0.5-R2-F3-gemini` → existing §13.6 shared-eviction-race bullet annotated with LRU→FIFO preconditions + `blocked-by` marker + default fix prescription). §13.R has zero open items. §13.0 section-wide TPR: user-accepted at iter_cap_reached after 3 rounds; 10 findings fixed inline (zero outstanding); survivor-mode all 3 rounds (gemini 429 persistent)."
sections:
  - id: "13.0"
    title: "Top-down spec audit + catalog row carve-out (BLOCKING)"
    status: complete
  - id: "13.0.5"
    title: "BUG-08-7 + BUG-08-8 closure (BLOCKING precondition for 13.1+)"
    status: complete
  - id: "13.1"
    title: "Verify kitty action + format + transmission combinations (t=d/f/t, f=24/32/100, a=t/T/p/q)"
    status: not-started
  - id: "13.2"
    title: "Verify chunked transmission (m=1 / m=0 coalesce + decode + malformed-base64 reply path)"
    status: not-started
  - id: "13.3"
    title: "Verify animation (a=f + a=a) with RenderScheduler-driven paced redraw"
    status: not-started
  - id: "13.4"
    title: "Implement + verify virtual placements (U=1 unicode placeholders — currently `missing`)"
    status: not-started
  - id: "13.5"
    title: "Verify image protocol replies via Effect transcript apex (Effect::Pty at effect_router)"
    status: not-started
  - id: "13.6"
    title: "Verify kitty + sixel cross-stack mixed-protocol rendering regressions (extends §12.5)"
    status: not-started
  - id: "13.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "13.N"
    title: "Completion Checklist"
    status: not-started
# TPR Checkpoint Placement: 13.0.5 (after BUG-08-7/8 closure — structural gate),
# 13.3 (after action+chunked+animation — covers §13.0 through §13.3),
# 13.6 (after placeholder+reply+cross-stack — covers §13.4 through §13.6),
# final in 13.N
---

# Section 13: Kitty Graphics Protocol

**Status:** Not Started
**Goal:** Verify every kitty graphics catalog row AND implement the currently-missing U=1 unicode placeholder rendering. Kitty is the second full visual stack and shares the image cache + GPU image pipeline with sixel; cross-stack regression sweeps catch interactions.

**Success Criteria:** see frontmatter.

**Code seam this section owns (in-crate anchors):**

- Parser — `oriterm_core/src/image/kitty/parse.rs` (`parse_kitty_command`, `apply_key_value`, `KittyAction` enum at lines 59-74, `KittyTransmission` enum at lines 77-87, `decode_base64` at 253-291).
- Dispatcher — `oriterm_core/src/term/handler/image/kitty.rs` (`handle_kitty_graphics` at 33-62, per-action arms `kitty_query`/`kitty_transmit`/`kitty_transmit_and_place`/`kitty_place`/`kitty_delete`/`kitty_store_image`/`kitty_respond`). **480 lines** as of this plan write — 20-line headroom against the 500-line hard limit in `.claude/rules/code-hygiene.md` §File Size (BUG-08-8 gates this structural split).
- Animation — `oriterm_core/src/term/handler/image/kitty_animation.rs` (`kitty_frame`, `kitty_animate`; composition-mode decision at 58-62).
- Animation timer — `oriterm_core/src/term/image_config.rs:65` (`Term::advance_animations`). **HAS NO PRODUCTION CALLER** — verified by grep across `oriterm/` and `oriterm_mux/`. §13.3 wires this up.
- Placeholder-mode site — `oriterm_core/src/term/handler/image/kitty.rs:106-108` (U=1 suppresses `kitty_create_placement`; the stored image is never reached at render time because no cell-level lookup of `U+10EEEE` exists). §13.4 owns the implementation.
- Reply emission — `Term::kitty_respond` at `oriterm_core/src/term/handler/image/kitty.rs:466-479` constructs `Effect::Pty(PtyEffect::Write { kind: PtyWriteKind::ImageProtocolReply, bytes })`.
- Reply routing — `Effect::Pty(PtyEffect::Write { bytes, .. })` arm at `oriterm_mux/src/pane/io_thread/effect_router/mod.rs:87-93` forwards as `MuxEvent::PtyWrite`. NOT via `response_poll` (that module handles only `HostRequest::ClipboardLoad`/`ColorQuery`).

**Context:** Pass 1 confirmed kitty graphics is implemented end-to-end at the paths above. The audit memory's "kitty q=1 query NOT IMPLEMENTED" claim is stale — the query IS handled (`parse.rs:197` + `kitty.rs:65-68`). Animation supports both Overwrite and AlphaBlend modes (`kitty_animation.rs:58-62`). The image protocol replies emit via `Term::kitty_respond` at `kitty.rs:466-479` onto the `EffectSink`; Section 03's effect boundary migration has already made this the production path.

**Blockers encoded as subsection gates (NOT as frontmatter `depends_on:`, which takes section-number tokens, not bug IDs):**

- **BUG-08-8** (kitty.rs BLOAT split) — `oriterm_core/src/term/handler/image/kitty.rs` is currently **480 lines** (verified via `wc -l` at plan-write time, 2026-04-21). The 500-line hard limit in `.claude/rules/code-hygiene.md` §File Size leaves 20 lines of headroom. Any per-action implementation or new-handler work landed on this baseline pushes the file through the limit, which would force a mid-subsection mechanical refactor at the worst time (feature work mixed with file moves). §13.0.5 is the blocking precondition that closes BUG-08-8 with the canonical `kitty/mod.rs` + per-action submodule split (`transmit.rs`, `place.rs`, `delete.rs`, `frame_compose.rs`, `animate.rs`, `query.rs`, `response.rs`). The split is a PRECONDITION for §13.1; verification against the pre-split monolith is disallowed.
- **BUG-08-7** (kitty delete specifier correctness) — `kitty_delete` at `oriterm_core/src/term/handler/image/kitty.rs:168-253` has 4 wrong specifier mappings (`d=a`, `d=c`, `d=p`, `d=r` all diverge from the protocol spec per BUG-08-7's detail in `plans/bug-tracker/section-08-core-terminal.md:85-90`) AND is missing `d=q`/`d=Q`/`d=f`/`d=F`/`d=n`/`d=N`. §13.1's per-specifier verification against broken code is meaningless — a green delete test against the wrong arm is a worse outcome than a red test. §13.0.5 is the blocking precondition that closes BUG-08-7 alongside BUG-08-8 (the file split creates the natural `delete.rs` home where the corrected specifier logic lives).
- The line-count reference in this paragraph (480 / 20-line headroom) is written with a tolerance: if `wc -l` at §13.0.5 entry returns any count ≥ 450, the §13.0.5 implementation plan and the completion-checklist line-count assertion MUST reflect the actual count at that moment. The 480 number is the plan-write baseline, not a frozen claim.

**Reference implementations:** see frontmatter. In particular, wezterm's `term/src/terminalstate/kitty.rs` is the production reference for chunked transmission, animation, frame composition, AND the unicode placeholder rendering path (the wezterm implementation is feature-complete on U=1; ori_term is not yet).

**Depends on:** Section 12 (sixel landed; image cache + GPU pipeline shared with kitty; §12.5's `cross_stack_handoff.rs` handshake is the coexistence baseline that §13.6 builds on). Section 07 (image lifecycle for non-placeholder placements; placeholder lifecycle is carved OUT to §13.4 per §07's body at line 84).

---

## 13.0 Top-down spec audit + catalog row carve-out (BLOCKING — precedes all other subsections)

**Goal:** Walk the canonical spec source(s) for this stack TOP-DOWN. Every sequence the spec defines gets a row in this section's audit file at `plans/spec-conformance/audits/section-13-top-down-inventory.md`, mapped to either an existing catalog row ID or an explicit `not-targeted` decision with rationale. **Expand `catalog/kitty-graphics.md` from its current 9 coarse rows into fine-grained per-arm rows** so broken branches cannot hide inside a green aggregate.

**Why this exists:** Section 09A introduced the `audits/` SSOT to close the bottom-up catalog construction gap. The current `catalog/kitty-graphics.md` has only 9 rows (7 core actions + 1 unicode-placeholder + 1 response), each collapsing multiple semantics into one row — `KG-TRANSMIT` covers 3 transmission modes × 3 formats × compression flag × chunked state (36 cells minimum); `KG-DELETE` covers 18 specifiers (`a`/`A`/`i`/`I`/`p`/`P`/`c`/`C`/`x`/`X`/`y`/`Y`/`z`/`Z`/`r`/`R`/`n`/`N`); `KG-FRAME` and `KG-ANIMATE` each collapse multiple sub-operations; `KG-RESPONSE` collapses 6+ distinct error codes. At this granularity, a per-specifier bug (BUG-08-7, 4 wrong arms) can sit inside a "verified" `KG-DELETE` row indefinitely. The §13.0 expansion forces the row granularity that makes per-arm failure visible.

**Canonical spec source(s):** sw.kovidgoyal.net/kitty/graphics-protocol/ docs (primary, kitty source is the de facto SPEC for this protocol) + kitty source `kittens/icat/icat.py` cross-reference + wezterm `term/src/terminalstate/kitty.rs` behavior reference.

**Files touched:**
- `plans/spec-conformance/audits/section-13-top-down-inventory.md` (POPULATE — currently a stub at `audits/section-13-top-down-inventory.md:19-21` with a TODO row and `last_walked: null`).
- `plans/spec-conformance/catalog/kitty-graphics.md` (EXPAND — open new fine-grained rows using the canonical 10-column schema from `plans/spec-conformance/00-overview.md §Catalog Row Schema`).

**Completion criteria:**

- [x] Audit file `plans/spec-conformance/audits/section-13-top-down-inventory.md` is populated with every sequence + every key-value arm in the canonical spec source(s). The table has one row per (action, parameter, value) triple the spec names.
- [x] Audit file contains **per-action rows** for every `a=` value: `a=t` (Transmit), `a=T` (TransmitAndPlace, parsed as the fallback arm in `parse.rs:198-199` rather than a distinct match arm — document this), `a=p` (Place), `a=d` (Delete), `a=f` (Frame), `a=a` (Animate), `a=q` (Query). For `a=c` (Compose — kitty's separate "compose frame" action operating on already-transmitted frames, with its own key set `r,c,w,h,x,y,X,Y,C` per the kitty graphics protocol docs), `KittyAction` has NO matching variant — `apply_key_value` at `parse.rs:189-200` silently falls back to `TransmitAndPlace` on any unrecognized `a=` value including `a=c`. Audit decision MUST capture this as an **implementation gap**, not as a rationalized `not-targeted`: open a dedicated `KG-ACTION-COMPOSE` catalog row (status `missing`) + a sibling `KG-ACTION-FALLBACK-TRANSMITANDPLACE` row (status `verified-with-deviation`) that pins the current silent fallback behavior. Do NOT conflate `a=c` with `a=f`'s `cell_x_offset == 1` → `CompositionMode::Overwrite` at `kitty_animation.rs:58-62` — that path controls the in-frame blend mode of a NEWLY-TRANSMITTED animation frame, not the compose-onto-existing-frame semantics of `a=c`. The fallback-to-TransmitAndPlace behavior is separately load-bearing on unknown `a=` values and keeps its own row.
- [x] Audit file contains **per-transmission rows** for every `t=` value: `t=d` (Direct), `t=f` (File), `t=t` (TempFile), `t=s` (SharedMemory). For `t=s`, audit decision MUST capture the EINVAL rejection at `kitty.rs:289-291` as `verified-with-deviation` with rationale "ori_term does not implement shared-memory transport; rejection IS the spec-compliant response on a platform that lacks the transport" — a bare `verified` on an aggregated KG-TRANSMIT would wrongly codify the rejection.
- [x] Audit file contains **per-format rows** for every `f=` value: `f=24` (RGB), `f=32` (RGBA), `f=100` (PNG), `f=<other>` (unsupported-format error reply emitted at `kitty.rs:320-344` inside `kitty_decode_pixels`; post-split: `store.rs`). The `KittyError::UnsupportedFormat(u32)` variant exists at `parse.rs:97` but is NEVER returned by `parse_kitty_command` — format validation is deferred to the decode path, which emits the `EINVAL: unsupported image format` reply string. Open a `KG-FORMAT-UNSUPPORTED` catalog row that pins the `kitty.rs:320-344` emission (not the unused enum variant).
- [x] Audit file contains **per-specifier rows** for every `d=` delete value in BUG-08-7's scope: `d=a`, `d=A`, `d=i`, `d=I`, `d=p`, `d=P`, `d=c`, `d=C`, `d=x`, `d=X`, `d=y`, `d=Y`, `d=z`, `d=Z`, `d=r`, `d=R`, `d=n`, `d=N`. `d=q`, `d=Q`, `d=f`, `d=F` (missing per BUG-08-7) get dedicated `missing` rows that §13.0.5 / §13.1 drive to `verified`.
- [x] Audit file contains **per-compression rows**: `o=z` (zlib — currently parsed at `parse.rs:215` but NEVER consumed; audit decision MUST be either (a) `not-targeted` with rationale "clients using compression see ENOMEM or garbled payload; ori_term does not decompress", OR (b) `mapped` to a concrete KG-COMPRESSION-OZ row that §13 must implement). Silent no-op consumption is NOT a valid decision.
- [x] Audit file contains **per-frame-action rows** for `a=f`: distinct catalog rows for `KG-FRAME-TRANSMIT` (append new frame), `KG-FRAME-REPLACE` (`c=` replace frame N), `KG-FRAME-EDIT` (`r=` edit frame N), `KG-FRAME-COMPOSITE-OVERWRITE` (cell_x_offset == 1), `KG-FRAME-COMPOSITE-ALPHABLEND` (default).
- [x] Audit file contains **per-animate-action rows** for `a=a`: distinct catalog rows for `KG-ANIMATE-STOP` (s=1), `KG-ANIMATE-RUN-WAIT` (s=2), `KG-ANIMATE-RUN` (s=3), `KG-ANIMATE-LOOP-COUNT` (v=), `KG-ANIMATE-SET-CURRENT-FRAME` (r= and c=), `KG-ANIMATE-SET-FRAME-GAP` (z=).
- [x] Audit file contains **per-response-code rows**: `KG-RESPONSE-OK`, `KG-RESPONSE-EBADF`, `KG-RESPONSE-EBIG`, `KG-RESPONSE-EINVAL`, `KG-RESPONSE-ENOENT`, `KG-RESPONSE-ENOMEM`, `KG-RESPONSE-EIO` + quiet-level gating (q=0/q=1/q=2) captured in a separate `KG-RESPONSE-QUIET` behavioral row.
- [x] Audit file contains **per-placeholder-arm rows** for U=1: `KG-UNICODE-PLACEHOLDER-TRANSMIT-U1` (transmit suppresses placement), `KG-UNICODE-PLACEHOLDER-PLACE-U1` (place suppresses), `KG-UNICODE-PLACEHOLDER-CELL-RESOLVE` (U+10EEEE + diacritic row/column encoding → image lookup at render time — currently MISSING, §13.4 implements), `KG-UNICODE-PLACEHOLDER-REFLOW` (placeholder cells move with text under reflow/scroll — currently MISSING, §13.4 implements).
- [x] Audit file contains **`not-targeted` rows with written rationale** for any key-value arms the plan intentionally excludes (e.g., experimental kitty keys not in the stable protocol, kittens/icat-specific client-only sequences).
- [x] Every row in the audit-file table has a `Decision` of `mapped` (cites a catalog row ID) or `not-targeted` (with one-line rationale).
- [x] Every `mapped` row resolves to a real catalog row that exists in `plans/spec-conformance/catalog/kitty-graphics.md` — if the row does not exist yet, THIS subsection adds it.
- [x] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes for this audit file.
- [x] Audit file `last_walked` frontmatter is set to today's date and `walked_by` to the implementer's handle.
- [x] New catalog rows in `catalog/kitty-graphics.md` use the canonical 10-column schema from `plans/spec-conformance/00-overview.md §Catalog Row Schema` (frozen v1.0 — 2026-04-13).
- [x] `catalog/kitty-graphics.md` row count grows from 9 (pre-expansion) to ≥ 40 post-expansion (minimum: 7 action + 4 transmission + 4 format + 18 delete-specifier + 6 response-code + 5 frame-action + 6 animate-action + 4 placeholder-arm ≈ 54 concrete rows, plus behavioral rows). The exact count MUST be asserted by the §13.N completion checklist grep.

**No other subsection in this section can begin work until §13.0 is complete.** This is a hard gate. The audit file IS the row set §13.1–§13.6 drive to `verified`.

---

## 13.0.5 BUG-08-7 + BUG-08-8 closure (BLOCKING precondition for §13.1+)

**Goal:** Close BUG-08-7 (delete-specifier correctness) and BUG-08-8 (`kitty.rs` BLOAT split) BEFORE any verification subsection runs. Both bugs directly block the §13.1 per-specifier + per-action matrix — a verified checkbox against either unfixed bug would encode broken behavior as spec-compliant.

**Why this is a subsection and not a trailing completion gate:** The prior plan body gated BUG-08-7 / BUG-08-8 in §13.N (final completion checklist), which makes their fix "work done last" — verification work in §13.1 would run against broken code, then the fix in §13.N would either (a) regress the §13.1 tests, forcing rework, or (b) fix the bug but leave the §13.1 tests asserting the old broken semantics (because writing tests against broken code and then fixing the code forces a test rewrite). Both outcomes are worse than closing the bugs first. Per `.claude/rules/tests.md` §TDD for Bugs, the fix is written before its verification; §13.0.5 restores that ordering.

**Files touched:**
- `plans/bug-tracker/section-08-core-terminal.md` (flip BUG-08-7 and BUG-08-8 entries from `- [ ]` to `- [x]` with fix citations — these are the scanner gates `/continue-roadmap` Step 1.92 reads).
- `oriterm_core/src/term/handler/image/kitty.rs` (delete the monolithic file after the split lands; the replacement is `kitty/mod.rs` per BUG-08-8's proposed fix).
- `oriterm_core/src/term/handler/image/kitty/mod.rs` (NEW — dispatch entry; reads `KittyCommand::action` and routes to per-action submodule).
- `oriterm_core/src/term/handler/image/kitty/transmit.rs` (NEW — `kitty_transmit`, `kitty_transmit_and_place`, `kitty_finalize_payload`, `kitty_accumulate_chunk`).
- `oriterm_core/src/term/handler/image/kitty/place.rs` (NEW — `kitty_place`, `kitty_create_placement`).
- `oriterm_core/src/term/handler/image/kitty/delete/mod.rs` (NEW — `kitty_delete` with the corrected specifier logic per BUG-08-7's spec citations; directory module per `.claude/rules/test-organization.md` §Sibling `tests.rs` Pattern because the module has tests — NO file-module `delete.rs` alongside `delete/`).
- `oriterm_core/src/term/handler/image/kitty/delete/tests.rs` (NEW — sibling tests for every delete specifier arm).
- `oriterm_core/src/term/handler/image/kitty/store.rs` (NEW — `kitty_store_image`, `kitty_decode_pixels`, `kitty_store_from_file`).
- `oriterm_core/src/term/handler/image/kitty/query.rs` (NEW — `kitty_query`).
- `oriterm_core/src/term/handler/image/kitty/response.rs` (NEW — `kitty_respond`).
- `oriterm_core/src/term/handler/image/kitty_animation.rs` (may stay in place as a sibling, OR move to `kitty/frame_compose.rs` + `kitty/animate.rs` if BUG-08-8's proposed fix diagram is followed verbatim; editor decision deferred to §13.0.5 implementation).

**Completion criteria:**

- [x] **Write failing test matrix BEFORE implementation** (TDD per `.claude/rules/tests.md` §TDD for Bugs). Per-specifier red tests for all 22 `d=` arms — the 18 existing-but-broken arms (`a`/`A`/`i`/`I`/`p`/`P`/`c`/`C`/`x`/`X`/`y`/`Y`/`z`/`Z`/`r`/`R`/`n`/`N`) AND the 4 missing arms that §13.0.5 implements (`q`/`Q`/`f`/`F`). Tests live at `oriterm_core/src/term/handler/image/kitty/delete/tests.rs` (29 tests — 22 per-arm coverage + 4 negative pins for BUG-08-7 regressions + `delete_specifier_matrix_completeness` + `delete_case_pair_contract_lowercase_keeps_data_uppercase_frees`). Red-phase confirmed: 20 of 29 failed against pre-fix code.
- [x] **Fix BUG-08-7's 4 wrong specifier arms** — all corrected in `oriterm_core/src/term/handler/image/kitty/delete/mod.rs`:
  - `d=a`: now `remove_visible_placements(viewport_top, viewport_bottom)` (keeps off-screen + image data for lowercase); `cache.clear()` path removed.
  - `d=c`: now `remove_by_position(cursor_col, cursor_row)` via `kitty_delete_at_position`; column-only match path removed.
  - `d=p`: now `remove_by_position((x-1), (y-1))` via `kitty_delete_at_cell`; ignores `i=`/`p=` keys per spec.
  - `d=r`: now `remove_placements_in_id_range(ImageId(x), ImageId(y))`; cursor-position deletion path removed.
- [x] **Implement the missing specifiers** — all 4 BUG-08-7 arms plus d=n/d=N:
  - `d=q`/`d=Q`: new `ImageCache::remove_placements_at_cell_with_z(col, row, z)`.
  - `d=f`/`d=F`: new `ImageCache::has_extra_animation_frames` + `remove_animation_frame(id, frame_number)`; `d=F` on a static image removes the image entirely per kitty graphics.c:1696.
  - `d=n`/`d=N`: new `ImageData::image_number: Option<u32>` threaded from `KittyCommand.image_number` → `KittyStoreParams::image_number` → `ImageData::image_number`. New `ImageCache::newest_by_image_number(number) -> Option<ImageId>` resolves the newest image by `last_accessed`. Audit at `plans/spec-conformance/audits/section-13-top-down-inventory.md:111-112` maps d=n/d=N to `KG-DELETE-n`/`KG-DELETE-N` (not `not-targeted`), so §13.0.5 implements rather than stubs.
- [x] **Close BUG-08-8 via the split** — landed in commit `4d46d793` (pre-dating §13.0.5 implementation): `kitty/mod.rs` + `kitty/{transmit,place,delete/mod,store,query,response,frame,animate}.rs`. `delete/` is a directory module because it has a sibling `tests.rs` (§13.0.5 added the tests).
- [x] **No file in the split exceeds the 500-line hard limit** — verified via `wc -l oriterm_core/src/term/handler/image/kitty/*.rs oriterm_core/src/term/handler/image/kitty/delete/mod.rs`: mod.rs=130, delete/mod.rs=198, store.rs=132, place.rs=91, frame.rs=75, animate.rs=66, transmit.rs=51, response.rs=23, query.rs=13. Every file ≤ 200 lines; well below the 500 hard limit.
- [x] **Semantic pins** — negative pins for all 4 BUG-08-7 regressions land in `delete/tests.rs`: `delete_a_negative_pin_does_not_clear_entire_cache`, `delete_p_negative_pin_ignores_placement_id_key`, `delete_c_negative_pin_does_not_delete_entire_cursor_column`, `delete_r_negative_pin_does_not_use_cursor_position`. Each would fail if the pre-fix broken code returned.
- [x] **Matrix dimension** — `delete_specifier_matrix_completeness` iterates all 22 specifiers and asserts `count == 22`; `delete_case_pair_contract_lowercase_keeps_data_uppercase_frees` enumerates 9 case-pairs (i/I, p/P, c/C, x/X, y/Y, z/Z, r/R, n/N, q/Q) and pins the lowercase-keeps-data / uppercase-frees-data invariant end-to-end.
- [x] **Update catalog** — `plans/spec-conformance/catalog/kitty-graphics.md` `KG-ACTION-DELETE` + all 22 `KG-DELETE-{a,A,i,I,p,P,c,C,x,X,y,Y,z,Z,r,R,q,Q,f,F,n,N}` flipped to `verified`. `KG-ACTION-FALLBACK-TRANSMITANDPLACE`, `KG-TRANSMIT-SHARED-MEM-REJECTED`, `KG-COMPRESSION-OZ-IGNORED` (§13.0 opened as `verified-with-deviation` but uncited) now cited by new parser/handler tests in `oriterm_core/src/image/kitty/tests.rs`.
- [x] **Bug tracker entries flipped** — `plans/bug-tracker/section-08-core-terminal.md` BUG-08-7 (:85) and BUG-08-8 (:92) both marked `- [x]` with full fix body. BUG-08-8's entry cites commit `4d46d793`; BUG-08-7's cites §13.0.5 anchors.
- [x] Verify all tests pass in both debug AND release builds — `./build-all.sh` green (debug + release + x86_64-pc-windows-gnu cross-compile), `./test-all.sh` green (1968 lib tests + spec-coverage + catalog-coverage), `./clippy-all.sh` green.
- [x] **TPR checkpoint** — `/tpr-review` ran 3 rounds (iter_cap_reached, user-accepted 2026-04-21). Codex surfaced 9 findings across rounds (high→low); gemini surfaced 8 (critical→low; round 0 clean). 10 in-scope findings fixed inline with 12 regression-test pins in `delete/tests.rs` (41 tests total); 2 out-of-scope findings (v=0 animate ownership → §13.3; LRU snapshot render-touching → future) filed in §13.R as `- [ ]` items per `CLAUDE.md §NEVER reason out of TPR findings`. Agreement findings (round 1 F2 / round 2 F2) attributed to both reviewers.

**No other verification subsection (§13.1–§13.6) can begin work until §13.0.5 is complete.** This is a hard gate. The §13.1 per-specifier verification matrix runs against the post-fix code.

---

## 13.1 Verify kitty action + format + transmission combinations

**File(s):** `oriterm_core/tests/spec_chain/kitty/actions.rs` (new)

**Depends on code paths:** post-split `kitty/mod.rs` + `kitty/transmit.rs` + `kitty/place.rs` + `kitty/query.rs` + `kitty/delete.rs` from §13.0.5. Parser at `oriterm_core/src/image/kitty/parse.rs`.

- [ ] Write failing test matrix BEFORE implementation (TDD per `.claude/rules/tests.md` §TDD for Bugs).
- [ ] **Per-action rung** — for each action in the post-13.0.5 `KittyAction` enum (`Transmit`, `TransmitAndPlace`, `Place`, `Delete`, `Frame`, `Animate`, `Query`), spec_chain test drives the action through parser → dispatch → state-or-effect rung. For `Query`, the apex is the `Effect::Pty(PtyEffect::Write { kind: PtyWriteKind::ImageProtocolReply, bytes: b"\x1b_Gi=<id>;OK\x1b\\" })` observation. For `Place`, the apex is `RenderableContent::images` containing the placement at the expected cell. For `Delete`, the apex is the pre/post placement count (§13.0.5's delete tests cover the per-specifier matrix; §13.1's delete test is a smoke that confirms the dispatch reaches `kitty_delete` — the heavy lifting is in §13.0.5).
- [ ] **Per-format rung** — for each format at `oriterm_core/src/term/handler/image/kitty/store.rs` `kitty_decode_pixels` (post-split), drive `f=24` (RGB) + `f=32` (RGBA) + `f=100` (PNG) transmit + place. Payload sizing assertions pin the expected-vs-actual bytes error path at `store.rs` lines corresponding to the current `kitty.rs:326-332` check. A bad-payload-length transmit asserts the reply is `Effect::Pty(PtyEffect::Write { bytes: b"\x1b_Gi=<id>;EINVAL: RGBA payload size ...\x1b\\" })`.
- [ ] **Per-transmission rung** — drive `t=d` (Direct, the common path), `t=f` (File, reads from a tempfile fixture; path-traversal guard at `kitty.rs:355-359` pinned by a `..`-in-path negative test), `t=t` (TempFile — verify the source file is removed after read via `std::fs::remove_file` at `kitty.rs:367-369,373-375`), `t=s` (SharedMemory — verify `Effect::Pty(PtyEffect::Write { bytes: b"...;EINVAL: shared memory ..." })` reply and NO placement created — this IS the `verified-with-deviation` behavior from §13.0's catalog row).
- [ ] **Matrix count assertion** — `action_format_transmission_matrix_completeness` test asserts `ACTIONS.len() * FORMATS.len() * TRANSMISSIONS.len()` cells exercised per `.claude/rules/tests.md` §Self-Verifying Matrix Completeness.
- [ ] **Semantic pin** — at least one test that ONLY passes with the post-13.0.5 code (e.g., `transmit_and_place_unknown_action_key_falls_back_to_TransmitAndPlace` pinning the `parse.rs:189-200` fallback behavior if §13.0 kept it as `verified-with-deviation`, OR failing if §13.0 implemented a distinct `KittyAction::Compose` variant).
- [ ] **Negative pin** — `a=<invalid>` unknown-value test that proves the fallback arm is exercised: the test MUST fail if `apply_key_value` at `parse.rs:189-200` is changed to `return Err(...)` instead of defaulting to `TransmitAndPlace`.
- [ ] Update catalog rows opened in §13.0 (`KG-TRANSMIT-DIRECT`, `KG-TRANSMIT-FILE`, `KG-TRANSMIT-TEMPFILE`, `KG-TRANSMIT-SHARED-MEM-REJECTED`, `KG-TRANSMIT-FORMAT-24/32/100`, `KG-FORMAT-UNSUPPORTED`, `KG-ACTION-{TRANSMIT,TRANSMIT-AND-PLACE,PLACE,QUERY}`, `KG-ACTION-FALLBACK-TRANSMITANDPLACE`) to `verified` (or `verified-with-deviation` for the SharedMemory rejection and the unknown-`a=` fallback). `KG-ACTION-COMPOSE` stays `missing` — §13 does NOT drive it to `verified` unless a dedicated `KittyAction::Compose` variant is implemented; the row remains a tracked implementation gap owned by a future section.
- [ ] Verify all tests pass in both debug AND release builds.
- [ ] **Validation:** parser → dispatch → state/effect rung green across the full action × format × transmission matrix.

---

## 13.2 Verify chunked transmission + malformed-base64 reply path

**File(s):** `oriterm_core/tests/spec_chain/kitty/chunked.rs` (new). Implementation edit: `oriterm_core/src/term/handler/image/kitty/mod.rs` (post-split `handle_kitty_graphics`) for the malformed-base64 reply wiring.

**Depends on code paths:** post-split `kitty/transmit.rs::kitty_accumulate_chunk` + `kitty/store.rs::kitty_store_image`. Parser at `oriterm_core/src/image/kitty/parse.rs` — `decode_base64` at 253-291 produces `KittyError::InvalidBase64`.

- [ ] Write failing test matrix BEFORE implementation (TDD per `.claude/rules/tests.md` §TDD for Bugs).
- [ ] **Chunked coalesce + decode** — feed an image transmission split across N chunks (`m=1`, `m=1`, ..., `m=0`). Assert via `RenderableContent::image_data` that the coalesced + decoded image matches the expected pixel data. Payload-independence pin per §12.5's handshake pattern: the expected image has a distinct identifying shape (e.g., 4×4 RGBA with specific color bands) so mis-coalesced chunks fail loudly.
- [ ] **Chunked out-of-order rejection** — feed chunks out of order. Assert the behavior matches spec: kitty protocol does NOT define per-chunk ordering, so ori_term's current `kitty_accumulate_chunk` at `kitty.rs:256-280` (post-split: `kitty/transmit.rs`) appends in arrival order. The audit file row captures this as `verified-with-deviation` if §13.0's decision was "append in arrival order; rely on sender"; the test pins this behavior as load-bearing.
- [ ] **Chunked size-limit rejection** — feed chunks that exceed `max_single_image_bytes` at `kitty.rs:257` (post-split: `kitty/transmit.rs`). Assert the loading-image state is discarded + `warn!` logged. No reply is emitted on this path (current code); audit decision captured in §13.0.
- [ ] **Malformed-base64 reply path (IMPLEMENTATION + verification)** — current behavior at `kitty.rs:37-42` (post-split: `kitty/mod.rs::handle_kitty_graphics`) on `parse_kitty_command` error:
  ```
  Err(e) => { warn!("kitty graphics parse error: {e}"); return; }
  ```
  This silently drops the chunk with no reply emitted. Per the kitty protocol spec, a malformed payload SHOULD emit an error reply so the client can recover. §13.2 MUST make a concrete decision:
  - **Option A (implement reply):** Modify `handle_kitty_graphics` to construct an `Effect::Pty(PtyEffect::Write { kind: PtyWriteKind::ImageProtocolReply, bytes: b"\x1b_Gi=0;EINVAL: base64 decode failed\x1b\\" })` reply when `parse_kitty_command` returns `Err(KittyError::InvalidBase64)`. The i=0 fallback is because the image_id is never extracted on parse failure; audit file documents the deviation from kitty's image-id-echo convention. Test asserts the effect emission via the `oriterm_core` test harness's `EffectSink` transcript.
  - **Option B (document silent drop):** Leave the silent drop as is. §13.0's catalog row decision MUST be `verified-with-deviation` with rationale "ori_term drops malformed payloads silently; kitty protocol is silent on whether a reply is required". Test asserts NO effect is emitted AND the `warn!` path is reached (via a test-only log observer).
  - §13.2 MUST pick ONE of these options in its opening bullet and drive it end-to-end. The prior plan wording ("assert the parser emits an error reply") presumed Option A without noting the implementation gap; this rewrite forces the decision.
- [ ] **Matrix count assertion** — `chunked_category_matrix_completeness` test asserts the 4 categories above are all exercised.
- [ ] **Semantic pin** — at least one test that ONLY passes with the chosen Option (A or B). For Option A: `malformed_base64_emits_EINVAL_reply`. For Option B: `malformed_base64_silent_drop_emits_no_reply`.
- [ ] **Negative pin** — at least one test that asserts the opposite behavior to Option (A or B) does NOT occur. Per `.claude/rules/tests.md` §Matrix Clamping, this pairs with the semantic pin.
- [ ] Update catalog rows opened in §13.0 for chunked + base64-error arms to `verified` (or `verified-with-deviation` per Option B if selected).
- [ ] Verify all tests pass in both debug AND release builds.
- [ ] **Validation:** chunked coalesce + malformed-payload path pinned by semantic + negative pins; the chosen reply-or-drop behavior is load-bearing.

---

## 13.3 Verify animation (a=f + a=a) with RenderScheduler-driven paced redraw

**File(s):** `oriterm_core/tests/spec_chain/kitty/animation.rs` (new). Implementation edits: `oriterm_ui/src/animation/scheduler/mod.rs` (new `request_frame_at(wake_at)` API + sibling `tests.rs` pin) + `oriterm_mux/src/pane/io_thread/mod.rs` (new `advance_animations` tick in the IO-thread loop) + `oriterm/src/app/event_loop.rs` (populate `ControlFlowInput.scheduler_wake` from `WindowRoot::scheduler().next_wake_time()` at the construction site — currently hardcoded `None` at `event_loop.rs:480`). `oriterm/src/app/event_loop_helpers/mod.rs` stays untouched as the pure control-flow consumer.

**Depends on code paths:** post-split `kitty/frame_compose.rs` (or `kitty_animation.rs` if unmoved) — `kitty_frame` + `kitty_animate`. `Term::advance_animations` at `oriterm_core/src/term/image_config.rs:65` returning `Option<Instant>` next-frame-deadline. `ImageCache::advance_animations` at `oriterm_core/src/image/cache/animation.rs:109`. `RenderScheduler` at `oriterm_ui/src/animation/scheduler/mod.rs:40` owned by `WindowRoot` at `oriterm_ui/src/window_root/mod.rs:65`.

**Why implementation + verification, not just verification:** `Term::advance_animations` exists (with passing unit tests at `oriterm_core/src/image/tests.rs:746-837`) but has NO production caller in `oriterm/src/` or `oriterm_mux/src/` — verified by `grep -r advance_animations`. The cache will happily advance frames when called from tests, but no frame deadline ever reaches the main render loop. Verification of "animation works" at the snapshot level is meaningless without the timer-driven path that actually shows the next frame on screen.

- [ ] Write failing test matrix BEFORE implementation (TDD per `.claude/rules/tests.md` §TDD for Bugs).
- [ ] **Implement the animation timer** — wire `Term::advance_animations` into the pane IO thread. Add the missing scheduler API first, then wire the path:
  - **Scheduler API addition (`oriterm_ui/src/animation/scheduler/mod.rs`):** add `pub fn request_frame_at(&mut self, wake_at: Instant)` that records a widget-less absolute-deadline wakeup on the `deferred_repaints` heap so `next_wake_time()` at `scheduler/mod.rs:93-97` surfaces it to the event loop. (The existing `request_repaint_after(widget_id, duration, now)` requires a widget id + relative duration; animation deadlines are pane-global with absolute instants, so a dedicated entrypoint is the correct shape — do NOT reuse `request_repaint_after` by synthesizing a fake widget id.) Sibling `tests.rs` pin per `.claude/rules/test-organization.md`: `request_frame_at_at_instant_surfaces_as_next_wake_time`.
  - In `oriterm_mux/src/pane/io_thread/mod.rs`, on each loop iteration (between VTE parse batches and snapshot production), call `term.advance_animations(Instant::now())`.
  - The returned `Option<Instant>` deadline is forwarded to the main thread as a new `MuxEvent::NextAnimationDeadline { pane_id, deadline }` (or equivalent wiring — editor note: final event variant may land as a `HostEffect` or `UiEffect` depending on the IO-thread event channel shape at implementation time; the invariant is that the deadline reaches the main thread without blocking).
  - In `oriterm/src/app/event_loop.rs`, feed the deadline into the new `WindowRoot::scheduler_mut().request_frame_at(deadline)` at the `ControlFlowInput` construction site — `ControlFlowInput.scheduler_wake` at `event_loop.rs:480` is currently hardcoded to `None`; populate it from `WindowRoot::scheduler().next_wake_time()` so the existing `compute_control_flow` consumer at `oriterm/src/app/event_loop_helpers/mod.rs:441,476` wakes the event loop at the right time. Do NOT reroute the wiring into `event_loop_helpers/mod.rs` itself — that helper is the pure control-flow consumer; owning the winit wake path belongs to `event_loop.rs`.
  - Cross-platform discipline per `.claude/rules/tests.md` §Cross-Platform Verification: the wiring lives in crate-internal code with no `#[cfg(target_os)]` branches, so Linux/macOS/Windows parity is automatic.
- [ ] **Frame transmit + composition modes** — transmit a base frame (`a=t` with `f=32` RGBA), then append frames via `a=f` with distinct pixel data. Composition matrix: `cell_x_offset == 1` (Overwrite) vs default (AlphaBlend). Assert via `RenderableContent::image_data` that the post-composition pixel buffer matches the expected blend.
- [ ] **Animation playback (`a=a`)** — `s=1` stop, `s=2` run-wait, `s=3` run. Assert `RenderableContent::images` current-frame index advances correctly over time (use a fake clock in tests — the harness exposes a `Term::advance_animations_at(instant)` testing entry point).
- [ ] **Loop count (`v=`)** — assert `v=N` (N>0) sets loops to N; **`v=0` sets infinite loops** (kitty spec: v=0 is infinite, not absent). This requires changing `KittyCommand::source_height` from `u32` to `Option<u32>` at `oriterm_core/src/image/kitty/parse.rs:28` (or adding a separate `loop_count: Option<u32>` field) so `kitty_animate` at `oriterm_core/src/term/handler/image/kitty/animate.rs:34-37` can distinguish absent (no `v=` key) from `v=0` (infinite). Today `if cmd.source_height > 0` silently drops `v=0`. Tests: `animate_v_zero_sets_infinite_loops` (positive pin — transmit `a=a,v=0`, assert `animation_state.loop_count == None` or equivalent infinite sentinel); `animate_v_absent_leaves_loop_count_unchanged` (negative pin — omit `v=`, assert no loop-count write). <!-- blocked-by:TPR-13.0.5-R1-F3-gemini -->
- [ ] **Frame gap timing (`z=` in a=f context, per `kitty_animation.rs:55-56`)** — assert that `ImageCache::advance_animations` returns the correct next-deadline based on frame durations.
- [ ] **Timer-driven redraw integration test** — assert end-to-end that (a) a frame deadline returned from `Term::advance_animations` reaches `RenderScheduler`, (b) the scheduler wakes the event loop at or after the deadline, (c) the next rendered frame reflects the advanced frame index. This test lives in `oriterm/src/app/event_loop_helpers/tests.rs` (cross-crate integration; Term+scheduler composition requires the `oriterm` app-layer test surface).
- [ ] **Semantic pin** — `animation_advances_only_when_production_timer_is_wired`: a test that fails if `Term::advance_animations` is never called from the IO thread (can be implemented as a compile-time check via `#[cfg(test)] static TIMER_WIRED: AtomicBool` that the IO thread sets on each tick; test asserts non-zero after running a pane for a bounded duration).
- [ ] **Negative pin** — `animation_frame_index_does_not_advance_without_timer_tick`: with a paused-time harness, advance the image cache manually but NOT via the IO thread tick — assert the `RenderableContent::images[0]` current-frame stays at 0 (proves the timer-driven path is the load-bearing path, not an internal auto-advance).
- [ ] **Matrix count assertion** — `animation_category_matrix_completeness` asserts 4 composition-mode × action × frame-count × gap-ms cells.
- [ ] Update catalog rows opened in §13.0 (`KG-FRAME-TRANSMIT`, `KG-FRAME-COMPOSITE-OVERWRITE`, `KG-FRAME-COMPOSITE-ALPHABLEND`, `KG-ANIMATE-{STOP,RUN-WAIT,RUN}`, `KG-ANIMATE-LOOP-COUNT`, `KG-ANIMATE-SET-CURRENT-FRAME`, `KG-ANIMATE-SET-FRAME-GAP`) to `verified`.
- [ ] Verify all tests pass in both debug AND release builds.
- [ ] **TPR checkpoint** — `/tpr-review` covering §13.0 through §13.3 (catalog carve-out + BUG-08-7/8 split + action/format/transmission matrix + chunked + animation-with-timer-wiring). Findings recorded in §13.R.

---

## 13.4 Implement + verify virtual placements (U=1 unicode placeholders)

**File(s):** Implementation: `oriterm_core/src/term/handler/image/kitty/placeholder.rs` (NEW — cell-level placeholder resolution) + possible edits in `oriterm_core/src/term/renderable/mod.rs` (expose placeholder cells in snapshot) + `oriterm/src/gpu/prepare/emit.rs` (emit image quads for placeholder-cell rows). Tests: `oriterm_core/tests/spec_chain/kitty/virtual_placements.rs` (new) + GPU-apex pilots at `oriterm/src/gpu/visual_regression/spec_chain/pilots/kitty_placeholder_*.rs` (new).

**Depends on code paths:** `oriterm_core/src/term/handler/image/kitty/transmit.rs` + `place.rs` (post-13.0.5 split) suppress `kitty_create_placement` on `cmd.unicode_placeholder` at `kitty.rs:106-108,161-164`. Image cache stores the image but no cell → image mapping exists yet. §07 explicitly carves placeholder lifecycle OUT at `section-07-image-lifecycle-correctness.md:84`.

**Scope vs §07:** §07 owns non-placeholder (cache-coordinate) image lifecycle — reflow translates `StableRowIndex` through `ReflowMapping::first_output_row`, ED/EL call `remove_placements_in_region`, scrollback eviction calls `prune_scrollback`. Those handlers do NOT touch placeholder cells because placeholder cells are grid cells carrying `U+10EEEE` glyphs — they move with the text under reflow/scroll automatically (the grid-cell data structure is reflow-aware). §13.4 owns:
- The cell-level placeholder glyph lookup at render time: `U+10EEEE` + diacritic row/column encoding → `image_id` → `ImageCache::get` → image bytes.
- The cell-level reflow/ED/EL behavior that asserts placeholder cells continue to resolve to the correct image after a grid mutation.

**Scope vs kitty's reference implementation:** kitty's `U=1` protocol uses U+10EEEE base codepoint with row/column/image-id diacritics encoded as Unicode combining marks (see kitty source `kittens/unicode_input/unicode_placeholder.py` + graphics-protocol docs). The implementation here MUST follow that encoding exactly — deviations are spec violations.

- [ ] Write failing test matrix BEFORE implementation (TDD per `.claude/rules/tests.md` §TDD for Bugs).
- [ ] **Implement placeholder cell encoding** — new module `oriterm_core/src/term/handler/image/kitty/placeholder.rs`:
  - Parser for `U+10EEEE + diacritic` sequences as written into the grid by the client.
  - Lookup: given a grid cell carrying the placeholder, return `(image_id, row_index, col_index)` so the GPU layer can map the cell to the image bytes.
- [ ] **Expose placeholder cells in snapshot** — `RenderableContent::images` carries `image_id`s; add a parallel field (or extend the existing cell metadata) so the snapshot surfaces which cells carry placeholder glyphs. Zero-allocation discipline per `.claude/rules/oriterm_core.md` §Performance Invariants — reuse existing buffers via `.clear()` + capacity retention; no `Vec::new()` per frame.
- [ ] **GPU emit path** — `oriterm/src/gpu/prepare/emit.rs` emits image quads at the placeholder-cell rect with texture source = the stored image. This reuses the existing `emit_image_quads` path used for non-placeholder kitty + sixel.
- [ ] **Reflow pin** — test: place placeholder cells at (row=5, col=10..20), resize columns to trigger reflow, assert the placeholder cells move with the text AND still resolve to the same `image_id`. The grid-cell-based SSOT for location (§07's carve-out at line 84) is the load-bearing invariant.
- [ ] **Scroll pin** — place placeholder cells, scroll the viewport, assert the image follows the cell to its new absolute row.
- [ ] **ED pin (erase display)** — `CSI 2 J` over a region containing placeholder cells: the cells are cleared (standard grid-cell erase); the stored image is NOT automatically evicted from the cache (§07's invariants hold for the image data; only the placeholder cells erase).
- [ ] **EL pin (erase line)** — `CSI K` over a line containing placeholder cells: same as ED at line granularity.
- [ ] **Alt-screen toggle pin** — placeholder cells on primary screen survive an alt-screen toggle + back; placeholder cells on alt-screen are cleared on alt-screen exit. Cross-reference §07's `BUG-08-10`-fixed primary/alt cache routing.
- [ ] **GPU-apex golden pilot** — `kitty_placeholder_basic.rs` drives transmit `a=t,U=1` + write placeholder cells at row=5,col=10..20 via stdout, assert the GPU renders the image at those cells via golden image (0-pixel diff; deterministic lane per §05).
- [ ] **Matrix count assertion** — `placeholder_category_matrix_completeness` asserts 8 categories (encoding, snapshot, emit, reflow, scroll, ED, EL, alt-screen).
- [ ] **Semantic pin** — `placeholder_cells_resolve_to_stored_image_after_reflow`: a test that ONLY passes when the grid-cell-based SSOT is wired (fails if §13.4 stops at cache mutation without the cell-resolve path).
- [ ] **Negative pin** — `placeholder_cell_without_image_renders_as_glyph_not_quad`: a cell carrying a bare `U+10EEEE` without a diacritic encoding MUST render as the glyph, NOT as an image quad. Proves the lookup gates on the full (base + diacritic) pattern, not just the base codepoint.
- [ ] Update catalog rows opened in §13.0 (`KG-UNICODE-PLACEHOLDER-TRANSMIT-U1`, `KG-UNICODE-PLACEHOLDER-PLACE-U1`, `KG-UNICODE-PLACEHOLDER-CELL-RESOLVE`, `KG-UNICODE-PLACEHOLDER-REFLOW`) from `missing` / `stub` to `verified`.
- [ ] Verify all tests pass in both debug AND release builds.
- [ ] **Validation:** placeholder rendering green at GPU-apex golden; reflow/scroll/ED/EL lifecycle green.

---

## 13.5 Verify image protocol replies via Effect transcript apex

**File(s):**
- `oriterm_core/tests/spec_chain/kitty/replies.rs` (new) — `Term`-boundary emission tests: drive kitty commands through the handler and assert `Effect::Pty(PtyEffect::Write { kind: PtyWriteKind::ImageProtocolReply, bytes })` appears on `EffectSink` for every reply code + quiet level.
- `oriterm_mux/src/pane/io_thread/effect_router/tests.rs` (append, sibling `tests.rs` per `.claude/rules/test-organization.md`) — mux-boundary routing tests: construct a `PaneIoThread` harness (or reuse the existing test harness at `oriterm_mux/src/pane/io_thread/effect_router/tests.rs:91` which already calls `drain_effects_into_mux_events()`), push an `Effect::Pty(PtyEffect::Write { kind: ImageProtocolReply, .. })`, assert it flows out as `MuxEvent::PtyWrite { pane_id, data }` via the `Effect::Pty(PtyEffect::Write)` arm.
- `oriterm_mux/src/pane/io_thread/response_poll/tests.rs` (append) — negative mux-boundary test: the `response_poll` register-arms do NOT accept `PtyWriteKind::ImageProtocolReply`.

The crate split is mandated by `.claude/rules/crate-boundaries.md`: `oriterm_mux` owns pane IO-thread routing — the mux-boundary tests MUST live there; `oriterm_core` is standalone with no `oriterm_mux` dep, so a cross-crate test would invert the dependency direction.

**Depends on code paths:** `Term::kitty_respond` at `oriterm_core/src/term/handler/image/kitty.rs:466-479` (post-split: `kitty/response.rs`) pushes `Effect::Pty(PtyEffect::Write { kind: PtyWriteKind::ImageProtocolReply, bytes })` onto `EffectSink`. The reply is routed by `PaneIoThread::drain_effects_into_mux_events` at `oriterm_mux/src/pane/io_thread/effect_router/mod.rs:51`, whose `Effect::Pty(PtyEffect::Write { bytes, .. })` arm at `mod.rs:88` forwards into `MuxEvent::PtyWrite { pane_id, data: bytes }`. **`response_poll` is NOT in this path** — `oriterm_mux/src/pane/io_thread/response_poll/mod.rs:33-54` wraps ONLY `HostRequest::ClipboardLoad` + `HostRequest::ColorQuery`.

- [ ] Write failing test matrix BEFORE implementation (TDD per `.claude/rules/tests.md` §TDD for Bugs).
- [ ] **Per-response-code rung** — for each reply code the kitty protocol defines, a spec_chain test that observes the `EffectSink` transcript:
  - `KG-RESPONSE-OK` — successful transmit/place/query emits `b"\x1b_Gi=<id>;OK\x1b\\"` (quiet q=0); q=1 suppresses OK; q=2 suppresses all.
  - `KG-RESPONSE-EBADF` — path-traversal guard at `kitty.rs:355-359` (post-split: `store.rs`) is currently phrased as `"EINVAL: path traversal not allowed"` — audit file decision in §13.0 captures whether this stays as EINVAL or is reworked to EBADF; test asserts the actual emitted string.
  - `KG-RESPONSE-EBIG` — file exceeds `max_single_image_bytes` at `kitty.rs:366-371` (post-split: `store.rs`) emits `ENOMEM` currently; audit decision captured.
  - `KG-RESPONSE-EINVAL` — malformed payload / unsupported format / shared-memory rejection / missing `s=`/`v=` emits `EINVAL` with specific suffix text.
  - `KG-RESPONSE-ENOENT` — place or frame with unknown image_id at `kitty.rs:150-153,155-157` (post-split: `place.rs` / `frame_compose.rs`) emits `ENOENT`.
  - `KG-RESPONSE-ENOMEM` — cache store overflow at `kitty.rs:305-308,395-397` (post-split: `store.rs`) emits `ENOMEM: <message>`.
  - `KG-RESPONSE-EIO` — `fs::read` failure at `kitty.rs:363-364` (post-split: `store.rs`) emits `EIO: failed to read file: <error>`.
- [ ] **Routing pin** — two-halves assertion across the crate boundary: (a) in `oriterm_core/tests/spec_chain/kitty/replies.rs`, assert `Term::kitty_respond` pushes `Effect::Pty(PtyEffect::Write { kind: PtyWriteKind::ImageProtocolReply, bytes })` onto the test-harness `EffectSink` (Term-side emission); (b) in `oriterm_mux/src/pane/io_thread/effect_router/tests.rs`, drive an `Effect::Pty(PtyEffect::Write { kind: ImageProtocolReply, .. })` through `PaneIoThread::drain_effects_into_mux_events` and assert the corresponding `MuxEvent::PtyWrite { pane_id, data }` lands on the mux event stream via the arm at `effect_router/mod.rs:88`. A regression that rewires the reply through `response_poll` (whether correctly or incorrectly) fails both halves.
- [ ] **Quiet-level gating** — for each reply code above, test with `q=0`, `q=1`, `q=2`. Assert `q=1` suppresses OK replies (current behavior at `kitty.rs:469-471`, post-split: `response.rs`) but still emits error replies. Assert `q=2` suppresses everything.
- [ ] **Bytes-format pin** — assert the exact bytes of each reply match the kitty protocol format: `\x1b_Gi=<id>;<msg>\x1b\\` per `kitty.rs:474` (post-split: `response.rs`). Wezterm reference at `term/src/terminalstate/kitty.rs` confirms this framing.
- [ ] **Matrix count assertion** — `response_code_matrix_completeness` asserts `(7 error codes) × (3 quiet levels)` = 21 cells.
- [ ] **Semantic pin** — `reply_flows_via_effect_pty_write_not_host_request`: asserts the emitted `Effect` is `Pty(PtyEffect::Write { kind: PtyWriteKind::ImageProtocolReply, .. })`, NOT `HostRequest::*`. This pin rejects any future refactor that mis-routes kitty replies through the HostRequest/response_poll path.
- [ ] **Negative pin** — `response_poll_does_not_handle_image_protocol_replies`: a test that enumerates the `register_host_request_response` match arms in `oriterm_mux/src/pane/io_thread/response_poll/mod.rs:33-54` and asserts NO arm accepts `PtyWriteKind::ImageProtocolReply`. If `response_poll` is extended to handle replies in the future, this test forces a conscious decision + plan update.
- [ ] Update catalog rows opened in §13.0 (`KG-RESPONSE-OK`, `KG-RESPONSE-EBADF`, `KG-RESPONSE-EBIG`, `KG-RESPONSE-EINVAL`, `KG-RESPONSE-ENOENT`, `KG-RESPONSE-ENOMEM`, `KG-RESPONSE-EIO`, `KG-RESPONSE-QUIET`) to `verified`.
- [ ] Verify all tests pass in both debug AND release builds.
- [ ] **Validation:** every reply code + quiet level pinned at both the `Term` emission boundary AND the mux routing boundary; response_poll bypass pinned as a negative invariant.

---

## 13.6 Verify kitty + sixel cross-stack mixed-protocol rendering regressions

**File(s):** `oriterm_core/tests/spec_chain/kitty/cross_stack_regression.rs` (new) + GPU-apex pilots at `oriterm/src/gpu/visual_regression/spec_chain/pilots/kitty_sixel_mixed_*.rs` (new).

**Depends on code paths:** §12.5's `oriterm_core/tests/spec_chain/sixel/cross_stack_handoff.rs` (status: complete; catalog row `SIXEL-CROSS-STACK-HANDOFF` verified) proved both protocols' placements coexist in a single `ImageCache` with distinct `image_id`s + `viewport_x`/`viewport_y` + `image_data` payloads at the public-snapshot level. §07's lifecycle handlers (`on_resize`, `remap_placements`, `prune_scrollback`, `remove_placements_in_region`) are the shared lifecycle infrastructure. §13.4's placeholder-cell resolve path is the newly-landed shared surface.

**Scope (what §13.6 owns):** The deep mixed-protocol rendering regressions that §12.5 explicitly DEFERRED-TO-DOWNSTREAM:
- **Overlapping placements** — a sixel image and a kitty image occupying the SAME cell at different `z_index` values (one below text, one above). Assert GPU render produces the correct z-ordered composite.
- **Z-order interleaving** — sixel + kitty + text glyphs all competing for the same cells. `emit_image_quads` at `oriterm/src/gpu/prepare/emit.rs:262-285` (§12.3-verified) splits by `z_index < 0` into `image_quads_below` vs `image_quads_above` for text — verify this split handles mixed-protocol inputs correctly.
- **Shared-eviction races** — LRU eviction under memory pressure affects both protocols' images. Transmit N sixel + N kitty images to exceed the cache's memory limit, assert the eviction policy drops images in arrival order regardless of protocol, assert the remaining placements still resolve to their images.
- **Mixed-protocol placeholder + cache-coordinate coexistence** (post-§13.4) — a kitty U=1 placeholder cell and a sixel cache-coordinate placement in the same viewport render correctly; neither leaks state into the other.

**Scope (what §13.6 does NOT own):**
- **Coexistence at the snapshot level** — already proven by §12.5's `cross_stack_handoff.rs`. §13.6 does NOT re-run that handshake.
- **§07 lifecycle matrix** — non-placeholder image lifecycle is §07's 42-scenario matrix, already complete. §13.6 does NOT re-prove reflow/ED/EL/scroll/alt-screen on cache-coordinate placements.
- **§13.4 placeholder lifecycle** — placeholder-specific reflow/scroll/ED/EL is §13.4's responsibility. §13.6 only tests the CROSS-PROTOCOL case where a placeholder cell and a cache-coordinate placement are both present.

- [ ] Write failing test matrix BEFORE implementation (TDD per `.claude/rules/tests.md` §TDD for Bugs).
- [ ] **Overlapping placements at same cell, different z_index** — place a sixel image at row=5,col=10 with `z=-1` (below text) and a kitty image at row=5,col=10 with `z=1` (above text). Assert GPU render produces sixel-below-text-above-kitty-top. GPU-apex golden at `oriterm/tests/references/kitty_sixel_mixed_z_order.png` (0-pixel diff).
- [ ] **Z-order interleaving with text** — 3-layer composition: sixel at `z=-1`, text glyphs, kitty at `z=1`. Verify via `emit_image_quads` split inspection (unit test at `oriterm/src/gpu/prepare/tests.rs` — extend existing z-split tests to cover mixed-protocol inputs) + golden at `oriterm/tests/references/kitty_sixel_mixed_with_text.png`.
- [ ] **Shared-eviction race** — transmit enough sixel + kitty images to exceed `ImageCache` memory limit. Assert: (a) eviction fires without panic; (b) evicted images are dropped in **true LRU order** regardless of protocol (oldest-accessed first, not oldest-stored first); (c) remaining placements still resolve correctly; (d) no orphan placements point to evicted images (the existing `prune_if_orphaned` path at `oriterm_core/src/image/cache/*.rs` handles this — test proves it covers both protocols). **Load-bearing precondition:** `ImageCache::get()` at `oriterm_core/src/image/cache/mod.rs:354` is currently `#[cfg(test)]`-gated — no production call path bumps `last_accessed`. All production access goes through `get_no_touch()` (see `oriterm_core/src/term/snapshot.rs:286,330` + kitty `place.rs:17,37`, `delete/mod.rs:192`, `frame.rs:31`), so eviction is effectively FIFO on store order, not LRU. §13.6 MUST fix this before sub-bullet (b) can pass: either (i) promote `get()` to non-test + change `renderable_content_into` to `&mut self` (breaks the pure-read snapshot contract at `snapshot.rs:27-30` — requires design review), OR (ii) bump `last_accessed` when a placement is created/used (placement activity IS the access signal for rendering), OR (iii) rename `last_accessed` → `store_time` and document eviction as FIFO (loses the "LRU order" assertion — would force sub-bullet (b) to pin FIFO instead). Pick (ii) as default: bump `last_accessed` in `kitty_create_placement` + sixel equivalent; add `access_on_placement_create_bumps_last_accessed` regression test. <!-- blocked-by:TPR-13.0.5-R2-F3-gemini -->
- [ ] **Rapid alternation stress test** — loop: transmit sixel → transmit kitty → delete sixel → place kitty → etc. for N iterations. Assert no memory leak (RSS before/after bounded per `oriterm_core/tests/rss_regression.rs`) + no panic + final state consistent.
- [ ] **Placeholder + cache-coordinate coexistence** — (depends on §13.4) place a kitty U=1 placeholder spanning row=5,col=10..20 + a sixel cache-coordinate placement at row=10,col=0. Assert both render correctly at the GPU apex. Golden at `oriterm/tests/references/kitty_placeholder_sixel_coexist.png`.
- [ ] **Matrix count assertion** — `cross_stack_regression_category_matrix_completeness` asserts 5 cross-protocol categories.
- [ ] **Semantic pin** — `sixel_and_kitty_z_order_independent_of_transmit_order`: transmit sixel-then-kitty vs kitty-then-sixel at the same z-config; assert both produce the same GPU output (proves z-ordering is driven by `z_index`, not transmit sequence).
- [ ] **Negative pin** — `mixed_protocol_eviction_does_not_cross_pollinate_image_data`: assert a kitty image's eviction does NOT free a sixel image's RGBA bytes (or vice versa) — proves cache eviction is per-`image_id`, not per-protocol.
- [ ] Update catalog rows for cross-stack regression (likely `KG-CROSS-STACK-SIXEL-MIXED-Z-ORDER`, `KG-CROSS-STACK-SIXEL-MIXED-EVICTION`, `KG-CROSS-STACK-SIXEL-PLACEHOLDER-COEXIST` — opened in §13.0) to `verified`.
- [ ] Verify all tests pass in both debug AND release builds.
- [ ] **TPR checkpoint** — `/tpr-review` covering §13.4 through §13.6 (placeholder implementation + reply chain + cross-stack regressions). Findings recorded in §13.R.

---

## 13.R Third Party Review Findings

Populated by `/tpr-review` at the §13.0.5, §13.3, and §13.6 checkpoints and the §13.N final gate. Every unchecked finding here MUST be resolved (fix or file+resolve via `/fix-bug`) before this section can close, per `CLAUDE.md §NEVER reason out of TPR findings`.

### §13.0.5 checkpoint (user-accepted at iter_cap_reached after 3 rounds; 2026-04-21)

All in-scope findings fixed inline with regression tests; 2 out-of-scope findings filed below.

**Round-0 (all in-scope, all fixed inline with regression tests):**
- `[TPR-13.0.5-R0-F1-codex][high]` — `d=p`/`d=c` used origin-cell equality, not span intersection. Fix: new `ImageCache::remove_placements_intersecting_cell` with span-intersection predicate. Test: `delete_p_removes_placement_when_target_cell_is_inside_span` + `delete_c_removes_placement_when_cursor_is_inside_span`.
- `[TPR-13.0.5-R0-F2-codex][high]` — `d=q` used origin-cell equality. Fix: `remove_placements_at_cell_with_z` now uses span-intersection. Test: `delete_q_removes_placement_when_target_cell_is_inside_span_and_z_matches`.
- `[TPR-13.0.5-R0-F3-codex][high]` — `d=n/N` used `last_accessed` (LRU recency) instead of creation order. Fix: new `store_order: HashMap<ImageId, u64>` + monotonic counter; `newest_by_image_number` reads `store_order`. Test: `delete_n_resolves_by_creation_order_not_lru_recency`.
- `[TPR-13.0.5-R0-F4-codex][medium]` — `d=f/F` ignored `I=` fallback. Fix: `kitty_delete_frame` falls back to `newest_by_image_number(image_number)` when `image_id` is absent. Test: `delete_f_uppercase_accepts_image_number_when_image_id_absent`.
- `[TPR-13.0.5-R0-F5-codex][medium]` — frame-index adjustment order (clamp before decrement) double-adjusted on root-frame removal. Fix: pre-decrement first, then size update + clamp. Test: `delete_f_root_frame_leaves_current_frame_pointing_at_same_logical_frame`.

**Round-1 (all in-scope, all fixed inline with regression tests):**
- `[TPR-13.0.5-R1-F1-codex][high]` — delete commands did not abort in-flight chunked upload. Fix: `kitty_delete` clears `self.loading_image` at entry per kitty/graphics.c:2093. Test: `delete_aborts_in_flight_chunked_upload`.
- `[TPR-13.0.5-R1-F2-codex][medium] + [TPR-13.0.5-R1-F1-gemini][critical]` (agreement) — root-frame deletion clobbered displayed frame buffer when current_frame > 0. Fix: `remove_animation_frame` syncs `ImageData.data` to `frames[current_frame]` post-adjustment. Test: `delete_f_root_syncs_image_data_to_surviving_current_frame`.
- `[TPR-13.0.5-R1-F2-gemini][medium]` — `remove_animation_frame` left stale `frame_starts[id]` timer. Fix: clear `frame_starts` on any frame removal. Test: `delete_f_resets_frame_starts_so_animation_timer_reinitializes`.
- `[TPR-13.0.5-R1-F3-codex][low] + [TPR-13.0.5-R1-F4-gemini][low]` (agreement) — zero-span placements counted as intersecting a cell. Fix: `placement_intersects_cell` returns false for `cols==0 || rows==0`. Test: `delete_p_does_not_match_zero_span_placement`.

**Round-2 (all in-scope, all fixed inline with regression tests):**
- `[TPR-13.0.5-R2-F1-codex][medium] + [TPR-13.0.5-R2-F2-gemini][low]` (agreement) — `ImagePlacement::intersects_viewport` had the same zero-span bug. Fix: mirror the `cols==0 || rows==0 → false` guard. Test: `delete_a_does_not_match_zero_height_placement`.
- `[TPR-13.0.5-R2-F1-gemini][critical]` — `remove_image` memory accounting drifted when `current_frame > 0` on an animated image. Fix: use `animation_frames` as SSOT (sum all frame bytes) when present, else `img.data`. Test: `delete_animated_image_after_advance_correctly_releases_memory` (stages frames of unequal sizes; asserts `memory_used == 0` after removal).
- `[TPR-13.0.5-R2-F4-gemini][low]` — `oriterm_core/src/image/cache/mod.rs` at 572 lines exceeded the 500-line hard limit. Fix: extract per-specifier delete-dispatch helpers into `oriterm_core/src/image/cache/deletion.rs` (cache/mod.rs = 409 lines; deletion.rs = 183 lines).

**Outstanding findings (out-of-scope for §13.0.5 delete-specifier closure; triaged via `/verify-tpr` on 2026-04-21 with concrete anchors in §13.3 + §13.6):**

- [x] `[TPR-13.0.5-R1-F3-gemini][medium]` `oriterm_core/src/term/handler/image/kitty/animate.rs:31` — `kitty_animate` ignores `v=0` (infinite loops) because `source_height` is a `u32` that cannot distinguish "absent" from "0". Evidence: `if cmd.source_height > 0 { self.image_cache_mut().set_animation_loops(id, cmd.source_height) }` skips the `v=0` case entirely. Per kitty graphics-protocol.rst §Animation control, `v=0` means infinite loops and must call `set_animation_loops(0)` which in turn maps to `loop_count = None`.
  Resolved: Accepted on 2026-04-21. Validated against code: `source_height: u32` at `oriterm_core/src/image/kitty/parse.rs:28`; `animate.rs:34-37` condition confirmed. Concrete anchor added to §13.3 as new `- [ ]` "Loop count (`v=`)" task with `<!-- blocked-by:TPR-13.0.5-R1-F3-gemini -->` marker, pinning the `Option<u32>` parser change + `animate_v_zero_sets_infinite_loops` positive pin + `animate_v_absent_leaves_loop_count_unchanged` negative pin. §13.3 status is `not-started`; task will execute when §13.3 runs.
- [x] `[TPR-13.0.5-R2-F3-gemini][medium]` `oriterm_core/src/term/snapshot.rs` — `ImageCache` LRU is effectively FIFO because snapshot extraction uses `cache.get_no_touch(id)` in production, never bumping `last_accessed`. Evidence: `get()` (which bumps `access_counter`) is `#[cfg(test)]`-gated; no production call path touches `last_accessed` after `store()`. Recommended fix (one option): replace `get_no_touch` with `get` in the snapshot extraction path and promote `get` to non-test, so rendering DOES update LRU recency. Alternative: rename `last_accessed` to `store_time` to reflect actual semantics.
  Resolved: Accepted on 2026-04-21. Validated against code: `get()` cfg-gated at `oriterm_core/src/image/cache/mod.rs:354`; production call sites (`snapshot.rs:286,330`, kitty `place.rs:17,37`, `delete/mod.rs:192`, `frame.rs:31`) all use `get_no_touch()`; eviction ranks by `last_accessed` at `eviction.rs:30`. Concrete anchor added to §13.6 "Shared-eviction race" bullet with `<!-- blocked-by:TPR-13.0.5-R2-F3-gemini -->` marker — the test asserts **true LRU order** (not FIFO), which forces the fix. Default fix prescribed inline: bump `last_accessed` in `kitty_create_placement` + sixel equivalent (placement activity as access signal) + `access_on_placement_create_bumps_last_accessed` regression pin. §13.6 status is `not-started`; task will execute when §13.6 runs.

### §13.3 checkpoint

Not yet run.

### §13.6 checkpoint

Not yet run.

### §13.N final gate

Not yet run.

---

## 13.N Completion Checklist

- [ ] **§13.0.5 gate**: `plans/bug-tracker/section-08-core-terminal.md` entry for BUG-08-7 is `- [x]` with the fix-commit SHA in its body; BUG-08-8 entry is `- [x]` with the fix-commit SHA in its body. Confirm via `grep -E '^- \[x\] .\[BUG-08-(7|8)\]'` returning both entries.
- [ ] **File-size invariant**: `wc -l oriterm_core/src/term/handler/image/kitty/*.rs` every output line ≤ 500 (the hard limit in `.claude/rules/code-hygiene.md` §File Size). `wc -l oriterm_core/src/term/handler/image/kitty.rs` returns an error (the monolith is gone — the directory module replaced it).
- [ ] Failing test matrix written FIRST (per subsection, flipped `[x]` on entry to each sub-§).
- [ ] **Matrix dimensions**: action × format × transmission × chunked-state × animation-mode × placement-type × reply-status × z-index-layer × protocol-neighbor — covered across the six new test files under `oriterm_core/tests/spec_chain/kitty/` + the GPU pilots under `oriterm/src/gpu/visual_regression/spec_chain/pilots/kitty_*.rs`.
- [ ] **Semantic pins (≥5 — one per invariant)**: (a) unknown `a=` falls back to TransmitAndPlace (or is implemented as Compose per §13.0 decision); (b) malformed-base64 reply path per §13.2's chosen Option A or B; (c) animation advances ONLY when IO-thread timer is wired (§13.3); (d) placeholder cells resolve to stored image after reflow (§13.4); (e) reply flows via `Effect::Pty(PtyEffect::Write)`, NOT via `HostRequest`/`response_poll` (§13.5); (f) sixel + kitty z-order independent of transmit order (§13.6).
- [ ] **Negative pins (≥5)**: (a) `a=c` silent-fallback does NOT route to `Transmit` (§13.1); (b) malformed-base64 does NOT emit the opposite-of-chosen behavior (§13.2); (c) animation frame does NOT advance without timer tick (§13.3); (d) bare `U+10EEEE` without diacritic does NOT render as quad (§13.4); (e) `response_poll` does NOT handle `PtyWriteKind::ImageProtocolReply` (§13.5); (f) mixed-protocol eviction does NOT cross-pollinate image data (§13.6).
- [ ] **Catalog row count**: `grep -c '^| KG-' plans/spec-conformance/catalog/kitty-graphics.md` returns ≥ 40 (§13.0 expansion from 9 coarse rows to fine-grained per-arm rows).
- [ ] **Catalog status invariant**: `grep -cE 'stub|missing|implemented-unverified' plans/spec-conformance/catalog/kitty-graphics.md` returns 0. Every row is `verified`, `verified-with-deviation`, or `not-targeted` (with rationale) — the `not-targeted` count MUST be ≤ `grep -c 'not-targeted' audits/section-13-top-down-inventory.md` (audit file is the source of truth for not-targeted decisions).
- [ ] **Audit file verification**: `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` returns green for `audits/section-13-top-down-inventory.md`. `last_walked` is set to the §13.N-close date; `walked_by` is set.
- [ ] **Animation production wiring**: `grep -r advance_animations oriterm/ oriterm_mux/` returns ≥ 1 production call site (confirms §13.3's timer wiring landed in non-test code).
- [ ] **Reply routing invariant**: `grep -r 'PtyWriteKind::ImageProtocolReply' oriterm_mux/src/pane/io_thread/response_poll/` returns 0 matches (confirms §13.5's negative pin — response_poll does NOT touch image-protocol replies).
- [ ] All existing teseq kitty tests pass (currently zero; baseline for §23.5 archival work).
- [ ] Alloc regression unchanged (`oriterm_core/tests/alloc_regression.rs`) — green via `./test-all.sh`.
- [ ] RSS stability regression green (`oriterm_core/tests/rss_regression.rs` `rss_stability_under_sustained_output`) — per §13.6's rapid-alternation stress test, the mixed-protocol path MUST NOT introduce unbounded growth.
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release (workspace + `x86_64-pc-windows-gnu` cross-compile).
- [ ] Plan annotation cleanup — `plan-annotations.py` scan returns 0 stale annotations.
- [ ] Section frontmatter `status` → `complete`.
- [ ] `00-overview.md` Quick Reference — §13 row flipped `Not Started` → `Complete`. Mission success criteria contributes to **Verification chain complete per row** (every kitty catalog row now `verified` / `verified-with-deviation` / `not-targeted`) + **Image lifecycle correct under resize/reflow/scrollback/alt-screen** (§13.4 placeholder lifecycle + §13.6 cross-stack regressions).
- [ ] `index.md` section 13 status flipped `Not Started` → `Complete`.
- [ ] Next section `depends_on` verification — §14 (iTerm2) and §15 (Cell-Level Alpha) inherit §13's z_index + transparency + animation-timer plumbing. Confirm §14's frontmatter `depends_on:` references §13 if iTerm2's OSC 1337 animation path shares the timer.
- [ ] `/tpr-review` passed (final, full-section).
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean).

**Exit Criteria:** Every kitty graphics catalog row (post-§13.0 expansion) is `verified` / `verified-with-deviation` / `not-targeted`; U=1 placeholder rendering implemented end-to-end; animation timer wired into production IO thread; mixed-protocol regression matrix green; BUG-08-7 and BUG-08-8 closed.
