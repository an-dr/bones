use std::sync::Arc;

use crate::logging::{Level, LogSink};

/// Passes on only lines at `min` or above; the rest are dropped.
pub struct LevelFilter {
    inner: Arc<dyn LogSink>,
    min: Level,
}

impl LevelFilter {
    /// A sink writing lines at `min` and above to `inner`.
    pub fn new(inner: Arc<dyn LogSink>, min: Level) -> Self {
        Self { inner, min }
    }

    /// The level a name such as `warn` or `INFO` means, if any.
    pub fn parse(name: &str) -> Option<Level> {
        match name.trim().to_ascii_lowercase().as_str() {
            "debug" => Some(Level::Debug),
            "info" => Some(Level::Info),
            "warn" | "warning" => Some(Level::Warn),
            "error" => Some(Level::Error),
            _ => None,
        }
    }
}

impl LogSink for LevelFilter {
    fn log(&self, level: Level, category: &str, message: &str) {
        if rank(level) >= rank(self.min) {
            self.inner.log(level, category, message);
        }
    }
}

/// Severity order, for filtering only.
fn rank(level: Level) -> u8 {
    match level {
        Level::Debug => 0,
        Level::Info => 1,
        Level::Warn => 2,
        Level::Error => 3,
    }
}

#[cfg(test)]
mod tests;
