//! Run-loop orchestration for `PaneIoThread` — the `select!` bodies and the
//! PTY-EOF drain sequence.
//!
//! Extracted from `mod.rs` (§ / §-b split) so the struct
//! definition stays short and the loop bodies + EOF sequence each have a
//! single-responsibility home. The loop arms remain duplicated between the
//! sync-deadline and no-deadline branches because the `default(timeout)` arm
//! is not expressible outside the `crossbeam_channel::select!` macro — any
//! attempt to share one closure forces a runtime `if let` inside the macro,
//! which `select!` doesn't let you conditionalise.

use std::io;
use std::sync::atomic::Ordering;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use oriterm_core::effect::sink::EffectSink;
use oriterm_core::effect::{Effect, HostEffect};

use crate::mux_event::MuxEvent;

use super::{CHILD_EXIT_WAIT_TIMEOUT, PaneIoCommand, PaneIoThread};

/// Ceiling on the `select!` default-arm timeout when no deadline (sync or
/// animation) is active. 24 hours is effectively infinite for an idle pane
/// while keeping a single bounded timeout path — the alternative (a
/// dedicated no-timeout `select!` branch) duplicates every match arm and
/// made the loop unreadable. Any real event (PTY byte, command, child
/// exit, response fulfillment) wakes the thread earlier.
const IDLE_WAKE_CEILING: Duration = Duration::from_hours(24);

impl<S: EffectSink> PaneIoThread<S> {
    /// Run the IO thread message loop.
    /// Priority: drain commands first, then process pending bytes with
    /// bounded chunking. Blocks via `crossbeam_channel::select!` when both
    /// channels are empty. Exits on `Shutdown` command or channel disconnect.
    pub fn run(mut self) {
        // Produce an initial snapshot so the main thread has valid content
        // immediately — before any PTY output or commands arrive. Without
        // this, freshly spawned panes expose PaneSnapshot::default() until
        // the shell writes its first output.
        self.grid_dirty.store(true, Ordering::Release);
        self.produce_snapshot();

        #[cfg(test)]
        if let Some(ref barrier) = self.start_barrier {
            barrier.wait();
        }

        loop {
            // 1. Drain all pending commands (priority over bytes).
            self.drain_commands();
            if self.shutdown.load(Ordering::Acquire) {
                // Flush any parsed-but-unpublished state before exiting.
                self.maybe_produce_snapshot();
                return;
            }

            // 2. Process available bytes (non-blocking drain with chunking).
            self.process_pending_bytes();
            if self.shutdown.load(Ordering::Acquire) {
                // Shutdown observed during a chunked-byte drain — `drain_commands`
                // is called between chunks per `handle_bytes_chunked`, and that
                // path can set the shutdown flag. Without this check the loop
                // would fall through to tick_animations + maybe_produce_snapshot
                // + maybe_shrink_buffers + select! (timeout up to
                // IDLE_WAKE_CEILING = 24h), delaying shutdown by an arbitrary
                // wait. Regression: / tpr-review round 4 F1.
                self.maybe_produce_snapshot();
                return;
            }

            // 3. Tick animations — advances any viewport-visible kitty/sixel
            // animations whose frame deadline has passed and emits
            // MuxEvent::AnimationDeadlineChanged on the next-deadline edge.
            // Runs between parse (which may create new animations) and
            // snapshot production (which serialises the post-advance state).
            let animation_deadline = self.tick_animations();

            // 4. Produce snapshot if state changed and sync output allows it.
            self.maybe_produce_snapshot();

            // 4b. Quiescence boundary — about to block waiting for work.
            // Runs every loop iteration; cap-gated `maybe_shrink` is
            // ~O(1) when capacity is already tight, so cost is
            // negligible during active parsing and only does real
            // work after a flood quiesces. Increments
            // `shrink_call_count` (cfg(test)) so §03 regression guard 3
            // can verify the OUTER-loop call path fires (not the
            // `select!` `default(timeout)` arm). Regression:.
            self.maybe_shrink_buffers();

            // 4c. Final pre-`select!` shutdown check — defense in depth covering any setter that bypasses the writer-thread `io_wake` path; otherwise the IO thread could block in `select!` until `byte_rx` hangs up.
            if self.shutdown.load(Ordering::Acquire) {
                self.maybe_produce_snapshot();
                return;
            }

            // 5. Block until the next event. Sync (Mode 2026) + animation deadlines unify via `min`; `IDLE_WAKE_CEILING` keeps the loop simple when neither is active.
            let sync_deadline = self.processor.sync_timeout().sync_timeout();
            let now = Instant::now();
            let timeout = match (sync_deadline, animation_deadline) {
                (Some(s), Some(a)) => s.min(a).saturating_duration_since(now),
                (Some(s), None) => s.saturating_duration_since(now),
                (None, Some(a)) => a.saturating_duration_since(now),
                (None, None) => IDLE_WAKE_CEILING,
            };

            crossbeam_channel::select! {
            recv(self.cmd_rx) -> msg => if self.handle_cmd_arm(msg) { return; },
            recv(self.byte_rx) -> msg => if self.handle_byte_arm(msg) { return; },
            recv(self.child_exit_rx) -> status => self.handle_child_exit_arm(status),
            recv(self.io_wake_rx) -> msg => self.handle_io_wake_arm(msg),
            default(timeout) => self.handle_timeout_arm(sync_deadline),
            }
        }
    }

