//! Periodic performance statistics logging.
//!
//! Counts renders, mux wakeups, and cursor-move events per interval, then
//! logs a summary line. Helps diagnose contention, rendering bottlenecks,
//! and unnecessary wakeups without runtime overhead beyond an atomic
//! increment per event.
//!
//! When `--profile` is passed, stats log at `info` level (visible without
//! `RUST_LOG=debug`). Frame timing min/max/avg and idle detection are
//! included in profiling mode.

use std::time::{Duration, Instant};

/// Interval between performance log lines.
const LOG_INTERVAL: Duration = Duration::from_secs(5);

/// Threshold for idle detection — no wakeups for this long means "truly idle".
const IDLE_THRESHOLD: Duration = Duration::from_secs(1);

/// Per-frame phase timing breakdown (profiling mode only).
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct FramePhases {
    /// Mux pump (PTY event drain + notification handling).
    pub mux_pump: Duration,
    /// Snapshot extraction (refresh + renderable content swap/copy).
    pub extract: Duration,
    /// Font shaping + glyph caching + instance buffer fill.
    pub prepare: Duration,
    /// Widget pipeline (layout + prepaint + paint + tab bar + overlays).
    pub widgets: Duration,
    /// GPU render (upload + render passes + present).
    pub gpu_render: Duration,
}

impl FramePhases {
    pub(super) fn accumulate(&mut self, other: &Self) {
        self.mux_pump += other.mux_pump;
        self.extract += other.extract;
        self.prepare += other.prepare;
        self.widgets += other.widgets;
        self.gpu_render += other.gpu_render;
    }

    fn max_merge(&mut self, other: &Self) {
        self.mux_pump = self.mux_pump.max(other.mux_pump);
        self.extract = self.extract.max(other.extract);
        self.prepare = self.prepare.max(other.prepare);
        self.widgets = self.widgets.max(other.widgets);
        self.gpu_render = self.gpu_render.max(other.gpu_render);
    }
}

/// Per-interval performance counters.
pub(super) struct PerfStats {
    /// Start of the current measurement window.
    window_start: Instant,
    /// Number of `handle_redraw` calls this window.
    renders: u32,
    /// Number of `MuxWakeup` / `pump_mux_events` calls this window.
    wakeups: u32,
    /// Number of `CursorMoved` events this window.
    cursor_moves: u32,
    /// Number of `about_to_wait` calls this window.
    ticks: u32,
    /// Minimum frame time in the current window.
    frame_time_min: Duration,
    /// Maximum frame time in the current window.
    frame_time_max: Duration,
    /// Sum of all frame times in the current window.
    frame_time_sum: Duration,
    /// Whether `--profile` was passed (logs at `info` instead of `debug`).
    profiling: bool,
    /// Last time any activity occurred (render, wakeup, cursor).
    last_activity: Instant,
    /// Whether we've logged the "entering idle" transition.
    idle_logged: bool,
    /// RSS at startup (first measurement). Used for delta reporting.
    initial_rss: Option<usize>,
    /// Peak RSS observed across all intervals.
    peak_rss: usize,

    // Phase timing accumulators (profiling mode only).
    phase_sum: FramePhases,
    phase_max: FramePhases,

    /// Most recent mux pump duration (set in `about_to_wait`).
    pub(super) last_pump_time: Duration,

    // Key-to-render latency tracking.
    /// Timestamp of the most recent key event (set in `handle_keyboard_input`).
    pub(super) last_key_time: Option<Instant>,
    /// Sum of key-to-render latencies for frames that had a pending key event.
    key_to_render_sum: Duration,
    /// Maximum key-to-render latency observed this interval.
    key_to_render_max: Duration,
    /// Number of frames that measured key-to-render latency.
    key_to_render_count: u32,
}

