---
section: "06"
title: "Terminal Mode Plumbing (Mode 2026 timeout-abort + mode metadata registry)"
status: not-started
reviewed: false
goal: "Wire the Mode 2026 timeout-abort path that currently has zero call sites in ori_term, and centralize DEC mode metadata into a single registry table that fixes the 5-sync-point LEAK across NamedPrivateMode consumers."
success_criteria:
  - "`Processor::sync_timeout()` and `Processor::stop_sync()` are CALLED from `oriterm_mux/src/pane/io_thread/mod.rs` — `grep -rn 'sync_timeout\\|stop_sync' oriterm_mux/src/pane/io_thread/` returns matches"
  - "Sync timeout policy documented: when sync buffer is non-empty AND no new bytes for N milliseconds (default 5000), call `processor.stop_sync(handler)` to flush the buffer and emit `Effect::Presentation(PresentationEffect::SyncAbort { reason: SyncAbortReason::Timeout })`"
  - "Sync abort test: feed BSU + writes + advance virtual clock past timeout — assert SyncAbort effect emitted, snapshot publishes the buffered writes, snapshot_seqno advances by exactly 1"
  - "Mode metadata registry exists at `crates/vte/src/ansi/mode_registry.rs`: a single `static MODE_REGISTRY: &[ModeEntry]` table that drives `PrivateMode::new`, `named_private_mode_number`, `named_private_mode_flag`, and is queried (NOT mirrored) by `apply_decset`/`apply_decrst`"
  - "Adding a new mode requires touching exactly one entry in `MODE_REGISTRY` (verified by walking `git diff` of a hypothetical new-mode commit, NOT a 5-file edit)"
  - "Behavior of DECSET/DECRST stays in match arms in `oriterm_core/src/term/handler/modes.rs` per Codex Q4 pushback (data registry only, no DSL)"
  - "All existing teseq tests covering modes still pass (`cargo test -p oriterm_core --test teseq mode_interactions`)"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Mode 2026 fully wired** + **DEC mode metadata LEAK fixed** mission criteria"
inspired_by:
  - "Alacritty `alacritty_terminal/src/event_loop.rs:228` — sync timeout wiring pattern, calls `processor.stop_sync(handler)` after the sync deadline elapses"
  - "ori_term existing `crates/vte/src/ansi/processor.rs` — `sync_timeout()` and `stop_sync()` API that exists but is never called"
  - "ori_term existing `crates/vte/src/ansi/types.rs:226-295` — NamedPrivateMode enum (canonical source); `helpers.rs:22,56` — the consumers that mirror this enum and form the LEAK"
