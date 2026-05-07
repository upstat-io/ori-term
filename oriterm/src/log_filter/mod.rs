//! `RUST_LOG` directive parser for the oriterm file logger.
//!
//! Supports both bare-level (`trace`) and `env_logger`-style directive-list
//! (`oriterm_core::grid::dirty=trace,oriterm::gpu::prepare::dirty_skip=trace`)
//! forms. The directive matcher preserves the wgpu/naga noise gate the
//! original logger had — non-`oriterm*` targets are always disabled,
//! regardless of any matching directive, so an operator who runs
//! `RUST_LOG=wgpu=trace` does NOT enable wgpu logging.

use log::{Level, LevelFilter};

/// Parsed `RUST_LOG` filter — a default level plus zero or more
/// target-prefix directives sorted longest-prefix-first.
pub struct LogFilter {
    default: LevelFilter,
    /// Sorted longest-prefix-first so `enabled()` does longest-match.
    directives: Vec<(String, LevelFilter)>,
}

impl LogFilter {
    /// Parse a `RUST_LOG` value.
    ///
    /// Empty / pure-garbage input returns a filter with `Info` default and
    /// no directives. Bare levels (`"trace"`) set the default. Directives
    /// (`"oriterm_core=trace"`) are accumulated. Invalid pieces are silently
    /// dropped — a single typo never panics or disables logging.
    pub fn parse(s: &str) -> Self {
        let s = s.trim();
        if s.is_empty() {
            return Self {
                default: LevelFilter::Info,
                directives: Vec::new(),
            };
        }
        let mut default = LevelFilter::Off;
        let mut had_default = false;
        let mut directives = Vec::new();
        for piece in s.split(',') {
            let piece = piece.trim();
            if piece.is_empty() {
                continue;
            }
            if let Some((target, lvl)) = piece.split_once('=') {
                if let Ok(lvl) = lvl.trim().parse::<LevelFilter>() {
                    directives.push((target.trim().to_string(), lvl));
                }
            } else if let Ok(lvl) = piece.parse::<LevelFilter>() {
                default = lvl;
                had_default = true;
            } else {
                // Garbage piece — silently drop. A single typo must never
                // panic or disable logging.
            }
        }
        if !had_default && directives.is_empty() {
            default = LevelFilter::Info;
        }
        directives.sort_by_key(|d| std::cmp::Reverse(d.0.len()));
        Self {
            default,
            directives,
        }
    }

    /// The maximum level across all directives + the default — used to set
    /// `log::set_max_level()` so the global short-circuit never vetoes a
    /// directive's level.
    pub fn max_level(&self) -> LevelFilter {
        self.directives
            .iter()
            .map(|(_, l)| *l)
            .chain(std::iter::once(self.default))
            .max()
            .unwrap_or(LevelFilter::Off)
    }

    /// Decide whether a record at `level` for `target` should be logged.
    ///
    /// Non-`oriterm*` targets are ALWAYS disabled — the noise gate the
    /// original logger had. Directives that name non-oriterm targets are
    /// silently dropped at this check; the operator never sees their effect.
    pub fn enabled(&self, target: &str, level: Level) -> bool {
        if !target.starts_with("oriterm") {
            return false;
        }
        for (prefix, lvl) in &self.directives {
            if target.starts_with(prefix.as_str()) {
                return level <= *lvl;
            }
        }
        level <= self.default
    }
}

#[cfg(test)]
mod tests;
