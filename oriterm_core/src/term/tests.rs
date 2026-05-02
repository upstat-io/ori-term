//! Tests for `Term<S>` — methods defined directly in `term/mod.rs`.
//!
//! Submodule-specific tests live in their owning sibling files:
//! - `alt_screen/tests.rs` — `swap_alt`, alt-grid lifecycle
//! - `image_config/tests.rs` — image protocol configuration
//! - `resize/tests.rs` — `Term::resize`
//! - `shell_state/tests.rs` — prompt markers, command range, finish_command
//! - `snapshot/tests.rs` — `renderable_content`, `damage`, `reset_damage`

use vte::ansi::Processor;

use crate::color::Rgb;
use crate::effect::{
    Effect, EffectSink, HostEffect, NotificationSource, QueueingEffectSink, VoidEffectSink,
};
use crate::grid::CursorShape;
use crate::index::{Column, Line};
use crate::theme::Theme;

use super::{Term, TermMode};

fn make_term() -> Term<VoidEffectSink> {
    Term::new(24, 80, 1000, Theme::default(), VoidEffectSink)
}

/// Create a term with a queuing sink so notification effects can be observed.
fn make_queuing_term() -> Term<QueueingEffectSink> {
    Term::new(24, 80, 1000, Theme::default(), QueueingEffectSink::new())
}

/// Drain effects from a queuing term and extract desktop notifications,
/// applying `ClearPendingNotifications` semantics (clears preceding notifs).
fn drain_desktop_notifications(
    term: &Term<QueueingEffectSink>,
) -> Vec<(NotificationSource, String, String)> {
    let mut effects = Vec::new();
    term.effect_sink().drain_into(&mut effects);
    let mut notifs = Vec::new();
    for effect in effects {
        match effect {
            Effect::Host(HostEffect::DesktopNotification {
                source,
                title,
                body,
            }) => notifs.push((source, title, body)),
            Effect::Host(HostEffect::ClearPendingNotifications) => notifs.clear(),
            _ => {}
        }
    }
    notifs
}

/// Feed raw bytes through the VTE processor.
fn feed(term: &mut impl vte::ansi::Handler, bytes: &[u8]) {
    let mut processor: Processor = Processor::new();
    processor.advance(term, bytes);
}

// ── Term construction + basic accessors ──

#[test]
fn new_creates_working_terminal() {
    let term = make_term();
    assert_eq!(term.grid().lines(), 24);
    assert_eq!(term.grid().cols(), 80);
}

#[test]
fn grid_returns_primary_by_default() {
    let mut term = make_term();
    term.grid_mut().put_char('A');
    assert_eq!(term.grid()[Line(0)][Column(0)].ch, 'A');
    assert!(!term.mode().contains(TermMode::ALT_SCREEN));
}

#[test]
fn mode_defaults_include_show_cursor_and_line_wrap() {
    let term = make_term();
    let mode = term.mode();
    assert!(mode.contains(TermMode::SHOW_CURSOR));
    assert!(mode.contains(TermMode::LINE_WRAP));
}

#[test]
fn default_title_is_empty() {
    let term = make_term();
    assert_eq!(term.title(), "");
}

#[test]
fn default_cursor_shape_is_block() {
    let term = make_term();
    assert_eq!(term.cursor_shape(), CursorShape::Block);
}

#[test]
fn primary_grid_has_scrollback() {
    let term = make_term();
    assert_eq!(term.grid().scrollback().max_scrollback(), 1000);
}

// ── Theme integration ──

#[test]
fn new_with_dark_theme_uses_dark_palette() {
    let t = Term::new(4, 10, 0, Theme::Dark, VoidEffectSink);
    assert_eq!(
        t.palette().foreground(),
        Rgb {
            r: 0xcc,
            g: 0xcc,
            b: 0xcc
        }
    );
    assert_eq!(
        t.palette().background(),
        Rgb {
            r: 0x00,
            g: 0x00,
            b: 0x00
        }
    );
    assert_eq!(t.theme(), Theme::Dark);
}

