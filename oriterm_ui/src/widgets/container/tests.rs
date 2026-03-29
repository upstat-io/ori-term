use crate::draw::Scene;
use crate::geometry::{Insets, Rect};
use crate::layout::{Align, Justify, SizeSpec, compute_layout};
use crate::sense::Sense;
use crate::widgets::button::ButtonWidget;
use crate::widgets::label::LabelWidget;
use crate::widgets::panel::PanelWidget;
use crate::widgets::spacer::SpacerWidget;
use crate::widgets::tests::MockMeasurer;
use crate::widgets::{DrawCtx, LayoutCtx, Widget, WidgetAction};

use super::ContainerWidget;

struct CountingWidget {
    id: crate::widget_id::WidgetId,
    size: Rect,
    draws: std::rc::Rc<std::cell::Cell<usize>>,
}

impl CountingWidget {
    fn new(width: f32, height: f32, draws: std::rc::Rc<std::cell::Cell<usize>>) -> Self {
        Self {
            id: crate::widget_id::WidgetId::next(),
            size: Rect::new(0.0, 0.0, width, height),
            draws,
        }
    }
}

impl Widget for CountingWidget {
    fn id(&self) -> crate::widget_id::WidgetId {
        self.id
    }

    fn sense(&self) -> Sense {
        Sense::none()
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> crate::layout::LayoutBox {
        crate::layout::LayoutBox::leaf(self.size.width(), self.size.height())
            .with_widget_id(self.id)
    }

    fn paint(&self, _ctx: &mut DrawCtx<'_>) {
        self.draws.set(self.draws.get() + 1);
    }

    fn accept_action(&mut self, _action: &WidgetAction) -> bool {
        false
    }

    fn focusable_children(&self) -> Vec<crate::widget_id::WidgetId> {
        Vec::new()
    }
}

fn label(text: &str) -> Box<dyn Widget> {
    Box::new(LabelWidget::new(text))
}

// --- Layout tests ---

#[test]
fn row_layout_places_children_horizontally() {
    let row = ContainerWidget::row().with_children(vec![label("AB"), label("CD")]);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = row.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);

    assert_eq!(node.children.len(), 2);
    assert_eq!(node.rect.width(), 32.0);
    assert_eq!(node.rect.height(), 16.0);
    assert_eq!(node.children[0].rect.x(), 0.0);
    assert_eq!(node.children[1].rect.x(), 16.0);
}

#[test]
fn column_layout_places_children_vertically() {
    let col = ContainerWidget::column().with_children(vec![label("AB"), label("CD")]);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = col.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);

    assert_eq!(node.children.len(), 2);
    assert_eq!(node.rect.width(), 16.0);
    assert_eq!(node.rect.height(), 32.0);
    assert_eq!(node.children[0].rect.y(), 0.0);
    assert_eq!(node.children[1].rect.y(), 16.0);
}

#[test]
fn row_with_gap() {
    let row = ContainerWidget::row()
        .with_children(vec![label("A"), label("B")])
        .with_gap(10.0);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = row.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);

    assert_eq!(node.rect.width(), 26.0);
    assert_eq!(node.children[0].rect.x(), 0.0);
    assert_eq!(node.children[1].rect.x(), 18.0);
}

#[test]
fn row_with_spacer_pushes_apart() {
    let row = ContainerWidget::row().with_children(vec![
        label("L"),
        Box::new(SpacerWidget::fill()),
        label("R"),
    ]);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let mut layout_box = row.layout(&ctx);
    layout_box.width = SizeSpec::Fill;
    let viewport = Rect::new(0.0, 0.0, 100.0, 50.0);
    let node = compute_layout(&layout_box, viewport);

    assert_eq!(node.children[0].rect.x(), 0.0);
    assert_eq!(node.children[2].rect.x(), 92.0);
}

