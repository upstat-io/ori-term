//! SVG path data (`d` attribute) parser.
//!
//! Parses SVG path commands M/L/H/V/C/S/A/Z (absolute and relative)
//! into normalized `PathCommand` sequences. Arc commands are delegated
//! to [`super::arc::arc_to_cubics`].

use super::PathCommand;
use super::arc;

/// Parse SVG path data string (the `d` attribute).
#[expect(clippy::too_many_lines, reason = "SVG path command dispatch table")]
pub(super) fn parse_path_data(d: &str, vb: f32, cmds: &mut Vec<PathCommand>) {
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
