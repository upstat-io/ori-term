//! Hit-test behavior for widget containers.
//!
//! Controls how a widget participates in hit testing relative to its
//! children. This is a leaf module with zero intra-crate imports, kept
//! separate to avoid circular dependencies.

/// Controls how a widget participates in hit testing relative to its children.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HitTestBehavior {
    /// Hit-test children first; self only if no child handles (default).
    #[default]
    DeferToChild,
    /// This widget absorbs the event — children behind it don't receive.
    Opaque,
    /// Both this widget and children behind it can receive the event.
    Translucent,
}
