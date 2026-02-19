//! Character-to-glyph resolution: style fallback, synthetic flags, emoji.

use super::face::glyph_id;
use super::{FaceIdx, FontCollection, GlyphStyle, ResolvedGlyph, SyntheticFlags};

impl FontCollection {
    // ── Resolution ──

    /// Resolve a character to a font face and glyph ID.
    ///
    /// Tries the requested style, falls back through style substitution
    /// (with appropriate synthetic flags), then tries fallback fonts,
    /// and finally returns .notdef from Regular.
    pub fn resolve(&self, ch: char, style: GlyphStyle) -> ResolvedGlyph {
        let idx = style as usize;

        // 1. Try requested style.
        if let Some(ref fd) = self.primary[idx] {
            let gid = glyph_id(fd, ch);
            if gid != 0 {
                return ResolvedGlyph {
                    glyph_id: gid,
                    face_idx: FaceIdx(idx as u16),
                    synthetic: SyntheticFlags::NONE,
                };
            }
        }

        // 2. Style substitution with synthetic flags.
        if style != GlyphStyle::Regular {
            let synthetic = match style {
                GlyphStyle::Bold => self.try_regular_with(ch, SyntheticFlags::BOLD),
                GlyphStyle::Italic => self.try_regular_with(ch, SyntheticFlags::ITALIC),
                GlyphStyle::BoldItalic => self.resolve_bold_italic_fallback(ch),
                GlyphStyle::Regular => unreachable!(),
            };
            if let Some(resolved) = synthetic {
                return resolved;
            }
        }

        // 3. Try fallback fonts.
        for (i, fb) in self.fallbacks.iter().enumerate() {
            let gid = glyph_id(fb, ch);
            if gid != 0 {
                return ResolvedGlyph {
                    glyph_id: gid,
                    face_idx: FaceIdx((4 + i) as u16),
                    synthetic: SyntheticFlags::NONE,
                };
            }
        }

        // 4. Ultimate fallback: .notdef from Regular.
        let gid = self.primary[0].as_ref().map_or(0, |fd| glyph_id(fd, ch));
        ResolvedGlyph {
            glyph_id: gid,
            face_idx: FaceIdx::REGULAR,
            synthetic: SyntheticFlags::NONE,
        }
    }

    /// Resolve preferring fallback fonts for emoji presentation (VS16).
    ///
    /// When VS16 (U+FE0F) forces emoji presentation, tries fallback fonts
    /// first because color emoji fonts (Segoe UI Emoji, Noto Color Emoji)
    /// are typically in the fallback chain, not the primary terminal font.
    ///
    /// Falls back to normal [`resolve`] if no fallback covers the character.
    pub fn resolve_prefer_emoji(&self, ch: char, style: GlyphStyle) -> ResolvedGlyph {
        // Try fallback fonts first (color emoji fonts are typically here).
        for (i, fb) in self.fallbacks.iter().enumerate() {
            let gid = glyph_id(fb, ch);
            if gid != 0 {
                return ResolvedGlyph {
                    glyph_id: gid,
                    face_idx: FaceIdx((4 + i) as u16),
                    synthetic: SyntheticFlags::NONE,
                };
            }
        }
        // No fallback covers it — use normal resolution.
        self.resolve(ch, style)
    }

    // ── Private helpers ──

    /// Try Regular face with the given synthetic flags.
    fn try_regular_with(&self, ch: char, flags: SyntheticFlags) -> Option<ResolvedGlyph> {
        let fd = self.primary[0].as_ref()?;
        let gid = glyph_id(fd, ch);
        if gid != 0 {
            Some(ResolvedGlyph {
                glyph_id: gid,
                face_idx: FaceIdx::REGULAR,
                synthetic: flags,
            })
        } else {
            None
        }
    }

    /// Try bold → italic → regular for `BoldItalic` style substitution.
    fn resolve_bold_italic_fallback(&self, ch: char) -> Option<ResolvedGlyph> {
        // Try bold face with synthetic italic.
        if let Some(ref fd) = self.primary[GlyphStyle::Bold as usize] {
            let gid = glyph_id(fd, ch);
            if gid != 0 {
                return Some(ResolvedGlyph {
                    glyph_id: gid,
                    face_idx: FaceIdx(GlyphStyle::Bold as u16),
                    synthetic: SyntheticFlags::ITALIC,
                });
            }
        }
        // Try italic face with synthetic bold.
        if let Some(ref fd) = self.primary[GlyphStyle::Italic as usize] {
            let gid = glyph_id(fd, ch);
            if gid != 0 {
                return Some(ResolvedGlyph {
                    glyph_id: gid,
                    face_idx: FaceIdx(GlyphStyle::Italic as u16),
                    synthetic: SyntheticFlags::BOLD,
                });
            }
        }
        // Try regular with both flags.
        self.try_regular_with(ch, SyntheticFlags::BOLD | SyntheticFlags::ITALIC)
    }
}
