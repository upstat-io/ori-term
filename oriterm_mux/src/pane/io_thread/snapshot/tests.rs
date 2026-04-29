//! Tests for `SnapshotDoubleBuffer`.

use oriterm_core::RenderableContent;

use super::SnapshotDoubleBuffer;

/// Helper: create a `RenderableContent` with `n` dummy cells on line 0.
///
/// Uses `renderable_content_into` indirectly — populates the `cells` vec
/// directly with default-constructed cells (all fields zero/empty).
fn content_with_cells(n: usize) -> RenderableContent {
    let mut c = RenderableContent::default();
    for i in 0..n {
        c.cells.push(oriterm_core::RenderableCell {
            line: 0,
            column: oriterm_core::Column(i),
            ch: ' ',
            fg: Default::default(),
            bg: Default::default(),
            flags: oriterm_core::cell::CellFlags::empty(),
            underline_color: None,
            has_hyperlink: false,
            hyperlink_uri: None,
            zerowidth: Vec::new(),
        });
    }
    c
}

/// Basic flip: producer fills 10 cells, flips, consumer swaps and gets them.
#[test]
fn flip_swap_exchanges_buffers() {
    let db = SnapshotDoubleBuffer::new();

    let mut producer_buf = content_with_cells(10);
    assert_eq!(producer_buf.cells.len(), 10);

    db.flip_swap(&mut producer_buf);

    // Producer got back the old front (empty default).
    assert_eq!(producer_buf.cells.len(), 0);

    // Consumer swaps and gets the 10-cell snapshot.
    let mut consumer_buf = RenderableContent::default();
    assert!(db.swap_front(&mut consumer_buf));
    assert_eq!(consumer_buf.cells.len(), 10);
}

/// `swap_front` returns `false` when no flip has occurred.
#[test]
fn no_new_when_not_flipped() {
    let db = SnapshotDoubleBuffer::new();

    assert!(!db.has_new());
    let mut buf = RenderableContent::default();
    assert!(!db.swap_front(&mut buf));
}

/// Two flips without consuming — second flip sets `all_dirty` on front.
#[test]
fn skipped_frame_sets_all_dirty() {
    let db = SnapshotDoubleBuffer::new();

    // First flip.
    let mut buf1 = content_with_cells(5);
    buf1.all_dirty = false;
    db.flip_swap(&mut buf1);

    // Second flip without consuming — frame 1 was skipped.
    let mut buf2 = content_with_cells(3);
    buf2.all_dirty = false;
    db.flip_swap(&mut buf2);

    // The front (buf2's data) should have `all_dirty` set.
    let mut consumer = RenderableContent::default();
    assert!(db.swap_front(&mut consumer));
    assert!(
        consumer.all_dirty,
        "skipped frame should set all_dirty on the new front"
    );
}

/// First flip (no skip) does NOT set `all_dirty`.
#[test]
fn first_flip_does_not_set_all_dirty() {
    let db = SnapshotDoubleBuffer::new();

    let mut buf = content_with_cells(5);
    buf.all_dirty = false;
    db.flip_swap(&mut buf);

    let mut consumer = RenderableContent::default();
    assert!(db.swap_front(&mut consumer));
    assert!(
        !consumer.all_dirty,
        "first flip (no skip) should not set all_dirty"
    );
}

/// Buffer allocations are retained through flip/swap cycles.
///
/// After warmup, both the producer (IO thread) and consumer (main thread)
/// get pre-allocated buffers back from each swap — no new allocations.
#[test]
fn allocation_reuse() {
    let db = SnapshotDoubleBuffer::new();

    // Round 1: producer fills 1000 cells, flips.
    let mut producer = content_with_cells(1000);
    db.flip_swap(&mut producer);
    // `producer` now holds the old front (default, small capacity).

    // Consumer swaps — gets the 1000-cell buffer, gives back default.
    let mut consumer = RenderableContent::default();
    db.swap_front(&mut consumer);
    assert_eq!(consumer.cells.len(), 1000);
    assert!(consumer.cells.capacity() >= 1000);

    // Round 2: producer fills and flips again (smaller content).
    producer = content_with_cells(50);
    db.flip_swap(&mut producer);
    // `producer` now holds the previous front — which was the consumer's
    // old default buffer (swapped in by consumer's swap_front).

    // Consumer swaps again — gives back the 1000-capacity buffer.
    db.swap_front(&mut consumer);
    assert_eq!(consumer.cells.len(), 50);

    // Round 3: producer flips again.
    producer.cells.clear();
    db.flip_swap(&mut producer);

    // `producer` now holds what the consumer swapped in during round 2 —
    // the 1000-capacity buffer.
    assert!(
        producer.cells.capacity() >= 1000,
        "producer should eventually get back the high-capacity buffer: cap={}",
        producer.cells.capacity()
    );
}

