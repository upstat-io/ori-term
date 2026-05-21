//! Kitty graphics protocol spec-conformance tests (§13).
//!
//! Each pilot drives kitty `_G` APC sequences through the `SpecHarness`
//! verification chain and asserts per-rung observations (placement in
//! `RenderableContent::images`, `Effect::Pty(ImageProtocolReply, …)` on
//! the effect sink). Catalog rows verified per test name the ID(s) they
//! cover from `plans/spec-conformance/catalog/kitty-graphics.md`.

mod fixtures;

pub mod actions;
pub mod animation;
pub mod chunked;
pub mod cross_stack_regression;
pub mod cross_stack_regression_gate;
pub mod formats;
pub mod placeholder_anchors;
pub mod replies;
pub mod transmissions;
pub mod virtual_placements;
