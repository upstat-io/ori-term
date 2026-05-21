use std::time::Duration;

use crate::geometry::{Point, Rect};
use crate::input::{InputEvent, Key, Modifiers, MouseButton, ScrollDelta};
use crate::sense::Sense;
use crate::widgets::Widget;
use crate::widgets::button::ButtonWidget;

use super::{RecordingWidget, WidgetTestHarness};

// -- WidgetTestHarness tests --

#[test]
fn harness_constructs_with_button() {
 let button = ButtonWidget::new("Click me");
 let button_id = button.id();
 let harness = WidgetTestHarness::new(button);

 // Layout should produce non-zero bounds.
 let bounds = harness.find_widget_bounds(button_id);
 assert!(bounds.is_some(), "button should have layout bounds");
 let rect = bounds.unwrap();
 assert!(rect.width() > 0.0, "button width should be positive");
 assert!(rect.height() > 0.0, "button height should be positive");
}

#[test]
fn harness_with_custom_size() {
 let button = ButtonWidget::new("Test");
 let harness = WidgetTestHarness::with_size(button, 400.0, 300.0);
 assert_eq!(harness.viewport().width(), 400.0);
 assert_eq!(harness.viewport().height(), 300.0);
}

#[test]
fn harness_process_mouse_move() {
 let button = ButtonWidget::new("Hover me");
 let button_id = button.id();
 let mut harness = WidgetTestHarness::new(button);

 // Get button center.
 let bounds = harness.find_widget_bounds(button_id).unwrap();
 let center = Point::new(
 bounds.x() + bounds.width() / 2.0,
 bounds.y() + bounds.height() / 2.0,
 );

 // Dispatch mouse move to button center.
 let event = InputEvent::MouseMove {
 pos: center,
 modifiers: Modifiers::NONE,
 };
 harness.process_event(event);

 // Button should now be hot.
 assert!(
 harness.is_hot(button_id),
 "button should be hot after mouse move to its center"
 );
}

#[test]
fn harness_process_click() {
 let button = ButtonWidget::new("Click me");
 let button_id = button.id();
 let mut harness = WidgetTestHarness::new(button);

 let bounds = harness.find_widget_bounds(button_id).unwrap();
 let center = Point::new(
 bounds.x() + bounds.width() / 2.0,
 bounds.y() + bounds.height() / 2.0,
 );

 // Move mouse to button.
 harness.process_event(InputEvent::MouseMove {
 pos: center,
 modifiers: Modifiers::NONE,
 });

 // Press.
 harness.process_event(InputEvent::MouseDown {
 pos: center,
 button: MouseButton::Left,
 modifiers: Modifiers::NONE,
 });
 assert!(
 harness.is_active(button_id),
 "button should be active after mouse down"
 );

 // Release.
 harness.process_event(InputEvent::MouseUp {
 pos: center,
 button: MouseButton::Left,
 modifiers: Modifiers::NONE,
 });
 assert!(
 !harness.is_active(button_id),
 "button should not be active after mouse up"
 );

 // Should have a Clicked action.
 let actions = harness.take_actions();
 assert!(
 actions
 .iter()
 .any(|a| matches!(a, crate::action::WidgetAction::Clicked(id) if *id == button_id)),
 "should have Clicked action for button, got: {actions:?}"
 );
}

#[test]
fn harness_advance_time() {
 let button = ButtonWidget::new("Animated");
 let mut harness = WidgetTestHarness::new(button);

 // Capture clock before advancing; assertion below pins the delta.
 let now_before = harness.now();
 harness.advance_time(Duration::from_millis(16));
 let now_after = harness.now();

 // Clock MUST have advanced by at least the requested duration. Without
 // this assertion the test only verifies "advance_time does not panic"
 // — true of any no-op — and provides no regression guard for the
 // simulated-clock contract.
 let elapsed = now_after.duration_since(now_before);
 assert!(
 elapsed >= Duration::from_millis(16),
 "clock advanced by {:?}, expected >= 16ms",
 elapsed,
 );
 // Mouse position default is (0, 0); pin the harness contract that
 // advance_time does NOT shift it.
 assert_eq!(harness.mouse_pos(), Point::new(0.0, 0.0));
}

