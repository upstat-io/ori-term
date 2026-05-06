//! Pure decision + side-effect protocol for the mux event pump.
//!
//! `pump_mux_events_core` carves the gate check, daemon-connectivity check,
//! and `poll_events → drain_notifications → empty-check → purge` ordering
//! out of `App::pump_mux_events` so the protocol is testable against a
//! recording `MuxBackend` without an `App` fixture. The App-coupled
//! callbacks (`handle_daemon_disconnect`, `with_drained_notifications`)
//! are dispatched by the caller based on the returned [`PumpResult`].

use oriterm_mux::MuxNotification;
use oriterm_mux::backend::MuxBackend;

use super::notification_purge::purge_pending_desktop_notifications;

/// Disposition of a pump cycle, decided by [`pump_mux_events_core`].
///
/// Each variant maps to a distinct App-layer side effect:
/// - `NoMux` / `NoPendingWakeup` / `EmptyDrain` — no follow-up; App takes
///   no further action.
/// - `DaemonDisconnect` — App must log + invoke `handle_daemon_disconnect`.
/// - `HasNotifications` — App must dispatch `with_drained_notifications`.
///
/// Pinned by tests in `mux_pump/tests.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub(super) enum PumpResult {
    NoMux,
    DaemonDisconnect,
    NoPendingWakeup,
    EmptyDrain,
    HasNotifications,
}

/// Drive one mux pump cycle: gate check, daemon-connectivity check,
/// `poll_events` → `drain_notifications` → empty-check → purge ordering.
///
/// Returns a [`PumpResult`] describing the disposition. App-coupled
/// side effects (`handle_daemon_disconnect`, `with_drained_notifications`)
/// are NOT performed here — the caller dispatches on the return value.
///
/// Free function (not `impl App`) so the side-effect protocol is testable
/// in isolation from the App's window/session/clipboard state.
pub(super) fn pump_mux_events_core(
    mux: Option<&mut (dyn MuxBackend + '_)>,
    notification_buf: &mut Vec<MuxNotification>,
) -> PumpResult {
    let Some(mux) = mux else {
        return PumpResult::NoMux;
    };

    if mux.is_daemon_mode() && !mux.is_connected() {
        return PumpResult::DaemonDisconnect;
    }

    if !mux.has_pending_wakeup() {
        return PumpResult::NoPendingWakeup;
    }

    mux.poll_events();
    mux.drain_notifications(notification_buf);

    if notification_buf.is_empty() {
        return PumpResult::EmptyDrain;
    }

    purge_pending_desktop_notifications(notification_buf);
    PumpResult::HasNotifications
}
