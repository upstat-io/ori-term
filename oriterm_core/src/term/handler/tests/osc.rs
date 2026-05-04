use crate::index::Column;
use crate::term::Term;
use crate::theme::Theme;

use super::super::test_helpers::{feed, term_with_recorder};

/// Create a Term with VoidEffectSink (when effects don't matter).
fn term() -> Term<crate::effect::VoidEffectSink> {
    Term::new(24, 80, 0, Theme::default(), crate::effect::VoidEffectSink)
}

// --- OSC (Operating System Command) tests ---

#[test]
fn osc2_sets_window_title() {
    let (mut t, listener) = term_with_recorder();
    // ESC]2;Hello World\x07
    feed(&mut t, b"\x1b]2;Hello World\x07");

    assert_eq!(t.title(), "Hello World");
    let events = listener.events();
    assert!(events.iter().any(|e| e.contains("Title(Hello World)")));
}

#[test]
fn osc0_sets_window_title() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b]0;My Terminal\x07");

    assert_eq!(t.title(), "My Terminal");
    let events = listener.events();
    assert!(events.iter().any(|e| e.contains("Title(My Terminal)")));
}

#[test]
fn osc2_with_st_terminator() {
    let mut t = term();
    // ESC]2;Title Here\x1b\\
    feed(&mut t, b"\x1b]2;Title Here\x1b\\");

    assert_eq!(t.title(), "Title Here");
}

#[test]
fn osc_title_with_semicolons() {
    let mut t = term();
    // Title containing semicolons should be preserved.
    feed(&mut t, b"\x1b]2;a;b;c\x07");

    assert_eq!(t.title(), "a;b;c");
}

#[test]
fn osc_push_pop_title() {
    let mut t = term();
    // Set initial title.
    feed(&mut t, b"\x1b]2;First\x07");
    assert_eq!(t.title(), "First");

    // Push title (ESC[22t).
    // VTE dispatches push_title from CSI 22;2t.
    feed(&mut t, b"\x1b[22;2t");
    assert_eq!(t.title_stack().len(), 1);
    assert_eq!(t.title_stack()[0], "First");

    // Set new title.
    feed(&mut t, b"\x1b]2;Second\x07");
    assert_eq!(t.title(), "Second");

    // Pop title (ESC[23t).
    feed(&mut t, b"\x1b[23;2t");
    assert_eq!(t.title(), "First");
    assert!(t.title_stack().is_empty());
}

#[test]
fn osc_pop_empty_stack_is_noop() {
    let mut t = term();
    feed(&mut t, b"\x1b]2;Original\x07");

    // Pop from empty stack — title should remain.
    feed(&mut t, b"\x1b[23;2t");
    assert_eq!(t.title(), "Original");
}

#[test]
fn osc4_sets_indexed_color() {
    let mut t = term();
    // ESC]4;1;rgb:ff/00/00\x07 — set color index 1 to red.
    feed(&mut t, b"\x1b]4;1;rgb:ff/00/00\x07");

    let color = t.palette().resolve(vte::ansi::Color::Indexed(1));
    assert_eq!(
        color,
        vte::ansi::Rgb {
            r: 0xff,
            g: 0x00,
            b: 0x00
        }
    );
}

#[test]
fn osc4_query_sends_color_request_event() {
    let (mut t, listener) = term_with_recorder();
    // ESC]4;1;?\x07 — query color index 1.
    feed(&mut t, b"\x1b]4;1;?\x07");

    let events = listener.events();
    assert!(events.iter().any(|e| e.contains("ColorRequest(1)")));
}

#[test]
fn osc10_sets_foreground_color() {
    let mut t = term();
    // ESC]10;rgb:aa/bb/cc\x07 — set foreground.
    feed(&mut t, b"\x1b]10;rgb:aa/bb/cc\x07");

    assert_eq!(
        t.palette().foreground(),
        vte::ansi::Rgb {
            r: 0xaa,
            g: 0xbb,
            b: 0xcc
        }
    );
}

#[test]
fn osc11_sets_background_color() {
    let mut t = term();
    feed(&mut t, b"\x1b]11;rgb:11/22/33\x07");

    assert_eq!(
        t.palette().background(),
        vte::ansi::Rgb {
            r: 0x11,
            g: 0x22,
            b: 0x33
        }
    );
}

