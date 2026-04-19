---
section: "20"
title: "Audio + Print"
status: not-started
reviewed: false
goal: "Drive every catalog row in `catalog/audio-print.md` from `implemented-unverified` to `verified`. IMPLEMENT the missing audio handlers (ANSI music CSI M, DECPS, visual bell DECVB) and print handlers (CSI i print screen, auto print mode, file transfer detection). Each audio effect emits via `Effect::Host(HostEffect::AudioRequest { kind, params })` for fire-and-forget host audio."
success_criteria:
  - "Top-down spec audit committed at `plans/spec-conformance/audits/section-20-top-down-inventory.md`. Every sequence in the canonical spec source(s) for this stack (multiple sub-stacks — DEC tech manual §DECPS for audio, MS-DOS ANSI.SYS reference §CSI M for ANSI music, ECMA-48 §8.3.7 BEL + §8.3.91 CSI i for printer functions) maps to a catalog row ID OR carries an explicit `not-targeted` decision with rationale. `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes for this audit file. This is enforced PER `plans/spec-conformance/audits/README.md` lint contract — added by Section 09A as the SSOT for top-down catalog coverage to prevent the bottom-up gap that hid DECRQCRA from the catalog."
  - "Every row in `catalog/audio-print.md` is `verified`"
  - "**BEL** (already implemented at handler/mod.rs:135 per Pass 1) verified — emits `Effect::Host(HostEffect::Bell)`"
  - "**ANSI music CSI M** implemented: parses MML-like notation, emits `Effect::Host(HostEffect::AudioRequest { kind: AudioKind::AnsiMusic, params: AudioParams::AnsiMml { notes } })`. Currently MISSING per Pass 1."
  - "**DECPS** (DEC play sound) implemented: ESC [ Vol Note Tones p, emits `Effect::Host(HostEffect::AudioRequest { kind: AudioKind::Decps, params: AudioParams::Decps { volume, note, duration } })`. Currently MISSING per Pass 1."
  - "**Visual bell DECVB** verified: emits the separate `HostEffect::VisualBell` variant (NOT a flag on `HostEffect::Bell`) — the variant is defined up-front in Section 03.1 and Section 20.1 wires the emission site (see cross-section coupling note in `00-overview.md`)"
  - "**Print screen CSI i** implemented: emits `Effect::Host(HostEffect::PrintRequest { kind: PrintKind::Screen, params: PrintParams { content: snapshot } })`"
  - "**Auto print mode** verified: terminal copies output to print buffer when mode is enabled; emits accumulated print buffer via `Effect::Host(HostEffect::PrintRequest { kind: AutoPrint, .. })` on flush trigger"
  - "**Print form / extent** verified: page-formatting controls"
  - "**File transfer protocol detection (Zmodem, Kermit) passthrough** verified: terminal recognizes the protocol introducer bytes and switches to passthrough mode (or marks the bytes as opaque); emits `Effect::Pty(PtyEffect::Write)` or similar to forward to the host file-transfer handler"
  - "Cross-platform: audio effects deliver to platform-specific audio adapters per the host integration design (macOS NSSound, Linux libpulse / aplay, Windows MessageBeep). The audio adapter contract is documented in `de-facto-behaviors.md`."
  - "All existing tests pass without modification"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Verification chain complete per row**"
inspired_by:
  - "DEC technical manual — DECPS reference"
  - "MS-DOS ANSI.SYS reference — CSI M ANSI music notation"
  - "ECMA-48 — BEL semantics, DECVB visual bell, CSI i print"
  - "Zmodem / Kermit protocol references for passthrough byte sequences"