/// Sequence numbers track correctly over many flips and partial consumes.
#[test]
fn seqno_monotonic() {
    let db = SnapshotDoubleBuffer::new();
    let mut producer = RenderableContent::default();
    let mut consumer = RenderableContent::default();

    for i in 0..100u64 {
        db.flip_swap(&mut producer);
        if i % 3 == 0 {
            assert!(db.has_new(), "should have new after flip (i={i})");
            assert!(db.swap_front(&mut consumer));
            assert!(!db.has_new(), "should be caught up after swap (i={i})");
        }
    }
}

/// `SnapshotDoubleBuffer` is `Send + Sync` (required for cross-thread use).
#[test]
fn is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<SnapshotDoubleBuffer>();
}

/// `seqno()` increments on each `flip_swap()`.
#[test]
fn seqno_increments_on_flip_swap() {
    let db = SnapshotDoubleBuffer::new();
    assert_eq!(db.seqno(), 0);

    let mut buf = RenderableContent::default();
    db.flip_swap(&mut buf);
    assert_eq!(db.seqno(), 1);

    db.flip_swap(&mut buf);
    assert_eq!(db.seqno(), 2);

    db.flip_swap(&mut buf);
    assert_eq!(db.seqno(), 3);
}

/// `seqno()` is stable when no `flip_swap()` occurs.
#[test]
fn seqno_stable_when_no_flip() {
    let db = SnapshotDoubleBuffer::new();
    assert_eq!(db.seqno(), 0);

    // has_new() and swap_front() don't change seqno.
    assert!(!db.has_new());
    assert_eq!(db.seqno(), 0);

    let mut buf = RenderableContent::default();
    assert!(!db.swap_front(&mut buf));
    assert_eq!(db.seqno(), 0);
}

/// `consumed_seqno()` advances when `swap_front()` consumes a snapshot.
#[test]
fn consumed_seqno_tracks_consumer() {
    let db = SnapshotDoubleBuffer::new();
    assert_eq!(db.consumed_seqno(), 0);

    let mut producer = RenderableContent::default();
    db.flip_swap(&mut producer);
    assert_eq!(db.seqno(), 1);
    assert_eq!(db.consumed_seqno(), 0);

    let mut consumer = RenderableContent::default();
    db.swap_front(&mut consumer);
    assert_eq!(db.consumed_seqno(), 1);

    // Multiple flips without consuming — consumed_seqno stays at 1.
    db.flip_swap(&mut producer);
    db.flip_swap(&mut producer);
    assert_eq!(db.seqno(), 3);
    assert_eq!(db.consumed_seqno(), 1);

    db.swap_front(&mut consumer);
    assert_eq!(db.consumed_seqno(), 3);
}

// --- BUG-11-002: front-buffer shrink at quiescence ---

/// `maybe_shrink_front` reduces front-buffer capacity at quiescence.
///
/// Regression: BUG-11-002. After a flood, the front buffer holds the
/// most-recent peak content — without an explicit shrink it retains the
/// peak capacity indefinitely. Pins that `maybe_shrink_front` calls
/// `RenderableContent::maybe_shrink` on the front (cap > 4*len &&
/// cap > 4096 → shrink_to(2*len)) without mutating `seqno` /
/// `consumed_seqno`. Sync-suppression invariants pinned by callers of
/// `seqno()` / `consumed_seqno()` must be preserved.
/// See: bug-tracker/plans/BUG-11-002/section-03-tdd-matrix.md §"Edge cases" — front-buffer shrink.
#[test]
fn front_buffer_shrinks_at_quiescence() {
    let db = SnapshotDoubleBuffer::new();

    // Build a producer buffer with high capacity but low len — mimics a
    // post-flood front buffer that has retained peak allocation while
    // the live content has shrunk to a typical viewport.
    let mut producer = content_with_cells(10_000);
    let cap_grown = producer.cells.capacity();
    assert!(
        cap_grown > 4096,
        "setup: capacity must exceed shrink gate, got {cap_grown}"
    );
    producer.cells.truncate(100); // len << cap

    db.flip_swap(&mut producer);
    // slot.front now holds cap=cap_grown, len=100.

    let seqno_before = db.seqno();
    let consumed_before = db.consumed_seqno();

    db.maybe_shrink_front();

    // Sync-suppression invariants preserved.
    assert_eq!(
        db.seqno(),
        seqno_before,
        "maybe_shrink_front must NOT mutate seqno"
    );
    assert_eq!(
        db.consumed_seqno(),
        consumed_before,
        "maybe_shrink_front must NOT mutate consumed_seqno"
    );

    // Capacity shrunk; content preserved. Inspect via swap_front.
    let mut consumer = RenderableContent::default();
    assert!(db.swap_front(&mut consumer), "front should still be available");
    assert_eq!(consumer.cells.len(), 100, "shrink must preserve content");
    assert!(
        consumer.cells.capacity() < cap_grown,
        "front capacity must shrink: {} → {}",
        cap_grown,
        consumer.cells.capacity()
    );
}
