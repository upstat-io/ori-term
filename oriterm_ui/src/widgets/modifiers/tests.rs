//! Tests for widget modifiers.

use crate::layout::{BoxContent, Direction, LayoutBox, SizeSpec};
use crate::sense::Sense;
use crate::testing::MockMeasurer;
use crate::widget_id::WidgetId;
use crate::widgets::tests::TEST_THEME;
use crate::widgets::{LayoutCtx, Widget, WidgetAction};

use super::visibility::{VisibilityMode, VisibilityWidget};

/// Minimal test widget with a known size and focusability.
struct TestLeaf {
    id: WidgetId,
    focusable: bool,
}

impl TestLeaf {
    fn new(focusable: bool) -> Self {
        Self {
            id: WidgetId::next(),
            focusable,
        }
    }
}

impl Widget for TestLeaf {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        self.focusable
    }

    fn sense(&self) -> Sense {
        if self.focusable {
            Sense::focusable()
        } else {
            Sense::hover()
        }
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
        LayoutBox::leaf(100.0, 30.0)
            .with_widget_id(self.id)
            .with_sense(self.sense())
    }
}

/// Helper to build a `LayoutCtx` for testing.
fn test_layout_ctx() -> LayoutCtx<'static> {
    LayoutCtx {
        measurer: &MockMeasurer::STANDARD,
        theme: &TEST_THEME,
    }
}

// --- VisibilityMode ---

#[test]
fn visible_mode_delegates_layout() {
    let child = TestLeaf::new(false);
    let wrapper = VisibilityWidget::new(Box::new(child), VisibilityMode::Visible);
    let ctx = test_layout_ctx();
    let layout = wrapper.layout(&ctx);

    // Visible: child layout is included with its widget_id and sense intact.
    assert!(has_widget_id_in_tree(&layout));
}

#[test]
fn hidden_mode_preserves_layout_size() {
    let child = TestLeaf::new(false);
    let wrapper = VisibilityWidget::new(Box::new(child), VisibilityMode::Hidden);
    let ctx = test_layout_ctx();
    let layout = wrapper.layout(&ctx);

    // Hidden: the tree has content (a child leaf with intrinsic size).
    let children = flex_children(&layout);
    assert!(
        !children.is_empty(),
        "Hidden should still have child in layout"
    );
}

#[test]
fn hidden_mode_scrubs_widget_ids() {
    let child = TestLeaf::new(true);
    let wrapper = VisibilityWidget::new(Box::new(child), VisibilityMode::Hidden);
    let ctx = test_layout_ctx();
    let layout = wrapper.layout(&ctx);

    // The child subtree inside the wrapper should have no widget IDs
    // (scrubbed by for_layout_only). The wrapper's own ID is present.
    let children = flex_children(&layout);
    for c in children {
        assert!(
            c.widget_id.is_none(),
            "Hidden child should have widget_id scrubbed"
        );
        assert_eq!(c.sense, Sense::none(), "Hidden child should have no sense");
    }
}

#[test]
fn display_none_produces_zero_layout_size() {
    let child = TestLeaf::new(false);
    let wrapper = VisibilityWidget::new(Box::new(child), VisibilityMode::DisplayNone);
    let ctx = test_layout_ctx();
    let layout = wrapper.layout(&ctx);

    // DisplayNone: zero-size leaf.
    match &layout.content {
        BoxContent::Leaf {
            intrinsic_width,
            intrinsic_height,
        } => {
            assert_eq!(*intrinsic_width, 0.0);
            assert_eq!(*intrinsic_height, 0.0);
        }
        _ => panic!("DisplayNone should produce a leaf"),
    }
}

// --- Traversal ---

#[test]
fn for_each_child_mut_visits_visible() {
    let child = TestLeaf::new(false);
    let mut wrapper = VisibilityWidget::new(Box::new(child), VisibilityMode::Visible);
    let mut count = 0;
    wrapper.for_each_child_mut(&mut |_| count += 1);
    assert_eq!(count, 1, "Visible should visit child");
}