#[test]
fn new_with_light_theme_uses_light_palette() {
    let t = Term::new(4, 10, 0, Theme::Light, VoidEffectSink);
    assert_eq!(
        t.palette().foreground(),
        Rgb {
            r: 0x2e,
            g: 0x34,
            b: 0x36
        }
    );
    assert_eq!(
        t.palette().background(),
        Rgb {
            r: 0xff,
            g: 0xff,
            b: 0xff
        }
    );
    assert_eq!(t.theme(), Theme::Light);
}

#[test]
fn set_theme_switches_palette() {
    let mut t = Term::new(4, 10, 0, Theme::Dark, VoidEffectSink);
    t.reset_damage();

    t.set_theme(Theme::Light);

    assert_eq!(t.theme(), Theme::Light);
    assert_eq!(
        t.palette().foreground(),
        Rgb {
            r: 0x2e,
            g: 0x34,
            b: 0x36
        }
    );
    assert_eq!(
        t.palette().background(),
        Rgb {
            r: 0xff,
            g: 0xff,
            b: 0xff
        }
    );

    let dmg = t.damage();
    assert!(dmg.is_all_dirty(), "set_theme should mark all dirty");
    drop(dmg);
}

#[test]
fn set_theme_same_theme_is_noop() {
    let mut t = Term::new(4, 10, 0, Theme::Dark, VoidEffectSink);
    t.reset_damage();

    t.set_theme(Theme::Dark);

    assert_eq!(t.theme(), Theme::Dark);
    let dmg: Vec<_> = t.damage().collect();
    assert!(dmg.is_empty(), "same theme should not produce damage");
}

#[test]
fn ris_resets_to_current_theme() {
    let mut t = Term::new(4, 10, 0, Theme::Light, VoidEffectSink);
    assert_eq!(
        t.palette().background(),
        Rgb {
            r: 0xff,
            g: 0xff,
            b: 0xff
        }
    );

    feed(&mut t, b"\x1bc");

    assert_eq!(t.theme(), Theme::Light);
    assert_eq!(
        t.palette().background(),
        Rgb {
            r: 0xff,
            g: 0xff,
            b: 0xff
        }
    );
    assert_eq!(
        t.palette().foreground(),
        Rgb {
            r: 0x2e,
            g: 0x34,
            b: 0x36
        }
    );
}

// ── Selection dirty flag ──

#[test]
fn selection_dirty_initially_false() {
    let term = make_term();
    assert!(!term.is_selection_dirty());
}

#[test]
fn selection_dirty_set_by_character_input() {
    let mut term = make_term();
    feed(&mut term, b"A");
    assert!(term.is_selection_dirty());
}

#[test]
fn selection_dirty_cleared_by_clear_selection_dirty() {
    let mut term = make_term();
    feed(&mut term, b"A");
    assert!(term.is_selection_dirty());
    term.clear_selection_dirty();
    assert!(!term.is_selection_dirty());
}

#[test]
fn selection_dirty_set_by_erase_display() {
    let mut term = make_term();
    term.clear_selection_dirty();
    feed(&mut term, b"\x1b[2J");
    assert!(term.is_selection_dirty());
}

#[test]
fn selection_dirty_set_by_erase_line() {
    let mut term = make_term();
    term.clear_selection_dirty();
    feed(&mut term, b"\x1b[K");
    assert!(term.is_selection_dirty());
}

#[test]
fn selection_dirty_set_by_insert_blank() {
    let mut term = make_term();
    term.clear_selection_dirty();
    feed(&mut term, b"\x1b[@");
    assert!(term.is_selection_dirty());
}

#[test]
fn selection_dirty_set_by_delete_chars() {
    let mut term = make_term();
    term.clear_selection_dirty();
    feed(&mut term, b"\x1b[P");
    assert!(term.is_selection_dirty());
}

#[test]
fn selection_dirty_set_by_scroll_up() {
    let mut term = make_term();
    term.clear_selection_dirty();
    feed(&mut term, b"\x1b[S");
    assert!(term.is_selection_dirty());
}

#[test]
fn selection_dirty_set_by_scroll_down() {
    let mut term = make_term();
    term.clear_selection_dirty();
    feed(&mut term, b"\x1b[T");
    assert!(term.is_selection_dirty());
}