impl PerfStats {
    /// Create a new counter set.
    ///
    /// Pass `profiling: true` for `--profile` mode (info-level logging).
    pub(super) fn new(profiling: bool) -> Self {
        let now = Instant::now();
        let initial_rss = if profiling {
            crate::platform::memory::rss_bytes()
        } else {
            None
        };
        Self {
            window_start: now,
            renders: 0,
            wakeups: 0,
            cursor_moves: 0,
            ticks: 0,
            frame_time_min: Duration::MAX,
            frame_time_max: Duration::ZERO,
            frame_time_sum: Duration::ZERO,
            profiling,
            last_activity: now,
            idle_logged: false,
            initial_rss,
            peak_rss: initial_rss.unwrap_or(0),
            phase_sum: FramePhases::default(),
            phase_max: FramePhases::default(),
            last_pump_time: Duration::ZERO,
            last_key_time: None,
            key_to_render_sum: Duration::ZERO,
            key_to_render_max: Duration::ZERO,
            key_to_render_count: 0,
        }
    }

    /// Record a render frame with its elapsed time and phase breakdown.
    pub(super) fn record_render(&mut self, frame_time: Duration, phases: &FramePhases) {
        self.renders += 1;
        self.frame_time_min = self.frame_time_min.min(frame_time);
        self.frame_time_max = self.frame_time_max.max(frame_time);
        self.frame_time_sum += frame_time;
        self.last_activity = Instant::now();
        self.idle_logged = false;

        // Copy mux pump time recorded in about_to_wait into phases.
        let mut full_phases = *phases;
        full_phases.mux_pump = self.last_pump_time;
        self.phase_sum.accumulate(&full_phases);
        self.phase_max.max_merge(&full_phases);

        // Key-to-render latency: measure from last key event to render completion.
        if let Some(key_time) = self.last_key_time.take() {
            let latency = key_time.elapsed();
            self.key_to_render_sum += latency;
            self.key_to_render_max = self.key_to_render_max.max(latency);
            self.key_to_render_count += 1;

            // Log slow frames individually for stutter diagnosis.
            if latency.as_millis() > 16 || frame_time.as_millis() > 8 {
                log::info!(
                    "perf: SLOW k2r={latency:.1?} frame={frame_time:.1?} \
                     prep={:.1?} gpu={:.1?}",
                    phases.prepare,
                    phases.gpu_render,
                );
            }
        }
    }

    /// Record a mux wakeup (PTY reader thread notification).
    pub(super) fn record_wakeup(&mut self) {
        self.wakeups += 1;
        self.last_activity = Instant::now();
        self.idle_logged = false;
    }

    /// Record a cursor-move event.
    pub(super) fn record_cursor_move(&mut self) {
        self.cursor_moves += 1;
    }

    /// Record an `about_to_wait` tick.
    pub(super) fn record_tick(&mut self) {
        self.ticks += 1;
    }

    /// Log per-phase frame timing breakdown.
    fn log_phase_breakdown(&self, log_fn: fn(&str)) {
        let n = self.renders;
        let avg = |d: Duration| d / n;
        log_fn(&format!(
            "perf: phases avg: mux={:.1?} extract={:.1?} prepare={:.1?} \
             widgets={:.1?} gpu={:.1?}",
            avg(self.phase_sum.mux_pump),
            avg(self.phase_sum.extract),
            avg(self.phase_sum.prepare),
            avg(self.phase_sum.widgets),
            avg(self.phase_sum.gpu_render),
        ));
        log_fn(&format!(
            "perf: phases max: mux={:.1?} extract={:.1?} prepare={:.1?} \
             widgets={:.1?} gpu={:.1?}",
            self.phase_max.mux_pump,
            self.phase_max.extract,
            self.phase_max.prepare,
            self.phase_max.widgets,
            self.phase_max.gpu_render,
        ));
        if self.key_to_render_count > 0 {
            let k2r_avg = self.key_to_render_sum / self.key_to_render_count;
            log_fn(&format!(
                "perf: key→render: avg={k2r_avg:.1?} max={:.1?} ({} samples)",
                self.key_to_render_max, self.key_to_render_count,
            ));
        }
    }

    /// Check and log idle state transitions.
    ///
    /// Logs when the event loop enters true idle (no activity for > 1s)
    /// and when it exits. Only active in profiling mode.
    pub(super) fn check_idle(&mut self) {
        if !self.profiling {
            return;
        }
        let since = self.last_activity.elapsed();
        if since >= IDLE_THRESHOLD && !self.idle_logged {
            log::info!("perf: entering idle (no activity for {since:.1?})");
            self.idle_logged = true;
        }
    }