#[test]
fn for_each_child_mut_skips_hidden() {
    let child = TestLeaf::new(false);
    let mut wrapper = VisibilityWidget::new(Box::new(child), VisibilityMode::Hidden);
    let mut count = 0;
    wrapper.for_each_child_mut(&mut |_| count += 1);
    assert_eq!(count, 0, "Hidden should skip child in active traversal");
}

#[test]
fn for_each_child_mut_skips_display_none() {
    let child = TestLeaf::new(false);
    let mut wrapper = VisibilityWidget::new(Box::new(child), VisibilityMode::DisplayNone);
    let mut count = 0;
    wrapper.for_each_child_mut(&mut |_| count += 1);
    assert_eq!(
        count, 0,
        "DisplayNone should skip child in active traversal"
    );
}

#[test]
fn for_each_child_mut_all_visits_hidden() {
    let child = TestLeaf::new(false);
    let mut wrapper = VisibilityWidget::new(Box::new(child), VisibilityMode::Hidden);
    let mut count = 0;
    wrapper.for_each_child_mut_all(&mut |_| count += 1);
    assert_eq!(count, 1, "for_each_child_mut_all should always visit child");
}

#[test]
fn for_each_child_mut_all_visits_display_none() {
    let child = TestLeaf::new(false);
    let mut wrapper = VisibilityWidget::new(Box::new(child), VisibilityMode::DisplayNone);
    let mut count = 0;
    wrapper.for_each_child_mut_all(&mut |_| count += 1);
    assert_eq!(count, 1, "for_each_child_mut_all should always visit child");
}

// --- Focus ---

#[test]
fn focusable_children_empty_for_hidden() {
    let child = TestLeaf::new(true);
    let wrapper = VisibilityWidget::new(Box::new(child), VisibilityMode::Hidden);
    assert!(
        wrapper.focusable_children().is_empty(),
        "Hidden should report no focusable children"
    );
}

#[test]
fn focusable_children_empty_for_display_none() {
    let child = TestLeaf::new(true);
    let wrapper = VisibilityWidget::new(Box::new(child), VisibilityMode::DisplayNone);
    assert!(
        wrapper.focusable_children().is_empty(),
        "DisplayNone should report no focusable children"
    );
}

#[test]
fn focusable_children_delegates_for_visible() {
    let child = TestLeaf::new(true);
    let child_id = child.id();
    let wrapper = VisibilityWidget::new(Box::new(child), VisibilityMode::Visible);
    let ids = wrapper.focusable_children();
    assert_eq!(ids, vec![child_id]);
}

// --- accept_action ---

#[test]
fn accept_action_noop_for_hidden() {
    let child = TestLeaf::new(false);
    let mut wrapper = VisibilityWidget::new(Box::new(child), VisibilityMode::Hidden);
    let id = WidgetId::next();
    let action = WidgetAction::Clicked(id);
    assert!(
        !wrapper.accept_action(&action),
        "Hidden should not propagate actions"
    );
}

#[test]
fn accept_action_noop_for_display_none() {
    let child = TestLeaf::new(false);
    let mut wrapper = VisibilityWidget::new(Box::new(child), VisibilityMode::DisplayNone);
    let id = WidgetId::next();
    let action = WidgetAction::Clicked(id);
    assert!(
        !wrapper.accept_action(&action),
        "DisplayNone should not propagate actions"
    );
}

// --- Layout transparency (TPR-06-007) ---

/// Test leaf that returns a Fill-sized layout box.
struct FillLeaf {
    id: WidgetId,
}

impl FillLeaf {
    fn new() -> Self {
        Self {
            id: WidgetId::next(),
        }
    }
}

impl Widget for FillLeaf {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn sense(&self) -> Sense {
        Sense::hover()
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
        LayoutBox::leaf(0.0, 0.0)
            .with_width(SizeSpec::Fill)
            .with_height(SizeSpec::Fill)
            .with_widget_id(self.id)
            .with_sense(self.sense())
    }
}

/// Test leaf that returns a `FillPortion`-sized layout box.
struct FillPortionLeaf {
    id: WidgetId,
    portion: u32,
}

impl FillPortionLeaf {
    fn new(portion: u32) -> Self {
        Self {
            id: WidgetId::next(),
            portion,
        }
    }
}

