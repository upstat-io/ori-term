//! Const-constructible scenario definitions for the verification chain.
//!
//! Every catalog row that reaches `verified` status is backed by a test
//! declaring a `const SpecScenario` and driving it through the harness.
//! Const-constructibility enables the citation scanner (04.8) to find
//! `catalog_row_id: "…"` via a literal grep.

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
    /// Expected final byte / action char (e.g. `'H'` for CUP).
    pub action: char,
    /// Expected parameter values (flattened subparams).
    pub params: &'static [u16],
    /// Expected CSI/ESC intermediate bytes.
    pub intermediates: &'static [u8],
}

impl ParserExpectation {
    /// Convenience: CSI sequence with given action and params.
    pub const fn csi_with_params(action: char, params: &'static [u16]) -> Self {
        Self {
            action,
            params,
            intermediates: &[],
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

/// Renderable rung expectation (stub — expanded when pilots exercise it).
#[derive(Copy, Clone, Debug, Default)]
pub struct RenderableExpectation;

/// Frame input rung expectation (stub — expanded in 04.3b).
#[derive(Copy, Clone, Debug, Default)]
pub struct FrameInputExpectation;

/// GPU instance rung expectation (stub — expanded in 04.3b).
#[derive(Copy, Clone, Debug, Default)]
pub struct GpuInstanceExpectation;

/// Texture render rung expectation (stub — expanded in 04.4).
#[derive(Copy, Clone, Debug, Default)]
pub struct TextureExpectation;

/// Golden image rung expectation (stub — expanded in 04.4).
#[derive(Copy, Clone, Debug, Default)]
pub struct GoldenExpectation;
