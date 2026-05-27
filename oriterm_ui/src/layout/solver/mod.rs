//! Two-pass flex layout solver.
//!
//! Computes concrete positions and sizes from a [`LayoutBox`] descriptor tree.
//! Pass 1 measures children along the main axis, distributing remaining space
//! to `Fill`/`FillPortion` children proportionally. Pass 2 arranges children
//! at computed positions with justification and cross-axis alignment.

mod arrange;

use crate::geometry::Rect;

use self::arrange::{ArrangeCtx, ContainerMetrics, arrange_children};
use super::constraints::LayoutConstraints;
use super::flex::{Direction, FlexSpec};
use super::grid_solver;
use super::layout_box::{BoxContent, LayoutBox};
use super::layout_node::LayoutNode;
use super::size_spec::SizeSpec;

/// Computes a layout tree from a root descriptor and viewport rectangle.
///
/// The viewport provides maximum available space. The root box's `SizeSpec`
/// determines whether it fills that space or shrinks to content.
pub fn compute_layout(root: &LayoutBox, viewport: Rect) -> LayoutNode {
    let constraints = LayoutConstraints::loose(viewport.width(), viewport.height());
    solve(root, constraints, viewport.x(), viewport.y())
}

/// Recursively solves layout for a single box at a given position.
pub(super) fn solve(
    layout_box: &LayoutBox,
    constraints: LayoutConstraints,
    pos_x: f32,
    pos_y: f32,
) -> LayoutNode {
    // Apply margin: offset position, shrink available space.
    let mx = pos_x + layout_box.margin.left;
    let my = pos_y + layout_box.margin.top;
    let inner = constraints.shrink(layout_box.margin);

    // Merge box-level min/max with incoming constraints.
    // min_width/min_height use content-box semantics: the constraint applies to
    // the content area, and padding is added on top. Inflate by padding so the
    // outer-space constraint enforced by `constrain_width`/`constrain_height`
    // accounts for both the content minimum and the padding.
    let pad_w = layout_box.padding.width();
    let pad_h = layout_box.padding.height();
    let box_min_w = if layout_box.min_width > 0.0 {
        layout_box.min_width + pad_w
    } else {
        0.0
    };
    let box_min_h = if layout_box.min_height > 0.0 {
        layout_box.min_height + pad_h
    } else {
        0.0
    };
    let constrained = LayoutConstraints {
        min_width: inner.min_width.max(box_min_w),
        max_width: inner.max_width.min(layout_box.max_width),
        min_height: inner.min_height.max(box_min_h),
        max_height: inner.max_height.min(layout_box.max_height),
    };

    match &layout_box.content {
        BoxContent::Leaf { .. } => solve_leaf(layout_box, &constrained, mx, my),
        BoxContent::Flex {
            direction,
            align,
            justify,
            gap,
            children,
        } => solve_flex(
            layout_box,
            &constrained,
            (mx, my),
            FlexSpec {
                dir: *direction,
                align: *align,
                justify: *justify,
                gap: *gap,
            },
            children,
        ),
        BoxContent::Grid {
            columns,
            row_gap,
            column_gap,
            children,
        } => grid_solver::solve_grid(
            layout_box,
            grid_solver::GridSpec {
                columns,
                row_gap: *row_gap,
                column_gap: *column_gap,
            },
            children,
            constrained,
            (mx, my),
        ),
    }
}