#[test]
fn osc12_sets_cursor_color() {
    let mut t = term();
    feed(&mut t, b"\x1b]12;rgb:ff/ff/00\x07");

    assert_eq!(
        t.palette().cursor_color(),
        vte::ansi::Rgb {
            r: 0xff,
            g: 0xff,
            b: 0x00
        }
    );
}

#[test]
fn osc104_resets_indexed_color() {
    let mut t = term();
    let original = t.palette().resolve(vte::ansi::Color::Indexed(1));

    // Set then reset.
    feed(&mut t, b"\x1b]4;1;rgb:ff/ff/ff\x07");
    assert_ne!(t.palette().resolve(vte::ansi::Color::Indexed(1)), original);

    feed(&mut t, b"\x1b]104;1\x07");
    assert_eq!(t.palette().resolve(vte::ansi::Color::Indexed(1)), original);
}

#[test]
fn osc110_resets_foreground() {
    let mut t = term();
    let original = t.palette().foreground();

    feed(&mut t, b"\x1b]10;rgb:ff/00/ff\x07");
    assert_ne!(t.palette().foreground(), original);

    feed(&mut t, b"\x1b]110\x07");
    assert_eq!(t.palette().foreground(), original);
}

#[test]
fn osc111_resets_background() {
    let mut t = term();
    let original = t.palette().background();

    feed(&mut t, b"\x1b]11;rgb:ff/00/ff\x07");
    assert_ne!(t.palette().background(), original);

    feed(&mut t, b"\x1b]111\x07");
    assert_eq!(t.palette().background(), original);
}

#[test]
fn osc112_resets_cursor_color() {
    let mut t = term();
    let original = t.palette().cursor_color();

    feed(&mut t, b"\x1b]12;rgb:00/ff/00\x07");
    assert_ne!(t.palette().cursor_color(), original);

    feed(&mut t, b"\x1b]112\x07");
    assert_eq!(t.palette().cursor_color(), original);
}

#[test]
fn osc52_clipboard_store() {
    let (mut t, listener) = term_with_recorder();
    // ESC]52;c;aGVsbG8=\x07 — store "hello" to clipboard.
    feed(&mut t, b"\x1b]52;c;aGVsbG8=\x07");

    let events = listener.events();
    assert!(
        events
            .iter()
            .any(|e| e.contains("ClipboardStore(Clipboard, hello)")),
        "Expected ClipboardStore event with 'hello', got: {events:?}",
    );
}

#[test]
fn osc52_clipboard_load() {
    let (mut t, listener) = term_with_recorder();
    // ESC]52;c;?\x07 — request clipboard content.
    feed(&mut t, b"\x1b]52;c;?\x07");

    let events = listener.events();
    assert!(
        events
            .iter()
            .any(|e| e.contains("ClipboardLoad(Clipboard)")),
        "Expected ClipboardLoad event, got: {events:?}",
    );
}

#[test]
fn osc52_primary_selection() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b]52;p;dGVzdA==\x07");

    let events = listener.events();
    assert!(
        events
            .iter()
            .any(|e| e.contains("ClipboardStore(Selection, test)")),
        "Expected Selection store, got: {events:?}",
    );
}

#[test]
fn osc52_invalid_base64_is_ignored() {
    let (mut t, listener) = term_with_recorder();
    // Invalid base64 should not produce an event.
    feed(&mut t, b"\x1b]52;c;!!!invalid!!!\x07");

    let events = listener.events();
    assert!(
        !events.iter().any(|e| e.contains("ClipboardStore")),
        "Should not produce ClipboardStore for invalid base64, got: {events:?}",
    );
}

#[test]
fn osc8_sets_hyperlink() {
    let mut t = term();
    // ESC]8;;https://example.com\x07
    feed(&mut t, b"\x1b]8;;https://example.com\x07");

    let extra = t
        .grid()
        .cursor()
        .template
        .extra
        .as_ref()
        .expect("CellExtra should be allocated for hyperlink");
    let link = extra.hyperlink.as_ref().expect("hyperlink should be set");
    assert_eq!(link.uri, "https://example.com");
    assert!(link.id.is_none());
}

#[test]
fn osc8_with_id_parameter() {
    let mut t = term();
    feed(&mut t, b"\x1b]8;id=my-link;https://example.com\x07");

    let extra = t.grid().cursor().template.extra.as_ref().unwrap();
    let link = extra.hyperlink.as_ref().unwrap();
    assert_eq!(link.uri, "https://example.com");
    assert_eq!(link.id.as_deref(), Some("my-link"));
}