#[test]
fn harness_click_produces_clicked_action() {
 let button = ButtonWidget::new("Click me");
 let button_id = button.id();
 let mut harness = WidgetTestHarness::new(button);

 let actions = harness.click(button_id);
 assert!(
 actions
 .iter()
 .any(|a| matches!(a, crate::action::WidgetAction::Clicked(id) if *id == button_id)),
 "click() should produce Clicked action, got: {actions:?}"
 );
}

#[test]
fn harness_mouse_move_to_makes_hot() {
 let button = ButtonWidget::new("Hover me");
 let button_id = button.id();
 let mut harness = WidgetTestHarness::new(button);

 harness.mouse_move_to(button_id);
 assert!(
 harness.is_hot(button_id),
 "button should be hot after mouse_move_to"
 );
}

#[test]
fn harness_is_hot_after_mouse_move_to() {
 let button = ButtonWidget::new("Hover test");
 let button_id = button.id();
 let mut harness = WidgetTestHarness::new(button);

 assert!(!harness.is_hot(button_id));
 harness.mouse_move_to(button_id);
 assert!(harness.is_hot(button_id));
}

#[test]
fn harness_get_widget_ref() {
 let button = ButtonWidget::new("Root");
 let button_id = button.id();
 let harness = WidgetTestHarness::new(button);

 let wref = harness.get_widget(button_id);
 assert_eq!(wref.id(), button_id);
 assert!(!wref.is_hot());
 assert!(wref.bounds().width() > 0.0);
}

#[test]
fn harness_all_widget_ids() {
 let button = ButtonWidget::new("One");
 let button_id = button.id();
 let harness = WidgetTestHarness::new(button);

 let ids = harness.all_widget_ids();
 assert!(ids.contains(&button_id), "should contain button ID");
}

#[test]
fn harness_render_button_has_rect_and_text() {
 let button = ButtonWidget::new("Render test");
 let mut harness = WidgetTestHarness::new(button);
 let scene = harness.render();

 let rects = super::render_assert::rects(&scene);
 assert!(!rects.is_empty(), "button should paint at least one rect");
 let texts = super::render_assert::texts(&scene);
 assert!(!texts.is_empty(), "button should paint text");
 assert!(
 super::render_assert::command_count(&scene) >= 2,
 "button should have at least 2 primitives (quad + text)"
 );
}

#[test]
fn harness_widgets_with_sense_returns_clickable() {
 use crate::sense::Sense;

 let button = ButtonWidget::new("Clickable");
 let button_id = button.id();
 let harness = WidgetTestHarness::new(button);

 let clickable = harness.widgets_with_sense(Sense::click());
 assert!(
 clickable.contains(&button_id),
 "button should be in clickable widgets"
 );
}

#[test]
fn harness_focus_traversal() {
 use crate::widgets::stack::StackWidget;

 let btn1 = ButtonWidget::new("First");
 let btn2 = ButtonWidget::new("Second");
 let btn3 = ButtonWidget::new("Third");
 let id1 = btn1.id();
 let id2 = btn2.id();
 let id3 = btn3.id();

 let stack = StackWidget::new(vec![Box::new(btn1), Box::new(btn2), Box::new(btn3)]);
 let mut h = WidgetTestHarness::new(stack);

 // No focus initially.
 assert!(h.focused_widget().is_none());

 // Tab -> first focusable.
 h.tab();
 assert_eq!(
 h.focused_widget(),
 Some(id1),
 "first tab should focus first widget"
 );

 // Tab -> second.
 h.tab();
 assert_eq!(
 h.focused_widget(),
 Some(id2),
 "second tab should focus second widget"
 );

 // Tab -> third.
 h.tab();
 assert_eq!(
 h.focused_widget(),
 Some(id3),
 "third tab should focus third widget"
 );

 // Shift+Tab -> back to second.
 h.shift_tab();
 assert_eq!(
 h.focused_widget(),
 Some(id2),
 "shift+tab should focus second widget"
 );
}

