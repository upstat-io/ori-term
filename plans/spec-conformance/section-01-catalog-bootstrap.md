---
section: "01"
title: "Catalog Bootstrap"
status: not-started
reviewed: false
goal: "Build the spec-conformance catalog as the empirical map of every protocol sequence ori_term targets, with implementation pointers and provisional verification status — no tests written in this section."
success_criteria:
  - "`plans/spec-conformance/catalog/` exists with one markdown file per protocol family (16 catalog files + README + _legacy-tack-mapping placeholder created by section 02)"
  - "Every C0/C1/ESC/CSI/OSC/DCS/APC/PM/SOS sequence handled in `crates/vte/src/ansi/dispatch/` has a row in the appropriate catalog file"
  - "Every numbered DEC private mode in `crates/vte/src/ansi/types.rs::NamedPrivateMode` has a row in `catalog/dec-private-modes.md`"
  - "Every SGR parameter in `crates/vte/src/ansi/dispatch/csi.rs:284-358` has a row in `catalog/ecma-48.md`"
  - "Every OSC number with a handler in `oriterm_core/src/term/handler/osc.rs` has a row in `catalog/osc.md`"
  - "Every cited spec is either committed in `plans/spec-conformance/specs/` (if redistributable) or listed in `specs/manifest.toml` with sha256 + fetch script entry (if license-restricted)"
  - "Every catalog row has its `Implementation` column filled (`file:line` or `MISSING`) and `Verification` column set to `implemented-unverified`, `stub`, or `missing`"
  - "Audit memory corrections applied: `architecture_graphics_audit.md` updated to reflect (a) HSL hue rotation IS correct, (b) kitty `q=1` IS implemented, (c) image cache default is 320 MiB"
  - "`./build-all.sh` and `./test-all.sh` and `./clippy-all.sh` green (no behavior change in this section)"
  - "Section's mission criterion connection: contributes to `Catalog complete` mission criterion in 00-overview.md"
inspired_by:
  - "wezterm `docs/escape-sequences.md` — per-sequence catalog table format with Seq | Hex | Name | Description | Action columns"
  - "ori_term `architecture_graphics_audit.md` memory — implementation file:line citations for graphics protocols"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "01.1"
    title: "Bottom-up harvest from ori_term VTE dispatch"
    status: not-started
  - id: "01.2"
    title: "Bottom-up harvest from wezterm escape-sequences.md"
    status: not-started
  - id: "01.3"
    title: "Bottom-up harvest from real-app captures"
    status: not-started
  - id: "01.4"
    title: "Top-down walk through primary specs"
    status: not-started
  - id: "01.5"
    title: "Spec corpus assembly + manifest"
    status: not-started
  - id: "01.6"
    title: "Audit memory corrections"
    status: not-started
  - id: "01.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "01.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 01: Catalog Bootstrap

**Status:** Not Started
**Goal:** Build the catalog as the empirical map of every protocol sequence ori_term targets. No tests are written in this section — the catalog itself is the deliverable. Every subsequent stack section consumes this catalog as its scope definition. The provisional row schema in `00-overview.md` is the starting template; section 04 freezes the schema after the verification chain pilots prove what fields are actually needed.

