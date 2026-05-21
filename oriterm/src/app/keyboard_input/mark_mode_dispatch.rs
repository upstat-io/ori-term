//! Mark-mode key-event dispatch — trait-based side-effect surface so the
//! wiring between mark-mode resource gating, snapshot+grid construction,
//! `mark_mode::handle_mark_mode_key`, and post-dispatch state mutations
//! is matrix-testable headlessly.
//!
//! Production: `impl MarkModeSink for App` delegates to `&mut self` methods.
//! Tests: `RecordingSink` in `tests.rs` records every call without
//! constructing `App`. Per & External Resource
//! Abstraction. Mirrors the pattern in
//! `term_repo/oriterm/src/app/mouse_report/wheel_dispatch.rs`.

use winit::event::ElementState;
use winit::keyboard::ModifiersState;

use oriterm_core::Selection;
use oriterm_mux::{MarkCursor, PaneId};

use super::super::App;
use super::super::mark_mode::{
 MarkAction, MarkModeKeyContext, MarkModeKeyEvent, MarkModeResult, SelectionUpdate,
};

/// Snapshot of which mark-mode resources are currently available.
/// Mark mode requires three resources to process a key: a mux to route
/// scroll/refresh, a per-pane mark cursor, and a pane snapshot to read
/// the grid contents. Any of these can become `None` during a race
/// (pane init, mark cursor eviction, mux disconnect mid-mark-mode).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MarkModeResources {
 pub(super) mux_present: bool,
 pub(super) cursor_present: bool,
 pub(super) snapshot_present: bool,
}

/// Pure decision predicate. Returns `true` when mark mode must be exited
/// because a required resource is missing.
/// The caller responds by calling `exit_mark_mode` and returning `false`
/// so the keystroke flows on to normal key dispatch (keybinding lookup,
/// then `encode_key_to_pty`) instead of being silently swallowed.
/// Returns `false` only when all three resources are present.
/// Note: the cursor-missing path is technically unreachable in the
/// single-threaded `App` context (no mutation of `mark_cursors` between
/// `is_mark_mode` and `pane_mark_cursor`). The recovery path is
/// intentional defense-in-depth, deliberately diverging from
/// the Defensive-Code-for-Impossible-States rule
/// — silently swallowing keystrokes when an "unreachable" precondition
/// fails is the bug this guard exists to prevent. See
/// "Reviewer notes adopted" for the full justification.
#[must_use]
pub(super) fn mark_mode_should_exit(resources: MarkModeResources) -> bool {
 !resources.mux_present || !resources.cursor_present || !resources.snapshot_present
}

/// Inputs to [`dispatch_mark_mode`] — one cohesive carrier for the
/// dispatch decision surface. Six fields: data-carrier struct exempt
/// from the Hygiene rule >4 rule (which applies
/// to function/method signatures, not struct field counts).
pub(super) struct MarkModeDispatch {
 /// The key event whose dispatch this struct describes.
 pub event: MarkModeKeyEvent,
 /// Pressed vs Released — Pressed is the only state that runs the
 /// dispatch chain; Released falls through to consume-all.
 pub event_state: ElementState,
 /// Modifiers for the dispatch event.
 pub modifiers: ModifiersState,
 /// Active pane for this dispatch — `None` short-circuits with `false`.
 pub active_pane_id: Option<PaneId>,
 /// Whether this is a key-repeat event. Carried for matrix-test
 /// orthogonality even though `dispatch_mark_mode` itself does not
 /// branch on it (existing branch keys only on `event_state`).
 #[expect(
 dead_code,
 reason = "carried for matrix-test orthogonality witness — dispatch deliberately ignores it"
 )]
 pub event_repeat: bool,
 /// Mark mode active flag, captured at the App boundary by
 /// `App::is_mark_mode` BEFORE `dispatch_mark_mode` runs.
 pub mark_mode_active: bool,
}

/// Bundled inputs to [`MarkModeSink::dispatch_mark_mode_key`].
/// Keeps the trait method signature within the `>4 parameters → struct`
/// Parameter Hygiene rule. Owns `MarkModeKeyEvent` by value (not borrow)
/// so [`RecordingSink`] tests can construct test inputs without a `'a`
/// lifetime parameter on the sink.
pub(super) struct MarkKeyInput {
 pub pane_id: PaneId,
 pub cursor: MarkCursor,
 pub selection: Option<Selection>,
 pub event: MarkModeKeyEvent,
 pub modifiers: ModifiersState,
}

