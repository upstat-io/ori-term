//! Per-pane `PreparedFrame` caching for multi-pane rendering.
//!
//! Avoids re-preparing unchanged panes on every frame. Only dirty panes
//! (new PTY output or layout changes) go through the full extract→shape→fill
//! pipeline; clean panes reuse their cached GPU instances.

use std::collections::HashMap;

use oriterm_mux::id::PaneId;

use crate::session::PaneLayout;

use super::prepared_frame::PreparedFrame;

/// Cached GPU-ready instances for a single pane.
struct CachedPaneFrame {
    /// GPU instances from the last prepare pass.
    prepared: PreparedFrame,
    /// Layout at time of preparation (for invalidation on resize/move).
    layout: PaneLayout,
    /// Damage key from the last prepare pass — composite of
    /// `compute_dispatch_fingerprint` + per-pane row-state.
    /// SSOT for "did any prepare-relevant input change since this entry
    /// was built?" Set by [`PaneRenderCache::get_or_prepare`] on miss;
    /// compared on every call.
    damage_key: u64,
}

/// Cache lookup key for [`PaneRenderCache::get_or_prepare`]: pane identity,
/// layout, the content-dirty gate, and the composite damage key.
#[derive(Clone, Copy)]
pub(crate) struct PaneCacheKey<'a> {
    pub pane_id: PaneId,
    pub layout: &'a PaneLayout,
    pub dirty: bool,
    pub damage_key: u64,
}

/// Per-pane render cache.
///
/// Stores one [`PreparedFrame`] per pane. On each frame, callers check
/// [`get_or_prepare`](Self::get_or_prepare) — if the pane is clean and
/// its layout unchanged, the cached frame is returned without re-preparing.
pub(crate) struct PaneRenderCache {
    entries: HashMap<PaneId, CachedPaneFrame>,
}

impl PaneRenderCache {
    /// Create an empty cache.
    pub(crate) fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Get a cached frame or prepare a new one.
    ///
    /// Returns the cached `PreparedFrame` when all three hold:
    /// - `dirty` is false (snapshot content unchanged),
    /// - `layout` matches the cached entry,
    /// - `damage_key` matches the cached entry's stored key.
    ///
    /// `damage_key` is the multi-pane SSOT for "did any prepare-relevant
    /// input change?" — composed via `compute_pane_damage_key`. The `dirty`
    /// gate handles grid-cell content changes (which are NOT in
    /// `compute_dispatch_fingerprint` per its docs); the `damage_key` handles
    /// every other input including the previously hand-rolled
    /// `is_focused || blink_opacity_changed` triggers.
    ///
    /// On miss, calls `prepare_fn` (which receives a cleared `PreparedFrame`
    /// for in-place fill) and stores the new `(layout, damage_key)` pair.
    pub(crate) fn get_or_prepare(
        &mut self,
        key: PaneCacheKey<'_>,
        prepare_fn: impl FnOnce(&mut PreparedFrame),
    ) -> &PreparedFrame {
        let PaneCacheKey {
            pane_id,
            layout,
            dirty,
            damage_key,
        } = key;
        let entry = self.entries.entry(pane_id);

        match entry {
            std::collections::hash_map::Entry::Occupied(mut occ) => {
                let cached = occ.get_mut();
                if !dirty && cached.layout == *layout && cached.damage_key == damage_key {
                    // Cache hit — reuse existing instances.
                    return &occ.into_mut().prepared;
                }
                // Cache miss — re-prepare in place.
                cached.prepared.clear();
                prepare_fn(&mut cached.prepared);
                cached.layout = *layout;
                cached.damage_key = damage_key;
                &occ.into_mut().prepared
            }
            std::collections::hash_map::Entry::Vacant(vac) => {
                // Placeholder viewport/color — prepare_fn fills the actual content.
                let mut prepared = PreparedFrame::new(
                    super::frame_input::ViewportSize::new(0, 0),
                    oriterm_core::Rgb { r: 0, g: 0, b: 0 },
                    1.0,
                );
                prepare_fn(&mut prepared);
                let cached = vac.insert(CachedPaneFrame {
                    prepared,
                    layout: *layout,
                    damage_key,
                });
                &cached.prepared
            }
        }
    }

    /// Check whether a valid cache entry exists for this pane at the given
    /// layout and `damage_key`. The full cache-hit predicate also requires
    /// `!dirty`; this method is used by callers that want to short-circuit
    /// extract work when the cache will definitely hit.
    pub(crate) fn is_cached(&self, pane_id: PaneId, layout: &PaneLayout, damage_key: u64) -> bool {
        self.entries
            .get(&pane_id)
            .is_some_and(|e| e.layout == *layout && e.damage_key == damage_key)
    }

    /// Read-only access to a cached pane frame.
    ///
    /// Returns `None` if no entry exists. Does not check layout staleness —
    /// call [`is_cached`](Self::is_cached) first if layout validation is needed.
    pub(crate) fn get_cached(&self, pane_id: PaneId) -> Option<&PreparedFrame> {
        self.entries.get(&pane_id).map(|e| &e.prepared)
    }

    /// Force a specific pane to re-prepare on the next frame.
    #[allow(
        dead_code,
        reason = "used for targeted invalidation (e.g. palette change per pane)"
    )]
    pub(crate) fn invalidate(&mut self, pane_id: PaneId) {
        self.entries.remove(&pane_id);
    }

    /// Remove a closed pane's cached frame, freeing memory.
    pub(crate) fn remove(&mut self, pane_id: PaneId) {
        self.entries.remove(&pane_id);
    }

    /// Invalidate all cached panes (e.g. atlas rebuild, font change).
    ///
    /// **5.16.2 recovery discipline:** during `GpuHealth::Recovering`, callers
    /// must NOT loop this on every mux notification. The 5.16.3 teardown
    /// drops the entire cache wholesale, so any per-notification invalidation
    /// is wasted CPU. Callers in hot paths should consult
    /// `gpu_health.is_healthy()` before invalidating; rare user-action sites
    /// (theme change, font reload, tab switch) need no gate because they are
    /// not reachable during recovery in practice — the gate at
    /// `App::render_dirty_windows` and `compute_control_flow` ensures the
    /// event loop sleeps through recovery.
    pub(crate) fn invalidate_all(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests;
