use vte::ansi::PerformAction;

use super::UncatalogedDetector;

#[test]
fn known_tuple_is_not_double_counted() {
    let mut detector = UncatalogedDetector::new();
    let actions = vec![
        PerformAction::CsiDispatch {
            params: vec![vec![0]],
            intermediates: vec![],
            ignore: false,
            action: 'H',
        },
        // Same CSI H again — should not increase count.
        PerformAction::CsiDispatch {
            params: vec![vec![1, 1]],
            intermediates: vec![],
            ignore: false,
            action: 'H',
        },
    ];
    detector.feed_actions(&actions);
    // Both CSI H have the same signature (params are stripped).
    assert_eq!(detector.seen().len(), 1);
}

#[test]
fn unknown_tuple_is_recorded_in_memory() {
    let mut detector = UncatalogedDetector::new();
    let actions = vec![PerformAction::CsiDispatch {
        params: vec![vec![99]],
        intermediates: vec![b'?'],
        ignore: false,
        action: 'z',
    }];
    detector.feed_actions(&actions);
    assert_eq!(detector.seen().len(), 1);
    let sig = detector.seen().iter().next().unwrap();
    assert_eq!(sig.0, "CSI");
    assert_eq!(sig.2, "z");
}

#[test]
fn print_and_put_are_not_catalogable() {
    let mut detector = UncatalogedDetector::new();
    let actions = vec![
        PerformAction::Print { c: 'A' },
        PerformAction::Put { byte: 0x41 },
        PerformAction::Unhook,
        PerformAction::ApcStart,
        PerformAction::ApcPut { byte: 0x42 },
        PerformAction::ApcEnd,
    ];
    detector.feed_actions(&actions);
    assert!(
        detector.seen().is_empty(),
        "data/framing actions should not be catalogable"
    );
}

#[test]
fn osc_dispatch_extracts_command_number() {
    let mut detector = UncatalogedDetector::new();
    let actions = vec![PerformAction::OscDispatch {
        params: vec![b"52".to_vec(), b"c;data".to_vec()],
        bell_terminated: false,
    }];
    detector.feed_actions(&actions);
    assert_eq!(detector.seen().len(), 1);
    let sig = detector.seen().iter().next().unwrap();
    assert_eq!(sig.0, "OSC");
    assert_eq!(sig.2, "52");
}

#[test]
fn esc_dispatch_converts_byte_to_char() {
    let mut detector = UncatalogedDetector::new();
    let actions = vec![PerformAction::EscDispatch {
        intermediates: vec![],
        ignore: false,
        byte: b'7',
    }];
    detector.feed_actions(&actions);
    assert_eq!(detector.seen().len(), 1);
    let sig = detector.seen().iter().next().unwrap();
    assert_eq!(sig.0, "ESC");
    assert_eq!(sig.2, "7");
}

#[test]
fn serialize_and_read_round_trip() {
    let mut detector = UncatalogedDetector::new();
    detector.feed_actions(&[
        PerformAction::CsiDispatch {
            params: vec![],
            intermediates: vec![b'?'],
            ignore: false,
            action: 'h',
        },
        PerformAction::Execute { byte: 0x0a },
    ]);

    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("uncataloged");
    detector.serialize_to_dir(&output).unwrap();

    let read_back = super::read_accumulated_tuples(&output).unwrap();
    assert_eq!(read_back.len(), 2);
    // Verify the CSI ? h tuple round-trips.
    let has_csi = read_back
        .iter()
        .any(|(cat, _, fb)| cat == "CSI" && fb == "h");
    assert!(has_csi, "CSI ?h should round-trip: {read_back:?}");
}

#[test]
fn read_accumulated_tuples_handles_missing_dir() {
    let dir = std::path::PathBuf::from("/nonexistent/path/uncataloged");
    let result = super::read_accumulated_tuples(&dir).unwrap();
    assert!(result.is_empty());
}