/// Solves a leaf node: resolves `SizeSpec` against constraints + intrinsic size.
fn solve_leaf(
    layout_box: &LayoutBox,
    constraints: &LayoutConstraints,
    pos_x: f32,
    pos_y: f32,
) -> LayoutNode {
    let BoxContent::Leaf {
        intrinsic_width,
        intrinsic_height,
    } = &layout_box.content
    else {
        debug_assert!(false, "solve_leaf called on non-leaf");
        return LayoutNode::new(Rect::default(), Rect::default());
    };
    let (iw, ih) = (*intrinsic_width, *intrinsic_height);

    let width = resolve_size(
        layout_box.width,
        constraints.max_width,
        iw + layout_box.padding.width(),
    );
    let height = resolve_size(
        layout_box.height,
        constraints.max_height,
        ih + layout_box.padding.height(),
    );
    let width = constraints.constrain_width(width);
    let height = constraints.constrain_height(height);

    let rect = Rect::new(pos_x, pos_y, width, height);
    let content_rect = rect.inset(layout_box.padding);
    let mut node = LayoutNode::new(rect, content_rect);
    node.widget_id = layout_box.widget_id;
    node.sense = layout_box.sense;
    node.hit_test_behavior = layout_box.hit_test_behavior;
    node.clip = layout_box.clip;
    node.disabled = layout_box.disabled;
    node.interact_radius = layout_box.interact_radius;
    node.content_offset = layout_box.content_offset;
    node.pointer_events = layout_box.pointer_events;
    node.cursor_icon = layout_box.cursor_icon;
    node
}

/// Resolves a `SizeSpec` to a concrete pixel value.
fn resolve_size(spec: SizeSpec, available: f32, intrinsic: f32) -> f32 {
    match spec {
        SizeSpec::Fixed(val) => val,
        SizeSpec::Fill | SizeSpec::FillPortion(_) => {
            if available.is_finite() {
                available
            } else {
                intrinsic
            }
        }
        SizeSpec::Hug => intrinsic,
    }
}

/// Returns the main-axis `SizeSpec` for a box in the given direction.
fn main_axis_spec(layout_box: &LayoutBox, dir: Direction) -> SizeSpec {
    match dir {
        Direction::Row => layout_box.width,
        Direction::Column => layout_box.height,
    }
}

/// Solves a flex container using a two-pass algorithm.
fn solve_flex(
    layout_box: &LayoutBox,
    constraints: &LayoutConstraints,
    pos: (f32, f32),
    spec: FlexSpec,
    children: &[LayoutBox],
) -> LayoutNode {
    let (pos_x, pos_y) = pos;
    let FlexSpec { dir, gap, .. } = spec;
    if children.is_empty() {
        return solve_empty(layout_box, constraints, pos_x, pos_y);
    }

    let pad_main = dir.main_insets(layout_box.padding);
    let pad_cross = dir.cross_insets(layout_box.padding);
    let avail_main = dir.main(constraints.max_width, constraints.max_height);
    let avail_cross = dir.cross(constraints.max_width, constraints.max_height);

    // Content space = available minus padding.
    let content_main = if avail_main.is_finite() {
        avail_main - pad_main
    } else {
        f32::INFINITY
    };
    let content_cross = if avail_cross.is_finite() {
        avail_cross - pad_cross
    } else {
        f32::INFINITY
    };

    // Scroll containers use infinite main-axis constraints so children can
    // grow beyond the viewport. The container itself stays at viewport size.
    let measure_main = if layout_box.overflow {
        f32::INFINITY
    } else {
        content_main
    };
    let measured = measure_children(children, dir, measure_main, content_cross, gap);

    // Resolve container's own size.
    let container_main = resolve_container_main(
        layout_box,
        dir,
        constraints,
        measured.children_main + pad_main,
    );
    let container_cross =
        resolve_container_cross(layout_box, dir, constraints, measured.max_cross + pad_cross);

    // Pass 2: Position children.
    arrange_children(ArrangeCtx {
        layout_box,
        pos,
        spec,
        children,
        child_mains: &measured.child_mains,
        children_main: measured.children_main,
        metrics: ContainerMetrics {
            main: container_main,
            cross: container_cross,
            pad_main,
            pad_cross,
        },
    })
}

/// Results from the measurement pass.
struct MeasureResult {
    /// Main-axis size for each child.
    child_mains: Vec<f32>,
    /// Total main-axis extent of all children including gaps.
    children_main: f32,
    /// Maximum cross-axis extent among children.
    max_cross: f32,
}

