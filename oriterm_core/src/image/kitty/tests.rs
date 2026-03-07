//! Tests for Kitty Graphics Protocol parsing and execution.

use std::sync::{Arc, Mutex};

use vte::ansi::Processor;

use super::parse::{KittyAction, KittyError, KittyTransmission, parse_kitty_command};
use crate::event::{Event, EventListener};
use crate::image::ImageId;
use crate::term::Term;
use crate::theme::Theme;

#[test]
fn parse_single_key() {
    let cmd = parse_kitty_command(b"a=t").unwrap();
    assert_eq!(cmd.action, KittyAction::Transmit);
}

#[test]
fn parse_multiple_keys() {
    let cmd = parse_kitty_command(b"a=T,i=42,f=32,s=100,v=50").unwrap();
    assert_eq!(cmd.action, KittyAction::TransmitAndPlace);
    assert_eq!(cmd.image_id, Some(42));
    assert_eq!(cmd.format, 32);
    assert_eq!(cmd.source_width, 100);
    assert_eq!(cmd.source_height, 50);
}

#[test]
fn parse_missing_value_ignored() {
    // Keys without '=' are skipped.
    let cmd = parse_kitty_command(b"a=t,x").unwrap();
    assert_eq!(cmd.action, KittyAction::Transmit);
}

#[test]
fn parse_unknown_key_ignored() {
    // Unknown keys logged but don't error.
    let cmd = parse_kitty_command(b"a=t,Z=99").unwrap();
    assert_eq!(cmd.action, KittyAction::Transmit);
}

#[test]
fn parse_all_actions() {
    assert_eq!(
        parse_kitty_command(b"a=t").unwrap().action,
        KittyAction::Transmit
    );
    assert_eq!(
        parse_kitty_command(b"a=T").unwrap().action,
        KittyAction::TransmitAndPlace
    );
    assert_eq!(
        parse_kitty_command(b"a=p").unwrap().action,
        KittyAction::Place
    );
    assert_eq!(
        parse_kitty_command(b"a=d").unwrap().action,
        KittyAction::Delete
    );
    assert_eq!(
        parse_kitty_command(b"a=f").unwrap().action,
        KittyAction::Frame
    );
    assert_eq!(
        parse_kitty_command(b"a=a").unwrap().action,
        KittyAction::Animate
    );
    assert_eq!(
        parse_kitty_command(b"a=q").unwrap().action,
        KittyAction::Query
    );
}

#[test]
fn parse_transmission_methods() {
    assert_eq!(
        parse_kitty_command(b"t=d").unwrap().transmission,
        KittyTransmission::Direct
    );
    assert_eq!(
        parse_kitty_command(b"t=f").unwrap().transmission,
        KittyTransmission::File
    );
    assert_eq!(
        parse_kitty_command(b"t=t").unwrap().transmission,
        KittyTransmission::TempFile
    );
    assert_eq!(
        parse_kitty_command(b"t=s").unwrap().transmission,
        KittyTransmission::SharedMemory
    );
}

#[test]
fn parse_base64_payload() {
    // "AAAA" in base64 = 3 zero bytes.
    let cmd = parse_kitty_command(b"f=32;AAAA").unwrap();
    assert_eq!(cmd.payload, vec![0, 0, 0]);
}

#[test]
fn parse_base64_payload_with_padding() {
    // "YQ==" in base64 = b"a".
    let cmd = parse_kitty_command(b"f=32;YQ==").unwrap();
    assert_eq!(cmd.payload, b"a");
}

#[test]
fn parse_rgba_transmission() {
    // 1x1 RGBA pixel (4 bytes) = AAAAAA== in base64.
    let cmd = parse_kitty_command(b"a=T,f=32,s=1,v=1;AAAAAA==").unwrap();
    assert_eq!(cmd.action, KittyAction::TransmitAndPlace);
    assert_eq!(cmd.format, 32);
    assert_eq!(cmd.source_width, 1);
    assert_eq!(cmd.source_height, 1);
    assert_eq!(cmd.payload.len(), 4);
}

#[test]
fn parse_chunked_transfer_more() {
    let cmd = parse_kitty_command(b"a=t,m=1;AAAA").unwrap();
    assert!(cmd.more_data);
}

#[test]
fn parse_chunked_transfer_final() {
    let cmd = parse_kitty_command(b"a=t,m=0;AAAA").unwrap();
    assert!(!cmd.more_data);
}

#[test]
fn parse_delete_command() {
    let cmd = parse_kitty_command(b"a=d,d=i,i=42").unwrap();
    assert_eq!(cmd.action, KittyAction::Delete);
    assert_eq!(cmd.delete_specifier, Some(b'i'));
    assert_eq!(cmd.image_id, Some(42));
}