impl Widget for FillPortionLeaf {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn sense(&self) -> Sense {
        Sense::hover()
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
        LayoutBox::leaf(0.0, 0.0)
            .with_width(SizeSpec::FillPortion(self.portion))
            .with_height(SizeSpec::FillPortion(self.portion))
            .with_widget_id(self.id)
            .with_sense(self.sense())
    }
}

#[test]
fn visibility_visible_preserves_fill_sizing() {
    let child = FillLeaf::new();
    let wrapper = VisibilityWidget::new(Box::new(child), VisibilityMode::Visible);
    let ctx = test_layout_ctx();
    let layout = wrapper.layout(&ctx);

    assert_eq!(
        layout.width,
        SizeSpec::Fill,
        "Visible wrapper should preserve Fill width"
    );
    assert_eq!(
        layout.height,
        SizeSpec::Fill,
        "Visible wrapper should preserve Fill height"
    );
}

#[test]
fn visibility_hidden_preserves_fill_sizing() {
    let child = FillLeaf::new();
    let wrapper = VisibilityWidget::new(Box::new(child), VisibilityMode::Hidden);
    let ctx = test_layout_ctx();
    let layout = wrapper.layout(&ctx);

    assert_eq!(
        layout.width,
        SizeSpec::Fill,
        "Hidden wrapper should preserve Fill width"
    );
    assert_eq!(
        layout.height,
        SizeSpec::Fill,
        "Hidden wrapper should preserve Fill height"
    );
}

#[test]
fn visibility_visible_preserves_fill_portion_sizing() {
    let child = FillPortionLeaf::new(3);
    let wrapper = VisibilityWidget::new(Box::new(child), VisibilityMode::Visible);
    let ctx = test_layout_ctx();
    let layout = wrapper.layout(&ctx);

    assert_eq!(
        layout.width,
        SizeSpec::FillPortion(3),
        "Visible wrapper should preserve FillPortion width"
    );
    assert_eq!(
        layout.height,
        SizeSpec::FillPortion(3),
        "Visible wrapper should preserve FillPortion height"
    );
}

#[test]
fn pointer_events_wrapper_preserves_fill_sizing() {
    use super::pointer_events::PointerEventsWidget;

    let child = FillLeaf::new();
    let wrapper = PointerEventsWidget::new(Box::new(child), false);
    let ctx = test_layout_ctx();
    let layout = wrapper.layout(&ctx);

    assert_eq!(
        layout.width,
        SizeSpec::Fill,
        "PointerEvents wrapper should preserve Fill width"
    );
    assert_eq!(
        layout.height,
        SizeSpec::Fill,
        "PointerEvents wrapper should preserve Fill height"
    );
}

#[test]
fn pointer_events_wrapper_preserves_fill_portion_sizing() {
    use super::pointer_events::PointerEventsWidget;

    let child = FillPortionLeaf::new(5);
    let wrapper = PointerEventsWidget::new(Box::new(child), true);
    let ctx = test_layout_ctx();
    let layout = wrapper.layout(&ctx);

    assert_eq!(
        layout.width,
        SizeSpec::FillPortion(5),
        "PointerEvents wrapper should preserve FillPortion width"
    );
    assert_eq!(
        layout.height,
        SizeSpec::FillPortion(5),
        "PointerEvents wrapper should preserve FillPortion height"
    );
}

// --- LayoutBox::for_layout_only ---

#[test]
fn for_layout_only_clears_widget_id() {
    let id = WidgetId::next();
    let b = LayoutBox::leaf(50.0, 20.0).with_widget_id(id);
    let scrubbed = b.for_layout_only();
    assert!(scrubbed.widget_id.is_none());
    assert_eq!(scrubbed.sense, Sense::none());
}

#[test]
fn for_layout_only_preserves_size() {
    let b = LayoutBox::leaf(50.0, 20.0)
        .with_widget_id(WidgetId::next())
        .with_width(SizeSpec::Fixed(100.0));
    let scrubbed = b.for_layout_only();
    assert_eq!(scrubbed.width, SizeSpec::Fixed(100.0));
    match &scrubbed.content {
        BoxContent::Leaf {
            intrinsic_width,
            intrinsic_height,
        } => {
            assert_eq!(*intrinsic_width, 50.0);
            assert_eq!(*intrinsic_height, 20.0);
        }
        _ => panic!("Should remain a leaf"),
    }
}

