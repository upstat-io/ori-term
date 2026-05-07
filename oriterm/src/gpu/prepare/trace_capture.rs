//! Process-wide log-capture helper for `prepare/dirty_skip` trace tests.
//!
//! Same shape as `oriterm_core::grid::dirty::trace_capture` — thread-local
//! sink isolation prevents cross-test contamination when `cargo test` runs
//! tests in parallel within the binary.

use std::cell::RefCell;
use std::sync::{Arc, Mutex, OnceLock};

use log::{Level, LevelFilter, Log, Metadata, Record};

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
        let logger: &'static TestLogger = &TestLogger;
        let _ = log::set_logger(logger);
        log::set_max_level(LevelFilter::Trace);
    });
}

pub fn with_capture(level: LevelFilter, body: impl FnOnce(&MemorySink)) {
    install_logger_once();
    let sink = MemorySink::new();
    THREAD_SINK.with(|s| *s.borrow_mut() = Some(sink.clone()));
    THREAD_LEVEL.with(|l| *l.borrow_mut() = level);
    body(&sink);
    THREAD_LEVEL.with(|l| *l.borrow_mut() = LevelFilter::Off);
    THREAD_SINK.with(|s| *s.borrow_mut() = None);
}