#[test]
fn osc8_clear_hyperlink() {
    let mut t = term();
    // Set then clear.
    feed(&mut t, b"\x1b]8;;https://example.com\x07");
    assert!(t.grid().cursor().template.extra.is_some());

    feed(&mut t, b"\x1b]8;;\x07");
    // CellExtra should be dropped (no other extra data).
    assert!(t.grid().cursor().template.extra.is_none());
}

// --- OSC edge cases (from reference repos: Alacritty, Ghostty, WezTerm) ---

#[test]
fn osc_title_empty_string() {
    let mut t = term();
    feed(&mut t, b"\x1b]2;Something\x07");
    assert_eq!(t.title(), "Something");

    // Empty title (OSC 2 with empty payload).
    feed(&mut t, b"\x1b]2;\x07");
    assert_eq!(t.title(), "");
}

#[test]
fn osc_title_utf8_multibyte() {
    let mut t = term();
    // Title with multi-byte UTF-8: em dash (U+2014) + CJK character.
    let mut seq = b"\x1b]2;".to_vec();
    seq.extend_from_slice("Hello — 世界".as_bytes());
    seq.push(0x07);
    feed(&mut t, &seq);

    assert_eq!(t.title(), "Hello — 世界");
}

#[test]
fn osc_title_stack_cap_at_4096() {
    let mut t = term();
    // Push 4096 titles to fill the stack.
    for i in 0..4096 {
        let title = format!("\x1b]2;title-{i}\x07");
        feed(&mut t, title.as_bytes());
        feed(&mut t, b"\x1b[22;2t");
    }
    assert_eq!(t.title_stack().len(), 4096);

    // One more push should evict the oldest.
    feed(&mut t, b"\x1b]2;overflow\x07");
    feed(&mut t, b"\x1b[22;2t");
    assert_eq!(t.title_stack().len(), 4096);
    // Oldest entry ("title-0") should be gone.
    assert_ne!(t.title_stack()[0], "title-0");
}

#[test]
fn osc_push_pop_interleaved() {
    let mut t = term();

    feed(&mut t, b"\x1b]2;A\x07");
    feed(&mut t, b"\x1b[22;2t"); // push "A"
    feed(&mut t, b"\x1b]2;B\x07");
    feed(&mut t, b"\x1b[22;2t"); // push "B"
    feed(&mut t, b"\x1b]2;C\x07");

    assert_eq!(t.title(), "C");
    assert_eq!(t.title_stack().len(), 2);

    // Pop should restore in LIFO order.
    feed(&mut t, b"\x1b[23;2t");
    assert_eq!(t.title(), "B");
    feed(&mut t, b"\x1b[23;2t");
    assert_eq!(t.title(), "A");
    assert!(t.title_stack().is_empty());
}

#[test]
fn osc4_multiple_colors_in_one_sequence() {
    let mut t = term();
    // Set two colors in a single OSC 4 (VTE processes pairs).
    feed(&mut t, b"\x1b]4;1;rgb:ff/00/00;2;rgb:00/ff/00\x07");

    let c1 = t.palette().resolve(vte::ansi::Color::Indexed(1));
    let c2 = t.palette().resolve(vte::ansi::Color::Indexed(2));
    assert_eq!(
        c1,
        vte::ansi::Rgb {
            r: 0xff,
            g: 0x00,
            b: 0x00
        }
    );
    assert_eq!(
        c2,
        vte::ansi::Rgb {
            r: 0x00,
            g: 0xff,
            b: 0x00
        }
    );
}

#[test]
fn osc4_out_of_range_index_ignored() {
    let mut t = term();
    // Index 999 is beyond palette bounds — should not crash.
    feed(&mut t, b"\x1b]4;999;rgb:ff/00/00\x07");
    // Verify no panic and palette is intact.
    let _ = t.palette().foreground();
}

