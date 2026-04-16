//! Legacy encoding — base-case keys, plain text, Enter/LF, release suppression.

use winit::keyboard::{Key, NamedKey};

use oriterm_core::TermMode;

use super::{Modifiers, enc, enc_release, enc_text, no_mode};

// --- Unmodified basic keys ---

#[test]
fn enter() {
    assert_eq!(
        enc(Key::Named(NamedKey::Enter), Modifiers::empty(), no_mode()),
        b"\r"
    );
}

#[test]
fn backspace() {
    assert_eq!(
        enc(
            Key::Named(NamedKey::Backspace),
            Modifiers::empty(),
            no_mode()
        ),
        vec![0x7f]
    );
}

#[test]
fn tab() {
    assert_eq!(
        enc(Key::Named(NamedKey::Tab), Modifiers::empty(), no_mode()),
        b"\t"
    );
}

#[test]
fn shift_tab() {
    assert_eq!(
        enc(Key::Named(NamedKey::Tab), Modifiers::SHIFT, no_mode()),
        b"\x1b[Z"
    );
}

#[test]
fn escape() {
    assert_eq!(
        enc(Key::Named(NamedKey::Escape), Modifiers::empty(), no_mode()),
        vec![0x1b]
    );
}

#[test]
fn alt_backspace() {
    assert_eq!(
        enc(Key::Named(NamedKey::Backspace), Modifiers::ALT, no_mode()),
        vec![0x1b, 0x7f]
    );
}

#[test]
fn space() {
    assert_eq!(
        enc(Key::Named(NamedKey::Space), Modifiers::empty(), no_mode()),
        vec![b' ']
    );
}

// --- Unmodified named keys ---

#[test]
fn arrow_up_normal() {
    assert_eq!(
        enc(Key::Named(NamedKey::ArrowUp), Modifiers::empty(), no_mode()),
        b"\x1b[A"
    );
}

#[test]
fn arrow_down_normal() {
    assert_eq!(
        enc(
            Key::Named(NamedKey::ArrowDown),
            Modifiers::empty(),
            no_mode()
        ),
        b"\x1b[B"
    );
}

#[test]
fn home_normal() {
    assert_eq!(
        enc(Key::Named(NamedKey::Home), Modifiers::empty(), no_mode()),
        b"\x1b[H"
    );
}

#[test]
fn end_normal() {
    assert_eq!(
        enc(Key::Named(NamedKey::End), Modifiers::empty(), no_mode()),
        b"\x1b[F"
    );
}

#[test]
fn insert() {
    assert_eq!(
        enc(Key::Named(NamedKey::Insert), Modifiers::empty(), no_mode()),
        b"\x1b[2~"
    );
}

#[test]
fn delete() {
    assert_eq!(
        enc(Key::Named(NamedKey::Delete), Modifiers::empty(), no_mode()),
        b"\x1b[3~"
    );
}

#[test]
fn page_up() {
    assert_eq!(
        enc(Key::Named(NamedKey::PageUp), Modifiers::empty(), no_mode()),
        b"\x1b[5~"
    );
}

#[test]
fn page_down() {
    assert_eq!(
        enc(
            Key::Named(NamedKey::PageDown),
            Modifiers::empty(),
            no_mode()
        ),
        b"\x1b[6~"
    );
}

#[test]
fn f1() {
    assert_eq!(
        enc(Key::Named(NamedKey::F1), Modifiers::empty(), no_mode()),
        b"\x1bOP"
    );
}

#[test]
fn f5() {
    assert_eq!(
        enc(Key::Named(NamedKey::F5), Modifiers::empty(), no_mode()),
        b"\x1b[15~"
    );
}

#[test]
fn f12() {
    assert_eq!(
        enc(Key::Named(NamedKey::F12), Modifiers::empty(), no_mode()),
        b"\x1b[24~"
    );
}

// --- Plain text fallback ---

#[test]
fn plain_text() {
    let r = enc_text(
        Key::Character("x".into()),
        Modifiers::empty(),
        no_mode(),
        "x",
    );
    assert_eq!(r, b"x");
}

#[test]
fn plain_utf8_text() {
    let r = enc_text(
        Key::Character("好".into()),
        Modifiers::empty(),
        no_mode(),
        "好",
    );
    assert_eq!(r, "好".as_bytes());
}

// --- Legacy release produces nothing ---

#[test]
fn legacy_release_empty() {
    let r = enc_release(Key::Named(NamedKey::ArrowUp), Modifiers::empty(), no_mode());
    assert!(r.is_empty());
}

#[test]
fn legacy_release_char_empty() {
    let r = enc_release(Key::Character("a".into()), Modifiers::empty(), no_mode());
    assert!(r.is_empty());
}

// --- Enter + LINE_FEED_NEW_LINE mode ---

fn linefeed_mode() -> TermMode {
    TermMode::default() | TermMode::LINE_FEED_NEW_LINE
}

#[test]
fn enter_linefeed_mode() {
    let r = enc(
        Key::Named(NamedKey::Enter),
        Modifiers::empty(),
        linefeed_mode(),
    );
    assert_eq!(r, b"\r\n");
}

#[test]
fn enter_normal_mode() {
    let r = enc(Key::Named(NamedKey::Enter), Modifiers::empty(), no_mode());
    assert_eq!(r, b"\r");
}

#[test]
fn alt_enter_normal() {
    let r = enc(Key::Named(NamedKey::Enter), Modifiers::ALT, no_mode());
    assert_eq!(r, b"\x1b\r");
}

#[test]
fn alt_enter_linefeed_mode() {
    let r = enc(Key::Named(NamedKey::Enter), Modifiers::ALT, linefeed_mode());
    assert_eq!(r, b"\x1b\r\n");
}

// --- Multi-char text / dead key compositions ---

#[test]
fn multi_char_text_passthrough() {
    // Dead key compositions can produce multi-char strings.
    // These should pass through as text, not be encoded as CSI u.
    let r = enc_text(
        Key::Character("ö".into()),
        Modifiers::empty(),
        no_mode(),
        "ö",
    );
    assert_eq!(r, "ö".as_bytes());
}
