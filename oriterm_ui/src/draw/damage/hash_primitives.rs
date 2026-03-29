//! Per-primitive hashing for damage tracking.
//!
//! Iterates all typed arrays in a `Scene`, groups primitives by widget ID,
//! and produces a hash + bounds for each widget. Uses `f32::to_bits()` for
//! deterministic float hashing.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::hash::Hasher;

use crate::color::Color;
use crate::draw::ContentMask;
use crate::draw::{RectStyle, Scene};
use crate::geometry::{Point, Rect};
use crate::widget_id::WidgetId;

use super::WidgetFrameState;

/// Builds per-widget hash + bounds state from a scene.
pub(super) fn hash_scene_widget(scene: &Scene, out: &mut HashMap<WidgetId, WidgetFrameState>) {
    for q in scene.quads() {
        if let Some(id) = q.widget_id {
            let entry = get_or_init(out, id);
            hash_rect(&mut entry.hash, q.bounds);
            hash_rect_style(&mut entry.hash, &q.style);
            hash_content_mask(&mut entry.hash, q.content_mask);
            entry.bounds = entry.bounds.union(q.bounds);
        }
    }

    for t in scene.text_runs() {
        if let Some(id) = t.widget_id {
            let entry = get_or_init(out, id);
            hash_point(&mut entry.hash, t.position);
            hash_shaped_text(&mut entry.hash, &t.shaped);
            hash_color(&mut entry.hash, t.color);
            hash_opt_color(&mut entry.hash, t.bg_hint);
            hash_content_mask(&mut entry.hash, t.content_mask);
            let r = Rect::new(t.position.x, t.position.y, t.shaped.width, t.shaped.height);
            entry.bounds = entry.bounds.union(r);
        }
    }

    for l in scene.lines() {
        if let Some(id) = l.widget_id {
            let entry = get_or_init(out, id);
            hash_point(&mut entry.hash, l.from);
            hash_point(&mut entry.hash, l.to);
            hash_f32(&mut entry.hash, l.width);
            hash_color(&mut entry.hash, l.color);
            hash_content_mask(&mut entry.hash, l.content_mask);
            let r = super::line_bounds(l.from, l.to, l.width);
            entry.bounds = entry.bounds.union(r);
        }
    }

    for i in scene.icons() {
        if let Some(id) = i.widget_id {
            let entry = get_or_init(out, id);
            hash_rect(&mut entry.hash, i.rect);
            hash_u32(&mut entry.hash, i.atlas_page);
            hash_uv(&mut entry.hash, i.uv);
            hash_color(&mut entry.hash, i.color);
            hash_content_mask(&mut entry.hash, i.content_mask);
            entry.bounds = entry.bounds.union(i.rect);
        }
    }

    for i in scene.images() {
        if let Some(id) = i.widget_id {
            let entry = get_or_init(out, id);
            hash_rect(&mut entry.hash, i.rect);
            hash_u32(&mut entry.hash, i.texture_id);
            hash_uv(&mut entry.hash, i.uv);
            hash_content_mask(&mut entry.hash, i.content_mask);
            entry.bounds = entry.bounds.union(i.rect);
        }
    }
}

/// Gets or initializes a per-widget entry with zero hash and zero-size bounds.
fn get_or_init(
    map: &mut HashMap<WidgetId, WidgetFrameState>,
    id: WidgetId,
) -> &mut WidgetFrameState {
    match map.entry(id) {
        Entry::Occupied(e) => e.into_mut(),
        Entry::Vacant(e) => e.insert(WidgetFrameState {
            hash: 0,
            bounds: Rect::default(),
        }),
    }
}

// --- Hash helpers using FNV-style mixing ---

/// Mix a u64 value into the running hash using a simple FNV-1a-like step.
fn mix(hash: &mut u64, val: u64) {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hasher.write_u64(*hash);
    hasher.write_u64(val);
    *hash = hasher.finish();
}

fn hash_f32(hash: &mut u64, v: f32) {
    mix(hash, v.to_bits() as u64);
}

fn hash_u32(hash: &mut u64, v: u32) {
    mix(hash, v as u64);
}

fn hash_point(hash: &mut u64, p: Point) {
    hash_f32(hash, p.x);
    hash_f32(hash, p.y);
}

fn hash_rect(hash: &mut u64, r: Rect) {
    hash_f32(hash, r.origin.x);
    hash_f32(hash, r.origin.y);
    hash_f32(hash, r.size.width());
    hash_f32(hash, r.size.height());
}

fn hash_color(hash: &mut u64, c: Color) {
    hash_f32(hash, c.r);
    hash_f32(hash, c.g);
    hash_f32(hash, c.b);
    hash_f32(hash, c.a);
}

fn hash_opt_color(hash: &mut u64, c: Option<Color>) {
    match c {
        Some(c) => {
            mix(hash, 1);
            hash_color(hash, c);
        }
        None => mix(hash, 0),
    }
}

fn hash_uv(hash: &mut u64, uv: [f32; 4]) {
    for v in uv {
        hash_f32(hash, v);
    }
}

fn hash_content_mask(hash: &mut u64, cm: ContentMask) {
    hash_rect(hash, cm.clip);
    hash_f32(hash, cm.opacity);
}

fn hash_rect_style(hash: &mut u64, s: &RectStyle) {
    // Fill.
    hash_opt_color(hash, s.fill);

    // Border (hash all four sides via normalized accessors).
    for w in s.border.widths() {
        hash_f32(hash, w);
    }
    for c in s.border.colors() {
        hash_color(hash, c);
    }

    // Corner radii.
    for r in s.corner_radius {
        hash_f32(hash, r);
    }

    // Shadow.
    match &s.shadow {
        Some(sh) => {
            mix(hash, 1);
            hash_f32(hash, sh.offset_x);
            hash_f32(hash, sh.offset_y);
            hash_f32(hash, sh.blur_radius);
            hash_f32(hash, sh.spread);
            hash_color(hash, sh.color);
        }
        None => mix(hash, 0),
    }

    // Gradient.
    match &s.gradient {
        Some(g) => {
            mix(hash, 1);
            hash_f32(hash, g.angle);
            mix(hash, g.stops.len() as u64);
            for stop in &g.stops {
                hash_f32(hash, stop.position);
                hash_color(hash, stop.color);
            }
        }
        None => mix(hash, 0),
    }
}

fn hash_shaped_text(hash: &mut u64, shaped: &crate::text::ShapedText) {
    hash_f32(hash, shaped.width);
    hash_f32(hash, shaped.height);
    hash_f32(hash, shaped.baseline);
    hash_u32(hash, shaped.size_q6);
    mix(hash, shaped.weight as u64);
    mix(hash, shaped.glyphs.len() as u64);
    for g in &shaped.glyphs {
        mix(hash, g.glyph_id as u64);
        mix(hash, g.face_index as u64);
        mix(hash, g.synthetic as u64);
        hash_f32(hash, g.x_advance);
        hash_f32(hash, g.x_offset);
        hash_f32(hash, g.y_offset);
    }
}
