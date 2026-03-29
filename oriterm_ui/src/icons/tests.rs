//! Tests for icon path definitions.

use super::{IconId, IconStyle, PathCommand};

/// Every icon must have at least one `MoveTo` command.
#[test]
fn all_icons_have_move_to() {
    for id in IconId::ALL {
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
    for id in IconId::ALL {
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
    for id in IconId::ALL {
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
    for id in IconId::ALL {
        let path = id.path();
        if let IconStyle::Stroke(w) = path.style {
            assert!(w > 0.0, "{id:?} has non-positive stroke width {w}");
        }
    }
}

/// All coordinates in path commands are within the 0.0–1.0 normalized range.
#[test]
fn coordinates_are_normalized() {
    for id in IconId::ALL {
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
    for id in IconId::ALL {
        assert!(set.insert(id));
    }
    assert_eq!(set.len(), IconId::ALL.len());
}

// Sidebar SVG fixture tests

const SIDEBAR_ICONS: &[IconId] = &[
    IconId::Sun,
    IconId::Palette,
    IconId::Type,
    IconId::Terminal,
    IconId::Keyboard,
    IconId::Window,
    IconId::Bell,
    IconId::Activity,
];

/// Every sidebar icon has a corresponding SVG fixture.
#[test]
fn sidebar_fixtures_cover_all_sidebar_icons() {
    use super::sidebar_fixtures::SIDEBAR_ICON_SOURCES;

    for &id in SIDEBAR_ICONS {
        assert!(
            SIDEBAR_ICON_SOURCES.iter().any(|f| f.id == id),
            "{id:?} has no SVG fixture"
        );
    }
}

/// All sidebar fixtures target logical size 16.
#[test]
fn sidebar_fixtures_target_size_16() {
    use super::sidebar_fixtures::SIDEBAR_ICON_SOURCES;

    for fixture in &SIDEBAR_ICON_SOURCES {
        assert_eq!(
            fixture.logical_size, 16,
            "{:?} fixture has logical_size {}, expected 16",
            fixture.id, fixture.logical_size
        );
    }
}

/// Fixture SVG strings are non-empty and contain a viewBox.
#[test]
fn sidebar_fixtures_have_valid_svg() {
    use super::sidebar_fixtures::SIDEBAR_ICON_SOURCES;

    for fixture in &SIDEBAR_ICON_SOURCES {
        assert!(
            !fixture.svg.is_empty(),
            "{:?} fixture has empty SVG",
            fixture.id
        );
        assert!(
            fixture.svg.contains("viewBox"),
            "{:?} fixture SVG missing viewBox",
            fixture.id
        );
        assert!(
            fixture.svg.contains("0 0 24 24"),
            "{:?} fixture SVG should use 24×24 viewBox",
            fixture.id
        );
    }
}

// SVG importer tests

/// SVG importer produces normalized commands for all 8 sidebar fixtures.
#[test]
fn svg_import_produces_commands_for_all_fixtures() {
    use super::sidebar_fixtures::SIDEBAR_ICON_SOURCES;
    use super::svg_import::svg_to_commands;

    for fixture in &SIDEBAR_ICON_SOURCES {
        let cmds = svg_to_commands(fixture.svg, 24.0);
        assert!(
            cmds.len() >= 2,
            "{:?}: expected >=2 commands, got {}",
            fixture.id,
            cmds.len()
        );
    }
}

/// SVG importer normalizes coordinates to 0.0–1.0.
#[test]
fn svg_import_normalizes_coordinates() {
    use super::svg_import::svg_to_commands;

    // Simple line from (6,12) to (18,12) in 24×24 viewBox.
    let svg = r#"<svg viewBox="0 0 24 24"><line x1="6" y1="12" x2="18" y2="12"/></svg>"#;
    let cmds = svg_to_commands(svg, 24.0);

    assert_eq!(cmds.len(), 2);
    assert_eq!(cmds[0], PathCommand::MoveTo(0.25, 0.5));
    assert_eq!(cmds[1], PathCommand::LineTo(0.75, 0.5));
}

/// SVG importer handles circle elements.
#[test]
fn svg_import_circle_to_cubics() {
    use super::svg_import::svg_to_commands;

    let svg = r#"<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="5"/></svg>"#;
    let cmds = svg_to_commands(svg, 24.0);

    // Circle = MoveTo + 4 CubicTo + Close = 6 commands.
    assert_eq!(cmds.len(), 6);
    assert!(matches!(cmds[0], PathCommand::MoveTo(..)));
    assert!(matches!(cmds[5], PathCommand::Close));
    // All cubics should have normalized coordinates in 0.0–1.0.
    for cmd in &cmds {
        for (x, y) in extract_coords(cmd) {
            assert!(
                (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y),
                "circle coord ({x}, {y}) outside 0.0–1.0"
            );
        }
    }
}

/// SVG importer handles rounded rect elements.
#[test]
fn svg_import_rounded_rect() {
    use super::svg_import::svg_to_commands;

    let svg = r#"<svg viewBox="0 0 24 24"><rect x="2" y="4" width="20" height="16" rx="2"/></svg>"#;
    let cmds = svg_to_commands(svg, 24.0);

    // Rounded rect: MoveTo + 4×(LineTo + CubicTo) + Close = 10 commands.
    assert_eq!(cmds.len(), 10);
    assert!(matches!(cmds[0], PathCommand::MoveTo(..)));
    assert!(matches!(cmds[9], PathCommand::Close));
}

/// SVG importer handles polyline elements.
#[test]
fn svg_import_polyline() {
    use super::svg_import::svg_to_commands;

    let svg = r#"<svg viewBox="0 0 24 24"><polyline points="4 17 10 11 4 5"/></svg>"#;
    let cmds = svg_to_commands(svg, 24.0);

    // polyline: MoveTo + 2 LineTo = 3 commands.
    assert_eq!(cmds.len(), 3);
    assert!(matches!(cmds[0], PathCommand::MoveTo(..)));
}

/// SVG importer handles path data with relative commands.
#[test]
fn svg_import_path_relative_commands() {
    use super::svg_import::svg_to_commands;

    // Activity icon: M22 12h-4l-3 9L9 3l-3 9H2
    let svg = r#"<svg viewBox="0 0 24 24"><path d="M22 12h-4l-3 9L9 3l-3 9H2"/></svg>"#;
    let cmds = svg_to_commands(svg, 24.0);

    assert!(cmds.len() >= 6, "expected 6+ commands, got {}", cmds.len());
    // First command should be MoveTo(22/24, 12/24).
    let expected_x = 22.0 / 24.0;
    let expected_y = 12.0 / 24.0;
    if let PathCommand::MoveTo(x, y) = cmds[0] {
        assert!(
            (x - expected_x).abs() < 0.001,
            "x={x}, expected {expected_x}"
        );
        assert!(
            (y - expected_y).abs() < 0.001,
            "y={y}, expected {expected_y}"
        );
    } else {
        panic!("expected MoveTo, got {:?}", cmds[0]);
    }
}

/// Dump generated Rust source for all 8 sidebar icons (run with --nocapture).
#[test]
#[ignore = "codegen helper — run manually with --nocapture to see output"]
fn dump_sidebar_icon_rust_source() {
    use super::sidebar_fixtures::SIDEBAR_ICON_SOURCES;
    use super::svg_import::{commands_to_rust_source, svg_to_commands};

    for fixture in &SIDEBAR_ICON_SOURCES {
        let cmds = svg_to_commands(fixture.svg, 24.0);
        println!("// {:?}", fixture.id);
        println!(
            "static ICON_{}: IconPath = IconPath {{",
            format!("{:?}", fixture.id).to_uppercase()
        );
        println!("    commands: {},", commands_to_rust_source(&cmds));
        println!("    style: IconStyle::Stroke(NAV_STROKE),");
        println!("}};");
        println!();
    }
}

/// Source-to-runtime equivalence: svg_to_commands output matches stored
/// PathCommand definitions for all 8 sidebar icons.
///
/// This catches drift between the SVG fixtures and the checked-in
/// sidebar_nav.rs definitions. If this fails, regenerate sidebar_nav.rs
/// using the `dump_sidebar_icon_rust_source` helper.
#[test]
fn sidebar_source_commands_match_runtime() {
    use super::sidebar_fixtures::SIDEBAR_ICON_SOURCES;
    use super::svg_import::svg_to_commands;

    for fixture in &SIDEBAR_ICON_SOURCES {
        let source_cmds = svg_to_commands(fixture.svg, 24.0);
        let runtime_cmds = fixture.id.path().commands;

        assert_eq!(
            source_cmds.len(),
            runtime_cmds.len(),
            "{:?}: command count mismatch — source {} vs runtime {}",
            fixture.id,
            source_cmds.len(),
            runtime_cmds.len(),
        );

        for (idx, (src, rt)) in source_cmds.iter().zip(runtime_cmds.iter()).enumerate() {
            let max_coord_diff = command_max_diff(src, rt);
            // 6-decimal codegen truncation can introduce ~0.0000005 per coordinate.
            assert!(
                max_coord_diff < 0.001,
                "{:?} command {idx}: coordinate diff {max_coord_diff:.8} — \
                 source and runtime definitions have diverged",
                fixture.id,
            );
        }
    }
}

/// Maximum coordinate difference between two PathCommands.
fn command_max_diff(a: &PathCommand, b: &PathCommand) -> f32 {
    match (a, b) {
        (PathCommand::MoveTo(ax, ay), PathCommand::MoveTo(bx, by))
        | (PathCommand::LineTo(ax, ay), PathCommand::LineTo(bx, by)) => {
            (ax - bx).abs().max((ay - by).abs())
        }
        (
            PathCommand::CubicTo(ax1, ay1, ax2, ay2, ax, ay),
            PathCommand::CubicTo(bx1, by1, bx2, by2, bx, by),
        ) => (ax1 - bx1)
            .abs()
            .max((ay1 - by1).abs())
            .max((ax2 - bx2).abs())
            .max((ay2 - by2).abs())
            .max((ax - bx).abs())
            .max((ay - by).abs()),
        (PathCommand::Close, PathCommand::Close) => 0.0,
        _ => f32::MAX, // Different command types.
    }
}

/// SVG importer handles SVG arc commands (A/a).
#[test]
fn svg_import_arc_to_cubics() {
    use super::svg_import::svg_to_commands;

    // Simple arc: half circle from (0,0) to (10,0) with radius 5.
    let svg = r#"<svg viewBox="0 0 24 24"><path d="M7 12A5 5 0 0117 12"/></svg>"#;
    let cmds = svg_to_commands(svg, 24.0);

    // Should produce MoveTo + some CubicTo commands.
    assert!(cmds.len() >= 2);
    assert!(matches!(cmds[0], PathCommand::MoveTo(..)));
    // Remaining should all be CubicTo (arc segments).
    for cmd in &cmds[1..] {
        assert!(
            matches!(cmd, PathCommand::CubicTo(..)),
            "arc should produce CubicTo, got {cmd:?}"
        );
    }
}
