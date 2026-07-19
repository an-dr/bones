use std::sync::{Arc, Mutex};

use crate::{Level, LogSink};

/// Test double: records every call instead of printing. Reused by bus/runner
/// tests to assert on log output without stdout capture.
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
