//! UI framework types for oriterm windowing, layout, and rendering.
//!
//! Provides platform-agnostic geometry primitives, DPI scaling, hit testing,
//! and window management. Platform-specific glue lives in `#[cfg]`-gated
//! submodules.

pub use winit::window::CursorIcon;

pub mod action;
pub mod animation;
pub mod color;
pub mod compositor;
pub mod controllers;
pub mod draw;
pub mod focus;
pub mod geometry;
pub mod hit_test;
pub mod hit_test_behavior;
pub mod icons;
pub mod input;
pub mod interaction;
pub mod invalidation;
pub mod layout;
pub mod overlay;
pub mod pipeline;
pub mod scale;
pub mod sense;
pub mod surface;
pub mod text;
pub mod theme;
pub mod visual_state;
pub mod widget_id;
pub mod widgets;
pub mod window;
pub mod window_root;

#[cfg(any(test, feature = "testing"))]
pub mod testing;

#[cfg(target_os = "linux")]
pub mod platform_linux;
#[cfg(target_os = "macos")]
pub mod platform_macos;
#[cfg(target_os = "windows")]
pub mod platform_windows;
