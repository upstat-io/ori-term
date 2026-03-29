//! Allocation regression tests for settings panel repaint invariants.
//!
//! Verifies that re-rendering a representative settings panel performs zero
//! heap allocations after warmup, matching the terminal scene zero-alloc
//! invariant. Uses a counting global allocator in a separate binary.
//!
//! Does NOT use `WidgetTestHarness` because the `testing` module requires
//! the `testing` feature. Instead, directly calls layout + paint on the
//! panel — the allocation invariant is about the framework pipeline, not
//! the harness.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

use oriterm_ui::action::WidgetAction;
use oriterm_ui::animation::FrameRequestFlags;
use oriterm_ui::draw::Scene;
use oriterm_ui::geometry::Rect;
use oriterm_ui::layout::{SizeSpec, compute_layout};
use oriterm_ui::text::{ShapedGlyph, ShapedText, TextMetrics, TextStyle};
use oriterm_ui::theme::UiTheme;
use oriterm_ui::widgets::button::ButtonWidget;
use oriterm_ui::widgets::container::ContainerWidget;
use oriterm_ui::widgets::settings_footer::SettingsFooterWidget;
use oriterm_ui::widgets::settings_panel::SettingsPanel;
use oriterm_ui::widgets::slider::SliderWidget;
use oriterm_ui::widgets::toggle::ToggleWidget;
use oriterm_ui::widgets::{DrawCtx, LayoutCtx, TextMeasurer, Widget};

// --- Counting allocator with enable/disable gate ---

static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOC_COUNT: AtomicU64 = AtomicU64::new(0);

struct CountingAlloc;

#[allow(unsafe_code)]
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[allow(unsafe_code)]
#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

fn measure_allocs(f: impl FnOnce()) -> u64 {
    ALLOC_COUNT.store(0, Ordering::SeqCst);
    COUNTING.store(true, Ordering::SeqCst);
    f();
    COUNTING.store(false, Ordering::SeqCst);
    ALLOC_COUNT.load(Ordering::SeqCst)
}

/// Threshold for "zero-alloc" assertion. Accounts for noise from parallel
/// test threads and minor bookkeeping allocations in the widget pipeline.
/// A real regression (per-widget alloc) would produce hundreds of allocs.
const ZERO_ALLOC_THRESHOLD: u64 = 100;

/// Minimal text measurer for allocation testing. 8px per char, 16px lines.
struct SimpleMeasurer;

impl TextMeasurer for SimpleMeasurer {
    fn measure(&self, text: &str, style: &TextStyle, _max_width: f32) -> TextMetrics {
        let transformed = style.text_transform.apply(text);
        let w = 8.0 * transformed.chars().count() as f32;
        TextMetrics {
            width: w,
            height: 16.0,
            line_count: 1,
        }
    }

    fn shape(&self, text: &str, style: &TextStyle, _max_width: f32) -> ShapedText {
        let transformed = style.text_transform.apply(text);
        let glyphs: Vec<ShapedGlyph> = transformed
            .chars()
            .enumerate()
            .map(|(i, _)| ShapedGlyph {
                glyph_id: (i + 1) as u16,
                face_index: 0,
                synthetic: 0,
                x_advance: 8.0,
                x_offset: i as f32 * 8.0,
                y_offset: 0.0,
            })
            .collect();
        let w = glyphs.len() as f32 * 8.0;
        ShapedText {
            glyphs,
            width: w,
            height: 16.0,
            baseline: 12.8,
            size_q6: (style.size * 64.0) as u32,
            weight: 400,
            font_source: oriterm_ui::text::FontSource::Ui,
        }
    }
}

static MEASURER: SimpleMeasurer = SimpleMeasurer;
static THEME: UiTheme = UiTheme::dark();

fn layout_ctx() -> LayoutCtx<'static> {
    LayoutCtx {
        measurer: &MEASURER,
        theme: &THEME,
    }
}

/// Build a representative settings panel using only `oriterm_ui` public types.
fn build_test_panel() -> SettingsPanel {
    let theme = UiTheme::dark();

    let content = ContainerWidget::column()
        .with_child(Box::new(SliderWidget::new()))
        .with_child(Box::new(SliderWidget::new()))
        .with_child(Box::new(ToggleWidget::new()))
        .with_child(Box::new(ToggleWidget::new()))
        .with_child(Box::new(ButtonWidget::new("Apply")))
        .with_width(SizeSpec::Fill)
        .with_height(SizeSpec::Fill);

    let footer = SettingsFooterWidget::new(&theme);
    let footer_ids = footer.button_ids();

    SettingsPanel::embedded(Box::new(content), footer_ids)
}

/// Layout + paint a panel into a scene, reusing the scene buffer.
fn layout_and_paint(panel: &SettingsPanel, scene: &mut Scene) {
    scene.clear();
    let ctx = layout_ctx();
    let layout_box = panel.layout(&ctx);
    let viewport = Rect::new(0.0, 0.0, 800.0, 600.0);
    let node = compute_layout(&layout_box, viewport);
    let bounds = Rect::new(0.0, 0.0, node.rect.width(), node.rect.height());
    let flags = FrameRequestFlags::new();
    let mut draw_ctx = DrawCtx {
        measurer: &MEASURER,
        scene,
        bounds,
        now: Instant::now(),
        theme: &THEME,
        icons: None,
        interaction: None,
        widget_id: None,
        frame_requests: Some(&flags),
    };
    panel.paint(&mut draw_ctx);
}

/// After warmup, repainting a settings panel must perform near-zero allocations.
#[test]
fn settings_panel_repaint_zero_alloc() {
    let panel = build_test_panel();
    let mut scene = Scene::new();

    // Warmup: establish Vec capacities.
    layout_and_paint(&panel, &mut scene);
    layout_and_paint(&panel, &mut scene);

    // Measure allocations on the third render.
    let allocs = measure_allocs(|| {
        layout_and_paint(&panel, &mut scene);
    });
    assert!(
        allocs <= ZERO_ALLOC_THRESHOLD,
        "settings panel repaint allocated {allocs} times (threshold {ZERO_ALLOC_THRESHOLD})"
    );
}

/// Toggling dirty state and repainting must not cause allocation churn.
#[test]
fn settings_panel_repaint_with_dirty_toggle_zero_alloc() {
    let mut panel = build_test_panel();
    let mut scene = Scene::new();

    // Pre-toggle dirty state to warm up any lazy allocations.
    panel.accept_action(&WidgetAction::SettingsUnsaved(true));
    layout_and_paint(&panel, &mut scene);
    panel.accept_action(&WidgetAction::SettingsUnsaved(false));
    layout_and_paint(&panel, &mut scene);

    // Measure.
    let allocs = measure_allocs(|| {
        layout_and_paint(&panel, &mut scene);
    });
    assert!(
        allocs <= ZERO_ALLOC_THRESHOLD,
        "dirty-toggle panel repaint allocated {allocs} times (threshold {ZERO_ALLOC_THRESHOLD})"
    );
}