/// Pass 1: Measures non-fill children and distributes space to fill children.
fn measure_children(
    children: &[LayoutBox],
    dir: Direction,
    content_main: f32,
    content_cross: f32,
    gap: f32,
) -> MeasureResult {
    let total_gap = if children.len() > 1 {
        gap * (children.len() - 1) as f32
    } else {
        0.0
    };

    let mut child_mains = vec![0.0_f32; children.len()];
    let mut used_main = total_gap;
    let mut total_fill: u32 = 0;
    let mut max_cross: f32 = 0.0;

    for (idx, child) in children.iter().enumerate() {
        let spec = main_axis_spec(child, dir);
        if spec.is_fill() {
            total_fill += spec.fill_weight();
        } else {
            let child_avail = if content_main.is_finite() {
                content_main - used_main
            } else {
                f32::INFINITY
            };
            let (cw, ch) = dir.compose(child_avail.max(0.0), content_cross);
            let measured = solve(child, LayoutConstraints::loose(cw, ch), 0.0, 0.0);
            let main_size = dir.main(measured.rect.width(), measured.rect.height());
            let cross_size = dir.cross(measured.rect.width(), measured.rect.height());
            child_mains[idx] = main_size;
            used_main += main_size;
            max_cross = max_cross.max(cross_size);
        }
    }

    // Distribute remaining space to fill children.
    if total_fill > 0 {
        let remaining = if content_main.is_finite() {
            (content_main - used_main).max(0.0)
        } else {
            0.0
        };
        let per_unit = remaining / total_fill as f32;
        for (idx, child) in children.iter().enumerate() {
            let spec = main_axis_spec(child, dir);
            if spec.is_fill() {
                child_mains[idx] = per_unit * spec.fill_weight() as f32;
            }
        }
    }

    let children_main: f32 = child_mains.iter().sum::<f32>() + total_gap;

    MeasureResult {
        child_mains,
        children_main,
        max_cross,
    }
}

/// Solves an empty flex container.
fn solve_empty(
    layout_box: &LayoutBox,
    constraints: &LayoutConstraints,
    pos_x: f32,
    pos_y: f32,
) -> LayoutNode {
    let width = resolve_size(
        layout_box.width,
        constraints.max_width,
        layout_box.padding.width(),
    );
    let height = resolve_size(
        layout_box.height,
        constraints.max_height,
        layout_box.padding.height(),
    );
    let width = constraints.constrain_width(width);
    let height = constraints.constrain_height(height);
    let rect = Rect::new(pos_x, pos_y, width, height);
    let content_rect = rect.inset(layout_box.padding);
    let mut node = LayoutNode::new(rect, content_rect);
    node.widget_id = layout_box.widget_id;
    node.sense = layout_box.sense;
    node.hit_test_behavior = layout_box.hit_test_behavior;
    node.clip = layout_box.clip;
    node.disabled = layout_box.disabled;
    node.interact_radius = layout_box.interact_radius;
    node.content_offset = layout_box.content_offset;
    node.pointer_events = layout_box.pointer_events;
    node.cursor_icon = layout_box.cursor_icon;
    node
}

/// Resolves the container's main-axis size.
fn resolve_container_main(
    layout_box: &LayoutBox,
    dir: Direction,
    constraints: &LayoutConstraints,
    children_with_padding: f32,
) -> f32 {
    let spec = main_axis_spec(layout_box, dir);
    let avail = dir.main(constraints.max_width, constraints.max_height);
    let raw = resolve_size(spec, avail, children_with_padding);
    match dir {
        Direction::Row => constraints.constrain_width(raw),
        Direction::Column => constraints.constrain_height(raw),
    }
}

/// Resolves the container's cross-axis size.
fn resolve_container_cross(
    layout_box: &LayoutBox,
    dir: Direction,
    constraints: &LayoutConstraints,
    content_with_padding: f32,
) -> f32 {
    let spec = match dir {
        Direction::Row => layout_box.height,
        Direction::Column => layout_box.width,
    };
    let avail = dir.cross(constraints.max_width, constraints.max_height);
    let raw = resolve_size(spec, avail, content_with_padding);
    match dir {
        Direction::Row => constraints.constrain_height(raw),
        Direction::Column => constraints.constrain_width(raw),
    }
}
