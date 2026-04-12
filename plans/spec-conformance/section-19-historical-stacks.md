---
section: "19"
title: "Historical Legacy Control Stacks (VT52, DEC LK201, Wyse 50/60, ADM-3A, IBM PC ANSI.SYS, Microsoft Console VT)"
status: not-started
reviewed: false
goal: "Drive every LEGACY CONTROL catalog row in `catalog/historical.md` to `verified` by IMPLEMENTING every legacy control stack in scope (VT52, DEC LK201 keyboard, Wyse 50/60, ADM-3A, IBM PC ANSI.SYS, Microsoft Console VT). Vector stacks (ReGIS, Tek 4014) are delivered by Section 26 — they depend on Section 05 (deterministic golden lane) and Section 07 (image lifecycle) in addition to Section 08, and are split out to keep the dependency graph clean."
success_criteria:
  - "Every legacy-control row in `catalog/historical.md` is `verified` (NOT `verified-with-deviation` — see goal statement). `verified-with-deviation` is reserved for rows where ori_term intentionally deviates from an ambiguous spec, not for rows where the implementation was skipped. Vector stack rows are owned by Section 26."
  - "**VT52 mode** verified: ESC `<` enters VT52, ESC A/B/C/D arrow keys, ESC F/G enter/exit graphics, ESC H home, ESC I reverse linefeed, ESC J/K erase, ESC Y row;col cursor positioning, ESC Z device attribute reply"
  - "**DEC LK201 keyboard protocol** verified: the LK201 keyboard emits a defined set of scan codes + key reports that the VT220/320/420/520 terminals expected. Section 17 covers modern keyboard encoding (kitty/modifyOtherKeys/Win32); this section covers the LK201 historical encoding that DEC-compatible apps still emit queries for. Verify the LK201 response bytes for the DA2 (secondary device attribute) reply identify ori_term as LK201-compatible, and verify the LK201 key codes that the authority-ladder DEC technical manual documents."
  - "**Wyse 50/60** IMPLEMENTED: protocol extensions (attribute byte ESC G, protected mode, status line ESC F, function key programming) — every Wyse 50/60 manual sequence has a handler and a spec_chain test"
  - "**ADM-3A** verified: basic control codes only (already covered by ECMA-48 C0 + the ESC = cursor addressing sequence); add catalog rows pointing to the existing handlers AND implement the ADM-3A-specific ESC = row+32,col+32 cursor addressing if not already present"
  - "**IBM PC ANSI.SYS** verified: every PC ANSI extension implemented — extended cursor codes, color codes (most overlap with ECMA-48 SGR), KEYBOARD REASSIGNMENT (CSI...p reassign key, a PC-specific extension not in ECMA-48), SAVE/RESTORE cursor. Catalog rows are `verified`, not deviation."
  - "**Microsoft Console VT** verified: every Microsoft-documented extension implemented. The Microsoft Console VT spec is mostly xterm-compatible but documents the exact subset Microsoft promises — every promise is tested."
  - "All existing tests pass without modification"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Verification chain complete per row**"
inspired_by:
  - "DEC user manuals (VT52, VT100, etc.) — committed under `plans/spec-conformance/specs/vt-series-manuals/` if license permits"
  - "Wyse 50 user manual"
  - "Microsoft Console Virtual Terminal Sequences spec — Microsoft docs"
  - "Free MS-DOS ANSI.SYS reference"
