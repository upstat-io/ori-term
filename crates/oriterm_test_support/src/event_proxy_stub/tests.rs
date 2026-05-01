//! Tests for the [`RecordedProxy`] fixture.

use std::thread;
use std::time::Duration;

use super::{RecordedProxy, RecordedTermEvent};

#[test]
fn fresh_proxy_has_empty_observed_log() {
    let proxy = RecordedProxy::new();
    assert!(proxy.observed().is_empty());
}

#[test]
fn make_mux_wakeup_records_each_fire() {
    let proxy = RecordedProxy::new();
    let wakeup = proxy.make_mux_wakeup();
    wakeup();
    wakeup();
    wakeup();
    let observed = proxy.observed();
    assert_eq!(observed.len(), 3);
    for event in &observed {
        assert_eq!(*event, RecordedTermEvent::MuxWakeup);
    }
}

#[test]
fn wait_for_wakeup_returns_event_when_fired() {
    let proxy = RecordedProxy::new();
    let wakeup = proxy.make_mux_wakeup();
    let handle = thread::spawn(move || {
        thread::sleep(Duration::from_millis(20));
        wakeup();
    });
    let event = proxy
        .wait_for_wakeup(Duration::from_secs(5))
        .expect("expected MuxWakeup within deadline");
    assert_eq!(event, RecordedTermEvent::MuxWakeup);
    handle.join().expect("wakeup thread panicked");
}

#[test]
fn wait_for_wakeup_times_out_when_idle() {
    let proxy = RecordedProxy::new();
    let _wakeup = proxy.make_mux_wakeup();
    let result = proxy.wait_for_wakeup(Duration::from_millis(50));
    assert!(matches!(
        result,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout)
    ));
}

#[test]
fn make_mux_wakeup_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>(_: &T) {}
    let proxy = RecordedProxy::new();
    let wakeup = proxy.make_mux_wakeup();
    assert_send_sync(&wakeup);
}

#[test]
fn wakeup_closure_is_byte_loss_safe_after_receiver_dropped() {
    // Mirrors the `let _ = proxy.send_event(...)` semantics in
    // oriterm/src/app/constructors.rs:make_mux_wakeup. Even if the
    // receiver were dropped, the closure must not panic.
    let proxy = RecordedProxy::new();
    let wakeup = proxy.make_mux_wakeup();
    drop(proxy);
    // Calling the wakeup after dropping the proxy must NOT panic.
    wakeup();
}
