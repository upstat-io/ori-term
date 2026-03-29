//! Benchmarks for the Scene pipeline: `build_scene()` equivalent + `compute_damage()`.
//!
//! Uses synthetic Scene content representing a typical 80x24 terminal with
//! ~1920 quads and ~1920 text runs. Establishes baseline frame time for the
//! Scene → DamageTracker path (GPU conversion lives in the `oriterm` crate).

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};

use oriterm_ui::color::Color;
use oriterm_ui::draw::{DamageTracker, RectStyle, Scene};
use oriterm_ui::geometry::Rect;
use oriterm_ui::widget_id::WidgetId;

/// Terminal sizes to benchmark.
const SIZES: [(usize, usize); 3] = [
    (80, 24),  // Classic terminal.
    (120, 50), // Modern split pane.
    (240, 80), // Full-screen 4K.
];

/// Populate a scene with quads simulating a terminal grid.
fn populate_quads(scene: &mut Scene, cols: usize, rows: usize, ids: &[WidgetId], color: Color) {
    scene.clear();
    let cell_w = 8.0_f32;
    let cell_h = 16.0_f32;
    for row in 0..rows {
        for col in 0..cols {
            let idx = row * cols + col;
            scene.set_widget_id(Some(ids[idx]));
            scene.push_quad(
                Rect::new(col as f32 * cell_w, row as f32 * cell_h, cell_w, cell_h),
                RectStyle::filled(color),
            );
        }
    }
}

/// Benchmark: Scene clear + repopulate (simulates `build_scene()`).
fn bench_scene_repopulate(c: &mut Criterion) {
    let mut group = c.benchmark_group("scene_repopulate");
    for &(cols, rows) in &SIZES {
        let cells = cols * rows;
        let ids: Vec<WidgetId> = (0..cells).map(|_| WidgetId::next()).collect();
        let mut scene = Scene::new();

        // Warmup to establish capacity.
        populate_quads(&mut scene, cols, rows, &ids, Color::WHITE);

        group.bench_with_input(
            BenchmarkId::new("quads", format!("{cols}x{rows}")),
            &(cols, rows),
            |b, &(cols, rows)| {
                b.iter(|| {
                    populate_quads(&mut scene, cols, rows, &ids, Color::WHITE);
                    black_box(&scene);
                });
            },
        );
    }
    group.finish();
}

/// Benchmark: DamageTracker compute_damage on identical scenes (clean path).
fn bench_damage_tracker_clean(c: &mut Criterion) {
    let mut group = c.benchmark_group("damage_tracker_clean");
    for &(cols, rows) in &SIZES {
        let cells = cols * rows;
        let ids: Vec<WidgetId> = (0..cells).map(|_| WidgetId::next()).collect();
        let mut scene = Scene::new();
        let mut tracker = DamageTracker::new();

        // Warmup: 2 frames to establish both HashMap capacities.
        populate_quads(&mut scene, cols, rows, &ids, Color::WHITE);
        tracker.compute_damage(&scene);
        tracker.compute_damage(&scene);

        group.bench_with_input(
            BenchmarkId::new("identical", format!("{cols}x{rows}")),
            &(cols, rows),
            |b, &(cols, rows)| {
                b.iter(|| {
                    populate_quads(&mut scene, cols, rows, &ids, Color::WHITE);
                    tracker.compute_damage(&scene);
                    black_box(tracker.has_damage());
                });
            },
        );
    }
    group.finish();
}

/// Benchmark: DamageTracker compute_damage with all widgets changed (dirty path).
fn bench_damage_tracker_dirty(c: &mut Criterion) {
    let mut group = c.benchmark_group("damage_tracker_dirty");
    for &(cols, rows) in &SIZES {
        let cells = cols * rows;
        let ids: Vec<WidgetId> = (0..cells).map(|_| WidgetId::next()).collect();
        let mut scene = Scene::new();
        let mut tracker = DamageTracker::new();
        let mut frame = 0u64;

        // Warmup.
        populate_quads(&mut scene, cols, rows, &ids, Color::WHITE);
        tracker.compute_damage(&scene);
        populate_quads(&mut scene, cols, rows, &ids, Color::BLACK);
        tracker.compute_damage(&scene);

        group.bench_with_input(
            BenchmarkId::new("all_changed", format!("{cols}x{rows}")),
            &(cols, rows),
            |b, &(cols, rows)| {
                b.iter(|| {
                    frame += 1;
                    let color = if frame % 2 == 0 {
                        Color::WHITE
                    } else {
                        Color::BLACK
                    };
                    populate_quads(&mut scene, cols, rows, &ids, color);
                    tracker.compute_damage(&scene);
                    black_box(tracker.has_damage());
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_scene_repopulate,
    bench_damage_tracker_clean,
    bench_damage_tracker_dirty,
);
criterion_main!(benches);
