//! Deterministic text serialization of [`Scene`] for snapshot testing.
//!
//! Converts a scene's primitives into a human-readable, diff-friendly format
//! suitable for golden tests via `insta`. Excludes non-deterministic fields
//! (`WidgetId`, `ContentMask`) and formats floats to 1 decimal place.

use std::fmt::Write;

use crate::color::Color;
use crate::draw::{BorderSides, LinePrimitive, Quad, Scene, TextRun};

/// Converts a scene to a deterministic, human-readable snapshot string.
///
/// Each primitive occupies one or more lines. Format:
/// ```text
/// Quad (x, y, w×h) fill=#rrggbb [radius=N] [border-T=W/#rrggbb]
/// Text (x, y) "source text" w×h #rrggbb
/// Line (x1, y1)→(x2, y2) w=N #rrggbb
/// Icon (x, y, w×h) #rrggbb page=N
/// Image (x, y, w×h) tex=N
/// ```
pub fn scene_to_snapshot(scene: &Scene) -> String {
    let mut out = String::with_capacity(512);
    let _ = writeln!(
        out,
        "Scene: {} quads, {} text, {} lines, {} icons, {} images",
        scene.quads().len(),
        scene.text_runs().len(),
        scene.lines().len(),
        scene.icons().len(),
        scene.images().len(),
    );

    for (i, q) in scene.quads().iter().enumerate() {
        let _ = write!(out, "  Q{i:02} ");
        format_quad(&mut out, q);
        out.push('\n');
    }

    for (i, t) in scene.text_runs().iter().enumerate() {
        let _ = write!(out, "  T{i:02} ");
        format_text_run(&mut out, t);
        out.push('\n');
    }

    for (i, l) in scene.lines().iter().enumerate() {
        let _ = write!(out, "  L{i:02} ");
        format_line(&mut out, l);
        out.push('\n');
    }

    for (i, icon) in scene.icons().iter().enumerate() {
        let _ = writeln!(
            out,
            "  I{i:02} ({:.1}, {:.1}, {:.1}\u{00d7}{:.1}) {} page={}",
            icon.rect.x(),
            icon.rect.y(),
            icon.rect.width(),
            icon.rect.height(),
            color_hex(&icon.color),
            icon.atlas_page,
        );
    }

    for (i, img) in scene.images().iter().enumerate() {
        let _ = writeln!(
            out,
            "  M{i:02} ({:.1}, {:.1}, {:.1}\u{00d7}{:.1}) tex={}",
            img.rect.x(),
            img.rect.y(),
            img.rect.width(),
            img.rect.height(),
            img.texture_id,
        );
    }

    out
}

/// Formats a quad primitive.
fn format_quad(out: &mut String, q: &Quad) {
    let _ = write!(
        out,
        "({:.1}, {:.1}, {:.1}\u{00d7}{:.1})",
        q.bounds.x(),
        q.bounds.y(),
        q.bounds.width(),
        q.bounds.height(),
    );

    if let Some(fill) = q.style.fill {
        let _ = write!(out, " fill={}", color_hex(&fill));
    }

    if let Some(ref gradient) = q.style.gradient {
        let _ = write!(out, " gradient={}", gradient.stops.len());
    }

    let r = &q.style.corner_radius;
    if r.iter().any(|&v| v > 0.0) {
        if r[0] == r[1] && r[1] == r[2] && r[2] == r[3] {
            let _ = write!(out, " radius={:.1}", r[0]);
        } else {
            let _ = write!(
                out,
                " radius=[{:.1},{:.1},{:.1},{:.1}]",
                r[0], r[1], r[2], r[3]
            );
        }
    }

    format_borders(out, &q.style.border);

    if q.style.shadow.is_some() {
        let _ = write!(out, " shadow");
    }
}

/// Formats a text run primitive.
fn format_text_run(out: &mut String, t: &TextRun) {
    let _ = write!(out, "({:.1}, {:.1})", t.position.x, t.position.y);

    if !t.shaped.source.is_empty() {
        let _ = write!(out, " {:?}", t.shaped.source);
    } else {
        let _ = write!(out, " [{}g]", t.shaped.glyph_count());
    }

    let _ = write!(
        out,
        " {:.1}\u{00d7}{:.1} {}",
        t.shaped.width,
        t.shaped.height,
        color_hex(&t.color),
    );

    if t.shaped.weight != 400 {
        let _ = write!(out, " w{}", t.shaped.weight);
    }
}

/// Formats a line primitive.
fn format_line(out: &mut String, l: &LinePrimitive) {
    let _ = write!(
        out,
        "({:.1}, {:.1})\u{2192}({:.1}, {:.1}) w={:.1} {}",
        l.from.x,
        l.from.y,
        l.to.x,
        l.to.y,
        l.width,
        color_hex(&l.color),
    );
}

/// Formats border sides (only non-empty sides).
fn format_borders(out: &mut String, borders: &BorderSides) {
    if borders.is_empty() {
        return;
    }
    if let Some(ref b) = borders.top {
        let _ = write!(out, " border-t={:.1}/{}", b.width, color_hex(&b.color));
    }
    if let Some(ref b) = borders.right {
        let _ = write!(out, " border-r={:.1}/{}", b.width, color_hex(&b.color));
    }
    if let Some(ref b) = borders.bottom {
        let _ = write!(out, " border-b={:.1}/{}", b.width, color_hex(&b.color));
    }
    if let Some(ref b) = borders.left {
        let _ = write!(out, " border-l={:.1}/{}", b.width, color_hex(&b.color));
    }
}

/// Converts a Color (f32 RGBA) to a hex string.
fn color_hex(c: &Color) -> String {
    let r = (c.r * 255.0).round() as u8;
    let g = (c.g * 255.0).round() as u8;
    let b = (c.b * 255.0).round() as u8;
    let a = (c.a * 255.0).round() as u8;
    if a == 255 {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else if a == 0 {
        "transparent".to_string()
    } else {
        format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
    }
}

#[cfg(test)]
mod tests;