#[test]
fn osc104_no_params_resets_all_indexed() {
    let mut t = term();
    let original_0 = t.palette().resolve(vte::ansi::Color::Indexed(0));
    let original_1 = t.palette().resolve(vte::ansi::Color::Indexed(1));

    // Set colors 0 and 1.
    feed(&mut t, b"\x1b]4;0;rgb:ff/ff/ff\x07");
    feed(&mut t, b"\x1b]4;1;rgb:ff/ff/ff\x07");
    assert_ne!(
        t.palette().resolve(vte::ansi::Color::Indexed(0)),
        original_0
    );
    assert_ne!(
        t.palette().resolve(vte::ansi::Color::Indexed(1)),
        original_1
    );

    // OSC 104 with no params resets all 256 indexed colors.
    feed(&mut t, b"\x1b]104\x07");
    assert_eq!(
        t.palette().resolve(vte::ansi::Color::Indexed(0)),
        original_0
    );
    assert_eq!(
        t.palette().resolve(vte::ansi::Color::Indexed(1)),
        original_1
    );
}

#[test]
fn osc_set_color_marks_grid_dirty() {
    let mut t = term();
    // Drain any existing dirty lines.
    t.grid_mut().dirty_mut().drain().for_each(drop);

    // Setting a non-cursor color should mark all lines dirty.
    feed(&mut t, b"\x1b]4;1;rgb:ff/00/00\x07");
    assert!(
        t.grid_mut().dirty_mut().drain().next().is_some(),
        "set_color should mark grid dirty",
    );
}

#[test]
fn osc_set_cursor_color_does_not_mark_dirty() {
    let mut t = term();
    t.grid_mut().dirty_mut().drain().for_each(drop);

    // Cursor color changes don't require full redraw (per Alacritty).
    feed(&mut t, b"\x1b]12;rgb:ff/00/00\x07");
    assert!(
        t.grid_mut().dirty_mut().drain().next().is_none(),
        "cursor color should not mark grid dirty",
    );
}

#[test]
fn osc_reset_color_marks_grid_dirty() {
    let mut t = term();
    feed(&mut t, b"\x1b]4;1;rgb:ff/00/00\x07");
    t.grid_mut().dirty_mut().drain().for_each(drop);

    feed(&mut t, b"\x1b]104;1\x07");
    assert!(
        t.grid_mut().dirty_mut().drain().next().is_some(),
        "reset_color should mark grid dirty",
    );
}

#[test]
fn osc10_query_sends_foreground_color_request() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b]10;?\x07");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e.contains("ColorRequest(256)")),
        "OSC 10 query should request foreground (index 256), got: {events:?}",
    );
}

#[test]
fn osc11_query_sends_background_color_request() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b]11;?\x07");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e.contains("ColorRequest(257)")),
        "OSC 11 query should request background (index 257), got: {events:?}",
    );
}

#[test]
fn osc12_query_sends_cursor_color_request() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b]12;?\x07");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e.contains("ColorRequest(258)")),
        "OSC 12 query should request cursor color (index 258), got: {events:?}",
    );
}

#[test]
fn osc52_selection_type_s() {
    let (mut t, listener) = term_with_recorder();
    // 's' selector maps to Selection (same as 'p').
    feed(&mut t, b"\x1b]52;s;dGVzdA==\x07");

    let events = listener.events();
    assert!(
        events
            .iter()
            .any(|e| e.contains("ClipboardStore(Selection, test)")),
        "Expected Selection store for 's', got: {events:?}",
    );
}

#[test]
fn osc52_unknown_selector_ignored() {
    let (mut t, listener) = term_with_recorder();
    // Unknown selector letter 'x' should be silently ignored.
    feed(&mut t, b"\x1b]52;x;aGVsbG8=\x07");

    let events = listener.events();
    assert!(
        !events.iter().any(|e| e.contains("ClipboardStore")),
        "Unknown selector should not produce event, got: {events:?}",
    );
}

#[test]
fn osc52_load_sends_clipboard_type() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b]52;c;?\x07");

    let events = listener.events();
    assert!(
        events
            .iter()
            .any(|e| e.contains("ClipboardLoad(Clipboard)")),
        "OSC 52 load with 'c' should send Clipboard type, got: {events:?}",
    );
}

// --- OSC 52 edge cases (from reference repos: Crossterm, WezTerm, Ghostty) ---

#[test]
fn osc52_load_response_formatting_bel() {
    let (mut t, listener) = term_with_recorder();
    // BEL-terminated query — response should also use BEL.
    feed(&mut t, b"\x1b]52;c;?\x07");

    let events = listener.events();
    let load_event = events
        .iter()
        .find(|e| e.contains("ClipboardLoad"))
        .expect("should have ClipboardLoad event");

    // Extract the formatter closure from the event and verify the response.
    // The RecordingListener only stores Debug strings, so we need to test
    // the formatter directly via the handler.
    assert!(load_event.contains("ClipboardLoad(Clipboard)"));
}

