//! Const-constructible scenario definitions for the verification chain.
//!
//! Every catalog row that reaches `verified` status is backed by a test
//! declaring a `const SpecScenario` and driving it through the harness.
//! Const-constructibility enables the citation scanner (04.8) to find
//! `catalog_row_id: "…"` via a literal grep.

use oriterm_core::{CursorShape, Rgb};
use vte::ansi::cursor_icon::CursorIcon;

/// Const-constructible scenario definition (no closures, function pointers only).
///
/// Every field type is `const`-constructible. Slices use `&'static [u16]`
/// / `&'static [u8]`. Expectation constructors are `const fn`.
#[derive(Copy, Clone, Debug)]
pub struct SpecScenario {
    /// Stable catalog row ID (e.g. `"ECMA48-CUP"`). Cross-checked by the
    /// coverage report's citation scanner.
    pub catalog_row_id: &'static str,
    /// Raw bytes to feed through the parser.
    pub bytes: &'static [u8],
    /// Highest rung the test drives — determines `applicable_rungs()`.
    pub apex_layer: ApexLayer,
    /// Optional pre-feed bytes (e.g. put the terminal in origin mode).
    /// Empty slice `b""` when no setup needed.
    pub setup: &'static [u8],
    /// Per-rung expectations.
    pub expectations: ScenarioExpectations,
}

impl SpecScenario {
    /// Returns the rungs this scenario covers, in execution order,
    /// up to and including the rung for the declared `apex_layer`.
    pub fn applicable_rungs(&self) -> &'static [RungName] {
        self.apex_layer.rung_chain()
    }
}

/// Per-rung expectations for a scenario.
///
/// Each field is `Option` — `None` means the rung is not applicable
/// (the scenario's apex is below that rung).
#[derive(Copy, Clone, Debug, Default)]
pub struct ScenarioExpectations {
    /// Rung 1: expected raw parser action.
    pub parser: Option<ParserExpectation>,
    /// Rung 2: expected semantic dispatch call.
    pub dispatch: Option<DispatchExpectation>,
    /// Rung 3a: expected terminal state after processing.
    pub state: Option<StateExpectation>,
    /// Rung 3b: expected effect emitted.
    pub effect: Option<EffectExpectation>,
    /// Rung 4: expected renderable content.
    pub renderable: Option<RenderableExpectation>,
    /// Rung 5: expected frame input.
    pub frame_input: Option<FrameInputExpectation>,
    /// Rung 6: expected GPU instance data.
    pub gpu_instance: Option<GpuInstanceExpectation>,
    /// Rung 7: expected texture render output.
    pub texture: Option<TextureExpectation>,
    /// Rung 8: expected golden image match.
    pub golden: Option<GoldenExpectation>,
}

/// The highest rung a scenario drives.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ApexLayer {
    // Visual chain.
    /// Only check parser tokenization.
    ParserOnly,
    /// Check parser + dispatch routing.
    Dispatch,
    /// Check parser + dispatch + terminal state mutation.
    State,
    /// Check through renderable content.
    Renderable,
    /// Check through frame input assembly.
    FrameInput,
    /// Check through GPU instance buffer.
    GpuInstance,
    /// Check through texture render.
    TextureRender,
    /// Check through golden image comparison.
    GoldenImage,

    // Non-visual chain (effect transcript apex).
    /// Apex: PTY write effect (device replies, mouse reports).
    EffectPtyWrite,
    /// Apex: clipboard store effect (OSC 52).
    EffectClipboard,
    /// Apex: host title change (OSC 0/2).
    EffectHostTitle,
    /// Apex: mode state change (DECSET/DECRST).
    EffectModeState,
    /// Apex: presentation commit (Mode 2026).
    EffectPresentationCommit,
    /// Apex: audio request (DECPS/OSC audio).
    EffectAudio,
    /// Apex: desktop notification (OSC 9/99/777).
    EffectHostNotification,
}

