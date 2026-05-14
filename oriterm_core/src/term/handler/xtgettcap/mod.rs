//! XTGETTCAP (DCS `+q Pt ST`) terminfo capability query reply emission.
//!
//! Implements the xterm terminfo capability query/reply protocol.
//!
//! Query format: `DCS + q <hex>;<hex>;<hex> ST` where each `<hex>` is a
//! hex-encoded ASCII capability name. Reply format:
//! - Known caps grouped: `DCS 1 + r <hex>=<hex>;...;<hex>=<hex> ST`
//! - Unknown caps grouped: `DCS 0 + r <hex>;...;<hex> ST`
//!
//! The reply is emitted synchronously via `Effect::Pty(PtyEffect::Write)`
//! mirroring `status_xtversion` — notcurses' handshake polling window
//! closes before async coordinator replies land.

use crate::effect::sink::EffectSink;
use crate::effect::{Effect, PtyEffect, PtyWriteKind};

use super::Term;

/// Capacity hint for the `DCS 1 + r` (known) reply buffer.
///
/// Sized for typical 3-cap queries (notcurses sends TN+RGB+hpa ≈ 85 bytes).
/// Worst-case 13-cap reply is ~520 bytes — `Vec` grows on overflow so
/// these are performance hints only. Correctness is unaffected.
const KNOWN_REPLY_CAPACITY_HINT: usize = 96;

/// Capacity hint for the `DCS 0 + r` (unknown) reply buffer.
///
/// Sized for typical single-unknown-cap queries.
const UNKNOWN_REPLY_CAPACITY_HINT: usize = 48;

/// Decode hex-encoded ASCII (e.g., `b"544E"` or `b"544e"` → `b"TN"`).
///
/// Case-insensitive on input — notcurses sends lowercase, xterm canon is
/// uppercase; both must decode. Returns `None` on odd length or non-hex
/// characters.
pub(super) fn hex_decode(input: &[u8]) -> Option<Vec<u8>> {
    if input.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(input.len() / 2);
    for chunk in input.chunks_exact(2) {
        let hi = hex_nibble(chunk[0])?;
        let lo = hex_nibble(chunk[1])?;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

/// Hex-encode ASCII bytes UPPERCASE per xterm canon (xterm
/// `charproc.c:9466` uses `"%02X"`). Example: `b"oriterm"` →
/// `b"6F72697465726D"`.
pub(super) fn hex_encode(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len() * 2);
    for &byte in input {
        out.push(hex_digit(byte >> 4));
        out.push(hex_digit(byte & 0x0F));
    }
    out
}

#[inline]
fn hex_nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

#[inline]
fn hex_digit(n: u8) -> u8 {
    debug_assert!(n < 16);
    if n < 10 {
        b'0' + n
    } else {
        b'A' + (n - 10)
    }
}

/// Resolve host OS name for fish's `query-os-name` cap.
///
/// Returns the canonical OS name fish expects per
/// `fish-shell/doc_src/cmds/status.rst:136-137` — the host OS NAME, NOT
/// the terminal name. Mapping per `std::env::consts::OS`:
/// `"linux" → "Linux"`, `"macos" → "Darwin"`, `"windows" → "Windows"`,
/// other → `"Unknown"`.
fn query_os_name_value() -> &'static [u8] {
    match std::env::consts::OS {
        "linux" => b"Linux",
        "macos" => b"Darwin",
        "windows" => b"Windows",
        _ => b"Unknown",
    }
}

