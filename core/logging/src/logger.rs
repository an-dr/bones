use std::sync::Arc;

use crate::{Level, LogSink, StdoutSink};

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