#[test]
fn column_with_center_align() {
    let col = ContainerWidget::column()
        .with_children(vec![label("AB"), label("ABCD")])
        .with_align(Align::Center);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = col.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);

    assert_eq!(node.children[0].rect.x(), 8.0);
    assert_eq!(node.children[1].rect.x(), 0.0);
}

#[test]
fn row_with_justify_space_between() {
    let row = ContainerWidget::row()
        .with_children(vec![label("A"), label("B"), label("C")])
        .with_justify(Justify::SpaceBetween);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let mut layout_box = row.layout(&ctx);
    layout_box.width = SizeSpec::Fill;
    let viewport = Rect::new(0.0, 0.0, 100.0, 50.0);
    let node = compute_layout(&layout_box, viewport);

    assert_eq!(node.children[0].rect.x(), 0.0);
    assert_eq!(node.children[1].rect.x(), 46.0);
    assert_eq!(node.children[2].rect.x(), 92.0);
}

#[test]
fn row_with_padding() {
    let row = ContainerWidget::row()
        .with_children(vec![label("A")])
        .with_padding(Insets::all(10.0));
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = row.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);

    // "A" = 8x16. With 10px padding all around: 28x36.
    assert_eq!(node.rect.width(), 28.0);
    assert_eq!(node.rect.height(), 36.0);
    // Child at (10, 10) inside the padded area.
    assert_eq!(node.children[0].rect.x(), 10.0);
    assert_eq!(node.children[0].rect.y(), 10.0);
}

#[test]
fn empty_container_produces_correct_layout() {
    let row = ContainerWidget::row();
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = row.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);
    assert_eq!(node.rect.width(), 0.0);
    assert_eq!(node.rect.height(), 0.0);
}

#[test]
fn container_not_focusable() {
    let row = ContainerWidget::row();
    assert!(!row.is_focusable());
}

#[test]
fn child_count_tracks_children() {
    let row = ContainerWidget::row().with_children(vec![label("A"), label("B"), label("C")]);
    assert_eq!(row.child_count(), 3);
}

// --- Child management tests ---

#[test]
fn add_child_increases_count() {
    let mut row = ContainerWidget::row();
    assert_eq!(row.child_count(), 0);
    row.add_child(label("A"));
    assert_eq!(row.child_count(), 1);
    row.add_child(label("B"));
    assert_eq!(row.child_count(), 2);
}

#[test]
fn remove_child_decreases_count() {
    let mut row = ContainerWidget::row().with_children(vec![label("A"), label("B"), label("C")]);
    assert_eq!(row.child_count(), 3);
    let _ = row.remove_child(1);
    assert_eq!(row.child_count(), 2);
}

#[test]
fn with_children_builder() {
    let row = ContainerWidget::row()
        .with_child(label("A"))
        .with_child(label("B"));
    assert_eq!(row.child_count(), 2);
}

#[test]
fn draw_skips_children_fully_outside_active_clip() {
    let draws = std::rc::Rc::new(std::cell::Cell::new(0));
    let row = ContainerWidget::column()
        .with_child(Box::new(CountingWidget::new(100.0, 20.0, draws.clone())))
        .with_child(Box::new(CountingWidget::new(100.0, 20.0, draws.clone())));

    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    scene.push_clip(Rect::new(0.0, 0.0, 100.0, 20.0));
    let mut ctx = DrawCtx {
        measurer: &measurer,
        scene: &mut scene,
        bounds: Rect::new(0.0, 0.0, 100.0, 40.0),
        now: std::time::Instant::now(),
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };

    row.paint(&mut ctx);

    assert_eq!(draws.get(), 1, "only the visible child should draw");
}

#[test]
fn focusable_children_collects_recursively() {
    let btn = ButtonWidget::new("OK");
    let btn_id = btn.id();
    let inner = ContainerWidget::row().with_child(Box::new(btn));
    let outer = ContainerWidget::column()
        .with_child(label("Title"))
        .with_child(Box::new(inner));
    let ids = outer.focusable_children();
    assert_eq!(ids, vec![btn_id]);
}

