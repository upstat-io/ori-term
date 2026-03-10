//! Tests for icon path definitions.

use super::{IconId, IconStyle, PathCommand};

/// Every icon must have at least one `MoveTo` command.
#[test]
fn all_icons_have_move_to() {
    for id in ALL_ICONS {
        let path = id.path();
        let has_move = path
            .commands
            .iter()
            .any(|c| matches!(c, PathCommand::MoveTo(_, _)));
        assert!(has_move, "{id:?} has no MoveTo command");
    }
}

/// Every icon must have at least two commands (MoveTo + something).
#[test]
fn all_icons_have_multiple_commands() {
    for id in ALL_ICONS {
        let path = id.path();
        assert!(
            path.commands.len() >= 2,
            "{id:?} has only {} command(s)",
            path.commands.len()
        );
    }
}

/// Fill-style icons must have at least one `Close` command.
#[test]
fn fill_icons_have_close_command() {
    for id in ALL_ICONS {
        let path = id.path();
        if matches!(path.style, IconStyle::Fill) {
            let has_close = path
                .commands
                .iter()
                .any(|c| matches!(c, PathCommand::Close));
            assert!(has_close, "{id:?} is Fill style but has no Close command");
        }
    }
}

/// Stroke-style icons must have a positive stroke width.
#[test]
fn stroke_icons_have_positive_width() {
    for id in ALL_ICONS {
        let path = id.path();
        if let IconStyle::Stroke(w) = path.style {
            assert!(w > 0.0, "{id:?} has non-positive stroke width {w}");
        }
    }
}

/// All coordinates in path commands are within the 0.0–1.0 normalized range.
#[test]
fn coordinates_are_normalized() {
    for id in ALL_ICONS {
        let path = id.path();
        for (i, cmd) in path.commands.iter().enumerate() {
            let coords = extract_coords(cmd);
            for (x, y) in coords {
                assert!(
                    (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y),
                    "{id:?} command {i}: ({x}, {y}) outside 0.0–1.0"
                );
            }
        }
    }
}

/// `IconId::path()` returns the correct static definition for each variant.
#[test]
fn icon_id_path_round_trip() {
    // Spot-check: Close icon has Stroke style, Maximize has Stroke style with Close command.
    assert!(matches!(IconId::Close.path().style, IconStyle::Stroke(_)));
    assert!(matches!(
        IconId::Maximize.path().style,
        IconStyle::Stroke(_)
    ));
    assert!(
        IconId::Maximize
            .path()
            .commands
            .iter()
            .any(|c| matches!(c, PathCommand::Close))
    );
}

/// Restore icon has two sub-paths (two MoveTo commands for back + front windows).
#[test]
fn restore_icon_has_two_subpaths() {
    let move_count = IconId::Restore
        .path()
        .commands
        .iter()
        .filter(|c| matches!(c, PathCommand::MoveTo(_, _)))
        .count();
    assert!(
        move_count >= 2,
        "Restore icon should have >=2 MoveTo commands, got {move_count}"
    );
}

// Helpers

const ALL_ICONS: &[IconId] = &[
    IconId::Close,
    IconId::Plus,
    IconId::ChevronDown,
    IconId::Minimize,
    IconId::Maximize,
    IconId::Restore,
    IconId::WindowClose,
];

fn extract_coords(cmd: &PathCommand) -> Vec<(f32, f32)> {
    match *cmd {
        PathCommand::MoveTo(x, y) | PathCommand::LineTo(x, y) => vec![(x, y)],
        PathCommand::CubicTo(cx1, cy1, cx2, cy2, x, y) => {
            vec![(cx1, cy1), (cx2, cy2), (x, y)]
        }
        PathCommand::Close => vec![],
    }
}

/// Checks that `IconPath` implements the expected traits.
#[test]
fn icon_path_is_copy_and_debug() {
    let path = *IconId::Close.path();
    let _dbg = format!("{path:?}");
}

/// `IconId` is `Hash` + `Eq` (required for cache keys).
#[test]
fn icon_id_is_hashable() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    for id in ALL_ICONS {
        assert!(set.insert(id));
    }
    assert_eq!(set.len(), ALL_ICONS.len());
}
