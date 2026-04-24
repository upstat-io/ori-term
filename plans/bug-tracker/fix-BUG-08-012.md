---
bug: "BUG-08-12"
title: "Kitty keyboard mode persists after program exit — shell renders raw CSI u sequences instead of typed characters"
severity: "high"
status: in-progress
goal: "Kitty keyboard enhancement modes pushed by a subprocess (e.g. notcurses-demo) are automatically restored to their pre-command depth when the shell regains control via OSC 133, so the shell prompt accepts keystrokes in its native encoding without requiring a blind `reset`."
success_criteria:
  - "After `CSI > 1 u` (push), `OSC 133 ; C ST` (command-start), child crash (no pop), `OSC 133 ; A ST` (next prompt): `keyboard_mode_stack` is truncated to its pre-command depth and `TermMode::KITTY_KEYBOARD_PROTOCOL` reflects the restored top — semantic pin test in `oriterm_mux/src/shell_integration/tests.rs`."
  - "Shell that itself holds kitty modes across commands (stack depth N at `OSC 133;C`) still has those N modes after `OSC 133;A` — restore only discards child-pushed pushes, never shell-pushed pushes."
  - "Child `CSI = Ps u` (same-depth mode mutation) during a command does not leak past `OSC 133;A` — restore unconditionally reapplies top-of-stack mode, so dirty `TermMode::KITTY_KEYBOARD_PROTOCOL` bits are reverted to shell-held state."
  - "Alt-screen per-screen snapshot: snapshot taken on one screen only fires restore on that screen — paired `inactive_pre_command_kb_stack_snapshot` swaps alongside stacks in `toggle_alt_common`."
  - "Pre-existing DRIFT in `toggle_alt_common` (stacks swapped without reapplying top-of-mode) is fixed in this same bug per Broken Window Policy — `TermMode::KITTY_KEYBOARD_PROTOCOL` reflects the newly-active stack's top after every `?1049h/l` toggle."
  - "Regression guard: without shell integration (no OSC 133 stream), push/pop semantics are unchanged — RIS and DECSTR remain the only resets."
  - "OSC 633 (VS Code shell integration) wires the same snapshot/restore points as OSC 133."
  - "No allocations added to the OSC 133 A/C/D hot paths or `toggle_alt_common` — interceptor stays allocation-free on every dispatch."
  - "`timeout 150 ./test-all.sh` green; `./build-all.sh` green; `./clippy-all.sh` green; Windows cross-compile green."
subsystem: "oriterm_core/src/term/mod.rs, oriterm_core/src/term/shell_state/mod.rs, oriterm_core/src/term/handler/esc.rs, oriterm_core/src/term/alt_screen.rs, oriterm_mux/src/shell_integration/interceptor.rs"
found: "2026-04-14"
source: "manual"
third_party_review:
  status: none
  updated: null
---

# Fix: BUG-08-12 — Kitty keyboard mode persists after program exit

**Status:** In Progress
**Severity:** high
**Goal:** When the shell re-emits `OSC 133 ; A ST` to draw its next prompt after a kitty-aware subprocess exits, the terminal restores the `keyboard_mode_stack` to the depth it had at `OSC 133 ; C ST` (command-start). Modes pushed by the now-dead subprocess are discarded; modes held by the shell itself are preserved.

**Success Criteria:** see frontmatter.

**Context:** `notcurses-demo` (and any kitty-aware TUI) pushes one or more `CSI > ... u` keyboard modes on entry and is expected to pop them with `CSI < ... u` on exit. Real programs crash, are SIGKILLed, or exit without cleanup. When the shell resumes, the stale modes remain in `Term::keyboard_mode_stack`, so `TermMode::KITTY_KEYBOARD_PROTOCOL` stays asserted. `oriterm/src/key_encoding/mod.rs:118` then routes every shell keystroke through the kitty `CSI u` encoder, which the shell (bash / zsh / fish) does not speak — the user sees raw `0;1;100u7;1;97u` fragments and the terminal is unusable until a blind `reset`.

---

## 1. Root Cause Analysis

- **Symptom**: After running `notcurses-demo` (or any kitty-aware TUI) and returning to the shell, typed characters display as raw escape fragments such as `0;1;100u7;1;97u`. The terminal is unusable until `reset` is typed blind.
- **Proximate cause**: `oriterm/src/key_encoding/mod.rs:118` gates dispatch on `input.mode.intersects(TermMode::KITTY_KEYBOARD_PROTOCOL)`. The flag is still set because `Term::keyboard_mode_stack` has entries pushed by the dead subprocess.
- **Root cause**: `keyboard_mode_stack` has only two reset paths — RIS (`ESC c`, `oriterm_core/src/term/handler/esc.rs:47-48`) and DECSTR (`CSI ! p`, same file). Subprocess exit is not a reset signal. The stack's lifecycle is tied to the lifetime of `Term`, not to the lifetime of the subprocess that pushed it.
- **Blast radius**: Every user who runs `notcurses-demo`, `kitty @ kitten`, newer `htop`/`btm`, or any CSI-u-aware TUI is exposed. The bug is a regression compared to legacy-only terminals because those terminals never honor kitty push in the first place. The symptom manifests only after the subprocess returns — so the damage is most visible at exactly the moment the user expects the shell to be usable again.
- **Affected files**:
  - `oriterm_core/src/term/mod.rs` — add PAIRED fields `pre_command_kb_stack_snapshot: Option<VecDeque<KeyboardModes>>` + `inactive_pre_command_kb_stack_snapshot: Option<VecDeque<KeyboardModes>>` on `Term`, mirroring the `keyboard_mode_stack`/`inactive_keyboard_mode_stack` per-screen pattern; seed both to `None` in `Term::new()`.
  - `oriterm_core/src/term/shell_state/mod.rs` — add two methods (`snapshot_keyboard_mode_stack`, `restore_keyboard_mode_stack`) that record / unconditionally reapply top-of-stack mode (covers truncation AND `CSI = Ps u` same-depth mutations).
  - `oriterm_core/src/term/handler/esc.rs` — RIS and DECSTR must clear BOTH paired snapshot fields alongside the existing stack clears.
  - `oriterm_core/src/term/alt_screen.rs` — `toggle_alt_common` must (a) swap the paired snapshot fields alongside the stacks, AND (b) fix pre-existing DRIFT by reapplying the new active stack's top-of-stack mode via `dcs_set_keyboard_mode` so `TermMode::KITTY_KEYBOARD_PROTOCOL` reflects the active screen — this pre-existing drift is load-bearing for this bug (kitty mode bits leak across screens), so it ships with BUG-08-012 per Broken Window Policy.
  - `oriterm_mux/src/shell_integration/interceptor.rs` — call `snapshot_keyboard_mode_stack` on `OSC 133 ; C` (and `OSC 633 ; C`); call `restore_keyboard_mode_stack` on `OSC 133 ; A` / `OSC 133 ; D` / `OSC 633 ; A` / `OSC 633 ; D`.

