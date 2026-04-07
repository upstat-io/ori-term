//! Tests for the GPU recovery state machine scaffold (5.16.1).
//!
//! These cover the [`GpuHealth`] type and the `wgpu::DeviceLostReason` →
//! [`GpuLossReason`] conversion. The state-machine transition tests
//! (Healthy → Recovering → Healthy, single-flight coalescing, OOM
//! short-circuit, etc.) land with 5.16.2.

use std::time::Instant;

use super::{GpuHealth, GpuLossReason};

#[test]
fn gpu_health_default_is_healthy_epoch_zero() {
    let h = GpuHealth::new();
    assert!(h.is_healthy());
    assert_eq!(h.epoch(), Some(0));
}

#[test]
fn gpu_health_recovering_carries_epoch_and_attempt() {
    let h = GpuHealth::Recovering {
        epoch: 7,
        attempt: 3,
        since: Instant::now(),
    };
    assert!(!h.is_healthy());
    assert_eq!(h.epoch(), Some(7));
}

#[test]
fn gpu_health_unavailable_has_no_epoch() {
    let h = GpuHealth::Unavailable {
        last_error: "no adapter".to_string(),
        since: Instant::now(),
    };
    assert!(!h.is_healthy());
    assert_eq!(h.epoch(), None);
}

#[test]
fn loss_reason_from_wgpu_destroyed() {
    assert_eq!(
        GpuLossReason::from_wgpu(wgpu::DeviceLostReason::Destroyed),
        GpuLossReason::Destroyed,
    );
}

#[test]
fn loss_reason_from_wgpu_unknown() {
    assert_eq!(
        GpuLossReason::from_wgpu(wgpu::DeviceLostReason::Unknown),
        GpuLossReason::Unknown,
    );
}

// --- next_after_loss transition table (5.16.1 detection) ---

#[test]
fn next_after_loss_healthy_unknown_to_recovering() {
    let h = GpuHealth::Healthy { epoch: 5 };
    let now = Instant::now();
    let next = h
        .next_after_loss(GpuLossReason::Unknown, "callback fired", now)
        .expect("must transition");
    match next {
        GpuHealth::Recovering {
            epoch,
            attempt,
            since,
        } => {
            assert_eq!(epoch, 5);
            assert_eq!(attempt, 0);
            assert_eq!(since, now);
        }
        other => panic!("expected Recovering, got {other:?}"),
    }
}

#[test]
fn next_after_loss_healthy_oom_to_unavailable() {
    let h = GpuHealth::Healthy { epoch: 0 };
    let now = Instant::now();
    let next = h
        .next_after_loss(GpuLossReason::OutOfMemory, "vram exhausted", now)
        .expect("must transition");
    match next {
        GpuHealth::Unavailable { last_error, since } => {
            assert!(
                last_error.contains("vram exhausted"),
                "last_error should preserve detail, got {last_error}",
            );
            assert_eq!(since, now);
        }
        other => panic!("expected Unavailable, got {other:?}"),
    }
}

#[test]
fn next_after_loss_recovering_unknown_coalesces() {
    let h = GpuHealth::Recovering {
        epoch: 1,
        attempt: 2,
        since: Instant::now(),
    };
    assert!(
        h.next_after_loss(GpuLossReason::Unknown, "second loss", Instant::now())
            .is_none(),
        "Recovering must coalesce non-OOM losses (single-flight)",
    );
}

#[test]
fn next_after_loss_recovering_oom_to_unavailable() {
    let h = GpuHealth::Recovering {
        epoch: 1,
        attempt: 2,
        since: Instant::now(),
    };
    let now = Instant::now();
    let next = h
        .next_after_loss(GpuLossReason::OutOfMemory, "OOM mid-recovery", now)
        .expect("OOM short-circuits even from Recovering");
    assert!(matches!(next, GpuHealth::Unavailable { .. }));
}

#[test]
fn next_after_loss_unavailable_oom_no_clobber() {
    let original = "first OOM";
    let h = GpuHealth::Unavailable {
        last_error: original.to_string(),
        since: Instant::now(),
    };
    assert!(
        h.next_after_loss(GpuLossReason::OutOfMemory, "second OOM", Instant::now())
            .is_none(),
        "Unavailable must not be clobbered by a second OOM",
    );
}

#[test]
fn next_after_loss_unavailable_other_no_transition() {
    let h = GpuHealth::Unavailable {
        last_error: "permanent failure".to_string(),
        since: Instant::now(),
    };
    assert!(
        h.next_after_loss(GpuLossReason::SurfaceOther, "noise", Instant::now())
            .is_none(),
        "Unavailable requires manual retry — no auto transition",
    );
}

#[test]
fn next_after_loss_surface_other_carries_through() {
    let h = GpuHealth::Healthy { epoch: 0 };
    let next = h
        .next_after_loss(
            GpuLossReason::SurfaceOther,
            "WSLg DX12 device removed",
            Instant::now(),
        )
        .expect("SurfaceOther escalates from Healthy");
    assert!(matches!(next, GpuHealth::Recovering { .. }));
}

// --- Unavailable accessor coverage ---

#[test]
fn unavailable_accessors_return_fields() {
    let now = Instant::now();
    let h = GpuHealth::Unavailable {
        last_error: "no adapter".to_string(),
        since: now,
    };
    assert_eq!(h.unavailable_message(), Some("no adapter"));
    assert_eq!(h.unavailable_since(), Some(now));
    assert_eq!(h.recovering_since(), None);
}

#[test]
fn recovering_since_accessor_returns_field() {
    let now = Instant::now();
    let h = GpuHealth::Recovering {
        epoch: 0,
        attempt: 0,
        since: now,
    };
    assert_eq!(h.recovering_since(), Some(now));
    assert_eq!(h.unavailable_since(), None);
    assert_eq!(h.unavailable_message(), None);
}

#[test]
fn healthy_accessors_all_none() {
    let h = GpuHealth::Healthy { epoch: 0 };
    assert_eq!(h.recovering_since(), None);
    assert_eq!(h.unavailable_since(), None);
    assert_eq!(h.unavailable_message(), None);
}
