//! Multi-line paste warning policy.

use serde::{Deserialize, Serialize};

/// When to warn before pasting multi-line text.
///
/// Accepts `"always"`, `"never"`, or a positive integer threshold.
/// Deserialized from TOML via a custom visitor.
///
/// Default: `Always` — warn on any paste containing newlines (unless
/// bracketed paste mode is active). Matches Windows Terminal's
/// `Automatic` default behavior.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PasteWarning {
    /// Warn on any paste containing newlines.
    #[default]
    Always,
    /// Never warn (paste immediately).
    Never,
    /// Warn when the paste contains >= N lines.
    ///
    /// `Threshold(5)` triggers on 5+ lines (4+ newlines).
    Threshold(u32),
}

impl Serialize for PasteWarning {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Always => serializer.serialize_str("always"),
            Self::Never => serializer.serialize_str("never"),
            Self::Threshold(n) => serializer.serialize_u32(*n),
        }
    }
}

impl<'de> Deserialize<'de> for PasteWarning {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(PasteWarningVisitor)
    }
}

/// Serde visitor that accepts `"always"`, `"never"`, or an integer.
struct PasteWarningVisitor;

impl serde::de::Visitor<'_> for PasteWarningVisitor {
    type Value = PasteWarning;

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(r#""always", "never", or a positive integer"#)
    }

    fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
        match v {
            "always" => Ok(PasteWarning::Always),
            "never" => Ok(PasteWarning::Never),
            _ => Err(E::invalid_value(serde::de::Unexpected::Str(v), &self)),
        }
    }

    fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
        let n = u32::try_from(v).map_err(|_e| {
            E::invalid_value(serde::de::Unexpected::Unsigned(v), &"a u32 threshold")
        })?;
        Ok(PasteWarning::Threshold(n))
    }

    fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
        let n = u32::try_from(v).map_err(|_e| {
            E::invalid_value(
                serde::de::Unexpected::Signed(v),
                &"a positive u32 threshold",
            )
        })?;
        Ok(PasteWarning::Threshold(n))
    }
}