// --- Draw tests ---

#[test]
fn draw_delegates_to_children() {
    let row = ContainerWidget::row().with_children(vec![label("A"), label("B")]);
    let measurer = MockMeasurer::STANDARD;
    let mut scene = Scene::new();
    let bounds = Rect::new(0.0, 0.0, 100.0, 50.0);
    let mut ctx = DrawCtx {
        measurer: &measurer,
        scene: &mut scene,
        bounds,
        now: std::time::Instant::now(),
        theme: &super::super::tests::TEST_THEME,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: None,
    };
    row.paint(&mut ctx);

    assert_eq!(scene.text_runs().len(), 2);
}

// --- Nested container tests ---

#[test]
fn deeply_nested_layout_correct() {
    let inner = ContainerWidget::row()
        .with_children(vec![label("A"), label("B")])
        .with_gap(4.0);
    let outer = ContainerWidget::column().with_children(vec![
        label("Header"),
        Box::new(inner),
        label("Footer"),
    ]);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = outer.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);

    assert_eq!(node.children.len(), 3);
    assert_eq!(node.children[0].rect.y(), 0.0);
    assert_eq!(node.children[1].rect.y(), 16.0);
    assert_eq!(node.children[2].rect.y(), 32.0);
    let inner = &node.children[1];
    assert_eq!(inner.rect.width(), 20.0);
    assert_eq!(inner.children.len(), 2);
    assert_eq!(inner.children[0].rect.x(), 0.0);
    assert_eq!(inner.children[1].rect.x(), 12.0);
}

#[test]
fn panel_inside_container_layout() {
    let panel = PanelWidget::new(Box::new(LabelWidget::new("Inner")));
    let row = ContainerWidget::row().with_children(vec![label("Before"), Box::new(panel)]);
    let ctx = LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &super::super::tests::TEST_THEME,
    };
    let layout_box = row.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 400.0, 300.0);
    let node = compute_layout(&layout_box, viewport);

    assert_eq!(node.children.len(), 2);
    assert_eq!(node.children[0].rect.width(), 48.0);
    assert_eq!(node.children[1].rect.width(), 64.0);
    assert_eq!(node.children[1].rect.x(), 48.0);
}

// --- Cache tests ---

#[test]
fn layout_cache_returns_same_result_for_same_bounds() {
    let mut row = ContainerWidget::row().with_children(vec![label("A")]);
    let measurer = MockMeasurer::STANDARD;
    let theme = super::super::tests::TEST_THEME;
    let bounds = Rect::new(0.0, 0.0, 100.0, 50.0);

    // First layout populates the cache and clears the dirty flag.
    let node1 = row.get_or_compute_layout(&measurer, &theme, bounds);
    row.clear_dirty();

    let node2 = row.get_or_compute_layout(&measurer, &theme, bounds);
    // Same Rc (pointer equality) — cache hit.
    assert!(std::rc::Rc::ptr_eq(&node1, &node2));
}

#[test]
fn layout_cache_recomputes_for_different_bounds() {
    let row = ContainerWidget::row().with_children(vec![label("A")]);
    let measurer = MockMeasurer::STANDARD;
    let theme = super::super::tests::TEST_THEME;

    let bounds1 = Rect::new(0.0, 0.0, 100.0, 50.0);
    let bounds2 = Rect::new(0.0, 0.0, 200.0, 100.0);
    let node1 = row.get_or_compute_layout(&measurer, &theme, bounds1);
    let node2 = row.get_or_compute_layout(&measurer, &theme, bounds2);
    assert!(!std::rc::Rc::ptr_eq(&node1, &node2));
}

// --- Dirty tracking ---

#[test]
fn new_container_starts_dirty() {
    let c = ContainerWidget::column();
    assert!(c.needs_paint());
    assert!(c.needs_layout());
}
