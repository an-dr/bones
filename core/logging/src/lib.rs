use std::sync::{Arc, Mutex};

/// Log severity. Ordering is not meaningful; sinks may filter as they see fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

/// Injected at kernel construction (ADR-012). Implementations must be
/// synchronous: `log` returning is the only delivery guarantee callers get.
pub trait LogSink: Send + Sync {
    fn log(&self, level: Level, category: &str, message: &str);
}

/// Default sink: writes `[LEVEL] category: message` to stdout.
pub struct StdoutSink;

impl LogSink for StdoutSink {
    fn log(&self, level: Level, category: &str, message: &str) {
        println!("[{}] {category}: {message}", level.as_str());
    }
}

/// Cheap to clone; every component that logs holds one of these.
#[derive(Clone)]
pub struct Logger {
    sink: Arc<dyn LogSink>,
}

impl Logger {
    pub fn new(sink: Arc<dyn LogSink>) -> Self {
        Self { sink }
    }

    pub fn log(&self, level: Level, category: &str, message: &str) {
        self.sink.log(level, category, message);
    }

    pub fn debug(&self, category: &str, message: &str) {
        self.log(Level::Debug, category, message);
    }

    pub fn info(&self, category: &str, message: &str) {
        self.log(Level::Info, category, message);
    }

    pub fn warn(&self, category: &str, message: &str) {
        self.log(Level::Warn, category, message);
    }

    pub fn error(&self, category: &str, message: &str) {
        self.log(Level::Error, category, message);
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new(Arc::new(StdoutSink))
    }
}

/// Test double: records every call instead of printing. Reused by bus/runner
/// tests in later increments to assert on log output without stdout capture.
#[derive(Clone, Default)]
pub struct RecordingSink {
    records: Arc<Mutex<Vec<(Level, String, String)>>>,
}

impl RecordingSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn records(&self) -> Vec<(Level, String, String)> {
        self.records.lock().unwrap().clone()
    }
}

impl LogSink for RecordingSink {
    fn log(&self, level: Level, category: &str, message: &str) {
        self.records
            .lock()
            .unwrap()
            .push((level, category.to_string(), message.to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logger_forwards_to_sink() {
        let sink = RecordingSink::new();
        let logger = Logger::new(Arc::new(sink.clone()));

        logger.debug("bus", "subscribed to input/*");
        logger.info("host", "extension loaded");
        logger.warn("bus", "queue nearing budget");
        logger.error("host", "extension trapped");

        let records = sink.records();
        assert_eq!(
            records,
            vec![
                (Level::Debug, "bus".to_string(), "subscribed to input/*".to_string()),
                (Level::Info, "host".to_string(), "extension loaded".to_string()),
                (Level::Warn, "bus".to_string(), "queue nearing budget".to_string()),
                (Level::Error, "host".to_string(), "extension trapped".to_string()),
            ]
        );
    }

    #[test]
    fn default_logger_uses_stdout_sink_without_panicking() {
        let logger = Logger::default();
        logger.info("smoke", "default logger is wired to stdout");
    }

    #[test]
    fn logger_clone_shares_the_same_sink() {
        let sink = RecordingSink::new();
        let logger = Logger::new(Arc::new(sink.clone()));
        let cloned = logger.clone();

        logger.info("a", "first");
        cloned.info("b", "second");

        assert_eq!(sink.records().len(), 2);
    }
}