impl ApexLayer {
    /// Returns the rung chain (in execution order) for this apex layer.
    pub const fn rung_chain(self) -> &'static [RungName] {
        match self {
            // Visual chain — increasingly deep.
            Self::ParserOnly => &[RungName::Parser],
            Self::Dispatch => &[RungName::Parser, RungName::Dispatch],
            Self::State => &[RungName::Parser, RungName::Dispatch, RungName::State],
            Self::Renderable => &[
                RungName::Parser,
                RungName::Dispatch,
                RungName::State,
                RungName::Renderable,
            ],
            Self::FrameInput => &[
                RungName::Parser,
                RungName::Dispatch,
                RungName::State,
                RungName::Renderable,
                RungName::FrameInput,
            ],
            Self::GpuInstance => &[
                RungName::Parser,
                RungName::Dispatch,
                RungName::State,
                RungName::Renderable,
                RungName::FrameInput,
                RungName::GpuInstance,
            ],
            Self::TextureRender => &[
                RungName::Parser,
                RungName::Dispatch,
                RungName::State,
                RungName::Renderable,
                RungName::FrameInput,
                RungName::GpuInstance,
                RungName::TextureRender,
            ],
            Self::GoldenImage => &[
                RungName::Parser,
                RungName::Dispatch,
                RungName::State,
                RungName::Renderable,
                RungName::FrameInput,
                RungName::GpuInstance,
                RungName::TextureRender,
                RungName::GoldenImage,
            ],

            // Non-visual chain — parser + dispatch + effect apex.
            Self::EffectPtyWrite
            | Self::EffectClipboard
            | Self::EffectHostTitle
            | Self::EffectModeState
            | Self::EffectPresentationCommit
            | Self::EffectAudio
            | Self::EffectHostNotification => {
                &[RungName::Parser, RungName::Dispatch, RungName::Effect]
            }
        }
    }
}

/// Rung names for the verification chain.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RungName {
    /// Rung 1: raw parser tokenization.
    Parser,
    /// Rung 2: semantic handler dispatch.
    Dispatch,
    /// Rung 3a: terminal state mutation.
    State,
    /// Rung 3b: effect transcript.
    Effect,
    /// Rung 4: renderable content extraction.
    Renderable,
    /// Rung 5: frame input assembly.
    FrameInput,
    /// Rung 6: GPU instance buffer.
    GpuInstance,
    /// Rung 7: texture render.
    TextureRender,
    /// Rung 8: golden image comparison.
    GoldenImage,
}

impl RungName {
    /// Map an `ApexLayer` to its terminal rung.
    pub const fn from_apex(apex: ApexLayer) -> Self {
        match apex {
            ApexLayer::ParserOnly => Self::Parser,
            ApexLayer::Dispatch => Self::Dispatch,
            ApexLayer::State => Self::State,
            ApexLayer::Renderable => Self::Renderable,
            ApexLayer::FrameInput => Self::FrameInput,
            ApexLayer::GpuInstance => Self::GpuInstance,
            ApexLayer::TextureRender => Self::TextureRender,
            ApexLayer::GoldenImage => Self::GoldenImage,
            ApexLayer::EffectPtyWrite
            | ApexLayer::EffectClipboard
            | ApexLayer::EffectHostTitle
            | ApexLayer::EffectModeState
            | ApexLayer::EffectPresentationCommit
            | ApexLayer::EffectAudio
            | ApexLayer::EffectHostNotification => Self::Effect,
        }
    }
}

// ── Expectation types ───────────────────────────────────────────────
//
// Lightweight const-constructible stubs. Observers (04.2) consume these
// and perform the actual assertions against SpecOutcome.

/// Parser rung expectation: expected raw `Perform` callback.
#[derive(Copy, Clone, Debug)]
pub struct ParserExpectation {
    /// Expected final byte / action char (e.g. `'H'` for CUP, `'q'` for DCS sixel).
    pub action: char,
    /// Expected parameter values (flattened subparams).
    pub params: &'static [u16],
    /// Expected CSI/ESC intermediate bytes.
    pub intermediates: &'static [u8],
    /// OSC command number (e.g. `Some(52)` for OSC 52 clipboard).
    /// When set, the observer matches `OscDispatch` with this command number
    /// parsed from the first param's ASCII digits. `None` means this
    /// expectation is not for an OSC sequence.
    pub osc_command: Option<u16>,
}

impl ParserExpectation {
    /// Convenience: CSI sequence with given action and params.
    pub const fn csi_with_params(action: char, params: &'static [u16]) -> Self {
        Self {
            action,
            params,
            intermediates: &[],
            osc_command: None,
        }
    }