#[test]
fn selection_dirty_set_by_insert_lines() {
    let mut term = make_term();
    term.clear_selection_dirty();
    feed(&mut term, b"\x1b[L");
    assert!(term.is_selection_dirty());
}

#[test]
fn selection_dirty_set_by_delete_lines() {
    let mut term = make_term();
    term.clear_selection_dirty();
    feed(&mut term, b"\x1b[M");
    assert!(term.is_selection_dirty());
}

#[test]
fn selection_dirty_set_by_erase_chars() {
    let mut term = make_term();
    term.clear_selection_dirty();
    feed(&mut term, b"\x1b[X");
    assert!(term.is_selection_dirty());
}

#[test]
fn selection_dirty_set_by_linefeed() {
    let mut term = make_term();
    term.clear_selection_dirty();
    feed(&mut term, b"\n");
    assert!(term.is_selection_dirty());
}

#[test]
fn selection_dirty_set_by_newline() {
    let mut term = make_term();
    term.clear_selection_dirty();
    feed(&mut term, b"\x1bE");
    assert!(term.is_selection_dirty());
}

#[test]
fn selection_dirty_set_by_reverse_index() {
    let mut term = make_term();
    term.clear_selection_dirty();
    feed(&mut term, b"\x1bM");
    assert!(term.is_selection_dirty());
}

#[test]
fn selection_dirty_set_by_reset() {
    let mut term = make_term();
    term.clear_selection_dirty();
    feed(&mut term, b"\x1bc");
    assert!(term.is_selection_dirty());
}

#[test]
fn selection_dirty_not_set_by_cursor_movement() {
    let mut term = make_term();
    term.clear_selection_dirty();
    feed(&mut term, b"\x1b[C");
    assert!(!term.is_selection_dirty());
}

#[test]
fn selection_dirty_not_set_by_sgr() {
    let mut term = make_term();
    term.clear_selection_dirty();
    feed(&mut term, b"\x1b[1m");
    assert!(!term.is_selection_dirty());
}

#[test]
fn selection_dirty_not_set_by_backspace() {
    let mut term = make_term();
    term.clear_selection_dirty();
    feed(&mut term, b"\x08");
    assert!(!term.is_selection_dirty());
}

#[test]
fn selection_dirty_not_set_by_carriage_return() {
    let mut term = make_term();
    term.clear_selection_dirty();
    feed(&mut term, b"\r");
    assert!(!term.is_selection_dirty());
}

#[test]
fn selection_dirty_not_set_by_cup_goto() {
    let mut term = make_term();
    term.clear_selection_dirty();
    feed(&mut term, b"\x1b[5;10H");
    assert!(!term.is_selection_dirty());
}

#[test]
fn selection_dirty_not_set_by_cursor_up() {
    let mut term = make_term();
    feed(&mut term, b"\x1b[10B");
    term.clear_selection_dirty();
    feed(&mut term, b"\x1b[A");
    assert!(!term.is_selection_dirty());
}

#[test]
fn selection_dirty_not_set_by_cursor_down() {
    let mut term = make_term();
    term.clear_selection_dirty();
    feed(&mut term, b"\x1b[B");
    assert!(!term.is_selection_dirty());
}

#[test]
fn selection_dirty_not_set_by_cursor_backward() {
    let mut term = make_term();
    feed(&mut term, b"\x1b[10C");
    term.clear_selection_dirty();
    feed(&mut term, b"\x1b[D");
    assert!(!term.is_selection_dirty());
}

#[test]
fn selection_dirty_not_set_by_save_restore_cursor() {
    let mut term = make_term();
    term.clear_selection_dirty();
    feed(&mut term, b"\x1b7\x1b8");
    assert!(!term.is_selection_dirty());
}

#[test]
fn selection_dirty_not_set_by_mode_set() {
    let mut term = make_term();
    term.clear_selection_dirty();
    feed(&mut term, b"\x1b[?25h");
    assert!(!term.is_selection_dirty());
}

#[test]
fn selection_dirty_set_by_erase_display_below() {
    let mut term = make_term();
    term.clear_selection_dirty();
    feed(&mut term, b"\x1b[0J");
    assert!(term.is_selection_dirty());
}

