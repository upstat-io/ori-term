---
section: "12"
title: "Sixel"
canonical_spec_sources:
  - "DEC STD 070 §5 — Sixel Color Extension (primary; defines `#` color operator with Pu=1 HLS / Pu=2 RGB definition and bare color selection)"
  - "DEC STD 070 §6 — Sixel Graphics Extension (primary; defines DCS q introducer, P1/P2/P3 parameters, raster attributes `\"`, repeat `!`, CR `$`, NL `-`, sixel data byte `?`..`~`, DCS abort via CAN/SUB/ESC)"
  - "libsixel src/decoder.c — reference implementation cross-reference for parsing edge cases (palette reset, repeat clamping, abort path)"
  - "wezterm term/src/terminalstate/sixel.rs — production cross-reference for HLS rotation (`hue - 120.0`), raster attrs, transparency compositing"
last_walked: 2026-04-20
walked_by: "elucidsoft"
---

# Top-Down Spec Audit — Section 12: Sixel

## Canonical spec source(s)

DEC STD 070 §5 and §6 are the authoritative top-down enumerators for sixel coverage. §5 defines the color extension (the `#` operator with color-definition sub-forms for Pu=1 HLS and Pu=2 RGB plus bare color selection) and §6 defines the sixel graphics extension (the DCS envelope, the P1/P2/P3 parameters, the raster-attribute `"` operator, the repeat `!` operator, the graphic CR `$` and NL `-` operators, the sixel data-byte encoding `?`..`~`, and the DCS-abort semantics for CAN/SUB/ESC-mid-DCS). Every sequence the spec defines maps to either a catalog row or an explicit `not-targeted` decision. libsixel `src/decoder.c` and wezterm `term/src/terminalstate/sixel.rs` are secondary cross-references used to tie-break ambiguities (HLS rotation sign, repeat clamping behavior, raster-attrs-mid-stream behavior).

The enclosing DCS envelope (`DCS`, `ST`) is an ECMA-48 §5.4 / §5.5 construct; rows for the envelope itself live in `catalog/ecma-48.md`. This audit treats the DCS envelope as the harness for sixel-only operators and enumerates the DCS-abort paths as behavioral rows because abort semantics are what couple the sixel parser to the VTE performer seam.

Two private-mode rows (`CSI ? 80 h/l` sixel-scrolling, `CSI ? 8452 h/l` sixel-cursor-right) cross into this audit because they gate sixel cursor-positioning behavior downstream of DCS-unhook. Their canonical rows live in `catalog/dec-private-modes.md`; this audit carries cross-reference rows that name the catalog IDs sixel depends on.

