//! Grid layout solver.
//!
//! Arranges children in a row-major grid with configurable column count
//! (fixed or auto-fill). Each row's height is the maximum child height
//! in that row. Column widths are uniform.

use crate::geometry::Rect;

use super::constraints::LayoutConstraints;
use super::layout_box::{GridColumns, LayoutBox};
use super::layout_node::LayoutNode;
use super::size_spec::SizeSpec;
use super::solver;

/// Solves a grid container, returning a `LayoutNode` with positioned children.
#[expect(clippy::too_many_arguments, reason = "extracted grid params from enum")]
pub(super) fn solve_grid(
    layout_box: &LayoutBox,
    columns: &GridColumns,
    row_gap: f32,
    column_gap: f32,
    children: &[LayoutBox],
    constraints: LayoutConstraints,
    pos_x: f32,
    pos_y: f32,
) -> LayoutNode {
    if children.is_empty() {
        return solve_empty_grid(layout_box, &constraints, pos_x, pos_y);
    }

    let avail_w = constraints.max_width - layout_box.padding.width();
    let avail_w = if avail_w.is_finite() { avail_w } else { 0.0 };

    let num_cols = resolve_column_count(columns, avail_w, column_gap);
    let col_w = column_width(avail_w, num_cols, column_gap);

    // Solve each child with the computed column width.
    let child_constraints = LayoutConstraints::loose(col_w, f32::INFINITY);
    let solved: Vec<LayoutNode> = children
        .iter()
        .map(|c| solver::solve(c, child_constraints, 0.0, 0.0))
        .collect();

    // Compute row heights.
    let num_rows = children.len().div_ceil(num_cols);
    let mut row_heights = vec![0.0_f32; num_rows];
    for (idx, node) in solved.iter().enumerate() {
        let row = idx / num_cols;
        row_heights[row] = row_heights[row].max(node.rect.height());
    }

    // Position children at their grid cells.
    let pad_left = layout_box.padding.left;
    let pad_top = layout_box.padding.top;

    let mut child_nodes = Vec::with_capacity(children.len());
    let mut row_y = pad_top;
    for (row, row_h) in row_heights.iter().enumerate() {
        let row_start = row * num_cols;
        let row_end = (row_start + num_cols).min(children.len());
        for col in 0..(row_end - row_start) {
            let idx = row_start + col;
            let cell_x = pos_x + pad_left + col as f32 * (col_w + column_gap);
            let cell_y = pos_y + row_y;
            let node = solver::solve(&children[idx], child_constraints, cell_x, cell_y);
            child_nodes.push(node);
        }
        row_y += row_h + row_gap;
    }

    // Total content height.
    let total_row_gap = if num_rows > 1 {
        row_gap * (num_rows - 1) as f32
    } else {
        0.0
    };
    let content_h: f32 = row_heights.iter().sum::<f32>() + total_row_gap;

    let width = resolve_grid_size(
        layout_box.width,
        constraints.max_width,
        avail_w + layout_box.padding.width(),
    );
    let height = resolve_grid_size(
        layout_box.height,
        constraints.max_height,
        content_h + layout_box.padding.height(),
    );
    let width = constraints.constrain_width(width);
    let height = constraints.constrain_height(height);

    let rect = Rect::new(pos_x, pos_y, width, height);
    let content_rect = rect.inset(layout_box.padding);
    let mut node = LayoutNode::new(rect, content_rect).with_children(child_nodes);
    node.widget_id = layout_box.widget_id;
    node.sense = layout_box.sense;
    node.hit_test_behavior = layout_box.hit_test_behavior;
    node.clip = layout_box.clip;
    node.disabled = layout_box.disabled;
    node.interact_radius = layout_box.interact_radius;
    node
}

/// Resolves column count from the `GridColumns` spec and available width.
fn resolve_column_count(columns: &GridColumns, avail_w: f32, column_gap: f32) -> usize {
    match columns {
        GridColumns::Fixed(n) => (*n).max(1),
        GridColumns::AutoFill { min_width } => {
            if avail_w <= 0.0 || *min_width <= 0.0 {
                return 1;
            }
            // How many columns fit? Each column is at least `min_width`, with
            // `column_gap` between them: n * min_width + (n-1) * gap <= avail_w.
            // Solving: n <= (avail_w + gap) / (min_width + gap).
            let n = ((avail_w + column_gap) / (min_width + column_gap)).floor() as usize;
            n.max(1)
        }
    }
}

/// Computes the uniform column width for `n` columns in `avail_w` with gaps.
fn column_width(avail_w: f32, num_cols: usize, column_gap: f32) -> f32 {
    let gaps = if num_cols > 1 {
        column_gap * (num_cols - 1) as f32
    } else {
        0.0
    };
    ((avail_w - gaps) / num_cols as f32).max(0.0)
}

/// Resolves a `SizeSpec` to a concrete pixel value for grid containers.
fn resolve_grid_size(spec: SizeSpec, available: f32, intrinsic: f32) -> f32 {
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

/// Solves an empty grid container.
fn solve_empty_grid(
    layout_box: &LayoutBox,
    constraints: &LayoutConstraints,
    pos_x: f32,
    pos_y: f32,
) -> LayoutNode {
    let width = resolve_grid_size(
        layout_box.width,
        constraints.max_width,
        layout_box.padding.width(),
    );
    let height = resolve_grid_size(
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
    node
}