**Success Criteria:**
- [ ] `plans/spec-conformance/catalog/` exists with 16 protocol-family markdown files plus `README.md` (and `_legacy-tack-mapping.md` created by section 02)
- [ ] Every sequence ori_term currently parses or dispatches has a row in the appropriate catalog file
- [ ] Every row has `Implementation` (file:line or `MISSING`), `Verification` status (`implemented-unverified` / `stub` / `missing`), and `Apex layer` (provisional, may revise after section 04)
- [ ] Spec corpus assembled in `plans/spec-conformance/specs/` with `manifest.toml` listing every cited document
- [ ] License-restricted specs are NOT committed; they have `manifest.toml` entries with sha256 + fetch script
- [ ] Audit memory at `architecture_graphics_audit.md` updated to correct three stale claims (HSL hue, kitty q=1, image cache size)
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` all green (no code behavior change in this section)
- [ ] Connects to mission criterion: **Catalog complete**

**Context:** The catalog is the prerequisite that makes every subsequent section's scope mechanical. Without it, "100% conformance" is unfalsifiable because you don't know what 100% means. Per Codex's Round 2 guidance, this section delivers catalog *breadth* first (every sequence enumerated); the row *schema* is provisional until section 04's pilots prove what fields the verification chain needs. The audit memory at `architecture_graphics_audit.md` provides a starting inventory of graphics protocol implementations, but research Pass 1 found three stale claims that this section corrects.

**Reference implementations:**
- **wezterm** `docs/escape-sequences.md` (415 lines) — per-sequence catalog with `Seq | Hex | Name | Description | Action` columns. Same shape ori_term's catalog uses, with the addition of verification status and apex layer columns.
- **ori_term** `crates/vte/src/ansi/dispatch/{mod,csi,osc}.rs` — bottom-up source of truth for what ori_term currently parses. Every match arm = one catalog row.
- **ori_term** `crates/vte/src/ansi/types.rs::NamedPrivateMode` — canonical enum of DEC private modes ori_term recognizes. Every variant = one row in `catalog/dec-private-modes.md`.

**Depends on:** None. This is the first section.

---

## 01.1 Bottom-up harvest from ori_term VTE dispatch

**File(s):** `plans/spec-conformance/catalog/{ecma-48,xterm-ctlseqs,dec-private-modes,osc,sixel,kitty-graphics,iterm2}.md` (created)
**Source code read:** `crates/vte/src/ansi/dispatch/mod.rs`, `crates/vte/src/ansi/dispatch/csi.rs`, `crates/vte/src/ansi/dispatch/osc.rs`, `crates/vte/src/ansi/types.rs`, `oriterm_core/src/term/handler/`

This subsection harvests every sequence ori_term currently parses or dispatches into the appropriate catalog file. Each row gets `Implementation` filled with `file:line` of the dispatch arm + handler method, and `Verification` set to `implemented-unverified` (the default per the new taxonomy in 00-overview.md).

- [ ] Read `crates/vte/src/ansi/dispatch/mod.rs` end-to-end. For every C0/ESC/C1 dispatch arm, add a row to the appropriate catalog file:
  - C0 (BEL/BS/HT/LF/VT/FF/CR/SO/SI) → `catalog/ecma-48.md` under "C0 Controls"
  - ESC sequences (RIS/DECSC/DECRC/DECPAM/DECPNM/IND/NEL/HTS/RI/SS2/SS3/G0-G3 designation) → `catalog/ecma-48.md` under "ESC Sequences"
  - C1 7-bit ESC-prefixed and 8-bit forms — note 8-bit forms are MISSING per Pass 1 finding
- [ ] Read `crates/vte/src/ansi/dispatch/csi.rs` end-to-end. For every CSI match arm, add a row to `catalog/ecma-48.md` (cursor/erase/insert/scroll/SGR/modes) or `catalog/xterm-ctlseqs.md` (window manipulation, focus events, bracketed paste, DECRQM, DECRQSS):
  - Cursor: CUU, CUD, CUF, CUB, CNL, CPL, CHA, CUP, HVP, CHT, CBT
  - Erase: ED, EL, ECH
  - Insert/Delete: ICH, DCH, IL, DL
  - Scroll: SU, SD, DECSTBM
  - SGR: every parameter from 0-9, 21-29, 30-37, 38, 39, 40-47, 48, 49, 51-55, 58, 59, 90-97, 100-107 (one row per SGR param)
  - Modes: SM, RM, DECSET, DECRST (one row per mode in `catalog/dec-private-modes.md`)
  - Status reports: DA1, DA2, DA3, DSR, CPR, DECRQM, DECRQSS
  - Window: CSI t (every sub-op), push/pop title
  - Cursor style: DECSCUSR
  - Tabs: TBC, CHT, CBT, HTS
- [ ] Read `crates/vte/src/ansi/dispatch/osc.rs` end-to-end. For every OSC handler arm, add a row to `catalog/osc.md`:
  - OSC 0/1/2 (title/icon)
  - OSC 4 (palette set/query)
  - OSC 7 (CWD)
  - OSC 8 (hyperlinks)
  - OSC 10/11/12 (default colors)
  - OSC 22 (mouse cursor icon)
  - OSC 50 (cursor shape legacy)
  - OSC 52 (clipboard)
  - OSC 104/110/111/112 (color reset)
  - OSC 1337 (iTerm2 inline images) → `catalog/iterm2.md` instead
- [ ] Read `crates/vte/src/ansi/dispatch/mod.rs` for DCS dispatch. Add rows to `catalog/sixel.md` (DCS q) and `catalog/ecma-48.md` (DECRQSS / DCS $ q).
- [ ] Read `oriterm_core/src/term/handler/image/kitty.rs` for APC `_G` dispatch. Add rows to `catalog/kitty-graphics.md` for every action handled (transmit, place, delete, animate, query, frame composition).
- [ ] For each row, fill `Implementation` with `file:line` of the dispatch arm AND the handler method (e.g., `crates/vte/src/ansi/dispatch/csi.rs:91 → oriterm_core/src/term/handler/cursor.rs:goto`).
- [ ] For each row, set `Verification` based on Pass 1 findings:
  - `implemented-unverified` if a handler exists and does meaningful work
  - `stub` for sequences in the Pass 1 STUB list (SGR 5/6 blink, SGR 8 conceal, mode 1007 alt scroll, mode 9001 Win32, modifyOtherKeys, SCP, DECLRMM)
  - `missing` for sequences explicitly NOT FOUND in Pass 1 (8-bit C1, ANSI music CSI M, DECPS, octants, NRCS variants beyond ASCII+Special)
- [ ] **Validation**: every match arm in `crates/vte/src/ansi/dispatch/{mod,csi,osc}.rs` must correspond to exactly one catalog row. Walk the files and check.

---

## 01.2 Bottom-up harvest from wezterm escape-sequences.md

**File(s):** `plans/spec-conformance/catalog/{ecma-48,xterm-ctlseqs,dec-private-modes,osc}.md` (extended)
**Source read:** `~/projects/reference_repos/console_repos/wezterm/docs/escape-sequences.md` (415 lines)

WezTerm's escape-sequences.md is a curated catalog with `Seq | Hex | Name | Description | Action` columns. It's the closest thing to a "modern terminal escape sequence registry" and covers ECMA-48, xterm extensions, and many DEC private modes. This subsection cross-references wezterm's catalog against ori_term's, adding any rows wezterm has that ori_term doesn't yet enumerate.

- [ ] Read `~/projects/reference_repos/console_repos/wezterm/docs/escape-sequences.md` cover-to-cover.
- [ ] For every sequence wezterm documents that is NOT yet in ori_term's catalog files, add a row with `Implementation: MISSING` and `Verification: missing`. Use wezterm's "Description" and "Action" columns as the spec source for this row (mark `Spec source: wezterm escape-sequences.md` until 01.4 finds a better authoritative source).
- [ ] For sequences that ARE in ori_term's catalog but where wezterm has additional notes (e.g., specific quirks or edge cases), add the wezterm note to the row's `Notes` column.
- [ ] **Validation**: every section header in wezterm's escape-sequences.md must correspond to entries in ori_term's catalog files (either as existing rows or as newly added MISSING rows).

---

## 01.3 Bottom-up harvest from real-app captures

**File(s):** `plans/spec-conformance/catalog/*.md` (extended)
**Source: live PTY capture from real applications**

The real test of "what does ori_term need to implement" is "what do real applications actually emit." This subsection captures byte streams from a curated set of real apps and adds catalog rows for any unique sequences observed.

- [ ] Set up capture infrastructure:
  - Use `script -c '<command>' -O - 2>/dev/null` to capture PTY output to a file
  - Or `ttyrec -e '<command>' /tmp/capture.tty` if available
- [ ] Run captures for each app for ~30s each, exercising typical flows:
  - `script -c '/usr/bin/notcurses-demo -p /usr/share/notcurses' /tmp/notcurses.cap` (if installed)
  - `script -c 'vim +set\ nu /etc/passwd' /tmp/vim.cap`
  - `script -c 'htop' /tmp/htop.cap` (10s then quit)
  - `script -c 'btop' /tmp/btop.cap` (10s then quit)
  - `script -c 'tmux new -d \"echo hello\" && tmux ls && tmux kill-server' /tmp/tmux.cap`
  - `script -c 'aerc -h' /tmp/aerc.cap`
  - `script -c 'helix /etc/passwd' /tmp/helix.cap` (if installed)
  - `script -c 'less /etc/passwd' /tmp/less.cap`
  - `script -c 'ncmpcpp -h' /tmp/ncmpcpp.cap` (if installed)
  - `script -c 'nvim +q' /tmp/nvim.cap`
- [ ] Parse each capture file with a simple Python/Rust script that extracts every unique escape sequence (CSI/OSC/DCS/APC). Sort by frequency.
- [ ] For each unique sequence not yet in the catalog, add a row with `Implementation: MISSING` and `Notes: emitted by <app>`.
- [ ] **Validation**: for each app captured, the most common 10 sequences must already have catalog rows after this subsection.

---

## 01.4 Top-down walk through primary specs

**File(s):** `plans/spec-conformance/catalog/*.md` (refined)
**Source read:** Spec documents in `plans/spec-conformance/specs/` (created by 01.5)

After 01.1-01.3 give bottom-up coverage, walk every primary spec document with the catalog open and check for gaps. This is slower but catches the 20% the bottom-up scan missed and grounds every row in its authoritative source.

- [ ] For each catalog file, identify the primary spec (per the authority ladder in `00-overview.md`):
  - `catalog/ecma-48.md` → ECMA-48 + xterm ctlseqs
  - `catalog/xterm-ctlseqs.md` → xterm ctlseqs
  - `catalog/dec-private-modes.md` → xterm ctlseqs + DEC technical manuals
  - `catalog/osc.md` → xterm ctlseqs + iTerm2 docs + per-OSC source
  - `catalog/sixel.md` → DEC STD 070 + libsixel
  - `catalog/kitty-graphics.md` → kitty source itself (`~/projects/reference_repos/console_repos/kitty/`)
  - `catalog/kitty-keyboard.md` → kitty source + sw.kovidgoyal.net docs
  - `catalog/iterm2.md` → iTerm2 docs
  - `catalog/mode-2026.md` → contour-terminal spec
  - `catalog/unicode-subcell.md` → Unicode chart PDFs (U+1FB00, U+1CD00)
  - `catalog/mouse.md` → xterm ctlseqs
  - `catalog/charsets.md` → ISO 2022 + DEC technical manuals + UAX
  - `catalog/audio-print.md` → DEC technical manuals + ANSI.SYS reference
  - `catalog/shell-integration.md` → Final Term + iTerm2 + VS Code source
  - `catalog/historical.md` → DEC user manuals (VT52 + VT100-520), DEC LK201 technical manual, DEC ReGIS technical manual, Tektronix 4014 Programmer's Reference Manual, Wyse 50/60 user manual, ADM-3A docs, MS-DOS ANSI.SYS reference, Microsoft Console VT spec
- [ ] For each spec section in each primary spec, check the catalog. Missing rows get added with `Implementation: MISSING` and the primary spec as the source.
- [ ] For ambiguous spec text (where multiple interpretations exist), populate the `De-facto reference` column with the chosen tiebreaker per the authority ladder.
- [ ] **Validation**: every spec section in every primary spec corresponds to at least one catalog row.

---

## 01.5 Spec corpus assembly + manifest

**File(s):** `plans/spec-conformance/specs/` (created)

The spec corpus lives in-tree under `plans/spec-conformance/specs/`. Freely-redistributable specs are committed; license-restricted specs use a manifest with sha256 + fetch script. This subsection assembles the corpus and writes the manifest.

- [ ] Create `plans/spec-conformance/specs/` directory.
- [ ] Create `plans/spec-conformance/specs/manifest.toml` with one entry per spec document:
  ```toml
  [specs.kitty_graphics_protocol]
  url = "https://sw.kovidgoyal.net/kitty/graphics-protocol/"
  local_path = "specs/kitty-graphics-protocol.md"
  license = "GPL-3.0"
  redistributable = true
  sha256 = "..."  # filled after fetch

  [specs.dec_std_070]
  url = "https://vt100.net/dec/ek-vt382-rm-001.pdf"
  local_path = "specs/dec-std-070.pdf"
  license = "Manufacturer documentation — verify before commit"
  redistributable = false  # stored via fetch script only
  sha256 = "..."
  ```
- [ ] Create `plans/spec-conformance/specs/manifest-fetch.sh` script that:
  - Reads `manifest.toml`
  - For each `redistributable = false` entry, downloads the document to a local cache directory and verifies sha256
  - Fails loudly if a sha256 doesn't match
  - Skips already-cached entries
- [ ] Commit redistributable specs:
  - kitty graphics protocol (markdown snapshot)
  - kitty keyboard protocol (markdown snapshot)
  - mode 2026 spec (contour-terminal docs/vt-extensions.md)
  - OSC 8 hyperlinks (gist:egmontkob spec)
  - Final Term semantic prompt (OSC 133 doc)
  - UAX #9, #11, #29 plain-text snapshots (Unicode publishes these freely)
  - Unicode Symbols for Legacy Computing chart PDFs (publicly redistributable)
- [ ] Restricted specs go in manifest only with fetch instructions:
  - ECMA-48 (verify license — likely fetchable but not committable)
  - xterm ctlseqs (verify with invisible-island.net)
  - DEC technical manuals (DEC reference material — verify)
  - Tektronix 4014 manual (vintage docs — verify)
- [ ] **Validation**: `bash plans/spec-conformance/specs/manifest-fetch.sh --verify` passes (every committed file matches its sha256, every restricted entry has a working fetch URL).

---

## 01.6 Audit memory corrections

**File(s):** `/home/eric/.claude/projects/-home-eric-projects-ori-term/memory/architecture_graphics_audit.md` (updated)

Pass 1 found three stale claims in the audit memory. This subsection corrects them so future sessions don't propagate the errors.

- [ ] Update `architecture_graphics_audit.md`:
  - **HSL hue rotation**: Remove "suspected wrong" status. Add note: "Verified correct as of 2026-04-08 — `oriterm_core/src/image/sixel/color.rs:41` does `hue - 120.0` correctly."
  - **Kitty q=1 query**: Remove "NOT IMPLEMENTED" status. Add note: "Verified implemented as of 2026-04-08 — `oriterm_core/src/image/kitty/parse.rs:197` returns `KittyAction::Query`; handler at `oriterm_core/src/term/handler/image/kitty.rs:320`."
  - **Image cache size**: Update "default 512 MiB cap" to "default 320 MiB (Ghostty parity per `oriterm_core/src/image/cache/mod.rs:15`)."
- [ ] Update `MEMORY.md` if it contains an entry about the image cache size — the project memory should reflect 320 MiB, not 512 MiB.
- [ ] **Validation**: `grep -r '512 MiB' /home/eric/.claude/projects/-home-eric-projects-ori-term/memory/` returns no matches related to image cache size.

---

## 01.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers. -->

- None.

---

## 01.N Completion Checklist

- [ ] Every match arm in `crates/vte/src/ansi/dispatch/{mod,csi,osc}.rs` corresponds to exactly one catalog row (`./scripts/catalog-coverage-check.sh` if implemented, else manual walk)
- [ ] Every variant in `NamedPrivateMode` enum has a row in `catalog/dec-private-modes.md`
- [ ] Every OSC number with a handler in `oriterm_core/src/term/handler/osc.rs` has a row in `catalog/osc.md`
- [ ] `plans/spec-conformance/specs/manifest.toml` exists and `bash specs/manifest-fetch.sh --verify` passes
- [ ] All freely-redistributable specs committed under `plans/spec-conformance/specs/`
- [ ] Audit memory corrections applied (HSL, kitty q=1, image cache size)
- [ ] `./build-all.sh` green (no code changes — should be a no-op)
- [ ] `./test-all.sh` green (no code changes)
- [ ] `./clippy-all.sh` green (no code changes)
- [ ] Plan annotation cleanup: any temporary notes or scaffolding removed
- [ ] Section frontmatter `status` → `complete`, subsection statuses updated
- [ ] `00-overview.md` Quick Reference table status updated for section 01
- [ ] `00-overview.md` mission success criteria updated (check off "Catalog complete" partially — full check after section 04 freezes the schema)
- [ ] `index.md` section 01 status updated
- [ ] `/tpr-review` passed — independent Codex review found no critical or major issues
- [ ] `/impl-hygiene-review last commit` passed — hygiene review clean. MUST run AFTER `/tpr-review` is clean.

**Exit Criteria:** `plans/spec-conformance/catalog/` contains 16+ markdown files with one row per known protocol sequence; every row has `Implementation` (file:line or `MISSING`) and `Verification` (one of: missing/stub/implemented-unverified) populated; spec corpus assembled with manifest; audit memory corrections applied; full build/test/clippy green. The catalog is now the empirical map of territory and section 02-25 can use it as their scope source.