    /// Convenience: OSC command with given numeric command number.
    ///
    /// Example: `osc(52)` matches `OSC 52` (clipboard).
    /// The observer parses the full ASCII command number from the
    /// OSC's first param and compares against this value.
    pub const fn osc(command: u16) -> Self {
        Self {
            action: '\0',
            params: &[],
            intermediates: &[],
            osc_command: Some(command),
        }
    }

    /// Convenience: DCS sequence with given action char and params.
    pub const fn dcs(action: char, params: &'static [u16]) -> Self {
        Self {
            action,
            params,
            intermediates: &[],
            osc_command: None,
        }
    }
}

/// Dispatch rung expectation: expected `Handler` method name.
#[derive(Copy, Clone, Debug)]
pub struct DispatchExpectation {
    /// Expected handler method name (e.g. `"goto"`).
    pub method: &'static str,
}

impl DispatchExpectation {
    /// Convenience: expect a specific handler method.
    pub const fn method(name: &'static str) -> Self {
        Self { method: name }
    }
}

/// State rung expectation: expected cursor / grid state.
#[derive(Copy, Clone, Debug)]
pub struct StateExpectation {
    /// Expected cursor line (0-based).
    pub cursor_line: Option<usize>,
    /// Expected cursor column (0-based).
    pub cursor_col: Option<usize>,
}

impl StateExpectation {
    /// Convenience: expect cursor at a specific position.
    pub const fn cursor_at(line: usize, col: usize) -> Self {
        Self {
            cursor_line: Some(line),
            cursor_col: Some(col),
        }
    }
}

/// Effect rung expectation.
///
/// Matches effects by top-level family and optional sub-variant.
/// Example: `EffectExpectation::pty("DeviceAttribute")` matches
/// `Effect::Pty(PtyEffect::Write { kind: PtyWriteKind::DeviceAttribute, .. })`.
#[derive(Copy, Clone, Debug)]
pub struct EffectExpectation {
    /// Expected effect variant name (e.g. `"Pty"`, `"Host"`).
    pub variant: &'static str,
    /// Optional sub-variant name (e.g. `"DeviceAttribute"` for `PtyWriteKind`).
    /// `None` means any sub-variant matches.
    pub sub_variant: Option<&'static str>,
}

impl EffectExpectation {
    /// Expect a PTY effect with a specific write kind.
    pub const fn pty(kind: &'static str) -> Self {
        Self {
            variant: "Pty",
            sub_variant: Some(kind),
        }
    }

    /// Expect a host effect (any sub-variant).
    pub const fn host() -> Self {
        Self {
            variant: "Host",
            sub_variant: None,
        }
    }

    /// Expect any effect of the given family.
    pub const fn family(variant: &'static str) -> Self {
        Self {
            variant,
            sub_variant: None,
        }
    }
}

/// Renderable rung (Rung 4) expectation.
///
/// Asserts properties of `Term::renderable_content()` — the snapshot the
/// renderer actually consumes (NOT the live state on `Term`). Every field
/// is `Option`; `None` means "do not check this property." Every field is
/// `Copy` and const-constructible — `&'static [...]` slices, `&'static str`,
/// no `Vec` / `String`. This preserves the `SpecScenario` const-constructible
/// invariant (see module doc) so const scenarios remain greppable by the
/// citation scanner.
#[derive(Copy, Clone, Debug, Default)]
pub struct RenderableExpectation {
    /// Cell contents at specific viewport positions: `(line, col, ch)`.
    pub cells: Option<&'static [(usize, usize, char)]>,
    /// OSC 8 hyperlink URI at a viewport position: `(line, col, uri)`.
    pub hyperlink_at: Option<(usize, usize, &'static str)>,
    /// Cursor viewport position: `(line, col)`.
    pub cursor_position: Option<(usize, usize)>,
    /// Cursor shape (block / beam / underline).
    pub cursor_shape: Option<CursorShape>,
    /// Palette entry color: `(index, expected_rgb)`.
    pub palette_index: Option<(usize, Rgb)>,
    /// OSC 22 mouse cursor icon.
    pub mouse_cursor_icon: Option<CursorIcon>,
    /// Damaged-line viewport indices reported by the renderable snapshot.
    pub damaged_lines: Option<&'static [usize]>,
}

/// Frame input rung expectation (rung 5).
///
/// Asserts that the `FrameInput` assembled from terminal state has the
/// expected grid dimensions, cursor visibility, and mode flags.
#[derive(Copy, Clone, Debug)]
pub struct FrameInputExpectation {
    /// Expected grid column count.
    pub cols: Option<usize>,
    /// Expected grid row count.
    pub rows: Option<usize>,
    /// Expected cursor visibility in the renderable content.
    pub cursor_visible: Option<bool>,
    /// Expected DECSCNM (reverse video) state.
    pub reverse_video: Option<bool>,
}

impl FrameInputExpectation {
    /// Expect specific grid dimensions.
    pub const fn grid(cols: usize, rows: usize) -> Self {
        Self {
            cols: Some(cols),
            rows: Some(rows),
            cursor_visible: None,
            reverse_video: None,
        }
    }