depends_on: ["08"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "19.1"
    title: "Implement VT52 mode (ESC < entry, navigation, graphics, device reply)"
    status: not-started
  - id: "19.2"
    title: "Implement DEC LK201 keyboard protocol responses"
    status: not-started
  - id: "19.3"
    title: "Implement Wyse 50/60 protocol extensions (attribute byte, protected mode, status line, key programming)"
    status: not-started
  - id: "19.4"
    title: "Implement ADM-3A ESC = cursor addressing + catalog rows for C0 overlap"
    status: not-started
  - id: "19.5"
    title: "Implement IBM PC ANSI.SYS extensions (keyboard reassignment CSI...p, save/restore cursor variants)"
    status: not-started
  - id: "19.6"
    title: "Verify Microsoft Console VT extensions against the Microsoft spec"
    status: not-started
  - id: "19.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "19.N"
    title: "Completion Checklist"
    status: not-started
# TPR Checkpoint Placement: 19.2 (after VT52 + LK201 — covers .1-.2),
# 19.5 (after Wyse + ADM-3A + IBM PC — covers .3-.5), final in 19.N
---

# Section 19: Historical Legacy Control Stacks

**Status:** Not Started
**Goal:** Implement and verify every legacy-control historical-stack catalog row. The maximalist mission says "every published terminal protocol spec — historical, modern, de-facto, obscure" — that is not compatible with deferring any stack. VT52, DEC LK201 keyboard protocol, Wyse 50/60, ADM-3A, IBM PC ANSI.SYS, and Microsoft Console VT all get real implementations in this section. Vector stacks (ReGIS, Tek 4014) and their shared rasterizer are owned by **Section 26** because they depend on Section 05 (deterministic golden lane for rasterizer goldens) and Section 07 (image lifecycle for the ImageCache placements they emit) in addition to Section 08.

**Success Criteria:** see frontmatter.

**Context:** VT100/102/220/320/420/520 are largely subsumed by the ECMA-48 baseline (section 08) + DEC private modes (section 09) — sections 08+09 already cover the bulk of DEC heritage. This section adds the genuinely separate legacy-control stacks: VT52 mode, DEC LK201 keyboard protocol historical responses, Wyse 50/60 extensions, ADM-3A primitives, IBM PC ANSI.SYS extensions, and Microsoft Console VT.

**No verified-with-deviation escape hatch.** Per the cohesion pass, this section does not contain a decision fork between implement and defer. Every legacy control stack gets implemented. `verified-with-deviation` is reserved for rows where ori_term intentionally deviates from an ambiguous spec (cited reason in the catalog row notes), not for rows where the implementation was skipped.

**Reference implementations:** see frontmatter. Additional for the new legacy control stacks:
- **Wyse WY-50/WY-60 Reference Manual** — attribute byte encoding, protected mode, status line ESC F, function key programming

**Depends on:** Section 08 (baseline solid).

---

## 19.1 Implement VT52 mode

**File(s):** `oriterm_core/src/term/handler/vt52.rs` (new), `oriterm_core/tests/spec_chain/historical/vt52.rs` (new), sibling tests

VT52 is a separate emulation mode entered via `ESC <` (exits ANSI mode) and exited via `ESC <` in VT52 mode (returns to ANSI). While in VT52 mode, the terminal interprets a smaller set of escape sequences (ESC A/B/C/D arrows, ESC H home, ESC J/K erase, ESC Y row;col position, ESC Z device attribute request, ESC F/G graphics enter/exit, ESC I reverse index).

- [ ] Add a `vt52_mode: bool` flag to `Term` (or to the mode bit set, wherever terminal mode flags live)
- [ ] Extend the VTE dispatch path to route escape sequences differently when `vt52_mode` is true — the ESC introducer in VT52 mode recognizes a different set of final bytes (`A-K`, `Y`, `Z`) rather than the ANSI CSI/DCS forms
- [ ] Implement each VT52 sequence handler in `vt52.rs`
- [ ] Spec_chain tests for each VT52 sequence: ESC A/B/C/D (cursor), ESC H (home), ESC I (reverse index), ESC J (erase to end of screen), ESC K (erase to end of line), ESC Y row;col (direct cursor), ESC Z (identify reply `ESC / Z`), ESC F (enter graphics), ESC G (exit graphics), ESC = (enter alternate keypad), ESC > (exit alternate keypad), ESC < (enter ANSI mode)
- [ ] Test VT52 ↔ ANSI mode toggling: `ESC [?2l` sets VT52 mode; `ESC <` exits VT52 back to ANSI
- [ ] Update `catalog/historical.md` VT52 rows to `verified`
- [ ] **Validation**: VT52 mode tests pass; entering and exiting VT52 toggles the dispatch correctly.

---

## 19.2 Implement DEC LK201 keyboard protocol responses

**File(s):** `oriterm_core/src/term/handler/lk201.rs` (new), `oriterm_core/tests/spec_chain/historical/lk201.rs` (new), sibling tests

The LK201 is the DEC keyboard that VT220/320/420/520 expected. Section 17 handles modern keyboard encoding (kitty/modifyOtherKeys/Win32). This subsection handles the historical contract: DEC-compatible apps can query the terminal for LK201 compatibility via DA2 (secondary device attribute), and the terminal emits the LK201-style key reports documented in the DEC technical manuals.

- [ ] Read the DEC LK201 technical manual for the key report format (scan codes + modifier bytes)
- [ ] Extend the DA2 reply in `handler/status.rs` to include the LK201 identification byte when the DA2 query arrives
- [ ] When the terminal is in VT52 or VT100 legacy mode, emit LK201-style key reports instead of modern CSI-tilde sequences for the function keys (F1-F20)
- [ ] Spec_chain tests for: DA2 reply includes LK201 identification; F1-F20 under LK201 mode emit the DEC-documented byte sequences; modifier keys (shift/ctrl) encode via LK201 modifier byte format
- [ ] Update `catalog/historical.md` LK201 rows to `verified`
- [ ] **Validation**: LK201 tests pass; modern keyboard tests from section 17 still pass (LK201 is a separate encoding path, not a replacement).
- [ ] **TPR checkpoint** — `/tpr-review` covering 19.1-19.2 (VT52 + LK201).

---

## 19.3 Implement Wyse 50/60 protocol extensions

**File(s):** `oriterm_core/src/term/handler/wyse.rs` (new), `oriterm_core/tests/spec_chain/historical/wyse_50_60.rs` (new), sibling tests

Wyse 50/60 extends ECMA-48 with: attribute byte (ESC G followed by a byte encoding the attribute), protected mode (write-protected cells, ESC ) and ESC ( toggle), status line (ESC F followed by a byte sequence defining the status line), function key programming (ESC Z followed by key + command bytes).

- [ ] Implement the Wyse attribute byte handler (ESC G n) — `n` encodes underline/blink/reverse/invisible/dim; map to existing `CellFlags`
- [ ] Implement protected mode: cells marked protected are not editable by subsequent writes; the protected-mode flag lives on `CellFlags::PROTECTED` (add if missing)
- [ ] Implement Wyse status line (ESC F ...) — writes to a reserved bottom row; on non-Wyse-mode terminals, the status row is a virtual extra row rendered separately
- [ ] Implement function key programming (ESC Z ...) — stores a string that the terminal emits when a specific function key is pressed (F1-F16)
- [ ] Spec_chain tests for every Wyse sequence
- [ ] Update `catalog/historical.md` Wyse rows to `verified`
- [ ] **Validation**: Wyse tests pass; existing tests still pass.

---

## 19.4 Implement ADM-3A ESC = cursor addressing + catalog rows for C0 overlap

**File(s):** `oriterm_core/src/term/handler/adm_3a.rs` (new), `oriterm_core/tests/spec_chain/historical/adm_3a.rs` (new)

ADM-3A's distinctive sequence is `ESC = row col` where row and col are offset by 0x20 (space) to avoid clashes with control bytes. The rest of ADM-3A control is C0 controls (BS, HT, LF, CR, etc.) which are already handled by section 08's baseline.

- [ ] Implement `ESC = row col` handler: decode row = byte - 0x20, col = byte - 0x20, call `Term::goto(row, col)`
- [ ] Add catalog rows for the ADM-3A C0 usage pointing to the existing C0 handlers (verified via existing tests — just cite the row)
- [ ] Spec_chain test for the ESC = cursor addressing
- [ ] Update `catalog/historical.md` ADM-3A rows to `verified`
- [ ] **Validation**: tests pass.

---

## 19.5 Implement IBM PC ANSI.SYS extensions (keyboard reassignment, save/restore variants)

**File(s):** `oriterm_core/src/term/handler/ibm_ansi_sys.rs` (new), `oriterm_core/tests/spec_chain/historical/ibm_ansi_sys.rs` (new)

IBM PC ANSI.SYS extended ECMA-48 with: keyboard reassignment (CSI ... p — remaps a key to emit a different byte sequence), save/restore cursor variants (CSI s / CSI u — save/restore cursor position, pre-DEC), and the DOS ANSI music sequence (CSI M — handled by section 20's audio section).

- [ ] Implement CSI ... p keyboard reassignment: parse the reassignment string, store the mapping in a `KeyboardReassignment` table on `Term`, emit the remapped bytes when the key is subsequently pressed
- [ ] Verify CSI s (save cursor, SCO / DOS variant) and CSI u (restore cursor, SCO variant) — these are distinct from the DEC `DECSC` (ESC 7) and `DECRC` (ESC 8)
- [ ] Add catalog rows pointing to existing SGR / cursor handlers for the PC ANSI.SYS overlap rows
- [ ] Spec_chain tests for each PC ANSI.SYS extension
- [ ] Update `catalog/historical.md` IBM PC ANSI.SYS rows to `verified`
- [ ] **Validation**: tests pass; existing DEC cursor save/restore still work (CSI s/u is a separate path from DECSC/DECRC).
- [ ] **TPR checkpoint** — `/tpr-review` covering 19.3-19.5 (Wyse + ADM-3A + IBM PC ANSI.SYS).

---

## 19.6 Verify Microsoft Console VT extensions against the Microsoft spec

**File(s):** `oriterm_core/tests/spec_chain/historical/microsoft_console_vt.rs` (new)

- [ ] Read the Microsoft Console Virtual Terminal Sequences spec end-to-end (snapshot under `plans/spec-conformance/specs/microsoft-console-vt.md`)
- [ ] For each Microsoft-documented extension, write a spec_chain test verifying ori_term's behavior matches the Microsoft spec
- [ ] Most extensions are xterm-compatible — the test verifies the xterm-compatible behavior AND cites the Microsoft doc as the secondary source
- [ ] Update `catalog/historical.md` Microsoft Console VT rows to `verified`
- [ ] **Validation**: every Microsoft-documented sequence has a test.

---

## 19.R Third Party Review Findings

- None.

---

## 19.N Completion Checklist

- [ ] Failing test matrix written FIRST (TDD): every legacy-control stack's spec_chain tests are written and failing before implementation lands
- [ ] **Matrix dimensions**: legacy control stack × sequence family (ESC/CSI/DA2/key) × exemplar sequence per stack — no deferral-fork dimension
- [ ] **Semantic pin**: VT52 mode entry/exit toggle test; Wyse attribute byte application; ADM-3A cursor addressing; IBM PC keyboard reassignment round-trip
- [ ] VT52 mode implemented and verified
- [ ] DEC LK201 keyboard protocol implemented and verified
- [ ] Wyse 50/60 protocol extensions (attribute byte, protected mode, status line, key programming) implemented and verified
- [ ] ADM-3A ESC = cursor addressing implemented; C0 overlap rows verified
- [ ] IBM PC ANSI.SYS extensions (keyboard reassignment CSI p, save/restore cursor variants) implemented and verified
- [ ] Microsoft Console VT extensions verified against the Microsoft spec snapshot
- [ ] No catalog rows marked `verified-with-deviation` for implementation-skip reasons (the deferral fork is gone; the only legitimate deviation is intentional-spec-deviation with citation)
- [ ] BLOAT check: none of the new modules exceed 500 lines (`oriterm_core/src/term/handler/{vt52,lk201,wyse,adm_3a,ibm_ansi_sys}.rs` all split as needed)
- [ ] All existing tests pass
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` + `index.md` updated
- [ ] `/tpr-review` passed
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** Every legacy-control historical stack catalog row is `verified`. VT52, LK201, Wyse 50/60, ADM-3A, IBM PC ANSI.SYS, and Microsoft Console VT all have real implementations and spec_chain tests. Vector stacks (ReGIS, Tek 4014, shared vector_raster helper) are delivered by **Section 26**. No row is deferred; no row is `verified-with-deviation` for an implementation-skip reason.