depends_on: ["03", "08"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "20.0"
    title: "Top-down spec audit (BLOCKING)"
    status: not-started
  - id: "20.1"
    title: "Verify BEL + visual bell DECVB"
    status: not-started
  - id: "20.2"
    title: "Implement ANSI music CSI M"
    status: not-started
  - id: "20.3"
    title: "Implement DECPS DEC play sound"
    status: not-started
  - id: "20.4"
    title: "Implement print screen + auto print + print form/extent"
    status: not-started
  - id: "20.5"
    title: "Implement file transfer protocol passthrough detection"
    status: not-started
  - id: "20.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "20.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 20: Audio + Print

**Status:** Not Started
**Goal:** Verify every audio + print catalog row, implementing the missing handlers (ANSI music, DECPS, print screen, file transfer detection).

**Success Criteria:** see frontmatter.

**Context:** Pass 1 confirmed BEL is implemented. ANSI music (CSI M), DECPS (DEC play sound), and several print/file-transfer handlers are MISSING. The audio effects emit via `Effect::Host(HostEffect::AudioRequest { ... })` from section 03's effect type. The host adapter (oriterm) routes audio requests to the platform-specific audio backend.

**Reference implementations:** see frontmatter.

**Depends on:** Section 08 (baseline correct).

---

## 20.0 Top-down spec audit (BLOCKING — precedes all other subsections)

**Goal:** Walk the canonical spec source(s) for this stack TOP-DOWN. Every sequence the spec defines gets a row in this section's audit file at `plans/spec-conformance/audits/section-20-top-down-inventory.md`, mapped to either an existing catalog row ID or an explicit `not-targeted` decision with rationale.

**Why this exists:** Section 09A introduced the `audits/` SSOT to close the bottom-up catalog construction gap that hid DECRQCRA (and the entire DEC private rectangular-ops family) from the catalog. The original Section 01 catalog bootstrap was bottom-up (audit existing dispatch + add tack/teseq-discovered items), which is incomplete by construction — sequences absent from both the catalog AND the test corpus are invisible. The per-section audit file makes top-down coverage mechanically lintable: `spec-coverage-report --check audit-files` fails CI if any audit-file mapping does not resolve to a real catalog row.

**Canonical spec source(s):** Three sub-stacks — DEC technical manual §DECPS (audio: volume/note/duration parameters); MS-DOS ANSI.SYS reference §CSI M (ANSI music MML-like notation); ECMA-48 §8.3.7 BEL + §8.3.91 CSI i (BEL semantics + printer functions). The audit stub pre-populates a separate sub-table per sub-stack.

**Files touched:**
- `plans/spec-conformance/audits/section-20-top-down-inventory.md` (NEW — stub created by Section 09A's §09A.10; populated by this subsection)
- `plans/spec-conformance/catalog/audio-print.md` (open new rows for any audio/print sequences that should be `mapped` but aren't catalogued yet — use the canonical schema per `plans/spec-conformance/00-overview.md §Catalog Row Schema`)

**Completion criteria:**

- [ ] Audit file `plans/spec-conformance/audits/section-20-top-down-inventory.md` is populated with every sequence in the canonical spec source(s) across all three sub-stacks (audio, ANSI music, print).
- [ ] Every row in the audit-file table has a `Decision` of `mapped` (cites a catalog row ID) or `not-targeted` (with one-line rationale).
- [ ] Every `mapped` row resolves to a real catalog row that exists in `plans/spec-conformance/catalog/`.
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes for this audit file.
- [ ] Audit file `last_walked` frontmatter is set to today's date and `walked_by` to the implementer's handle.
- [ ] Any new catalog rows opened in this subsection use the canonical 10-column schema from `plans/spec-conformance/00-overview.md §Catalog Row Schema`.

**No other subsection in this section can begin work until §20.0 is complete.** This is a hard gate.

---

## 20.1 Verify BEL + visual bell DECVB

**File(s):** `oriterm_core/tests/spec_chain/audio_print/bell.rs` (new), `oriterm_core/src/effect/families/host.rs` (updated)

**Design decision (resolved, not deferred)**: visual bell is a separate variant — `HostEffect::VisualBell` — NOT a flag on `HostEffect::Bell`. Rationale: audible and visual bells have different host-side consumers (audio adapter vs UI flash animator), and conflating them into a single variant forces every consumer to check the flag to decide whether to act. Separate variants are routed separately, cleaner dispatch, easier testing.

- [ ] Extend `HostEffect` in `oriterm_core/src/effect/families/host.rs` (from section 03) to add `VisualBell` as a separate variant alongside `Bell`
- [ ] Update `handler/mod.rs` BEL handler: when mode 12 (DECVB) is enabled, emit `HostEffect::VisualBell`; otherwise emit `HostEffect::Bell`
- [ ] Spec_chain test for BEL: feed `\x07` (mode 12 disabled), assert `Effect::Host(HostEffect::Bell)` observed
- [ ] Spec_chain test for DECVB: enable mode 12, feed `\x07`, assert `Effect::Host(HostEffect::VisualBell)` emitted
- [ ] Document the variant split decision in `de-facto-behaviors.md` (one-line entry citing this section)
- [ ] Update catalog rows for BEL and DECVB to `verified`

---

## 20.2 Implement ANSI music CSI M

**File(s):** `crates/vte/src/ansi/dispatch/csi.rs`, `oriterm_core/src/term/handler/audio.rs` (new), sibling tests

- [ ] Add CSI M handler to dispatch (currently MISSING — Pass 1 found no handler)
- [ ] Read MS-DOS ANSI.SYS reference for CSI M music notation (it's a sub-language: notes encoded as `M` + sequences like `o4l4cdefgab` for octave/length/notes)
- [ ] Implement a parser for the music notation
- [ ] Emit `Effect::Host(HostEffect::AudioRequest { kind: AudioKind::AnsiMusic, params: AudioParams::AnsiMml { notes: parsed_notes } })`
- [ ] Spec_chain tests covering basic note sequences, octave changes, length changes, rests
- [ ] Update catalog row to `verified`

---

## 20.3 Implement DECPS DEC play sound

**File(s):** `crates/vte/src/ansi/dispatch/csi.rs`, `oriterm_core/src/term/handler/audio.rs`, sibling tests

- [ ] Add DECPS handler: ESC [ Vol Note Tones p — Vol is volume (0-7), Note is the note (1-25, where 1=silence and 2-25 are MIDI notes), Tones is duration in 1/32 second
- [ ] Reference: DEC technical manual for DECPS semantics
- [ ] Emit `Effect::Host(HostEffect::AudioRequest { kind: AudioKind::Decps, params: AudioParams::Decps { volume, note, duration } })`
- [ ] Spec_chain tests
- [ ] Update catalog row

---

## 20.4 Implement print screen + auto print + print form/extent

**File(s):** `crates/vte/src/ansi/dispatch/csi.rs`, `oriterm_core/src/term/handler/print.rs` (new), sibling tests

- [ ] Add CSI i (print screen) handler: emit `Effect::Host(HostEffect::PrintRequest { kind: PrintKind::Screen, params: PrintParams { content: current_grid_snapshot } })`
- [ ] Add auto print mode handler: when enabled, terminal accumulates output to a print buffer; on flush trigger (mode disabled or explicit flush sequence), emit `Effect::Host(HostEffect::PrintRequest { kind: AutoPrint, .. })`
- [ ] Add print form / extent handlers (page formatting controls)
- [ ] Spec_chain tests
- [ ] Update catalog rows

---

## 20.5 Implement file transfer protocol passthrough detection

**File(s):** `oriterm_core/src/term/handler/file_transfer.rs` (new), sibling tests

- [ ] Recognize Zmodem / Kermit protocol introducer byte sequences (`**\x18B00`/`**\x18B01` for Zmodem, `\x01` start of header for Kermit)
- [ ] On detection, switch to passthrough mode: subsequent bytes are forwarded via `Effect::Pty(PtyEffect::Write)` or marked opaque so the host file-transfer handler can intercept
- [ ] Spec_chain tests for both protocols
- [ ] Update catalog rows

---

## 20.R Third Party Review Findings

- None.

---

## 20.N Completion Checklist

- [ ] Failing test matrix written FIRST
- [ ] **Matrix dimensions**: audio sequence (BEL/DECVB/CSI M/DECPS) × print sequence (CSI i/auto print/form/extent) × file transfer protocol (Zmodem/Kermit)
- [ ] **Semantic pin**: each new audio/print handler has a regression-guarding test
- [ ] BEL + DECVB verified
- [ ] ANSI music CSI M implemented
- [ ] DECPS implemented
- [ ] Print screen + auto print + form/extent implemented
- [ ] File transfer passthrough implemented
- [ ] All existing tests pass
- [ ] Cross-platform audio adapter contract documented
- [ ] Alloc regression unchanged
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` + `index.md` updated
- [ ] `/tpr-review` passed
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** Every audio + print catalog row is `verified`.