**Reference implementations**:

- `~/projects/reference_repos/console_repos/wezterm/term/src/screen.rs:92-94` — only clears `keyboard_stack` on `full_reset` (RIS). WezTerm does NOT auto-restore on prompt boundary. The bug report's claim that "WezTerm doesn't exhibit this" is not backed by a prompt-boundary hook in wezterm's source — verified by grep over the whole `wezterm/term/src/`. WezTerm users may avoid the symptom because bash/zsh with wezterm's `shell-integration` binding blindly emits `CSI < 127 u` on every prompt, not because the terminal does anything itself.
- `~/projects/reference_repos/console_repos/ghostty/src/Surface.zig:1270` — only resets kitty keyboard when the SHELL itself exits and Ghostty shows the "Process exited" message. Not a per-prompt-boundary reset.
- `~/projects/reference_repos/console_repos/alacritty/alacritty_terminal/src/term/mod.rs:519` — same as WezTerm: reset on RIS only.

No reference terminal auto-pops at OSC 133;A out of the box. This fix uses the existing shell-integration hook as a signal the reference terminals do not have. The chosen approach is conservative: it restores to the snapshotted depth rather than clearing, so shells that themselves push kitty modes are preserved.

---

## 1.5 Fix Consensus (via /tp-help)

Independent dual-source design review of the proposed fix approach. Ran BEFORE tests or implementation to catch wrong-approach errors before they lock in.

- **Proposed approach (pre-consensus)**: Snapshot `keyboard_mode_stack.len()` into a new `Term` field `pre_command_kb_stack_snapshot: Option<VecDeque<KeyboardModes>>` when `OSC 133 ; C` (or `OSC 633 ; C`) arrives. Restore by truncating `keyboard_mode_stack` back to that depth and reapplying the top-of-stack mode when `OSC 133 ; A` / `;D` (or `OSC 633 ; A` / `;D`) arrives. Clear the field on RIS and DECSTR. No changes to the kitty push/pop parsing path — this is additive around the existing stack.
- **tp-help run scratch dir**: `/tmp/tpr-round-ori_term-6MfXaEtv` (2026-04-24 Phase 1.75 consensus round).

### Round 1

- **Codex (HIGH trust)**: Endorsed OSC 133/633 mechanism. Flagged three load-bearing refinements:
  1. **Alt-screen**: a single `Option<usize>` cannot track per-screen snapshot depths. Recommended either (a) one snapshot struct `{primary, alt}` never swapped, or (b) paired fields swapped alongside the stacks.
  2. **Restore must UNCONDITIONALLY reapply top-of-stack mode** — not just when `len() > saved`. `CSI = Ps u` (`crates/vte/src/ansi/dispatch/csi/mod.rs:333-340`) routes through `handler.set_keyboard_mode(mode, behavior)` → `dcs_set_keyboard_mode` (`oriterm_core/src/term/handler/dcs.rs:68-84`) which mutates `TermMode::KITTY_KEYBOARD_PROTOCOL` bits without touching the stack. A strict `>` check would leave same-depth dirty state alive after restore.
  3. **Pre-existing DRIFT in `toggle_alt_common`**: `oriterm_core/src/term/alt_screen.rs:90-106` swaps `keyboard_mode_stack` ↔ `inactive_keyboard_mode_stack` but NEVER reapplies the new top — kitty mode bits leak across screens on every `?1049h/l` toggle. This is the exact pathway by which BUG-08-012 manifests in alt-screen paths even with restore implemented.
- **Gemini (LOWER trust, full-file verification)**: Converged on the same core mechanism and same alt-screen gap. Specifically endorsed paired-fields-swapped-in-toggle variant. Flagged the same pre-existing `toggle_alt_common` DRIFT under Broken Window Policy. Proposed three TDD cells: `keyboard_mode_stack_restore_is_per_screen`, `_restore_across_toggles`, `_restore_depth_decreased`.
- **Independent code verification (fresh reads of actual files)**:
  - `CSI = Ps u` dispatch → `handler.set_keyboard_mode(mode, behavior)` verified at `crates/vte/src/ansi/dispatch/csi/mod.rs:340` ✓
  - `dcs_set_keyboard_mode` mutates `self.mode` without touching stack verified at `oriterm_core/src/term/handler/dcs.rs:73-84` ✓
  - `toggle_alt_common` ends at `alt_screen.rs:105` with `grid_mut().dirty_mut().mark_all()` and NO `dcs_set_keyboard_mode` call after the swap ✓ (DRIFT verified)
  - Existing paired-state convention: `keyboard_mode_stack` + `inactive_keyboard_mode_stack` at `oriterm_core/src/term/mod.rs:104,107` ✓
- **Disagreement resolution**:
  - **Alt-screen storage shape**: Codex preferred `{primary, alt}` struct never swapped; Gemini preferred paired `active + inactive` `Option<usize>` swapped in `toggle_alt_common`. **Adopted Gemini's shape** — it mirrors the existing `keyboard_mode_stack`/`inactive_keyboard_mode_stack` convention (consistency > marginal simplification).
  - **Restore semantics**: Gemini said `> saved` strict-check is sufficient; Codex showed `CSI = Ps u` dirty-bits-without-push case breaks that. **Adopted Codex's unconditional-reapply** — the `CSI = Ps u` case is a real hole verified against `crates/vte` dispatch.

### Final agreed approach (revised per Round 1 Plan TPR — 6 codex findings accepted)