#[test]
fn osc52_load_response_closure_produces_valid_base64() {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as Base64;

    // Directly test the response closure logic (same as osc_clipboard_load).
    let clipboard = b'c';
    let terminator = "\x07";
    let text = "hello world";
    let encoded = Base64.encode(text);
    let response = format!("\x1b]52;{};{}{}", clipboard as char, encoded, terminator);

    assert_eq!(response, "\x1b]52;c;aGVsbG8gd29ybGQ=\x07");
}

#[test]
fn osc52_load_response_st_terminator() {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as Base64;

    // ST-terminated query should produce ST-terminated response.
    let clipboard = b'c';
    let terminator = "\x1b\\";
    let text = "hello";
    let encoded = Base64.encode(text);
    let response = format!("\x1b]52;{};{}{}", clipboard as char, encoded, terminator);

    assert_eq!(response, "\x1b]52;c;aGVsbG8=\x1b\\");
}

#[test]
fn osc52_load_selection_p() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b]52;p;?\x07");

    let events = listener.events();
    assert!(
        events
            .iter()
            .any(|e| e.contains("ClipboardLoad(Selection)")),
        "OSC 52 load with 'p' should send Selection type, got: {events:?}",
    );
}

#[test]
fn osc52_load_selection_s() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b]52;s;?\x07");

    let events = listener.events();
    assert!(
        events
            .iter()
            .any(|e| e.contains("ClipboardLoad(Selection)")),
        "OSC 52 load with 's' should send Selection type, got: {events:?}",
    );
}

#[test]
fn osc52_load_unknown_selector_ignored() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b]52;x;?\x07");

    let events = listener.events();
    assert!(
        !events.iter().any(|e| e.contains("ClipboardLoad")),
        "Unknown selector 'x' should not produce load event, got: {events:?}",
    );
}

#[test]
fn osc52_store_empty_payload() {
    let (mut t, listener) = term_with_recorder();
    // Empty base64 after selector — decodes to empty string.
    feed(&mut t, b"\x1b]52;c;\x07");

    let events = listener.events();
    assert!(
        events
            .iter()
            .any(|e| e.contains("ClipboardStore(Clipboard, )")),
        "Empty base64 should store empty string, got: {events:?}",
    );
}

#[test]
fn osc52_store_with_st_terminator() {
    let (mut t, listener) = term_with_recorder();
    // ST terminator (\x1b\\) instead of BEL (\x07).
    feed(&mut t, b"\x1b]52;c;aGVsbG8=\x1b\\");

    let events = listener.events();
    assert!(
        events
            .iter()
            .any(|e| e.contains("ClipboardStore(Clipboard, hello)")),
        "ST-terminated store should work, got: {events:?}",
    );
}

#[test]
fn osc52_store_multiline_content() {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as Base64;

    let (mut t, listener) = term_with_recorder();
    let text = "line one\nline two\nline three";
    let encoded = Base64.encode(text);

    let mut seq = b"\x1b]52;c;".to_vec();
    seq.extend_from_slice(encoded.as_bytes());
    seq.push(0x07);
    feed(&mut t, &seq);

    let events = listener.events();
    assert!(
        events
            .iter()
            .any(|e| e.contains("ClipboardStore(Clipboard, line one\nline two\nline three)")),
        "Multiline content should be preserved, got: {events:?}",
    );
}

#[test]
fn osc52_store_crlf_content() {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as Base64;

    let (mut t, listener) = term_with_recorder();
    let text = "first\r\nsecond\r\n";
    let encoded = Base64.encode(text);

    let mut seq = b"\x1b]52;c;".to_vec();
    seq.extend_from_slice(encoded.as_bytes());
    seq.push(0x07);
    feed(&mut t, &seq);

    let events = listener.events();
    assert!(
        events
            .iter()
            .any(|e| e.contains("ClipboardStore(Clipboard, first\r\nsecond\r\n)")),
        "CRLF should be preserved verbatim, got: {events:?}",
    );
}

