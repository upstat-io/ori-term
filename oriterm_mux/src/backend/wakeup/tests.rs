//! Tests for the wakeup-coalescing SSOT helper.
//!
//! Helper-level behavioral pins live alongside three structural pins:
//!
//! - `wakeup_coalescing_ssot_helper_call_present` — both backend files
//!   reach the SSOT
//! - `wakeup_coalescing_ssot_old_scaffold_absent` — neither backend file
//!   reintroduces the inline scaffold
//! - `wakeup_coalescing_helper_uses_release_swap` — helper preserves
//!   `swap(true, Ordering::Release)` ordering invariant

use std::fs;
use std::sync::Arc;
use std::sync::Barrier;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use oriterm_test_support::paths::term_workspace_root;

use super::guarded_wakeup;

fn counting_raw() -> (Arc<dyn Fn() + Send + Sync>, Arc<AtomicUsize>) {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);
    let raw = Arc::new(move || {
        counter_clone.fetch_add(1, Ordering::AcqRel);
    }) as Arc<dyn Fn() + Send + Sync>;
    (raw, counter)
}

/// Pins the leading-edge invocation gate for the wakeup-coalescing helper; raw wakeup
/// fires only on the first call and is suppressed on subsequent calls
/// until `wakeup_pending` is cleared.
#[test]
fn guarded_wakeup_idle_invokes_raw_once() {
    let (raw, counter) = counting_raw();
    let (guarded, _pending) = guarded_wakeup(raw);
    guarded();
    guarded();
    assert_eq!(counter.load(Ordering::Acquire), 1);
}

/// Pins the clear-and-rearm protocol for the wakeup-coalescing helper: storing
/// `false` to the returned flag re-arms the gate so the next guarded
/// call fires the raw wakeup again.
#[test]
fn guarded_wakeup_after_clear_invokes_raw_again() {
    let (raw, counter) = counting_raw();
    let (guarded, pending) = guarded_wakeup(raw);
    guarded();
    pending.store(false, Ordering::Release);
    guarded();
    assert_eq!(counter.load(Ordering::Acquire), 2);
}

/// Pins the flood-rate coalescing invariant for the wakeup-coalescing helper: `N`
/// guarded calls without an intervening clear collapse to a single raw
/// wakeup invocation, regardless of `N`.
#[test]
fn guarded_wakeup_flood_invokes_raw_once_per_clear_cycle() {
    let (raw, counter) = counting_raw();
    let (guarded, _pending) = guarded_wakeup(raw);
    for _ in 0..1024 {
        guarded();
    }
    assert_eq!(counter.load(Ordering::Acquire), 1);
}

/// Pins the initial-state invariant for the wakeup-coalescing helper: the returned
/// `wakeup_pending` flag reads `false` immediately after construction
/// (no spurious wakeup pending before any call).
#[test]
fn guarded_wakeup_initial_state_flag_is_false() {
    let (raw, _counter) = counting_raw();
    let (_guarded, pending) = guarded_wakeup(raw);
    assert!(!pending.load(Ordering::Acquire));
}

/// Pins the post-call flag invariant for the wakeup-coalescing helper: after the
/// first guarded call the returned `wakeup_pending` flag reads `true`.
#[test]
fn guarded_wakeup_after_first_call_sets_flag() {
    let (raw, _counter) = counting_raw();
    let (guarded, pending) = guarded_wakeup(raw);
    guarded();
    assert!(pending.load(Ordering::Acquire));
}

/// Pins the production-shape concurrent-coalescing invariant: 8
/// reader threads each issuing 1024 guarded calls against the SAME
/// helper instance collapse to exactly one raw wakeup between two
/// `clear` boundaries. Mirrors the production usage (multiple PTY
/// reader threads racing the main-thread poll). Uses `Barrier` for
/// thread sync — wall-clock-free.
#[test]
fn guarded_wakeup_concurrent_threads_coalesce() {
    let (raw, counter) = counting_raw();
    let (guarded, _pending) = guarded_wakeup(raw);
    let n_threads = 8usize;
    let calls_per_thread = 1024usize;
    let barrier = Arc::new(Barrier::new(n_threads));
    let handles: Vec<_> = (0..n_threads)
        .map(|_| {
            let guarded_t = Arc::clone(&guarded);
            let barrier_t = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier_t.wait();
                for _ in 0..calls_per_thread {
                    guarded_t();
                }
            })
        })
        .collect();
    for h in handles {
        h.join().expect("thread panicked");
    }
    assert_eq!(counter.load(Ordering::Acquire), 1);
}