    /// `cmd_rx` arm: apply any pending resize first, then dispatch or shutdown.
    /// Returns `true` when the run loop should exit (Shutdown command or
    /// channel disconnect); `false` otherwise. Apply-resize-first preserves
    /// post-resize state across idle-wake → `SnapshotNow` ordering races.
    fn handle_cmd_arm(&mut self, msg: Result<PaneIoCommand, crossbeam_channel::RecvError>) -> bool {
        match msg {
            Ok(cmd) => {
                self.apply_pending_resize();
                if matches!(&cmd, PaneIoCommand::Shutdown) {
                    self.shutdown.store(true, Ordering::Release);
                    self.maybe_produce_snapshot();
                    return true;
                }
                self.handle_command(cmd);
                false
            }
            Err(_) => true,
        }
    }

    /// `byte_rx` arm: chunk-drain bytes through VTE, or hand off to EOF
    /// handler when the writer dropped. Returns `true` to exit the loop.
    fn handle_byte_arm(&mut self, msg: Result<Vec<u8>, crossbeam_channel::RecvError>) -> bool {
        if let Ok(bytes) = msg {
            self.handle_bytes_chunked(&bytes);
            false
        } else {
            self.handle_pty_eof();
            true
        }
    }

    /// `child_exit_rx` arm: record the exit status for later coalescing with
    /// the EOF path. On watcher-sender drop, switch to `never()` so the
    /// `select!` does not spin on a closed receiver.
    fn handle_child_exit_arm(
        &mut self,
        status: Result<crate::pty::spawn::ExitStatus, crossbeam_channel::RecvError>,
    ) {
        if let Ok(status) = status {
            self.pending_child_exit = Some(status);
        } else {
            self.child_exit_rx = crossbeam_channel::never();
        }
    }

    /// `io_wake_rx` arm: handle wake events from response fulfillment, pending
    /// resize, or writer-thread shutdown. On all-senders-dropped, switch to
    /// `never()` to prevent the same spin hazard as `child_exit_rx`.
    fn handle_io_wake_arm(&mut self, msg: Result<(), crossbeam_channel::RecvError>) {
        if msg.is_err() {
            self.io_wake_rx = crossbeam_channel::never();
        }
        // Otherwise: woken by response fulfillment, a pending-resize store,
        // writer shutdown, or send_command Shutdown — next iteration handles.
    }

