//! Compositor layer system for GPU-backed composition.
//!
//! Provides layer primitives, a layer tree with parent-child hierarchy,
//! property animation, and GPU composition. Each layer renders to its own
//! texture; a composition pass blends layers with per-layer opacity and
//! transforms.

pub mod delegate;
pub mod layer;
pub mod layer_animator;
pub mod layer_tree;
pub mod transform;

pub use delegate::LayerDelegate;
pub use layer::{Layer, LayerId, LayerProperties, LayerType};
pub use layer_animator::{AnimationParams, LayerAnimator, PreemptionStrategy};
pub use layer_tree::LayerTree;
pub use transform::Transform2D;

#[cfg(test)]
mod tests;
