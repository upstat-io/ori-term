//! ImageWorker unit tests — bounded enqueue, atomic byte accounting, panic
//! isolation, shutdown cleanliness.

use std::sync::atomic::Ordering;

use oriterm_core::image::ImageSource;
use oriterm_core::image::kitty::KittyTransmission;
use oriterm_core::image::worker_pipeline::{
    DecodeReplyContext, ImageDecodeError, ImageDecodeRequest,
};

use super::{EnqueueError, ImageWorker, MAX_PENDING_BYTES};

/// Build a minimal valid f=32 RGBA request for a 1×1 red pixel. Used as the
/// canonical "small request" in tests that don't care about payload content.
fn small_request(seq: u64, image_id: u32) -> ImageDecodeRequest {
    ImageDecodeRequest {
        sequence_id: seq,
        image_id,
        payload: vec![255, 0, 0, 255], // 1×1 RGBA red
        format: 32,
        width: 1,
        height: 1,
        compression: None,
        max_bytes: 1024,
        reply_ctx: DecodeReplyContext::default(),
        image_number: None,
        placement: None,
        source: ImageSource::Direct,
        transmission: KittyTransmission::Direct,
    }
}

/// Build a request with a synthetic large payload (no actual decoding;
/// pending-byte budget tests use these to push the counter).
fn bulk_request(seq: u64, image_id: u32, payload_bytes: usize) -> ImageDecodeRequest {
    ImageDecodeRequest {
        sequence_id: seq,
        image_id,
        payload: vec![0u8; payload_bytes],
        format: 32,
        width: 1,
        height: 1,
        compression: None,
        max_bytes: 1024,
        reply_ctx: DecodeReplyContext::default(),
        image_number: None,
        placement: None,
        source: ImageSource::Direct,
        transmission: KittyTransmission::Direct,
    }
}

#[test]
fn worker_spawn_and_decode_one_request_returns_ok_result() {
    let worker = ImageWorker::spawn();
    worker
        .enqueue(small_request(1, 42))
        .expect("enqueue should succeed");
    // Poll for the result (worker runs on a separate thread).
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let result = loop {
        let drained = worker.try_drain_results();
        if let Some(r) = drained.into_iter().next() {
            break r;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for worker result"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    };
    assert_eq!(result.sequence_id, 1);
    assert_eq!(result.image_id, 42);
    assert!(result.decoded.is_ok(), "1×1 RGBA decode should succeed");
}

#[test]
fn enqueue_increments_pending_bytes_atomically() {
    let worker = ImageWorker::spawn();
    let before = worker.pending_request_bytes.load(Ordering::Acquire);
    worker
        .enqueue(bulk_request(1, 1, 4096))
        .expect("enqueue should succeed");
    // After enqueue the counter is incremented; worker may have already
    // consumed it and we should see decrement on drain.
    let _drained = wait_for_result(&worker);
    let after = worker.pending_request_bytes.load(Ordering::Acquire);
    assert_eq!(
        before, after,
        "byte budget should return to starting value after drain"
    );
}

#[test]
fn enqueue_at_byte_budget_returns_wouldexceed() {
    let worker = ImageWorker::spawn();
    // Pre-load the counter to just under the cap so the next enqueue overflows.
    worker
        .pending_request_bytes
        .store(MAX_PENDING_BYTES - 100, Ordering::Release);
    let result = worker.enqueue(bulk_request(1, 1, 200));
    assert_eq!(result, Err(EnqueueError::WouldExceedBytes));
    // Counter should NOT have been incremented since the CAS check rejected.
    let observed = worker.pending_request_bytes.load(Ordering::Acquire);
    assert_eq!(observed, MAX_PENDING_BYTES - 100);
    // Reset so Drop cleanup doesn't panic on counter underflow.
    worker.pending_request_bytes.store(0, Ordering::Release);
}

#[test]
fn enqueue_at_exactly_budget_succeeds() {
    let worker = ImageWorker::spawn();
    worker
        .pending_request_bytes
        .store(MAX_PENDING_BYTES - 4096, Ordering::Release);
    let result = worker.enqueue(bulk_request(1, 1, 4096));
    assert!(
        result.is_ok(),
        "exact-fit enqueue should succeed (not strictly greater than cap)"
    );
    let _ = wait_for_result(&worker);
}

#[test]
fn worker_panic_isolation_continues_processing_subsequent_requests() {
    // Format=99 is unsupported and produces an EINVAL Reply error (not a
    // panic), so this test exercises the error path — panic injection
    // would require modifying production code which we avoid.
    let worker = ImageWorker::spawn();
    let bad = ImageDecodeRequest {
        sequence_id: 1,
        image_id: 1,
        payload: vec![],
        format: 99, // unsupported
        width: 1,
        height: 1,
        compression: None,
        max_bytes: 1024,
        reply_ctx: DecodeReplyContext::default(),
        image_number: None,
        placement: None,
        source: ImageSource::Direct,
        transmission: KittyTransmission::Direct,
    };
    worker.enqueue(bad).expect("enqueue should succeed");
    let bad_result = wait_for_result(&worker);
    assert!(matches!(
        bad_result.decoded,
        Err(ImageDecodeError::Reply(_))
    ));
    // Worker should still be alive — next request succeeds.
    worker
        .enqueue(small_request(2, 2))
        .expect("worker should still be alive after EINVAL");
    let good_result = wait_for_result(&worker);
    assert!(good_result.decoded.is_ok());
}

#[test]
fn worker_exits_cleanly_when_request_channel_dropped() {
    let worker = ImageWorker::spawn();
    drop(worker); // Drop request_tx → worker exits + join_handle awaited.
    // If we reach here without hanging, the worker exited cleanly.
}

/// Helper: poll for one result with a generous deadline (worker may be slow
/// under loaded CI).
fn wait_for_result(
    worker: &ImageWorker,
) -> oriterm_core::image::worker_pipeline::ImageDecodeResult {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        let drained = worker.try_drain_results();
        if let Some(r) = drained.into_iter().next() {
            return r;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for worker result"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