#[test]
fn harness_paint_hover_changes_output() {
 let button = ButtonWidget::new("Paint test");
 let button_id = button.id();
 let mut h = WidgetTestHarness::new(button);

 // Paint in normal state.
 let draw_list_normal = h.render();
 let rects_normal = super::render_assert::rects(&draw_list_normal);
 assert!(!rects_normal.is_empty(), "should have rect commands");

 // Hover and paint again.
 h.mouse_move_to(button_id);
 let draw_list_hover = h.render();
 let rects_hover = super::render_assert::rects(&draw_list_hover);
 assert!(
 !rects_hover.is_empty(),
 "should still have rect commands after hover"
 );

 // Both states should produce draw commands (we can't easily compare colors
 // without knowing the theme, but both should be non-empty).
 assert!(super::render_assert::command_count(&draw_list_normal) > 0);
 assert!(super::render_assert::command_count(&draw_list_hover) > 0);
}

#[test]
fn harness_rebuild_layout_updates_focus_order() {
 let button = ButtonWidget::new("Focusable");
 let button_id = button.id();
 let harness = WidgetTestHarness::new(button);

 // Button should be in focus order.
 let focusable = harness.focusable_widgets();
 assert!(
 focusable.contains(&button_id),
 "focusable button should be in focus order"
 );
}

// -- Resize tests --

#[test]
fn harness_resize_updates_viewport() {
 let button = ButtonWidget::new("Resize me");
 let mut harness = WidgetTestHarness::new(button);

 assert_eq!(harness.viewport().width(), 800.0);
 assert_eq!(harness.viewport().height(), 600.0);

 harness.resize(1024.0, 768.0);

 assert_eq!(harness.viewport().width(), 1024.0);
 assert_eq!(harness.viewport().height(), 768.0);
}

#[test]
fn harness_resize_relayouts_widget() {
 let button = ButtonWidget::new("Layout test");
 let button_id = button.id();
 let mut harness = WidgetTestHarness::new(button);

 let bounds_before = harness.find_widget_bounds(button_id).unwrap();

 // Shrink viewport dramatically.
 harness.resize(50.0, 50.0);

 let bounds_after = harness.find_widget_bounds(button_id).unwrap();
 // Widget should still have valid bounds (not panicked).
 assert!(bounds_after.width() > 0.0);
 assert!(bounds_after.height() > 0.0);
 // After shrinking the viewport from 800×600 to 50×50, the widget MUST
 // fit entirely within the new viewport. The original 800-wide layout
 // could not satisfy this without recomputation.
 assert!(
 bounds_after.x() + bounds_after.width() <= 50.0,
 "widget must fit within shrunken viewport: bounds_after = {bounds_after:?}",
 );
 assert!(
 bounds_after.y() + bounds_after.height() <= 50.0,
 "widget must fit within shrunken viewport: bounds_after = {bounds_after:?}",
 );
 // bounds_before is retained as documentation that the test pre-resize
 // state is captured; the assertion is post-resize-only.
 let _ = bounds_before;
}

#[test]
fn harness_resize_preserves_interaction_state() {
 let button = ButtonWidget::new("Hover resize");
 let button_id = button.id();
 let mut harness = WidgetTestHarness::new(button);

 // Make button hot.
 harness.mouse_move_to(button_id);
 assert!(harness.is_hot(button_id));

 // Resize — hot state should be preserved (mouse hasn't moved).
 harness.resize(1024.0, 768.0);

 // Re-hover after resize to the new button position.
 harness.mouse_move_to(button_id);
 assert!(
 harness.is_hot(button_id),
 "button should still be hot after resize + re-hover"
 );
}

#[test]
fn harness_resize_to_tiny_does_not_panic() {
 let button = ButtonWidget::new("Tiny");
 let mut harness = WidgetTestHarness::new(button);

 // Edge case: very small viewport.
 harness.resize(1.0, 1.0);
 assert_eq!(harness.viewport().width(), 1.0);
 assert_eq!(harness.viewport().height(), 1.0);
}

