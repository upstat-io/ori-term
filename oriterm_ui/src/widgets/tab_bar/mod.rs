//! Tab bar widget — layout, colors, hit testing, and rendering for the tab strip.
//!
//! - [`constants`] — DPI-independent layout dimensions in logical pixels.
//! - [`TabBarLayout`] — compute tab widths and element positions from
//!   tab count and window width.
//! - [`TabBarColors`] — all colors needed for rendering, derived from
//!   [`UiTheme`](crate::theme::UiTheme).
//! - [`TabBarHit`] — hit-test result identifying which tab bar element the
//!   cursor targets.
//! - [`TabBarWidget`] — rendering widget that draws tabs, buttons, and
//!   separators into a [`DrawList`](crate::draw::DrawList).
//!
//! All coordinates are in logical pixels; the rendering layer applies the
//! DPI scale factor at the boundary. Follows the same pattern as
//! [`window_chrome`](crate::widgets::window_chrome): pure layout computation
//! separate from rendering and event handling.

pub mod colors;
pub mod constants;
pub mod emoji;
pub mod hit;
pub mod layout;
pub mod slide;
pub mod widget;

pub use colors::TabBarColors;
pub use emoji::extract_emoji_icon;
pub use hit::{TabBarHit, hit_test};
pub use layout::TabBarLayout;
pub use slide::TabSlideState;
pub use widget::{TabBarWidget, TabEntry, TabIcon};

#[cfg(test)]
mod tests;
