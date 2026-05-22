//! Process-wide log-capture helper for trace tests.
//!
//! # Design
//!
//! `log::set_logger` is global + one-shot, but `cargo test` runs tests in
//! parallel within a binary AND non-capturing tests in the same module
//! also fire traces (e.g. `mark`, `drain`, `process_incremental_cells`).
//! The captured sink lives in a `thread_local!` and the global logger
//! routes records only when the firing thread has an installed sink —
//! concurrent tests on other threads see their traces silently dropped.
//!
//! # Notes
//!
//! Canonical home for trace-emission tests across the workspace. Earlier
//! duplicates in `oriterm_core/src/grid/dirty/trace_capture.rs` and
//! `oriterm/src/gpu/prepare/trace_capture.rs` were consolidated here per
//! impl-hygiene F-01 .

use std::cell::RefCell;
use std::sync::{Arc, Mutex, OnceLock};

use log::{Level, LevelFilter, Log, Metadata, Record};

/// One captured log record snapshot — the fields a test typically asserts on.
#[derive(Debug, Clone)]
pub struct CapturedRecord {
    pub target: String,
    pub level: Level,
    pub message: String,
}

/// Shared `Vec<CapturedRecord>` cloneable handle. Internally `Arc<Mutex<...>>`
/// so the test body and the capturing logger see the same buffer.
#[derive(Default, Clone)]
pub struct MemorySink {
    inner: Arc<Mutex<Vec<CapturedRecord>>>,
}

impl MemorySink {
    /// Create an empty sink.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Snapshot the captured records.
    /// Panics if the internal mutex was poisoned by a panic in another
    /// thread — that's a real bug to surface, not a silent empty-Vec.
    pub fn records(&self) -> Vec<CapturedRecord> {
        self.inner
            .lock()
            .expect("log_capture sink mutex poisoned")
            .clone()
    }

    fn push(&self, record: &Record<'_>) {
        self.inner
            .lock()
            .expect("log_capture sink mutex poisoned")
            .push(CapturedRecord {
                target: record.target().to_string(),
                level: record.level(),
                message: format!("{}", record.args()),
            });
    }
}

thread_local! {
 static THREAD_SINK: RefCell<Option<MemorySink>> = const { RefCell::new(None) };
 static THREAD_LEVEL: RefCell<LevelFilter> = const { RefCell::new(LevelFilter::Off) };
}

struct TestLogger;

impl Log for TestLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        THREAD_LEVEL.with(|l| metadata.level() <= *l.borrow())
    }
    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        THREAD_SINK.with(|s| {
            if let Some(sink) = s.borrow().as_ref() {
                sink.push(record);
            }
        });
    }
    fn flush(&self) {}
}

static LOGGER_INSTALLED: OnceLock<()> = OnceLock::new();

fn install_logger_once() {
    LOGGER_INSTALLED.get_or_init(|| {
        let logger: &'static TestLogger = &TestLogger;
        let _ = log::set_logger(logger);
        // Set the global max to Trace so the macro short-circuit doesn't
        // veto records before they reach our per-thread filter.
        log::set_max_level(LevelFilter::Trace);
    });
}

/// Run `body` with a fresh `MemorySink` installed at `level` for the calling
/// thread only.
///
/// See: `MemorySink` for the captured-record buffer + thread-local install.
/// Concurrent tests on other threads see their traces silently dropped (no
/// thread-local sink installed); records fired on this thread go into the
/// fresh `MemorySink` for the duration of `body`. Per-thread isolation
/// avoids the race where tests that don't use `with_capture` (e.g.
/// `mark_single_line` in `dirty/tests.rs`) emit traces that contaminate the
/// captured sink of a concurrent `with_capture` body.
pub fn with_capture(level: LevelFilter, body: impl FnOnce(&MemorySink)) {
    install_logger_once();
    let sink = MemorySink::new();
    THREAD_SINK.with(|s| *s.borrow_mut() = Some(sink.clone()));
    THREAD_LEVEL.with(|l| *l.borrow_mut() = level);
    body(&sink);
    THREAD_LEVEL.with(|l| *l.borrow_mut() = LevelFilter::Off);
    THREAD_SINK.with(|s| *s.borrow_mut() = None);
}
