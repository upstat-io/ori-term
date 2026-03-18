//! Layout box descriptor — the input to the layout solver.
//!
//! A tree of `LayoutBox` nodes describes the desired sizing and arrangement.
//! Widgets will construct `LayoutBox` trees; the solver produces `LayoutNode`
//! trees as output.

use crate::geometry::Insets;
use crate::hit_test_behavior::HitTestBehavior;
use crate::sense::Sense;
use crate::widget_id::WidgetId;

use super::flex::{Align, Direction, Justify};
use super::size_spec::SizeSpec;

/// Column specification for grid layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GridColumns {
    /// Fixed number of columns.
    Fixed(usize),
    /// Fill as many columns as fit, each at least `min_width` wide.
    /// Remaining space distributed equally (CSS `auto-fill` behavior).
    AutoFill {
        /// Minimum column width in logical pixels.
        min_width: f32,
    },
}

/// Content of a layout box — either a leaf with intrinsic size, a
/// flex container, or a grid container.
#[derive(Debug, Clone, PartialEq)]
pub enum BoxContent {
    /// A leaf node with intrinsic dimensions.
    Leaf {
        /// Natural width of the content.
        intrinsic_width: f32,
        /// Natural height of the content.
        intrinsic_height: f32,
    },
    /// A flex container that arranges children along an axis.
    Flex {
        /// Layout direction.
        direction: Direction,
        /// Cross-axis alignment.
        align: Align,
        /// Main-axis justification.
        justify: Justify,
        /// Spacing between children along the main axis.
        gap: f32,
        /// Child layout boxes.
        children: Vec<LayoutBox>,
    },
    /// A grid container that arranges children in rows and columns.
    Grid {
        /// Column specification.
        columns: GridColumns,
        /// Vertical gap between rows.
        row_gap: f32,
        /// Horizontal gap between columns.
        column_gap: f32,
        /// Child layout boxes.
        children: Vec<LayoutBox>,
    },
}

/// A layout box describing desired size, spacing, and content.
///
/// This is a pure data descriptor — no rendering, no trait objects.
/// The layout solver reads the tree and produces [`super::LayoutNode`] output.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutBox {
    /// How width is determined.
    pub width: SizeSpec,
    /// How height is determined.
    pub height: SizeSpec,
    /// Inner padding (shrinks content area).
    pub padding: Insets,
    /// Outer margin (offsets position, consumes parent space).
    pub margin: Insets,
    /// Minimum width constraint (`0.0` = no minimum).
    pub min_width: f32,
    /// Maximum width constraint (`f32::INFINITY` = no maximum).
    pub max_width: f32,
    /// Minimum height constraint (`0.0` = no minimum).
    pub min_height: f32,
    /// Maximum height constraint (`f32::INFINITY` = no maximum).
    pub max_height: f32,
    /// What this box contains.
    pub content: BoxContent,
    /// Optional widget ID for hit testing and event routing.
    pub widget_id: Option<WidgetId>,
    /// Sense flags for hit-test filtering.
    pub sense: Sense,
    /// Hit-test behavior relative to children.
    pub hit_test_behavior: HitTestBehavior,
    /// Whether children are clipped to this box's bounds.
    pub clip: bool,
    /// Whether the widget is disabled (treated as `Sense::none()`).
    pub disabled: bool,
    /// Hit area expansion for small targets (pixels).
    pub interact_radius: f32,
    /// Content offset applied to children (scroll offset).
    ///
    /// When non-zero, hit testing translates the test point by this offset
    /// before checking children. Rendering applies the same offset via
    /// `push_translate`. Used by `ScrollWidget` to keep layout stable
    /// (children at natural positions) while offsetting interaction.
    pub content_offset: (f32, f32),
}

impl LayoutBox {
    /// Creates a leaf box with intrinsic dimensions and `Hug` sizing.
    pub fn leaf(intrinsic_width: f32, intrinsic_height: f32) -> Self {
        Self {
            width: SizeSpec::Hug,
            height: SizeSpec::Hug,
            padding: Insets::default(),
            margin: Insets::default(),
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: f32::INFINITY,
            content: BoxContent::Leaf {
                intrinsic_width,
                intrinsic_height,
            },
            widget_id: None,
            sense: Sense::all(),
            hit_test_behavior: HitTestBehavior::default(),
            clip: false,
            disabled: false,
            interact_radius: 0.0,
            content_offset: (0.0, 0.0),
        }
    }

    /// Creates a grid container with default gaps.
    pub fn grid(columns: GridColumns, children: Vec<Self>) -> Self {
        Self {
            width: SizeSpec::Hug,
            height: SizeSpec::Hug,
            padding: Insets::default(),
            margin: Insets::default(),
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: f32::INFINITY,
            content: BoxContent::Grid {
                columns,
                row_gap: 0.0,
                column_gap: 0.0,
                children,
            },
            widget_id: None,
            sense: Sense::all(),
            hit_test_behavior: HitTestBehavior::default(),
            clip: false,
            disabled: false,
            interact_radius: 0.0,
            content_offset: (0.0, 0.0),
        }
    }