1. **Paired snapshot fields** on `Term`: `pre_command_kb_stack_snapshot: Option<VecDeque<KeyboardModes>>` + `inactive_pre_command_kb_stack_snapshot: Option<VecDeque<KeyboardModes>>` mirroring `keyboard_mode_stack`/`inactive_keyboard_mode_stack`.
2. **Contents-based snapshot of BOTH stacks at `;C`** (per Round 1 F1 + Round 2 F1 — fixes inactive-stack leak AND over-pop / max-depth eviction leak): `snapshot_keyboard_mode_stack` clones both stacks (`Some(keyboard_mode_stack.clone())` + `Some(inactive_keyboard_mode_stack.clone())`). **Restore BOTH at `;A`/`;D`** — replace each stack verbatim with its saved contents. Contents (not just depth) so we recover shell-held modes even when the child over-pops or pushes past `KEYBOARD_MODE_STACK_MAX_DEPTH` (evicting shell entries from the front via `dcs_push_keyboard_mode`'s ring-buffer semantics at `oriterm_core/src/term/handler/dcs.rs:44-47`). Cost: one `VecDeque<KeyboardModes>` clone per command boundary (~10-byte bounded payload); allocation once per `;C`, NOT in any hot path.
3. **Widen `dcs_set_keyboard_mode` visibility** (per Round 1 F4): change from `pub(super)` (only visible inside `term::handler`) to `pub(in crate::term)` at `oriterm_core/src/term/handler/dcs.rs:68` so `alt_screen.rs` and `shell_state/mod.rs` can call it. The toggle-alt-common DRIFT fix and the restore method both need this call site.
4. **`toggle_alt_common` updates** (two changes, both ship with this bug):
   a. `std::mem::swap(&mut self.pre_command_kb_stack_snapshot, &mut self.inactive_pre_command_kb_stack_snapshot)` alongside the existing stack swap.
   b. **Fix pre-existing DRIFT**: after the swap, call `self.dcs_set_keyboard_mode(new_top_or_NO_MODE, Replace)` so `TermMode::KITTY_KEYBOARD_PROTOCOL` reflects the newly active stack's top. Per Broken Window Policy this MUST ship with BUG-08-012 — the bug's alt-screen pathway runs through this drift.
5. **`restore_keyboard_mode_stack` — verbatim contents replace + unconditional top reapply** (per Round 1 F1 + Round 2 F1 refinement):
   ```rust
   pub fn restore_keyboard_mode_stack(&mut self) {
       if let Some(saved) = self.pre_command_kb_stack_snapshot.take() {
           self.keyboard_mode_stack = saved;
           let mode = self.keyboard_mode_stack.back().copied()
               .unwrap_or(vte::ansi::KeyboardModes::NO_MODE);
           self.dcs_set_keyboard_mode(mode, vte::ansi::KeyboardModesApplyBehavior::Replace);
       }
       if let Some(saved) = self.inactive_pre_command_kb_stack_snapshot.take() {
           self.inactive_keyboard_mode_stack = saved;
           // No reapply on inactive — next toggle_alt_common reapplies top
           // when the user switches to that screen. Reapplying inactive top
           // here would mutate TermMode bits that belong to the other screen.
       }
   }
   ```
   Covers truncation, `CSI = Ps u` same-depth mutations, inactive-stack alt-leak, child over-pop of shell-held modes, AND max-depth ring-buffer eviction of shell state. Verbatim contents replacement is simpler than truncate-and-reapply: no conditional truncate logic, no "depth ≤ saved" edge case — the snapshot IS the restored state.
6. **RIS/DECSTR: saved → `Some(VecDeque::new())`, NOT `None`** (per Round 1 F3 + Round 2 F1 contents-based shape): at `esc_reset_state` (RIS) and `soft_reset` (DECSTR) in `esc.rs:47-48, 107-108`, set BOTH paired fields to `Some(VecDeque::new())` alongside the existing stack clears. Reason: if RIS/DECSTR fires mid-command and a child later pushes kitty modes before crashing, the subsequent `;A` must still restore to an empty stack. `None` would leave child-pushed modes live past the prompt; `Some(empty)` cleans them. When RIS/DECSTR fires between commands, `Some(empty)` is a harmless no-op at next restore (replaces already-empty stack with empty); the next `;C` overwrites saved with a fresh cloned snapshot.
7. **OSC 133 + OSC 633 wiring**: snapshot on `b'C'`, restore on `b'A'` and `b'D'` in both `handle_osc133` and `handle_osc633`. Codex suggested a shared prompt-action helper — deferred as YAGNI for 3 new lines × 2 handlers; revisit if divergence grows.
8. **Test-file layout fix** (per Round 1 F5): convert `oriterm_core/src/term/alt_screen.rs` to directory-module `oriterm_core/src/term/alt_screen/mod.rs` + sibling `oriterm_core/src/term/alt_screen/tests.rs` before adding new alt-screen tests. Mandatory per `test-organization.md §Sibling tests.rs Pattern` — a module with tests MUST be a directory module.
9. **Test naming discipline** (per Round 1 F6): all test names in §2 follow strict `<subject>_<scenario>_<expected>` per `impl-hygiene.md §Test Function Naming`. Banned prefixes: `test_`, `should_`, `can_`, `is_`, `it_`; banned mid-name identifiers: plan/bug IDs. §2 is rewritten accordingly.
10. **Same-chunk parser-pass ordering tests** (per Round 1 F2 — narrowed): `handle_bytes` at `oriterm_mux/src/pane/io_thread/mod.rs:205-215` runs raw interceptor on the full chunk FIRST, then the high-level processor on the full chunk. The design is sound (raw snapshot-at-`;C` captures current stack depth; subsequent high-level pushes land above saved and are truncated at next `;A`), but the TDD matrix must pin this ordering invariant — see new cells in §2.

---

## 2. TDD — Test Matrix (revised per Round 1 Plan TPR — strict `<subject>_<scenario>_<expected>` naming, expanded for F1/F2/F3)

Write ALL tests BEFORE the fix. Verify they fail against current code. Names follow `<subject>_<scenario>_<expected>` per `impl-hygiene.md §Test Function Naming` — no `test_` / `should_` / `can_` / `is_` / `it_` prefixes, no plan/bug IDs in names.

### Exact failing case
- [ ] `keyboard_mode_stack_child_crash_on_osc_133_a_restores_to_snapshot_depth` — push one kitty mode, emit `OSC 133 ; C`, simulate child crash (no pop), emit `OSC 133 ; A`. Assert `keyboard_mode_stack.is_empty()` AND `!mode.intersects(TermMode::KITTY_KEYBOARD_PROTOCOL)`.

### Edge cases
- [ ] `keyboard_mode_stack_child_clean_exit_on_osc_133_d_restores_to_snapshot_depth` — same as above but child emitted `OSC 133 ; D` (clean command-done). Restore fires on `;D` even without `;A`.
- [ ] `keyboard_mode_stack_empty_at_c_and_a_stays_empty` — no push, `OSC 133 ; C`, `OSC 133 ; A`. Stack stays empty (no-op restore).
- [ ] `keyboard_mode_stack_shell_held_mode_at_c_preserved_after_a` — push one shell-owned mode, `OSC 133 ; C` (snapshots depth 1), push two child-owned modes, child exits, `OSC 133 ; A`. Assert `keyboard_mode_stack.len() == 1` and the shell-owned mode bits are active.
- [ ] `keyboard_mode_stack_three_command_cycles_each_restores_independently` — run 3 command cycles, each pushing 1 mode and not popping. After each `OSC 133 ; A` the stack is at its post-previous-cycle depth.
- [ ] `keyboard_mode_stack_in_band_csi_pop_without_prior_c_still_pops` — push two modes, emit one `CSI < u`, emit `OSC 133 ; A` without any prior `;C`. Only the in-band pop took effect (no restore — no snapshot).

### Cross-coverage — RIS / DECSTR (per Round 1 F3 — saved → Some(empty VecDeque), NOT None; contents-based per Round 2 F1)
- [ ] `keyboard_mode_stack_ris_during_command_sets_saved_to_empty_snapshot` — `OSC 133 ; C` (snapshot active+inactive), push child modes, RIS (`ESC c`). Assert BOTH paired fields are `Some(VecDeque::new())` (not `None`), both stacks empty.
- [ ] `keyboard_mode_stack_decstr_during_command_sets_saved_to_empty_snapshot` — same as above but via DECSTR (`CSI ! p`). Same assertions.
- [ ] `keyboard_mode_stack_ris_mid_command_then_child_push_then_a_cleans_pushes` — `OSC 133 ; C`, push mode, RIS, child later pushes two more modes and crashes, `OSC 133 ; A`. Assert stack empty and `KITTY_KEYBOARD_PROTOCOL` bits clear — saved=Some(empty) enabled cleanup of post-RIS child pushes.
- [ ] `keyboard_mode_stack_decstr_mid_command_then_child_push_then_a_cleans_pushes` — DECSTR variant of above.

### Cross-coverage — OSC 633 parallel (VS Code shell integration superset)
- [ ] `osc_633_c_snapshots_both_paired_depths` — OSC 633 `;C` snapshots both active and inactive paired fields. Same semantics as OSC 133.
- [ ] `osc_633_a_restores_both_paired_depths` — mirrors the OSC 133 case.
- [ ] `osc_633_d_restores_both_paired_depths` — mirrors the OSC 133;D case.

### Cross-coverage — alt-screen × paired per-screen snapshot (per Phase 1.75 consensus + Round 1 F1)
- [ ] `keyboard_mode_stack_snapshot_on_primary_then_alt_push_then_a_cleans_primary_not_alt` — snapshot on Primary (active depth 0), swap to Alt, push one mode on Alt, swap back to Primary, `OSC 133 ; A` on Primary. Assert Primary stack still empty AND inactive (Alt) stack has its push removed — both-stack snapshot caught the alt-side leak.
- [ ] `keyboard_mode_stack_snapshot_on_primary_child_alt_push_exit_alt_before_a_restores_inactive_from_snapshot` — **Round 1 F1's scenario**: snapshot on Primary at depth 1 (shell mode), child enters Alt (alt depth 0 snapshotted as inactive), pushes modeX on Alt, exits Alt without popping (swap back to Primary). Now active=Primary=[shell_mode], inactive=Alt=[modeX]. `OSC 133 ; A` on Primary. Assert: active stack restored to `[shell_mode]` (verbatim from snapshot), inactive stack restored to `[]` (verbatim from snapshot — modeX removed), TermMode bits reflect shell_mode. Pins the inactive-stack cleanup invariant.
- [ ] `keyboard_mode_stack_snapshot_and_restore_across_one_toggle_preserves_primary` — snapshot Primary at depth 1, enter Alt, push 2 modes on Alt, exit Alt (swap back), `OSC 133 ; A`. Primary stack restored from snapshot to `[shell_mode]`, TermMode bits reflect Primary-top.
- [ ] `keyboard_mode_stack_child_pops_one_and_crashes_restores_from_snapshot` — snapshot at depth 2 with `[A, B]`, child pops 1 (legitimate, stack becomes `[A]`), child then crashes. `OSC 133 ; A`: stack restored verbatim to `[A, B]` AND top-of-stack mode is reapplied — contents-based snapshot recovers shrink-then-crash scenarios.
- [ ] `keyboard_mode_stack_child_over_pops_shell_held_modes_restore_recovers_shell_state` — **Round 2 F1's scenario**: snapshot at depth 2 with `[shell_mode_A, shell_mode_B]`, child over-pops with `CSI < 5 u` (truncates to empty), child crashes. `OSC 133 ; A`: stack restored verbatim to `[shell_mode_A, shell_mode_B]`, TermMode bits reflect shell_mode_B. Pins contents-based recovery of over-popped shell state.
- [ ] `keyboard_mode_stack_child_push_past_max_depth_evicts_shell_mode_then_a_recovers_evicted_mode` — snapshot at depth 1 with `[shell_held_X]`, child pushes KEYBOARD_MODE_STACK_MAX_DEPTH (10) modes — the `dcs_push_keyboard_mode` ring-buffer evicts `shell_held_X` via `pop_front()`. Child crashes. `OSC 133 ; A`: stack restored verbatim to `[shell_held_X]` — contents-based snapshot recovers front-evicted shell state. Pins Round 2 F1 max-depth-overflow recovery.
- [ ] `keyboard_mode_stack_csi_equals_u_mutates_without_push_then_crash_then_a_reapplies_stack_top` — snapshot at depth 1 with shell_mode. Child emits `CSI = 31 u` (mutates TermMode bits via `dcs_set_keyboard_mode` Replace without touching stack). Child crashes. `OSC 133 ; A`. TermMode bits match shell_mode, NOT the child's mutation. Pins Phase 1.75 Codex's unconditional-reapply refinement against `CSI = Ps u` dirty-bits leak.

### Cross-coverage — `toggle_alt_common` drift fix (per Phase 1.75 Broken Window Policy hit)
- [ ] `toggle_alt_common_swaps_nonempty_stacks_reapplies_new_active_top` — Primary stack `[modeA]`, Alt stack `[modeB]`. Toggle via `?1049h`. TermMode bits reflect modeB. Pins DRIFT fix.
- [ ] `toggle_alt_common_swaps_to_empty_alt_clears_mode_bits` — Primary stack `[modeA]` (KITTY_KEYBOARD_PROTOCOL set), Alt stack empty. Toggle via `?1049h`. KITTY_KEYBOARD_PROTOCOL bits cleared — reapply pushes `NO_MODE`.
- [ ] `toggle_alt_common_also_swaps_paired_snapshots` — primary's snapshot = `Some([X])`, alt's snapshot = `None`; toggle. After: active's snapshot = `None` (was alt's), inactive's snapshot = `Some([X])` (was primary's). Pins the paired-snapshot swap invariant.

