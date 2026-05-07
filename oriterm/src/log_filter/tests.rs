use log::{Level, LevelFilter};

use super::LogFilter;

#[test]
fn parse_bare_level_sets_default() {
    let f = LogFilter::parse("trace");
    assert_eq!(f.max_level(), LevelFilter::Trace);
    // Bare-level acts as the default; oriterm targets at Trace are enabled.
    assert!(f.enabled("oriterm_core::grid::dirty", Level::Trace));
}

#[test]
fn parse_single_directive() {
    let f = LogFilter::parse("oriterm_core::grid::dirty=trace");
    assert_eq!(f.max_level(), LevelFilter::Trace);
    assert!(f.enabled("oriterm_core::grid::dirty", Level::Trace));
    // Default is Off — other oriterm targets at Info are disabled.
    assert!(!f.enabled("oriterm_core::term", Level::Info));
}

#[test]
fn parse_multiple_directives_full_recipe() {
    let f = LogFilter::parse(
        "oriterm_core::grid::dirty=trace,oriterm::gpu::prepare::dirty_skip=trace",
    );
    assert_eq!(f.max_level(), LevelFilter::Trace);
    assert!(f.enabled("oriterm_core::grid::dirty", Level::Trace));
    assert!(f.enabled("oriterm::gpu::prepare::dirty_skip", Level::Trace));
    // Other oriterm targets stay off (default = Off).
    assert!(!f.enabled("oriterm_core::term", Level::Info));
}

#[test]
fn parse_mixed_default_and_directives() {
    let f = LogFilter::parse("info,oriterm_core::grid::dirty=trace");
    assert_eq!(f.max_level(), LevelFilter::Trace);
    // Default Info — other oriterm targets at Info are enabled.
    assert!(f.enabled("oriterm_core::term", Level::Info));
    assert!(!f.enabled("oriterm_core::term", Level::Debug));
    // Directive overrides default for matching prefix.
    assert!(f.enabled("oriterm_core::grid::dirty", Level::Trace));
}

#[test]
fn parse_invalid_directive_falls_back_to_info() {
    let f = LogFilter::parse("this is garbage");
    // Invalid pieces dropped; nothing parsed → default Info per fallback.
    assert_eq!(f.max_level(), LevelFilter::Info);
    assert!(f.enabled("oriterm", Level::Info));
}

#[test]
fn parse_empty_string_uses_default_info() {
    let f = LogFilter::parse("");
    assert_eq!(f.max_level(), LevelFilter::Info);
    assert!(f.enabled("oriterm", Level::Info));
}

#[test]
fn enabled_with_directive_matches_target_prefix() {
    let f = LogFilter::parse("oriterm_core::grid::dirty=trace");
    // Same target at Trace → enabled.
    assert!(f.enabled("oriterm_core::grid::dirty", Level::Trace));
    // Same target at Debug → enabled (Trace is more permissive).
    assert!(f.enabled("oriterm_core::grid::dirty", Level::Debug));
    // Different oriterm target → falls through to default Off.
    assert!(!f.enabled("oriterm_core::term", Level::Trace));
}

#[test]
fn enabled_uses_longest_matching_prefix() {
    let f = LogFilter::parse("oriterm_core=info,oriterm_core::grid::dirty=trace");
    // Longer prefix wins → grid::dirty enabled at Trace.
    assert!(f.enabled("oriterm_core::grid::dirty", Level::Trace));
    // Shorter prefix matches → oriterm_core::term limited to Info.
    assert!(f.enabled("oriterm_core::term", Level::Info));
    assert!(!f.enabled("oriterm_core::term", Level::Debug));
}

#[test]
fn enabled_falls_through_to_default_only_for_oriterm_targets() {
    // TPR-04-001 pin: default applies ONLY to oriterm targets.
    let f = LogFilter::parse("trace");
    assert!(f.enabled("oriterm_core::grid::dirty", Level::Trace));
    assert!(f.enabled("oriterm::gpu::prepare", Level::Trace));
    // Non-oriterm targets ALWAYS disabled regardless of default.
    assert!(!f.enabled("wgpu_hal::vulkan", Level::Trace));
    assert!(!f.enabled("naga", Level::Info));
}

#[test]
fn enabled_directive_for_non_oriterm_target_is_silently_dropped() {
    // TPR-04-001 pin: directives that name non-oriterm targets cannot
    // accidentally enable wgpu/naga spam.
    let f = LogFilter::parse("wgpu=trace,naga=trace");
    assert!(!f.enabled("wgpu_hal::vulkan", Level::Trace));
    assert!(!f.enabled("naga", Level::Trace));
    // Directive prefixes are still in the parsed list (they were valid
    // syntax), but the noise gate vetoes their effect at enabled().
}

#[test]
fn set_max_level_uses_max_across_directives_and_default() {
    let f = LogFilter::parse(
        "info,oriterm_core::grid::dirty=trace,oriterm_core::term=warn",
    );
    // Max across {info, trace, warn} = trace.
    assert_eq!(f.max_level(), LevelFilter::Trace);
}
