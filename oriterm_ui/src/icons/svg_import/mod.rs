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
                parse_path_data(d, vb, cmds);
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

// SVG path data parser

/// Parse SVG path data string (the `d` attribute).
#[expect(clippy::too_many_lines, reason = "SVG path command dispatch table")]
fn parse_path_data(d: &str, vb: f32, cmds: &mut Vec<PathCommand>) {
    let mut tokens = tokenize_path(d);
    let mut i = 0;
    let mut cx = 0.0_f32; // Current point.
    let mut cy = 0.0_f32;
    let mut sx = 0.0_f32; // Subpath start.
    let mut sy = 0.0_f32;
    let mut last_cp2_x = 0.0_f32; // Last cubic control point 2 (for S/s).
    let mut last_cp2_y = 0.0_f32;
    let mut last_cmd = b' ';

    while i < tokens.len() {
        let tok = &tokens[i];
        let cmd_byte = if tok.len() == 1 && tok.as_bytes()[0].is_ascii_alphabetic() {
            let b = tok.as_bytes()[0];
            i += 1;
            b
        } else {
            // Implicit repeat of last command (M becomes L after first pair).
            if last_cmd == b'M' {
                b'L'
            } else if last_cmd == b'm' {
                b'l'
            } else {
                last_cmd
            }
        };

        match cmd_byte {
            b'M' => {
                let (x, y) = (num(&tokens, &mut i), num(&tokens, &mut i));
                cmds.push(PathCommand::MoveTo(x / vb, y / vb));
                cx = x;
                cy = y;
                sx = x;
                sy = y;
                last_cp2_x = cx;
                last_cp2_y = cy;
            }
            b'm' => {
                let (dx, dy) = (num(&tokens, &mut i), num(&tokens, &mut i));
                cx += dx;
                cy += dy;
                sx = cx;
                sy = cy;
                cmds.push(PathCommand::MoveTo(cx / vb, cy / vb));
                last_cp2_x = cx;
                last_cp2_y = cy;
            }
            b'L' => {
                let (x, y) = (num(&tokens, &mut i), num(&tokens, &mut i));
                cmds.push(PathCommand::LineTo(x / vb, y / vb));
                cx = x;
                cy = y;
                last_cp2_x = cx;
                last_cp2_y = cy;
            }
            b'l' => {
                let (dx, dy) = (num(&tokens, &mut i), num(&tokens, &mut i));
                cx += dx;
                cy += dy;
                cmds.push(PathCommand::LineTo(cx / vb, cy / vb));
                last_cp2_x = cx;
                last_cp2_y = cy;
            }
            b'H' => {
                let x = num(&tokens, &mut i);
                cx = x;
                cmds.push(PathCommand::LineTo(cx / vb, cy / vb));
                last_cp2_x = cx;
                last_cp2_y = cy;
            }
            b'h' => {
                let dx = num(&tokens, &mut i);
                cx += dx;
                cmds.push(PathCommand::LineTo(cx / vb, cy / vb));
                last_cp2_x = cx;
                last_cp2_y = cy;
            }
            b'V' => {
                let y = num(&tokens, &mut i);
                cy = y;
                cmds.push(PathCommand::LineTo(cx / vb, cy / vb));
                last_cp2_x = cx;
                last_cp2_y = cy;
            }
            b'v' => {
                let dy = num(&tokens, &mut i);
                cy += dy;
                cmds.push(PathCommand::LineTo(cx / vb, cy / vb));
                last_cp2_x = cx;
                last_cp2_y = cy;
            }
            b'C' => {
                let (cx1, cy1) = (num(&tokens, &mut i), num(&tokens, &mut i));
                let (cx2, cy2) = (num(&tokens, &mut i), num(&tokens, &mut i));
                let (x, y) = (num(&tokens, &mut i), num(&tokens, &mut i));
                cmds.push(PathCommand::CubicTo(
                    cx1 / vb,
                    cy1 / vb,
                    cx2 / vb,
                    cy2 / vb,
                    x / vb,
                    y / vb,
                ));
                last_cp2_x = cx2;
                last_cp2_y = cy2;
                cx = x;
                cy = y;
            }
            b'c' => {
                let (dcx1, dcy1) = (num(&tokens, &mut i), num(&tokens, &mut i));
                let (dcx2, dcy2) = (num(&tokens, &mut i), num(&tokens, &mut i));
                let (dx, dy) = (num(&tokens, &mut i), num(&tokens, &mut i));
                let (cx1, cy1) = (cx + dcx1, cy + dcy1);
                let (cx2, cy2) = (cx + dcx2, cy + dcy2);
                let (x, y) = (cx + dx, cy + dy);
                cmds.push(PathCommand::CubicTo(
                    cx1 / vb,
                    cy1 / vb,
                    cx2 / vb,
                    cy2 / vb,
                    x / vb,
                    y / vb,
                ));
                last_cp2_x = cx2;
                last_cp2_y = cy2;
                cx = x;
                cy = y;
            }
            b'S' => {
                // Smooth cubic: first control point is reflection of previous cp2.
                let cp1_x = 2.0 * cx - last_cp2_x;
                let cp1_y = 2.0 * cy - last_cp2_y;
                let (cx2, cy2) = (num(&tokens, &mut i), num(&tokens, &mut i));
                let (x, y) = (num(&tokens, &mut i), num(&tokens, &mut i));
                cmds.push(PathCommand::CubicTo(
                    cp1_x / vb,
                    cp1_y / vb,
                    cx2 / vb,
                    cy2 / vb,
                    x / vb,
                    y / vb,
                ));
                last_cp2_x = cx2;
                last_cp2_y = cy2;
                cx = x;
                cy = y;
            }
            b's' => {
                let cp1_x = 2.0 * cx - last_cp2_x;
                let cp1_y = 2.0 * cy - last_cp2_y;
                let (dcx2, dcy2) = (num(&tokens, &mut i), num(&tokens, &mut i));
                let (dx, dy) = (num(&tokens, &mut i), num(&tokens, &mut i));
                let (cx2, cy2) = (cx + dcx2, cy + dcy2);
                let (x, y) = (cx + dx, cy + dy);
                cmds.push(PathCommand::CubicTo(
                    cp1_x / vb,
                    cp1_y / vb,
                    cx2 / vb,
                    cy2 / vb,
                    x / vb,
                    y / vb,
                ));
                last_cp2_x = cx2;
                last_cp2_y = cy2;
                cx = x;
                cy = y;
            }
            b'A' => {
                let rx = num(&tokens, &mut i);
                let ry = num(&tokens, &mut i);
                let rotation = num(&tokens, &mut i);
                let large_arc = arc_flag(&mut tokens, &mut i);
                let sweep = arc_flag(&mut tokens, &mut i);
                let x = num(&tokens, &mut i);
                let y = num(&tokens, &mut i);
                arc::arc_to_cubics(cx, cy, rx, ry, rotation, large_arc, sweep, x, y, vb, cmds);
                cx = x;
                cy = y;
                last_cp2_x = cx;
                last_cp2_y = cy;
            }
            b'a' => {
                let rx = num(&tokens, &mut i);
                let ry = num(&tokens, &mut i);
                let rotation = num(&tokens, &mut i);
                let large_arc = arc_flag(&mut tokens, &mut i);
                let sweep = arc_flag(&mut tokens, &mut i);
                let dx = num(&tokens, &mut i);
                let dy = num(&tokens, &mut i);
                let x = cx + dx;
                let y = cy + dy;
                arc::arc_to_cubics(cx, cy, rx, ry, rotation, large_arc, sweep, x, y, vb, cmds);
                cx = x;
                cy = y;
                last_cp2_x = cx;
                last_cp2_y = cy;
            }
            b'Z' | b'z' => {
                cmds.push(PathCommand::Close);
                cx = sx;
                cy = sy;
                last_cp2_x = cx;
                last_cp2_y = cy;
            }
            _ => {}
        }
        last_cmd = cmd_byte;
    }
}