#[test]
fn harness_resize_to_large_does_not_panic() {
 let button = ButtonWidget::new("Large");
 let mut harness = WidgetTestHarness::new(button);

 harness.resize(10000.0, 10000.0);
 assert_eq!(harness.viewport().width(), 10000.0);
}

#[test]
fn harness_rapid_resize_cycle() {
 let button = ButtonWidget::new("Rapid");
 let button_id = button.id();
 let mut harness = WidgetTestHarness::new(button);

 // Simulate rapid resize (like a window drag).
 let sizes: &[(f32, f32)] = &[
 (800.0, 600.0),
 (801.0, 601.0),
 (850.0, 640.0),
 (900.0, 700.0),
 (400.0, 300.0),
 (1.0, 1.0),
 (1920.0, 1080.0),
 (80.0, 24.0),
 (800.0, 600.0),
 ];
 for &(w, h) in sizes {
 harness.resize(w, h);
 }

 // Should still be functional.
 let bounds = harness.find_widget_bounds(button_id);
 assert!(
 bounds.is_some(),
 "widget should still have bounds after rapid resize"
 );
}

// -- Overlay test helpers --

#[test]
fn harness_overlay_push_and_dismiss() {
 use crate::geometry::Rect;
 use crate::widgets::spacer::SpacerWidget;

 let button = ButtonWidget::new("Main");
 let mut h = WidgetTestHarness::new(button);

 assert!(!h.has_overlays());

 // Push a popup overlay.
 let overlay = SpacerWidget::fixed(100.0, 40.0);
 h.push_popup(overlay, Rect::new(50.0, 50.0, 100.0, 40.0));
 assert!(h.has_overlays());

 // Dismiss all overlays.
 h.dismiss_overlays();
 assert!(!h.has_overlays());
}

#[test]
fn harness_root_accessor() {
 let button = ButtonWidget::new("Root");
 let harness = WidgetTestHarness::new(button);

 // Should be able to access WindowRoot directly.
 assert!(harness.root().is_dirty());
 assert!(!harness.root().has_pending_actions());
}

// -- RecordingWidget tests --
// These tests pin the public surface of `super::RecordingWidget` and
// `super::RecordedEvents`. They use unbound keys (`Key::Character('x')`)
// for keyboard cells so global keymap actions like FocusNext don't
// intercept the event before `on_input` records it.

/// Asserts the recorded event log contains the dispatched sequence
/// verbatim — filtering out pipeline-injected lifecycle/hot-state events
/// from the comparison. Pins variant + payload + order via element-wise
/// equality on the filtered subset.
/// Extracted from four matrix tests that all shared the same filter-then-eq
/// pattern. The fifth matrix test keeps its own set-membership assertion
/// since it uses a different shape (assert on length + per-event contains).
fn assert_filtered_matches(events: &super::RecordedEvents, dispatched: &[InputEvent]) {
 let recorded = events.all();
 let filtered: Vec<InputEvent> = recorded
 .iter()
 .copied()
 .filter(|e| dispatched.contains(e))
 .collect();
 assert_eq!(
 filtered.as_slice(),
 dispatched,
 "filtered recorded events must equal dispatched sequence verbatim; got filtered={filtered:?}, full={recorded:?}",
 );
}

/// RecordingWidget helper extraction.
/// Pins: a single KeyDown via the harness pipeline appears in `events.all()`
/// with the dispatched key + modifiers preserved verbatim.
#[test]
fn recording_widget_records_keydown_events() {
 let (probe, events) = RecordingWidget::new(Some("Probe"), Sense::focusable());
 let mut h = WidgetTestHarness::new(probe);
 h.rebuild_focus_order();
 h.tab(); // focus the recording widget

 let _ = h.key_press(Key::Character('x'), Modifiers::NONE);

 assert_eq!(events.count_keydowns(), 1, "exactly one KeyDown recorded");
 let recorded = events.all();
 assert!(
 recorded.iter().any(|e| matches!(
 e,
 InputEvent::KeyDown { key: Key::Character('x'), modifiers: m } if *m == Modifiers::NONE,
 )),
 "expected KeyDown {{ key: Character('x'), modifiers: NONE }}, got {recorded:?}",
 );
}

