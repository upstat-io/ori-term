//! Worker thread body — receives `ImageDecodeRequest`s, calls
//! `run_image_decode` (panic-isolated), pushes results to the result channel.
//!
//! See: bug-tracker/plans/BUG-06-088/section-05-implementation.md

use std::panic::{AssertUnwindSafe, catch_unwind};

use crossbeam_channel::{Receiver, Sender};
use oriterm_core::image::worker_pipeline::{
    ImageDecodeError, ImageDecodeRequest, ImageDecodeResult, run_image_decode,
};

/// Worker thread loop. Exits cleanly when the request channel is dropped.
///
/// Receiver/Sender are owned by the worker thread (moved in via spawn closure);
/// dropping them at function exit closes the channels and notifies the IO
/// thread of shutdown.
#[allow(
    clippy::needless_pass_by_value,
    reason = "worker thread takes ownership of channels; drop-on-exit signals shutdown"
)]
pub(super) fn run(
    request_rx: Receiver<ImageDecodeRequest>,
    result_tx: Sender<ImageDecodeResult>,
) {
    loop {
        let Ok(req) = request_rx.recv() else {
            // Sender dropped — pane shutdown. Exit cleanly.
            log::debug!("image worker exiting (request channel closed)");
            return;
        };

        let seq = req.sequence_id;
        let image_id = req.image_id;
        let payload_bytes = req.payload.len();
        let reply_ctx = req.reply_ctx;
        let placement = req.placement.clone();

        // Panic isolation: a panic in run_image_decode (e.g., from a malformed
        // payload triggering an unwrap somewhere in the decode chain) must NOT
        // kill the worker thread. Subsequent requests on this pane continue.
        let result = match catch_unwind(AssertUnwindSafe(|| run_image_decode(req))) {
            Ok(r) => r,
            Err(panic_payload) => {
                let message = panic_message(&panic_payload);
                log::error!(
                    "image worker panic on image_id={image_id} seq={seq}: {message}"
                );
                ImageDecodeResult {
                    sequence_id: seq,
                    image_id,
                    decoded: Err(ImageDecodeError::Panicked { message }),
                    reply_ctx,
                    placement,
                    payload_bytes,
                }
            }
        };

        if result_tx.send(result).is_err() {
            // Receiver dropped — IO thread already shut down. Exit.
            log::debug!("image worker exiting (result channel closed)");
            return;
        }
    }
}

fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<non-string panic payload>".to_string()
    }
}