    /// `default(timeout)` arm: either the sync deadline fired (close Mode 2026
    /// window — clear `SYNC_UPDATE`, emit Abort, publish), an animation
    /// deadline fired (next iteration's `tick_animations` advances the frame),
    /// or the idle sentinel expired (loop around harmlessly).
    fn handle_timeout_arm(&mut self, sync_deadline: Option<Instant>) {
        if sync_deadline.is_some_and(|d| d <= Instant::now()) {
            self.handle_sync_timeout();
        }
    }

    /// Tick animation state and emit `MuxEvent::AnimationDeadlineChanged` on
    /// the deadline edge.
    /// Returns the next frame deadline (or `None` when no animation is
    /// active or viewport-visible) for use as the `select!` timeout below.
    /// Sets `grid_dirty` when the image cache was mutated (frame applied)
    /// so `maybe_produce_snapshot` flips the double buffer — otherwise an
    /// animation frame advance between VTE reads would be silently lost.
    fn tick_animations(&mut self) -> Option<Instant> {
        let now = Instant::now();
        let new_deadline = self.terminal.advance_animations(now);

        if self.terminal.take_image_dirty() {
            self.grid_dirty.store(true, Ordering::Release);
        }

        if new_deadline != self.last_animation_deadline {
            // Ignore send failure — the mux may be shutting down; the
            // snapshot path will still publish any pending state.
            let _ = self.mux_tx.send(MuxEvent::AnimationDeadlineChanged {
                pane_id: self.pane_id,
                deadline: new_deadline,
            });
            self.last_animation_deadline = new_deadline;
        }

        new_deadline
    }

    /// Spawn the IO thread.
    pub fn spawn(self) -> io::Result<JoinHandle<()>> {
        thread::Builder::new()
            .name("terminal-io".into())
            .spawn(move || self.run())
    }

    /// Handle PTY end-of-file: flush pending effects, produce the final
    /// snapshot, consume or wait for the child's exit status, emit
    /// `HostEffect::ChildExit`, flush once more, then return.
    /// Sequence (per §17 of the effect-cutover plan blind-spot analysis):
    /// 1. Final `drain_effects_into_mux_events()` — flush any effects
    /// produced during the preceding parse cycle.
    /// 2. `maybe_produce_snapshot()` — publish the PTY's final cell
    /// content to the main thread BEFORE `MuxEvent::PaneExited`
    /// arrives. Gated by Mode 2026 synchronized-output so sync-active
    /// panes defer the snapshot per the application's request.
    /// 3. Exit-code source: cached `pending_child_exit` if the watcher
    /// already fired, otherwise `child_exit_rx.recv_timeout(5s)`.
    /// 4. Emit `HostEffect::ChildExit { code }` through the sink.
    /// 5. Final `drain_effects_into_mux_events()` — routes to
    /// `MuxEvent::PaneExited { pane_id, exit_code }`.
    /// 6. Return from `run()`.
    fn handle_pty_eof(&mut self) {
        // (1) Flush in-flight effects from the last parse chunk.
        self.drain_effects_into_mux_events();

        // (2) Final snapshot BEFORE `PaneExited` fires.
        self.grid_dirty.store(true, Ordering::Release);
        self.maybe_produce_snapshot();

        // (3) Determine the exit code.
        let exit_code = if let Some(status) = self.pending_child_exit.take() {
            status.exit_code() as i32
        } else if let Ok(status) = self.child_exit_rx.recv_timeout(CHILD_EXIT_WAIT_TIMEOUT) {
            status.exit_code() as i32
        } else {
            log::error!(
                "PaneIoThread ({}): child exit not observed within {:?}; emitting \
 ChildExit {{ code: 0 }} as fallback",
                self.pane_id,
                CHILD_EXIT_WAIT_TIMEOUT,
            );
            0
        };

        // (4) Push the exit effect into the sink.
        self.terminal
            .effect_sink()
            .push(Effect::Host(HostEffect::ChildExit { code: exit_code }));

        // (5) Final drain — routes to `MuxEvent::PaneExited` via the
        // effect router (fires wakeup so the main thread sees the
        // pane close within one event loop iteration).
        self.drain_effects_into_mux_events();
    }
}