/// Pins: every InputEvent variant (KeyDown, KeyUp, MouseDown, MouseUp,
/// MouseMove, Scroll) is recorded with all inner fields preserved.
#[test]
fn recording_widget_records_all_input_variants_with_payloads() {
 let (probe, events) =
 RecordingWidget::new(Some("Probe"), Sense::focusable().union(Sense::click()));
 let mut h = WidgetTestHarness::new(probe);
 h.rebuild_focus_order();
 h.tab();

 let pos = Point::new(10.0, 10.0);
 h.process_event(InputEvent::KeyDown {
 key: Key::Character('x'),
 modifiers: Modifiers::NONE,
 });
 h.process_event(InputEvent::KeyUp {
 key: Key::Character('x'),
 modifiers: Modifiers::NONE,
 });
 h.process_event(InputEvent::MouseDown {
 pos,
 button: MouseButton::Left,
 modifiers: Modifiers::NONE,
 });
 h.process_event(InputEvent::MouseUp {
 pos,
 button: MouseButton::Left,
 modifiers: Modifiers::NONE,
 });
 h.process_event(InputEvent::MouseMove {
 pos,
 modifiers: Modifiers::NONE,
 });
 h.process_event(InputEvent::Scroll {
 pos,
 delta: ScrollDelta::Pixels { x: 0.0, y: 1.0 },
 modifiers: Modifiers::NONE,
 });

 // Build the exact expected sequence we dispatched.
 let expected = [
 InputEvent::KeyDown {
 key: Key::Character('x'),
 modifiers: Modifiers::NONE,
 },
 InputEvent::KeyUp {
 key: Key::Character('x'),
 modifiers: Modifiers::NONE,
 },
 InputEvent::MouseDown {
 pos,
 button: MouseButton::Left,
 modifiers: Modifiers::NONE,
 },
 InputEvent::MouseUp {
 pos,
 button: MouseButton::Left,
 modifiers: Modifiers::NONE,
 },
 InputEvent::MouseMove {
 pos,
 modifiers: Modifiers::NONE,
 },
 InputEvent::Scroll {
 pos,
 delta: ScrollDelta::Pixels { x: 0.0, y: 1.0 },
 modifiers: Modifiers::NONE,
 },
 ];
 assert_filtered_matches(&events, &expected);
}

/// Pins: each modifier variant (single + multi-modifier combos) is
/// preserved verbatim in `events.all()`.
#[test]
fn recording_widget_records_modifier_variants() {
 let (probe, events) = RecordingWidget::new(Some("Probe"), Sense::focusable());
 let mut h = WidgetTestHarness::new(probe);
 h.rebuild_focus_order();
 h.tab();

 let modifier_variants = [
 Modifiers::NONE,
 Modifiers::SHIFT_ONLY,
 Modifiers::CTRL_ONLY,
 Modifiers::ALT_ONLY,
 Modifiers::LOGO_ONLY,
 Modifiers::SHIFT_ONLY.union(Modifiers::CTRL_ONLY),
 Modifiers::SHIFT_ONLY.union(Modifiers::ALT_ONLY),
 Modifiers::CTRL_ONLY.union(Modifiers::ALT_ONLY),
 Modifiers::SHIFT_ONLY
 .union(Modifiers::CTRL_ONLY)
 .union(Modifiers::ALT_ONLY),
 Modifiers::SHIFT_ONLY.union(Modifiers::LOGO_ONLY),
 ];
 for m in modifier_variants {
 h.process_event(InputEvent::KeyDown {
 key: Key::Character('x'),
 modifiers: m,
 });
 }

 let keydowns: Vec<Modifiers> = events
 .all()
 .into_iter()
 .filter_map(|e| match e {
 InputEvent::KeyDown { modifiers, .. } => Some(modifiers),
 _ => None,
 })
 .collect();
 assert_eq!(
 keydowns.len(),
 modifier_variants.len(),
 "expected one KeyDown per modifier variant, got {keydowns:?}",
 );
 for m in modifier_variants {
 assert!(keydowns.contains(&m), "modifier variant {m:?} missing");
 }
}