/// Tokenize SVG path data into command letters and number strings.
#[expect(clippy::string_slice, reason = "SVG path data is ASCII")]
fn tokenize_path(d: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut i = 0;
    let bytes = d.as_bytes();

    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_alphabetic() {
            tokens.push(String::from(b as char));
            i += 1;
        } else if b == b'-' || b == b'.' || b.is_ascii_digit() {
            let start = i;
            if b == b'-' {
                i += 1;
            }
            // Integer part.
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            // Decimal part.
            if i < bytes.len() && bytes[i] == b'.' {
                i += 1;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            // Exponent (rare but valid).
            if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
                i += 1;
                if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
                    i += 1;
                }
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            tokens.push(d[start..i].to_string());
        } else {
            // Skip whitespace and commas.
            i += 1;
        }
    }

    tokens
}

/// Read next number from token stream.
fn num(tokens: &[String], i: &mut usize) -> f32 {
    if *i < tokens.len() {
        let val = tokens[*i].parse::<f32>().unwrap_or(0.0);
        *i += 1;
        val
    } else {
        0.0
    }
}

/// Read an SVG arc flag (single digit 0 or 1) from the token stream.
///
/// SVG arc flags can be concatenated without separators (e.g. `006` is
/// flag=0, flag=0, coordinate=6). This function handles splitting.
#[expect(
    clippy::ptr_arg,
    reason = "must mutate Vec elements in-place when splitting tokens"
)]
#[expect(clippy::string_slice, reason = "SVG arc flag tokens are ASCII digits")]
fn arc_flag(tokens: &mut Vec<String>, i: &mut usize) -> bool {
    if *i >= tokens.len() {
        return false;
    }
    let tok = &tokens[*i];
    if tok == "0" || tok == "1" {
        let val = tok == "1";
        *i += 1;
        return val;
    }
    // Token may start with a flag digit concatenated with subsequent data.
    // Split: first char is the flag, rest becomes a new token.
    let first = tok.as_bytes()[0];
    if first == b'0' || first == b'1' {
        let val = first == b'1';
        let rest = tok[1..].to_string();
        tokens[*i] = rest;
        // Don't advance i — the remaining token still needs to be consumed.
        return val;
    }
    // Fallback: interpret as number and check if 0 or 1.
    let val = tokens[*i].parse::<f32>().unwrap_or(0.0) != 0.0;
    *i += 1;
    val
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