### Cross-coverage — same-chunk parser-pass ordering (per Round 1 F2)
- [ ] `osc_133_c_and_csi_push_same_chunk_snapshot_captures_pre_push_depth` — PTY chunk contains `\x1b]133;C\x1b\\` immediately followed by `\x1b[>31u` in one byte slice. Verify raw interceptor snapshots at pre-push depth, then high-level push lands above saved. Next `;A` truncates the push. Pins the two-pass ordering invariant in `handle_bytes`.
- [ ] `csi_push_and_osc_133_c_same_chunk_snapshot_still_captures_pre_chunk_depth` — reverse order: `\x1b[>31u` then `\x1b]133;C\x1b\\` in one chunk. Raw interceptor still fires first, snapshot captures stack BEFORE high-level processes the push. Document that subsequent `;A` removes the push. Pins the raw-first-then-high-level semantics as spec.

### Semantic pin (end-to-end, user-visible symptom)
- [ ] `legacy_key_encoding_after_child_crash_produces_raw_ascii_not_csi_u` — after repro (push kitty mode, `OSC 133 ; C`, child crash, `OSC 133 ; A`), a `Key::Character("a")` event through `encode_key()` produces `b"a"`, NOT a kitty `CSI u` payload. Lives in `oriterm/src/key_encoding/tests/kitty_precedence.rs`. This is the headline symptom pin — ties the fix to the user-visible bug.

