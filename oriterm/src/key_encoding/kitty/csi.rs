//! Kitty CSI sequence building: legacy terminator lookup + CSI-u encoding.
//!
//! Extracted from `kitty/mod.rs` to keep that file under the 500-line limit.

use winit::keyboard::NamedKey;

use super::super::Modifiers;
use super::super::cursor_keys::CursorKey;
use super::super::legacy::{cursor_key_for_named, function_key_terminator, tilde_key};

/// Whether a named key has an unambiguous legacy encoding.
///
/// These keys have unique VT/xterm escape sequences that no other key
/// shares, so they don't need CSI u disambiguation. Used to keep
/// `DISAMBIGUATE_ESC_CODES` mode compatible with shells that don't
/// bind the CSI u functional key codepoints.
pub(super) fn has_unambiguous_legacy(named: NamedKey) -> bool {
    legacy_csi_info(named).is_some()
}

/// Legacy CSI encoding for a named key.
///
/// When a key has a well-known legacy CSI sequence, the Kitty spec prefers
/// that terminator over the universal `u`. For letter-terminated keys
/// (arrows, Home/End, F1-F4), the base number is 1. For tilde-terminated
/// keys (Insert, Delete, PageUp/Down, F5-F12), it is the traditional
/// numeric parameter.
struct LegacyCsiInfo {
    /// Numeric parameter (1 for letter keys, traditional number for tilde keys).
    base: u32,
    /// Terminator byte (`A`-`S` for letter keys, `~` for tilde keys).
    terminator: u8,
}

/// Look up legacy CSI info for a named key.
///
/// Returns `None` for keys that have no legacy terminator (they use `u`).
/// The terminator-byte selection routes through the SSOT helpers in
/// `super::legacy` (cursor keys → [`cursor_key_for_named`] / [`CursorKey::terminator`];
/// F1-F4 → [`function_key_terminator`]; tilde keys → [`tilde_key`]) so all
/// protocol paths (legacy keyboard, alt-scroll, Kitty CSI-u) share one
/// canonical table per key category.
#[must_use]
fn legacy_csi_info(named: NamedKey) -> Option<LegacyCsiInfo> {
    // Letter-terminated keys: base = 1.
    let letter = cursor_key_for_named(named)
        .map(CursorKey::terminator)
        .or_else(|| function_key_terminator(named));
    if let Some(term) = letter {
        return Some(LegacyCsiInfo {
            base: 1,
            terminator: term,
        });
    }

    // Tilde-terminated keys: base = traditional numeric parameter (SSOT in `legacy::tilde_key`).
    tilde_key(named).map(|tk| LegacyCsiInfo {
        base: u32::from(tk.num),
        terminator: b'~',
    })
}

/// All inputs needed to encode one Kitty CSI key sequence.
#[derive(Clone, Copy)]
pub(super) struct CsiKeyParams<'a> {
    /// Unicode codepoint of the key (or CSI-u base when no named key).
    pub(super) codepoint: u32,
    /// Active keyboard modifiers.
    pub(super) mods: Modifiers,
    /// Event-type suffix (`:1`/`:2`/`:3` for press/repeat/release).
    pub(super) event_suffix: &'a str,
    /// Associated text payload, if any.
    pub(super) text: Option<&'a str>,
    /// Named (functional) key, if this is one.
    pub(super) named: Option<NamedKey>,
    /// Alternate (shifted/base-layout) codepoint, if reported.
    pub(super) alternate_key: Option<u32>,
}

/// Build a CSI key sequence with the appropriate terminator.
///
/// Keys with legacy CSI encodings use their traditional terminator
/// (e.g., `A` for `ArrowUp`, `~` for `Insert`). All other keys use `u`.
/// When `alternate_key` is `Some`, the base field includes it as
/// `base::alternate` (per Kitty `REPORT_ALTERNATE_KEYS` spec).
pub(super) fn build_csi_sequence(params: CsiKeyParams<'_>) -> Vec<u8> {
    let CsiKeyParams {
        codepoint,
        mods,
        event_suffix,
        text,
        named,
        alternate_key,
    } = params;
    let (base, terminator) = match named.and_then(legacy_csi_info) {
        Some(info) => (info.base, info.terminator),
        None => (codepoint, b'u'),
    };

    // Format base field: `base` or `base::alternate` (skipping shifted_key).
    let base_field = match alternate_key {
        Some(alt) => format!("{base}::{alt}"),
        None => base.to_string(),
    };

    let mod_param = mods.xterm_param();
    let t = terminator as char;
    if text.is_some() || mod_param > 0 || !event_suffix.is_empty() || alternate_key.is_some() {
        let m = if mod_param > 0 { mod_param } else { 1 };
        let text_suffix = text.map_or(String::new(), |txt| format!(";{txt}"));
        format!("\x1b[{base_field};{m}{event_suffix}{text_suffix}{t}").into_bytes()
    } else {
        format!("\x1b[{base_field}{t}").into_bytes()
    }
}