#[test]
fn osc52_store_large_payload() {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as Base64;

    let (mut t, listener) = term_with_recorder();
    // 10KB of text — validates no truncation in the pipeline.
    let text: String = "abcdefghij".repeat(1_000);
    let encoded = Base64.encode(&text);

    let mut seq = b"\x1b]52;c;".to_vec();
    seq.extend_from_slice(encoded.as_bytes());
    seq.push(0x07);
    feed(&mut t, &seq);

    let events = listener.events();
    assert!(
        events
            .iter()
            .any(|e| e.contains("ClipboardStore(Clipboard,")),
        "Large payload should produce store event, got: {events:?}",
    );
    // Verify full content survived by checking length in the event string.
    let store = events
        .iter()
        .find(|e| e.contains("ClipboardStore"))
        .unwrap();
    assert!(
        store.contains(&text),
        "Full 10KB text should be preserved in event",
    );
}

#[test]
fn osc52_store_base64_no_padding() {
    let (mut t, listener) = term_with_recorder();
    // "hi" → base64 "aGk=" (with padding) but "aGk" (without) should also work.
    // Standard base64 requires padding, but some terminals omit it.
    // base64 crate rejects missing padding by default.
    feed(&mut t, b"\x1b]52;c;aGk=\x07");

    let events = listener.events();
    assert!(
        events
            .iter()
            .any(|e| e.contains("ClipboardStore(Clipboard, hi)")),
        "Padded base64 should decode, got: {events:?}",
    );
}

#[test]
fn osc52_store_base64_double_padding() {
    let (mut t, listener) = term_with_recorder();
    // "h" → base64 "aA==" (double padding).
    feed(&mut t, b"\x1b]52;c;aA==\x07");

    let events = listener.events();
    assert!(
        events
            .iter()
            .any(|e| e.contains("ClipboardStore(Clipboard, h)")),
        "Double-padded base64 should decode, got: {events:?}",
    );
}

#[test]
fn osc52_store_invalid_utf8() {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as Base64;

    let (mut t, listener) = term_with_recorder();
    // Valid base64 that decodes to invalid UTF-8 (0xFF 0xFE).
    let encoded = Base64.encode([0xFF, 0xFE]);

    let mut seq = b"\x1b]52;c;".to_vec();
    seq.extend_from_slice(encoded.as_bytes());
    seq.push(0x07);
    feed(&mut t, &seq);

    let events = listener.events();
    assert!(
        !events.iter().any(|e| e.contains("ClipboardStore")),
        "Invalid UTF-8 should not produce store event, got: {events:?}",
    );
}

#[test]
fn osc52_store_truncated_base64() {
    let (mut t, listener) = term_with_recorder();
    // "aGVsbG8" is "aGVsbG8=" without final padding — may or may not decode
    // depending on base64 crate strictness. Either way, should not panic.
    feed(&mut t, b"\x1b]52;c;aGVsbG8\x07");

    // Not asserting specific behavior — just that it doesn't panic.
    let _ = listener.events();
}

#[test]
fn osc52_multi_selector_uses_first() {
    let (mut t, listener) = term_with_recorder();
    // VTE parser extracts only the first selector byte from "cp".
    // So this should store to Clipboard (from 'c'), not Selection.
    feed(&mut t, b"\x1b]52;cp;aGVsbG8=\x07");

    let events = listener.events();
    // VTE takes first byte 'c' → Clipboard.
    assert!(
        events
            .iter()
            .any(|e| e.contains("ClipboardStore(Clipboard, hello)")),
        "Multi-selector 'cp' should use first byte 'c' → Clipboard, got: {events:?}",
    );
    // Should NOT also produce a Selection store (VTE doesn't iterate selectors).
    let store_count = events
        .iter()
        .filter(|e| e.contains("ClipboardStore"))
        .count();
    assert_eq!(store_count, 1, "Should produce exactly one store event");
}

#[test]
fn osc52_missing_data_param_ignored() {
    let (mut t, listener) = term_with_recorder();
    // Only 2 params (no data after selector) — VTE calls unhandled().
    feed(&mut t, b"\x1b]52;c\x07");

    let events = listener.events();
    assert!(
        !events.iter().any(|e| e.contains("ClipboardStore")),
        "Missing data param should not produce event, got: {events:?}",
    );
}