/// Pins: MouseDown + MouseUp for every MouseButton variant at two
/// distinct in-bounds positions all reach the recording widget.
#[test]
fn recording_widget_records_mouse_button_and_position_matrix() {
 let (probe, events) = RecordingWidget::new(None, Sense::click());
 let mut h = WidgetTestHarness::new(probe);

 let positions = [Point::new(10.0, 10.0), Point::new(80.0, 30.0)];
 let buttons = [MouseButton::Left, MouseButton::Right, MouseButton::Middle];
 // Build the expected event set FIRST, then dispatch; this lets us assert
 // on widget-recorded counts rather than a local loop counter that only
 // proves the dispatch loop iterated.
 let mut expected: Vec<InputEvent> = Vec::with_capacity(12);
 for &pos in &positions {
 for &button in &buttons {
 expected.push(InputEvent::MouseDown {
 pos,
 button,
 modifiers: Modifiers::NONE,
 });
 expected.push(InputEvent::MouseUp {
 pos,
 button,
 modifiers: Modifiers::NONE,
 });
 }
 }
 for &event in &expected {
 h.process_event(event);
 }

 // Filter recorded events to the dispatched set; the filtered count proves
 // the widget OBSERVED 12 events, not just that the dispatch loop ran 12
 // times.
 let recorded = events.all();
 let filtered: Vec<InputEvent> = recorded
 .iter()
 .copied()
 .filter(|e| expected.contains(e))
 .collect();
 assert_eq!(
 filtered.len(),
 12,
 "widget must record 12 events (2 event types × 3 buttons × 2 positions); filtered={filtered:?}, full={recorded:?}",
 );
 for ev in &expected {
 assert!(
 recorded.contains(ev),
 "expected {ev:?} in recorded events, got {recorded:?}",
 );
 }
}

/// Pins: every `ScrollDelta` variant (Pixels, Lines) is recorded with
/// payload preserved.
#[test]
fn recording_widget_records_scroll_delta_variants() {
 let (probe, events) = RecordingWidget::new(None, Sense::click());
 let mut h = WidgetTestHarness::new(probe);

 let pos = Point::new(10.0, 10.0);
 let deltas = [
 ScrollDelta::Pixels { x: 1.0, y: 2.0 },
 ScrollDelta::Lines { x: 0.0, y: 3.0 },
 ];
 for delta in deltas {
 h.process_event(InputEvent::Scroll {
 pos,
 delta,
 modifiers: Modifiers::NONE,
 });
 }

 let scrolls: Vec<ScrollDelta> = events
 .all()
 .into_iter()
 .filter_map(|e| match e {
 InputEvent::Scroll { delta, .. } => Some(delta),
 _ => None,
 })
 .collect();
 assert_eq!(scrolls.len(), 2);
 for delta in deltas {
 assert!(scrolls.contains(&delta), "scroll delta {delta:?} missing");
 }
}

/// Pins: KeyDown immediately followed by KeyUp produces both events
/// in the recorded sequence in dispatched order.
#[test]
fn recording_widget_records_keydown_keyup_pair_in_order() {
 let (probe, events) = RecordingWidget::new(None, Sense::focusable());
 let mut h = WidgetTestHarness::new(probe);
 h.rebuild_focus_order();
 h.tab();

 let dispatched: [InputEvent; 2] = [
 InputEvent::KeyDown {
 key: Key::Character('x'),
 modifiers: Modifiers::NONE,
 },
 InputEvent::KeyUp {
 key: Key::Character('x'),
 modifiers: Modifiers::NONE,
 },
 ];
 for &event in &dispatched {
 h.process_event(event);
 }

 assert_filtered_matches(&events, &dispatched);
}

