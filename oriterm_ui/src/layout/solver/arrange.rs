//! Flex pass-2: position children with justification and cross-axis alignment.

use crate::geometry::Rect;

use super::super::constraints::LayoutConstraints;
use super::super::flex::{Align, Direction, FlexSpec, Justify};
use super::super::layout_box::LayoutBox;
use super::super::layout_node::LayoutNode;
use super::solve;

/// Resolved container extents and padding for the arrange pass.
#[derive(Debug, Clone, Copy)]
pub(super) struct ContainerMetrics {
    /// Main-axis extent of the container.
    pub main: f32,
    /// Cross-axis extent of the container.
    pub cross: f32,
    /// Main-axis padding total.
    pub pad_main: f32,
    /// Cross-axis padding total.
    pub pad_cross: f32,
}

/// All inputs to the pass-2 arrange step.
#[derive(Clone, Copy)]
pub(super) struct ArrangeCtx<'a> {
    /// The flex container box being arranged.
    pub layout_box: &'a LayoutBox,
    /// Container origin `(pos_x, pos_y)`.
    pub pos: (f32, f32),
    /// Flex direction / alignment / justification / gap.
    pub spec: FlexSpec,
    /// Child descriptors.
    pub children: &'a [LayoutBox],
    /// Per-child main-axis sizes from pass 1.
    pub child_mains: &'a [f32],
    /// Total main-axis extent of children including gaps.
    pub children_main: f32,
    /// Resolved container extents and padding.
    pub metrics: ContainerMetrics,
}

/// Pass 2: Positions children with justification and alignment.
pub(super) fn arrange_children(ctx: ArrangeCtx<'_>) -> LayoutNode {
    let ArrangeCtx {
        layout_box,
        pos: (pos_x, pos_y),
        spec:
            FlexSpec {
                dir,
                align,
                justify,
                gap,
            },
        children,
        child_mains,
        children_main,
        metrics:
            ContainerMetrics {
                main: container_main,
                cross: container_cross,
                pad_main,
                pad_cross,
            },
    } = ctx;
    let (start_offset, between) = compute_justification(
        justify,
        container_main - pad_main,
        children_main,
        children.len(),
    );

    let pad_main_start = dir.main_start(layout_box.padding);
    let pad_cross_start = dir.cross_start(layout_box.padding);
    let mut cursor = pad_main_start + start_offset;
    let child_cross_avail = container_cross - pad_cross;

    let mut child_nodes = Vec::with_capacity(children.len());

    for (idx, child) in children.iter().enumerate() {
        let child_main = child_mains[idx];

        // Solve child at cross-axis start position.
        let (cw, ch) = dir.compose(child_main, child_cross_avail);
        let child_constraints = LayoutConstraints::loose(cw, ch);
        let (cx, cy) = dir.compose(cursor, pad_cross_start);
        let mut node = solve(child, child_constraints, pos_x + cx, pos_y + cy);

        // Compute alignment offset using actual solved dimensions.
        let actual_cross = dir.cross(node.rect.width(), node.rect.height());
        let cross_offset = match align {
            Align::Start | Align::Stretch => 0.0,
            Align::Center => (child_cross_avail - actual_cross) / 2.0,
            Align::End => child_cross_avail - actual_cross,
        };
        if cross_offset.abs() > f32::EPSILON {
            offset_node_cross(&mut node, dir, cross_offset);
        }

        child_nodes.push(node);

        cursor += child_main + gap + between;
    }

    let (width, height) = dir.compose(container_main, container_cross);
    let rect = Rect::new(pos_x, pos_y, width, height);
    let content_rect = rect.inset(layout_box.padding);
    let mut node = LayoutNode::new(rect, content_rect).with_children(child_nodes);
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

/// Computes start offset and extra between-child spacing for justification.
fn compute_justification(justify: Justify, available: f32, used: f32, count: usize) -> (f32, f32) {
    let free = (available - used).max(0.0);
    match justify {
        Justify::Start => (0.0, 0.0),
        Justify::Center => (free / 2.0, 0.0),
        Justify::End => (free, 0.0),
        Justify::SpaceBetween => {
            if count <= 1 {
                (0.0, 0.0)
            } else {
                (0.0, free / (count - 1) as f32)
            }
        }
        Justify::SpaceAround => {
            if count == 0 {
                (0.0, 0.0)
            } else {
                let per = free / count as f32;
                (per / 2.0, per)
            }
        }
    }
}

/// Offsets a solved node and all descendants along the cross axis.
fn offset_node_cross(node: &mut LayoutNode, dir: Direction, delta: f32) {
    let (dx, dy) = match dir {
        Direction::Row => (0.0, delta),
        Direction::Column => (delta, 0.0),
    };
    node.rect = node.rect.offset(dx, dy);
    node.content_rect = node.content_rect.offset(dx, dy);
    for child in &mut node.children {
        offset_node_cross(child, dir, delta);
    }
}