/// Look up a terminfo capability name → value.
///
/// Returns `None` for unrecognized caps. 13-cap minimum-viable set:
/// - notcurses probes: `TN`, `RGB`, `hpa`
/// - fish probes: `indn`, `query-os-name`
/// - Section 38.4 minimum-viable: `Co`, `colors`, `Smulx`, `Setulc`,
///   `Smol`, `Se`, `Ss`, `Ms`
fn lookup_tcap(name: &[u8]) -> Option<&'static [u8]> {
    match name {
        b"TN" => Some(b"oriterm"),
        b"Co" => Some(b"256"),
        b"colors" => Some(b"256"),
        b"RGB" => Some(b"8/8/8"),
        b"hpa" => Some(b"\x1b[%i%p1%dG"),
        b"indn" => Some(b"\x1b[%p1%dS"),
        b"query-os-name" => Some(query_os_name_value()),
        b"Smulx" => Some(b"\x1b[4:%p1%dm"),
        b"Setulc" => Some(b"\x1b[58:2::%p1%d:%p2%d:%p3%dm"),
        b"Smol" => Some(b"\x1b[53m"),
        b"Se" => Some(b"\x1b[2 q"),
        b"Ss" => Some(b"\x1b[%p1%d q"),
        b"Ms" => Some(b"\x1b]52;%p1%s;%p2%s\x1b\\"),
        _ => None,
    }
}

/// Build a `DCS 1 + r <hex>=<hex>;...;<hex>=<hex> ST` reply for known caps.
fn build_known_reply(entries: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(KNOWN_REPLY_CAPACITY_HINT);
    bytes.extend_from_slice(b"\x1bP1+r");
    for (i, (name, value)) in entries.iter().enumerate() {
        if i > 0 {
            bytes.push(b';');
        }
        bytes.extend_from_slice(name);
        bytes.push(b'=');
        bytes.extend_from_slice(value);
    }
    bytes.extend_from_slice(b"\x1b\\");
    bytes
}

/// Build a `DCS 0 + r <hex>;...;<hex> ST` reply for unknown caps.
fn build_unknown_reply(names: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(UNKNOWN_REPLY_CAPACITY_HINT);
    bytes.extend_from_slice(b"\x1bP0+r");
    for (i, name) in names.iter().enumerate() {
        if i > 0 {
            bytes.push(b';');
        }
        bytes.extend_from_slice(name);
    }
    bytes.extend_from_slice(b"\x1b\\");
    bytes
}

impl<S: EffectSink> Term<S> {
    /// XTGETTCAP (DCS `+q Pt ST`): respond with capability value(s).
    ///
    /// `payload` is hex-encoded capability name(s) separated by `;`.
    /// On `aborted == true`: drop the reply (DCS interrupted mid-payload).
    ///
    /// Reply format per xterm ctlseqs:
    /// - Known caps grouped: `\x1bP1+r<hex>=<hex>;...;<hex>=<hex>\x1b\\`
    /// - Unknown caps grouped: `\x1bP0+r<hex>;...;<hex>\x1b\\`
    ///
    /// Hex names in the reply are canonicalized to UPPERCASE per xterm
    /// `charproc.c:9466 ("%02X")` regardless of the input case.
    pub(super) fn status_xtgettcap(&mut self, payload: &[u8], aborted: bool) {
        if aborted {
            return;
        }

        let mut known: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        let mut unknown: Vec<Vec<u8>> = Vec::new();

        for chunk in payload.split(|&b| b == b';') {
            if chunk.is_empty() {
                continue;
            }
            let Some(decoded) = hex_decode(chunk) else {
                continue;
            };
            // Canonicalize hex name to uppercase (xterm canon) — do NOT
            // preserve input case via `chunk.to_vec()`.
            let canonical_name = hex_encode(&decoded);
            match lookup_tcap(&decoded) {
                Some(value) => known.push((canonical_name, hex_encode(value))),
                None => unknown.push(canonical_name),
            }
        }

        if !known.is_empty() {
            self.effect_sink.push(Effect::Pty(PtyEffect::Write {
                bytes: build_known_reply(&known),
                kind: PtyWriteKind::DeviceAttribute,
            }));
        }
        if !unknown.is_empty() {
            self.effect_sink.push(Effect::Pty(PtyEffect::Write {
                bytes: build_unknown_reply(&unknown),
                kind: PtyWriteKind::DeviceAttribute,
            }));
        }
    }
}

#[cfg(test)]
mod tests;