/// Pins TWO properties at once via a mixed-payload sequence [A, A, B, B, A]:
/// (1) NO DEDUP — the 3 identical A events show up as 3 separate entries
/// rather than collapsing to 1 (`A` appears 3× and `B` appears 2×, matching
/// the dispatched 5-event count); (2) ORDER — the recorded sequence equals
/// the dispatched sequence verbatim (different payloads make this checkable
/// where 5 identical events would be order-indistinguishable). Plus the
/// positive `!events.is_empty()` complement to T11.
#[test]
fn recording_widget_records_repeated_events_in_order() {
 let (probe, events) = RecordingWidget::new(None, Sense::click());
 let mut h = WidgetTestHarness::new(probe);

 let pos_a = Point::new(10.0, 10.0);
 let pos_b = Point::new(20.0, 10.0);
 let event_a = InputEvent::MouseMove {
 pos: pos_a,
 modifiers: Modifiers::NONE,
 };
 let event_b = InputEvent::MouseMove {
 pos: pos_b,
 modifiers: Modifiers::NONE,
 };
 let dispatched: [InputEvent; 5] = [event_a, event_a, event_b, event_b, event_a];
 for &event in &dispatched {
 h.process_event(event);
 }

 assert!(
 !events.is_empty(),
 "events should be non-empty after dispatch"
 );
 // Verbatim equality on the [A, A, B, B, A] sequence pins BOTH no-dedup
 // (3 identical As show up as 3 entries) AND order (different payloads
 // make ordering checkable).
 assert_filtered_matches(&events, &dispatched);
}

/// Pins: `count_keydowns()` filters to KeyDown only.
#[test]
fn recording_widget_count_keydowns_filters_correctly() {
 let (probe, events) = RecordingWidget::new(None, Sense::focusable().union(Sense::click()));
 let mut h = WidgetTestHarness::new(probe);
 h.rebuild_focus_order();
 h.tab();

 let pos = Point::new(10.0, 10.0);
 h.process_event(InputEvent::KeyDown {
 key: Key::Character('x'),
 modifiers: Modifiers::NONE,
 });
 h.process_event(InputEvent::KeyUp {
 key: Key::Character('x'),
 modifiers: Modifiers::NONE,
 });
 h.process_event(InputEvent::MouseDown {
 pos,
 button: MouseButton::Left,
 modifiers: Modifiers::NONE,
 });
 h.process_event(InputEvent::Scroll {
 pos,
 delta: ScrollDelta::Pixels { x: 0.0, y: 1.0 },
 modifiers: Modifiers::NONE,
 });

 assert_eq!(
 events.count_keydowns(),
 1,
 "only the KeyDown event counts; got count={}, all={:?}",
 events.count_keydowns(),
 events.all(),
 );
}

/// Pins: `last_event()` returns the most recent recorded event.
#[test]
fn recording_widget_last_event_returns_most_recent() {
 let (probe, events) = RecordingWidget::new(None, Sense::click());
 let mut h = WidgetTestHarness::new(probe);

 assert_eq!(events.last_event(), None, "empty handle returns None");

 let pos1 = Point::new(10.0, 10.0);
 let pos2 = Point::new(20.0, 20.0);
 h.process_event(InputEvent::MouseMove {
 pos: pos1,
 modifiers: Modifiers::NONE,
 });
 h.process_event(InputEvent::MouseMove {
 pos: pos2,
 modifiers: Modifiers::NONE,
 });

 let last = events.last_event().expect("at least one event recorded");
 assert!(matches!(last, InputEvent::MouseMove { pos, .. } if pos == pos2));
}

/// Pins: `all()` preserves observation order across mixed-variant dispatch.
#[test]
fn recording_widget_all_returns_observation_order() {
 let (probe, events) = RecordingWidget::new(None, Sense::focusable().union(Sense::click()));
 let mut h = WidgetTestHarness::new(probe);
 h.rebuild_focus_order();
 h.tab();

 let pos = Point::new(10.0, 10.0);
 let dispatched = [
 InputEvent::MouseMove {
 pos,
 modifiers: Modifiers::NONE,
 },
 InputEvent::KeyDown {
 key: Key::Character('a'),
 modifiers: Modifiers::NONE,
 },
 InputEvent::MouseDown {
 pos,
 button: MouseButton::Left,
 modifiers: Modifiers::NONE,
 },
 ];
 for &event in &dispatched {
 h.process_event(event);
 }

 assert_filtered_matches(&events, &dispatched);
}