### Negative pin
- [ ] `keyboard_mode_stack_osc_133_a_without_prior_c_does_not_modify_stack` — without `OSC 133 ; C` (shell integration disabled), `OSC 133 ; A` does NOT clear the stack. Restore is snapshot-gated, not blindly clearing.
- [ ] `keyboard_mode_stack_restore_without_snapshot_leaves_paired_fields_none` — explicit assertion that after `OSC 133 ; A` without prior `;C`, `pre_command_kb_stack_snapshot` AND `inactive_pre_command_kb_stack_snapshot` are both still `None` and both stacks untouched.

### Verify tests fail before fix
- [ ] Every new test fails against current code. The bulk should fail at the `assert!(keyboard_mode_stack.is_empty())` (or equivalent depth / mode-bit) assertion after the simulated child crash.

---

## 2.5 Fix Plan TPR Findings

Adversarial review of this fix PLAN (§1–§3) before implementation. Ran AFTER `/tp-help` consensus (§1.5) and plan finalization (§2) but BEFORE writing tests or code.

**Gate:** Mandatory — high severity + complexity-elevated subsystem (VTE / core grid — `oriterm_core/src/term/`).

**Scratch dir (Round 1)**: `/tmp/tpr-round-ori_term-c5ovo2IV`.

### Round 1 — 2026-04-24

- **Gemini**: clean, 0 findings. Summary: "Plan for BUG-08-012 is sound and rigorously grounded in project invariants."
- **Codex**: findings — 3 high + 2 medium + 1 low. All actionable, all verified against code, all folded into the plan revisions above (§1.5 "Final agreed approach" items 2, 3, 5, 6, 8, 9, 10 all trace to a Round 1 finding):
  - **F1 [high]** *Alt-screen child pushes can survive in the inactive stack* — verified. Initially fixed by snapshotting BOTH paired depths at `;C` and truncating both at `;A`/`;D`. Round 2 F1 further refined this to **contents-based snapshot** (§1.5 item 2/5/6 after Round 2; §2 `snapshot_on_primary_child_alt_push_exit_alt_before_a_restores_inactive_from_snapshot` test; revised `restore_keyboard_mode_stack` in §3).
  - **F2 [high]** *Full raw-pass ordering can mis-snapshot same-chunk CSI/OSC* — narrowed and accepted. `handle_bytes` at `oriterm_mux/src/pane/io_thread/mod.rs:205-215` runs raw-then-high-level on the full chunk; the design is sound (raw snapshot captures pre-push depth, subsequent high-level pushes land above saved and are truncated at next `;A`) but the invariant was undertested. Added `osc_133_c_and_csi_push_same_chunk_snapshot_captures_pre_push_depth` + `csi_push_and_osc_133_c_same_chunk_snapshot_still_captures_pre_chunk_depth` in §2. Codex's broader recommendation to re-architect the parser chain was rejected — the two-pass architecture is the correct SSOT for raw vs high-level responsibilities.
  - **F3 [high]** *DECSTR during a command can disable later prompt restore* — verified + accepted. RIS/DECSTR now set BOTH paired fields to `Some(VecDeque::new())` (not `None`; contents-based shape per Round 2 F1), preserving command-boundary semantics at a clean empty snapshot. If RIS/DECSTR fires mid-command and a child later pushes before crashing, `;A` still restores to an empty stack (§1.5 item 6, §2 RIS/DECSTR tests, §3 RIS/DECSTR implementation step).
  - **F4 [medium]** *Alt-screen plan calls a handler-private helper* — verified. `dcs_set_keyboard_mode` is `pub(super)` at `oriterm_core/src/term/handler/dcs.rs:68`, only visible within `term::handler`. The toggle-drift fix in `alt_screen.rs` and the restore reapply in `shell_state/mod.rs` both need to call it from outside `term::handler`. Widened to `pub(in crate::term)` (§1.5 item 3, §3 visibility-widen step).
  - **F5 [medium]** *Proposed alt-screen test file violates sibling-test layout* — verified against `.claude/rules/test-organization.md §Sibling tests.rs Pattern`. Convert `alt_screen.rs` → `alt_screen/mod.rs` via `git mv` before adding `alt_screen/tests.rs` (§1.5 item 8, §3 directory-module conversion step).
  - **F6 [low]** *Test names violated subject_scenario_expected* — verified against `.claude/rules/impl-hygiene.md §Test Function Naming`. All test names in §2 rewritten to strict `<subject>_<scenario>_<expected>` form, banned prefixes eliminated (§2 full rewrite).
