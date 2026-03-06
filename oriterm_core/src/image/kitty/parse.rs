//! Kitty Graphics Protocol command parser.
//!
//! Parses the APC body (after the `G` prefix byte) into a structured
//! `KittyCommand`. Format: `key=value,key=value;base64payload`.

use log::debug;

/// Parsed representation of one Kitty graphics command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KittyCommand {
    /// What action to perform.
    pub action: KittyAction,
    /// Image ID (`i=`).
    pub image_id: Option<u32>,
    /// Image number (`I=`).
    pub image_number: Option<u32>,
    /// Placement ID (`p=`).
    pub placement_id: Option<u32>,
    /// Pixel format: 24 (RGB), 32 (RGBA), 100 (PNG).
    pub format: u32,
    /// Transmission method.
    pub transmission: KittyTransmission,
    /// Compression: `o=z` for zlib.
    pub compression: Option<u8>,
    /// Source rect width in pixels (`s=`).
    pub source_width: u32,
    /// Source rect height in pixels (`v=`).
    pub source_height: u32,
    /// Source rect X offset (`x=`).
    pub source_x: u32,
    /// Source rect Y offset (`y=`).
    pub source_y: u32,
    /// Display width in cells (`c=`).
    pub display_cols: Option<u32>,
    /// Display height in cells (`r=`).
    pub display_rows: Option<u32>,
    /// Cell X offset in pixels (`X=`).
    pub cell_x_offset: u32,
    /// Cell Y offset in pixels (`Y=`).
    pub cell_y_offset: u32,
    /// Z-index for layering (`z=`).
    pub z_index: i32,
    /// Suppress cursor movement (`C=1`).
    pub no_cursor_move: bool,
    /// Unicode placeholder mode (`U=1`).
    pub unicode_placeholder: bool,
    /// Quiet mode: 0=normal, 1=suppress OK, 2=suppress all.
    pub quiet: u8,
    /// More data follows (`m=1`).
    pub more_data: bool,
    /// Delete specifier (`d=` value for delete actions).
    pub delete_specifier: Option<u8>,
    /// Base64-decoded payload data.
    pub payload: Vec<u8>,
}

/// The action to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KittyAction {
    /// Upload image data.
    Transmit,
    /// Upload and immediately place.
    TransmitAndPlace,
    /// Place a previously uploaded image.
    Place,
    /// Delete image/placement.
    Delete,
    /// Animation frame operation.
    Frame,
    /// Animation control.
    Animate,
    /// Query support (no side effects).
    Query,
}

/// How image data is delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KittyTransmission {
    /// Payload contains base64 image data.
    Direct,
    /// Payload contains base64 file path.
    File,
    /// Payload contains temp file path (deleted after read).
    TempFile,
    /// Payload contains shared memory name.
    SharedMemory,
}

/// Kitty protocol errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KittyError {
    /// Invalid key-value pair in control data.
    InvalidControlData(String),
    /// Invalid base64 payload.
    InvalidBase64,
    /// Unsupported format value.
    UnsupportedFormat(u32),
}

impl std::fmt::Display for KittyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidControlData(s) => write!(f, "invalid control data: {s}"),
            Self::InvalidBase64 => write!(f, "invalid base64 payload"),
            Self::UnsupportedFormat(n) => write!(f, "unsupported format: {n}"),
        }
    }
}

impl std::error::Error for KittyError {}

impl Default for KittyCommand {
    fn default() -> Self {
        Self {
            action: KittyAction::TransmitAndPlace,
            image_id: None,
            image_number: None,
            placement_id: None,
            format: 32,
            transmission: KittyTransmission::Direct,
            compression: None,
            source_width: 0,
            source_height: 0,
            source_x: 0,
            source_y: 0,
            display_cols: None,
            display_rows: None,
            cell_x_offset: 0,
            cell_y_offset: 0,
            z_index: 0,
            no_cursor_move: false,
            unicode_placeholder: false,
            quiet: 0,
            more_data: false,
            delete_specifier: None,
            payload: Vec::new(),
        }
    }
}

