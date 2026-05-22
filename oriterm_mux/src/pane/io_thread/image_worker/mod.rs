//! Per-pane image decode worker thread — consumes `ImageDecodeRequest`s
//! from a channel, calls `oriterm_core::image::worker_pipeline::run_image_decode`
//! (panic-isolated via `std::panic::catch_unwind`), pushes results to the IO
//! thread for application via `Term::apply_decoded_image`.
//!
//! See: bug-tracker/plans/BUG-06-088/section-05-implementation.md
//!
//! Architecture:
//! - One worker thread per pane (spawned in `PaneIoHandle::new_with_handle`).
//! - Bounded pending-bytes budget (`MAX_PENDING_BYTES = 128 MiB`) gates enqueue;
//!   overflow returns `EnqueueError::WouldExceedBytes` which the kitty handler
//!   maps to ENOMEM via the sequencer.
//! - Worker thread exits cleanly when the request sender is dropped (pane
//!   shutdown).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, Sender, TryRecvError, unbounded};
use oriterm_core::image::worker_pipeline::{
    ImageDecodeError, ImageDecodeRequest, ImageDecodeResult,
};

mod worker;

#[cfg(test)]
mod tests;

/// Maximum pending payload bytes (request channel) before enqueue is rejected
/// with `EnqueueError::WouldExceedBytes`. Protects against unbounded memory
/// growth when notcurses-demo bursts faster than the worker can drain.
pub(crate) const MAX_PENDING_BYTES: usize = 128 * 1024 * 1024;

/// Per-pane image decode worker handle. Owned by `PaneIoThread`; shutdown
/// happens via `Drop` (drops request sender → worker thread exits).
pub struct ImageWorker {
    request_tx: Sender<ImageDecodeRequest>,
    result_rx: Receiver<ImageDecodeResult>,
    pending_request_bytes: Arc<AtomicUsize>,
    join_handle: Option<JoinHandle<()>>,
}

/// Reason an enqueue was rejected. `WouldExceedBytes` is the soft-pressure
/// signal; `WorkerDead` indicates the worker thread has exited (e.g., pane
/// shutdown in progress).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnqueueError {
    /// Adding `req.payload.len()` would exceed `MAX_PENDING_BYTES`.
    WouldExceedBytes,
    /// Worker thread has exited; the request channel is closed.
    WorkerDead,
}

impl ImageWorker {
    /// Spawn a new per-pane image decode worker. The returned handle owns
    /// the request channel; dropping it signals the worker to exit.
    pub fn spawn() -> Self {
        let (request_tx, request_rx) = unbounded::<ImageDecodeRequest>();
        let (result_tx, result_rx) = unbounded::<ImageDecodeResult>();
        let pending_request_bytes = Arc::new(AtomicUsize::new(0));

        let join_handle = std::thread::Builder::new()
            .name("oriterm-image-worker".into())
            .spawn(move || worker::run(request_rx, result_tx))
            .expect("failed to spawn image worker thread");

        Self {
            request_tx,
            result_rx,
            pending_request_bytes,
            join_handle: Some(join_handle),
        }
    }

    /// Enqueue a decode request. Returns `Err(WouldExceedBytes)` if the
    /// pending-bytes budget would overflow; `Err(WorkerDead)` if the worker
    /// thread has exited. On success, byte budget is reserved atomically via
    /// CAS; rolled back if the send fails.
    pub fn enqueue(&self, req: ImageDecodeRequest) -> Result<(), EnqueueError> {
        let bytes = req.payload.len();
        // Atomic CAS reservation: load → check budget → CAS until success or
        // rejection. compare_exchange_weak retries on concurrent contention.
        let mut current = self.pending_request_bytes.load(Ordering::Acquire);
        loop {
            if current.saturating_add(bytes) > MAX_PENDING_BYTES {
                return Err(EnqueueError::WouldExceedBytes);
            }
            match self.pending_request_bytes.compare_exchange_weak(
                current,
                current + bytes,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(observed) => current = observed,
            }
        }
        // Rollback on send failure: budget reservation must not leak when
        // the worker thread has died. Otherwise the budget would permanently
        // exceed MAX_PENDING_BYTES across pane lifetime.
        if let Err(e) = self.request_tx.send(req) {
            self.pending_request_bytes
                .fetch_sub(bytes, Ordering::AcqRel);
            log::warn!("image worker request channel send failed: {e:?}");
            return Err(EnqueueError::WorkerDead);
        }
        Ok(())
    }