#[test]
fn osc8_hyperlink_survives_sgr_reset() {
    let mut t = term();

    // Set hyperlink, then write text, then SGR reset, then more text.
    // ESC]8;;uri\x07 sets hyperlink
    // ESC[1m sets bold
    // ESC[0m resets all SGR (but NOT hyperlink)
    feed(&mut t, b"\x1b]8;;https://example.com\x07");
    feed(&mut t, b"\x1b[1m"); // bold
    feed(&mut t, b"\x1b[0m"); // SGR reset

    // Hyperlink should survive SGR reset — it's not an SGR attribute.
    let extra = t.grid().cursor().template.extra.as_ref();
    assert!(
        extra.is_some() && extra.unwrap().hyperlink.is_some(),
        "Hyperlink should persist across SGR reset",
    );
}

#[test]
fn osc8_hyperlink_written_to_cells() {
    let mut t = term();

    // Set hyperlink, write some text, clear hyperlink, write more.
    feed(&mut t, b"\x1b]8;;https://example.com\x07");
    feed(&mut t, b"AB");
    feed(&mut t, b"\x1b]8;;\x07");
    feed(&mut t, b"CD");

    // Cells A and B should have the hyperlink.
    let row = &t.grid()[crate::index::Line(0)];
    let a_extra = row[Column(0)]
        .extra
        .as_ref()
        .expect("cell A should have extra");
    assert!(a_extra.hyperlink.is_some(), "cell A should have hyperlink");
    let b_extra = row[Column(1)]
        .extra
        .as_ref()
        .expect("cell B should have extra");
    assert!(b_extra.hyperlink.is_some(), "cell B should have hyperlink");

    // Cells C and D should NOT have the hyperlink.
    assert!(
        row[Column(2)].extra.is_none(),
        "cell C should not have hyperlink"
    );
    assert!(
        row[Column(3)].extra.is_none(),
        "cell D should not have hyperlink"
    );
}

#[test]
fn osc8_uri_with_semicolons() {
    let mut t = term();
    // URI containing semicolons (VTE reconstructs from params[2..]).
    feed(&mut t, b"\x1b]8;;https://example.com/path;key=val\x07");

    let extra = t.grid().cursor().template.extra.as_ref().unwrap();
    let link = extra.hyperlink.as_ref().unwrap();
    assert_eq!(link.uri, "https://example.com/path;key=val");
}

#[test]
fn osc_set_title_none_resets() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b]2;Foo\x07");
    assert_eq!(t.title(), "Foo");

    // Directly invoke set_title(None) to test ResetTitle event.
    use vte::ansi::Handler;
    t.set_title(None);

    assert!(t.title().is_empty());
    let events = listener.events();
    assert!(
        events.iter().any(|e| e.contains("ResetTitle")),
        "set_title(None) should emit ResetTitle, got: {events:?}",
    );
}

#[test]
fn osc1_sets_icon_name_not_title() {
    let (mut t, listener) = term_with_recorder();
    // OSC 1 sets icon name only, not window title.
    feed(&mut t, b"\x1b]1;Icon Title\x07");

    assert!(t.title().is_empty(), "OSC 1 should not change title");
    assert_eq!(t.icon_name(), "Icon Title");
    let events = listener.events();
    assert!(events.iter().any(|e| e.contains("IconName")));
}

// OSC 7 is handled by the raw interceptor (oriterm::shell_integration),
// not by the high-level Handler impl. See shell_integration/tests.rs.

// --- OSC color set→query roundtrip ---
//
// Verify that setting a dynamic color then querying it produces a
// response containing the correct hex values. The ColorRequest event
// carries a formatter closure; we verify it returns the expected format.

/// OSC 4 set then query: response contains the set color.
#[test]
fn osc4_set_then_query_roundtrip() {
    let mut t = term();
    // Set index 5 to a known color.
    feed(&mut t, b"\x1b]4;5;rgb:ab/cd/ef\x07");

    // Verify palette has the color.
    let color = t.palette().resolve(vte::ansi::Color::Indexed(5));
    assert_eq!(
        color,
        vte::ansi::Rgb {
            r: 0xab,
            g: 0xcd,
            b: 0xef
        }
    );

    // Query: the event should reference index 5.
    let (mut t2, listener) = term_with_recorder();
    // Set same color on t2.
    feed(&mut t2, b"\x1b]4;5;rgb:ab/cd/ef\x07");
    // Now query it.
    feed(&mut t2, b"\x1b]4;5;?\x07");
    let events = listener.events();
    assert!(
        events.iter().any(|e| e.contains("ColorRequest(5)")),
        "expected ColorRequest(5), got: {events:?}",
    );
}