/// Parse an APC body (after the `G` prefix) into a `KittyCommand`.
///
/// Format: `key=value,key=value;base64payload`
/// The control data section precedes the semicolon; the payload follows.
pub fn parse_kitty_command(raw: &[u8]) -> Result<KittyCommand, KittyError> {
    let mut cmd = KittyCommand::default();

    // Split at first `;` — control data vs payload.
    let (control, payload_b64) = match raw.iter().position(|&b| b == b';') {
        Some(pos) => (&raw[..pos], &raw[pos + 1..]),
        None => (raw, &[] as &[u8]),
    };

    // Parse control data: comma-separated key=value pairs.
    parse_control_data(control, &mut cmd);

    // Decode base64 payload.
    if !payload_b64.is_empty() {
        cmd.payload = decode_base64(payload_b64)?;
    }

    Ok(cmd)
}

/// Parse comma-separated `key=value` pairs from control data.
fn parse_control_data(data: &[u8], cmd: &mut KittyCommand) {
    for pair in data.split(|&b| b == b',') {
        if pair.is_empty() {
            continue;
        }

        // Find '=' separator.
        let eq_pos = pair.iter().position(|&b| b == b'=');
        let (key, value) = match eq_pos {
            Some(0) => continue, // No key.
            Some(pos) => (pair[0], &pair[pos + 1..]),
            None => {
                // Single char with no value — skip gracefully.
                continue;
            }
        };

        apply_key_value(key, value, cmd);
    }
}

/// Apply a single key=value pair to the command.
fn apply_key_value(key: u8, value: &[u8], cmd: &mut KittyCommand) {
    match key {
        b'a' => {
            cmd.action = match value.first() {
                Some(b't') => KittyAction::Transmit,
                Some(b'p') => KittyAction::Place,
                Some(b'd') => KittyAction::Delete,
                Some(b'f') => KittyAction::Frame,
                Some(b'a') => KittyAction::Animate,
                Some(b'q') => KittyAction::Query,
                // 'T' and unknown values default to TransmitAndPlace.
                _ => KittyAction::TransmitAndPlace,
            };
        }
        b'i' => cmd.image_id = parse_u32(value),
        b'I' => cmd.image_number = parse_u32(value),
        b'p' => cmd.placement_id = parse_u32(value),
        b'f' => cmd.format = parse_u32(value).unwrap_or(32),
        b't' => {
            cmd.transmission = match value.first() {
                Some(b'f') => KittyTransmission::File,
                Some(b't') => KittyTransmission::TempFile,
                Some(b's') => KittyTransmission::SharedMemory,
                // 'd' and unknown values default to Direct.
                _ => KittyTransmission::Direct,
            };
        }
        b'o' => cmd.compression = value.first().copied(),
        b's' => cmd.source_width = parse_u32(value).unwrap_or(0),
        b'v' => cmd.source_height = parse_u32(value).unwrap_or(0),
        b'x' => cmd.source_x = parse_u32(value).unwrap_or(0),
        b'y' => cmd.source_y = parse_u32(value).unwrap_or(0),
        b'c' => cmd.display_cols = parse_u32(value),
        b'r' => cmd.display_rows = parse_u32(value),
        b'X' => cmd.cell_x_offset = parse_u32(value).unwrap_or(0),
        b'Y' => cmd.cell_y_offset = parse_u32(value).unwrap_or(0),
        b'z' => cmd.z_index = parse_i32(value),
        b'C' => cmd.no_cursor_move = value == b"1",
        b'U' => cmd.unicode_placeholder = value == b"1",
        b'q' => cmd.quiet = parse_u32(value).unwrap_or(0) as u8,
        b'm' => cmd.more_data = value == b"1",
        b'd' => cmd.delete_specifier = value.first().copied(),
        _ => {
            debug!("kitty graphics: unknown key {:?}", key as char);
        }
    }
}

/// Parse a byte slice as a u32 decimal number.
fn parse_u32(value: &[u8]) -> Option<u32> {
    let s = std::str::from_utf8(value).ok()?;
    s.parse().ok()
}

/// Parse a byte slice as an i32 decimal number.
fn parse_i32(value: &[u8]) -> i32 {
    std::str::from_utf8(value)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

/// Decode standard base64 (with or without padding).
///
/// Kitty protocol uses standard base64 (A-Z, a-z, 0-9, +, /).
fn decode_base64(data: &[u8]) -> Result<Vec<u8>, KittyError> {
    // Filter out whitespace that some implementations insert.
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
            b'=' => continue, // Padding.
            _ => return Err(KittyError::InvalidBase64),
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
