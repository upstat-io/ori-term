//! Computed layout output node.

use crate::geometry::Rect;
use crate::hit_test_behavior::HitTestBehavior;
use crate::sense::Sense;
use crate::widget_id::WidgetId;

/// A computed layout node — the output of the layout solver.
///
/// Each node stores its outer rectangle (including margin offset), the
/// content rectangle (outer rect inset by padding), and child nodes
/// for flex containers. Optionally carries a `WidgetId` for hit testing.
/// Sense flags and hit-test behavior control how the node participates
/// in hit testing.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutNode {
    /// Outer bounding rectangle (position relative to parent's content area).
    pub rect: Rect,
    /// Content area (rect inset by padding).
    pub content_rect: Rect,
    /// Child layout nodes (empty for leaves).
    pub children: Vec<Self>,
    /// Widget that owns this node (used by hit testing).
    pub widget_id: Option<WidgetId>,
    /// Sense flags for hit-test filtering.
    pub sense: Sense,
    /// Hit-test behavior relative to children.
    pub hit_test_behavior: HitTestBehavior,
    /// When `true`, children are clipped to this node's `rect` during
    /// hit testing and rendering.
    pub clip: bool,
    /// When `true`, widget is disabled and treated as `Sense::none()`.
    pub disabled: bool,
    /// Expands the hit area beyond `rect` for small targets.
    /// `0.0` means no expansion (default).
    pub interact_radius: f32,
}

impl LayoutNode {
    /// Creates a leaf node with no children and no widget ID.
    pub fn new(rect: Rect, content_rect: Rect) -> Self {
        Self {
            rect,
            content_rect,
            children: Vec::new(),
            widget_id: None,
            sense: Sense::all(),
            hit_test_behavior: HitTestBehavior::default(),
            clip: false,
            disabled: false,
            interact_radius: 0.0,
        }
    }

    /// Attaches children to this node.
    #[must_use]
    pub fn with_children(mut self, children: Vec<Self>) -> Self {
        self.children = children;
        self
    }

    /// Attaches a widget ID.
    #[must_use]
    pub fn with_widget_id(mut self, id: WidgetId) -> Self {
        self.widget_id = Some(id);
        self
    }

    /// Sets the sense flags for hit-test filtering.
    #[must_use]
    pub fn with_sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    /// Sets the hit-test behavior relative to children.
    #[must_use]
    pub fn with_hit_test_behavior(mut self, behavior: HitTestBehavior) -> Self {
        self.hit_test_behavior = behavior;
        self
    }

    /// Sets the clip flag for child hit testing and rendering.
    #[must_use]
    pub fn with_clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }

    /// Sets the disabled flag.
    #[must_use]
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets the interact radius for expanding hit areas.
    #[must_use]
    pub fn with_interact_radius(mut self, radius: f32) -> Self {
        self.interact_radius = radius;
        self
    }
}