- **Verification artifacts**: `codex-report.txt`, `gemini-report.txt` in scratch dir. Shadow-edit check clean.
- **Round 1 outcome**: all 6 codex findings verified and folded into the plan; gemini clean corroborates baseline soundness.

### Round 2 — 2026-04-24

**Scratch dir**: `/tmp/tpr-round-ori_term-MnA8pGQS`.

- **Gemini**: clean, 0 findings. Summary: "The revised approach correctly handles alt-screen inactive stack leaks (F1), pins raw-pass ordering invariants (F2), and preserves command-boundary semantics across soft resets (F3). Visibility and layout concerns (F4/F5) are addressed. The TDD matrix is comprehensive."
- **Codex**: 3 findings — 2 high + 1 medium. Verified and triaged:
  - **Round 2 F1 [high]** *Depth-only restore does not preserve shell-held modes after pops* — verified against code. Scenario: child over-pops shell-held modes (`CSI < N u` with N larger than child's push count) truncates shell state from the stack via `dcs_pop_keyboard_modes` at `oriterm_core/src/term/handler/dcs.rs:52-64`; OR child pushes past `KEYBOARD_MODE_STACK_MAX_DEPTH` (10) and `dcs_push_keyboard_mode` evicts shell-held modes from the front (`pop_front` at dcs.rs:44-47). Depth-based snapshot cannot recover — the entries at the saved depth are gone. **Accepted**: switched to **contents-based snapshot** (`Option<VecDeque<KeyboardModes>>`) — clone at `;C`, replace at `;A`/`;D`. Adds one bounded clone (≤10 × 1 byte) per command boundary; not a hot path. §1.5 items 2/5/6 revised; §2 gained `child_over_pops_shell_held_modes_restore_recovers_shell_state` + `child_push_past_max_depth_evicts_shell_mode_then_a_recovers_evicted_mode`; §3 Implementation revised for contents-based semantics.
  - **Round 2 F2 [high]** *Reverse-order same-chunk test strips a pre-C shell push* — verified test behavior, **rejected** the recommendation. Codex's proposed scenario (`\x1b[>31u\x1b]133;C\x1b\\` with no content between) does not occur in real shells — prompt markers emit as `;A → prompt text → ;B → user input → ;C → output`; `CSI > u` is emitted at shell init or in dedicated integration-script lines, never adjacent to `;C`. Codex's architectural fix (byte-order-preserving interleaved dispatch) would require rewiring the dual-parser chain for a non-realistic scenario. **Disagreement recorded**: current `raw-first-then-high-level` architecture is the correct SSOT for raw vs high-level dispatch; the test pins THAT invariant. If a future bug surfaces real shell output matching the pathological pattern, re-open the architecture.
  - **Round 2 F3 [medium]** *Alt-screen tests remain split after adding sibling tests* — verified. The existing `swap_alt_preserves_keyboard_mode_stacks` at `oriterm_core/src/term/tests.rs:128` and eight other alt-screen tests live in `term/tests.rs`. Per `.claude/rules/test-organization.md §Sibling tests.rs Pattern` rule 2 ("One `tests.rs` per source file"), leaving those in `term/tests.rs` after creating `term/alt_screen/tests.rs` would violate the rule. **Accepted**: §3 implementation step now lists all nine existing alt-screen tests and mandates their migration to `term/alt_screen/tests.rs` during the `git mv`.
- **Verification artifacts**: `codex-report.txt`, `gemini-report.txt` in scratch dir. Shadow-edit check clean.
- **Round 2 outcome**: 2 of 3 codex findings accepted and folded (F1 contents-based, F3 test migration); 1 rejected with recorded disagreement (F2 architecture). Gemini clean. Convergence reached after Round 2 — the design changes from F1 resolve the last substantive correctness concern (contents-based recovers shell state across over-pop / max-depth-eviction). Phase 2.5 complete.

---

## 3. Implementation (revised per Phase 1.75 consensus)

- [ ] **Add PAIRED contents-snapshot fields to `Term`** in `oriterm_core/src/term/mod.rs` (per Round 2 F1 — contents-based, not depth-based; closes over-pop and max-depth-eviction leaks). Both initialized to `None` in `Term::new()`.
  ```rust
  /// Full snapshot of `keyboard_mode_stack` taken at OSC 133 ; C
  /// (command-start) on the ACTIVE screen. Restored on the next OSC 133 ; A
  /// or ; D so kitty keyboard modes pushed, popped, OR evicted (at
  /// KEYBOARD_MODE_STACK_MAX_DEPTH) by a subprocess that exited without
  /// cleanly popping don't persist or erase shell state. `None` means no
  /// snapshot active for this screen.
  ///
  /// Contents-based (not depth-based) so that a child that over-pops
  /// shell-held modes or pushes past max-depth (evicting shell modes from
  /// the front) is fully reversed at the next prompt boundary. Paired
  /// with `inactive_pre_command_kb_stack_snapshot`; swapped alongside
  /// the stacks in `toggle_alt_common`. See BUG-08-12.
  pre_command_kb_stack_snapshot: Option<VecDeque<KeyboardModes>>,
  inactive_pre_command_kb_stack_snapshot: Option<VecDeque<KeyboardModes>>,
  ```

- [ ] **Widen `dcs_set_keyboard_mode` visibility** (per Round 1 F4): at `oriterm_core/src/term/handler/dcs.rs:68`, change `pub(super) fn dcs_set_keyboard_mode` to `pub(in crate::term) fn dcs_set_keyboard_mode`. The `pub(super)` scope is `term::handler`; callers in `alt_screen.rs` (toggle-drift fix) and `shell_state/mod.rs` (restore reapply) both live under `term::` but NOT under `term::handler`, so they cannot call `pub(super)` items. `pub(in crate::term)` widens to the whole `term::` module tree without exposing the helper outside `oriterm_core`.

- [ ] **Add methods to `Term` in `oriterm_core/src/term/shell_state/mod.rs`**: clone BOTH stack contents at snapshot + verbatim-restore at `;A`/`;D` with unconditional top-of-active-stack mode reapply.
  ```rust
  /// Clone BOTH active and inactive keyboard-mode stack contents so a
  /// subsequent OSC 133 `;A` / `;D` can restore them verbatim. Contents
  /// (not just depth) so we recover shell-held modes even when the child
  /// over-pops or pushes past KEYBOARD_MODE_STACK_MAX_DEPTH. Allocates
  /// up to 2 × KEYBOARD_MODE_STACK_MAX_DEPTH * size_of::<KeyboardModes>()
  /// per command boundary — ~20 bytes, infrequent (once per command), not
  /// a hot path. See BUG-08-12.
  pub fn snapshot_keyboard_mode_stack(&mut self) {
      self.pre_command_kb_stack_snapshot = Some(self.keyboard_mode_stack.clone());
      self.inactive_pre_command_kb_stack_snapshot =
          Some(self.inactive_keyboard_mode_stack.clone());
  }

  /// If a snapshot is active, replace BOTH stacks with the snapshotted
  /// contents and unconditionally reapply the top-of-active-stack mode.
  /// The unconditional reapply covers `CSI = Ps u` same-depth mutations
  /// that modify `TermMode::KITTY_KEYBOARD_PROTOCOL` bits via
  /// `dcs_set_keyboard_mode` without touching the stack (see
  /// `crates/vte/src/ansi/dispatch/csi/mod.rs:333-340`). Inactive stack
  /// is restored but not reapplied — `toggle_alt_common` reapplies top
  /// when the user switches to that screen. See BUG-08-12.
  pub fn restore_keyboard_mode_stack(&mut self) {
      if let Some(saved) = self.pre_command_kb_stack_snapshot.take() {
          self.keyboard_mode_stack = saved;
          let mode = self
              .keyboard_mode_stack
              .back()
              .copied()
              .unwrap_or(vte::ansi::KeyboardModes::NO_MODE);
          self.dcs_set_keyboard_mode(mode, vte::ansi::KeyboardModesApplyBehavior::Replace);
      }
      if let Some(saved) = self.inactive_pre_command_kb_stack_snapshot.take() {
          self.inactive_keyboard_mode_stack = saved;
      }
  }
  ```
  Semantics: snapshot clones BOTH current stacks. Restore is a no-op when both saved are `None`. When active-saved is `Some`, restore replaces the active stack with the saved contents AND reapplies top mode (covers truncation + over-pop + `CSI = Ps u` dirty bits). When inactive-saved is `Some`, restore replaces the inactive stack (covers alt-enter-push-exit-before-A leak) but does NOT reapply — mode bits belong to the other screen. The single allocation per command boundary is the `Option<VecDeque>` taking ownership of the cloned contents; `VecDeque::clone` is bounded by `KEYBOARD_MODE_STACK_MAX_DEPTH` (10 entries × 1 byte KeyboardModes bitfield).

- [ ] **RIS/DECSTR set BOTH paired fields to `Some(VecDeque::new())`** (per Round 1 F3 — now contents-based per Round 2 F1) at `oriterm_core/src/term/handler/esc.rs`. At both `esc_reset_state` (RIS, after `keyboard_mode_stack.clear()` at line 47-48) and `soft_reset` (DECSTR, after the equivalent at line 107-108), add:
  ```rust
  self.pre_command_kb_stack_snapshot = Some(VecDeque::new());
  self.inactive_pre_command_kb_stack_snapshot = Some(VecDeque::new());
  ```
  NOT `None`. Reason: if RIS/DECSTR fires mid-command and the child later pushes kitty modes before crashing, the next `;A` must still restore to an empty stack. `None` would leave post-reset child pushes live past the prompt; `Some(empty)` cleans them. Between commands (no snapshot active), `Some(empty)` is a harmless no-op at next restore (replaces already-empty stack with empty).

- [ ] **`toggle_alt_common` updates** in `oriterm_core/src/term/alt_screen.rs` — TWO changes, both ship with this bug:
  ```rust
  fn toggle_alt_common(&mut self) {
      self.mode.toggle(TermMode::ALT_SCREEN);
      std::mem::swap(
          &mut self.keyboard_mode_stack,
          &mut self.inactive_keyboard_mode_stack,
      );
      // NEW (BUG-08-12): swap the paired snapshot field so a command-boundary
      // snapshot taken on one screen only fires restore on that screen.
      std::mem::swap(
          &mut self.pre_command_kb_stack_snapshot,
          &mut self.inactive_pre_command_kb_stack_snapshot,
      );
      std::mem::swap(&mut self.saved_charset, &mut self.inactive_saved_charset);
      std::mem::swap(
          &mut self.saved_origin_mode,
          &mut self.inactive_saved_origin_mode,
      );
      // NEW (BUG-08-12, pre-existing DRIFT fix): reapply the newly-active
      // stack's top-of-mode so `TermMode::KITTY_KEYBOARD_PROTOCOL` reflects
      // the active screen. Without this, kitty mode bits leak across
      // `?1049h/l` toggles — the exact pathway by which this bug manifests
      // in alt-screen programs even with OSC 133 restore in place.
      let new_top = self
          .keyboard_mode_stack
          .back()
          .copied()
          .unwrap_or(vte::ansi::KeyboardModes::NO_MODE);
      self.dcs_set_keyboard_mode(new_top, vte::ansi::KeyboardModesApplyBehavior::Replace);
      self.grid_mut().dirty_mut().mark_all();
  }
  ```
  The `dcs_set_keyboard_mode(new_top, Replace)` call is the pre-existing-DRIFT fix. Per Broken Window Policy this MUST ship with BUG-08-012 — alt-screen is part of this bug's blast radius, not a separate issue.

- [ ] **Wire OSC 133 in `oriterm_mux/src/shell_integration/interceptor.rs`** `handle_osc133`:
  - `b'A'` arm: call `self.term.restore_keyboard_mode_stack()` BEFORE `set_prompt_state(PromptStart)`. `;A` is the safety net when `;D` didn't fire (child crashed).
  - `b'C'` arm: call `self.term.snapshot_keyboard_mode_stack()` AFTER `set_command_start(now)`.
  - `b'D'` arm: call `self.term.restore_keyboard_mode_stack()` alongside `finish_command`. Clean-path restore — most programs reach `;D`.

- [ ] **Mirror in OSC 633**: `handle_osc633`'s `A`/`C`/`D` arms receive the same three calls (VS Code superset). Deferred shared helper per Phase 1.75 (YAGNI for 3 lines × 2 handlers; revisit if OSC 633 diverges further).

- [ ] **Documentation**: rustdoc on both paired fields explaining the OSC 133 lifecycle + alt-screen swap; rustdoc on the two methods citing BUG-08-12; rustdoc inline comment on the `toggle_alt_common` pre-existing-DRIFT fix.

- [ ] **Convert `alt_screen.rs` to directory module AND migrate existing alt-screen tests** (per Round 1 F5 + Round 2 F3): TWO sub-steps.
  1. `git mv oriterm_core/src/term/alt_screen.rs oriterm_core/src/term/alt_screen/mod.rs` to preserve blame. Mandatory per `test-organization.md §Sibling tests.rs Pattern` — "A module with tests MUST be a directory module."
  2. Migrate existing alt-screen tests from `oriterm_core/src/term/tests.rs` into the new `oriterm_core/src/term/alt_screen/tests.rs`. The following tests in `term/tests.rs` are owned by `alt_screen` and MUST move in the same commit:
     - `swap_alt_switches_to_alt_grid_and_back` (line ~71)
     - `swap_alt_preserves_keyboard_mode_stacks` (line ~128)
     - `damage_swap_alt_marks_all_dirty` (line ~629)
     - `damage_swap_alt_back_marks_all_dirty` (line ~641)
     - `selection_dirty_set_by_swap_alt` (line ~1060)
     - `selection_dirty_set_by_alt_screen_via_decset` (line ~1199)
     - `resize_before_alt_screen_no_crash` (line ~2013)
     - `alt_screen_reentry_correct` (line ~2026)
     - `resize_on_alt_screen_then_snapshot` (line ~2205)
     - Any other `swap_alt*` / `alt_screen_*` tests surfacing at migration time — `grep -nE 'fn (swap_alt|alt_screen|damage_swap_alt|selection_dirty_set_by_alt|resize_(before_)?(on_)?alt_screen)' oriterm_core/src/term/tests.rs` is the authoritative list.
  3. Keep the new Round-0 / Round-1 / Round-2 regression tests AT the migrated location (`oriterm_core/src/term/alt_screen/tests.rs`). Do NOT leave alt-screen tests split across two files — Round 2 F3 specifically flagged that as a test-organization.md violation.

- [ ] **Regression test locations** (Phase 3):
  - `oriterm_mux/src/shell_integration/tests.rs` — OSC 133/633 dispatch tests, including same-chunk parser-pass ordering tests and the per-screen inactive-stack cleanup tests + over-pop / max-depth-eviction tests.
  - `oriterm/src/key_encoding/tests/kitty_precedence.rs` — `legacy_key_encoding_after_child_crash_produces_raw_ascii_not_csi_u` semantic pin.
  - `oriterm_core/src/term/handler/tests/esc.rs` — RIS/DECSTR tests pinning `saved = Some(VecDeque::new())` (NOT `None`) for both paired fields, including the mid-command child-push tests.
  - `oriterm_core/src/term/alt_screen/tests.rs` (created by the directory-module conversion above, populated by the migration step above) — existing alt-screen tests + the three new `toggle_alt_common` tests pinning the DRIFT fix + paired-snapshot swap.

---

## R. Third Party Review Findings

TPR findings raised against this fix are recorded here by the executor (Claude) during Phase 5.

*(Initially empty — populated during Phase 5 completion checklist.)*

---

## 4. Completion Checklist

- [ ] All new tests pass unchanged after fix (no test modifications needed)
- [ ] Matrix completeness verified — every cell in the OSC 133 / OSC 633 × alt-screen × snapshot-state grid has a test
- [ ] Debug AND release builds pass
- [ ] Windows cross-compile green (`cargo build --target x86_64-pc-windows-gnu`)
- [ ] `timeout 150 ./test-all.sh` green — no regressions
- [ ] `./clippy-all.sh` green
- [ ] `./build-all.sh` green
- [ ] `cargo test -p oriterm_core` + `cargo test -p oriterm_mux` + `cargo test -p oriterm` green
- [ ] `/commit-push` — commit all changes before review
- [ ] Plan TPR (Phase 2.5) completed — see §2.5 above
- [ ] `/tpr-review` (Phase 5 — code review) passed
- [ ] `/impl-hygiene-review` passed — after code TPR is clean
- [ ] `/improve-tooling` retrospective completed
- [ ] Bug entry in `plans/bug-tracker/section-08-core-terminal.md` updated: `- [x]` with `Resolved: fixed on {YYYY-MM-DD}. See plans/bug-tracker/fix-BUG-08-012.md.`
- [ ] Fix section frontmatter `status` updated to `complete`
- [ ] Bug-tracker `00-overview.md` Quick Reference open bug count decremented for section 08
- [ ] Final `/commit-push` — commit closure artifacts

**Exit Criteria:** Running the repro (push kitty modes via DCS, emit `OSC 133 ; C`, send bytes that would push more modes without matching pops, emit `OSC 133 ; A`) results in an empty `keyboard_mode_stack` and `!mode.intersects(TermMode::KITTY_KEYBOARD_PROTOCOL)` — proven by the `keyboard_mode_stack_restored_after_child_crash_on_osc_133_a` semantic pin plus the `typed_key_after_child_crash_routes_through_legacy_encoder` end-to-end pin. Full workspace test suite (`./test-all.sh`) remains green. Shell integration paths that skip `OSC 133 ; C` have their stacks untouched (negative pin). Live validation: run `notcurses-demo` in a bash shell with OSC 133 integration, exit, verify typing produces normal characters, not `CSI u` fragments.
