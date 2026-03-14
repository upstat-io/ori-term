use crate::color::Color;
use crate::draw::DrawCommand;
use crate::draw::rect_style::RectStyle;
use crate::geometry::{Point, Rect};
use crate::widget_id::WidgetId;

use super::SceneNode;

fn sample_rect() -> Rect {
    Rect::new(10.0, 20.0, 100.0, 50.0)
}

fn sample_commands() -> Vec<DrawCommand> {
    vec![
        DrawCommand::Rect {
            rect: sample_rect(),
            style: RectStyle {
                fill: Some(Color::rgb(1.0, 0.0, 0.0)),
                ..RectStyle::default()
            },
        },
        DrawCommand::Line {
            from: Point::new(0.0, 0.0),
            to: Point::new(100.0, 0.0),
            width: 1.0,
            color: Color::WHITE,
        },
    ]
}

#[test]
fn new_node_is_invalid() {
    let id = WidgetId::next();
    let node = SceneNode::new(id);

    assert!(!node.is_valid());
    assert!(node.commands().is_empty());
    assert_eq!(node.bounds(), Rect::default());
    assert_eq!(node.widget_id(), id);
}

#[test]
fn update_makes_valid() {
    let mut node = SceneNode::new(WidgetId::next());
    let bounds = sample_rect();
    let cmds = sample_commands();

    node.update(cmds.clone(), bounds);

    assert!(node.is_valid());
    assert_eq!(node.commands().len(), 2);
    assert_eq!(node.bounds(), bounds);
}

#[test]
fn invalidate_clears_validity() {
    let mut node = SceneNode::new(WidgetId::next());
    node.update(sample_commands(), sample_rect());
    assert!(node.is_valid());

    node.invalidate();

    assert!(!node.is_valid());
    // Commands are retained — only validity flag changes.
    assert_eq!(node.commands().len(), 2);
}

#[test]
fn update_replaces_previous_commands() {
    let mut node = SceneNode::new(WidgetId::next());
    node.update(sample_commands(), sample_rect());
    assert_eq!(node.commands().len(), 2);

    let new_cmds = vec![DrawCommand::Line {
        from: Point::new(0.0, 0.0),
        to: Point::new(50.0, 50.0),
        width: 2.0,
        color: Color::BLACK,
    }];
    let new_bounds = Rect::new(0.0, 0.0, 200.0, 100.0);

    node.update(new_cmds, new_bounds);

    assert!(node.is_valid());
    assert_eq!(node.commands().len(), 1);
    assert_eq!(node.bounds(), new_bounds);
}

#[test]
fn invalidate_then_update_restores_validity() {
    let mut node = SceneNode::new(WidgetId::next());
    node.update(sample_commands(), sample_rect());
    node.invalidate();
    assert!(!node.is_valid());

    node.update(sample_commands(), sample_rect());
    assert!(node.is_valid());
}

#[test]
fn widget_id_is_preserved() {
    let id = WidgetId::next();
    let mut node = SceneNode::new(id);

    node.update(sample_commands(), sample_rect());
    node.invalidate();
    node.update(Vec::new(), Rect::default());

    assert_eq!(node.widget_id(), id);
}
