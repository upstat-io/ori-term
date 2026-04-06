//! Scenario specification and TOML sidecar parser.
//!
//! Discovers `.teseq` scenario files and parses optional `.toml` sidecars
//! into `ScenarioSpec` structs that drive the runner.

use std::path::Path;

use serde::Deserialize;

/// Configuration for a single teseq test scenario.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct ScenarioSpec {
    /// Terminal dimensions and configuration.
    pub terminal: TerminalConfig,
    /// Pre-scenario setup (mode enabling, cursor positioning).
    pub setup: SetupConfig,
    /// Expected outcomes for assertion.
    pub expect: ExpectConfig,
}

impl Default for ScenarioSpec {
    fn default() -> Self {
        Self {
            terminal: TerminalConfig::default(),
            setup: SetupConfig::default(),
            expect: ExpectConfig::default(),
        }
    }
}

/// Terminal size and theme configuration.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct TerminalConfig {
    /// Viewport width in columns.
    pub cols: usize,
    /// Viewport height in rows.
    pub rows: usize,
    /// Scrollback buffer size (lines).
    pub scrollback: usize,
    /// Color theme: "dark" (default) or "light".
    pub theme: String,
}

impl Default for TerminalConfig {
    fn default() -> Self {
        Self {
            cols: 80,
            rows: 24,
            scrollback: 0,
            theme: String::from("dark"),
        }
    }
}

/// Pre-scenario setup sequences.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct SetupConfig {
    /// Raw escape sequences to feed before the scenario.
    ///
    /// TOML strings use `\\x1b` for ESC (double-escape: TOML `\\` → Rust `\`).
    pub pre_feed: Vec<String>,
}

/// Expected outcomes for assertion helpers.
#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct ExpectConfig {
    /// Whether to assert grid state via insta snapshot.
    pub grid_snapshot: bool,
    /// Whether to assert event sequence via insta snapshot.
    pub event_snapshot: bool,
    /// Expected cursor position after scenario.
    pub cursor: Option<CursorExpect>,
    /// Expected event type names (matched via `contains()` on Debug output).
    pub events: Vec<String>,
}

impl Default for ExpectConfig {
    fn default() -> Self {
        Self {
            grid_snapshot: true,
            event_snapshot: true,
            cursor: None,
            events: Vec::new(),
        }
    }
}

/// Expected cursor position.
#[derive(Debug, Deserialize)]
pub struct CursorExpect {
    /// Column (0-based).
    pub col: usize,
    /// Line (0-based, viewport-relative).
    pub line: usize,
}

impl ScenarioSpec {
    /// Load a scenario spec from an optional TOML sidecar.
    ///
    /// Given `bel.teseq`, looks for `bel.toml` in the same directory.
    /// Returns defaults if no sidecar exists.
    pub fn load(teseq_path: &Path) -> Self {
        let toml_path = teseq_path.with_extension("toml");
        if toml_path.exists() {
            let content = std::fs::read_to_string(&toml_path).unwrap_or_else(|e| {
                panic!("failed to read {}: {e}", toml_path.display());
            });
            toml::from_str(&content).unwrap_or_else(|e| {
                panic!("failed to parse {}: {e}", toml_path.display());
            })
        } else {
            Self::default()
        }
    }
}

/// Discover all `.teseq` scenario files in a directory.
///
/// Non-recursive scan, sorted deterministically by filename.
pub fn discover_scenarios(family_dir: &Path) -> Vec<std::path::PathBuf> {
    let mut scenarios: Vec<_> = std::fs::read_dir(family_dir)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", family_dir.display()))
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("teseq") {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    scenarios.sort();
    scenarios
}
