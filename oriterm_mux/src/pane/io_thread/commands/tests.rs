//! Tests for PaneIoCommand.

use oriterm_core::Selection;
use oriterm_core::grid::StableRowIndex;
use oriterm_core::index::Side;

use super::PaneIoCommand;

/// Static assertion that `PaneIoCommand` is `Send`.
#[test]
fn pane_io_command_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<PaneIoCommand>();
}

/// Verify the manual `Debug` impl produces readable output for simple variants.
#[test]
fn debug_simple_variants() {
    let cmd = PaneIoCommand::SetCellDimensions {
        width: 16,
        height: 32,
    };
    let s = format!("{cmd:?}");
    assert!(
        s.contains("SetCellDimensions"),
        "expected 'SetCellDimensions' in: {s}"
    );
    assert!(s.contains("16"), "expected width in: {s}");
    assert!(s.contains("32"), "expected height in: {s}");

    assert_eq!(format!("{:?}", PaneIoCommand::Shutdown), "Shutdown");
    assert_eq!(
        format!("{:?}", PaneIoCommand::ScrollToBottom),
        "ScrollToBottom"
    );
    assert_eq!(
        format!("{:?}", PaneIoCommand::ScrollDisplay(-5)),
        "ScrollDisplay(-5)"
    );
}

/// Verify `Debug` doesn't panic on reply-channel variants.
#[test]
fn debug_reply_variants_no_panic() {
    let (tx, _rx) = crossbeam_channel::bounded(1);
    let sel = Selection::new_char(StableRowIndex(0), 0, Side::Left);
    let cmd = PaneIoCommand::ExtractText {
        selection: sel,
        reply: tx,
    };
    // Should not panic — reply field is skipped in Debug.
    let s = format!("{cmd:?}");
    assert!(s.contains("ExtractText"), "expected 'ExtractText' in: {s}");
}