#[test]
fn selection_dirty_set_by_erase_display_above() {
    let mut term = make_term();
    term.clear_selection_dirty();
    feed(&mut term, b"\x1b[1J");
    assert!(term.is_selection_dirty());
}

#[test]
fn selection_dirty_set_by_erase_scrollback() {
    let mut term = make_term();
    term.clear_selection_dirty();
    feed(&mut term, b"\x1b[3J");
    assert!(term.is_selection_dirty());
}

#[test]
fn selection_dirty_cleared_then_resets_on_new_output() {
    let mut term = make_term();
    feed(&mut term, b"A");
    assert!(term.is_selection_dirty());
    term.clear_selection_dirty();
    assert!(!term.is_selection_dirty());
    feed(&mut term, b"B");
    assert!(term.is_selection_dirty());
}

// ── RIS effect emission + drain idempotency ──

#[test]
fn ris_emits_clear_pending_notifications_effect() {
    let mut term = make_queuing_term();
    term.effect_sink()
        .push(Effect::Host(HostEffect::DesktopNotification {
            source: NotificationSource::Osc9,
            title: "Build".into(),
            body: "Done".into(),
        }));

    assert_eq!(drain_desktop_notifications(&term).len(), 1);

    term.effect_sink()
        .push(Effect::Host(HostEffect::DesktopNotification {
            source: NotificationSource::Osc9,
            title: "Test".into(),
            body: "Pass".into(),
        }));

    feed(&mut term, b"\x1bc");

    assert!(drain_desktop_notifications(&term).is_empty());
}

#[test]
fn drain_into_returns_empty_on_second_call() {
    let term = make_queuing_term();
    term.effect_sink()
        .push(Effect::Host(HostEffect::DesktopNotification {
            source: NotificationSource::Osc9,
            title: String::new(),
            body: "hello".into(),
        }));

    let first = drain_desktop_notifications(&term);
    assert_eq!(first.len(), 1);

    let second = drain_desktop_notifications(&term);
    assert!(second.is_empty(), "second drain should return empty");
}

// ── Scroll region scrollback preservation (Term-level) ──

#[test]
fn scroll_region_preserves_scrollback_content() {
    let mut term = Term::new(5, 10, 100, Theme::default(), VoidEffectSink);

    feed(&mut term, b"Line 0\r\nLine 1\r\nLine 2\r\nLine 3\r\nLine 4");

    feed(&mut term, b"\x1b[2;4r");

    feed(&mut term, b"\x1b[2;1H");
    feed(&mut term, b"\x1b[S");

    let grid = term.grid();
    assert_eq!(
        grid[Line(0)][Column(0)].ch,
        'L',
        "line 0 outside region should be preserved"
    );

    assert_eq!(
        grid[Line(4)][Column(0)].ch,
        'L',
        "line 4 outside region should be preserved"
    );
}

#[test]
fn scrollback_survives_region_scroll_down() {
    let mut term = Term::new(5, 10, 100, Theme::default(), VoidEffectSink);

    for i in 0..8u8 {
        if i > 0 {
            feed(&mut term, b"\r\n");
        }
        feed(&mut term, &[b'A' + i]);
    }

    let sb_before = term.grid().scrollback().len();
    assert!(sb_before > 0, "should have scrollback content");

    feed(&mut term, b"\x1b[2;4r");
    feed(&mut term, b"\x1b[2;1H");
    feed(&mut term, b"\x1b[T");

    assert_eq!(
        term.grid().scrollback().len(),
        sb_before,
        "scrollback should not be affected by region scroll"
    );
}

// ── OSC 22: Mouse cursor icon (state plumbing) ──

#[test]
fn term_mouse_cursor_icon_starts_none() {
    let term = make_term();
    assert!(term.mouse_cursor_icon().is_none());
}

#[test]
fn term_set_mouse_cursor_icon_stores_icon() {
    use vte::ansi::Handler;
    use vte::ansi::cursor_icon::CursorIcon;

    let mut term = make_term();
    Handler::set_mouse_cursor_icon(&mut term, CursorIcon::Pointer);
    assert_eq!(term.mouse_cursor_icon(), Some(CursorIcon::Pointer));
}