#[test]
fn parse_placement_params() {
    let cmd = parse_kitty_command(b"a=p,i=1,p=2,c=10,r=5,X=3,Y=4,z=-1,C=1").unwrap();
    assert_eq!(cmd.action, KittyAction::Place);
    assert_eq!(cmd.image_id, Some(1));
    assert_eq!(cmd.placement_id, Some(2));
    assert_eq!(cmd.display_cols, Some(10));
    assert_eq!(cmd.display_rows, Some(5));
    assert_eq!(cmd.cell_x_offset, 3);
    assert_eq!(cmd.cell_y_offset, 4);
    assert_eq!(cmd.z_index, -1);
    assert!(cmd.no_cursor_move);
}

#[test]
fn parse_cursor_movement_suppression() {
    let cmd = parse_kitty_command(b"C=1").unwrap();
    assert!(cmd.no_cursor_move);

    let cmd = parse_kitty_command(b"C=0").unwrap();
    assert!(!cmd.no_cursor_move);
}

#[test]
fn parse_quiet_modes() {
    assert_eq!(parse_kitty_command(b"q=0").unwrap().quiet, 0);
    assert_eq!(parse_kitty_command(b"q=1").unwrap().quiet, 1);
    assert_eq!(parse_kitty_command(b"q=2").unwrap().quiet, 2);
}

#[test]
fn parse_query_command() {
    let cmd = parse_kitty_command(b"i=31,s=1,v=1,a=q,t=d,f=24;AAAA").unwrap();
    assert_eq!(cmd.action, KittyAction::Query);
    assert_eq!(cmd.image_id, Some(31));
    assert_eq!(cmd.format, 24);
}

#[test]
fn invalid_base64_error() {
    let result = parse_kitty_command(b"f=32;!!!invalid!!!");
    assert!(matches!(result, Err(KittyError::InvalidBase64)));
}

#[test]
fn empty_payload() {
    let cmd = parse_kitty_command(b"a=t,i=1").unwrap();
    assert!(cmd.payload.is_empty());
}

#[test]
fn empty_control_data() {
    let cmd = parse_kitty_command(b";AAAA").unwrap();
    assert_eq!(cmd.payload, vec![0, 0, 0]);
}

#[test]
fn default_action_is_transmit_and_place() {
    let cmd = parse_kitty_command(b"f=32,s=1,v=1").unwrap();
    assert_eq!(cmd.action, KittyAction::TransmitAndPlace);
}

#[test]
fn unicode_placeholder_mode() {
    let cmd = parse_kitty_command(b"U=1").unwrap();
    assert!(cmd.unicode_placeholder);

    let cmd = parse_kitty_command(b"U=0").unwrap();
    assert!(!cmd.unicode_placeholder);
}

#[test]
fn negative_z_index() {
    let cmd = parse_kitty_command(b"z=-5").unwrap();
    assert_eq!(cmd.z_index, -5);
}

#[test]
fn delete_uppercase_variants() {
    for spec in [b'A', b'I', b'P', b'C', b'N', b'R', b'X', b'Y', b'Z'] {
        let input = format!("a=d,d={}", spec as char);
        let cmd = parse_kitty_command(input.as_bytes()).unwrap();
        assert_eq!(cmd.delete_specifier, Some(spec));
    }
}

#[test]
fn source_rect_params() {
    let cmd = parse_kitty_command(b"x=10,y=20,s=100,v=50").unwrap();
    assert_eq!(cmd.source_x, 10);
    assert_eq!(cmd.source_y, 20);
    assert_eq!(cmd.source_width, 100);
    assert_eq!(cmd.source_height, 50);
}

// --- Handler-level tests (Term + VTE processor) ---

/// Event listener that records PtyWrite events for response verification.
#[derive(Clone)]
struct RecordingListener {
    events: Arc<Mutex<Vec<String>>>,
}

impl RecordingListener {
    fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn pty_writes(&self) -> Vec<String> {
        self.events
            .lock()
            .expect("lock poisoned")
            .iter()
            .filter_map(|e| e.strip_prefix("pty:").map(String::from))
            .collect()
    }
}

impl EventListener for RecordingListener {
    fn send_event(&self, event: Event) {
        let s = match &event {
            Event::PtyWrite(data) => format!("pty:{data}"),
            other => format!("{other:?}"),
        };
        self.events.lock().expect("lock poisoned").push(s);
    }
}

fn term_with_recorder() -> (Term<RecordingListener>, RecordingListener) {
    let listener = RecordingListener::new();
    let term = Term::new(24, 80, 100, Theme::default(), listener.clone());
    (term, listener)
}

