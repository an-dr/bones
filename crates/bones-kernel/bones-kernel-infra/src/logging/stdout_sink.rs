use crate::logging::{Level, LogSink};

/// Default sink: writes `[LEVEL] category: message` to stdout.
pub struct StdoutSink;

impl LogSink for StdoutSink {
    fn log(&self, level: Level, category: &str, message: &str) {
        println!("[{}] {category}: {message}", level.as_str());
    }
}
