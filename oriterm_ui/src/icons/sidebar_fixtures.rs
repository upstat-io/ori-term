//! Source SVG fixtures for the 8 settings sidebar icons.
//!
//! These are the authoritative SVG snippets extracted from
//! `mockups/settings-brutal.html` (lines 1541–1572). They serve as the
//! source of truth for generating `PathCommand` definitions and for
//! raster-fidelity comparison tests.
//!
//! All icons use a 24×24 viewBox with `stroke-width="2"` and
//! `stroke-linecap="round"` / `stroke-linejoin="round"`.

use super::IconId;

/// A sidebar icon's authoritative SVG source from the mockup.
pub struct SidebarIconSource {
    /// Which icon this fixture represents.
    pub id: IconId,
    /// Target logical size in the UI (pixels).
    pub logical_size: u32,
    /// `viewBox` size of the source SVG (both width and height, assumed square).
    pub viewbox_size: f32,
    /// Stroke width in source SVG coordinate space.
    pub source_stroke_width: f32,
    /// Exact SVG markup from the mockup (24×24 viewBox, stroke-width 2).
    pub svg: &'static str,
}

impl SidebarIconSource {
    /// Compute the runtime stroke width for a given target size.
    ///
    /// Scales the source stroke proportionally: `source_stroke × target / viewbox`.
    pub const fn scaled_stroke(&self, target_size: f32) -> f32 {
        self.source_stroke_width * target_size / self.viewbox_size
    }
}

/// The 8 settings sidebar icon SVG fixtures, in sidebar display order.
pub static SIDEBAR_ICON_SOURCES: [SidebarIconSource; 8] = [
    SidebarIconSource {
        id: IconId::Sun,
        logical_size: 16,
        viewbox_size: 24.0,
        source_stroke_width: 2.0,
        svg: r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="5"/><path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42"/></svg>"#,
    },
    SidebarIconSource {
        id: IconId::Palette,
        logical_size: 16,
        viewbox_size: 24.0,
        source_stroke_width: 2.0,
        svg: r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10c1.1 0 2-.9 2-2 0-.5-.2-1-.5-1.3-.3-.4-.5-.8-.5-1.3 0-1.1.9-2 2-2h2.4c3.1 0 5.6-2.5 5.6-5.6C23 5.8 18.1 2 12 2z"/><circle cx="6.5" cy="11.5" r="1.5"/><circle cx="9.5" cy="7.5" r="1.5"/><circle cx="14.5" cy="7.5" r="1.5"/><circle cx="17.5" cy="11.5" r="1.5"/></svg>"#,
    },
    SidebarIconSource {
        id: IconId::Type,
        logical_size: 16,
        viewbox_size: 24.0,
        source_stroke_width: 2.0,
        svg: r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 7V4h16v3M9 20h6M12 4v16"/></svg>"#,
    },
    SidebarIconSource {
        id: IconId::Terminal,
        logical_size: 16,
        viewbox_size: 24.0,
        source_stroke_width: 2.0,
        svg: r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/></svg>"#,
    },
    SidebarIconSource {
        id: IconId::Keyboard,
        logical_size: 16,
        viewbox_size: 24.0,
        source_stroke_width: 2.0,
        svg: r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="4" width="20" height="16" rx="2"/><path d="M6 8h.01M10 8h.01M14 8h.01M18 8h.01M8 12h.01M12 12h.01M16 12h.01M8 16h8"/></svg>"#,
    },
    SidebarIconSource {
        id: IconId::Window,
        logical_size: 16,
        viewbox_size: 24.0,
        source_stroke_width: 2.0,
        svg: r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="18" height="18" rx="2"/><path d="M3 9h18"/></svg>"#,
    },
    SidebarIconSource {
        id: IconId::Bell,
        logical_size: 16,
        viewbox_size: 24.0,
        source_stroke_width: 2.0,
        svg: r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 8A6 6 0 006 8c0 7-3 9-3 9h18s-3-2-3-9"/><path d="M13.73 21a2 2 0 01-3.46 0"/></svg>"#,
    },
    SidebarIconSource {
        id: IconId::Activity,
        logical_size: 16,
        viewbox_size: 24.0,
        source_stroke_width: 2.0,
        svg: r#"<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 12h-4l-3 9L9 3l-3 9H2"/></svg>"#,
    },
];
