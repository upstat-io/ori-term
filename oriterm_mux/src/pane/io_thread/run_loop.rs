//! Run-loop orchestration for `PaneIoThread` — the `select!` bodies and the
//! PTY-EOF drain sequence.
//!
//! Extracted from `mod.rs` (§BUG-11-14 / §BUG-08-19-b split) so the struct
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
    ///
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

            // 3. Tick animations — advances any viewport-visible kitty/sixel
            //    animations whose frame deadline has passed and emits
            //    MuxEvent::AnimationDeadlineChanged on the next-deadline edge.
            //    Runs between parse (which may create new animations) and
            //    snapshot production (which serialises the post-advance state).
            let animation_deadline = self.tick_animations();

            // 4. Produce snapshot if state changed and sync output allows it.
            self.maybe_produce_snapshot();

            // 5. Block until the next event.
            //
            // Mode 2026 (synchronized output): when a sync buffer is pending,
            // the VTE processor's StdSyncHandler tracks a deadline; we must
            // wake before the sync deadline to call stop_sync and flush the
            // buffer — otherwise an app that crashes mid-sync hangs the
            // terminal forever.
            //
            // Animation: when at least one pane has an active animation,
            // `animation_deadline` carries the next-frame instant so the
            // `default(timeout)` arm wakes us at the right moment to re-run
            // the tick on the next loop iteration.
            //
            // The two deadlines are unified via `min`. When neither is set,
            // the `IDLE_WAKE_CEILING` sentinel keeps the loop structurally
            // simple — any real event wakes earlier.
            let sync_deadline = self.processor.sync_timeout().sync_timeout();
            let now = Instant::now();
            let timeout = match (sync_deadline, animation_deadline) {
                (Some(s), Some(a)) => s.min(a).saturating_duration_since(now),
                (Some(s), None) => s.saturating_duration_since(now),
                (None, Some(a)) => a.saturating_duration_since(now),
                (None, None) => IDLE_WAKE_CEILING,
            };

            crossbeam_channel::select! {
                recv(self.cmd_rx) -> msg => {
                    match msg {
                        Ok(PaneIoCommand::Shutdown) => {
                            self.shutdown.store(true, Ordering::Release);
                            self.maybe_produce_snapshot();
                            return;
                        }
                        Ok(cmd) => self.handle_command(cmd),
                        Err(_) => return,
                    }
                },
                recv(self.byte_rx) -> msg => {
                    if let Ok(bytes) = msg {
                        self.handle_bytes_chunked(&bytes);
                    } else {
                        self.handle_pty_eof();
                        return;
                    }
                },
                recv(self.child_exit_rx) -> status => {
                    if let Ok(status) = status {
                        self.pending_child_exit = Some(status);
                    } else {
                        // Watcher-thread sender dropped without sending a
                        // status. Replace the receiver with `never()` so
                        // `select!` does not pick this arm again on every
                        // iteration — `recv` on a disconnected channel
                        // returns `Err` immediately, which would burn a CPU
                        // core in a tight loop until shutdown. The EOF path
                        // in `handle_pty_eof` still emits
                        // `HostEffect::ChildExit { code: 0 }` when `byte_rx`
                        // subsequently closes.
                        self.child_exit_rx = crossbeam_channel::never();
                    }
                }
                recv(self.response_wake_rx) -> msg => {
                    if msg.is_err() {
                        // Handle dropped its `response_wake_tx`. Same spin
                        // hazard as the `child_exit_rx` arm above.
                        self.response_wake_rx = crossbeam_channel::never();
                    }
                    // Otherwise: woken by response fulfillment — next loop
                    // iteration drains commands which polls pending responses
                    // and emits PTY replies.
                }
                default(timeout) => {
                    // Either the sync deadline fired (flush the buffer),
                    // an animation deadline fired (next iteration's
                    // tick_animations advances the frame), or the idle
                    // sentinel expired (harmless — loop around).
                    if sync_deadline.is_some_and(|d| d <= Instant::now()) {
                        self.handle_sync_timeout();
                    }
                },
            }
        }
    }

    /// Tick animation state and emit `MuxEvent::AnimationDeadlineChanged` on
    /// the deadline edge.
    ///
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
    ///
    /// Sequence (per §17 of the effect-cutover plan blind-spot analysis):
    ///
    /// 1. Final `drain_effects_into_mux_events()` — flush any effects
    ///    produced during the preceding parse cycle.
    /// 2. `maybe_produce_snapshot()` — publish the PTY's final cell
    ///    content to the main thread BEFORE `MuxEvent::PaneExited`
    ///    arrives. Gated by Mode 2026 synchronized-output so sync-active
    ///    panes defer the snapshot per the application's request.
    /// 3. Exit-code source: cached `pending_child_exit` if the watcher
    ///    already fired, otherwise `child_exit_rx.recv_timeout(5s)`.
    /// 4. Emit `HostEffect::ChildExit { code }` through the sink.
    /// 5. Final `drain_effects_into_mux_events()` — routes to
    ///    `MuxEvent::PaneExited { pane_id, exit_code }`.
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
        //     effect router (fires wakeup so the main thread sees the
        //     pane close within one event loop iteration).
        self.drain_effects_into_mux_events();
    }
}
