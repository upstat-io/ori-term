//! Tab bar widget — layout, colors, and hit testing for the tab strip.
//!
//! Provides pure-computation types for tab bar geometry:
//!
//! - [`constants`] — DPI-independent layout dimensions in logical pixels.
//! - [`TabBarLayout`] — compute tab widths and element positions from
//!   tab count and window width.
//! - [`TabBarColors`] — all colors needed for rendering, derived from
//!   [`UiTheme`](crate::theme::UiTheme).
//!
//! All coordinates are in logical pixels; the rendering layer applies the
//! DPI scale factor at the boundary. Follows the same pattern as
//! [`window_chrome`](crate::widgets::window_chrome): pure layout computation
//! separate from rendering and event handling.

pub mod colors;
pub mod constants;
pub mod layout;

pub use colors::TabBarColors;
pub use layout::TabBarLayout;

#[cfg(test)]
mod tests;
