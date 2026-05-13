//! Font / tab-bar / config-conversion bootstrap helpers used by `App::init`.
//!
//! Extracted from `app/init/mod.rs` to keep that file under the 500-line
//! budget. Each helper is a `pub(super) fn` consumed by the top-level
//! `App::try_init` orchestrator.

use crate::app::App;
use crate::font::{FontByteCache, FontCollection, FontSet, GlyphFormat, HintingMode};
use crate::window::TermWindow;

use super::DEFAULT_DPI;

impl App {
    /// Spawn font discovery on a background thread.
    ///
    /// Returns (`FontCollection`, `FontSet`, `FontByteCache`, `fallback_map`, elapsed).
    /// The `FontSet` is an `Arc`-cloned copy preserved before `FontCollection` consumes
    /// the original — zero additional disk reads.
    #[expect(
        clippy::type_complexity,
        reason = "thread join handle with font discovery result — not worth a type alias"
    )]
    pub(in crate::app) fn spawn_font_discovery(
        &self,
    ) -> Result<
        std::thread::JoinHandle<
            Result<
                (
                    FontCollection,
                    FontSet,
                    FontByteCache,
                    Vec<usize>,
                    std::time::Duration,
                ),
                crate::font::FontError,
            >,
        >,
        Box<dyn std::error::Error>,
    > {
        let font_weight = self.config.font.effective_weight();
        let font_bold_weight = self.config.font.effective_bold_weight();
        let font_size_pt = self.config.font.size;
        let font_config = self.config.font.clone();
        let font_dpi = DEFAULT_DPI;

        std::thread::Builder::new()
            .name("font-discovery".into())
            .spawn(move || {
                let t0 = std::time::Instant::now();
                let mut cache = FontByteCache::new();
                let mut font_set = FontSet::load_cached(
                    font_config.family.as_deref(),
                    font_weight,
                    font_bold_weight,
                    &mut cache,
                )?;

                // Prepend user-configured fallback fonts.
                let user_fb_families: Vec<&str> = font_config
                    .fallback
                    .iter()
                    .map(|f| f.family.as_str())
                    .collect();
                let fallback_map = font_set.prepend_user_fallbacks(&user_fb_families, &mut cache);

                // Clone before FontCollection consumes the FontSet (Arc clone, no disk I/O).
                let cached_set = font_set.clone();

                // Default to Full hinting + Alpha format; adjusted after window
                // creation once the actual display scale factor is known.
                let fc = FontCollection::new(
                    font_set,
                    font_size_pt,
                    font_dpi,
                    GlyphFormat::Alpha,
                    font_weight,
                    font_bold_weight,
                    HintingMode::Full,
                )?;
                Ok((fc, cached_set, cache, fallback_map, t0.elapsed()))
            })
            .map_err(|e| -> Box<dyn std::error::Error> {
                format!("failed to spawn font discovery thread: {e}").into()
            })
    }

    /// Create a tab bar widget and install platform window chrome.
    ///
    /// The tab bar is the sole chrome bar (unified tab-in-titlebar).
    /// Chrome installation (Aero Snap on Windows, no-op on other platforms)
    /// goes through [`NativeChromeOps`] — no `#[cfg]` blocks needed.
    pub(in crate::app) fn create_tab_bar_widget(
        &self,
        window: &TermWindow,
    ) -> oriterm_ui::widgets::tab_bar::TabBarWidget {
        let (w, _) = window.size_px();
        let scale = window.scale_factor().factor() as f32;
        let logical_w = w as f32 / scale;
        let metrics = metrics_from_style(self.config.window.tab_bar_style);

        // When the tab bar is hidden, chrome should report zero caption height
        // so macOS traffic lights and Windows Aero Snap use the correct geometry
        // from the start, not just after the first relayout.
        let hidden = self.config.window.tab_bar_position == crate::config::TabBarPosition::Hidden;
        let chrome_h = if hidden { 0.0 } else { metrics.height };

        // Publish the active tab bar height so macOS fullscreen notification
        // callbacks can center traffic lights at the correct height.
        #[cfg(target_os = "macos")]
        crate::window_manager::platform::macos::set_tab_bar_height(chrome_h);

        // Install platform chrome (Aero Snap subclass on Windows, no-op elsewhere).
        // Empty rects — the tab bar widget is created next.
        crate::app::chrome::install_chrome(
            window.window(),
            crate::window_manager::platform::ChromeMode::Main,
            &[],
            chrome_h,
            scale,
        );
        let mut tab_bar_widget = oriterm_ui::widgets::tab_bar::TabBarWidget::with_theme_and_metrics(
            logical_w,
            &self.ui_theme,
            metrics,
        );

        // Reserve space for macOS traffic light buttons on the left.
        #[cfg(target_os = "macos")]
        tab_bar_widget
            .set_left_inset(oriterm_ui::widgets::tab_bar::constants::MACOS_TRAFFIC_LIGHT_WIDTH);

        tab_bar_widget.set_tabs(vec![oriterm_ui::widgets::tab_bar::TabEntry::new("")]);

        // Set initial platform hit test rects from the tab bar.
        crate::app::chrome::refresh_chrome(
            window.window(),
            &tab_bar_widget.interactive_rects(),
            chrome_h,
            scale,
            true,
        );

        tab_bar_widget
    }
}
/// Convert a [`Decorations`](crate::config::Decorations) config value into
/// [`DecorationMode`](oriterm_ui::window::DecorationMode).
pub(in crate::app) fn decoration_to_mode(
    decorations: crate::config::Decorations,
) -> oriterm_ui::window::DecorationMode {
    match decorations {
        crate::config::Decorations::None => oriterm_ui::window::DecorationMode::Frameless,
        crate::config::Decorations::Full => oriterm_ui::window::DecorationMode::Native,
        crate::config::Decorations::Transparent => {
            oriterm_ui::window::DecorationMode::TransparentTitlebar
        }
        crate::config::Decorations::Buttonless => oriterm_ui::window::DecorationMode::Buttonless,
    }
}

/// Convert a [`TabBarStyle`](crate::config::TabBarStyle) config value into
/// [`TabBarMetrics`](oriterm_ui::widgets::tab_bar::constants::TabBarMetrics).
pub(in crate::app) fn metrics_from_style(
    style: crate::config::TabBarStyle,
) -> oriterm_ui::widgets::tab_bar::constants::TabBarMetrics {
    match style {
        crate::config::TabBarStyle::Default => {
            oriterm_ui::widgets::tab_bar::constants::TabBarMetrics::DEFAULT
        }
        crate::config::TabBarStyle::Compact => {
            oriterm_ui::widgets::tab_bar::constants::TabBarMetrics::COMPACT
        }
    }
}