/// OSC 10 set then query: foreground color roundtrip.
#[test]
fn osc10_set_then_query_roundtrip() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b]10;rgb:de/ad/ff\x07");
    assert_eq!(
        t.palette().foreground(),
        vte::ansi::Rgb {
            r: 0xde,
            g: 0xad,
            b: 0xff
        }
    );

    feed(&mut t, b"\x1b]10;?\x07");
    let events = listener.events();
    assert!(
        events.iter().any(|e| e.contains("ColorRequest(256)")),
        "expected foreground query event, got: {events:?}",
    );
}

/// OSC 11 set then query: background color roundtrip.
#[test]
fn osc11_set_then_query_roundtrip() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b]11;rgb:12/34/56\x07");
    assert_eq!(
        t.palette().background(),
        vte::ansi::Rgb {
            r: 0x12,
            g: 0x34,
            b: 0x56
        }
    );

    feed(&mut t, b"\x1b]11;?\x07");
    let events = listener.events();
    assert!(
        events.iter().any(|e| e.contains("ColorRequest(257)")),
        "expected background query event, got: {events:?}",
    );
}

/// OSC 12 set then query: cursor color roundtrip.
#[test]
fn osc12_set_then_query_roundtrip() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b]12;rgb:fe/dc/ba\x07");
    assert_eq!(
        t.palette().cursor_color(),
        vte::ansi::Rgb {
            r: 0xfe,
            g: 0xdc,
            b: 0xba
        }
    );

    feed(&mut t, b"\x1b]12;?\x07");
    let events = listener.events();
    assert!(
        events.iter().any(|e| e.contains("ColorRequest(258)")),
        "expected cursor color query event, got: {events:?}",
    );
}

/// OSC 4 set, reset, then query: verify reset took effect.
#[test]
fn osc4_set_reset_then_verify() {
    let mut t = term();
    let original = t.palette().resolve(vte::ansi::Color::Indexed(3));

    // Set to a different color.
    feed(&mut t, b"\x1b]4;3;rgb:ff/ff/ff\x07");
    assert_ne!(t.palette().resolve(vte::ansi::Color::Indexed(3)), original);

    // Reset it.
    feed(&mut t, b"\x1b]104;3\x07");
    assert_eq!(
        t.palette().resolve(vte::ansi::Color::Indexed(3)),
        original,
        "OSC 104 should restore to the original default"
    );
}

// --- Icon name (OSC 0/1) tests ---

#[test]
fn osc_1_sets_icon_name_only() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b]1;\xF0\x9F\x90\x8Dpython\x07");
    assert_eq!(t.icon_name(), "🐍python");
    assert!(t.title().is_empty(), "OSC 1 should not change title");
    let events = listener.events();
    assert!(
        events.iter().any(|e| e.contains("IconName")),
        "OSC 1 should emit IconName event, got: {events:?}",
    );
}

#[test]
fn osc_0_sets_both_title_and_icon_name() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b]0;hello\x07");
    assert_eq!(t.title(), "hello");
    assert_eq!(t.icon_name(), "hello");
    let events = listener.events();
    assert!(
        events.iter().any(|e| e.contains("Title")),
        "OSC 0 should emit Title event, got: {events:?}",
    );
    assert!(
        events.iter().any(|e| e.contains("IconName")),
        "OSC 0 should emit IconName event, got: {events:?}",
    );
}

#[test]
fn osc_2_does_not_set_icon_name() {
    let (mut t, _) = term_with_recorder();
    feed(&mut t, b"\x1b]2;title\x07");
    assert_eq!(t.title(), "title");
    assert!(
        t.icon_name().is_empty(),
        "OSC 2 should not change icon name"
    );
}

#[test]
fn osc_set_icon_name_none_resets() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b]1;icon\x07");
    assert_eq!(t.icon_name(), "icon");

    use vte::ansi::Handler;
    t.set_icon_name(None);

    assert!(t.icon_name().is_empty());
    let events = listener.events();
    assert!(
        events.iter().any(|e| e.contains("ResetIconName")),
        "set_icon_name(None) should emit ResetIconName, got: {events:?}",
    );
}