/// Side-effect surface consumed by [`dispatch_mark_mode`].
/// Extracted so the mark-mode-key wiring can be matrix-tested headlessly
/// (a `RecordingSink` impl in `tests.rs` records calls; production uses
/// `impl MarkModeSink for App`). Per & External
/// Resource Abstraction — the logic layer must not embed concrete runtime
/// resources, so the side effects flow through this trait.
/// Method-name overlap with `App` inherent methods: six trait methods
/// share names with inherent methods on `App`
/// (`pane_mark_cursor`, `pane_selection`, `exit_mark_mode`,
/// `set_pane_selection`, `clear_pane_selection`, `copy_selection`).
/// The `impl MarkModeSink for App` block uses UFCS-via-`Self::method(self,
///...)` to resolve each call to the inherent method without recursing
/// through the trait — see the safety comment at the top of that impl
/// block.
pub(super) trait MarkModeSink {
 /// Resource snapshot for the gate. `&mut self` because
 /// the production impl reads mux state which can require `&mut`
 /// borrow paths.
 fn mark_mode_resources(&mut self, pane_id: PaneId) -> MarkModeResources;
 /// Current mark cursor position for the pane (None if no cursor).
 fn pane_mark_cursor(&self, pane_id: PaneId) -> Option<MarkCursor>;
 /// Current selection on the pane (None if no selection).
 /// Returns `Selection` by value (not by borrow) so the test sink can
 /// satisfy the trait without exposing a borrow into its `HashMap`
 /// storage. `Selection` is `Copy`, so the cost is a stack copy on
 /// the cold mark-mode dispatch path.
 fn pane_selection(&self, pane_id: PaneId) -> Option<Selection>;
 /// Trigger a snapshot refresh on the pane's mux (no-op if mux absent
 /// or snapshot is fresh — the production impl checks both internally).
 fn refresh_pane_snapshot(&mut self, pane_id: PaneId);
 /// Exit mark mode for the pane (clears mark cursor entry).
 fn exit_mark_mode(&mut self, pane_id: PaneId);
 /// Update the pane's mark cursor.
 fn set_mark_cursor(&mut self, pane_id: PaneId, cursor: MarkCursor);
 /// Set the pane's selection to the given range.
 fn set_pane_selection(&mut self, pane_id: PaneId, sel: Selection);
 /// Clear the pane's selection.
 fn clear_pane_selection(&mut self, pane_id: PaneId);
 /// Scroll the pane's viewport by `lines` (positive = up).
 fn scroll_display(&mut self, pane_id: PaneId, lines: isize);
 /// Copy the current selection to the clipboard.
 fn copy_selection(&mut self);
 /// Mark the focused window as needing a redraw.
 fn mark_dirty(&mut self);
 /// Snapshot+grid+pure-handler dispatch. Production reads
 /// `self.mux.pane_snapshot` and `self.config.behavior.word_delimiters`
 /// internally and calls `mark_mode::handle_mark_mode_key` against the
 /// constructed `SnapshotGrid`. Tests return a pre-configured
 /// `MarkModeResult` via `take()` (`MarkModeResult` is non-`Clone`).
 /// Infallible per the Defensive-Code-for-Impossible-States rule:
 /// `mark_mode_should_exit` synchronously gates resource presence;
 /// the snapshot is guaranteed by the caller.
 fn dispatch_mark_mode_key(&mut self, input: MarkKeyInput) -> MarkModeResult;
}