fn feed(term: &mut impl vte::ansi::Handler, bytes: &[u8]) {
    let mut processor: Processor = Processor::new();
    processor.advance(term, bytes);
}

/// Build an APC Kitty graphics command: ESC _ G <body> ESC \
fn kitty_apc(body: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(b"\x1b_G");
    buf.extend_from_slice(body.as_bytes());
    buf.extend_from_slice(b"\x1b\\");
    buf
}

#[test]
fn handler_unknown_image_id_enoent() {
    let (mut term, listener) = term_with_recorder();
    // Place a nonexistent image ID.
    let apc = kitty_apc("a=p,i=999");
    feed(&mut term, &apc);

    let writes = listener.pty_writes();
    assert_eq!(writes.len(), 1);
    assert!(
        writes[0].contains("ENOENT"),
        "expected ENOENT, got: {}",
        writes[0]
    );
}

#[test]
fn handler_delete_by_placement_id() {
    let (mut term, _listener) = term_with_recorder();

    // Transmit a 1x1 RGBA image (4 bytes = AAAAAA== in base64).
    let transmit = kitty_apc("a=t,i=1,f=32,s=1,v=1,q=2;AAAAAA==");
    feed(&mut term, &transmit);
    assert_eq!(term.image_cache().image_count(), 1);

    // Place it twice with different placement IDs.
    let place1 = kitty_apc("a=p,i=1,p=10,q=2");
    let place2 = kitty_apc("a=p,i=1,p=20,q=2");
    feed(&mut term, &place1);
    feed(&mut term, &place2);
    assert_eq!(term.image_cache().placement_count(), 2);

    // Delete only placement 10.
    let delete = kitty_apc("a=d,d=p,i=1,p=10,q=2");
    feed(&mut term, &delete);
    assert_eq!(term.image_cache().placement_count(), 1);

    // Remaining placement should be the one with id=20.
    let placements = term.image_cache().placements_in_viewport(
        crate::grid::StableRowIndex(0),
        crate::grid::StableRowIndex(u64::MAX),
    );
    assert_eq!(placements.len(), 1);
    assert_eq!(placements[0].placement_id, Some(20));
}

#[test]
fn handler_chunked_exceeds_limit() {
    let (mut term, listener) = term_with_recorder();

    // Set a very small max single image size to trigger the limit.
    term.image_cache_mut().set_max_single_image(16);

    // Send chunk 1 (m=1 = more data follows). 12 decoded bytes.
    let chunk1 = kitty_apc("a=t,i=1,f=32,s=2,v=2,m=1,q=2;AAAAAAAAAAAAAAAA");
    feed(&mut term, &chunk1);
    // Loading image should be in progress.
    assert!(term.image_cache().image_count() == 0, "not stored yet");

    // Send chunk 2 — total will exceed 16 bytes. Should discard.
    let chunk2 = kitty_apc("a=t,i=1,f=32,s=2,v=2,m=1,q=2;AAAAAAAAAAAAAAAA");
    feed(&mut term, &chunk2);

    // After exceeding limit, loading_image is discarded.
    // Send final chunk to trigger finalization — should fail.
    let final_chunk = kitty_apc("a=t,i=1,f=32,s=2,v=2,m=0,q=0;AAAA");
    feed(&mut term, &final_chunk);

    // No image should be stored (accumulated data was discarded).
    // The final chunk alone would try to store but dimensions won't match payload.
    assert_eq!(term.image_cache().image_count(), 0);

    // Should have received an error response (not OK).
    let writes = listener.pty_writes();
    assert!(!writes.is_empty());
    let last = writes.last().unwrap();
    assert!(!last.contains("OK"), "expected error, got: {last}");
}

#[test]
fn handler_unicode_placeholder_skips_placement() {
    let (mut term, _listener) = term_with_recorder();

    // Transmit+place with U=1: image stored but no placement created.
    let apc = kitty_apc("a=T,i=1,f=32,s=1,v=1,U=1,q=2;AAAAAA==");
    feed(&mut term, &apc);

    // Image should be stored.
    assert_eq!(term.image_cache().image_count(), 1);
    // But no placement (deferred to U+10EEEE chars in cells).
    assert_eq!(term.image_cache().placement_count(), 0);
}

#[test]
fn handler_unicode_placeholder_place_skips() {
    let (mut term, _listener) = term_with_recorder();

    // First transmit without U=1.
    let transmit = kitty_apc("a=t,i=1,f=32,s=1,v=1,q=2;AAAAAA==");
    feed(&mut term, &transmit);
    assert_eq!(term.image_cache().image_count(), 1);

    // Place with U=1: should not create a placement.
    let place = kitty_apc("a=p,i=1,U=1,q=2");
    feed(&mut term, &place);
    assert_eq!(term.image_cache().placement_count(), 0);
}

