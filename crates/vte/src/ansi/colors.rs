//! Color types and XParseColor parsing.

extern crate alloc;

use alloc::string::String;
use core::fmt::{self, Display, Formatter};
#[cfg(feature = "std")]
use core::ops::Mul;
use core::ops::{Add, Sub};
use core::str;
use core::str::FromStr;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Hyperlink reference with optional id.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Hyperlink {
    /// Identifier for the given hyperlink.
    pub id: Option<String>,
    /// Resource identifier of the hyperlink.
    pub uri: String,
}

/// RGB color triplet.
#[derive(Debug, Eq, PartialEq, Copy, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    /// Implementation of [W3C's luminance algorithm].
    ///
    /// [W3C's luminance algorithm]: https://www.w3.org/TR/WCAG20/#relativeluminancedef
    #[cfg(feature = "std")]
    pub fn luminance(self) -> f64 {
        let channel_luminance = |channel| {
            let channel = channel as f64 / 255.;
            if channel <= 0.03928 {
                channel / 12.92
            } else {
                f64::powf((channel + 0.055) / 1.055, 2.4)
            }
        };

        let r_luminance = channel_luminance(self.r);
        let g_luminance = channel_luminance(self.g);
        let b_luminance = channel_luminance(self.b);

        0.2126 * r_luminance + 0.7152 * g_luminance + 0.0722 * b_luminance
    }

    /// Implementation of [W3C's contrast algorithm].
    ///
    /// [W3C's contrast algorithm]: https://www.w3.org/TR/WCAG20/#contrast-ratiodef
    #[cfg(feature = "std")]
    pub fn contrast(self, other: Rgb) -> f64 {
        let self_luminance = self.luminance();
        let other_luminance = other.luminance();

        let (darker, lighter) = if self_luminance > other_luminance {
            (other_luminance, self_luminance)
        } else {
            (self_luminance, other_luminance)
        };

        (lighter + 0.05) / (darker + 0.05)
    }
}

// A multiply function for Rgb, as the default dim is just *2/3.
#[cfg(feature = "std")]
impl Mul<f32> for Rgb {
    type Output = Rgb;

    fn mul(self, rhs: f32) -> Rgb {
        let result = Rgb {
            r: (f32::from(self.r) * rhs).clamp(0.0, 255.0) as u8,
            g: (f32::from(self.g) * rhs).clamp(0.0, 255.0) as u8,
            b: (f32::from(self.b) * rhs).clamp(0.0, 255.0) as u8,
        };

        log::trace!("Scaling RGB by {} from {:?} to {:?}", rhs, self, result);
        result
    }
}

impl Add<Rgb> for Rgb {
    type Output = Rgb;

    fn add(self, rhs: Rgb) -> Rgb {
        Rgb {
            r: self.r.saturating_add(rhs.r),
            g: self.g.saturating_add(rhs.g),
            b: self.b.saturating_add(rhs.b),
        }
    }
}

impl Sub<Rgb> for Rgb {
    type Output = Rgb;

    fn sub(self, rhs: Rgb) -> Rgb {
        Rgb {
            r: self.r.saturating_sub(rhs.r),
            g: self.g.saturating_sub(rhs.g),
            b: self.b.saturating_sub(rhs.b),
        }
    }
}

impl Display for Rgb {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

impl FromStr for Rgb {
    type Err = ();

    fn from_str(s: &str) -> Result<Rgb, ()> {
        let chars = if s.starts_with("0x") && s.len() == 8 {
            &s[2..]
        } else if s.starts_with('#') && s.len() == 7 {
            &s[1..]
        } else {
            return Err(());
        };

        match u32::from_str_radix(chars, 16) {
            Ok(mut color) => {
                let b = (color & 0xFF) as u8;
                color >>= 8;
                let g = (color & 0xFF) as u8;
                color >>= 8;
                let r = color as u8;
                Ok(Rgb { r, g, b })
            },
            Err(_) => Err(()),
        }
    }
}

/// Parse colors in XParseColor format.
pub(super) fn xparse_color(color: &[u8]) -> Option<Rgb> {
    if !color.is_empty() && color[0] == b'#' {
        parse_legacy_color(&color[1..])
    } else if color.len() >= 4 && &color[..4] == b"rgb:" {
        parse_rgb_color(&color[4..])
    } else {
        None
    }
}

/// Parse colors in `rgb:r(rrr)/g(ggg)/b(bbb)` format.
fn parse_rgb_color(color: &[u8]) -> Option<Rgb> {
    let colors = str::from_utf8(color).ok()?.split('/').collect::<alloc::vec::Vec<_>>();

    if colors.len() != 3 {
        return None;
    }

    // Scale values instead of filling with `0`s.
    let scale = |input: &str| {
        if input.len() > 4 {
            None
        } else {
            let max = u32::pow(16, input.len() as u32) - 1;
            let value = u32::from_str_radix(input, 16).ok()?;
            Some((255 * value / max) as u8)
        }
    };

    Some(Rgb { r: scale(colors[0])?, g: scale(colors[1])?, b: scale(colors[2])? })
}

/// Parse colors in `#r(rrr)g(ggg)b(bbb)` format.
fn parse_legacy_color(color: &[u8]) -> Option<Rgb> {
    let item_len = color.len() / 3;

    // Truncate/Fill to two byte precision.
    let color_from_slice = |slice: &[u8]| {
        let col = usize::from_str_radix(str::from_utf8(slice).ok()?, 16).ok()? << 4;
        Some((col >> (4 * slice.len().saturating_sub(1))) as u8)
    };

    Some(Rgb {
        r: color_from_slice(&color[0..item_len])?,
        g: color_from_slice(&color[item_len..item_len * 2])?,
        b: color_from_slice(&color[item_len * 2..])?,
    })
}

/// Parse a decimal number from ASCII bytes.
pub(super) fn parse_number(input: &[u8]) -> Option<u8> {
    if input.is_empty() {
        return None;
    }
    let mut num: u8 = 0;
    for c in input {
        let c = *c as char;
        let digit = c.to_digit(10)?;
        num = num.checked_mul(10).and_then(|v| v.checked_add(digit as u8))?;
    }
    Some(num)
}