## Sequence-to-catalog mapping

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| `DCS Ps1 ; Ps2 ; Ps3 q <data> ST` (introducer — envelope + P1/P2/P3) | DEC STD 070 §6.2 | `SIXEL-DCS-q` | mapped |
| `P1` parameter — aspect ratio / Pan-Pad (values 0–9) | DEC STD 070 §6.2.1 | `SIXEL-P1-ASPECT` | mapped |
| `P2` parameter — background-select value `0` (device default) | DEC STD 070 §6.2.2 | `SIXEL-BG-DeviceDefault` | mapped |
| `P2` parameter — background-select value `1` (no change / transparent) | DEC STD 070 §6.2.2 | `SIXEL-BG-NoChange` | mapped |
| `P2` parameter — background-select value `2` (set-to-background) | DEC STD 070 §6.2.2 | `SIXEL-BG-SetToBg` | mapped |
| `P3` parameter — horizontal grid size | DEC STD 070 §6.2.3 | `SIXEL-P3-HGRID` | mapped (verified-with-deviation — ori_term ignores P3 per impl at `oriterm_core/src/image/sixel/mod.rs:80`; documented divergence) |
| `"Pan;Pad;Ph;Pv` — raster attributes operator | DEC STD 070 §6.3.1 | `SIXEL-RASTER-ATTRS` | mapped |
| `Pan`/`Pad` raster attribute sub-params — aspect-ratio numerator/denominator | DEC STD 070 §6.3.1 | `SIXEL-RASTER-ATTRS-PAN-PAD` | mapped (verified-with-deviation — Pan/Pad read but unused at `oriterm_core/src/image/sixel/mod.rs:311-330`; aspect ratio not applied to output buffer) |
| `Ph`/`Pv` raster attribute sub-params — horizontal/vertical extent | DEC STD 070 §6.3.1 | `SIXEL-RASTER-ATTRS-PH-PV` | mapped |
| `#Pc;Pu;Px;Py;Pz` — color definition operator, Pu=2 (RGB, 0–100) | DEC STD 070 §5.1 | `SIXEL-COLOR-DEFINE-RGB` | mapped |
| `#Pc;Pu;Px;Py;Pz` — color definition operator, Pu=1 (HLS, H=0–360, L=0–100, S=0–100) | DEC STD 070 §5.1 | `SIXEL-COLOR-DEFINE-HLS` | mapped |
| `#Pc` — bare color selection operator (no Pu / no RGB triple) | DEC STD 070 §5.2 | `SIXEL-COLOR-SELECT` | mapped |
| `!Pn <data>` — graphic repeat operator (repeat next sixel data byte Pn times) | DEC STD 070 §6.3.2 | `SIXEL-REPEAT` | mapped |
| `$` — graphic carriage return (reset x, keep y) | DEC STD 070 §6.3.3 | `SIXEL-CR` | mapped |
| `-` — graphic newline (reset x, advance y by 6) | DEC STD 070 §6.3.4 | `SIXEL-NL` | mapped |
| `?`..`~` — sixel data byte (0x3F..0x7E, 6-bit pixel column encoding via `byte - 0x3F`) | DEC STD 070 §6.3.5 | `SIXEL-DATA-BYTE` | mapped |
| `CAN` (0x18) mid-DCS — abort | DEC STD 070 §6.4 (referencing ECMA-48 §8.2.1) | `SIXEL-ABORT-CAN` | mapped |
| `SUB` (0x1A) mid-DCS — abort | DEC STD 070 §6.4 (referencing ECMA-48 §8.2.10) | `SIXEL-ABORT-SUB` | mapped |
| `ESC` (0x1B) mid-DCS — abort (non-ST-terminated) | DEC STD 070 §6.4 | `SIXEL-ABORT-ESC` | mapped |
| `"` raster-attrs-before-data behavior pin | DEC STD 070 §6.3.1 | `SIXEL-RASTER-BEFORE-DATA` | mapped |
| `"` raster-attrs-mid-stream behavior pin | DEC STD 070 §6.3.1 | `SIXEL-RASTER-MID-STREAM` | mapped (verified-with-deviation — ori_term treats mid-stream `"` as a re-dimension rather than ignoring it; documented divergence at `oriterm_core/src/image/sixel/mod.rs:311-330`) |
| Palette-reset-per-DCS-q invariant (every DCS q rebuilds VT340 palette) | DEC STD 070 §5.1 (implicit — each DCS is a fresh sixel session) | `SIXEL-PALETTE-RESET-PER-DCS` | mapped |
| `!` repeat clamping at `MAX_DIMENSION` (10,000 pixels) | libsixel `src/decoder.c` — no explicit clamp in DEC STD 070 | `SIXEL-REPEAT-CLAMP` | mapped (verified-with-deviation — ori_term clamps at 10,000 to prevent OOM; de-facto compatible with libsixel's protective clamp) |
| `"` raster-attrs pixel-buffer cap (MAX_PIXEL_BYTES = 100 MB) | ori_term impl at `oriterm_core/src/image/sixel/mod.rs:20` — no DEC STD 070 equivalent | `SIXEL-PIXEL-BUFFER-CAP` | mapped (verified-with-deviation — ori_term caps buffer at 100 MB to prevent DoS; aborts via `ImageError::OversizedImage`) |
| `CSI ? 80 h/l` — DECSDM / sixel-scrolling mode | xterm ctlseqs §DECSET 80 | `DEC-SIXEL-SCROLLING` (cross-ref to `catalog/dec-private-modes.md`) | mapped |
| `CSI ? 8452 h/l` — sixel-cursor-right mode | xterm ctlseqs §DECSET 8452 | `DEC-SIXEL-CURSOR-RIGHT` (cross-ref to `catalog/dec-private-modes.md`) | mapped |
| Sixel placement `z_index: 0` (above text) — occlusion contract with §11 unicode subcell | ori_term impl at `oriterm_core/src/term/handler/image/sixel.rs:68-139` + `oriterm/src/gpu/prepare/emit.rs:262-285`; §11 is the consumer side of the contract | `SIXEL-Z-ORDER` | mapped |
| Sixel ↔ Kitty shared `ImageCache` hand-off (public snapshot surface) | ori_term impl at `oriterm_core/src/term/snapshot.rs:33,79`; contract consumed by §13 Kitty + §14 iTerm2 | `SIXEL-CROSS-STACK-HANDOFF` | mapped |
| `DCS Pid ; Ptrm ; Ptid Pfinal <data> ST` — DECDMAC (macro-set definition) | DEC STD 070 §7.1 | — | not-targeted: ori_term does not implement DEC macro storage; the VTE parser routes non-`q`-final DCS sequences through their own handler paths and the sixel parser never observes these bytes. If the user attempts DECDMAC, the DCS lands in the generic DCS handler and is silently dropped (not sent to `SixelParser`). Revisitable if a consumer surfaces — no real-world sixel producer relies on macro storage. |
| `DCS Pid Pfinal` — DECRQVSS (request visual setup) and other non-sixel DCS sequences | DEC STD 070 §7.2 + ECMA-48 §5.4 | — | not-targeted: same rationale as DECDMAC — the VTE parser dispatches DCS by final character; `q` routes to sixel, everything else routes elsewhere or drops. This audit's concern is that non-sixel DCS bytes do not corrupt sixel state; that property is implicit in the parser's DCS-final-character dispatch and does not need a catalog row of its own. |
| `DCS Ps $ r … ST` — DECGRA (graphic reproduce) | DEC STD 070 §7.3 | — | not-targeted: DECGRA is a query/response for graphic-region state, not a render operator. ori_term does not implement the reply path; the VTE dispatcher drops the sequence. Revisitable only if a real-app capture surfaces. |
| `DCS $ q` / `DCS $ p` / `DCS ! { … ST` — DECRSPS (restore presentation state), DECRQSS (request selection/setting), DECDLD (download soft font) | DEC STD 070 §8 (presentation state) + §9 (soft fonts) | — | not-targeted: these are presentation-state / soft-font DCS sequences owned by Section 09B (dec-presentation) per `catalog/dec-presentation.md`. They share the DCS envelope but not the sixel final character (`q` alone is sixel; `$ q` / `$ p` / `! {` are non-sixel finals). The audit row set for these lives in Section 09B's audit file, not here. |
| DECGNL / DECGOFC / other DEC STD 070 §10+ print-related sequences | DEC STD 070 §10 (print extension) | — | not-targeted: print extension is owned by Section 20 (audio + print) per `catalog/audio-print.md`. |

## Decisions

Every `not-targeted` row above documents its rationale inline. Summary of exclusion categories:

- **Non-sixel DCS sequences** (DECDMAC, DECRQVSS, DECGRA, DECRSPS, DECRQSS, DECDLD): these share the DCS envelope with sixel but not the final character `q`. The VTE parser dispatches by final character, so they never reach `SixelParser`. The property the sixel audit actually needs — that non-sixel DCS bytes do not corrupt sixel state — is automatically enforced by the parser's dispatch-by-final-character architecture and is covered by tests in those sequences' owning sections (09B for dec-presentation, 20 for print) rather than duplicated here.

- **DEC macro-set (DECDMAC / DECINVM)**: ori_term does not implement macro storage. The VTE parser silently drops macro-set DCS sequences. Revisitable only if a real-world consumer surfaces that requires macro storage; no sixel producer depends on macros.

- **P3 parameter divergence**: ori_term reads P1 and P2 from the DCS introducer but ignores P3 (horizontal grid size) per `oriterm_core/src/image/sixel/mod.rs:80`. This is `mapped` with `verified-with-deviation` rather than `not-targeted` because P3 is part of the DCS q introducer row contract — the row exists, the implementation diverges, and the catalog row documents the divergence with a Notes cross-reference to this audit.

- **Pan/Pad raster-attrs divergence**: ori_term reads Pan/Pad from the `"` operator but does not apply the aspect ratio to the output buffer. Mapped as `verified-with-deviation` for the same reason as P3 — the row exists, implementation diverges, divergence documented.

- **Raster-attrs-mid-stream divergence**: DEC STD 070 §6.3.1 is ambiguous on whether `"` after data re-dimensions the image or is ignored. libsixel ignores mid-stream `"`; ori_term treats it as a re-dimension because `apply_raster_attrs` unconditionally writes `raster_width`/`raster_height`. Mapped as `verified-with-deviation`; catalog Notes cross-reference this decision.

- **Cross-stack rows** (`SIXEL-Z-ORDER`, `SIXEL-CROSS-STACK-HANDOFF`): these rows are in the sixel catalog because sixel is the FIRST section that drives them through the spec_chain apex. §11 (unicode subcell) and §13 (kitty graphics) consume these contracts. Alternative placement would be a "cross-stack" catalog file, but §04.7's frozen schema did not create one — the contracts live in the section that first establishes them, with cross-reference Notes in the consumer catalog files.

## Verification

- [x] Every row with `Decision: mapped` resolves to a real catalog row that exists in `plans/spec-conformance/catalog/sixel.md` (or cross-referenced to `catalog/dec-private-modes.md` for `DEC-SIXEL-SCROLLING` and `DEC-SIXEL-CURSOR-RIGHT`) with the cited row ID.
- [x] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes with this audit file in scope. (Verified at §12.0 close-out.)
- [x] No sequence defined in DEC STD 070 §5 or §6 is missing from this table — all operators (DCS q with P1/P2/P3, `"` raster attrs with Pan/Pad/Ph/Pv, `#` color define RGB/HLS + bare select, `!` repeat, `$` CR, `-` NL, `?`..`~` data byte, CAN/SUB/ESC-mid-DCS abort) are enumerated plus the behavioral invariants (palette reset per DCS q, repeat clamp, pixel-buffer cap, raster-before-data vs raster-mid-stream, background-mode distinctions, z-order, cross-stack hand-off) and the cross-reference private modes.
- [x] `last_walked` date is set (2026-04-20); `walked_by` is set (elucidsoft).