/// Pins the per-call allocation isolation invariant: two independent
/// `guarded_wakeup` invocations produce two independent flag/closure
/// pairs that don't share state. Pinning against future "share one
/// global flag" regressions.
#[test]
fn guarded_wakeup_two_independent_instances_isolate_state() {
    let (raw_a, counter_a) = counting_raw();
    let (raw_b, counter_b) = counting_raw();
    let (guarded_a, pending_a) = guarded_wakeup(raw_a);
    let (guarded_b, pending_b) = guarded_wakeup(raw_b);
    guarded_a();
    guarded_a();
    assert_eq!(
        counter_a.load(Ordering::Acquire),
        1,
        "A's raw fired exactly once after two A calls (coalesced)"
    );
    assert_eq!(
        counter_b.load(Ordering::Acquire),
        0,
        "B's raw must not fire when only A was called (state isolation)"
    );
    assert!(
        pending_a.load(Ordering::Acquire),
        "A's flag is set after A calls"
    );
    assert!(
        !pending_b.load(Ordering::Acquire),
        "B's flag must not be set when only A was called (flag isolation)"
    );
    guarded_b();
    assert_eq!(
        counter_a.load(Ordering::Acquire),
        1,
        "A's raw count unchanged after B-only call (counter isolation)"
    );
    assert_eq!(
        counter_b.load(Ordering::Acquire),
        1,
        "B's raw fired on its first call (independent leading-edge)"
    );
}

/// Structural assertion for the wakeup-coalescing helper (helper-call presence).
/// Both backend files MUST contain the SSOT helper invocation
/// `crate::backend::wakeup::guarded_wakeup`. Source-file paths
/// resolved via `oriterm_test_support::paths::term_workspace_root`
/// — never `manifest_dir.parent()` arithmetic.
#[test]
fn wakeup_coalescing_ssot_helper_call_present() {
    let root = term_workspace_root();
    let embedded = fs::read_to_string(root.join("oriterm_mux/src/backend/embedded/mod.rs"))
        .expect("read embedded/mod.rs");
    let transport =
        fs::read_to_string(root.join("oriterm_mux/src/backend/client/transport/mod.rs"))
            .expect("read client/transport/mod.rs");
    assert!(
        embedded.contains("crate::backend::wakeup::guarded_wakeup"),
        "embedded/mod.rs must invoke crate::backend::wakeup::guarded_wakeup"
    );
    assert!(
        transport.contains("crate::backend::wakeup::guarded_wakeup"),
        "client/transport/mod.rs must invoke crate::backend::wakeup::guarded_wakeup"
    );
}

/// Structural assertion for the wakeup-coalescing helper (old-scaffold absence).
/// Neither backend file may contain the duplicated coalescing-pattern
/// scaffold post-fix. Catches reintroduction of the inline closure
/// either by reallocating an `AtomicBool` flag (covered by the
/// `Arc::new(AtomicBool::new(false))` literal) OR by reusing an
/// existing flag with a fresh swap-guard closure (covered by the
/// `swap(true, Ordering::Release)` literal). False-positive boundary:
/// if a future change adds an unrelated `AtomicBool::new(false)` to
/// either backend file, this canary will fire and the developer must
/// verify the addition is not a coalescing-pattern reintroduction.
#[test]
fn wakeup_coalescing_ssot_old_scaffold_absent() {
    let root = term_workspace_root();
    let embedded_path = root.join("oriterm_mux/src/backend/embedded/mod.rs");
    let transport_path = root.join("oriterm_mux/src/backend/client/transport/mod.rs");
    let embedded = fs::read_to_string(&embedded_path).expect("read embedded/mod.rs");
    let transport = fs::read_to_string(&transport_path).expect("read client/transport/mod.rs");
    for (label, body) in [
        ("embedded/mod.rs", &embedded),
        ("transport/mod.rs", &transport),
    ] {
        assert!(
            !body.contains("Arc::new(AtomicBool::new(false))"),
            "{label} must not reintroduce the inline AtomicBool scaffold"
        );
        assert!(
            !body.contains("swap(true, Ordering::Release)"),
            "{label} must not reintroduce a swap-guard closure"
        );
    }
}

/// Structural ordering pin for the wakeup-coalescing helper (helper preserves
/// `Ordering::Release`). Reads the helper source and asserts the
/// literal `swap(true, Ordering::Release)`. Behavioral matrix tests
/// verify coalescing semantics but cannot detect ordering changes
/// that still produce the same single-threaded answer; this pin
/// documents the invariant lexically. Resolves source via
/// `term_workspace_root().join("oriterm_mux/src/...")` per the
/// project's path-discovery rule.
#[test]
fn wakeup_coalescing_helper_uses_release_swap() {
    let root = term_workspace_root();
    let helper = fs::read_to_string(root.join("oriterm_mux/src/backend/wakeup/mod.rs"))
        .expect("read backend/wakeup/mod.rs");
    assert!(
        helper.contains("swap(true, Ordering::Release)"),
        "wakeup/mod.rs helper must preserve swap(true, Ordering::Release) ordering"
    );
}
