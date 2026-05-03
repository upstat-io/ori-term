//! Cross-batch `ClearPendingDesktopNotifications` collapse against the
//! main-thread staging buffer.
//!
//! The IO-thread router collapses notifications intra-batch; this
//! module handles the cross-batch case where a `DesktopNotification`
//! reached `notification_buf` in an earlier drain cycle and a clear
//! marker arrives before the next dispatch tick.

use oriterm_mux::MuxNotification;

/// In-place collapse of `ClearPendingDesktopNotifications` against
/// preceding `DesktopNotification` entries in the same staging
/// buffer. For each clear marker at position `i` for pane `P`,
/// removes every `DesktopNotification { pane_id: P, .. }` at
/// positions `< i`. Iteration order preserves remaining markers.
pub(super) fn purge_pending_desktop_notifications(buf: &mut Vec<MuxNotification>) {
    let mut i = 0;
    while i < buf.len() {
        if let MuxNotification::ClearPendingDesktopNotifications(target_pane) = buf[i] {
            let mut j = 0;
            while j < i {
                let drop_it = matches!(
                    &buf[j],
                    MuxNotification::DesktopNotification { pane_id, .. }
                        if *pane_id == target_pane
                );
                if drop_it {
                    buf.remove(j);
                    i -= 1;
                } else {
                    j += 1;
                }
            }
        }
        i += 1;
    }
}