/// Wire a mark-mode key event through the gate decision and, if all
/// resources are present, into [`MarkModeSink::dispatch_mark_mode_key`].
/// Generic over the sink type for static dispatch per
/// Choice.
/// Returns `false` when:
/// - `input.active_pane_id == None` (no pane to dispatch against);
/// - `input.mark_mode_active == false` (mark mode not active — caller
/// forwards to keybinding/PTY);
/// - `mark_mode_should_exit(resources) == true` ( path —
/// `sink.exit_mark_mode` called, caller forwards to keybinding/PTY).
/// Returns `true` when:
/// - Released event with mark mode active (consume-all);
/// - Pressed event after successful key dispatch.
/// The return value drives the caller's `if x { return; }` short-circuit
/// — ignoring it would silently mis-route every key event.
#[must_use]
pub(super) fn dispatch_mark_mode<S: MarkModeSink>(input: MarkModeDispatch, sink: &mut S) -> bool {
 let Some(pane_id) = input.active_pane_id else {
 return false;
 };
 if !input.mark_mode_active {
 return false;
 }
 if input.event_state == ElementState::Pressed {
 sink.refresh_pane_snapshot(pane_id);

 let resources = sink.mark_mode_resources(pane_id);
 if mark_mode_should_exit(resources) {
 sink.exit_mark_mode(pane_id);
 return false;
 }

 let cursor = sink
 .pane_mark_cursor(pane_id)
 .expect("cursor_present validated by mark_mode_should_exit guard");
 let selection = sink.pane_selection(pane_id);
 let result = sink.dispatch_mark_mode_key(MarkKeyInput {
 pane_id,
 cursor,
 selection,
 event: input.event,
 modifiers: input.modifiers,
 });

 if let Some(mc) = result.new_cursor {
 sink.set_mark_cursor(pane_id, mc);
 }
 if let Some(sel_update) = result.new_selection {
 match sel_update {
 SelectionUpdate::Set(sel) => sink.set_pane_selection(pane_id, sel),
 SelectionUpdate::Clear => sink.clear_pane_selection(pane_id),
 }
 }

 match result.action {
 MarkAction::Handled { scroll_delta } => {
 if let Some(delta) = scroll_delta {
 sink.scroll_display(pane_id, delta);
 }
 }
 MarkAction::Exit { copy } => {
 sink.exit_mark_mode(pane_id);
 if copy {
 sink.copy_selection();
 }
 }
 MarkAction::Ignored => {}
 }
 sink.mark_dirty();
 }
 true
}

impl MarkModeSink for App {
 fn mark_mode_resources(&mut self, pane_id: PaneId) -> MarkModeResources {
 // UFCS-via-Self resolves to the inherent `App::pane_mark_cursor`
 // (pane_accessors.rs), not the trait method — Rust prefers
 // inherent methods when both exist. Renaming the inherent method
 // without updating this site would silently route through the
 // trait and recurse.
 let cursor_present = Self::pane_mark_cursor(self, pane_id).is_some();
 let snapshot_present = self
 .mux
 .as_ref()
 .and_then(|m| m.pane_snapshot(pane_id))
 .is_some();
 MarkModeResources {
 mux_present: self.mux.is_some(),
 cursor_present,
 snapshot_present,
 }
 }
 fn pane_mark_cursor(&self, pane_id: PaneId) -> Option<MarkCursor> {
 Self::pane_mark_cursor(self, pane_id)
 }
 fn pane_selection(&self, pane_id: PaneId) -> Option<Selection> {
 Self::pane_selection(self, pane_id).copied()
 }
 fn refresh_pane_snapshot(&mut self, pane_id: PaneId) {
 if let Some(mux) = self.mux.as_mut()
 && (mux.pane_snapshot(pane_id).is_none() || mux.is_pane_snapshot_dirty(pane_id))
 {
 mux.refresh_pane_snapshot(pane_id);
 }
 }
 fn exit_mark_mode(&mut self, pane_id: PaneId) {
 Self::exit_mark_mode(self, pane_id);
 }
 fn set_mark_cursor(&mut self, pane_id: PaneId, cursor: MarkCursor) {
 self.mark_cursors.insert(pane_id, cursor);
 }
 fn set_pane_selection(&mut self, pane_id: PaneId, sel: Selection) {
 Self::set_pane_selection(self, pane_id, sel);
 }
 fn clear_pane_selection(&mut self, pane_id: PaneId) {
 Self::clear_pane_selection(self, pane_id);
 }
 fn scroll_display(&mut self, pane_id: PaneId, lines: isize) {
 if let Some(mux) = self.mux.as_mut() {
 mux.scroll_display(pane_id, lines);
 }
 }
 fn copy_selection(&mut self) {
 let _ = Self::copy_selection(self);
 }
 fn mark_dirty(&mut self) {
 if let Some(ctx) = self.focused_ctx_mut() {
 ctx.root.mark_dirty();
 }
 }
 fn dispatch_mark_mode_key(&mut self, input: MarkKeyInput) -> MarkModeResult {
 let snapshot = self
 .mux
 .as_ref()
 .and_then(|m| m.pane_snapshot(input.pane_id))
 .expect("snapshot_present validated by mark_mode_should_exit guard");
 let grid = super::super::snapshot_grid::SnapshotGrid::new(snapshot);
 super::super::mark_mode::handle_mark_mode_key(&MarkModeKeyContext {
 grid: &grid,
 cursor: input.cursor,
 selection: input.selection.as_ref(),
 event: &input.event,
 mods: input.modifiers,
 word_delimiters: &self.config.behavior.word_delimiters,
 })
 }
}
