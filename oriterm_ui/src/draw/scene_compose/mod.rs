//! Scene composition — retained rendering via per-widget draw caching.
//!
//! [`compose_scene`] invalidates dirty scene nodes, then draws the widget
//! tree with caching enabled. Container widgets check the scene cache to
//! skip unchanged children and replay their cached draw commands.

use crate::draw::SceneCache;
use crate::invalidation::InvalidationTracker;
use crate::widgets::{DrawCtx, Widget};

/// Draw a widget tree with scene caching.
///
/// Marks scene nodes as invalid for widgets that the [`InvalidationTracker`]
/// reports as dirty, then draws `root` with `ctx.scene_cache` enabled so
/// container widgets can skip unchanged children.
///
/// Call this instead of `root.draw(ctx)` to enable retained rendering.
pub fn compose_scene<'a>(
    root: &dyn Widget,
    ctx: &mut DrawCtx<'a>,
    tracker: &InvalidationTracker,
    cache: &'a mut SceneCache,
) {
    cache.reset_log();
    invalidate_dirty_nodes(cache, tracker);

    let prev_cache = ctx.scene_cache.take();
    ctx.scene_cache = Some(cache);
    root.draw(ctx);
    ctx.scene_cache = prev_cache;
}

/// Invalidate scene nodes for widgets the tracker reports as dirty.
///
/// Propagates invalidation upward: if a descendant widget is dirty, any
/// ancestor `SceneNode` whose cached output includes that descendant is also
/// invalidated. This ensures that replaying an ancestor's cache never
/// produces stale output from a changed descendant.
fn invalidate_dirty_nodes(cache: &mut SceneCache, tracker: &InvalidationTracker) {
    if tracker.needs_full_rebuild() {
        for node in cache.values_mut() {
            node.invalidate();
        }
        return;
    }

    // Two-pass invalidation:
    // 1. Invalidate directly dirty widgets.
    // 2. Invalidate any node whose `contained` list includes a dirty widget.
    //
    // Pass 2 handles ancestor containers whose cached subtree output embeds
    // a dirty descendant. Without this, the ancestor would replay stale
    // commands from the previous frame.
    for node in cache.values_mut() {
        let wid = node.widget_id();

        // Direct hit.
        if tracker.is_paint_dirty(wid) || tracker.is_layout_dirty(wid) {
            node.invalidate();
            continue;
        }

        // Containment propagation: any descendant dirty?
        let stale = node
            .contained()
            .iter()
            .any(|id| tracker.is_paint_dirty(*id) || tracker.is_layout_dirty(*id));
        if stale {
            node.invalidate();
        }
    }
}

#[cfg(test)]
mod tests;
