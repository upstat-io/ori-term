//! Rustybuzz face creation for text shaping.
//!
//! Creates `rustybuzz::Face` objects from the loaded font data. Provides both
//! a convenience allocating API ([`create_shaping_faces`]) and a zero-alloc
//! hot-path API ([`fill_shaping_faces`]) that reuses a caller-provided buffer.

use super::metadata::{FaceVariationResult, face_variations, face_variations_for_ui_weight};
use super::{FaceIdx, FontCollection, SyntheticFlags};

/// Apply computed variation settings to a rustybuzz face.
fn apply_variations(face: &mut rustybuzz::Face<'_>, vars: &FaceVariationResult) {
    if vars.settings.is_empty() {
        return;
    }
    let mut rb_vars = [rustybuzz::Variation {
        tag: rustybuzz::ttf_parser::Tag(0),
        value: 0.0,
    }; 2];
    for (j, (tag, val)) in vars.settings.iter().enumerate() {
        rb_vars[j] = rustybuzz::Variation {
            tag: rustybuzz::ttf_parser::Tag::from_bytes(
                tag.as_bytes().first_chunk::<4>().expect("4-byte tag"),
            ),
            value: *val,
        };
    }
    face.set_variations(&rb_vars[..vars.settings.len()]);
}

impl FontCollection {
    /// Create rustybuzz `Face` objects using the collection-global weight.
    ///
    /// Convenience allocating API for tests. Production grid shaping uses
    /// [`fill_shaping_faces`] (zero-alloc); UI text shaping uses
    /// [`create_shaping_faces_for_weight`] (explicit weight).
    #[cfg(test)]
    pub fn create_shaping_faces(&self) -> Vec<Option<rustybuzz::Face<'_>>> {
        let mut faces = Vec::with_capacity(self.face_count());
        self.push_faces_into(&mut faces);
        faces
    }

    /// Create rustybuzz `Face` objects with a specific UI weight.
    ///
    /// Unlike [`create_shaping_faces`] (which uses the collection-global weight),
    /// this applies the exact `requested_weight` to the `wght` axis via
    /// [`face_variations_for_ui_weight`]. This ensures shaping metrics match
    /// the weight used for atlas rasterization.
    pub fn create_shaping_faces_for_weight(
        &self,
        requested_weight: u16,
        synthetic: SyntheticFlags,
    ) -> Vec<Option<rustybuzz::Face<'_>>> {
        let mut faces = Vec::with_capacity(self.face_count());
        for slot in &self.primary {
            faces.push(slot.as_ref().and_then(|fd| {
                let mut face = rustybuzz::Face::from_slice(&fd.bytes, fd.face_index)?;
                let vars = face_variations_for_ui_weight(synthetic, requested_weight, &fd.axes);
                apply_variations(&mut face, &vars);
                Some(face)
            }));
        }
        for fb in &self.fallbacks {
            faces.push(
                rustybuzz::Face::from_slice(&fb.bytes, fb.face_index).map(|mut face| {
                    let vars = face_variations_for_ui_weight(synthetic, requested_weight, &fb.axes);
                    apply_variations(&mut face, &vars);
                    face
                }),
            );
        }
        faces
    }

    /// Fill a reusable buffer with rustybuzz `Face` objects for all loaded faces.
    ///
    /// The buffer is stored with `'static` lifetime for cross-frame reuse on
    /// `ShapingScratch`. Clears and refills each call. The transmute is sound
    /// because the buffer is cleared before every fill and only accessed while
    /// `&self` is borrowed.
    #[expect(
        unsafe_code,
        reason = "lifetime transmute for cross-frame Face buffer reuse — see safety comment"
    )]
    pub fn fill_shaping_faces(&self, out: &mut Vec<Option<rustybuzz::Face<'static>>>) {
        out.clear();
        out.reserve(self.face_count().saturating_sub(out.capacity()));
        // SAFETY: `Face<'_>` borrows from `FaceData.bytes` owned by `self`.
        // The buffer is cleared above and only accessed while `&self` is
        // held, so the `'static` lifetime cannot outlive the actual data.
        let typed: &mut Vec<Option<rustybuzz::Face<'_>>> =
            unsafe { &mut *std::ptr::from_mut(out).cast() };
        self.push_faces_into(typed);
    }

    /// Total face slot count (4 primary + fallbacks).
    fn face_count(&self) -> usize {
        4 + self.fallbacks.len()
    }

    /// Push all face objects into a pre-cleared Vec using collection-global weight.
    fn push_faces_into<'a>(&'a self, out: &mut Vec<Option<rustybuzz::Face<'a>>>) {
        for (i, slot) in self.primary.iter().enumerate() {
            out.push(slot.as_ref().and_then(|fd| {
                let mut face = rustybuzz::Face::from_slice(&fd.bytes, fd.face_index)?;
                let vars = face_variations(
                    FaceIdx(i as u16),
                    SyntheticFlags::NONE,
                    self.weight,
                    &fd.axes,
                );
                apply_variations(&mut face, &vars);
                Some(face)
            }));
        }
        for fb in &self.fallbacks {
            out.push(rustybuzz::Face::from_slice(&fb.bytes, fb.face_index));
        }
    }
}
