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
        }
    }

    /// Record a render frame with its elapsed time.
    pub(super) fn record_render(&mut self, frame_time: Duration) {
        self.renders += 1;
        self.frame_time_min = self.frame_time_min.min(frame_time);
        self.frame_time_max = self.frame_time_max.max(frame_time);
        self.frame_time_sum += frame_time;
        self.last_activity = Instant::now();
        self.idle_logged = false;
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
        self.window_start = Instant::now();
        true
    }
}
