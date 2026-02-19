//! DPI scale factor abstractions.
//!
//! [`ScaleFactor`] wraps the raw `f64` scale factor from the windowing
//! system as a clamped newtype. [`Scale`] provides type-safe conversion
//! between coordinate spaces (e.g. [`Logical`](crate::geometry::Logical)
//! to [`Physical`](crate::geometry::Physical)).

mod scale_factor;
mod typed_scale;

pub use scale_factor::ScaleFactor;
pub use typed_scale::Scale;

#[cfg(test)]
mod tests;
