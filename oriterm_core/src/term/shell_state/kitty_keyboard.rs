//! Kitty keyboard protocol snapshot / restore for OSC 133 / OSC 633
//! shell-integration boundaries.
//!
//! Separate file (vs `mod.rs`) to keep the `shell_state` parent under
//! the 500-line file-size cap. All public accessors and the snapshot /
//! restore state machine for `keyboard_mode_stack`,
//! `inactive_keyboard_mode_stack`, `pre_command_kb_stack_snapshot`,
//! `inactive_pre_command_kb_stack_snapshot`,
//! `pre_command_kb_mode_bits_snapshot`, `inactive_keyboard_mode_bits`,
//! and `inactive_pre_command_kb_mode_bits_snapshot` live here.
//!
//! See BUG-08-012 (`bug-tracker/plans/completed/BUG-08-012/00-overview.md`) for the full
//! contract: paired per-screen snapshots + live per-screen bits.

use std::collections::VecDeque;

use vte::ansi::{KeyboardModes, KeyboardModesApplyBehavior};

use super::super::Term;
use crate::effect::sink::EffectSink;

impl<S: EffectSink> Term<S> {
    // -- Kitty keyboard mode stack accessors --

    /// Current keyboard mode stack (Kitty keyboard protocol).
    pub fn keyboard_mode_stack(&self) -> &VecDeque<KeyboardModes> {
        &self.keyboard_mode_stack
    }

    /// Inactive-screen keyboard mode stack (swapped on alt-screen toggle).
    pub fn inactive_keyboard_mode_stack(&self) -> &VecDeque<KeyboardModes> {
        &self.inactive_keyboard_mode_stack
    }

    /// Pre-command snapshot of the active-screen keyboard mode stack
    /// (taken at OSC 133 ; C, consumed at OSC 133 ; A / ; D).
    ///
    /// `None` means no snapshot is active for this screen. `Some(deque)`
    /// holds the verbatim contents of [`Self::keyboard_mode_stack`] at
    /// the moment the pre-command snapshot was taken. See BUG-08-012.
    pub fn pre_command_kb_stack_snapshot(&self) -> Option<&VecDeque<KeyboardModes>> {
        self.pre_command_kb_stack_snapshot.as_ref()
    }

    /// Pre-command snapshot of the inactive-screen keyboard mode stack.
    ///
    /// See [`Self::pre_command_kb_stack_snapshot`] for semantics. Paired
    /// with that field and swapped alongside the stacks on alt-screen
    /// toggle so a snapshot taken on one screen only fires restore on
    /// that screen.
    pub fn inactive_pre_command_kb_stack_snapshot(&self) -> Option<&VecDeque<KeyboardModes>> {
        self.inactive_pre_command_kb_stack_snapshot.as_ref()
    }

    /// Pre-command snapshot of the active-screen Kitty-keyboard-protocol
    /// `TermMode` bits. Paired with [`Self::pre_command_kb_stack_snapshot`]
    /// — preserves shell-held kitty bits set via `CSI = Ps u` that never
    /// enter the stack. See BUG-08-012 TPR round-1 F1.
    pub fn pre_command_kb_mode_bits_snapshot(&self) -> Option<KeyboardModes> {
        self.pre_command_kb_mode_bits_snapshot
    }

    /// Live Kitty-keyboard-protocol bits for the INACTIVE screen. Swapped
    /// with the active bits (the `TermMode::KITTY_KEYBOARD_PROTOCOL`
    /// subset of `self.mode`) in `toggle_alt_common` so set-only kitty
    /// bits survive alt-screen toggles. See BUG-08-012 TPR round-3.
    pub fn inactive_keyboard_mode_bits(&self) -> KeyboardModes {
        self.inactive_keyboard_mode_bits
    }

    /// Paired inactive-screen bits snapshot — see
    /// [`Self::pre_command_kb_mode_bits_snapshot`]. Swapped alongside
    /// the paired stacks in `toggle_alt_common` so the snapshot stays
    /// with the screen where `;C` was emitted. BUG-08-012 TPR round-4.
    pub fn inactive_pre_command_kb_mode_bits_snapshot(&self) -> Option<KeyboardModes> {
        self.inactive_pre_command_kb_mode_bits_snapshot
    }