    /// Expect default 80×24 grid with cursor visible.
    pub const fn default_grid() -> Self {
        Self {
            cols: Some(80),
            rows: Some(24),
            cursor_visible: Some(true),
            reverse_video: Some(false),
        }
    }
}

/// GPU instance rung expectation (rung 6).
///
/// Asserts that the prepared GPU instance buffers contain the expected
/// number and kind of instances after `WindowRenderer::prepare()`.
#[derive(Copy, Clone, Debug)]
pub struct GpuInstanceExpectation {
    /// Minimum number of background quad instances.
    pub min_backgrounds: Option<usize>,
    /// Minimum total glyph instances (mono + subpixel + color).
    pub min_glyphs: Option<usize>,
    /// Whether a cursor instance should be present.
    pub has_cursor: Option<bool>,
    /// Minimum number of image quad instances (below + above layers).
    /// Used by sixel / kitty graphics / iTerm2 pilots to assert that
    /// image content reached the GPU prepare phase.
    pub min_image_quads: Option<usize>,
}

impl GpuInstanceExpectation {
    /// Expect at least one background and at least one glyph.
    pub const fn non_empty() -> Self {
        Self {
            min_backgrounds: Some(1),
            min_glyphs: Some(1),
            has_cursor: None,
            min_image_quads: None,
        }
    }

    /// Expect specific minimum instance counts.
    pub const fn at_least(backgrounds: usize, glyphs: usize) -> Self {
        Self {
            min_backgrounds: Some(backgrounds),
            min_glyphs: Some(glyphs),
            has_cursor: None,
            min_image_quads: None,
        }
    }

    /// Expect image quads from sixel / kitty / iTerm2 content.
    #[must_use]
    pub const fn with_images(mut self, min: usize) -> Self {
        self.min_image_quads = Some(min);
        self
    }
}

/// Texture render rung expectation.
///
/// Validates properties of the pixel buffer produced by `render_frame_cached()`.
/// All fields are optional — `None` means "don't check this property."
#[derive(Copy, Clone, Debug, Default)]
pub struct TextureExpectation {
    /// Minimum number of non-zero pixels (any RGBA channel != 0) in the
    /// rendered buffer. This is a FLOOR GUARD, not a content detector —
    /// with an opaque background (e.g. `PALETTE_BG = Rgb(1,1,1)`), every
    /// background pixel is counted as non-zero. Use this to catch fully
    /// uninitialized targets (all transparent black), not to prove content
    /// was rendered. The golden image comparison is the content assertion.
    pub min_non_zero_pixels: Option<usize>,
    /// Expected pixel buffer dimensions. If set, both must match.
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// Golden image rung expectation.
///
/// Compares the rendered pixel buffer against a committed reference PNG
/// via `compare_with_reference_strict()`. The `golden_name` field is the
/// base name for the reference file (e.g., `"sixel_minimal"` resolves to
/// `oriterm/tests/references/sixel_minimal.png`).
#[derive(Copy, Clone, Debug, Default)]
pub struct GoldenExpectation {
    /// Base name for the reference PNG (passed to `compare_with_reference_strict`).
    /// When `None`, the scenario's `catalog_row_id` is used as the name.
    pub golden_name: Option<&'static str>,
}