    /// Flush counters and log if the interval has elapsed.
    ///
    /// Returns `true` if a log line was emitted.
    pub(super) fn maybe_log(&mut self) -> bool {
        let elapsed = self.window_start.elapsed();
        if elapsed < LOG_INTERVAL {
            return false;
        }

        let secs = elapsed.as_secs_f64();

        let frame_avg = if self.renders > 0 {
            self.frame_time_sum / self.renders
        } else {
            Duration::ZERO
        };

        // Choose log level: info for --profile, debug otherwise.
        let log_fn: fn(&str) = if self.profiling {
            |msg| log::info!("{msg}")
        } else {
            |msg| log::debug!("{msg}")
        };

        #[cfg(feature = "profile")]
        {
            let renders_f = f64::from(self.renders).max(1.0);
            let snap = crate::alloc::snapshot_and_reset();
            let net = snap.bytes_allocated.saturating_sub(snap.bytes_deallocated);
            log_fn(&format!(
                "perf: {:.0} renders/s, {:.0} wakeups/s, {:.0} cursor/s, {:.0} ticks/s \
                 | frame: {:.1?}/{:.1?}/{:.1?} (min/avg/max) \
                 | allocs: {:.0}/s ({:.0}/frame), deallocs: {:.0}/s, \
                 bytes: +{:.0}/s -{:.0}/s (net {:.0}/s)",
                f64::from(self.renders) / secs,
                f64::from(self.wakeups) / secs,
                f64::from(self.cursor_moves) / secs,
                f64::from(self.ticks) / secs,
                self.frame_time_min,
                frame_avg,
                self.frame_time_max,
                snap.allocs as f64 / secs,
                snap.allocs as f64 / renders_f,
                snap.deallocs as f64 / secs,
                snap.bytes_allocated as f64 / secs,
                snap.bytes_deallocated as f64 / secs,
                net as f64 / secs,
            ));
        }

        #[cfg(not(feature = "profile"))]
        if self.profiling {
            log_fn(&format!(
                "perf: {:.0} renders/s, {:.0} wakeups/s, {:.0} cursor/s, {:.0} ticks/s \
                 | frame: {:.1?}/{:.1?}/{:.1?} (min/avg/max)",
                f64::from(self.renders) / secs,
                f64::from(self.wakeups) / secs,
                f64::from(self.cursor_moves) / secs,
                f64::from(self.ticks) / secs,
                self.frame_time_min,
                frame_avg,
                self.frame_time_max,
            ));
        } else {
            log_fn(&format!(
                "perf: {:.0} renders/s, {:.0} wakeups/s, {:.0} cursor/s, {:.0} ticks/s",
                f64::from(self.renders) / secs,
                f64::from(self.wakeups) / secs,
                f64::from(self.cursor_moves) / secs,
                f64::from(self.ticks) / secs,
            ));
        }

        // Phase breakdown (profiling mode only).
        if self.profiling && self.renders > 0 {
            self.log_phase_breakdown(log_fn);
        }

        // Memory watermark (profiling mode only).
        if self.profiling {
            if let Some(rss) = crate::platform::memory::rss_bytes() {
                if rss > self.peak_rss {
                    self.peak_rss = rss;
                }
                let fmt = crate::platform::memory::format_bytes;
                let delta = match self.initial_rss {
                    Some(init) if rss >= init => {
                        format!(" (+{} since start)", fmt(rss - init))
                    }
                    Some(init) => format!(" (-{} since start)", fmt(init - rss)),
                    None => String::new(),
                };
                log::info!(
                    "perf: RSS {} (peak {}){delta}",
                    fmt(rss),
                    fmt(self.peak_rss),
                );
            }
        }

        self.renders = 0;
        self.wakeups = 0;
        self.cursor_moves = 0;
        self.ticks = 0;
        self.frame_time_min = Duration::MAX;
        self.frame_time_max = Duration::ZERO;
        self.frame_time_sum = Duration::ZERO;
        self.phase_sum = FramePhases::default();
        self.phase_max = FramePhases::default();
        self.key_to_render_sum = Duration::ZERO;
        self.key_to_render_max = Duration::ZERO;
        self.key_to_render_count = 0;
        self.window_start = Instant::now();
        true
    }
}
