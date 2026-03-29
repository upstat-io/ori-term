//! SVG-to-`PathCommand` importer for generating icon definitions.
//!
//! Parses a subset of SVG sufficient for the mockup sidebar icons:
//! `<path>`, `<line>`, `<polyline>`, `<circle>`, and `<rect rx="...">`.
//! SVG path data commands M/L/H/V/C/S/A/Z (absolute and relative) are
//! supported. Arcs are converted to cubic Bézier segments using the
//! SVG spec Appendix F endpoint-to-center parameterization.
//!
//! All coordinates are normalized from the SVG viewBox to 0.0–1.0.
//!
//! Clippy notes: This module is a specialized SVG parser. String slicing by
//! byte position is safe because input is ASCII SVG markup.

mod arc;
mod path_data;

use std::fmt::Write;

use super::PathCommand;

/// Parse an SVG snippet and return normalized `PathCommand`s (0.0–1.0).
///
/// `viewbox_size` is the width/height of the square viewBox (e.g. 24.0).
#[expect(
    clippy::string_slice,
    reason = "SVG markup is ASCII — byte slicing is safe"
)]
pub fn svg_to_commands(svg: &str, viewbox_size: f32) -> Vec<PathCommand> {
    let mut cmds = Vec::new();

    // Parse SVG elements with a simple state machine.
    let mut pos = 0;
    let bytes = svg.as_bytes();

    while pos < bytes.len() {
        if bytes[pos] == b'<' && pos + 1 < bytes.len() && bytes[pos + 1] != b'/' {
            // Find the first '>' after this '<'.
            let end = match svg[pos..].find('>') {
                Some(i) => pos + i + 1,
                None => break,
            };
            let tag = &svg[pos..end];
            parse_element(tag, viewbox_size, &mut cmds);
            pos = end;
        } else {
            pos += 1;
        }
    }

    cmds
}

/// Parse a single SVG element tag.
fn parse_element(tag: &str, vb: f32, cmds: &mut Vec<PathCommand>) {
    let inner = tag
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim_end_matches('/')
        .trim();
    let tag_name = inner.split_whitespace().next().unwrap_or("");

    match tag_name {
        "path" => {
            if let Some(d) = attr(tag, "d") {
                path_data::parse_path_data(d, vb, cmds);
            }
        }
        "circle" => {
            let cx = attr_f32(tag, "cx").unwrap_or(0.0);
            let cy = attr_f32(tag, "cy").unwrap_or(0.0);
            let r = attr_f32(tag, "r").unwrap_or(0.0);
            circle_to_cubics(cx, cy, r, vb, cmds);
        }
        "rect" => {
            let x = attr_f32(tag, "x").unwrap_or(0.0);
            let y = attr_f32(tag, "y").unwrap_or(0.0);
            let w = attr_f32(tag, "width").unwrap_or(0.0);
            let h = attr_f32(tag, "height").unwrap_or(0.0);
            let rx = attr_f32(tag, "rx").unwrap_or(0.0);
            rect_to_commands(x, y, w, h, rx, vb, cmds);
        }
        "line" => {
            let x1 = attr_f32(tag, "x1").unwrap_or(0.0);
            let y1 = attr_f32(tag, "y1").unwrap_or(0.0);
            let x2 = attr_f32(tag, "x2").unwrap_or(0.0);
            let y2 = attr_f32(tag, "y2").unwrap_or(0.0);
            cmds.push(PathCommand::MoveTo(x1 / vb, y1 / vb));
            cmds.push(PathCommand::LineTo(x2 / vb, y2 / vb));
        }
        "polyline" => {
            if let Some(pts) = attr(tag, "points") {
                parse_polyline(pts, vb, cmds);
            }
        }
        _ => {}
    }
}

/// Extract an attribute value from an SVG tag string.
#[expect(clippy::string_slice, reason = "SVG markup is ASCII")]
fn attr<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("{name}=\"");
    let start = tag.find(&needle)? + needle.len();
    let end = start + tag[start..].find('"')?;
    Some(&tag[start..end])
}

/// Extract a float attribute value.
fn attr_f32(tag: &str, name: &str) -> Option<f32> {
    attr(tag, name)?.parse().ok()
}

/// Parse SVG polyline `points` attribute.
fn parse_polyline(pts: &str, vb: f32, cmds: &mut Vec<PathCommand>) {
    let nums: Vec<f32> = pts
        .split([' ', ','])
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();

    let mut first = true;
    for chunk in nums.chunks(2) {
        if chunk.len() == 2 {
            let (x, y) = (chunk[0] / vb, chunk[1] / vb);
            if first {
                cmds.push(PathCommand::MoveTo(x, y));
                first = false;
            } else {
                cmds.push(PathCommand::LineTo(x, y));
            }
        }
    }
}