depends_on: ["03", "04"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "06.1"
    title: "Wire Processor::sync_timeout/stop_sync in io_thread"
    status: not-started
  - id: "06.2"
    title: "Emit PresentationEffect::SyncAbort on timeout"
    status: not-started
  - id: "06.3"
    title: "Build mode metadata registry table"
    status: not-started
  - id: "06.4"
    title: "Migrate consumers to query the registry"
    status: not-started
  - id: "06.5"
    title: "Verify behavior unchanged via teseq mode tests"
    status: not-started
  - id: "06.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "06.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 06: Terminal Mode Plumbing

**Status:** Not Started
**Goal:** Fix two control-plane gaps that block Phase 2 + Phase 3 sections: the Mode 2026 timeout-abort path is completely unwired (Pass 2 confirmed `oriterm_mux/src/pane/io_thread/mod.rs::handle_bytes` calls `processor.advance` but never `sync_timeout`/`stop_sync`), and the DEC mode metadata is scattered across 5 sync points that must stay consistent (LEAK). This section wires the timeout-abort and consolidates mode metadata into a single registry. **Per Codex Q4 pushback, the registry contains DATA ONLY** — DECSET/DECRST behavior stays in match arms because side effects are clearer in code than in a table-driven DSL.

**Success Criteria:**
- [ ] `Processor::sync_timeout`/`stop_sync` called from io_thread on the documented timeout policy
- [ ] `PresentationEffect::SyncAbort` emitted on timeout, snapshot_seqno advances correctly after abort
- [ ] Single mode metadata registry table; consumers query it instead of mirroring
- [ ] Adding a new mode = one registry entry edit (verified by walk-through)
- [ ] All existing mode tests in `teseq/mode_interactions` pass without modification
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green
- [ ] Connects to mission criteria: **Mode 2026 fully wired**, **DEC mode metadata LEAK fixed**

**Context:** Mode 2026 (synchronized output) has two halves. The first half — publication suppression while the sync buffer is non-empty — IS wired in `io_thread/mod.rs:226,263-276`. The second half — timeout-abort that flushes the buffer if the application crashes mid-sync — is NOT wired. The vte processor exposes `sync_timeout()` and `stop_sync()` methods, but Pass 2 confirmed they have zero call sites in `oriterm_mux`. Alacritty wires this in `event_loop.rs:228`. Without the timeout, an app that opens BSU and crashes hangs the terminal indefinitely. The DEC mode metadata LEAK is a separate but related concern: every time someone adds a new private mode, they must edit `NamedPrivateMode` (the canonical enum), `PrivateMode::new` (the parser-side mapper), `named_private_mode_number` (the helper), `named_private_mode_flag` (the helper), AND `apply_decset`/`apply_decrst` (the dispatch). 5 sync points = drift bugs waiting to happen.

**Reference implementations:**
- **Alacritty** `alacritty_terminal/src/event_loop.rs:228` — `processor.stop_sync(handler)` is called from the event loop when the sync deadline elapses. ori_term's io_thread should follow the same pattern.
- **ori_term existing** `oriterm_mux/src/pane/io_thread/mod.rs:200-275` — the io thread's main loop, where the timeout check goes.
- **ori_term existing** `crates/vte/src/ansi/processor.rs` — `sync_timeout`/`stop_sync` API that exists and is never called.

**Depends on:** Section 04 (the verification chain harness exists; the SyncAbort tests use it).

---

## 06.1 Wire Processor::sync_timeout/stop_sync in io_thread

**File(s):** `oriterm_mux/src/pane/io_thread/mod.rs`, `oriterm_mux/src/pane/io_thread/sync_timer.rs` (new), sibling tests

The io thread's main loop must check the sync timeout on every iteration and call `stop_sync(handler)` when the deadline elapses.

- [ ] Read `oriterm_mux/src/pane/io_thread/mod.rs` around lines 200-275 to understand the current loop structure.
- [ ] Add a `sync_deadline: Option<Instant>` field to the io thread state (or wherever the per-pane state lives that's accessible to `handle_bytes` and the loop).
- [ ] In `handle_bytes` (around line 214), after `processor.advance(...)`, check `processor.sync_bytes_count()`:
  - If > 0 and `sync_deadline` is `None`, set `sync_deadline = Some(Instant::now() + processor.sync_timeout())`
  - If = 0, clear `sync_deadline`
- [ ] In the io thread's main loop (the wait loop that polls for new bytes/commands), check `sync_deadline.map(|d| Instant::now() >= d)`. If the deadline has elapsed, call `processor.stop_sync(&mut self.terminal)` to flush the buffer.
- [ ] After `stop_sync`, force a snapshot publication: set `grid_dirty.store(true, Release)` and call `maybe_produce_snapshot()`. The snapshot_seqno must advance by exactly 1 after the abort.
- [ ] Sibling test in `oriterm_mux/src/pane/io_thread/tests.rs`:
  - `sync_timeout_aborts_and_publishes_buffered_writes()` — feeds BSU + writes, advances clock, asserts publication and seqno advance
- [ ] **Validation**: test passes; existing io thread tests still pass.

---

## 06.2 Emit PresentationEffect::SyncAbort on timeout

**File(s):** `oriterm_mux/src/pane/io_thread/mod.rs`

When `stop_sync` is called due to timeout, emit `Effect::Presentation(PresentationEffect::SyncAbort { reason: SyncAbortReason::Timeout })` so test observers (and production consumers) can detect the abort happened.

- [ ] After the `stop_sync` call site in 06.1, push `Effect::Presentation(PresentationEffect::SyncAbort { reason: SyncAbortReason::Timeout })` to the effect sink (Term's effect_sink from section 03).
- [ ] Sibling test:
  - `sync_timeout_emits_sync_abort_presentation_effect()`
- [ ] **Validation**: spec_chain harness can observe the SyncAbort effect via `EffectExpectation::presentation(...)`.

---

## 06.3 Build mode metadata registry table

**File(s):** `crates/vte/src/ansi/mode_registry.rs` (new), `crates/vte/src/ansi/mode_registry/tests.rs` (new)

A single static table with one entry per DEC private mode. The entry contains the mode number, the canonical name, whether it has a TermMode flag, and a pointer to the flag (if applicable). Behavior — the actual side effects of setting/unsetting the mode — stays in match arms per Codex's Q4 pushback.

- [ ] Create `crates/vte/src/ansi/mode_registry.rs`:
  ```rust
  use super::types::NamedPrivateMode;

  pub struct ModeEntry {
      pub number: u16,
      pub name: NamedPrivateMode,
      pub canonical_name: &'static str,
      pub has_term_mode_flag: bool,
      // The flag itself is a function pointer that returns the flag bit,
      // because TermMode is in oriterm_core not crates/vte.
      // Consumers in oriterm_core wrap this with their own dispatch table.
  }

  pub static MODE_REGISTRY: &[ModeEntry] = &[
      ModeEntry { number: 1, name: NamedPrivateMode::CursorKeys, canonical_name: "DECCKM", has_term_mode_flag: true },
      ModeEntry { number: 3, name: NamedPrivateMode::ColumnMode, canonical_name: "DECCOLM", has_term_mode_flag: true },
      ModeEntry { number: 5, name: NamedPrivateMode::ScreenMode, canonical_name: "DECSCNM", has_term_mode_flag: true },
      ModeEntry { number: 6, name: NamedPrivateMode::Origin, canonical_name: "DECOM", has_term_mode_flag: true },
      ModeEntry { number: 7, name: NamedPrivateMode::LineWrap, canonical_name: "DECAWM", has_term_mode_flag: true },
      // ... every mode in NamedPrivateMode
  ];

  /// Lookup by mode number.
  pub fn lookup_by_number(num: u16) -> Option<&'static ModeEntry> {
      MODE_REGISTRY.iter().find(|e| e.number == num)
  }

  /// Lookup by NamedPrivateMode variant.
  pub fn lookup_by_name(name: NamedPrivateMode) -> Option<&'static ModeEntry> {
      MODE_REGISTRY.iter().find(|e| e.name == name)
  }
  ```
- [ ] Sibling tests:
  - `every_named_private_mode_variant_has_a_registry_entry()` — exhaustive match on NamedPrivateMode variants asserts each is present in the registry
  - `lookup_by_number_returns_correct_entry()`
  - `lookup_by_name_returns_correct_entry()`
  - `registry_is_sorted_by_number()` (optional, for binary search later)
- [ ] **Validation**: registry compiles, exhaustive test passes (catches the LEAK condition where someone adds a NamedPrivateMode variant but forgets to add a registry entry).

---

## 06.4 Migrate consumers to query the registry

**File(s):** `crates/vte/src/ansi/types.rs`, `oriterm_core/src/term/handler/helpers.rs`

The 4 consumers that previously mirrored `NamedPrivateMode` data now query the registry instead. The 5th consumer (`apply_decset`/`apply_decrst` match arms) keeps its match arms but the arms internally call `mode_registry::lookup_by_name(...).has_term_mode_flag` and similar queries when they need to know "does this mode have a flag" — they DON'T mirror the answer.

- [ ] Migrate `PrivateMode::new()` in `crates/vte/src/ansi/types.rs:175` to call `mode_registry::lookup_by_number(num)` and return the variant from the entry.
- [ ] Migrate `named_private_mode_number()` in `oriterm_core/src/term/handler/helpers.rs:22` to call `mode_registry::lookup_by_name(name).map(|e| e.number)`.
- [ ] Migrate `named_private_mode_flag()` in `oriterm_core/src/term/handler/helpers.rs:56` similarly.
- [ ] Leave `apply_decset`/`apply_decrst` match arms in `oriterm_core/src/term/handler/modes.rs:17-102` AS-IS (per Codex Q4 pushback). The match arms contain the side-effect logic; they query the registry only when they need to look up metadata.
- [ ] **Validation**: `cargo test -p oriterm_core --test teseq mode_interactions` passes (the existing mode tests are the regression guard); `grep -rn 'NamedPrivateMode::' oriterm_core/src/term/handler/helpers.rs` returns ZERO matches (the helpers no longer hardcode the variants).

---

## 06.5 Verify behavior unchanged via teseq mode tests

**File(s):** `oriterm_core/tests/teseq/mode_interactions.rs` (no changes; the existing tests are the regression guard)

The mode metadata refactor must not change observable behavior. The existing teseq mode_interactions tests cover ~50 mode toggle scenarios and serve as the regression guard.

- [ ] Run `cargo test -p oriterm_core --test teseq mode_interactions` and confirm all tests pass.
- [ ] If any test fails, the registry migration is wrong somewhere — investigate the specific mode and trace through the registry lookup.
- [ ] **Validation**: zero test failures in `mode_interactions`.

---

## 06.R Third Party Review Findings

- None.

---

## 06.N Completion Checklist

- [ ] Failing test matrix written FIRST: `sync_timeout_aborts_and_publishes_buffered_writes` and `every_named_private_mode_variant_has_a_registry_entry` written before implementation
- [ ] **Matrix dimensions**: sync state (no-sync, in-sync, post-sync, post-abort) × event type (timeout/manual-end/nested-BSU) × mode metadata consumer × NamedPrivateMode variant (every one)
- [ ] **Semantic pin**: `every_named_private_mode_variant_has_a_registry_entry` — exhaustive match catches the LEAK condition forever; if a future PR adds a NamedPrivateMode variant without a registry entry, this test fails at compile time
- [ ] `Processor::sync_timeout`/`stop_sync` called from io_thread (`grep` confirms)
- [ ] SyncAbort effect emitted on timeout
- [ ] Mode metadata registry exists; consumers query it; LEAK fixed
- [ ] Behavior unchanged: existing teseq mode tests pass without modification
- [ ] Alloc regression unchanged
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` Quick Reference + mission criteria updated (mode 2026 partially checked off; full check in section 09)
- [ ] `index.md` section 06 status updated
- [ ] `/tpr-review` passed
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** Mode 2026 timeout-abort wired and emits SyncAbort; mode metadata registry consolidates the 5-sync-point LEAK into one table; existing mode tests pass.
