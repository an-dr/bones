use std::sync::Arc;

use crate::logging::{Level, LogSink};

/// Hands every line to two sinks, in order: how a composition keeps its own
/// log while also recording it somewhere else.
pub struct TeeSink {
    first: Arc<dyn LogSink>,
    second: Arc<dyn LogSink>,
}

impl TeeSink {
    /// A sink writing to `first`, then `second`.
    pub fn new(first: Arc<dyn LogSink>, second: Arc<dyn LogSink>) -> Self {
        Self { first, second }
    }
}

impl LogSink for TeeSink {
    fn log(&self, level: Level, category: &str, message: &str) {
        self.first.log(level, category, message);
        self.second.log(level, category, message);
    }
}