    // -- Kitty keyboard mode stack snapshot / restore (OSC 133 C/A/D) --

    /// Clone BOTH active and inactive keyboard-mode stack contents AND
    /// capture the current Kitty-keyboard-protocol `TermMode` bits so a
    /// subsequent OSC 133 `;A` / `;D` (or OSC 633 `;A` / `;D`) can
    /// restore them verbatim.
    ///
    /// Contents-based (not just depth-based) so we recover shell-held
    /// modes even when the child over-pops via `CSI < N u` or pushes past
    /// [`crate::term::KEYBOARD_MODE_STACK_MAX_DEPTH`] and `pop_front`
    /// evicts shell-held entries. Paired bits snapshot (not derived from
    /// stack top) so shells that set kitty modes via `CSI = Ps u` without
    /// pushing are also restored — see BUG-08-012 TPR round-1 F1. Allocates
    /// up to `2 × KEYBOARD_MODE_STACK_MAX_DEPTH × size_of::<KeyboardModes>()`
    /// plus 2 bytes of bits snapshot per command boundary — infrequent
    /// (once per command), not a hot path. See BUG-08-012.
    pub fn snapshot_keyboard_mode_stack(&mut self) {
        self.pre_command_kb_stack_snapshot = Some(self.keyboard_mode_stack.clone());
        self.inactive_pre_command_kb_stack_snapshot =
            Some(self.inactive_keyboard_mode_stack.clone());
        self.pre_command_kb_mode_bits_snapshot = Some(KeyboardModes::from(self.mode));
        // Inactive bits snapshot mirrors the live per-screen state so the
        // command-boundary snapshot travels with its owning screen across
        // alt-screen toggles. Runtime state is tracked in
        // `inactive_keyboard_mode_bits`; the snapshot below is the restore
        // target for the inactive screen's `;A`/`;D` if it ever fires.
        self.inactive_pre_command_kb_mode_bits_snapshot = Some(self.inactive_keyboard_mode_bits);
    }

    /// If a snapshot is active, replace BOTH stacks with the snapshotted
    /// contents and apply the snapshotted `TermMode` bits directly.
    ///
    /// Applying the paired BITS snapshot (rather than deriving from
    /// stack top) preserves shell-held kitty state set via `CSI = Ps u`
    /// WITHOUT pushing — the top of an empty stack is `NO_MODE`, which
    /// would silently clear a shell's set-only kitty mode at prompt
    /// boundary. Also covers `CSI = Ps u` same-depth mutations during a
    /// command (child changes bits without touching the stack; restore
    /// reverts both stack AND bits). The inactive bits snapshot is
    /// applied to `inactive_keyboard_mode_bits` to revert mid-command
    /// mutations on the inactive screen (BUG-08-012 TPR round-5).
    pub fn restore_keyboard_mode_stack(&mut self) {
        if let Some(saved) = self.pre_command_kb_stack_snapshot.take() {
            self.keyboard_mode_stack = saved;
        }
        if let Some(saved_bits) = self.pre_command_kb_mode_bits_snapshot.take() {
            self.dcs_set_keyboard_mode(saved_bits, KeyboardModesApplyBehavior::Replace);
        }
        if let Some(saved) = self.inactive_pre_command_kb_stack_snapshot.take() {
            self.inactive_keyboard_mode_stack = saved;
        }
        // Apply the inactive bits snapshot to the live inactive bits
        // field — restores inactive-screen kitty bits that were mutated
        // during the command (e.g. child `CSI = Ps u` inside alt-screen
        // while shell's `;C` was active on primary). Without this apply,
        // the alt screen's mid-command bit mutations would persist past
        // the shell's prompt-boundary restore. BUG-08-012 TPR round-5.
        if let Some(saved_bits) = self.inactive_pre_command_kb_mode_bits_snapshot.take() {
            self.inactive_keyboard_mode_bits = saved_bits;
        }
    }
}