// --- Animation tests ---

#[test]
fn handler_frame_adds_animation() {
    let (mut term, _listener) = term_with_recorder();

    // Transmit a 1x1 RGBA image.
    let transmit = kitty_apc("a=t,i=1,f=32,s=1,v=1,q=2;AAAAAA==");
    feed(&mut term, &transmit);
    assert_eq!(term.image_cache().image_count(), 1);
    assert!(term.image_cache().animation_state(ImageId(1)).is_none());

    // Add a frame (a=f) — promotes image to animated.
    // z=100 → 100ms gap, X=1 → overwrite composition.
    let frame = kitty_apc("a=f,i=1,f=32,s=1,v=1,z=100,X=1,q=2;AAAAAA==");
    feed(&mut term, &frame);

    let state = term.image_cache().animation_state(ImageId(1));
    assert!(state.is_some(), "image should now be animated");
    assert_eq!(state.unwrap().total_frames, 2);
}

#[test]
fn handler_frame_adds_multiple_frames() {
    let (mut term, _listener) = term_with_recorder();

    let transmit = kitty_apc("a=t,i=1,f=32,s=1,v=1,q=2;AAAAAA==");
    feed(&mut term, &transmit);

    // Add 3 more frames.
    for _ in 0..3 {
        let frame = kitty_apc("a=f,i=1,f=32,s=1,v=1,z=50,q=2;AAAAAA==");
        feed(&mut term, &frame);
    }

    let state = term
        .image_cache()
        .animation_state(ImageId(1))
        .expect("animated");
    assert_eq!(state.total_frames, 4);
}

#[test]
fn handler_frame_nonexistent_image_enoent() {
    let (mut term, listener) = term_with_recorder();

    let frame = kitty_apc("a=f,i=999,f=32,s=1,v=1,q=0;AAAAAA==");
    feed(&mut term, &frame);

    let writes = listener.pty_writes();
    assert!(!writes.is_empty());
    assert!(writes.last().unwrap().contains("ENOENT"), "expected ENOENT");
}

#[test]
fn handler_animate_stop_and_run() {
    let (mut term, _listener) = term_with_recorder();

    // Set up an animated image.
    let transmit = kitty_apc("a=t,i=1,f=32,s=1,v=1,q=2;AAAAAA==");
    feed(&mut term, &transmit);
    let frame = kitty_apc("a=f,i=1,f=32,s=1,v=1,z=50,q=2;AAAAAA==");
    feed(&mut term, &frame);

    // Stop animation (s=1).
    let stop = kitty_apc("a=a,i=1,s=1,q=2");
    feed(&mut term, &stop);
    assert!(
        term.image_cache()
            .animation_state(ImageId(1))
            .unwrap()
            .paused
    );

    // Run animation (s=3).
    let run = kitty_apc("a=a,i=1,s=3,q=2");
    feed(&mut term, &run);
    assert!(
        !term
            .image_cache()
            .animation_state(ImageId(1))
            .unwrap()
            .paused
    );
}

#[test]
fn handler_animate_set_loops() {
    let (mut term, _listener) = term_with_recorder();

    let transmit = kitty_apc("a=t,i=1,f=32,s=1,v=1,q=2;AAAAAA==");
    feed(&mut term, &transmit);
    let frame = kitty_apc("a=f,i=1,f=32,s=1,v=1,z=50,q=2;AAAAAA==");
    feed(&mut term, &frame);

    // Set loop count to 5 (v=5).
    let animate = kitty_apc("a=a,i=1,v=5,q=2");
    feed(&mut term, &animate);

    let state = term
        .image_cache()
        .animation_state(ImageId(1))
        .expect("animated");
    assert_eq!(state.loop_count, Some(5));
}

#[test]
fn handler_animate_set_current_frame() {
    let (mut term, _listener) = term_with_recorder();

    let transmit = kitty_apc("a=t,i=1,f=32,s=1,v=1,q=2;AAAAAA==");
    feed(&mut term, &transmit);
    // Add 2 more frames (total 3).
    for _ in 0..2 {
        let frame = kitty_apc("a=f,i=1,f=32,s=1,v=1,z=50,q=2;AAAAAA==");
        feed(&mut term, &frame);
    }

    // Jump to frame 2 (c=2, 1-based → internal frame index 1).
    let animate = kitty_apc("a=a,i=1,c=2,q=2");
    feed(&mut term, &animate);

    let state = term
        .image_cache()
        .animation_state(ImageId(1))
        .expect("animated");
    assert_eq!(state.current_frame, 1);
}
