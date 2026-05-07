//! Process-wide log-capture helper for `DirtyTracker` trace tests.
//!
//! `set_logger` is global + one-shot, but `cargo test` runs tests in parallel
//! within a binary AND non-capturing tests in this module also fire traces
//! (`mark`, `drain`, etc.). To prevent cross-test contamination the captured
//! sink is held in a `thread_local!` and the global logger only routes
//! records whose firing thread has an installed sink. Concurrent tests that
//! never installed a sink see their traces silently dropped — exactly what
//! they want.

use std::cell::RefCell;
use std::sync::{Arc, Mutex, OnceLock};

use log::{Level, LevelFilter, Log, Metadata, Record};

/// Captured record snapshot — the fields a test typically asserts on.
#[derive(Debug, Clone)]
pub struct CapturedRecord {
    pub target: String,
    pub level: Level,
    pub message: String,
}

#[derive(Default, Clone)]
pub struct MemorySink {
    inner: Arc<Mutex<Vec<CapturedRecord>>>,
}

impl MemorySink {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn records(&self) -> Vec<CapturedRecord> {
        self.inner.lock().map(|g| g.clone()).unwrap_or_default()
    }

    fn push(&self, record: &Record<'_>) {
        if let Ok(mut g) = self.inner.lock() {
            g.push(CapturedRecord {
                target: record.target().to_string(),
                level: record.level(),
                message: format!("{}", record.args()),
            });
        }
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
        // The TestLogger is a zero-sized type with no state; safe to leak.
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
/// Concurrent tests on other threads see their traces silently dropped (no
/// thread-local sink installed). Records fired on this thread go into the
/// fresh `MemorySink` for the duration of `body`. Per-thread isolation
/// avoids the race where tests that don't use `with_capture` (e.g. the
/// existing `mark_single_line` test) emit traces that contaminate the
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