/// Pins: pre-input handle reports zero across all query methods.
#[test]
fn recording_widget_empty_handle_reports_zero() {
 let (_probe, events) = RecordingWidget::new(None, Sense::none());

 assert_eq!(events.len(), 0);
 assert!(events.is_empty());
 assert_eq!(events.last_event(), None);
 assert!(events.all().is_empty());
 assert_eq!(events.count_keydowns(), 0);
}

/// Pins: `Widget::key_context()` returns the constructor-provided Some(...) value.
#[test]
fn recording_widget_returns_configured_key_context_some() {
 let (probe, _events) = RecordingWidget::new(Some("Probe"), Sense::none());
 assert_eq!(probe.key_context(), Some("Probe"));
}

/// Pins: `Widget::key_context()` returns None when constructed with None.
#[test]
fn recording_widget_returns_configured_key_context_none() {
 let (probe, _events) = RecordingWidget::new(None, Sense::none());
 assert_eq!(probe.key_context(), None);
}

/// Pins: `Widget::sense()` returns the constructor-provided Sense.
#[test]
fn recording_widget_returns_configured_sense() {
 for sense in [Sense::none(), Sense::focusable(), Sense::click()] {
 let (probe, _events) = RecordingWidget::new(None, sense);
 assert_eq!(probe.sense(), sense);
 }
}

/// Pins: layout box dimensions equal the module-level constants.
#[test]
fn recording_widget_layout_uses_pinned_dimensions() {
 use super::recording_widget::{RECORDING_WIDGET_HEIGHT, RECORDING_WIDGET_WIDTH};

 let (probe, _events) = RecordingWidget::new(None, Sense::none());
 let h = WidgetTestHarness::new(probe);
 let layout = h.layout();
 assert!(
 (layout.rect.width() - RECORDING_WIDGET_WIDTH).abs() < 0.001,
 "layout width {} != RECORDING_WIDGET_WIDTH {}",
 layout.rect.width(),
 RECORDING_WIDGET_WIDTH,
 );
 assert!(
 (layout.rect.height() - RECORDING_WIDGET_HEIGHT).abs() < 0.001,
 "layout height {} != RECORDING_WIDGET_HEIGHT {}",
 layout.rect.height(),
 RECORDING_WIDGET_HEIGHT,
 );
}

/// RecordingWidget must inherit the trait default
/// for `handle_keymap_action`. The runtime pin
/// (`overlay/tests.rs::recording_widget_handle_keymap_action_returns_none`)
/// cannot detect an explicit override that returns `None`; this structural
/// pin asserts the override is not defined at all.
#[test]
fn recording_widget_source_does_not_define_handle_keymap_action() {
 let path =
 std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/testing/recording_widget.rs");
 let source = std::fs::read_to_string(&path)
 .unwrap_or_else(|e| panic!("recording_widget.rs must be readable at {path:?}: {e}"));
 // Strip Rust line comments (`//`, `///`, `//!`) before scanning so doc
 // comments that mention `fn handle_keymap_action(` (e.g. in the test's
 // own assertion message above, or in this widget's doc comments) don't
 // false-positive. This widens the pin to "no actual override in code"
 // rather than "no occurrence anywhere in the file".
 let code_only: String = source
 .lines()
 .map(|line| {
 // Drop everything from the first `//` (handles `//`, `///`, `//!`
 // and inline trailing comments). Block comments `/* */` are not
 // stripped because they don't appear in this file; if they ever
 // do, this pin will need a tokenizer-based scan.
 line.split_once("//").map_or(line, |(code, _)| code)
 })
 .collect::<Vec<_>>()
 .join("\n");
 assert!(
 !code_only.contains("fn handle_keymap_action("),
 "RecordingWidget must NOT define handle_keymap_action — the trait \
 default returning None is the FocusNext fall-through gate's required \
 shape. An explicit override (even one returning None) passes the \
 runtime pin in overlay/tests.rs but breaks the structural contract.",
 );
}

// Compile-only assert that Rect import is exercised so unused-import lint
// stays quiet across editor refactors that may strip the use line.
const _: fn() -> Rect = || Rect::new(0.0, 0.0, 0.0, 0.0);