    /// Creates a flex container with default alignment and justification.
    pub fn flex(direction: Direction, children: Vec<Self>) -> Self {
        Self {
            width: SizeSpec::Hug,
            height: SizeSpec::Hug,
            padding: Insets::default(),
            margin: Insets::default(),
            min_width: 0.0,
            max_width: f32::INFINITY,
            min_height: 0.0,
            max_height: f32::INFINITY,
            content: BoxContent::Flex {
                direction,
                align: Align::Start,
                justify: Justify::Start,
                gap: 0.0,
                children,
            },
            widget_id: None,
            sense: Sense::all(),
            hit_test_behavior: HitTestBehavior::default(),
            clip: false,
            disabled: false,
            interact_radius: 0.0,
            content_offset: (0.0, 0.0),
        }
    }

    /// Sets the width spec.
    #[must_use]
    pub fn with_width(mut self, spec: SizeSpec) -> Self {
        self.width = spec;
        self
    }

    /// Sets the height spec.
    #[must_use]
    pub fn with_height(mut self, spec: SizeSpec) -> Self {
        self.height = spec;
        self
    }

    /// Sets padding on all sides.
    #[must_use]
    pub fn with_padding(mut self, padding: Insets) -> Self {
        self.padding = padding;
        self
    }

    /// Sets margin on all sides.
    #[must_use]
    pub fn with_margin(mut self, margin: Insets) -> Self {
        self.margin = margin;
        self
    }

    /// Sets the minimum width.
    #[must_use]
    pub fn with_min_width(mut self, v: f32) -> Self {
        self.min_width = v;
        self
    }

    /// Sets the maximum width.
    #[must_use]
    pub fn with_max_width(mut self, v: f32) -> Self {
        self.max_width = v;
        self
    }

    /// Sets the minimum height.
    #[must_use]
    pub fn with_min_height(mut self, v: f32) -> Self {
        self.min_height = v;
        self
    }

    /// Sets the maximum height.
    #[must_use]
    pub fn with_max_height(mut self, v: f32) -> Self {
        self.max_height = v;
        self
    }

    /// Sets cross-axis alignment (only meaningful for flex containers).
    #[must_use]
    pub fn with_align(mut self, align: Align) -> Self {
        if let BoxContent::Flex {
            align: ref mut a, ..
        } = self.content
        {
            *a = align;
        }
        self
    }

    /// Sets main-axis justification (only meaningful for flex containers).
    #[must_use]
    pub fn with_justify(mut self, justify: Justify) -> Self {
        if let BoxContent::Flex {
            justify: ref mut j, ..
        } = self.content
        {
            *j = justify;
        }
        self
    }

    /// Sets the gap between children (only meaningful for flex containers).
    #[must_use]
    pub fn with_gap(mut self, gap: f32) -> Self {
        if let BoxContent::Flex { gap: ref mut g, .. } = self.content {
            *g = gap;
        }
        self
    }

    /// Sets the vertical gap between rows (only meaningful for grid containers).
    #[must_use]
    pub fn with_row_gap(mut self, gap: f32) -> Self {
        if let BoxContent::Grid {
            row_gap: ref mut g, ..
        } = self.content
        {
            *g = gap;
        }
        self
    }

    /// Sets the horizontal gap between columns (only meaningful for grid containers).
    #[must_use]
    pub fn with_column_gap(mut self, gap: f32) -> Self {
        if let BoxContent::Grid {
            column_gap: ref mut g,
            ..
        } = self.content
        {
            *g = gap;
        }
        self
    }

    /// Attaches a widget ID for hit testing and event routing.
    #[must_use]
    pub fn with_widget_id(mut self, id: WidgetId) -> Self {
        self.widget_id = Some(id);
        self
    }

    /// Sets sense flags for hit-test filtering.
    #[must_use]
    pub fn with_sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    /// Sets hit-test behavior relative to children.
    #[must_use]
    pub fn with_hit_test_behavior(mut self, behavior: HitTestBehavior) -> Self {
        self.hit_test_behavior = behavior;
        self
    }

    /// Sets the clip flag (children clipped to this box's bounds).
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

    /// Sets the content offset for scroll containers.
    ///
    /// The offset translates children during hit testing, matching the
    /// visual translate applied during rendering. Positive values scroll
    /// content upward (revealing content below).
    #[must_use]
    pub fn with_content_offset(mut self, x: f32, y: f32) -> Self {
        self.content_offset = (x, y);
        self
    }
}