#[test]
fn for_layout_only_recurses_into_flex_children() {
    let child1 = LayoutBox::leaf(10.0, 10.0).with_widget_id(WidgetId::next());
    let child2 = LayoutBox::leaf(20.0, 20.0)
        .with_widget_id(WidgetId::next())
        .with_sense(Sense::click());

    let parent =
        LayoutBox::flex(Direction::Row, vec![child1, child2]).with_widget_id(WidgetId::next());

    let scrubbed = parent.for_layout_only();
    assert!(scrubbed.widget_id.is_none());

    let children = flex_children(&scrubbed);
    assert_eq!(children.len(), 2);
    for c in children {
        assert!(c.widget_id.is_none());
        assert_eq!(c.sense, Sense::none());
    }
}

// --- Pipeline integration (harness) ---

#[test]
fn visibility_visible_registers_child_widgets() {
    use crate::testing::WidgetTestHarness;

    let child = TestLeaf::new(true);
    let child_id = child.id();
    let wrapper = VisibilityWidget::new(Box::new(child), VisibilityMode::Visible);
    let harness = WidgetTestHarness::new(wrapper);

    let ids = harness.all_widget_ids();
    assert!(
        ids.contains(&child_id),
        "Visible child should appear in layout widget IDs"
    );
}

#[test]
fn visibility_hidden_removes_child_from_layout_ids() {
    use crate::testing::WidgetTestHarness;

    let child = TestLeaf::new(true);
    let child_id = child.id();
    let wrapper = VisibilityWidget::new(Box::new(child), VisibilityMode::Hidden);
    let harness = WidgetTestHarness::new(wrapper);

    let ids = harness.all_widget_ids();
    assert!(
        !ids.contains(&child_id),
        "Hidden child's widget ID should be scrubbed from layout"
    );
}

#[test]
fn visibility_hidden_drops_focus_targets() {
    use crate::testing::WidgetTestHarness;

    let child = TestLeaf::new(true);
    let child_id = child.id();
    let wrapper = VisibilityWidget::new(Box::new(child), VisibilityMode::Hidden);
    let harness = WidgetTestHarness::new(wrapper);

    let focus = harness.focusable_widgets();
    assert!(
        !focus.contains(&child_id),
        "Hidden child should not be in the focus order"
    );
}

#[test]
fn visibility_display_none_drops_focus_targets() {
    use crate::testing::WidgetTestHarness;

    let child = TestLeaf::new(true);
    let child_id = child.id();
    let wrapper = VisibilityWidget::new(Box::new(child), VisibilityMode::DisplayNone);
    let harness = WidgetTestHarness::new(wrapper);

    let focus = harness.focusable_widgets();
    assert!(
        !focus.contains(&child_id),
        "DisplayNone child should not be in the focus order"
    );
}

#[test]
fn visibility_visible_includes_child_in_focus_order() {
    use crate::testing::WidgetTestHarness;

    let child = TestLeaf::new(true);
    let child_id = child.id();
    let wrapper = VisibilityWidget::new(Box::new(child), VisibilityMode::Visible);
    let harness = WidgetTestHarness::new(wrapper);

    let focus = harness.focusable_widgets();
    assert!(
        focus.contains(&child_id),
        "Visible focusable child should be in the focus order"
    );
}

// --- Helpers ---

/// Returns `true` if any node in the layout tree has a `widget_id` set
/// (excluding the root wrapper node).
fn has_widget_id_in_tree(b: &LayoutBox) -> bool {
    match &b.content {
        BoxContent::Leaf { .. } => b.widget_id.is_some(),
        BoxContent::Flex { children, .. } | BoxContent::Grid { children, .. } => {
            children.iter().any(has_widget_id_in_tree)
        }
    }
}

/// Extracts the children vec from a flex `LayoutBox`.
fn flex_children(b: &LayoutBox) -> &[LayoutBox] {
    match &b.content {
        BoxContent::Flex { children, .. } => children,
        _ => &[],
    }
}
