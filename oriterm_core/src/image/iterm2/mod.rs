//! iTerm2 inline image protocol parser.
//!
//! Parses `OSC 1337 ; File=[args] : <base64-data> ST` sequences into
//! structured `Iterm2Image` commands. Supports width/height specs
//! (auto, pixel, cell, percentage), aspect ratio preservation, and
//! inline vs download mode.

/// Parsed iTerm2 image command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Iterm2Image {
    /// Filename (decoded from base64).
    pub name: Option<String>,
    /// File size hint (informational only).
    pub size: Option<u64>,
    /// Display width specification.
    pub width: SizeSpec,
    /// Display height specification.
    pub height: SizeSpec,
    /// Whether to preserve aspect ratio (default: true).
    pub preserve_aspect_ratio: bool,
    /// Display inline (true) or as download (false).
    pub inline: bool,
    /// Raw base64-decoded image data.
    pub data: Vec<u8>,
}

/// Display size specification for width or height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeSpec {
    /// Automatic sizing based on image native dimensions.
    Auto,
    /// Size in terminal cells.
    Cells(u32),
    /// Size in pixels.
    Pixels(u32),
    /// Size as percentage of terminal dimensions.
    Percent(u32),
}

/// Parse raw OSC params (after `1337`) into an `Iterm2Image`.
///
/// `params` is the slice of OSC params after the `1337` prefix, where
/// the VTE parser has split on `;`. The first param starts with `File=`.
/// The base64 payload follows `:` in the last key=value param.
pub fn parse_iterm2_file(params: &[&[u8]]) -> Result<Iterm2Image, Iterm2Error> {
    if params.is_empty() {
        return Err(Iterm2Error::MissingFilePrefix);
    }

    let mut image = Iterm2Image {
        name: None,
        size: None,
        width: SizeSpec::Auto,
        height: SizeSpec::Auto,
        preserve_aspect_ratio: true,
        inline: false,
        data: Vec::new(),
    };

    // The params are semicolon-split by VTE. We need to find the `:` that
    // separates key=value args from the base64 payload. The colon appears
    // in the last key=value param (or could be the only param).
    let mut payload_b64: &[u8] = &[];

    for (i, param) in params.iter().enumerate() {
        let kv = if i == 0 {
            // First param starts with "File=" — strip that prefix.
            &param[b"File=".len()..]
        } else {
            param
        };

        // Check if this param contains the `:` separator to base64 data.
        if let Some(colon_pos) = kv.iter().position(|&b| b == b':') {
            // Everything before `:` is the last key=value pair.
            let before_colon = &kv[..colon_pos];
            payload_b64 = &kv[colon_pos + 1..];

            if !before_colon.is_empty() {
                parse_key_value(before_colon, &mut image);
            }

            // If there are more params after this one, they are part of
            // the base64 payload (shouldn't happen, but handle gracefully).
            // The VTE parser shouldn't split base64 since it contains no `;`.
            break;
        }

        // No colon — this is a pure key=value param.
        if !kv.is_empty() {
            parse_key_value(kv, &mut image);
        }
    }

    if payload_b64.is_empty() {
        return Err(Iterm2Error::MissingPayload);
    }

    // Decode base64 payload.
    image.data = decode_base64(payload_b64)?;

    if image.data.is_empty() {
        return Err(Iterm2Error::MissingPayload);
    }

    Ok(image)
}

/// Parse a single `key=value` pair and apply it to the image.
fn parse_key_value(kv: &[u8], image: &mut Iterm2Image) {
    let Some(eq_pos) = kv.iter().position(|&b| b == b'=') else {
        return;
    };

    let key = &kv[..eq_pos];
    let value = &kv[eq_pos + 1..];

    match key {
        b"name" => {
            image.name = decode_base64(value)
                .ok()
                .and_then(|b| String::from_utf8(b).ok());
        }
        b"size" => {
            image.size = std::str::from_utf8(value).ok().and_then(|s| s.parse().ok());
        }
        b"width" => {
            image.width = parse_size_spec(value);
        }
        b"height" => {
            image.height = parse_size_spec(value);
        }
        b"preserveAspectRatio" => {
            image.preserve_aspect_ratio = value != b"0";
        }
        b"inline" => {
            image.inline = value == b"1";
        }
        _ => {
            // Unknown keys ignored gracefully.
        }
    }
}

/// Parse a size specification: `auto`, `N` (cells), `Npx` (pixels), `N%` (percentage).
fn parse_size_spec(value: &[u8]) -> SizeSpec {
    let s = match std::str::from_utf8(value) {
        Ok(s) => s,
        Err(_) => return SizeSpec::Auto,
    };

    if s.eq_ignore_ascii_case("auto") || s.is_empty() {
        return SizeSpec::Auto;
    }

    if let Some(px) = s.strip_suffix("px") {
        return px.parse().map_or(SizeSpec::Auto, SizeSpec::Pixels);
    }

    if let Some(pct) = s.strip_suffix('%') {
        return pct.parse().map_or(SizeSpec::Auto, SizeSpec::Percent);
    }

    // Plain number = cell count.
    s.parse().map_or(SizeSpec::Auto, SizeSpec::Cells)
}

/// iTerm2 protocol errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Iterm2Error {
    /// OSC params don't start with `File=`.
    MissingFilePrefix,
    /// No base64 payload after `:`.
    MissingPayload,
    /// Invalid base64 encoding.
    InvalidBase64,
}

impl std::fmt::Display for Iterm2Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingFilePrefix => write!(f, "missing File= prefix in OSC 1337"),
            Self::MissingPayload => write!(f, "missing base64 payload after ':'"),
            Self::InvalidBase64 => write!(f, "invalid base64 encoding"),
        }
    }
}

impl std::error::Error for Iterm2Error {}

/// Decode standard base64 (with or without padding).
fn decode_base64(data: &[u8]) -> Result<Vec<u8>, Iterm2Error> {
    let clean: Vec<u8> = data
        .iter()
        .copied()
        .filter(|&b| !b.is_ascii_whitespace())
        .collect();

    if clean.is_empty() {
        return Ok(Vec::new());
    }

    let mut out = Vec::with_capacity(clean.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;

    for &byte in &clean {
        let val = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' => continue,
            _ => return Err(Iterm2Error::InvalidBase64),
        };

        buf = (buf << 6) | u32::from(val);
        bits += 6;

        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests;
