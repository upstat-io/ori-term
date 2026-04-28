//! Tests for EffectSink trait and concrete implementations.

use super::*;
use crate::effect::{Effect, HostEffect, PtyEffect, PtyWriteKind, UiEffect};

#[test]
fn queueing_sink_push_drain_roundtrip() {
    let sink = QueueingEffectSink::new();
    sink.push(Effect::Host(HostEffect::Bell));
    sink.push(Effect::Ui(UiEffect::MouseCursorDirty));

    let mut out = Vec::new();
    sink.drain_into(&mut out);
    assert_eq!(out.len(), 2);
    assert!(matches!(out[0], Effect::Host(HostEffect::Bell)));
    assert!(matches!(out[1], Effect::Ui(UiEffect::MouseCursorDirty)));

    // Drain again — should be empty.
    let mut out2 = Vec::new();
    sink.drain_into(&mut out2);
    assert!(out2.is_empty());
}

#[test]
fn queueing_sink_retains_capacity_after_drain() {
    let sink = QueueingEffectSink::new();

    // Push 10 effects to force allocation.
    for _ in 0..10 {
        sink.push(Effect::Host(HostEffect::Bell));
    }

    let mut out = Vec::new();
    sink.drain_into(&mut out);
    assert_eq!(out.len(), 10);

    // The internal queue should still have capacity >= 10.
    let cap_after_drain = sink.queue.lock().capacity();
    assert!(cap_after_drain >= 10);

    // Push 5 more — should NOT allocate (capacity already available).
    for _ in 0..5 {
        sink.push(Effect::Host(HostEffect::Bell));
    }

    let cap_after_refill = sink.queue.lock().capacity();
    assert_eq!(cap_after_drain, cap_after_refill);
}

#[test]
fn queueing_sink_drain_empty_does_not_allocate() {
    let sink = QueueingEffectSink::new();
    let mut out = Vec::new();

    // Draining an empty sink should not change the output vec.
    sink.drain_into(&mut out);
    assert!(out.is_empty());
    assert_eq!(out.capacity(), 0);
}

#[test]
fn void_sink_drops_effects_silently() {
    let sink = VoidEffectSink;
    sink.push(Effect::Host(HostEffect::Bell));
    sink.push(Effect::Pty(PtyEffect::Write {
        bytes: vec![0x1b],
        kind: PtyWriteKind::Other,
    }));

    let mut out = Vec::new();
    sink.drain_into(&mut out);
    assert!(out.is_empty());
}

#[test]
fn drain_into_appends_to_existing_vec() {
    let sink = QueueingEffectSink::new();
    sink.push(Effect::Host(HostEffect::Bell));
    sink.push(Effect::Host(HostEffect::TitleSet {
        value: Some("test".to_owned()),
    }));
    sink.push(Effect::Ui(UiEffect::MouseCursorDirty));

    // Start with a vec that already has 2 items.
    let mut out = vec![
        Effect::Host(HostEffect::Bell),
        Effect::Host(HostEffect::Bell),
    ];
    assert_eq!(out.len(), 2);

    sink.drain_into(&mut out);
    assert_eq!(out.len(), 5);
}
