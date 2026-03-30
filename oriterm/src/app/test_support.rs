//! Test helpers for dialog scene composition.
//!
//! Bridges `build_settings_dialog` (which is `pub(in crate::app)`) with
//! the golden test infrastructure in `gpu::visual_regression`. Produces a
//! painted `Scene` from a real settings widget tree without requiring a
//! window, event loop, or `DialogWindowContext`.

#[cfg(feature = "gpu-tests")]
use std::time::Instant;

#[cfg(feature = "gpu-tests")]
use oriterm_ui::animation::FrameRequestFlags;
#[cfg(feature = "gpu-tests")]
use oriterm_ui::draw::Scene;
#[cfg(feature = "gpu-tests")]
use oriterm_ui::geometry::Rect;
#[cfg(feature = "gpu-tests")]
use oriterm_ui::layout::compute_layout;
#[cfg(feature = "gpu-tests")]
use oriterm_ui::theme::UiTheme;
#[cfg(feature = "gpu-tests")]
use oriterm_ui::widgets::{DrawCtx, LayoutCtx, Widget};

#[cfg(feature = "gpu-tests")]
use crate::config::Config;
#[cfg(feature = "gpu-tests")]
use crate::font::CachedTextMeasurer;
#[cfg(feature = "gpu-tests")]
use crate::font::shaper::TextShapeCache;
#[cfg(feature = "gpu-tests")]
use crate::gpu::window_renderer::WindowRenderer;

#[cfg(feature = "gpu-tests")]
use super::settings_overlay::form_builder::build_settings_dialog;

/// Build the settings dialog for a given page, run layout + paint, return the Scene.
///
/// `renderer` must be a `WindowRenderer::new_ui_only(...)` instance with
/// `resolve_icons()` already called. `page` selects which settings page is
/// active (0 = Appearance, 1 = Colors, etc.).
#[cfg(all(test, feature = "gpu-tests"))]
pub(crate) fn build_dialog_scene(
    renderer: &WindowRenderer,
    page: usize,
    dirty: bool,
    width: f32,
    height: f32,
    scale: f32,
) -> Scene {
    let theme = UiTheme::dark();
    let config = Config::default();
    let (content_widget, _ids, footer_ids) =
        build_settings_dialog(&config, &theme, page, 1.0, 1.0, None);

    // Wrap content in SettingsPanel (the real dialog does this).
    let mut panel =
        oriterm_ui::widgets::settings_panel::SettingsPanel::embedded(content_widget, footer_ids);
    if dirty {
        panel.accept_action(&oriterm_ui::action::WidgetAction::SettingsUnsaved(true));
    }

    // Build measurer from renderer's UiFontSizes.
    let text_cache = TextShapeCache::new();
    let measurer = CachedTextMeasurer::new(renderer.ui_measurer(scale), &text_cache, scale);
    let icons = renderer.resolved_icons();

    // Layout.
    let layout_ctx = LayoutCtx {
        measurer: &measurer,
        theme: &theme,
    };
    let layout_box = panel.layout(&layout_ctx);
    let viewport = Rect::new(0.0, 0.0, width, height);
    let layout_node = compute_layout(&layout_box, viewport);

    // Paint.
    let mut scene = Scene::new();
    let paint_flags = FrameRequestFlags::new();
    let bounds = Rect::new(
        0.0,
        0.0,
        layout_node.rect.width(),
        layout_node.rect.height(),
    );
    let mut draw_ctx = DrawCtx {
        measurer: &measurer,
        scene: &mut scene,
        bounds,
        now: Instant::now(),
        theme: &theme,
        icons: Some(icons),
        interaction: None,
        widget_id: None,
        frame_requests: Some(&paint_flags),
    };
    panel.paint(&mut draw_ctx);

    scene
}