/// Convert a circle to 4 cubic Bézier segments.
fn circle_to_cubics(cx: f32, cy: f32, r: f32, vb: f32, cmds: &mut Vec<PathCommand>) {
    // Magic number for circle approximation: 4/3 * tan(π/8).
    let k = r * 0.552_284_8;

    let n = |v: f32| v / vb; // Normalize.

    // Start at top of circle.
    cmds.push(PathCommand::MoveTo(n(cx), n(cy - r)));
    // Top to right.
    cmds.push(PathCommand::CubicTo(
        n(cx + k),
        n(cy - r),
        n(cx + r),
        n(cy - k),
        n(cx + r),
        n(cy),
    ));
    // Right to bottom.
    cmds.push(PathCommand::CubicTo(
        n(cx + r),
        n(cy + k),
        n(cx + k),
        n(cy + r),
        n(cx),
        n(cy + r),
    ));
    // Bottom to left.
    cmds.push(PathCommand::CubicTo(
        n(cx - k),
        n(cy + r),
        n(cx - r),
        n(cy + k),
        n(cx - r),
        n(cy),
    ));
    // Left to top.
    cmds.push(PathCommand::CubicTo(
        n(cx - r),
        n(cy - k),
        n(cx - k),
        n(cy - r),
        n(cx),
        n(cy - r),
    ));
    cmds.push(PathCommand::Close);
}

/// Convert a rect (optionally rounded) to path commands.
#[expect(
    clippy::too_many_arguments,
    reason = "SVG rect naturally has x, y, w, h, rx, vb"
)]
#[expect(
    clippy::many_single_char_names,
    reason = "x, y, w, h, k are standard geometry names"
)]
fn rect_to_commands(x: f32, y: f32, w: f32, h: f32, rx: f32, vb: f32, cmds: &mut Vec<PathCommand>) {
    let n = |v: f32| v / vb;

    if rx <= 0.0 {
        cmds.push(PathCommand::MoveTo(n(x), n(y)));
        cmds.push(PathCommand::LineTo(n(x + w), n(y)));
        cmds.push(PathCommand::LineTo(n(x + w), n(y + h)));
        cmds.push(PathCommand::LineTo(n(x), n(y + h)));
        cmds.push(PathCommand::Close);
        return;
    }

    let ry = rx; // Uniform corner radius.
    let k = 0.552_284_8_f32;
    let kx = rx * k;
    let ky = ry * k;

    // Start at top-left, just past the corner.
    cmds.push(PathCommand::MoveTo(n(x + rx), n(y)));
    // Top edge.
    cmds.push(PathCommand::LineTo(n(x + w - rx), n(y)));
    // Top-right corner.
    cmds.push(PathCommand::CubicTo(
        n(x + w - rx + kx),
        n(y),
        n(x + w),
        n(y + ry - ky),
        n(x + w),
        n(y + ry),
    ));
    // Right edge.
    cmds.push(PathCommand::LineTo(n(x + w), n(y + h - ry)));
    // Bottom-right corner.
    cmds.push(PathCommand::CubicTo(
        n(x + w),
        n(y + h - ry + ky),
        n(x + w - rx + kx),
        n(y + h),
        n(x + w - rx),
        n(y + h),
    ));
    // Bottom edge.
    cmds.push(PathCommand::LineTo(n(x + rx), n(y + h)));
    // Bottom-left corner.
    cmds.push(PathCommand::CubicTo(
        n(x + rx - kx),
        n(y + h),
        n(x),
        n(y + h - ry + ky),
        n(x),
        n(y + h - ry),
    ));
    // Left edge.
    cmds.push(PathCommand::LineTo(n(x), n(y + ry)));
    // Top-left corner.
    cmds.push(PathCommand::CubicTo(
        n(x),
        n(y + ry - ky),
        n(x + rx - kx),
        n(y),
        n(x + rx),
        n(y),
    ));
    cmds.push(PathCommand::Close);
}

/// Generate Rust source code for a `PathCommand` array from an SVG snippet.
///
/// Returns a string like `&[\n    PathCommand::MoveTo(0.25, 0.5),\n    ...\n]`.
pub fn commands_to_rust_source(cmds: &[PathCommand]) -> String {
    let mut s = String::from("&[\n");
    for cmd in cmds {
        match *cmd {
            PathCommand::MoveTo(x, y) => {
                let _ = writeln!(s, "    PathCommand::MoveTo({x:.6}, {y:.6}),");
            }
            PathCommand::LineTo(x, y) => {
                let _ = writeln!(s, "    PathCommand::LineTo({x:.6}, {y:.6}),");
            }
            PathCommand::CubicTo(cx1, cy1, cx2, cy2, x, y) => {
                let _ = writeln!(
                    s,
                    "    PathCommand::CubicTo({cx1:.6}, {cy1:.6}, {cx2:.6}, {cy2:.6}, {x:.6}, {y:.6}),",
                );
            }
            PathCommand::Close => {
                s.push_str("    PathCommand::Close,\n");
            }
        }
    }
    s.push(']');
    s
}