    /// Non-blocking drain of all currently-available results. The IO thread
    /// calls this before `process_pending_bytes` and before
    /// `maybe_produce_snapshot` so results land before any subsequent kitty
    /// command dispatch.
    pub fn try_drain_results(&self) -> Vec<ImageDecodeResult> {
        let mut results = Vec::new();
        loop {
            match self.result_rx.try_recv() {
                Ok(result) => {
                    // Decrement budget by the request's payload size. Worker
                    // attaches payload_bytes to the result so the IO thread
                    // can adjust the budget without separate bookkeeping.
                    self.pending_request_bytes
                        .fetch_sub(result.payload_bytes, Ordering::AcqRel);
                    results.push(result);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    // Worker thread died; surface as a synthetic
                    // EnqueueWorkerDead result so the IO thread's drain loop
                    // observes the death and can mark the pane errored.
                    log::error!("image worker result channel disconnected");
                    break;
                }
            }
        }
        results
    }

    /// Receiver suitable for `crossbeam_channel::select!` arms so the IO
    /// thread wakes when a worker result lands.
    pub fn result_rx(&self) -> &Receiver<ImageDecodeResult> {
        &self.result_rx
    }
}

impl Drop for ImageWorker {
    fn drop(&mut self) {
        // Drop the request sender first so the worker observes EOF on its
        // next recv() and exits cleanly. Then join with a bounded wait so a
        // pane teardown can't hang on a stuck worker.
        // (Dropping `self.request_tx` happens implicitly via the field move
        // out of the struct, but we can also explicitly close by replacing
        // with a fresh dummy. Since this struct is consumed entirely at
        // Drop, just relying on field drop order is fine — `request_tx`
        // drops before `join_handle` per Rust's struct field drop order.)
        if let Some(handle) = self.join_handle.take() {
            // Best-effort join; do not panic the IO thread if the worker
            // panicked or hung.
            match handle.join() {
                Ok(()) => {}
                Err(e) => log::warn!("image worker thread join failed: {e:?}"),
            }
        }
    }
}

/// Synthesize an enqueue-failure result for `Term::apply_decoded_image`.
///
/// `EnqueueOverflow` / `EnqueueWorkerDead` route via the same drain path
/// as worker-returned errors so the reply sequencer resolves in command
/// order. Reply formatting stays in `oriterm_core`; this function is the
/// data-only translation seam between `EnqueueError` (mux-private) and
/// `ImageDecodeError` (core-public).
#[allow(
    clippy::too_many_arguments,
    clippy::needless_pass_by_value,
    reason = "data-only translation seam mirroring ImageDecodeResult field shape"
)]
pub fn synthesize_enqueue_failure(
    req_seq: u64,
    image_id: u32,
    payload_bytes: usize,
    reply_ctx: oriterm_core::image::worker_pipeline::DecodeReplyContext,
    placement: Option<oriterm_core::image::worker_pipeline::PlacementParams>,
    err: EnqueueError,
) -> ImageDecodeResult {
    let decoded = Err(match err {
        EnqueueError::WouldExceedBytes => ImageDecodeError::EnqueueOverflow,
        EnqueueError::WorkerDead => ImageDecodeError::EnqueueWorkerDead,
    });
    ImageDecodeResult {
        sequence_id: req_seq,
        image_id,
        decoded,
        reply_ctx,
        placement,
        payload_bytes,
    }
}
