//! Content provider for compositor layers.
//!
//! A [`LayerDelegate`] is called by the compositor to paint a layer's
//! content into a `DrawCtx` when the layer's `needs_paint` flag is set.
//! The draw context's bounds are the layer's own bounds (origin at 0,0).

use super::LayerId;
use crate::widgets::DrawCtx;

/// Provides content for a compositor layer.
///
/// Implementations render into a `DrawCtx` whose bounds match the
/// layer's local coordinate space. Called by the compositor during
/// the paint phase for layers with `needs_paint == true`.
///
/// Future consumers: overlay manager, tab bar widget, terminal grid,
/// search bar, context menu, settings panel, Quick Terminal panel,
/// expose mode thumbnails.
pub trait LayerDelegate {
    /// Paints this layer's content into the given draw context.
    ///
    /// The context's bounds are the layer's own bounds (origin at 0,0).
    /// Only called when the layer's `needs_paint` flag is set.
    fn paint_layer(&self, layer_id: LayerId, ctx: &mut DrawCtx<'_>);
}
